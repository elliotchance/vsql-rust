//! Fetching the rows of a query a rowset (many rows) at a time, into arrays
//! that are bound to its columns. Each libvsql routine call has a large
//! overhead, so this is much faster than SQLFetch and SQLGetData for each
//! row and column.

use vsql_sys::*;

use super::Column;
use crate::error::{Error, Result};
use crate::raw;
use crate::types::{Kind, Type, Value};

/// The most rows in a rowset.
const MAX_ROWS: usize = 100;

/// The most octets of the arrays of a rowset, which is fewer rows for wider
/// rows.
const MAX_OCTETS: usize = 1 << 20;

/// The widest value that's bound. A column that may be wider (such as a
/// CHARACTER LARGE OBJECT without a length) is read with SQLGetData instead,
/// so a query with one doesn't use rowsets.
const MAX_WIDTH: usize = 1 << 16;

/// The octets for the string of a value that isn't a character string (such
/// as a DATE or a DECIMAL), on top of its column size, which is the number of
/// digits or characters of the value without its sign, decimal point, literal
/// prefix (such as `DATE '`) or interval fields.
const STRING_SLACK: usize = 128;

/// The bound array of a column.
enum Values {
    Integer(Vec<SQLBIGINT>),
    Double(Vec<SQLDOUBLE>),
    Real(Vec<SQLREAL>),
    /// Null terminated strings of the width.
    Text(Vec<u8>),
    Binary(Vec<u8>),
}

/// A bound column.
struct Bound {
    values: Values,
    /// The octets of each value.
    width: usize,
    /// The length of each value, or SQL_NULL_DATA.
    lengths: Vec<SQLINTEGER>,
}

impl Bound {
    fn new(column: &Column, kind: Kind) -> Option<Bound> {
        let size = usize::try_from(column.size).unwrap_or(0);
        // A string whose length libvsql doesn't know (such as the CHARACTER
        // VARYING of CURRENT_SCHEMA) has a size of 0, so it can't be bound.
        let unknown = size == 0;
        let (values, width) = match kind {
            Kind::Integer => (Values::Integer(Vec::new()), 8),
            Kind::Double => (Values::Double(Vec::new()), 8),
            Kind::Real => (Values::Real(Vec::new()), 4),
            // The size of a binary string is its length in octets.
            Kind::Binary if unknown => return None,
            Kind::Binary => (Values::Binary(Vec::new()), size),
            // The size of a character string is its length in octets, and
            // the string is null terminated.
            Kind::Text => match column.ty {
                Type::Character | Type::CharacterVarying | Type::CharacterLargeObject => {
                    if unknown {
                        return None;
                    }
                    (Values::Text(Vec::new()), size.saturating_add(1))
                }
                _ => (
                    Values::Text(Vec::new()),
                    size.saturating_add(STRING_SLACK + 1),
                ),
            },
        };
        if width > MAX_WIDTH {
            return None;
        }

        Some(Bound {
            values,
            width,
            lengths: Vec::new(),
        })
    }

    /// Allocates the arrays for the rows, and binds them to the column
    /// (starting at 1).
    fn bind(&mut self, stmt: SQLHSTMT, n: usize, rows: usize) -> Result<()> {
        self.lengths = vec![0; rows];
        let (target_type, values) = match &mut self.values {
            Values::Integer(v) => (SQL_BIGINT, array(v, rows)),
            Values::Double(v) => (SQL_DOUBLE, array(v, rows)),
            Values::Real(v) => (SQL_REAL, array(v, rows)),
            Values::Text(v) => (SQL_CHAR, array(v, rows * self.width)),
            Values::Binary(v) => (SQL_VARBINARY, array(v, rows * self.width)),
        };
        let lengths = self.lengths.as_mut_ptr();

        raw::bind_col(stmt, n, target_type, values, self.width, lengths)
    }

    /// The value of a row of the rowset, for the column (at index).
    fn value(&self, row: usize, index: usize, column: &Column) -> Result<Value> {
        let length = self.lengths[row];
        if length == SQL_NULL_DATA {
            return Ok(Value::Null);
        }

        Ok(match &self.values {
            Values::Integer(v) => Value::Integer(v[row]),
            Values::Double(v) => Value::Double(v[row]),
            Values::Real(v) => Value::Double(f64::from(v[row])),
            Values::Binary(v) => Value::Binary(self.octets(v, row, length, 0)?.to_vec()),
            Values::Text(v) => {
                let s = String::from_utf8(self.octets(v, row, length, 1)?.to_vec())
                    .map_err(|_| sql_error("22021", "a string isn't valid UTF-8"))?;
                Value::parse(s, column.ty).map_err(|source| Error::FromSqlConversion {
                    index,
                    name: column.name.clone(),
                    ty: column.ty,
                    source,
                })?
            }
        })
    }

    /// The octets of a string of a row, which is an error if it didn't fit
    /// (which the widths should prevent).
    fn octets<'a>(
        &self,
        v: &'a [u8],
        row: usize,
        length: SQLINTEGER,
        nul: usize,
    ) -> Result<&'a [u8]> {
        let length = usize::try_from(length).unwrap_or(0);
        if length > self.width - nul {
            return Err(sql_error("01004", "string data, right truncation"));
        }
        let start = row * self.width;

        Ok(&v[start..start + length])
    }
}

/// An array of n values, as a pointer for SQLBindCol.
fn array<T: Default + Clone>(v: &mut Vec<T>, n: usize) -> SQLPOINTER {
    *v = vec![T::default(); n];

    v.as_mut_ptr() as SQLPOINTER
}

fn sql_error(sqlstate: &str, message: &str) -> Error {
    Error::Sql {
        sqlstate: sqlstate.to_string(),
        message: message.to_string(),
        diagnostics: Box::default(),
    }
}

/// The bound columns of a query, which fetch a rowset at a time. They must
/// be unbound (see [`raw::unbind_cols`]) before the rowset is dropped, or the
/// statement is used again.
pub(super) struct Rowset {
    columns: Vec<Bound>,
    rows: usize,
    /// The number of rows that were fetched into the arrays, which libvsql
    /// sets.
    fetched: Box<SQLINTEGER>,
    /// The next row (of those fetched).
    next: usize,
}

impl Rowset {
    /// A rowset for the columns, or None if a column can't be bound.
    pub(super) fn new(columns: &[Column], kinds: &[Kind]) -> Option<Rowset> {
        let columns = columns
            .iter()
            .zip(kinds)
            .map(|(column, &kind)| Bound::new(column, kind))
            .collect::<Option<Vec<_>>>()?;
        let row_width: usize = columns.iter().map(|c| c.width + 4).sum();

        Some(Rowset {
            rows: (MAX_OCTETS / row_width.max(1)).clamp(1, MAX_ROWS),
            columns,
            fetched: Box::new(0),
            next: 0,
        })
    }

    /// Binds the columns of the statement's cursor to the arrays.
    pub(super) fn bind(&mut self, stmt: SQLHSTMT) -> Result<()> {
        raw::set_rowset_size(stmt, self.rows, &mut *self.fetched)?;
        for (i, column) in self.columns.iter_mut().enumerate() {
            column.bind(stmt, i + 1, self.rows)?;
        }

        Ok(())
    }

    /// The values of the next row, which fetches the next rowset after the
    /// last row of this one. It's None after the last row.
    ///
    /// A row that can't be fetched isn't counted as fetched, but the bound
    /// types are the types of the columns (or their strings), so there are
    /// no such rows.
    pub(super) fn next(
        &mut self,
        stmt: SQLHSTMT,
        columns: &[Column],
    ) -> Result<Option<Vec<Value>>> {
        if self.next >= usize::try_from(*self.fetched).unwrap_or(0) {
            if !raw::fetch(stmt)? {
                return Ok(None);
            }
            self.next = 0;
        }
        let row = self.next;
        self.next += 1;

        self.columns
            .iter()
            .zip(columns)
            .enumerate()
            .map(|(i, (bound, column))| bound.value(row, i, column))
            .collect::<Result<_>>()
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(ty: Type, size: i32) -> Column {
        Column {
            name: "X".to_string(),
            ty,
            nullable: true,
            size,
            digits: 0,
        }
    }

    #[test]
    fn widths() {
        let width = |ty, size, kind| Bound::new(&column(ty, size), kind).map(|b| b.width);
        assert_eq!(width(Type::CharacterVarying, 40, Kind::Text), Some(41));
        assert_eq!(width(Type::Date, 10, Kind::Text), Some(139));
        assert_eq!(width(Type::Date, -1, Kind::Text), Some(129));
        assert_eq!(width(Type::BinaryVarying, 1, Kind::Binary), Some(1));
        assert_eq!(width(Type::BinaryVarying, 0, Kind::Binary), None);
        assert_eq!(width(Type::CharacterVarying, 0, Kind::Text), None);
        assert_eq!(
            width(Type::CharacterLargeObject, i32::MAX, Kind::Text),
            None
        );
        assert_eq!(width(Type::BinaryLargeObject, i32::MAX, Kind::Binary), None);
    }

    #[test]
    fn strings_that_dont_fit() {
        // The widths should prevent this, but a string that didn't fit is an
        // error rather than part of the string.
        let c = column(Type::CharacterVarying, 4);
        let bound = Bound {
            values: Values::Text(b"abcd\0".to_vec()),
            width: 5,
            lengths: vec![8],
        };
        assert!(matches!(
            bound.value(0, 0, &c),
            Err(Error::Sql { sqlstate, .. }) if sqlstate == "01004"
        ));

        let bound = Bound {
            values: Values::Text(b"maybe\0".to_vec()),
            width: 6,
            lengths: vec![5],
        };
        assert!(matches!(
            bound.value(0, 2, &column(Type::Boolean, 5)),
            Err(Error::FromSqlConversion { index: 2, .. })
        ));

        let bound = Bound {
            values: Values::Text(b"\xff\0".to_vec()),
            width: 2,
            lengths: vec![1],
        };
        assert!(matches!(
            bound.value(0, 0, &c),
            Err(Error::Sql { sqlstate, .. }) if sqlstate == "22021"
        ));
    }
}

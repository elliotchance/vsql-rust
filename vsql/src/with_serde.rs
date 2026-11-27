// Deserializing a Row with serde.

use std::fmt;

use serde::de::value::{BorrowedStrDeserializer, SeqDeserializer};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use crate::error::{Error, Result};
use crate::statement::{Column, Row};
use crate::types::{FromSql, FromSqlError, Value};

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Error::Deserialize(msg.to_string())
    }
}

/// Deserializes a row as `T`, with the `serde` feature.
///
/// A struct's fields are the columns with the same name (ignoring ASCII case,
/// so a field `id` is the column `ID`), in any order. Columns that aren't
/// fields are ignored, and a missing column is an error unless the field is
/// an `Option` (or has a `#[serde(default)]`). A tuple (or tuple struct) is
/// the columns in order (it's an error if there are more or fewer columns), a
/// `Vec` is all of the columns, a map is each column's name and value, and
/// anything else is the only column of the row.
///
/// Each value is converted like [`Row::get`] (see [`FromSql`]), so an integer
/// field can be a DECIMAL that's an integer, and a NULL is only an `Option`.
/// Strings and binary strings can be borrowed from the row (`&str` and
/// `&[u8]`, with `#[serde(borrow)]`), and a datetime or interval is the
/// string of its `Display`, such as `"2022-01-02"` for a [`Date`](crate::Date).
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Person {
///     id: i64,
///     name: String,
///     email: Option<String>,
/// }
///
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE person (id INTEGER, name VARCHAR(10), email VARCHAR(20))", ())?;
/// conn.execute("INSERT INTO person (id, name) VALUES (1, 'Bob')", ())?;
///
/// let mut stmt = conn.prepare("SELECT * FROM person")?;
/// let people = stmt
///     .query_map((), |row| vsql::from_row::<Person>(row))?
///     .collect::<vsql::Result<Vec<_>>>()?;
/// assert_eq!(
///     people,
///     [Person { id: 1, name: "Bob".to_string(), email: None }]
/// );
/// # Ok(())
/// # }
/// ```
pub fn from_row<'de, T: de::Deserialize<'de>>(row: &'de Row) -> Result<T> {
    T::deserialize(row)
}

/// The deserializer of a column's value.
struct ValueDeserializer<'de> {
    value: &'de Value,
    column: &'de Column,
    index: usize,
}

impl ValueDeserializer<'_> {
    fn error(&self, source: FromSqlError) -> Error {
        Error::FromSqlConversion {
            index: self.index,
            name: self.column.name().to_string(),
            ty: self.column.ty(),
            source,
        }
    }

    fn get<T: FromSql>(&self) -> Result<T> {
        T::from_sql(self.value).map_err(|source| self.error(source))
    }
}

/// Deserializes column `index` with `f`. A message from deserializing it
/// (such as an invalid type) is a FromSqlConversion of the column.
fn deserialize_column<'de, T>(
    row: &'de Row,
    index: usize,
    f: impl FnOnce(ValueDeserializer<'de>) -> Result<T>,
) -> Result<T> {
    let column = &row.columns()[index];
    let deserializer = ValueDeserializer {
        value: row.get_ref(index)?,
        column,
        index,
    };

    f(deserializer).map_err(|e| match e {
        Error::Deserialize(message) => Error::FromSqlConversion {
            index,
            name: column.name().to_string(),
            ty: column.ty(),
            source: FromSqlError::Other(message.into()),
        },
        e => e,
    })
}

macro_rules! deserialize_from_sql {
    ($($method:ident => $visit:ident),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            visitor.$visit(self.get()?)
        }
    )*};
}

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Boolean(b) => visitor.visit_bool(*b),
            Value::Integer(i) => visitor.visit_i64(*i),
            Value::Double(d) => visitor.visit_f64(*d),
            Value::Numeric(s) | Value::DecFloat(s) | Value::Text(s) => {
                visitor.visit_borrowed_str(s)
            }
            Value::Binary(b) => visitor.visit_borrowed_bytes(b),
            Value::Date(d) => visitor.visit_string(d.to_string()),
            Value::Time(t) => visitor.visit_string(t.to_string()),
            Value::Timestamp(t) => visitor.visit_string(t.to_string()),
            Value::Interval(i) => visitor.visit_string(i.to_string()),
        }
    }

    deserialize_from_sql!(
        deserialize_bool => visit_bool,
        deserialize_i8 => visit_i8,
        deserialize_i16 => visit_i16,
        deserialize_i32 => visit_i32,
        deserialize_i64 => visit_i64,
        deserialize_i128 => visit_i128,
        deserialize_u8 => visit_u8,
        deserialize_u16 => visit_u16,
        deserialize_u32 => visit_u32,
        deserialize_u64 => visit_u64,
        deserialize_u128 => visit_u128,
        deserialize_f32 => visit_f32,
        deserialize_f64 => visit_f64
    );

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    /// A NULL is only an `Option` (or `()`).
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.error(FromSqlError::InvalidType)),
        }
    }

    /// A binary string is also a sequence of octets, for a `Vec<u8>`.
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::Binary(b) => visitor.visit_seq(SeqDeserializer::new(b.iter().copied())),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    /// An enum of unit variants is the variant's name.
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.value {
            Value::Text(s) => {
                BorrowedStrDeserializer::<Error>::new(s).deserialize_enum(name, variants, visitor)
            }
            _ => Err(self.error(FromSqlError::InvalidType)),
        }
    }

    forward_to_deserialize_any! {
        char str string bytes byte_buf unit_struct tuple tuple_struct map struct
        identifier ignored_any
    }
}

/// The columns of a row (or the fields of a struct) and their values.
struct RowMap<'de> {
    row: &'de Row,
    /// The key and the index of its column.
    entries: Vec<(&'de str, usize)>,
    next: usize,
}

impl<'de> MapAccess<'de> for RowMap<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        match self.entries.get(self.next) {
            Some((key, _)) => seed
                .deserialize(BorrowedStrDeserializer::new(key))
                .map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let index = self.entries[self.next].1;
        self.next += 1;
        deserialize_column(self.row, index, |d| seed.deserialize(d))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len() - self.next)
    }
}

/// The values of a row in order.
struct RowSeq<'de> {
    row: &'de Row,
    next: usize,
}

impl<'de> SeqAccess<'de> for RowSeq<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.next == self.row.len() {
            return Ok(None);
        }
        self.next += 1;
        deserialize_column(self.row, self.next - 1, |d| seed.deserialize(d)).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.row.len() - self.next)
    }
}

/// The only column, for a value that isn't a struct, tuple or map.
fn only_column<'de, T>(
    row: &'de Row,
    f: impl FnOnce(ValueDeserializer<'de>) -> Result<T>,
) -> Result<T> {
    columns(row, 1)?;
    deserialize_column(row, 0, f)
}

/// An error if the row doesn't have `n` columns.
fn columns(row: &Row, n: usize) -> Result<()> {
    if row.len() != n {
        return Err(Error::Deserialize(format!(
            "the row has {} columns, not {n}",
            row.len()
        )));
    }

    Ok(())
}

fn visit_map<'de, V: Visitor<'de>>(
    row: &'de Row,
    entries: Vec<(&'de str, usize)>,
    visitor: V,
) -> Result<V::Value> {
    visitor.visit_map(RowMap {
        row,
        entries,
        next: 0,
    })
}

macro_rules! only_column {
    ($($method:ident),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            only_column(self, |d| d.$method(visitor))
        }
    )*};
}

/// A row is a struct, tuple or map of its columns (see [`from_row`]), with the
/// `serde` feature.
impl<'de> Deserializer<'de> for &'de Row {
    type Error = Error;

    /// A map of the columns.
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    only_column!(
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_bytes,
        deserialize_byte_buf,
        deserialize_option,
        deserialize_unit,
        deserialize_identifier,
        deserialize_ignored_any
    );

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(RowSeq { row: self, next: 0 })
    }

    /// The columns in order, which must be `len` columns.
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        columns(self, len)?;
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    /// Each column's name and value.
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let entries = self
            .columns()
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name(), i))
            .collect();
        visit_map(self, entries, visitor)
    }

    /// Each field that has a column with the same name (ignoring ASCII case).
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let entries = fields
            .iter()
            .filter_map(|field| {
                self.columns()
                    .iter()
                    .position(|c| c.name().eq_ignore_ascii_case(field))
                    .map(|i| (*field, i))
            })
            .collect();
        visit_map(self, entries, visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        only_column(self, |d| d.deserialize_enum(name, variants, visitor))
    }
}

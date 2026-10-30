use std::marker::PhantomData;
use std::sync::Arc;

use vsql_sys::*;

use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::params::Params;
use crate::raw;
use crate::types::{FromSql, Kind, Type, Value};

/// A prepared statement, see [`Connection::prepare`]. It can be executed
/// many times, with different parameters.
pub struct Statement<'conn> {
    conn: &'conn Connection,
    stmt: SQLHSTMT,
}

impl<'conn> Statement<'conn> {
    pub(crate) fn new(conn: &'conn Connection) -> Result<Statement<'conn>> {
        Ok(Statement {
            conn,
            stmt: raw::alloc_stmt(conn.dbc())?,
        })
    }

    pub(crate) fn prepare_sql(&mut self, sql: &str) -> Result<()> {
        raw::prepare(self.stmt, sql)
    }

    pub(crate) fn exec_direct_sql(&mut self, sql: &str) -> Result<()> {
        raw::exec_direct(self.stmt, sql)
    }

    /// The number of rows that the statement changed, after closing the cursor
    /// if it was a query.
    pub(crate) fn finish_execute(&mut self) -> Result<usize> {
        let n = raw::row_count(self.stmt)?;
        if raw::num_result_cols(self.stmt)? > 0 {
            raw::close_cursor(self.stmt)?;
        }

        Ok(n)
    }

    fn run(&mut self, values: Vec<Value>) -> Result<()> {
        let mut params = values
            .iter()
            .enumerate()
            .map(|(i, v)| v.to_param(i + 1))
            .collect::<Result<Vec<_>>>()?;

        raw::execute(self.stmt, &mut params)
    }

    pub(crate) fn execute_values(&mut self, values: Vec<Value>) -> Result<usize> {
        self.run(values)?;
        self.finish_execute()
    }

    /// Executes the statement, with a `?` for each of the parameters, and
    /// returns the number of rows that it changed.
    pub fn execute<P: Params>(&mut self, params: P) -> Result<usize> {
        self.execute_values(params.to_values()?)
    }

    /// Executes the query, with a `?` for each of the parameters. The rows are
    /// an iterator, and the cursor is closed (which commits the statement,
    /// unless it's in a transaction) when they're dropped.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// let mut stmt = conn.prepare("VALUES ROW(1, 'a'), ROW(2, 'b')")?;
    /// for row in stmt.query(())? {
    ///     let row = row?;
    ///     let x: i64 = row.get(0)?;
    ///     let y: String = row.get(1)?;
    ///     println!("{x} {y}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<P: Params>(&mut self, params: P) -> Result<Rows<'_>> {
        self.run(params.to_values()?)?;

        Rows::new(self.stmt)
    }

    /// Executes the query and maps each row with `f`.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// let mut stmt = conn.prepare("VALUES 1, 2, 3")?;
    /// let xs = stmt
    ///     .query_map((), |row| row.get::<_, i64>(0))?
    ///     .collect::<vsql::Result<Vec<_>>>()?;
    /// assert_eq!(xs, [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_map<T, P, F>(&mut self, params: P, f: F) -> Result<MappedRows<'_, F>>
    where
        P: Params,
        F: FnMut(&Row) -> Result<T>,
    {
        Ok(MappedRows {
            rows: self.query(params)?,
            f,
        })
    }

    /// Executes the query and returns `f` of its first row, or
    /// [`Error::QueryReturnedNoRows`].
    pub fn query_row<T, P, F>(&mut self, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row) -> Result<T>,
    {
        match self.query(params)?.next() {
            Some(Ok(row)) => f(&row),
            Some(Err(e)) => Err(e),
            None => Err(Error::QueryReturnedNoRows),
        }
    }

    /// The connection of the statement.
    pub fn connection(&self) -> &'conn Connection {
        self.conn
    }
}

impl std::fmt::Debug for Statement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Statement").finish_non_exhaustive()
    }
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        let _ = raw::free_handle(SQL_HANDLE_STMT, self.stmt);
    }
}

/// A column of a query's rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    name: String,
    ty: Type,
    nullable: bool,
    size: i32,
    digits: i16,
}

impl Column {
    /// The name of the column. Names that weren't quoted are upper case.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The SQL type of the column.
    pub fn ty(&self) -> Type {
        self.ty
    }

    /// Whether the column can be NULL.
    pub fn is_nullable(&self) -> bool {
        self.nullable
    }

    /// The maximum number of characters of a CHARACTER or CHARACTER VARYING.
    pub fn length(&self) -> Option<usize> {
        match self.ty {
            // The size is in octets, with 4 for each character.
            Type::Character | Type::CharacterVarying if self.size > 0 => {
                Some(self.size as usize / 4)
            }
            _ => None,
        }
    }

    /// The precision and scale of a DECIMAL or NUMERIC.
    pub fn precision_scale(&self) -> Option<(u32, u32)> {
        match self.ty {
            Type::Decimal | Type::Numeric => Some((self.size as u32, self.digits as u32)),
            _ => None,
        }
    }
}

/// The rows of a query, see [`Statement::query`]. The cursor is closed when
/// they're dropped (or after the last row).
pub struct Rows<'stmt> {
    stmt: SQLHSTMT,
    columns: Arc<[Column]>,
    kinds: Vec<Kind>,
    buf: Vec<u8>,
    open: bool,
    _stmt: PhantomData<&'stmt mut ()>,
}

impl Rows<'_> {
    fn new<'stmt>(stmt: SQLHSTMT) -> Result<Rows<'stmt>> {
        let mut rows = Rows {
            stmt,
            columns: Arc::new([]),
            kinds: Vec::new(),
            buf: Vec::new(),
            open: true,
            _stmt: PhantomData,
        };

        // Dropped if it fails, which closes the cursor.
        let n = raw::num_result_cols(stmt)?;
        let mut columns = Vec::with_capacity(n);
        for i in 1..=n {
            let d = raw::describe_col(stmt, i)?;
            columns.push(Column {
                name: d.name,
                ty: Type::from_code(d.data_type),
                nullable: d.nullable != SQL_NO_NULLS,
                size: d.column_size,
                digits: d.decimal_digits,
            });
        }
        rows.kinds = columns.iter().map(|c| Kind::of(c.ty)).collect();
        rows.columns = columns.into();

        Ok(rows)
    }

    /// The columns of the rows.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The names of the columns.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(Column::name).collect()
    }

    fn close(&mut self) -> Result<()> {
        if !self.open {
            return Ok(());
        }
        self.open = false;

        raw::close_cursor(self.stmt)
    }

    fn fetch(&mut self) -> Result<Option<Row>> {
        if !self.open {
            return Ok(None);
        }
        if !raw::fetch(self.stmt)? {
            self.close()?;
            return Ok(None);
        }

        let mut values = Vec::with_capacity(self.columns.len());
        for (i, (column, kind)) in self.columns.iter().zip(&self.kinds).enumerate() {
            let n = i + 1;
            let value = match kind {
                Kind::Integer => raw::get_i64(self.stmt, n)?.map(Value::Integer),
                Kind::Double => raw::get_f64(self.stmt, n)?.map(Value::Double),
                Kind::Real => raw::get_f32(self.stmt, n)?.map(|f| Value::Double(f64::from(f))),
                Kind::Text => match raw::get_string(self.stmt, n, &mut self.buf)? {
                    Some(s) => Some(Value::parse(s, column.ty).map_err(|source| {
                        Error::FromSqlConversion {
                            index: i,
                            name: column.name.clone(),
                            ty: column.ty,
                            source,
                        }
                    })?),
                    None => None,
                },
            };
            values.push(value.unwrap_or(Value::Null));
        }

        Ok(Some(Row {
            values,
            columns: Arc::clone(&self.columns),
        }))
    }
}

impl std::fmt::Debug for Rows<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rows")
            .field("columns", &self.columns)
            .finish_non_exhaustive()
    }
}

impl Iterator for Rows<'_> {
    type Item = Result<Row>;

    fn next(&mut self) -> Option<Result<Row>> {
        match self.fetch() {
            Ok(row) => row.map(Ok),
            Err(e) => {
                // The rows end after an error.
                let _ = self.close();
                Some(Err(e))
            }
        }
    }
}

impl Drop for Rows<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// The rows of a query mapped by a function, see [`Statement::query_map`].
pub struct MappedRows<'stmt, F> {
    rows: Rows<'stmt>,
    f: F,
}

impl<T, F> Iterator for MappedRows<'_, F>
where
    F: FnMut(&Row) -> Result<T>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        let f = &mut self.f;
        self.rows.next().map(|row| row.and_then(|row| f(&row)))
    }
}

/// A row of a query.
#[derive(Debug, Clone)]
pub struct Row {
    values: Vec<Value>,
    columns: Arc<[Column]>,
}

impl Row {
    /// The value of a column (by its index, starting at 0, or its name) as
    /// `T`. NULL is only a `T` that's an `Option`.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.query_row("VALUES ROW(1, CAST(NULL AS INTEGER))", (), |row| {
    ///     let x: i64 = row.get(0)?;
    ///     let y: Option<i64> = row.get("COL2")?;
    ///     assert_eq!((x, y), (1, None));
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn get<I: RowIndex, T: FromSql>(&self, index: I) -> Result<T> {
        let i = index.index(&self.columns)?;
        let column = &self.columns[i];

        T::from_sql(&self.values[i]).map_err(|source| Error::FromSqlConversion {
            index: i,
            name: column.name.clone(),
            ty: column.ty,
            source,
        })
    }

    /// The value of a column (by its index, starting at 0, or its name).
    pub fn get_ref<I: RowIndex>(&self, index: I) -> Result<&Value> {
        Ok(&self.values[index.index(&self.columns)?])
    }

    /// The columns of the row.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The number of columns.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The values of the columns.
    pub fn into_values(self) -> Vec<Value> {
        self.values
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for &str {}
}

/// The index of a column: its index (starting at 0), or its name (ignoring
/// ASCII case).
pub trait RowIndex: sealed::Sealed {
    #[doc(hidden)]
    fn index(&self, columns: &[Column]) -> Result<usize>;
}

impl RowIndex for usize {
    fn index(&self, columns: &[Column]) -> Result<usize> {
        if *self < columns.len() {
            Ok(*self)
        } else {
            Err(Error::InvalidColumnIndex(*self))
        }
    }
}

impl RowIndex for &str {
    fn index(&self, columns: &[Column]) -> Result<usize> {
        columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(self))
            .ok_or_else(|| Error::InvalidColumnName(self.to_string()))
    }
}

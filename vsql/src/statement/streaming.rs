//! The rows of a query that are read when they're asked for, so that a large
//! value can be read in pieces (see Statement::query_streaming). Nothing is
//! bound, each row is fetched with SQLFetch, and each column is read with
//! SQLGetData.

use std::io;
use std::marker::PhantomData;

use vsql_sys::*;

use super::{read_value, Column, RowIndex};
use crate::error::{Error, Result};
use crate::raw;
use crate::read_param::PIECE;
use crate::types::{FromSql, Kind};

/// The size of the first piece of a column that's read with a reader. A value
/// that's longer is read in larger pieces.
const FIRST_PIECE: usize = 1 << 16;

/// The rows of a query that are read when they're asked for, see
/// [`Statement::query_streaming`](crate::Statement::query_streaming). The
/// cursor is closed when they're dropped (or after the last row).
pub struct StreamingRows<'stmt> {
    stmt: SQLHSTMT,
    columns: Vec<Column>,
    kinds: Vec<Kind>,
    open: bool,
    _stmt: PhantomData<&'stmt mut ()>,
}

impl<'stmt> StreamingRows<'stmt> {
    pub(super) fn new(stmt: SQLHSTMT) -> Result<StreamingRows<'stmt>> {
        let mut rows = StreamingRows {
            stmt,
            columns: Vec::new(),
            kinds: Vec::new(),
            // A statement that isn't a query has no cursor, so it has no rows.
            open: false,
            _stmt: PhantomData,
        };
        let n = raw::num_result_cols(stmt)?;
        rows.open = n > 0;
        for i in 1..=n {
            rows.columns.push(Column::describe(stmt, i)?);
        }
        rows.kinds = rows.columns.iter().map(|c| Kind::of(c.ty)).collect();

        Ok(rows)
    }

    /// The columns of the rows.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The next row, or None after the last row.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<StreamingRow<'_>>> {
        if !self.open {
            return Ok(None);
        }
        // The cursor is closed when the rows are dropped, after an error.
        if !raw::fetch(self.stmt)? {
            self.close()?;
            return Ok(None);
        }

        Ok(Some(StreamingRow {
            stmt: self.stmt,
            columns: &self.columns,
            kinds: &self.kinds,
            partial: None,
            buf: Vec::new(),
        }))
    }

    fn close(&mut self) -> Result<()> {
        if !self.open {
            return Ok(());
        }
        self.open = false;
        raw::close_cursor(self.stmt)
    }
}

impl std::fmt::Debug for StreamingRows<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingRows")
            .field("columns", &self.columns)
            .finish_non_exhaustive()
    }
}

impl Drop for StreamingRows<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// The current row of [`StreamingRows`]. Each column is read when it's asked
/// for, and it can be read more than once.
pub struct StreamingRow<'rows> {
    stmt: SQLHSTMT,
    columns: &'rows [Column],
    kinds: &'rows [Kind],
    // The column (starting at 1) that a reader didn't read to the end, and
    // its target type.
    partial: Option<(usize, SQLSMALLINT)>,
    buf: Vec<u8>,
}

impl StreamingRow<'_> {
    /// The columns of the row.
    pub fn columns(&self) -> &[Column] {
        self.columns
    }

    /// Reads a column (by its index, starting at 0, or its name) as `T`, see
    /// [`Row::get`](crate::Row::get).
    pub fn get<I: RowIndex, T: FromSql>(&mut self, index: I) -> Result<T> {
        let i = index.index(self.columns)?;
        self.restart(i + 1)?;
        let column = &self.columns[i];
        let value = read_value(self.stmt, i, column, self.kinds[i], &mut self.buf)?;

        T::from_sql(&value).map_err(|source| Error::FromSqlConversion {
            index: i,
            name: column.name.clone(),
            ty: column.ty,
            source,
        })
    }

    /// Reads a column (by its index, starting at 0, or its name) in pieces,
    /// which is None for a NULL. A binary string (such as a BLOB) is its
    /// octets, and any other value (such as a CLOB) is the UTF-8 of the
    /// string that vsql gives for it (such as `DATE '2024-01-02'` for a
    /// DATE).
    ///
    /// See [`Statement::query_streaming`](crate::Statement::query_streaming).
    pub fn reader<I: RowIndex>(&mut self, index: I) -> Result<Option<ColumnReader<'_>>> {
        let i = index.index(self.columns)?;
        self.restart(i + 1)?;
        let target_type = match self.kinds[i] {
            Kind::Binary => SQL_VARBINARY,
            _ => SQL_CHAR,
        };
        let mut reader = ColumnReader {
            stmt: self.stmt,
            column: &self.columns[i],
            partial: &mut self.partial,
            n: i + 1,
            target_type,
            // A small value only needs a small buffer.
            buf: vec![0; FIRST_PIECE],
            start: 0,
            end: 0,
            last: false,
        };
        // The first piece says whether it's NULL.
        if !reader.fill()? {
            return Ok(None);
        }

        Ok(Some(reader))
    }

    /// A column that a reader didn't finish is read to the end, so that it's
    /// read from the start again.
    fn restart(&mut self, n: usize) -> Result<()> {
        if let Some((partial, target_type)) = self.partial.take() {
            if partial == n {
                let mut buf = vec![0; PIECE];
                while let Some((_, false)) = raw::get_piece(self.stmt, n, target_type, &mut buf)? {}
            }
        }

        Ok(())
    }
}

impl std::fmt::Debug for StreamingRow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingRow")
            .field("columns", &self.columns)
            .finish_non_exhaustive()
    }
}

/// A column of a [`StreamingRow`] that's read in pieces, see
/// [`StreamingRow::reader`].
pub struct ColumnReader<'row> {
    stmt: SQLHSTMT,
    column: &'row Column,
    partial: &'row mut Option<(usize, SQLSMALLINT)>,
    n: usize,
    target_type: SQLSMALLINT,
    buf: Vec<u8>,
    start: usize,
    end: usize,
    last: bool,
}

impl ColumnReader<'_> {
    /// Reads the next piece into the buffer, it's false for a NULL.
    fn fill(&mut self) -> Result<bool> {
        let piece = raw::get_piece(self.stmt, self.n, self.target_type, &mut self.buf)?;
        let Some((len, last)) = piece else {
            return Ok(false);
        };
        self.start = 0;
        self.end = len;
        self.last = last;
        if !last && self.buf.len() < PIECE {
            self.buf.resize(PIECE, 0);
        }

        Ok(true)
    }
}

impl io::Read for ColumnReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.start == self.end && !self.last {
            self.fill().map_err(io::Error::other)?;
        }
        let n = buf.len().min(self.end - self.start);
        buf[..n].copy_from_slice(&self.buf[self.start..self.start + n]);
        self.start += n;

        Ok(n)
    }
}

impl std::fmt::Debug for ColumnReader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnReader")
            .field("column", &self.column.name)
            .finish_non_exhaustive()
    }
}

impl Drop for ColumnReader<'_> {
    fn drop(&mut self) {
        // The next read of the column continues from here, unless it's read
        // to the end first.
        if !self.last {
            *self.partial = Some((self.n, self.target_type));
        }
    }
}

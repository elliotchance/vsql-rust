//! A parameter that's read from an `impl Read` when the statement executes.

use std::cell::RefCell;
use std::fmt;
use std::io::Read;

use crate::error::{Error, Result};
use crate::params::Arg;
use crate::types::{ToSql, Value};

/// A CHARACTER LARGE OBJECT or BINARY LARGE OBJECT parameter that's read
/// from a reader (such as a file) in pieces when the statement executes, so
/// the whole value is never in memory in Rust.
///
/// It's read once, so a statement that's executed again with the same
/// `ReadParam` gets what's left (nothing, after the end). An error from the
/// reader is [`Error::ToSqlConversion`], and the statement isn't executed.
///
/// ```
/// use vsql::ReadParam;
///
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE files (name VARCHAR(100), body BLOB)", ())?;
/// let file: &[u8] = b"the contents of a large file";
/// conn.execute(
///     "INSERT INTO files (name, body) VALUES (?, ?)",
///     vsql::params!["a.txt", ReadParam::binary(file)],
/// )?;
/// # Ok(())
/// # }
/// ```
pub struct ReadParam<R> {
    binary: bool,
    reader: RefCell<R>,
}

impl<R: Read> ReadParam<R> {
    /// A CHARACTER LARGE OBJECT, which must be UTF-8.
    pub fn text(reader: R) -> ReadParam<R> {
        ReadParam {
            binary: false,
            reader: RefCell::new(reader),
        }
    }

    /// A BINARY LARGE OBJECT.
    pub fn binary(reader: R) -> ReadParam<R> {
        ReadParam {
            binary: true,
            reader: RefCell::new(reader),
        }
    }

    /// The reader.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}

impl<R> fmt::Debug for ReadParam<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadParam")
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl<R: Read> ToSql for ReadParam<R> {
    /// The whole value, which is read into memory. A statement's parameter is
    /// read in pieces instead.
    fn to_sql(&self) -> Result<Value> {
        let mut value = Vec::new();
        self.reader
            .borrow_mut()
            .read_to_end(&mut value)
            .map_err(|e| Error::ToSqlConversion(Box::new(e)))?;
        if self.binary {
            return Ok(Value::Binary(value));
        }
        String::from_utf8(value)
            .map(Value::Text)
            .map_err(|e| Error::ToSqlConversion(Box::new(e)))
    }

    fn to_arg(&self) -> Result<Arg<'_>> {
        Ok(Arg::Stream {
            binary: self.binary,
            reader: &self.reader,
        })
    }
}

/// The most octets of each piece of a value that's read or written in pieces.
/// libvsql copies the value so far (or what's left of it) for each piece, so
/// the time grows with the square of the number of pieces: 8 MiB pieces are
/// about as fast as a whole value for 100 MB.
pub(crate) const PIECE: usize = 8 << 20;

/// Gives the value of a reader in pieces, with `put` (which is called at least
/// once, since a value that isn't given is an error).
pub(crate) fn send(
    reader: &RefCell<dyn Read + '_>,
    put: &mut dyn FnMut(&[u8]) -> Result<()>,
) -> Result<()> {
    let mut reader = reader.borrow_mut();
    let mut buf = Vec::new();
    let mut first = true;
    loop {
        buf.clear();
        let n = (&mut *reader)
            .take(PIECE as u64)
            .read_to_end(&mut buf)
            .map_err(|e| Error::ToSqlConversion(Box::new(e)))?;
        if n > 0 || first {
            put(&buf)?;
        }
        first = false;
        if n < PIECE {
            return Ok(());
        }
    }
}

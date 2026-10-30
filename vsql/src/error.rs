use std::error;
use std::fmt;
use std::path::PathBuf;

use crate::types::{FromSqlError, Type};

/// A `Result` with an [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error from vsql, or from converting a value.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error from vsql, with its SQLSTATE (such as `"42P01"` for an
    /// undefined table) and message.
    Sql { sqlstate: String, message: String },

    /// The path of a database isn't valid UTF-8.
    InvalidPath(PathBuf),

    /// A parameter (starting at 1) can't be sent as its SQL value.
    InvalidParameter { index: usize, message: String },

    /// A Rust value couldn't be converted to an SQL value (see
    /// [`ToSql`](crate::ToSql)).
    ToSqlConversion(Box<dyn error::Error + Send + Sync>),

    /// There's no column at the index (starting at 0).
    InvalidColumnIndex(usize),

    /// There's no column with the name.
    InvalidColumnName(String),

    /// The value of a column (starting at 0) couldn't be converted to the Rust
    /// type.
    FromSqlConversion {
        index: usize,
        name: String,
        ty: Type,
        source: FromSqlError,
    },

    /// `query_row` returned no rows.
    QueryReturnedNoRows,
}

impl Error {
    /// The SQLSTATE of an error from vsql, such as `"42P01"` for an undefined
    /// table.
    pub fn sqlstate(&self) -> Option<&str> {
        match self {
            Error::Sql { sqlstate, .. } => Some(sqlstate),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sql { sqlstate, message } => write!(f, "{message} (SQLSTATE {sqlstate})"),
            Error::InvalidPath(path) => write!(f, "the path isn't valid UTF-8: {}", path.display()),
            Error::InvalidParameter { index, message } => write!(f, "parameter {index}: {message}"),
            Error::ToSqlConversion(e) => e.fmt(f),
            Error::InvalidColumnIndex(index) => write!(f, "there's no column {index}"),
            Error::InvalidColumnName(name) => write!(f, "there's no column named {name}"),
            Error::FromSqlConversion {
                index,
                name,
                ty,
                source,
            } => write!(f, "column {index} ({name}, {ty}): {source}"),
            Error::QueryReturnedNoRows => f.write_str("the query returned no rows"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::FromSqlConversion { source, .. } => Some(source),
            Error::ToSqlConversion(e) => Some(&**e),
            _ => None,
        }
    }
}

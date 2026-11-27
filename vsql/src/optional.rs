use crate::error::{Error, Result};

/// Turns [`Error::QueryReturnedNoRows`] into `Ok(None)`, such as for
/// [`query_row`](crate::Connection::query_row) when a row is optional.
///
/// ```
/// use vsql::OptionalExtension;
///
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
/// let x: Option<i64> = conn
///     .query_row("SELECT x FROM foo", (), |row| row.get(0))
///     .optional()?;
/// assert_eq!(x, None);
/// # Ok(())
/// # }
/// ```
pub trait OptionalExtension<T> {
    /// `Ok(None)` for [`Error::QueryReturnedNoRows`], `Ok(Some(_))` for a
    /// value, and any other error.
    fn optional(self) -> Result<Option<T>>;
}

impl<T> OptionalExtension<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

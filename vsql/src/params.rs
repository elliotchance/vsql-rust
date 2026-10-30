use crate::error::Result;
use crate::types::{ToSql, Value};

/// The parameters of a statement, one for each `?`.
///
/// They can be `()` (no parameters), a tuple (such as `(1, "a")`), an array or
/// slice of one type, a slice of [`&dyn ToSql`](ToSql) (see
/// [`params!`](crate::params!)), or an iterator (see [`params_from_iter`]).
pub trait Params {
    #[doc(hidden)]
    fn to_values(self) -> Result<Vec<Value>>;
}

impl Params for () {
    fn to_values(self) -> Result<Vec<Value>> {
        Ok(Vec::new())
    }
}

impl<T: ToSql> Params for &[T] {
    fn to_values(self) -> Result<Vec<Value>> {
        self.iter().map(ToSql::to_sql).collect()
    }
}

impl<T: ToSql, const N: usize> Params for [T; N] {
    fn to_values(self) -> Result<Vec<Value>> {
        self.iter().map(ToSql::to_sql).collect()
    }
}

impl<T: ToSql, const N: usize> Params for &[T; N] {
    fn to_values(self) -> Result<Vec<Value>> {
        self.iter().map(ToSql::to_sql).collect()
    }
}

impl<T: ToSql> Params for &Vec<T> {
    fn to_values(self) -> Result<Vec<Value>> {
        self.iter().map(ToSql::to_sql).collect()
    }
}

macro_rules! params_tuple {
    ($($name:ident),+) => {
        impl<$($name: ToSql),+> Params for ($($name,)+) {
            #[allow(non_snake_case)]
            fn to_values(self) -> Result<Vec<Value>> {
                let ($($name,)+) = self;
                Ok(vec![$($name.to_sql()?),+])
            }
        }
    };
}

params_tuple!(A);
params_tuple!(A, B);
params_tuple!(A, B, C);
params_tuple!(A, B, C, D);
params_tuple!(A, B, C, D, E);
params_tuple!(A, B, C, D, E, F);
params_tuple!(A, B, C, D, E, F, G);
params_tuple!(A, B, C, D, E, F, G, H);
params_tuple!(A, B, C, D, E, F, G, H, I);
params_tuple!(A, B, C, D, E, F, G, H, I, J);
params_tuple!(A, B, C, D, E, F, G, H, I, J, K);
params_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

/// Parameters from an iterator, see [`params_from_iter`].
pub struct ParamsFromIter<I>(I);

impl<I> Params for ParamsFromIter<I>
where
    I: IntoIterator,
    I::Item: ToSql,
{
    fn to_values(self) -> Result<Vec<Value>> {
        self.0.into_iter().map(|v| v.to_sql()).collect()
    }
}

/// Parameters from an iterator, for a number of parameters that's only known
/// at runtime.
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// let ids = vec![1, 2, 3];
/// let placeholders = vec!["?"; ids.len()].join(", ");
/// let sql = format!("VALUES ROW({placeholders})");
/// conn.query_row(&sql, vsql::params_from_iter(&ids), |_| Ok(()))?;
/// # Ok(())
/// # }
/// ```
pub fn params_from_iter<I>(iter: I) -> ParamsFromIter<I>
where
    I: IntoIterator,
    I::Item: ToSql,
{
    ParamsFromIter(iter)
}

/// Parameters of different types, as a `&[&dyn ToSql]`.
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE foo (x INTEGER, y VARCHAR(10))", ())?;
/// conn.execute("INSERT INTO foo (x, y) VALUES (?, ?)", vsql::params![1, "a"])?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! params {
    () => {
        &[] as &[&dyn $crate::ToSql]
    };
    ($($param:expr),+ $(,)?) => {
        &[$(&$param as &dyn $crate::ToSql),+] as &[&dyn $crate::ToSql]
    };
}

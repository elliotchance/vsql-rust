use std::cell::RefCell;
use std::io::Read;

use crate::error::Result;
use crate::types::{ToSql, Value};

/// A parameter as it's sent: a value, or a value that's read in pieces when
/// the statement executes (see [`ReadParam`](crate::ReadParam)).
#[doc(hidden)]
pub enum Arg<'a> {
    Value(Value),
    Stream {
        binary: bool,
        reader: &'a RefCell<dyn Read + 'a>,
    },
}

/// The parameters of a statement, one for each `?`.
///
/// They can be `()` (no parameters), a tuple (such as `(1, "a")`), an array or
/// slice of one type, a slice of [`&dyn ToSql`](ToSql) (see
/// [`params!`](crate::params!)), or an iterator (see [`params_from_iter`]).
pub trait Params {
    /// Calls `f` with the parameters, which may borrow from them.
    #[doc(hidden)]
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>;
}

/// The parameters of a slice.
fn args<T: ToSql>(values: &[T]) -> Result<Vec<Arg<'_>>> {
    values.iter().map(ToSql::to_arg).collect()
}

impl Params for () {
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        f(Vec::new())
    }
}

impl<P: ToSql> Params for &[P] {
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        f(args(self)?)
    }
}

impl<P: ToSql, const N: usize> Params for [P; N] {
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        f(args(&self)?)
    }
}

impl<P: ToSql, const N: usize> Params for &[P; N] {
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        f(args(self)?)
    }
}

impl<P: ToSql> Params for &Vec<P> {
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        f(args(self)?)
    }
}

macro_rules! params_tuple {
    ($($name:ident),+) => {
        impl<$($name: ToSql),+> Params for ($($name,)+) {
            #[allow(non_snake_case)]
            fn with_args<Out, Call>(self, f: Call) -> Result<Out>
            where
                Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
            {
                let ($($name,)+) = &self;
                f(vec![$($name.to_arg()?),+])
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
    fn with_args<Out, Call>(self, f: Call) -> Result<Out>
    where
        Call: FnOnce(Vec<Arg<'_>>) -> Result<Out>,
    {
        let values: Vec<I::Item> = self.0.into_iter().collect();
        f(args(&values)?)
    }
}

/// Parameters from an iterator, for a number of parameters that's only known
/// at runtime.
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// let ids = vec![1, 2, 3];
/// let placeholders = vec!["CAST(? AS INTEGER)"; ids.len()].join(", ");
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

use crate::error::{Error, Result};
use crate::sql;
use crate::types::{ToSql, Value};

/// The parameters of a statement, one for each `?` or `:name`.
///
/// They can be `()` (no parameters), a tuple (such as `(1, "a")`), an array or
/// slice of one type, a slice of [`&dyn ToSql`](ToSql) (see
/// [`params!`](crate::params!)), an iterator (see [`params_from_iter`]), or
/// the values of names (see [`named_params!`](crate::named_params!)).
pub trait Params {
    /// The names of the values, which are in the same order as
    /// [`to_values`](Params::to_values), or `None` if the values are
    /// positional.
    #[doc(hidden)]
    fn names(&self) -> Option<Vec<String>> {
        None
    }

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

/// The values of named parameters (`:name`), see
/// [`named_params!`](crate::named_params!). A name can be given with or
/// without its `:`.
impl Params for &[(&str, &dyn ToSql)] {
    fn names(&self) -> Option<Vec<String>> {
        Some(self.iter().map(|(name, _)| name.to_string()).collect())
    }

    fn to_values(self) -> Result<Vec<Value>> {
        self.iter().map(|(_, v)| v.to_sql()).collect()
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

/// Named parameters (`:name`), as a `&[(&str, &dyn ToSql)]`. Each name is a
/// parameter, which can be used more than once in the statement.
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE foo (x INTEGER, y VARCHAR(10))", ())?;
/// conn.execute(
///     "INSERT INTO foo (x, y) VALUES (:x, :y)",
///     vsql::named_params! { ":x": 1, ":y": "a" },
/// )?;
///
/// let n: i64 = conn.query_row(
///     "SELECT COUNT(*) FROM foo WHERE x = :x OR x = :x + 1",
///     vsql::named_params! { ":x": 1 },
///     |row| row.get(0),
/// )?;
/// assert_eq!(n, 1);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! named_params {
    () => {
        &[] as &[(&str, &dyn $crate::ToSql)]
    };
    ($($name:literal : $param:expr),+ $(,)?) => {
        &[$(($name, &$param as &dyn $crate::ToSql)),+] as &[(&str, &dyn $crate::ToSql)]
    };
}

/// The named parameters (`:name`) of a statement, which are given to libvsql
/// as `?`.
#[derive(Debug)]
pub(crate) struct NamedParams {
    /// The name of each parameter, or `None` for a `?`. A name is a parameter
    /// once, however many times it's used.
    names: Vec<Option<String>>,
    /// The parameter (an index of names) of each `?` given to libvsql.
    markers: Vec<usize>,
}

impl NamedParams {
    /// The SQL with each `:name` replaced by `?`, and its parameters, or
    /// `None` if it doesn't have a `:name`. A `:name` in a string, a quoted
    /// identifier or a `/* */` comment isn't a parameter (the `--` comments
    /// have been removed).
    pub(crate) fn parse(sql: &str) -> Option<(String, NamedParams)> {
        let bytes = sql.as_bytes();
        let is_name = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let mut out = String::with_capacity(sql.len());
        let mut params = NamedParams {
            names: Vec::new(),
            markers: Vec::new(),
        };
        // The start of the text that hasn't been copied to out yet.
        let mut start = 0;
        let mut i = 0;
        while i < bytes.len() {
            if let Some(end) = sql::skip_quoted(sql, i) {
                i = end;
            } else if bytes[i] == b'?' {
                params.markers.push(params.names.len());
                params.names.push(None);
                i += 1;
            } else if bytes[i] == b':'
                && bytes
                    .get(i + 1)
                    .is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_')
            {
                let end = i + 1 + bytes[i + 1..].iter().take_while(|&&b| is_name(b)).count();
                let name = &sql[i + 1..end];
                let param = match params.index(name) {
                    Some(param) => param,
                    None => {
                        params.names.push(Some(name.to_string()));
                        params.names.len() - 1
                    }
                };
                params.markers.push(param);
                out.push_str(&sql[start..i]);
                out.push('?');
                start = end;
                i = end;
            } else {
                i += 1;
            }
        }
        if start == 0 {
            return None;
        }
        out.push_str(&sql[start..]);

        Some((out, params))
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n.as_deref() == Some(name))
    }

    /// The number of parameters.
    pub(crate) fn len(&self) -> usize {
        self.names.len()
    }

    /// The value of each `?` given to libvsql, from the values of the names
    /// (`names` is `Some`), or of the parameters in order.
    pub(crate) fn bind(
        &self,
        names: Option<Vec<String>>,
        values: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let params = match names {
            Some(names) => {
                let mut params = vec![None; self.names.len()];
                for (name, value) in names.into_iter().zip(values) {
                    let name = strip_colon(&name);
                    let param = self
                        .index(name)
                        .ok_or_else(|| Error::InvalidParameterName(name.to_string()))?;
                    params[param] = Some(value);
                }
                params
            }
            // A different number of values is left for libvsql to report.
            None if values.len() != self.names.len() => return Ok(values),
            None => values.into_iter().map(Some).collect(),
        };

        self.markers
            .iter()
            .map(|&param| {
                params[param].clone().ok_or_else(|| {
                    let message = match &self.names[param] {
                        Some(name) => format!("there's no value for :{name}"),
                        None => "a ? can't be used with named parameters".to_string(),
                    };
                    Error::InvalidParameter {
                        index: param + 1,
                        message,
                    }
                })
            })
            .collect()
    }
}

/// A name of a parameter without its `:`.
pub(crate) fn strip_colon(name: &str) -> &str {
    name.strip_prefix(':').unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert!(NamedParams::parse("VALUES ?, ':x', \":x\", /* :x */ 1").is_none());

        let (sql, params) = NamedParams::parse(
            "VALUES ROW(:a, ?, :b_2, ':a', \":a\", /* :a */ :a, :A, a:b, :1, ::a)",
        )
        .unwrap();
        assert_eq!(
            sql,
            "VALUES ROW(?, ?, ?, ':a', \":a\", /* :a */ ?, ?, a?, :1, :?)"
        );
        assert_eq!(
            params.names,
            [
                Some("a".to_string()),
                None,
                Some("b_2".to_string()),
                Some("A".to_string()),
                Some("b".to_string()),
            ]
        );
        assert_eq!(params.markers, [0, 1, 2, 0, 3, 4, 0]);
    }
}

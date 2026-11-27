use std::sync::OnceLock;

use vsql_sys::*;

use crate::connection::Connection;
use crate::error::Result;
use crate::raw;

/// How libvsql was built, see [`build_info`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BuildInfo {
    /// The vsql release, such as `"v1.7.58"` (empty if it isn't a release).
    pub version: String,
    /// The git commit of the release (empty if it isn't a release).
    pub commit: String,
    /// The date it was built, such as `"2026-11-12"` (UTC).
    pub date: String,
    /// The version of V that compiled vsql to C, such as `"0.5.2"`.
    pub v_version: String,
    /// The git commit of V.
    pub v_commit: String,
    /// The C compiler, such as `"clang"`.
    pub c_compiler: String,
    /// The operating system, such as `"macos"`.
    pub operating_system: String,
    /// The machine, such as `"arm64"`.
    pub machine: String,
}

struct Version {
    version: String,
    number: i32,
    build: BuildInfo,
}

fn load() -> Result<Version> {
    let conn = Connection::open_in_memory()?;
    let info = |info_type| raw::get_info(conn.dbc(), info_type);

    let (version, number) = parse_dbms_version(info(SQL_DBMS_VERSION)?);

    let build = BuildInfo {
        version: info(SQL_VSQL_BUILD_VERSION)?,
        commit: info(SQL_VSQL_BUILD_COMMIT)?,
        date: info(SQL_VSQL_BUILD_DATE)?,
        v_version: info(SQL_VSQL_BUILD_V_VERSION)?,
        v_commit: info(SQL_VSQL_BUILD_V_COMMIT)?,
        c_compiler: info(SQL_VSQL_BUILD_C_COMPILER)?,
        operating_system: info(SQL_VSQL_OPERATING_SYSTEM)?,
        machine: info(SQL_VSQL_MACHINE)?,
    };

    Ok(Version {
        version,
        number,
        build,
    })
}

/// The version and its number from SQL_DBMS_VERSION, which is "01.07.0058" for
/// 1.7.58. The number is 0 if it isn't a version.
fn parse_dbms_version(dbms_version: String) -> (String, i32) {
    let parts: Vec<u32> = dbms_version
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    match parts[..] {
        [major, minor, patch] => (
            format!("{major}.{minor}.{patch}"),
            (major * 1_000_000 + minor * 1_000 + patch) as i32,
        ),
        _ => (dbms_version, 0),
    }
}

fn get() -> &'static Version {
    static VERSION: OnceLock<Version> = OnceLock::new();
    VERSION.get_or_init(|| load().expect("can't read the version of libvsql"))
}

/// The version of vsql (libvsql), such as `"1.7.58"`.
///
/// ```
/// let version = vsql::version();
/// assert!(version.starts_with("1."));
/// ```
pub fn version() -> &'static str {
    &get().version
}

/// The version of vsql (libvsql) as a number, such as `1007058` for 1.7.58.
pub fn version_number() -> i32 {
    get().number
}

/// How libvsql was built.
///
/// ```
/// let info = vsql::build_info();
/// println!("vsql {} ({}) built with V {}", info.version, info.date, info.v_version);
/// ```
pub fn build_info() -> &'static BuildInfo {
    &get().build
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbms_versions() {
        let version = |s: &str| parse_dbms_version(s.to_string());
        assert_eq!(version("01.07.0058"), ("1.7.58".to_string(), 1_007_058));
        assert_eq!(version("1.2"), ("1.2".to_string(), 0));
        assert_eq!(version("dev"), ("dev".to_string(), 0));
    }
}

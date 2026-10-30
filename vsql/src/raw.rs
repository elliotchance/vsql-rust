// raw.rs is the only code that calls libvsql (see vsql-sys). Each function is
// a thin, safe wrapper around a libvsql routine, with handles as integers, and
// an error from the first diagnostic record of the handle.

use std::ptr;
use std::sync::Once;

use vsql_sys::*;

use crate::error::{Error, Result};

/// The error for a return code (SQL_NO_DATA isn't one), from the diagnostics
/// of the handle.
fn check(rc: SQLRETURN, handle_type: SQLSMALLINT, handle: SQLINTEGER) -> Result<()> {
    match rc {
        SQL_SUCCESS | SQL_SUCCESS_WITH_INFO | SQL_NO_DATA => Ok(()),
        SQL_INVALID_HANDLE => Err(sql_error("HY000", "invalid handle")),
        _ => Err(diagnostics(handle_type, handle)),
    }
}

fn sql_error(sqlstate: &str, message: &str) -> Error {
    Error::Sql {
        sqlstate: sqlstate.to_string(),
        message: message.to_string(),
    }
}

/// The first diagnostic record of the last routine called with a handle.
fn diagnostics(handle_type: SQLSMALLINT, handle: SQLINTEGER) -> Error {
    let mut sqlstate = [0u8; 6];
    let mut message = vec![0u8; 512];
    loop {
        let mut length: SQLSMALLINT = 0;
        let rc = unsafe {
            SQLGetDiagRec(
                handle_type,
                handle,
                1,
                sqlstate.as_mut_ptr(),
                ptr::null_mut(),
                message.as_mut_ptr(),
                message.len() as SQLSMALLINT,
                &mut length,
            )
        };
        if rc != SQL_SUCCESS && rc != SQL_SUCCESS_WITH_INFO {
            return sql_error("HY000", "unknown error");
        }

        // The message was truncated.
        let length = length as usize;
        if length >= message.len() {
            message.resize(length + 1, 0);
            continue;
        }

        return Error::Sql {
            sqlstate: String::from_utf8_lossy(&sqlstate[..5]).into_owned(),
            message: String::from_utf8_lossy(&message[..length]).into_owned(),
        };
    }
}

/// A length for libvsql, which is an error if it's too long.
fn length<T: TryFrom<usize>>(len: usize, what: &str) -> Result<T> {
    T::try_from(len).map_err(|_| sql_error("HY090", &format!("the {what} is too long")))
}

/// libvsql initializes itself (and its garbage collector) in the first routine
/// that's called, which isn't thread safe: another thread that calls a routine
/// at the same time can use it before it's initialized (and abort). So the
/// first routine is called once, before any other. Every connection starts
/// with alloc_env, so it's called there.
fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        let mut env = 0;
        if SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &mut env) == SQL_SUCCESS {
            SQLFreeHandle(SQL_HANDLE_ENV, env);
        }
    });
}

pub fn alloc_env() -> Result<SQLHENV> {
    init();
    let mut env = 0;
    let rc = unsafe { SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &mut env) };
    if rc != SQL_SUCCESS {
        return Err(sql_error("HY001", "can't allocate an environment"));
    }

    Ok(env)
}

pub fn alloc_dbc(env: SQLHENV) -> Result<SQLHDBC> {
    let mut dbc = 0;
    check(
        unsafe { SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc) },
        SQL_HANDLE_ENV,
        env,
    )?;

    Ok(dbc)
}

pub fn alloc_stmt(dbc: SQLHDBC) -> Result<SQLHSTMT> {
    let mut stmt = 0;
    check(
        unsafe { SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt) },
        SQL_HANDLE_DBC,
        dbc,
    )?;

    Ok(stmt)
}

pub fn free_handle(handle_type: SQLSMALLINT, handle: SQLINTEGER) -> Result<()> {
    check(
        unsafe { SQLFreeHandle(handle_type, handle) },
        handle_type,
        handle,
    )
}

pub fn connect(dbc: SQLHDBC, name: &str) -> Result<()> {
    let len: SQLSMALLINT = length(name.len(), "database name")?;
    check(
        unsafe { SQLConnect(dbc, name.as_ptr(), len, ptr::null(), 0, ptr::null(), 0) },
        SQL_HANDLE_DBC,
        dbc,
    )
}

pub fn disconnect(dbc: SQLHDBC) -> Result<()> {
    check(unsafe { SQLDisconnect(dbc) }, SQL_HANDLE_DBC, dbc)
}

pub fn set_autocommit(dbc: SQLHDBC, on: bool) -> Result<()> {
    // libvsql reads the value after SQLSetConnectAttr returns, so it can't be
    // on the stack.
    static ON: SQLINTEGER = SQL_AUTOCOMMIT_ON;
    static OFF: SQLINTEGER = SQL_AUTOCOMMIT_OFF;
    let value = if on { &ON } else { &OFF };
    check(
        unsafe {
            SQLSetConnectAttr(
                dbc,
                SQL_ATTR_AUTOCOMMIT,
                value as *const SQLINTEGER as SQLPOINTER,
                0,
            )
        },
        SQL_HANDLE_DBC,
        dbc,
    )
}

pub fn start_tran(
    dbc: SQLHDBC,
    access_mode: SQLINTEGER,
    isolation_level: SQLINTEGER,
) -> Result<()> {
    check(
        unsafe { SQLStartTran(SQL_HANDLE_DBC, dbc, access_mode, isolation_level) },
        SQL_HANDLE_DBC,
        dbc,
    )
}

pub fn end_tran(dbc: SQLHDBC, commit: bool) -> Result<()> {
    let completion = if commit { SQL_COMMIT } else { SQL_ROLLBACK };
    check(
        unsafe { SQLEndTran(SQL_HANDLE_DBC, dbc, completion) },
        SQL_HANDLE_DBC,
        dbc,
    )
}

pub fn exec_direct(stmt: SQLHSTMT, sql: &str) -> Result<()> {
    let len: SQLINTEGER = length(sql.len(), "statement")?;
    check(
        unsafe { SQLExecDirect(stmt, sql.as_ptr(), len) },
        SQL_HANDLE_STMT,
        stmt,
    )
}

pub fn prepare(stmt: SQLHSTMT, sql: &str) -> Result<()> {
    let len: SQLINTEGER = length(sql.len(), "statement")?;
    check(
        unsafe { SQLPrepare(stmt, sql.as_ptr(), len) },
        SQL_HANDLE_STMT,
        stmt,
    )
}

/// A parameter for [`execute`].
pub struct Param {
    pub value_type: SQLSMALLINT,
    pub parameter_type: SQLSMALLINT,
    pub column_size: SQLINTEGER,
    pub decimal_digits: SQLSMALLINT,
    pub value: ParamValue,
}

pub enum ParamValue {
    Null,
    BigInt(SQLBIGINT),
    Double(SQLDOUBLE),
    Text(String),
}

/// Executes the prepared statement with the parameters, which are bound for
/// this execution only: they're borrowed until SQLExecute returns, and then
/// unbound so that libvsql never reads them again.
pub fn execute(stmt: SQLHSTMT, params: &mut [Param]) -> Result<()> {
    let mut inds: Vec<SQLINTEGER> = vec![0; params.len()];
    let result = bind_and_execute(stmt, params, &mut inds);
    if !params.is_empty() {
        unsafe { SQLFreeStmt(stmt, SQL_RESET_PARAMS) };
    }

    result
}

fn bind_and_execute(stmt: SQLHSTMT, params: &mut [Param], inds: &mut [SQLINTEGER]) -> Result<()> {
    for (i, (param, ind)) in params.iter_mut().zip(inds.iter_mut()).enumerate() {
        let number: SQLSMALLINT = length(i + 1, "parameter list")?;
        let (pointer, buffer_length): (SQLPOINTER, SQLINTEGER) = match &mut param.value {
            ParamValue::Null => {
                *ind = SQL_NULL_DATA;
                (ptr::null_mut(), 0)
            }
            ParamValue::BigInt(v) => (v as *mut SQLBIGINT as SQLPOINTER, 8),
            ParamValue::Double(v) => (v as *mut SQLDOUBLE as SQLPOINTER, 8),
            ParamValue::Text(s) => {
                *ind = length(s.len(), "parameter")?;
                (s.as_mut_ptr() as SQLPOINTER, *ind)
            }
        };
        check(
            unsafe {
                SQLBindParameter(
                    stmt,
                    number,
                    SQL_PARAM_MODE_IN,
                    param.value_type,
                    param.parameter_type,
                    param.column_size,
                    param.decimal_digits,
                    pointer,
                    buffer_length,
                    ind,
                )
            },
            SQL_HANDLE_STMT,
            stmt,
        )?;
    }

    check(unsafe { SQLExecute(stmt) }, SQL_HANDLE_STMT, stmt)
}

pub fn row_count(stmt: SQLHSTMT) -> Result<usize> {
    let mut n = 0;
    check(unsafe { SQLRowCount(stmt, &mut n) }, SQL_HANDLE_STMT, stmt)?;

    Ok(usize::try_from(n).unwrap_or(0))
}

pub fn num_result_cols(stmt: SQLHSTMT) -> Result<usize> {
    let mut n = 0;
    check(
        unsafe { SQLNumResultCols(stmt, &mut n) },
        SQL_HANDLE_STMT,
        stmt,
    )?;

    Ok(usize::try_from(n).unwrap_or(0))
}

/// A column of a cursor, from SQLDescribeCol.
pub struct ColumnDescription {
    pub name: String,
    pub data_type: SQLSMALLINT,
    pub column_size: SQLINTEGER,
    pub decimal_digits: SQLSMALLINT,
    pub nullable: SQLSMALLINT,
}

/// Describes a column (starting at 1).
pub fn describe_col(stmt: SQLHSTMT, n: usize) -> Result<ColumnDescription> {
    let number: SQLSMALLINT = length(n, "column list")?;
    let mut name = vec![0u8; 129];
    loop {
        let mut d = ColumnDescription {
            name: String::new(),
            data_type: 0,
            column_size: 0,
            decimal_digits: 0,
            nullable: 0,
        };
        let mut name_length: SQLSMALLINT = 0;
        check(
            unsafe {
                SQLDescribeCol(
                    stmt,
                    number,
                    name.as_mut_ptr(),
                    name.len() as SQLSMALLINT,
                    &mut name_length,
                    &mut d.data_type,
                    &mut d.column_size,
                    &mut d.decimal_digits,
                    &mut d.nullable,
                )
            },
            SQL_HANDLE_STMT,
            stmt,
        )?;

        // The name was truncated.
        let name_length = name_length as usize;
        if name_length >= name.len() {
            name.resize(name_length + 1, 0);
            continue;
        }

        d.name = String::from_utf8_lossy(&name[..name_length]).into_owned();
        return Ok(d);
    }
}

/// Moves to the next row, it's false after the last row.
pub fn fetch(stmt: SQLHSTMT) -> Result<bool> {
    let rc = unsafe { SQLFetch(stmt) };
    if rc == SQL_NO_DATA {
        return Ok(false);
    }
    check(rc, SQL_HANDLE_STMT, stmt)?;

    Ok(true)
}

pub fn close_cursor(stmt: SQLHSTMT) -> Result<()> {
    check(unsafe { SQLCloseCursor(stmt) }, SQL_HANDLE_STMT, stmt)
}

/// Reads a number column (starting at 1) of the fetched row as `T` (which is
/// target_type), it's None for a NULL.
fn get_number<T: Default>(
    stmt: SQLHSTMT,
    n: SQLSMALLINT,
    target_type: SQLSMALLINT,
) -> Result<Option<T>> {
    let mut value = T::default();
    let mut ind: SQLINTEGER = 0;
    check(
        unsafe {
            SQLGetData(
                stmt,
                n,
                target_type,
                &mut value as *mut T as SQLPOINTER,
                std::mem::size_of::<T>() as SQLINTEGER,
                &mut ind,
            )
        },
        SQL_HANDLE_STMT,
        stmt,
    )?;
    if ind == SQL_NULL_DATA {
        return Ok(None);
    }

    Ok(Some(value))
}

pub fn get_i64(stmt: SQLHSTMT, n: usize) -> Result<Option<i64>> {
    get_number::<SQLBIGINT>(stmt, length(n, "column list")?, SQL_BIGINT)
}

pub fn get_f64(stmt: SQLHSTMT, n: usize) -> Result<Option<f64>> {
    get_number::<SQLDOUBLE>(stmt, length(n, "column list")?, SQL_DOUBLE)
}

pub fn get_f32(stmt: SQLHSTMT, n: usize) -> Result<Option<f32>> {
    get_number::<SQLREAL>(stmt, length(n, "column list")?, SQL_REAL)
}

/// Reads a column (starting at 1) of the fetched row as SQL_CHAR, it's None
/// for a NULL. A string that doesn't fit in the buffer is read in pieces.
pub fn get_string(stmt: SQLHSTMT, n: usize, buf: &mut Vec<u8>) -> Result<Option<String>> {
    let n: SQLSMALLINT = length(n, "column list")?;
    if buf.len() < 256 {
        buf.resize(256, 0);
    }
    let mut s: Vec<u8> = Vec::new();
    loop {
        let mut ind: SQLINTEGER = 0;
        let size = buf.len().min(SQLINTEGER::MAX as usize);
        let rc = unsafe {
            SQLGetData(
                stmt,
                n,
                SQL_CHAR,
                buf.as_mut_ptr() as SQLPOINTER,
                size as SQLINTEGER,
                &mut ind,
            )
        };
        check(rc, SQL_HANDLE_STMT, stmt)?;
        if ind == SQL_NULL_DATA {
            return Ok(None);
        }

        // The rest of the string (ind) didn't fit, so the buffer has a null
        // terminated piece of it, and it grows to the size of the rest.
        let ind = ind as usize;
        if rc == SQL_SUCCESS_WITH_INFO && ind >= size {
            let piece = buf.iter().position(|&b| b == 0).unwrap_or(size);
            s.extend_from_slice(&buf[..piece]);
            buf.resize(ind - piece + 1, 0);
            continue;
        }

        s.extend_from_slice(&buf[..ind]);
        return String::from_utf8(s)
            .map(Some)
            .map_err(|_| sql_error("22021", "a string isn't valid UTF-8"));
    }
}

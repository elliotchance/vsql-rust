//! Raw bindings to libvsql, the SQL/CLI (ISO/IEC 9075-3) library of
//! [vsql](https://github.com/elliotchance/vsql), which is compiled into this
//! crate from the vsql amalgamation (`amalgamation/libvsql.h` documents each
//! routine).
//!
//! Only the routines and constants that the `vsql` crate uses are declared.
//! Use the [`vsql`](https://docs.rs/vsql) crate instead, unless you need to
//! call libvsql directly.

#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::c_void;

// The types are the same width on every platform (see libvsql.h).
pub type SQLCHAR = u8;
pub type SQLSMALLINT = i16;
pub type SQLINTEGER = i32;
pub type SQLREAL = f32;
pub type SQLDOUBLE = f64;
pub type SQLPOINTER = *mut c_void;
pub type SQLBIGINT = i64;
pub type SQLRETURN = SQLSMALLINT;
pub type SQLHENV = SQLINTEGER;
pub type SQLHDBC = SQLINTEGER;
pub type SQLHSTMT = SQLINTEGER;
pub type SQLHDESC = SQLINTEGER;

// Lengths and indicators.
pub const SQL_NULL_DATA: SQLINTEGER = -1;
pub const SQL_NTS: SQLINTEGER = -3;

// Return codes.
pub const SQL_SUCCESS: SQLRETURN = 0;
pub const SQL_SUCCESS_WITH_INFO: SQLRETURN = 1;
pub const SQL_NO_DATA: SQLRETURN = 100;
pub const SQL_ERROR: SQLRETURN = -1;
pub const SQL_INVALID_HANDLE: SQLRETURN = -2;

// Handle types.
pub const SQL_HANDLE_ENV: SQLSMALLINT = 1;
pub const SQL_HANDLE_DBC: SQLSMALLINT = 2;
pub const SQL_HANDLE_STMT: SQLSMALLINT = 3;
pub const SQL_HANDLE_DESC: SQLSMALLINT = 4;
pub const SQL_NULL_HANDLE: SQLINTEGER = 0;

// Data types.
pub const SQL_CHAR: SQLSMALLINT = 1;
pub const SQL_NUMERIC: SQLSMALLINT = 2;
pub const SQL_DECIMAL: SQLSMALLINT = 3;
pub const SQL_INTEGER: SQLSMALLINT = 4;
pub const SQL_SMALLINT: SQLSMALLINT = 5;
pub const SQL_FLOAT: SQLSMALLINT = 6;
pub const SQL_REAL: SQLSMALLINT = 7;
pub const SQL_DOUBLE: SQLSMALLINT = 8;
pub const SQL_VARCHAR: SQLSMALLINT = 12;
pub const SQL_BOOLEAN: SQLSMALLINT = 16;
pub const SQL_BIGINT: SQLSMALLINT = 25;
pub const SQL_TYPE_DATE: SQLSMALLINT = 91;
pub const SQL_TYPE_TIME: SQLSMALLINT = 92;
pub const SQL_TYPE_TIMESTAMP: SQLSMALLINT = 93;
pub const SQL_TYPE_TIME_WITH_TIMEZONE: SQLSMALLINT = 94;
pub const SQL_TYPE_TIMESTAMP_WITH_TIMEZONE: SQLSMALLINT = 95;

// Connection attributes.
pub const SQL_ATTR_AUTOCOMMIT: SQLINTEGER = 102;
pub const SQL_AUTOCOMMIT_OFF: SQLINTEGER = 0;
pub const SQL_AUTOCOMMIT_ON: SQLINTEGER = 1;

// SQLEndTran.
pub const SQL_COMMIT: SQLSMALLINT = 0;
pub const SQL_ROLLBACK: SQLSMALLINT = 1;

// SQLStartTran access modes and isolation levels.
pub const SQL_TRANSACTION_READ_ONLY: SQLINTEGER = 0x0000_0001;
pub const SQL_TRANSACTION_READ_WRITE: SQLINTEGER = 0x0000_0002;
pub const SQL_TRANSACTION_READ_UNCOMMITTED: SQLINTEGER = 0x0000_0001;
pub const SQL_TRANSACTION_READ_COMMITTED: SQLINTEGER = 0x0000_0002;
pub const SQL_TRANSACTION_REPEATABLE_READ: SQLINTEGER = 0x0000_0004;
pub const SQL_TRANSACTION_SERIALIZABLE: SQLINTEGER = 0x0000_0008;

// SQLBindParameter.
pub const SQL_PARAM_MODE_IN: SQLSMALLINT = 1;

// SQLFreeStmt.
pub const SQL_CLOSE: SQLSMALLINT = 0;
pub const SQL_RESET_PARAMS: SQLSMALLINT = 3;

// Nullability, from SQLDescribeCol.
pub const SQL_NO_NULLS: SQLSMALLINT = 0;
pub const SQL_NULLABLE: SQLSMALLINT = 1;

extern "C" {
    pub fn SQLAllocHandle(
        HandleType: SQLSMALLINT,
        InputHandle: SQLINTEGER,
        OutputHandle: *mut SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLFreeHandle(HandleType: SQLSMALLINT, Handle: SQLINTEGER) -> SQLRETURN;

    pub fn SQLConnect(
        ConnectionHandle: SQLHDBC,
        ServerName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        UserName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        Authentication: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
    ) -> SQLRETURN;

    pub fn SQLDisconnect(ConnectionHandle: SQLHDBC) -> SQLRETURN;

    pub fn SQLSetConnectAttr(
        ConnectionHandle: SQLHDBC,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        StringLength: SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLStartTran(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        AccessMode: SQLINTEGER,
        IsolationLevel: SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLEndTran(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        CompletionType: SQLSMALLINT,
    ) -> SQLRETURN;

    pub fn SQLExecDirect(
        StatementHandle: SQLHSTMT,
        StatementText: *const SQLCHAR,
        TextLength: SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLPrepare(
        StatementHandle: SQLHSTMT,
        StatementText: *const SQLCHAR,
        TextLength: SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLBindParameter(
        StatementHandle: SQLHSTMT,
        ParameterNumber: SQLSMALLINT,
        InputOutputMode: SQLSMALLINT,
        ValueType: SQLSMALLINT,
        ParameterType: SQLSMALLINT,
        ColumnSize: SQLINTEGER,
        DecimalDigits: SQLSMALLINT,
        ParameterValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StrLen_or_Ind: *mut SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLExecute(StatementHandle: SQLHSTMT) -> SQLRETURN;

    pub fn SQLRowCount(StatementHandle: SQLHSTMT, RowCount: *mut SQLINTEGER) -> SQLRETURN;

    pub fn SQLNumResultCols(StatementHandle: SQLHSTMT, ColumnCount: *mut SQLSMALLINT) -> SQLRETURN;

    pub fn SQLDescribeCol(
        StatementHandle: SQLHSTMT,
        ColumnNumber: SQLSMALLINT,
        ColumnName: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        NameLength: *mut SQLSMALLINT,
        DataType: *mut SQLSMALLINT,
        ColumnSize: *mut SQLINTEGER,
        DecimalDigits: *mut SQLSMALLINT,
        Nullable: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    pub fn SQLFetch(StatementHandle: SQLHSTMT) -> SQLRETURN;

    pub fn SQLGetData(
        StatementHandle: SQLHSTMT,
        ColumnNumber: SQLSMALLINT,
        TargetType: SQLSMALLINT,
        TargetValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StrLen_or_Ind: *mut SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLCloseCursor(StatementHandle: SQLHSTMT) -> SQLRETURN;

    pub fn SQLFreeStmt(StatementHandle: SQLHSTMT, Option: SQLSMALLINT) -> SQLRETURN;

    pub fn SQLGetDiagRec(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        RecordNumber: SQLSMALLINT,
        Sqlstate: *mut SQLCHAR,
        NativeError: *mut SQLINTEGER,
        MessageText: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        TextLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;
}

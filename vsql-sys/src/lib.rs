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
pub const SQL_DECFLOAT: SQLSMALLINT = 26;
pub const SQL_BLOB: SQLSMALLINT = 30;
pub const SQL_CLOB: SQLSMALLINT = 40;
pub const SQL_BINARY: SQLSMALLINT = 60;
pub const SQL_VARBINARY: SQLSMALLINT = 61;

// Concise codes of the interval data types.
pub const SQL_INTERVAL_YEAR: SQLSMALLINT = 101;
pub const SQL_INTERVAL_MONTH: SQLSMALLINT = 102;
pub const SQL_INTERVAL_DAY: SQLSMALLINT = 103;
pub const SQL_INTERVAL_HOUR: SQLSMALLINT = 104;
pub const SQL_INTERVAL_MINUTE: SQLSMALLINT = 105;
pub const SQL_INTERVAL_SECOND: SQLSMALLINT = 106;
pub const SQL_INTERVAL_YEAR_TO_MONTH: SQLSMALLINT = 107;
pub const SQL_INTERVAL_DAY_TO_HOUR: SQLSMALLINT = 108;
pub const SQL_INTERVAL_DAY_TO_MINUTE: SQLSMALLINT = 109;
pub const SQL_INTERVAL_DAY_TO_SECOND: SQLSMALLINT = 110;
pub const SQL_INTERVAL_HOUR_TO_MINUTE: SQLSMALLINT = 111;
pub const SQL_INTERVAL_HOUR_TO_SECOND: SQLSMALLINT = 112;
pub const SQL_INTERVAL_MINUTE_TO_SECOND: SQLSMALLINT = 113;

// Record fields of SQLGetDiagField.
pub const SQL_DIAG_CONSTRAINT_CATALOG: SQLSMALLINT = 15;
pub const SQL_DIAG_CONSTRAINT_SCHEMA: SQLSMALLINT = 16;
pub const SQL_DIAG_CONSTRAINT_NAME: SQLSMALLINT = 17;
pub const SQL_DIAG_CATALOG_NAME: SQLSMALLINT = 18;
pub const SQL_DIAG_SCHEMA_NAME: SQLSMALLINT = 19;
pub const SQL_DIAG_TABLE_NAME: SQLSMALLINT = 20;
pub const SQL_DIAG_COLUMN_NAME: SQLSMALLINT = 21;
pub const SQL_DIAG_CURSOR_NAME: SQLSMALLINT = 22;

// SQLGetInfo InfoTypes (the SQL_VSQL_* ones are vsql's own).
pub const SQL_DBMS_VERSION: SQLSMALLINT = 18;
pub const SQL_VSQL_BUILD_VERSION: SQLSMALLINT = 11000;
pub const SQL_VSQL_BUILD_COMMIT: SQLSMALLINT = 11001;
pub const SQL_VSQL_BUILD_DATE: SQLSMALLINT = 11002;
pub const SQL_VSQL_BUILD_V_VERSION: SQLSMALLINT = 11003;
pub const SQL_VSQL_BUILD_V_COMMIT: SQLSMALLINT = 11004;
pub const SQL_VSQL_BUILD_C_COMPILER: SQLSMALLINT = 11005;
pub const SQL_VSQL_OPERATING_SYSTEM: SQLSMALLINT = 11006;
pub const SQL_VSQL_MACHINE: SQLSMALLINT = 11007;

// Connection attributes.
pub const SQL_ATTR_AUTOCOMMIT: SQLINTEGER = 102;
pub const SQL_AUTOCOMMIT_OFF: SQLINTEGER = 0;
pub const SQL_AUTOCOMMIT_ON: SQLINTEGER = 1;
pub const SQL_ATTR_SAVEPOINT_NAME: SQLINTEGER = 10027;

// SQLEndTran.
pub const SQL_COMMIT: SQLSMALLINT = 0;
pub const SQL_ROLLBACK: SQLSMALLINT = 1;
pub const SQL_SAVEPOINT_NAME_ROLLBACK: SQLSMALLINT = 2;
pub const SQL_SAVEPOINT_NAME_RELEASE: SQLSMALLINT = 4;

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

// Statement attributes and descriptor fields.
pub const SQL_ATTR_IMP_PARAM_DESC: SQLINTEGER = 10013;
pub const SQL_DESC_COUNT: SQLSMALLINT = 1001;

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

    pub fn SQLGetDiagField(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        RecordNumber: SQLSMALLINT,
        DiagIdentifier: SQLSMALLINT,
        DiagInfo: SQLPOINTER,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    pub fn SQLGetInfo(
        ConnectionHandle: SQLHDBC,
        InfoType: SQLSMALLINT,
        InfoValue: SQLPOINTER,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    pub fn SQLGetStmtAttr(
        StatementHandle: SQLHSTMT,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;

    pub fn SQLGetDescField(
        DescriptorHandle: SQLHDESC,
        RecordNumber: SQLSMALLINT,
        FieldIdentifier: SQLSMALLINT,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;
}

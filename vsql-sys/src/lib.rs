//! Raw bindings to libvsql, the SQL/CLI (ISO/IEC 9075-3) library of
//! [vsql](https://github.com/elliotchance/vsql), which is compiled into this
//! crate from the vsql amalgamation (`amalgamation/libvsql.h` documents each
//! routine).
//!
//! Every type, constant and routine in libvsql.h is declared, with the same
//! names and values (a test checks them against the header). Each constant has
//! the type of the argument or field it's used for, the string constants are
//! `&CStr`, and the string arguments that libvsql only reads are
//! `*const SQLCHAR`. Use the
//! [`vsql`](https://docs.rs/vsql) crate instead, unless you need to call
//! libvsql directly.

#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_void, CStr};

// The types are the same width on every platform (see libvsql.h).
pub type SQLCHAR = u8;
pub type SQLSMALLINT = i16;
pub type SQLINTEGER = i32;
pub type SQLREAL = f32;
pub type SQLDOUBLE = f64;
pub type SQLPOINTER = *mut c_void;
pub type SQLBIGINT = i64;
pub type SQLRETURN = SQLSMALLINT;

// Handles are integers that identify an allocated resource.
pub type SQLHENV = SQLINTEGER;
pub type SQLHDBC = SQLINTEGER;
pub type SQLHSTMT = SQLINTEGER;
pub type SQLHDESC = SQLINTEGER;

// Special length/indicator values.
/// The value is NULL.
pub const SQL_NULL_DATA: SQLINTEGER = -1;
/// A null terminated string.
pub const SQL_NTS: SQLINTEGER = -3;
/// Given with SQLPutData.
pub const SQL_DATA_AT_EXEC: SQLINTEGER = -2;

// Return codes.
/// Completed.
pub const SQL_SUCCESS: SQLRETURN = 0;
/// Completed with a warning.
pub const SQL_SUCCESS_WITH_INFO: SQLRETURN = 1;
/// See SQLParamData.
pub const SQL_NEED_DATA: SQLRETURN = 99;
/// No more rows or records.
pub const SQL_NO_DATA: SQLRETURN = 100;
/// Failed, see SQLGetDiagRec.
pub const SQL_ERROR: SQLRETURN = -1;
/// Not an allocated handle.
pub const SQL_INVALID_HANDLE: SQLRETURN = -2;

// Handle types.
/// An environment.
pub const SQL_HANDLE_ENV: SQLSMALLINT = 1;
/// A connection.
pub const SQL_HANDLE_DBC: SQLSMALLINT = 2;
/// A statement.
pub const SQL_HANDLE_STMT: SQLSMALLINT = 3;
/// A descriptor.
pub const SQL_HANDLE_DESC: SQLSMALLINT = 4;
/// No handle.
pub const SQL_NULL_HANDLE: SQLINTEGER = 0;

// Data type codes.
/// CHARACTER, or a C string.
pub const SQL_CHAR: SQLSMALLINT = 1;
/// NUMERIC.
pub const SQL_NUMERIC: SQLSMALLINT = 2;
/// DECIMAL.
pub const SQL_DECIMAL: SQLSMALLINT = 3;
/// INTEGER, or SQLINTEGER.
pub const SQL_INTEGER: SQLSMALLINT = 4;
/// SMALLINT, or SQLSMALLINT.
pub const SQL_SMALLINT: SQLSMALLINT = 5;
/// FLOAT, see SQL_DOUBLE.
pub const SQL_FLOAT: SQLSMALLINT = 6;
/// REAL, or SQLREAL.
pub const SQL_REAL: SQLSMALLINT = 7;
/// DOUBLE PRECISION, SQLDOUBLE.
pub const SQL_DOUBLE: SQLSMALLINT = 8;
/// Any datetime type.
pub const SQL_DATETIME: SQLSMALLINT = 9;
/// Any interval type.
pub const SQL_INTERVAL: SQLSMALLINT = 10;
/// CHARACTER VARYING.
pub const SQL_VARCHAR: SQLSMALLINT = 12;
/// BOOLEAN.
pub const SQL_BOOLEAN: SQLSMALLINT = 16;
/// BIGINT, or SQLBIGINT.
pub const SQL_BIGINT: SQLSMALLINT = 25;
/// DECFLOAT (not in SQL/CLI 2003, see Part 2, Table 30).
pub const SQL_DECFLOAT: SQLSMALLINT = 26;
/// BINARY LARGE OBJECT, or an array of unsigned char.
pub const SQL_BLOB: SQLSMALLINT = 30;
/// CHARACTER LARGE OBJECT, or a C string.
pub const SQL_CLOB: SQLSMALLINT = 40;

// BINARY and BINARY VARYING aren't in SQL/CLI 2003, so these are their codes in
// dynamic SQL (Part 2, Table 30). The C data type of each is an array of
// unsigned char, with a length in octets.
/// BINARY.
pub const SQL_BINARY: SQLSMALLINT = 60;
/// BINARY VARYING.
pub const SQL_VARBINARY: SQLSMALLINT = 61;

// Large object locator types, for the LocatorType of SQLGetLength,
// SQLGetPosition and SQLGetSubString.
/// BINARY LARGE OBJECT LOCATOR.
pub const SQL_BLOB_LOCATOR: SQLSMALLINT = 31;
/// CHARACTER LARGE OBJECT LOCATOR.
pub const SQL_CLOB_LOCATOR: SQLSMALLINT = 41;

// SQL_DESC_DATETIME_INTERVAL_CODEs of SQL_DATETIME.
/// DATE.
pub const SQL_CODE_DATE: SQLSMALLINT = 1;
/// TIME.
pub const SQL_CODE_TIME: SQLSMALLINT = 2;
/// TIMESTAMP.
pub const SQL_CODE_TIMESTAMP: SQLSMALLINT = 3;
/// TIME WITH TIME ZONE.
pub const SQL_CODE_TIME_ZONE: SQLSMALLINT = 4;
/// TIMESTAMP WITH TIME ZONE.
pub const SQL_CODE_TIMESTAMP_ZONE: SQLSMALLINT = 5;

// Concise codes for datetime data types.
/// DATE.
pub const SQL_TYPE_DATE: SQLSMALLINT = 91;
/// TIME.
pub const SQL_TYPE_TIME: SQLSMALLINT = 92;
/// TIMESTAMP.
pub const SQL_TYPE_TIMESTAMP: SQLSMALLINT = 93;
/// TIME WITH TIME ZONE.
pub const SQL_TYPE_TIME_WITH_TIMEZONE: SQLSMALLINT = 94;
/// TIMESTAMP WITH TIME ZONE.
pub const SQL_TYPE_TIMESTAMP_WITH_TIMEZONE: SQLSMALLINT = 95;

// SQL_DESC_DATETIME_INTERVAL_CODEs of SQL_INTERVAL.
/// YEAR.
pub const SQL_CODE_YEAR: SQLSMALLINT = 1;
/// MONTH.
pub const SQL_CODE_MONTH: SQLSMALLINT = 2;
/// DAY.
pub const SQL_CODE_DAY: SQLSMALLINT = 3;
/// HOUR.
pub const SQL_CODE_HOUR: SQLSMALLINT = 4;
/// MINUTE.
pub const SQL_CODE_MINUTE: SQLSMALLINT = 5;
/// SECOND.
pub const SQL_CODE_SECOND: SQLSMALLINT = 6;
/// YEAR TO MONTH.
pub const SQL_CODE_YEAR_TO_MONTH: SQLSMALLINT = 7;
/// DAY TO HOUR.
pub const SQL_CODE_DAY_TO_HOUR: SQLSMALLINT = 8;
/// DAY TO MINUTE.
pub const SQL_CODE_DAY_TO_MINUTE: SQLSMALLINT = 9;
/// DAY TO SECOND.
pub const SQL_CODE_DAY_TO_SECOND: SQLSMALLINT = 10;
/// HOUR TO MINUTE.
pub const SQL_CODE_HOUR_TO_MINUTE: SQLSMALLINT = 11;
/// HOUR TO SECOND.
pub const SQL_CODE_HOUR_TO_SECOND: SQLSMALLINT = 12;
/// MINUTE TO SECOND.
pub const SQL_CODE_MINUTE_TO_SECOND: SQLSMALLINT = 13;

// Concise codes for interval data types, which are the DATA_TYPE of
// SQLDescribeCol (and the ParameterType of SQLBindParameter).
/// INTERVAL YEAR.
pub const SQL_INTERVAL_YEAR: SQLSMALLINT = 101;
/// INTERVAL MONTH.
pub const SQL_INTERVAL_MONTH: SQLSMALLINT = 102;
/// INTERVAL DAY.
pub const SQL_INTERVAL_DAY: SQLSMALLINT = 103;
/// INTERVAL HOUR.
pub const SQL_INTERVAL_HOUR: SQLSMALLINT = 104;
/// INTERVAL MINUTE.
pub const SQL_INTERVAL_MINUTE: SQLSMALLINT = 105;
/// INTERVAL SECOND.
pub const SQL_INTERVAL_SECOND: SQLSMALLINT = 106;
/// INTERVAL YEAR TO MONTH.
pub const SQL_INTERVAL_YEAR_TO_MONTH: SQLSMALLINT = 107;
/// INTERVAL DAY TO HOUR.
pub const SQL_INTERVAL_DAY_TO_HOUR: SQLSMALLINT = 108;
/// INTERVAL DAY TO MINUTE.
pub const SQL_INTERVAL_DAY_TO_MINUTE: SQLSMALLINT = 109;
/// INTERVAL DAY TO SECOND.
pub const SQL_INTERVAL_DAY_TO_SECOND: SQLSMALLINT = 110;
/// INTERVAL HOUR TO MINUTE.
pub const SQL_INTERVAL_HOUR_TO_MINUTE: SQLSMALLINT = 111;
/// INTERVAL HOUR TO SECOND.
pub const SQL_INTERVAL_HOUR_TO_SECOND: SQLSMALLINT = 112;
/// INTERVAL MINUTE TO SECOND.
pub const SQL_INTERVAL_MINUTE_TO_SECOND: SQLSMALLINT = 113;

// Special TargetType values for SQLGetData.
/// The type in the ARD.
pub const SQL_ARD_TYPE: SQLSMALLINT = -99;
/// The type of the column.
pub const SQL_DEFAULT: SQLSMALLINT = 99;

// Header fields for SQLGetDiagField.
/// The return code.
pub const SQL_DIAG_RETURNCODE: SQLSMALLINT = 1;
/// The number of records.
pub const SQL_DIAG_NUMBER: SQLSMALLINT = 2;
/// See SQLRowCount.
pub const SQL_DIAG_ROW_COUNT: SQLSMALLINT = 3;
/// The statement executed.
pub const SQL_DIAG_DYNAMIC_FUNCTION: SQLSMALLINT = 7;
/// The code of the statement.
pub const SQL_DIAG_DYNAMIC_FUNCTION_CODE: SQLSMALLINT = 12;
/// 1 if a condition was dropped.
pub const SQL_DIAG_MORE: SQLSMALLINT = 13;
/// Transactions committed.
pub const SQL_DIAG_TRANSACTIONS_COMMITTED: SQLSMALLINT = 34;
/// Transactions rolled back.
pub const SQL_DIAG_TRANSACTIONS_ROLLED_BACK: SQLSMALLINT = 35;
/// 1 if one is active.
pub const SQL_DIAG_TRANSACTION_ACTIVE: SQLSMALLINT = 36;

// Record fields for SQLGetDiagField.
/// The SQLSTATE.
pub const SQL_DIAG_SQLSTATE: SQLSMALLINT = 4;
/// Always 0.
pub const SQL_DIAG_NATIVE_CODE: SQLSMALLINT = 5;
/// The message.
pub const SQL_DIAG_MESSAGE_TEXT: SQLSMALLINT = 6;
/// Who defined the class.
pub const SQL_DIAG_CLASS_ORIGIN: SQLSMALLINT = 8;
/// And the subclass.
pub const SQL_DIAG_SUBCLASS_ORIGIN: SQLSMALLINT = 9;
/// The ServerName.
pub const SQL_DIAG_CONNECTION_NAME: SQLSMALLINT = 10;
/// The ServerName.
pub const SQL_DIAG_SERVER_NAME: SQLSMALLINT = 11;
/// The record number.
pub const SQL_DIAG_CONDITION_NUMBER: SQLSMALLINT = 14;
/// The violated constraint.
pub const SQL_DIAG_CONSTRAINT_CATALOG: SQLSMALLINT = 15;
/// Its schema.
pub const SQL_DIAG_CONSTRAINT_SCHEMA: SQLSMALLINT = 16;
/// Its name.
pub const SQL_DIAG_CONSTRAINT_NAME: SQLSMALLINT = 17;
/// The table's catalog.
pub const SQL_DIAG_CATALOG_NAME: SQLSMALLINT = 18;
/// Its schema.
pub const SQL_DIAG_SCHEMA_NAME: SQLSMALLINT = 19;
/// The table.
pub const SQL_DIAG_TABLE_NAME: SQLSMALLINT = 20;
/// For 42703.
pub const SQL_DIAG_COLUMN_NAME: SQLSMALLINT = 21;
/// For 24000.
pub const SQL_DIAG_CURSOR_NAME: SQLSMALLINT = 22;
/// In characters.
pub const SQL_DIAG_MESSAGE_LENGTH: SQLSMALLINT = 23;
/// In octets.
pub const SQL_DIAG_MESSAGE_OCTET_LENGTH: SQLSMALLINT = 24;
/// Always "".
pub const SQL_DIAG_CONDITION_IDENTIFIER: SQLSMALLINT = 25;
/// Always "".
pub const SQL_DIAG_PARAMETER_NAME: SQLSMALLINT = 26;
/// Always "".
pub const SQL_DIAG_ROUTINE_CATALOG: SQLSMALLINT = 27;
/// Always "".
pub const SQL_DIAG_ROUTINE_SCHEMA: SQLSMALLINT = 28;
/// Always "".
pub const SQL_DIAG_ROUTINE_NAME: SQLSMALLINT = 29;
/// Always "".
pub const SQL_DIAG_SPECIFIC_NAME: SQLSMALLINT = 30;
/// Always "".
pub const SQL_DIAG_TRIGGER_CATALOG: SQLSMALLINT = 31;
/// Always "".
pub const SQL_DIAG_TRIGGER_SCHEMA: SQLSMALLINT = 32;
/// Always "".
pub const SQL_DIAG_TRIGGER_NAME: SQLSMALLINT = 33;
/// Always "".
pub const SQL_DIAG_PARAMETER_MODE: SQLSMALLINT = 37;
/// Always 0.
pub const SQL_DIAG_PARAMETER_ORDINAL_POSITION: SQLSMALLINT = 38;
/// The row, or 0.
pub const SQL_DIAG_ROW_NUMBER: SQLSMALLINT = -1248;
/// The column, or 0.
pub const SQL_DIAG_COLUMN_NUMBER: SQLSMALLINT = -1247;

// SQL_DIAG_DYNAMIC_FUNCTION_CODEs. The other statements have the codes from
// ISO/IEC 9075-2: ALTER SEQUENCE (134), COMMIT WORK (11), CREATE SEQUENCE
// (133), DROP SEQUENCE (135), ROLLBACK WORK (62) and START TRANSACTION (111).
// EXPLAIN is -1.
/// No statement.
pub const SQL_DIAG_UNKNOWN_STATEMENT: SQLINTEGER = 0;
/// ALTER TABLE.
pub const SQL_DIAG_ALTER_TABLE: SQLINTEGER = 4;
/// DELETE.
pub const SQL_DIAG_DELETE_WHERE: SQLINTEGER = 19;
/// DROP SCHEMA.
pub const SQL_DIAG_DROP_SCHEMA: SQLINTEGER = 31;
/// DROP TABLE.
pub const SQL_DIAG_DROP_TABLE: SQLINTEGER = 32;
/// INSERT.
pub const SQL_DIAG_INSERT: SQLINTEGER = 50;
/// CREATE SCHEMA.
pub const SQL_DIAG_CREATE_SCHEMA: SQLINTEGER = 64;
/// SET CATALOG.
pub const SQL_DIAG_SET_CATALOG: SQLINTEGER = 66;
/// SET SCHEMA.
pub const SQL_DIAG_SET_SCHEMA: SQLINTEGER = 74;
/// CREATE TABLE.
pub const SQL_DIAG_CREATE_TABLE: SQLINTEGER = 77;
/// UPDATE.
pub const SQL_DIAG_UPDATE_WHERE: SQLINTEGER = 82;
/// A query.
pub const SQL_DIAG_SELECT_CURSOR: SQLINTEGER = 85;

// Boolean values.
/// False.
pub const SQL_FALSE: SQLSMALLINT = 0;
/// True.
pub const SQL_TRUE: SQLSMALLINT = 1;

// Environment attributes.
/// Null terminate strings.
pub const SQL_ATTR_OUTPUT_NTS: SQLINTEGER = 10001;

// Connection attributes.
/// Always SQL_TRUE.
pub const SQL_ATTR_AUTO_IPD: SQLINTEGER = 10001;
/// A savepoint for SQLEndTran.
pub const SQL_ATTR_SAVEPOINT_NAME: SQLINTEGER = 10027;
/// See SQLSetConnectAttr.
pub const SQL_ATTR_AUTOCOMMIT: SQLINTEGER = 102;

// SQL_ATTR_AUTOCOMMIT values.
/// The default.
pub const SQL_AUTOCOMMIT_OFF: SQLINTEGER = 0;
/// Commit each statement.
pub const SQL_AUTOCOMMIT_ON: SQLINTEGER = 1;

// SQLEndTran CompletionTypes.
/// Commit.
pub const SQL_COMMIT: SQLSMALLINT = 0;
/// Roll back.
pub const SQL_ROLLBACK: SQLSMALLINT = 1;
/// Roll back to a savepoint.
pub const SQL_SAVEPOINT_NAME_ROLLBACK: SQLSMALLINT = 2;
/// Destroy a savepoint.
pub const SQL_SAVEPOINT_NAME_RELEASE: SQLSMALLINT = 4;
/// Commit and start another.
pub const SQL_COMMIT_AND_CHAIN: SQLSMALLINT = 6;
/// Roll back and start another.
pub const SQL_ROLLBACK_AND_CHAIN: SQLSMALLINT = 7;

// SQLStartTran AccessModes.
/// Only read.
pub const SQL_TRANSACTION_READ_ONLY: SQLINTEGER = 0x0000_0001;
/// Read and write.
pub const SQL_TRANSACTION_READ_WRITE: SQLINTEGER = 0x0000_0002;

// SQLStartTran IsolationLevels.
/// READ COMMITTED.
pub const SQL_TRANSACTION_READ_UNCOMMITTED: SQLINTEGER = 0x0000_0001;
/// READ COMMITTED.
pub const SQL_TRANSACTION_READ_COMMITTED: SQLINTEGER = 0x0000_0002;
/// SERIALIZABLE.
pub const SQL_TRANSACTION_REPEATABLE_READ: SQLINTEGER = 0x0000_0004;
/// SERIALIZABLE.
pub const SQL_TRANSACTION_SERIALIZABLE: SQLINTEGER = 0x0000_0008;

// SQLGetSessionInfo InfoTypes.
/// CURRENT_USER.
pub const SQL_CURRENT_USER: SQLSMALLINT = 47;
/// The default transform group.
pub const SQL_CURRENT_DEFAULT_TRANSFORM_GROUP: SQLSMALLINT = 20004;
/// CURRENT_PATH.
pub const SQL_CURRENT_PATH: SQLSMALLINT = 20005;
/// CURRENT_ROLE.
pub const SQL_CURRENT_ROLE: SQLSMALLINT = 20006;
/// SESSION_USER.
pub const SQL_SESSION_USER: SQLSMALLINT = 20007;
/// SYSTEM_USER.
pub const SQL_SYSTEM_USER: SQLSMALLINT = 20008;

// Special values for the catalog routines.
/// Every catalog (SQLTables).
pub const SQL_ALL_CATALOGS: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"%\0") };
/// Every schema (SQLTables).
pub const SQL_ALL_SCHEMAS: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"%\0") };
/// Every table type (SQLTables).
pub const SQL_ALL_TABLE_TYPES: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"%\0") };
/// Every data type (SQLGetTypeInfo).
pub const SQL_ALL_TYPES: SQLSMALLINT = 0;

// UPDATE_RULE and DELETE_RULE of SQLForeignKeys.
/// CASCADE.
pub const SQL_CASCADE: SQLSMALLINT = 0;
/// RESTRICT.
pub const SQL_RESTRICT: SQLSMALLINT = 1;
/// SET NULL.
pub const SQL_SET_NULL: SQLSMALLINT = 2;
/// NO ACTION.
pub const SQL_NO_ACTION: SQLSMALLINT = 3;
/// SET DEFAULT.
pub const SQL_SET_DEFAULT: SQLSMALLINT = 4;

// DEFERABILITY of SQLForeignKeys.
/// DEFERRABLE INITIALLY DEFERRED.
pub const SQL_INITIALLY_DEFERRED: SQLSMALLINT = 5;
/// DEFERRABLE INITIALLY IMMEDIATE.
pub const SQL_INITIALLY_IMMEDIATE: SQLSMALLINT = 6;
/// NOT DEFERRABLE.
pub const SQL_NOT_DEFERRABLE: SQLSMALLINT = 7;

// SQLSpecialColumns IdentifierType, Scope and PSEUDO_COLUMN.
/// The columns that identify a row.
pub const SQL_BEST_ROWID: SQLSMALLINT = 1;
/// Valid for the current row.
pub const SQL_SCOPE_CURROW: SQLSMALLINT = 0;
/// Valid for the transaction.
pub const SQL_SCOPE_TRANSACTION: SQLSMALLINT = 1;
/// Valid for the session.
pub const SQL_SCOPE_SESSION: SQLSMALLINT = 2;
/// Not used.
pub const SQL_PC_UNKNOWN: SQLSMALLINT = 0;
/// A real column.
pub const SQL_PC_NOT_PSEUDO: SQLSMALLINT = 1;
/// Not used.
pub const SQL_PC_PSEUDO: SQLSMALLINT = 2;

// SQLGetInfo InfoTypes.
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_DRIVER_CONNECTIONS: SQLSMALLINT = 0;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_CONCURRENT_ACTIVITIES: SQLSMALLINT = 1;
/// The ServerName.
pub const SQL_DATA_SOURCE_NAME: SQLSMALLINT = 2;
/// INTEGER, SQL_FD_FETCH_*.
pub const SQL_FETCH_DIRECTION: SQLSMALLINT = 8;
/// The ServerName.
pub const SQL_SERVER_NAME: SQLSMALLINT = 13;
/// "\\".
pub const SQL_SEARCH_PATTERN_ESCAPE: SQLSMALLINT = 14;
/// "vsql".
pub const SQL_DBMS_NAME: SQLSMALLINT = 17;
/// "01.02.0003".
pub const SQL_DBMS_VERSION: SQLSMALLINT = 18;
/// SMALLINT, SQL_CB_*.
pub const SQL_CURSOR_COMMIT_BEHAVIOR: SQLSMALLINT = 23;
/// "N".
pub const SQL_DATA_SOURCE_READ_ONLY: SQLSMALLINT = 25;
/// INTEGER, SQL_TRANSACTION_*.
pub const SQL_DEFAULT_TRANSACTION_ISOLATION: SQLSMALLINT = 26;
/// SMALLINT, SQL_IC_*.
pub const SQL_IDENTIFIER_CASE: SQLSMALLINT = 28;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_COLUMN_NAME_LENGTH: SQLSMALLINT = 30;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_CURSOR_NAME_LENGTH: SQLSMALLINT = 31;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_SCHEMA_NAME_LENGTH: SQLSMALLINT = 32;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_CATALOG_NAME_LENGTH: SQLSMALLINT = 34;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_TABLE_NAME_LENGTH: SQLSMALLINT = 35;
/// INTEGER, SQL_SCCO_*.
pub const SQL_SCROLL_CONCURRENCY: SQLSMALLINT = 43;
/// SMALLINT, SQL_TC_*.
pub const SQL_TRANSACTION_CAPABLE: SQLSMALLINT = 46;
/// "" (no users).
pub const SQL_USER_NAME: SQLSMALLINT = 47;
/// INTEGER, SQL_TRANSACTION_*.
pub const SQL_TRANSACTION_ISOLATION_OPTION: SQLSMALLINT = 72;
/// "Y".
pub const SQL_INTEGRITY: SQLSMALLINT = 73;
/// INTEGER, SQL_GD_*.
pub const SQL_GETDATA_EXTENSIONS: SQLSMALLINT = 81;
/// SMALLINT, SQL_NC_*.
pub const SQL_NULL_COLLATION: SQLSMALLINT = 85;
/// INTEGER, SQL_AT_*.
pub const SQL_ALTER_TABLE: SQLSMALLINT = 86;
/// "N".
pub const SQL_ORDER_BY_COLUMNS_IN_SELECT: SQLSMALLINT = 90;
/// "".
pub const SQL_SPECIAL_CHARACTERS: SQLSMALLINT = 94;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_COLUMNS_IN_GROUP_BY: SQLSMALLINT = 97;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_COLUMNS_IN_ORDER_BY: SQLSMALLINT = 99;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_COLUMNS_IN_SELECT: SQLSMALLINT = 100;
/// SMALLINT, 1600.
pub const SQL_MAXIMUM_COLUMNS_IN_TABLE: SQLSMALLINT = 101;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_TABLES_IN_SELECT: SQLSMALLINT = 106;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_USER_NAME_LENGTH: SQLSMALLINT = 107;
/// INTEGER, SQL_OUTER_JOIN_*.
pub const SQL_OUTER_JOIN_CAPABILITIES: SQLSMALLINT = 115;
/// INTEGER, SQL_INSENSITIVE.
pub const SQL_CURSOR_SENSITIVITY: SQLSMALLINT = 10001;
/// "N".
pub const SQL_DESCRIBE_PARAMETER: SQLSMALLINT = 10002;
/// "Y".
pub const SQL_CATALOG_NAME: SQLSMALLINT = 10003;
/// "".
pub const SQL_COLLATING_SEQUENCE: SQLSMALLINT = 10004;
/// SMALLINT, 128.
pub const SQL_MAXIMUM_IDENTIFIER_LENGTH: SQLSMALLINT = 10005;
/// "v1.2.3", "" if not a release.
pub const SQL_VSQL_BUILD_VERSION: SQLSMALLINT = 11000;
/// Git commit, "" if not a release.
pub const SQL_VSQL_BUILD_COMMIT: SQLSMALLINT = 11001;
/// "2026-11-12" (UTC).
pub const SQL_VSQL_BUILD_DATE: SQLSMALLINT = 11002;
/// "0.5.2".
pub const SQL_VSQL_BUILD_V_VERSION: SQLSMALLINT = 11003;
/// "5634a1f".
pub const SQL_VSQL_BUILD_V_COMMIT: SQLSMALLINT = 11004;
/// "clang".
pub const SQL_VSQL_BUILD_C_COMPILER: SQLSMALLINT = 11005;
/// "macos".
pub const SQL_VSQL_OPERATING_SYSTEM: SQLSMALLINT = 11006;
/// "arm64".
pub const SQL_VSQL_MACHINE: SQLSMALLINT = 11007;
/// SMALLINT, 0 for :memory:.
pub const SQL_VSQL_FILE_READ_VERSION: SQLSMALLINT = 11008;
/// SMALLINT, 0 for :memory:.
pub const SQL_VSQL_FILE_WRITE_VERSION: SQLSMALLINT = 11009;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_STMT_OCTETS: SQLSMALLINT = 20000;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_STMT_OCTETS_DATA: SQLSMALLINT = 20001;
/// SMALLINT, 0 (no limit).
pub const SQL_MAXIMUM_STMT_OCTETS_SCHEMA: SQLSMALLINT = 20002;

// SQLGetInfo values.
/// ADD COLUMN.
pub const SQL_AT_ADD_COLUMN: SQLINTEGER = 0x0000_0001;
/// DROP COLUMN.
pub const SQL_AT_DROP_COLUMN: SQLINTEGER = 0x0000_0002;
/// ALTER COLUMN.
pub const SQL_AT_ALTER_COLUMN: SQLINTEGER = 0x0000_0004;
/// ADD CONSTRAINT.
pub const SQL_AT_ADD_CONSTRAINT: SQLINTEGER = 0x0000_0008;
/// DROP CONSTRAINT.
pub const SQL_AT_DROP_CONSTRAINT: SQLINTEGER = 0x0000_0010;
/// Cursors are deleted.
pub const SQL_CB_DELETE: SQLSMALLINT = 0;
/// Cursors are closed.
pub const SQL_CB_CLOSE: SQLSMALLINT = 1;
/// Cursors stay open.
pub const SQL_CB_PRESERVE: SQLSMALLINT = 2;
/// SQL_FETCH_NEXT.
pub const SQL_FD_FETCH_NEXT: SQLINTEGER = 0x0000_0001;
/// SQL_FETCH_FIRST.
pub const SQL_FD_FETCH_FIRST: SQLINTEGER = 0x0000_0002;
/// SQL_FETCH_LAST.
pub const SQL_FD_FETCH_LAST: SQLINTEGER = 0x0000_0004;
/// SQL_FETCH_PRIOR.
pub const SQL_FD_FETCH_PRIOR: SQLINTEGER = 0x0000_0008;
/// SQL_FETCH_ABSOLUTE.
pub const SQL_FD_FETCH_ABSOLUTE: SQLINTEGER = 0x0000_0010;
/// SQL_FETCH_RELATIVE.
pub const SQL_FD_FETCH_RELATIVE: SQLINTEGER = 0x0000_0020;
/// Any unbound column.
pub const SQL_GD_ANY_COLUMN: SQLINTEGER = 0x0000_0001;
/// Columns in any order.
pub const SQL_GD_ANY_ORDER: SQLINTEGER = 0x0000_0002;
/// Upper case.
pub const SQL_IC_UPPER: SQLSMALLINT = 1;
/// Lower case.
pub const SQL_IC_LOWER: SQLSMALLINT = 2;
/// Case sensitive.
pub const SQL_IC_SENSITIVE: SQLSMALLINT = 3;
/// Mixed case.
pub const SQL_IC_MIXED: SQLSMALLINT = 4;
/// NULLs sort high.
pub const SQL_NC_HIGH: SQLSMALLINT = 1;
/// NULLs sort low.
pub const SQL_NC_LOW: SQLSMALLINT = 2;
/// LEFT JOIN.
pub const SQL_OUTER_JOIN_LEFT: SQLINTEGER = 0x0000_0001;
/// RIGHT JOIN.
pub const SQL_OUTER_JOIN_RIGHT: SQLINTEGER = 0x0000_0002;
/// FULL JOIN.
pub const SQL_OUTER_JOIN_FULL: SQLINTEGER = 0x0000_0004;
/// Nested joins.
pub const SQL_OUTER_JOIN_NESTED: SQLINTEGER = 0x0000_0008;
/// ON columns in any order.
pub const SQL_OUTER_JOIN_NOT_ORDERED: SQLINTEGER = 0x0000_0010;
/// Inner table in an INNER JOIN.
pub const SQL_OUTER_JOIN_INNER: SQLINTEGER = 0x0000_0020;
/// Any comparison in ON.
pub const SQL_OUTER_JOIN_ALL_COMPARISON_OPS: SQLINTEGER = 0x0000_0040;
/// Read only.
pub const SQL_SCCO_READ_ONLY: SQLINTEGER = 0x0000_0001;
/// Locking.
pub const SQL_SCCO_LOCK: SQLINTEGER = 0x0000_0002;
/// Row versions.
pub const SQL_SCCO_OPT_ROWVER: SQLINTEGER = 0x0000_0004;
/// Values.
pub const SQL_SCCO_OPT_VALUES: SQLINTEGER = 0x0000_0008;
/// No transactions.
pub const SQL_TC_NONE: SQLSMALLINT = 0;
/// DML only.
pub const SQL_TC_DML: SQLSMALLINT = 1;
/// DML and DDL.
pub const SQL_TC_ALL: SQLSMALLINT = 2;
/// DDL commits.
pub const SQL_TC_DDL_COMMIT: SQLSMALLINT = 3;
/// DDL is ignored.
pub const SQL_TC_DDL_IGNORE: SQLSMALLINT = 4;

// SQLGetFunctions FunctionIds.
/// Supported.
pub const SQL_API_SQLALLOCCONNECT: SQLSMALLINT = 1;
/// Supported.
pub const SQL_API_SQLALLOCENV: SQLSMALLINT = 2;
/// Supported.
pub const SQL_API_SQLALLOCHANDLE: SQLSMALLINT = 1001;
/// Supported.
pub const SQL_API_SQLALLOCSTMT: SQLSMALLINT = 3;
/// Supported.
pub const SQL_API_SQLBINDCOL: SQLSMALLINT = 4;
/// Supported.
pub const SQL_API_SQLBINDPARAMETER: SQLSMALLINT = 72;
/// Supported.
pub const SQL_API_SQLCANCEL: SQLSMALLINT = 5;
/// Supported.
pub const SQL_API_SQLCLOSECURSOR: SQLSMALLINT = 1003;
/// Supported.
pub const SQL_API_SQLCOLATTRIBUTE: SQLSMALLINT = 6;
/// Not supported.
pub const SQL_API_SQLCOLUMNPRIVILEGES: SQLSMALLINT = 56;
/// Supported.
pub const SQL_API_SQLCOLUMNS: SQLSMALLINT = 40;
/// Supported.
pub const SQL_API_SQLCONNECT: SQLSMALLINT = 7;
/// Supported.
pub const SQL_API_SQLCOPYDESC: SQLSMALLINT = 1004;
/// Supported.
pub const SQL_API_SQLDATASOURCES: SQLSMALLINT = 57;
/// Supported.
pub const SQL_API_SQLDESCRIBECOL: SQLSMALLINT = 8;
/// Supported.
pub const SQL_API_SQLDISCONNECT: SQLSMALLINT = 9;
/// Supported.
pub const SQL_API_SQLENDTRAN: SQLSMALLINT = 1005;
/// Supported.
pub const SQL_API_SQLERROR: SQLSMALLINT = 10;
/// Supported.
pub const SQL_API_SQLEXECDIRECT: SQLSMALLINT = 11;
/// Supported.
pub const SQL_API_SQLEXECUTE: SQLSMALLINT = 12;
/// Supported.
pub const SQL_API_SQLFETCH: SQLSMALLINT = 13;
/// Supported.
pub const SQL_API_SQLFETCHSCROLL: SQLSMALLINT = 1021;
/// Supported.
pub const SQL_API_SQLFOREIGNKEYS: SQLSMALLINT = 60;
/// Supported.
pub const SQL_API_SQLFREECONNECT: SQLSMALLINT = 14;
/// Supported.
pub const SQL_API_SQLFREEENV: SQLSMALLINT = 15;
/// Supported.
pub const SQL_API_SQLFREEHANDLE: SQLSMALLINT = 1006;
/// Supported.
pub const SQL_API_SQLFREESTMT: SQLSMALLINT = 16;
/// Supported.
pub const SQL_API_SQLGETCONNECTATTR: SQLSMALLINT = 1007;
/// Supported.
pub const SQL_API_SQLGETCURSORNAME: SQLSMALLINT = 17;
/// Supported.
pub const SQL_API_SQLGETDATA: SQLSMALLINT = 43;
/// Supported.
pub const SQL_API_SQLGETDESCFIELD: SQLSMALLINT = 1008;
/// Supported.
pub const SQL_API_SQLGETDESCREC: SQLSMALLINT = 1009;
/// Supported.
pub const SQL_API_SQLGETDIAGFIELD: SQLSMALLINT = 1010;
/// Supported.
pub const SQL_API_SQLGETDIAGREC: SQLSMALLINT = 1011;
/// Supported.
pub const SQL_API_SQLGETENVATTR: SQLSMALLINT = 1012;
/// Supported.
pub const SQL_API_SQLGETFEATUREINFO: SQLSMALLINT = 1027;
/// Supported.
pub const SQL_API_SQLGETFUNCTIONS: SQLSMALLINT = 44;
/// Supported.
pub const SQL_API_SQLGETINFO: SQLSMALLINT = 45;
/// Not supported.
pub const SQL_API_SQLGETLENGTH: SQLSMALLINT = 1022;
/// Not supported.
pub const SQL_API_SQLGETPARAMDATA: SQLSMALLINT = 1025;
/// Not supported.
pub const SQL_API_SQLGETPOSITION: SQLSMALLINT = 1023;
/// Not supported.
pub const SQL_API_SQLGETSESSIONINFO: SQLSMALLINT = 1028;
/// Supported.
pub const SQL_API_SQLGETSTMTATTR: SQLSMALLINT = 1014;
/// Not supported.
pub const SQL_API_SQLGETSUBSTRING: SQLSMALLINT = 1024;
/// Supported.
pub const SQL_API_SQLGETTYPEINFO: SQLSMALLINT = 47;
/// Supported.
pub const SQL_API_SQLMORERESULTS: SQLSMALLINT = 61;
/// Supported.
pub const SQL_API_SQLNEXTRESULT: SQLSMALLINT = 73;
/// Supported.
pub const SQL_API_SQLNUMRESULTCOLS: SQLSMALLINT = 18;
/// Supported.
pub const SQL_API_SQLPARAMDATA: SQLSMALLINT = 48;
/// Supported.
pub const SQL_API_SQLPREPARE: SQLSMALLINT = 19;
/// Supported.
pub const SQL_API_SQLPRIMARYKEYS: SQLSMALLINT = 65;
/// Supported.
pub const SQL_API_SQLPUTDATA: SQLSMALLINT = 49;
/// Supported.
pub const SQL_API_SQLROWCOUNT: SQLSMALLINT = 20;
/// Supported.
pub const SQL_API_SQLSETCONNECTATTR: SQLSMALLINT = 1016;
/// Supported.
pub const SQL_API_SQLSETCURSORNAME: SQLSMALLINT = 21;
/// Supported.
pub const SQL_API_SQLSETDESCFIELD: SQLSMALLINT = 1017;
/// Supported.
pub const SQL_API_SQLSETDESCREC: SQLSMALLINT = 1018;
/// Supported.
pub const SQL_API_SQLSETENVATTR: SQLSMALLINT = 1019;
/// Supported.
pub const SQL_API_SQLSETSTMTATTR: SQLSMALLINT = 1020;
/// Supported.
pub const SQL_API_SQLSPECIALCOLUMNS: SQLSMALLINT = 52;
/// Supported.
pub const SQL_API_SQLSTARTTRAN: SQLSMALLINT = 74;
/// Not supported.
pub const SQL_API_SQLTABLEPRIVILEGES: SQLSMALLINT = 70;
/// Supported.
pub const SQL_API_SQLTABLES: SQLSMALLINT = 54;

// SQLBindParameter InputOutputModes.
/// An input parameter.
pub const SQL_PARAM_MODE_IN: SQLSMALLINT = 1;
/// The same as an input parameter.
pub const SQL_PARAM_MODE_INOUT: SQLSMALLINT = 2;
/// Not supported.
pub const SQL_PARAM_MODE_OUT: SQLSMALLINT = 4;

// SQLFetchScroll FetchOrientations.
/// The next rowset.
pub const SQL_FETCH_NEXT: SQLSMALLINT = 1;
/// The first rowset.
pub const SQL_FETCH_FIRST: SQLSMALLINT = 2;
/// The last rowset.
pub const SQL_FETCH_LAST: SQLSMALLINT = 3;
/// The rowset before.
pub const SQL_FETCH_PRIOR: SQLSMALLINT = 4;
/// From the first (or last) row.
pub const SQL_FETCH_ABSOLUTE: SQLSMALLINT = 5;
/// From the first row of the rowset.
pub const SQL_FETCH_RELATIVE: SQLSMALLINT = 6;

// The status of each row of a rowset (SQL_DESC_ARRAY_STATUS_POINTER).
/// Fetched.
pub const SQL_ROW_SUCCESS: SQLINTEGER = 0;
/// There isn't a row.
pub const SQL_ROW_NO_ROW: SQLINTEGER = 3;
/// Couldn't be fetched.
pub const SQL_ROW_ERROR: SQLINTEGER = 5;
/// Fetched with a warning.
pub const SQL_ROW_SUCCESS_WITH_INFO: SQLINTEGER = 6;

// Statement attributes.
/// SQL_HOLDABLE (the default).
pub const SQL_ATTR_CURSOR_HOLDABLE: SQLINTEGER = -3;
/// SQL_ASENSITIVE (the default).
pub const SQL_ATTR_CURSOR_SENSITIVITY: SQLINTEGER = -2;
/// SQL_NONSCROLLABLE (the default).
pub const SQL_ATTR_CURSOR_SCROLLABLE: SQLINTEGER = -1;
/// The current ARD.
pub const SQL_ATTR_APP_ROW_DESC: SQLINTEGER = 10010;
/// The current APD.
pub const SQL_ATTR_APP_PARAM_DESC: SQLINTEGER = 10011;
/// The IRD (read only).
pub const SQL_ATTR_IMP_ROW_DESC: SQLINTEGER = 10012;
/// The IPD (read only).
pub const SQL_ATTR_IMP_PARAM_DESC: SQLINTEGER = 10013;
/// SQL_FALSE (the default).
pub const SQL_ATTR_METADATA_ID: SQLINTEGER = 10014;
/// The row of the rowset.
pub const SQL_ATTR_CURRENT_OF_POSITION: SQLINTEGER = 10027;
/// SQL_FALSE (the default).
pub const SQL_ATTR_NEST_DESCRIPTOR: SQLINTEGER = 10029;

// Values of the cursor statement attributes.
/// Only SQL_FETCH_NEXT.
pub const SQL_NONSCROLLABLE: SQLINTEGER = 0;
/// Any fetch orientation.
pub const SQL_SCROLLABLE: SQLINTEGER = 1;
/// May see changes (but doesn't).
pub const SQL_ASENSITIVE: SQLINTEGER = 0;
/// Doesn't see changes.
pub const SQL_INSENSITIVE: SQLINTEGER = 1;
/// Not supported.
pub const SQL_SENSITIVE: SQLINTEGER = 2;
/// Closed by a commit.
pub const SQL_NONHOLDABLE: SQLINTEGER = 0;
/// Stays open after a commit.
pub const SQL_HOLDABLE: SQLINTEGER = 1;

// Descriptor header fields.
/// The number of rows.
pub const SQL_DESC_ARRAY_SIZE: SQLSMALLINT = 20;
/// SQLINTEGER[] row statuses.
pub const SQL_DESC_ARRAY_STATUS_POINTER: SQLSMALLINT = 21;
/// SQLINTEGER * rows fetched.
pub const SQL_DESC_ROWS_PROCESSED_POINTER: SQLSMALLINT = 34;
/// The number of records.
pub const SQL_DESC_COUNT: SQLSMALLINT = 1001;
pub const SQL_DESC_KEY_TYPE: SQLSMALLINT = 1029;
/// The statement.
pub const SQL_DESC_DYNAMIC_FUNCTION: SQLSMALLINT = 1031;
/// The code of the statement.
pub const SQL_DESC_DYNAMIC_FUNCTION_CODE: SQLSMALLINT = 1032;
/// The number of columns.
pub const SQL_DESC_TOP_LEVEL_COUNT: SQLSMALLINT = 1044;
/// SQL_DESC_ALLOC_AUTO or USER.
pub const SQL_DESC_ALLOC_TYPE: SQLSMALLINT = 1099;

// Descriptor record fields.
/// Leading field precision.
pub const SQL_DESC_DATETIME_INTERVAL_PRECISION: SQLSMALLINT = 26;
/// The data type.
pub const SQL_DESC_TYPE: SQLSMALLINT = 1002;
/// In characters.
pub const SQL_DESC_LENGTH: SQLSMALLINT = 1003;
/// SQLINTEGER *length.
pub const SQL_DESC_OCTET_LENGTH_POINTER: SQLSMALLINT = 1004;
/// The precision.
pub const SQL_DESC_PRECISION: SQLSMALLINT = 1005;
/// The scale.
pub const SQL_DESC_SCALE: SQLSMALLINT = 1006;
/// Such as SQL_CODE_DATE.
pub const SQL_DESC_DATETIME_INTERVAL_CODE: SQLSMALLINT = 1007;
/// SQL_NULLABLE.
pub const SQL_DESC_NULLABLE: SQLSMALLINT = 1008;
/// SQLINTEGER * indicator.
pub const SQL_DESC_INDICATOR_POINTER: SQLSMALLINT = 1009;
/// The value.
pub const SQL_DESC_DATA_POINTER: SQLSMALLINT = 1010;
/// The name of the column.
pub const SQL_DESC_NAME: SQLSMALLINT = 1011;
/// SQL_UNNAMED or SQL_NAMED.
pub const SQL_DESC_UNNAMED: SQLSMALLINT = 1012;
/// The length in octets.
pub const SQL_DESC_OCTET_LENGTH: SQLSMALLINT = 1013;
/// Always "".
pub const SQL_DESC_COLLATION_CATALOG: SQLSMALLINT = 1015;
/// Always "".
pub const SQL_DESC_COLLATION_SCHEMA: SQLSMALLINT = 1016;
/// Always "".
pub const SQL_DESC_COLLATION_NAME: SQLSMALLINT = 1017;
/// The current catalog.
pub const SQL_DESC_CHARACTER_SET_CATALOG: SQLSMALLINT = 1018;
/// "INFORMATION_SCHEMA".
pub const SQL_DESC_CHARACTER_SET_SCHEMA: SQLSMALLINT = 1019;
/// "UTF8".
pub const SQL_DESC_CHARACTER_SET_NAME: SQLSMALLINT = 1020;
/// SQL_PARAM_MODE_IN.
pub const SQL_DESC_PARAMETER_MODE: SQLSMALLINT = 1021;
/// Not used.
pub const SQL_DESC_PARAMETER_ORDINAL_POSITION: SQLSMALLINT = 1022;
/// Not used.
pub const SQL_DESC_PARAMETER_SPECIFIC_CATALOG: SQLSMALLINT = 1023;
/// Not used.
pub const SQL_DESC_PARAMETER_SPECIFIC_SCHEMA: SQLSMALLINT = 1024;
/// Not used.
pub const SQL_DESC_PARAMETER_SPECIFIC_NAME: SQLSMALLINT = 1025;
/// Not used.
pub const SQL_DESC_UDT_CATALOG: SQLSMALLINT = 1026;
/// Not used.
pub const SQL_DESC_UDT_SCHEMA: SQLSMALLINT = 1027;
/// Not used.
pub const SQL_DESC_UDT_NAME: SQLSMALLINT = 1028;
pub const SQL_DESC_KEY_MEMBER: SQLSMALLINT = 1030;
/// Not used.
pub const SQL_DESC_SCOPE_CATALOG: SQLSMALLINT = 1033;
/// Not used.
pub const SQL_DESC_SCOPE_SCHEMA: SQLSMALLINT = 1034;
/// Not used.
pub const SQL_DESC_SCOPE_NAME: SQLSMALLINT = 1035;
/// Not used.
pub const SQL_DESC_SPECIFIC_TYPE_CATALOG: SQLSMALLINT = 1036;
/// Not used.
pub const SQL_DESC_SPECIFIC_TYPE_SCHEMA: SQLSMALLINT = 1037;
/// Not used.
pub const SQL_DESC_SPECIFIC_TYPE_NAME: SQLSMALLINT = 1038;
/// Not used.
pub const SQL_DESC_CURRENT_TRANSFORM_GROUP: SQLSMALLINT = 1039;
/// Not used.
pub const SQL_DESC_CARDINALITY: SQLSMALLINT = 1040;
/// Not used.
pub const SQL_DESC_DEGREE: SQLSMALLINT = 1041;
/// Always 0.
pub const SQL_DESC_LEVEL: SQLSMALLINT = 1042;
/// Not used.
pub const SQL_DESC_RETURNED_CARDINALITY_POINTER: SQLSMALLINT = 1043;
/// Not used.
pub const SQL_DESC_USER_DEFINED_TYPE_CODE: SQLSMALLINT = 1045;

// Other descriptor values.
/// Allocated with a statement.
pub const SQL_DESC_ALLOC_AUTO: SQLSMALLINT = 1;
/// Allocated with SQLAllocHandle.
pub const SQL_DESC_ALLOC_USER: SQLSMALLINT = 2;
/// The column has a name.
pub const SQL_NAMED: SQLSMALLINT = 0;
/// The column doesn't have a name.
pub const SQL_UNNAMED: SQLSMALLINT = 1;

// SQLFreeStmt options (and their ODBC names).
/// Close the cursor.
pub const SQL_CLOSE_CURSOR: SQLSMALLINT = 0;
/// Free the statement.
pub const SQL_FREE_HANDLE: SQLSMALLINT = 1;
/// Unbind every column.
pub const SQL_UNBIND_COLUMNS: SQLSMALLINT = 2;
/// Unbind every parameter.
pub const SQL_UNBIND_PARAMETERS: SQLSMALLINT = 3;
/// Forget the statement.
pub const SQL_REALLOCATE: SQLSMALLINT = 4;
/// SQL_CLOSE_CURSOR.
pub const SQL_CLOSE: SQLSMALLINT = 0;
/// SQL_FREE_HANDLE.
pub const SQL_DROP: SQLSMALLINT = 1;
/// SQL_UNBIND_COLUMNS.
pub const SQL_UNBIND: SQLSMALLINT = 2;
/// SQL_UNBIND_PARAMETERS.
pub const SQL_RESET_PARAMS: SQLSMALLINT = 3;

// Nullability.
/// The column is never NULL.
pub const SQL_NO_NULLS: SQLSMALLINT = 0;
/// The column may be NULL.
pub const SQL_NULLABLE: SQLSMALLINT = 1;

extern "C" {
    /// SQLAllocConnect is the same as SQLAllocHandle(SQL_HANDLE_DBC,
    /// EnvironmentHandle, ConnectionHandle).
    pub fn SQLAllocConnect(EnvironmentHandle: SQLHENV, ConnectionHandle: *mut SQLHDBC)
        -> SQLRETURN;

    /// SQLAllocEnv is the same as SQLAllocHandle(SQL_HANDLE_ENV,
    /// SQL_NULL_HANDLE, EnvironmentHandle).
    pub fn SQLAllocEnv(EnvironmentHandle: *mut SQLHENV) -> SQLRETURN;

    /// SQLAllocHandle allocates an environment (SQL_HANDLE_ENV), a connection
    /// (SQL_HANDLE_DBC) for an environment, or a statement (SQL_HANDLE_STMT) or
    /// descriptor (SQL_HANDLE_DESC, see SQLGetDescField) for an established
    /// connection.
    pub fn SQLAllocHandle(
        HandleType: SQLSMALLINT,
        InputHandle: SQLINTEGER,
        OutputHandle: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLAllocStmt is the same as SQLAllocHandle(SQL_HANDLE_STMT,
    /// ConnectionHandle, StatementHandle).
    pub fn SQLAllocStmt(ConnectionHandle: SQLHDBC, StatementHandle: *mut SQLHSTMT) -> SQLRETURN;

    /// SQLBindCol binds a column (starting at 1) so that SQLFetch and
    /// SQLFetchScroll set TargetValue (of TargetType, see SQLGetData) and
    /// StrLen_or_Ind.
    pub fn SQLBindCol(
        StatementHandle: SQLHSTMT,
        ColumnNumber: SQLSMALLINT,
        TargetType: SQLSMALLINT,
        TargetValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StrLen_or_Ind: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLBindParameter binds a dynamic parameter ("?", starting at 1) of a
    /// statement.
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

    /// SQLCancel cancels a routine that's running on the statement.
    pub fn SQLCancel(StatementHandle: SQLHSTMT) -> SQLRETURN;

    /// SQLCloseCursor closes the cursor opened by SQLExecDirect so that the
    /// statement can be used to execute another statement.
    pub fn SQLCloseCursor(StatementHandle: SQLHSTMT) -> SQLRETURN;

    /// SQLColAttribute reads a field of the IRD (see SQLGetDescField) for a
    /// column (starting at 1) of the executed statement.
    pub fn SQLColAttribute(
        StatementHandle: SQLHSTMT,
        ColumnNumber: SQLSMALLINT,
        FieldIdentifier: SQLSMALLINT,
        CharacterAttribute: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
        NumericAttribute: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLColumnPrivileges would return a result set that describes the
    /// privileges of columns.
    pub fn SQLColumnPrivileges(
        StatementHandle: SQLHSTMT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
        ColumnName: *const SQLCHAR,
        NameLength4: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLColumns returns a result set (like a query) that describes the
    /// columns of tables, from the INFORMATION_SCHEMA.
    pub fn SQLColumns(
        StatementHandle: SQLHSTMT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
        ColumnName: *const SQLCHAR,
        NameLength4: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLConnect opens a database.
    pub fn SQLConnect(
        ConnectionHandle: SQLHDBC,
        ServerName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        UserName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        Authentication: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLCopyDesc replaces the fields and records of TargetDescHandle with
    /// those of SourceDescHandle (except for SQL_DESC_ALLOC_TYPE).
    pub fn SQLCopyDesc(SourceDescHandle: SQLHDESC, TargetDescHandle: SQLHDESC) -> SQLRETURN;

    /// SQLDataSources returns the ServerNames that SQLConnect accepts without a
    /// file path (see libvsql.h).
    pub fn SQLDataSources(
        EnvironmentHandle: SQLHENV,
        Direction: SQLSMALLINT,
        ServerName: *mut SQLCHAR,
        BufferLength1: SQLSMALLINT,
        NameLength1: *mut SQLSMALLINT,
        Description: *mut SQLCHAR,
        BufferLength2: SQLSMALLINT,
        NameLength2: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLDescribeCol describes a column (starting at 1) of the cursor.
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

    /// SQLDisconnect closes the database and frees the statements of the
    /// connection.
    pub fn SQLDisconnect(ConnectionHandle: SQLHDBC) -> SQLRETURN;

    /// SQLEndTran commits (SQL_COMMIT) or rolls back (SQL_ROLLBACK) the
    /// transaction of a connection (SQL_HANDLE_DBC), or of every connection of
    /// an environment (SQL_HANDLE_ENV).
    pub fn SQLEndTran(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        CompletionType: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLError reads the next diagnostic record that it hasn't already read
    /// for the last routine called with a handle.
    pub fn SQLError(
        EnvironmentHandle: SQLHENV,
        ConnectionHandle: SQLHDBC,
        StatementHandle: SQLHSTMT,
        Sqlstate: *mut SQLCHAR,
        NativeError: *mut SQLINTEGER,
        MessageText: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        TextLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLExecDirect executes an SQL statement.
    pub fn SQLExecDirect(
        StatementHandle: SQLHSTMT,
        StatementText: *const SQLCHAR,
        TextLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLExecute executes the statement prepared by SQLPrepare (which can be
    /// executed again), the same as SQLExecDirect.
    pub fn SQLExecute(StatementHandle: SQLHSTMT) -> SQLRETURN;

    /// SQLFetch moves the cursor to the next row, which is read with SQLGetData
    /// (or set in the columns bound with SQLBindCol).
    pub fn SQLFetch(StatementHandle: SQLHSTMT) -> SQLRETURN;

    /// SQLFetchScroll moves the cursor to a rowset (see SQL_FETCH_NEXT and the
    /// others).
    pub fn SQLFetchScroll(
        StatementHandle: SQLHSTMT,
        FetchOrientation: SQLSMALLINT,
        FetchOffset: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLForeignKeys returns a result set that describes the columns of
    /// foreign keys, from the INFORMATION_SCHEMA.
    pub fn SQLForeignKeys(
        StatementHandle: SQLHSTMT,
        PKCatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        PKSchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        PKTableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
        FKCatalogName: *const SQLCHAR,
        NameLength4: SQLSMALLINT,
        FKSchemaName: *const SQLCHAR,
        NameLength5: SQLSMALLINT,
        FKTableName: *const SQLCHAR,
        NameLength6: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLFreeConnect is the same as SQLFreeHandle(SQL_HANDLE_DBC,
    /// ConnectionHandle).
    pub fn SQLFreeConnect(ConnectionHandle: SQLHDBC) -> SQLRETURN;

    /// SQLFreeEnv is the same as SQLFreeHandle(SQL_HANDLE_ENV,
    /// EnvironmentHandle).
    pub fn SQLFreeEnv(EnvironmentHandle: SQLHENV) -> SQLRETURN;

    /// SQLFreeHandle frees a handle from SQLAllocHandle.
    pub fn SQLFreeHandle(HandleType: SQLSMALLINT, Handle: SQLINTEGER) -> SQLRETURN;

    /// SQLFreeStmt does one of the following, depending on Option (see
    /// libvsql.h).
    pub fn SQLFreeStmt(StatementHandle: SQLHSTMT, Option: SQLSMALLINT) -> SQLRETURN;

    /// SQLGetConnectAttr reads an attribute of a connection (see libvsql.h).
    pub fn SQLGetConnectAttr(
        ConnectionHandle: SQLHDBC,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetCursorName reads the name of the statement's cursor (see
    /// SQLGetData for SQL_CHAR).
    pub fn SQLGetCursorName(
        StatementHandle: SQLHSTMT,
        CursorName: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        NameLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetData reads a column (starting at 1) of the fetched row, converted
    /// to TargetType (see libvsql.h).
    pub fn SQLGetData(
        StatementHandle: SQLHSTMT,
        ColumnNumber: SQLSMALLINT,
        TargetType: SQLSMALLINT,
        TargetValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StrLen_or_Ind: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetDescField reads a field of a descriptor.
    pub fn SQLGetDescField(
        DescriptorHandle: SQLHDESC,
        RecordNumber: SQLSMALLINT,
        FieldIdentifier: SQLSMALLINT,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetDescRec reads the SQL_DESC_NAME, SQL_DESC_TYPE,
    /// SQL_DESC_DATETIME_INTERVAL_CODE (SubType), SQL_DESC_OCTET_LENGTH
    /// (Length), SQL_DESC_PRECISION, SQL_DESC_SCALE and SQL_DESC_NULLABLE
    /// fields of a record (starting at 1) of a descriptor.
    pub fn SQLGetDescRec(
        DescriptorHandle: SQLHDESC,
        RecordNumber: SQLSMALLINT,
        Name: *mut SQLCHAR,
        BufferLength: SQLSMALLINT,
        NameLength: *mut SQLSMALLINT,
        Type: *mut SQLSMALLINT,
        SubType: *mut SQLSMALLINT,
        Length: *mut SQLINTEGER,
        Precision: *mut SQLSMALLINT,
        Scale: *mut SQLSMALLINT,
        Nullable: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetDiagField reads one field of the diagnostics for the last routine
    /// that was called with a handle (other than SQLError, SQLGetDiagField and
    /// SQLGetDiagRec).
    pub fn SQLGetDiagField(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        RecordNumber: SQLSMALLINT,
        DiagIdentifier: SQLSMALLINT,
        DiagInfo: SQLPOINTER,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetDiagRec reads a diagnostic record (starting at 1) for the last
    /// routine that was called with the handle.
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

    /// SQLGetEnvAttr reads an attribute of an environment.
    pub fn SQLGetEnvAttr(
        EnvironmentHandle: SQLHENV,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetFeatureInfo sets Supported to 1 if vsql supports a feature
    /// ("FEATURE" with an id such as "E011") or a subfeature ("SUBFEATURE" with
    /// "E011" and "01"), otherwise 0.
    pub fn SQLGetFeatureInfo(
        ConnectionHandle: SQLHDBC,
        FeatureType: *const SQLCHAR,
        FeatureTypeLength: SQLSMALLINT,
        FeatureId: *const SQLCHAR,
        FeatureIdLength: SQLSMALLINT,
        SubFeatureId: *const SQLCHAR,
        SubFeatureIdLength: SQLSMALLINT,
        Supported: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetFunctions sets Supported to 1 if a routine (one of the SQL_API_*
    /// FunctionIds) is supported, otherwise 0.
    pub fn SQLGetFunctions(
        ConnectionHandle: SQLHDBC,
        FunctionId: SQLSMALLINT,
        Supported: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetInfo reads information about the connection and vsql.
    pub fn SQLGetInfo(
        ConnectionHandle: SQLHDBC,
        InfoType: SQLSMALLINT,
        InfoValue: SQLPOINTER,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetLength would read the length of the value of a large object
    /// locator.
    pub fn SQLGetLength(
        StatementHandle: SQLHSTMT,
        LocatorType: SQLSMALLINT,
        Locator: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
        IndicatorValue: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetParamData would read the value of a dynamic output parameter of a
    /// CALL statement.
    pub fn SQLGetParamData(
        StatementHandle: SQLHSTMT,
        ParameterNumber: SQLSMALLINT,
        TargetType: SQLSMALLINT,
        TargetValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StrLen_or_Ind: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetPosition would find a string within the value of a large object
    /// locator.
    pub fn SQLGetPosition(
        StatementHandle: SQLHSTMT,
        LocatorType: SQLSMALLINT,
        SourceLocator: SQLINTEGER,
        SearchLocator: SQLINTEGER,
        SearchLiteral: *const SQLCHAR,
        SearchLiteralLength: SQLINTEGER,
        FromPosition: SQLINTEGER,
        LocatedAt: *mut SQLINTEGER,
        IndicatorValue: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetSessionInfo reads the value of CURRENT_USER (SQL_CURRENT_USER),
    /// SESSION_USER (SQL_SESSION_USER), etc.
    pub fn SQLGetSessionInfo(
        ConnectionHandle: SQLHDBC,
        InfoType: SQLSMALLINT,
        InfoValue: SQLPOINTER,
        BufferLength: SQLSMALLINT,
        StringLength: *mut SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLGetStmtAttr reads an attribute of a statement (an SQLINTEGER), such
    /// as SQL_ATTR_IMP_ROW_DESC (see SQLGetDescField) or
    /// SQL_ATTR_CURSOR_SCROLLABLE.
    pub fn SQLGetStmtAttr(
        StatementHandle: SQLHSTMT,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetSubString would read part of the value of a large object locator.
    pub fn SQLGetSubString(
        StatementHandle: SQLHSTMT,
        LocatorType: SQLSMALLINT,
        SourceLocator: SQLINTEGER,
        FromPosition: SQLINTEGER,
        ForLength: SQLINTEGER,
        TargetType: SQLSMALLINT,
        TargetValue: SQLPOINTER,
        BufferLength: SQLINTEGER,
        StringLength: *mut SQLINTEGER,
        IndicatorValue: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLGetTypeInfo returns a result set that describes a data type (such as
    /// SQL_VARCHAR), or each data type (SQL_ALL_TYPES).
    pub fn SQLGetTypeInfo(StatementHandle: SQLHSTMT, DataType: SQLSMALLINT) -> SQLRETURN;

    /// SQLMoreResults closes the cursor, and moves to the next result set of
    /// the executed statement.
    pub fn SQLMoreResults(StatementHandle: SQLHSTMT) -> SQLRETURN;

    /// SQLNextResult opens the next result set of StatementHandle1 on
    /// StatementHandle2.
    pub fn SQLNextResult(StatementHandle1: SQLHSTMT, StatementHandle2: SQLHSTMT) -> SQLRETURN;

    /// SQLNumResultCols sets ColumnCount to the number of columns of the
    /// cursor, or 0 for a statement that isn't a query.
    pub fn SQLNumResultCols(StatementHandle: SQLHSTMT, ColumnCount: *mut SQLSMALLINT) -> SQLRETURN;

    /// SQLParamData gives the value of each SQL_DATA_AT_EXEC parameter, after
    /// SQLExecute (or SQLExecDirect) returns SQL_NEED_DATA.
    pub fn SQLParamData(StatementHandle: SQLHSTMT, Value: *mut SQLPOINTER) -> SQLRETURN;

    /// SQLPrepare prepares a statement to be executed (maybe many times) with
    /// SQLExecute.
    pub fn SQLPrepare(
        StatementHandle: SQLHSTMT,
        StatementText: *const SQLCHAR,
        TextLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLPrimaryKeys returns a result set that describes the columns of the
    /// PRIMARY KEY of a table (TableName can't be a null pointer).
    pub fn SQLPrimaryKeys(
        StatementHandle: SQLHSTMT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLPutData gives the value of the SQL_DATA_AT_EXEC parameter, see
    /// SQLParamData.
    pub fn SQLPutData(
        StatementHandle: SQLHSTMT,
        Data: SQLPOINTER,
        StrLen_or_Ind: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLRowCount sets RowCount to the number of rows inserted, updated or
    /// deleted by the last statement that was executed, or 0 for any other
    /// statement.
    pub fn SQLRowCount(StatementHandle: SQLHSTMT, RowCount: *mut SQLINTEGER) -> SQLRETURN;

    /// SQLSetConnectAttr sets an attribute of a connection (see libvsql.h).
    pub fn SQLSetConnectAttr(
        ConnectionHandle: SQLHDBC,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        StringLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLSetCursorName names the statement's cursor.
    pub fn SQLSetCursorName(
        StatementHandle: SQLHSTMT,
        CursorName: *const SQLCHAR,
        NameLength: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLSetDescField sets a field of a descriptor (see SQLGetDescField).
    pub fn SQLSetDescField(
        DescriptorHandle: SQLHDESC,
        RecordNumber: SQLSMALLINT,
        FieldIdentifier: SQLSMALLINT,
        Value: SQLPOINTER,
        BufferLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLSetDescRec sets the SQL_DESC_TYPE, SQL_DESC_DATETIME_INTERVAL_CODE
    /// (SubType), SQL_DESC_PRECISION, SQL_DESC_SCALE, SQL_DESC_DATA_POINTER
    /// (Data), SQL_DESC_OCTET_LENGTH_POINTER (StringLength) and
    /// SQL_DESC_INDICATOR_POINTER (Indicator) fields of a record (starting at
    /// 1).
    pub fn SQLSetDescRec(
        DescriptorHandle: SQLHDESC,
        RecordNumber: SQLSMALLINT,
        Type: SQLSMALLINT,
        SubType: SQLSMALLINT,
        Length: SQLINTEGER,
        Precision: SQLSMALLINT,
        Scale: SQLSMALLINT,
        Data: SQLPOINTER,
        StringLength: *mut SQLINTEGER,
        Indicator: *mut SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLSetEnvAttr sets an attribute of an environment, before it has any
    /// connections.
    pub fn SQLSetEnvAttr(
        EnvironmentHandle: SQLHENV,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        StringLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLSetStmtAttr sets an attribute of a statement.
    pub fn SQLSetStmtAttr(
        StatementHandle: SQLHSTMT,
        Attribute: SQLINTEGER,
        Value: SQLPOINTER,
        StringLength: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLSpecialColumns returns a result set that describes the columns that
    /// identify a row of a table (SQL_BEST_ROWID).
    pub fn SQLSpecialColumns(
        StatementHandle: SQLHSTMT,
        IdentifierType: SQLSMALLINT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
        Scope: SQLSMALLINT,
        Nullable: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLStartTran starts a transaction for a connection (SQL_HANDLE_DBC),
    /// which becomes the current connection (see SQLConnect).
    pub fn SQLStartTran(
        HandleType: SQLSMALLINT,
        Handle: SQLINTEGER,
        AccessMode: SQLINTEGER,
        IsolationLevel: SQLINTEGER,
    ) -> SQLRETURN;

    /// SQLTablePrivileges would return a result set that describes the
    /// privileges of tables.
    pub fn SQLTablePrivileges(
        StatementHandle: SQLHSTMT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
    ) -> SQLRETURN;

    /// SQLTables returns a result set that describes tables, from the
    /// INFORMATION_SCHEMA.
    pub fn SQLTables(
        StatementHandle: SQLHSTMT,
        CatalogName: *const SQLCHAR,
        NameLength1: SQLSMALLINT,
        SchemaName: *const SQLCHAR,
        NameLength2: SQLSMALLINT,
        TableName: *const SQLCHAR,
        NameLength3: SQLSMALLINT,
        TableType: *const SQLCHAR,
        NameLength4: SQLSMALLINT,
    ) -> SQLRETURN;

    /// vsql_module_agent isn't part of the SQL/CLI.
    pub fn vsql_module_agent() -> *mut *mut c_void;
}

use vsql_sys::*;

/// Every routine is linked (a routine that's declared but not defined in
/// libvsql would only fail when it's used).
#[test]
fn every_routine_links() {
    let routines = [
        SQLAllocConnect as *const (),
        SQLAllocEnv as *const (),
        SQLAllocHandle as *const (),
        SQLAllocStmt as *const (),
        SQLBindCol as *const (),
        SQLBindParameter as *const (),
        SQLCancel as *const (),
        SQLCloseCursor as *const (),
        SQLColAttribute as *const (),
        SQLColumnPrivileges as *const (),
        SQLColumns as *const (),
        SQLConnect as *const (),
        SQLCopyDesc as *const (),
        SQLDataSources as *const (),
        SQLDescribeCol as *const (),
        SQLDisconnect as *const (),
        SQLEndTran as *const (),
        SQLError as *const (),
        SQLExecDirect as *const (),
        SQLExecute as *const (),
        SQLFetch as *const (),
        SQLFetchScroll as *const (),
        SQLForeignKeys as *const (),
        SQLFreeConnect as *const (),
        SQLFreeEnv as *const (),
        SQLFreeHandle as *const (),
        SQLFreeStmt as *const (),
        SQLGetConnectAttr as *const (),
        SQLGetCursorName as *const (),
        SQLGetData as *const (),
        SQLGetDescField as *const (),
        SQLGetDescRec as *const (),
        SQLGetDiagField as *const (),
        SQLGetDiagRec as *const (),
        SQLGetEnvAttr as *const (),
        SQLGetFeatureInfo as *const (),
        SQLGetFunctions as *const (),
        SQLGetInfo as *const (),
        SQLGetLength as *const (),
        SQLGetParamData as *const (),
        SQLGetPosition as *const (),
        SQLGetSessionInfo as *const (),
        SQLGetStmtAttr as *const (),
        SQLGetSubString as *const (),
        SQLGetTypeInfo as *const (),
        SQLMoreResults as *const (),
        SQLNextResult as *const (),
        SQLNumResultCols as *const (),
        SQLParamData as *const (),
        SQLPrepare as *const (),
        SQLPrimaryKeys as *const (),
        SQLPutData as *const (),
        SQLRowCount as *const (),
        SQLSetConnectAttr as *const (),
        SQLSetCursorName as *const (),
        SQLSetDescField as *const (),
        SQLSetDescRec as *const (),
        SQLSetEnvAttr as *const (),
        SQLSetStmtAttr as *const (),
        SQLSpecialColumns as *const (),
        SQLStartTran as *const (),
        SQLTablePrivileges as *const (),
        SQLTables as *const (),
        vsql_module_agent as *const (),
    ];
    assert!(routines.iter().all(|routine| !routine.is_null()));
}

/// Some of the routines that the vsql crate doesn't use.
#[test]
fn routines() {
    unsafe {
        let mut env = 0;
        let mut dbc = 0;
        let mut stmt = 0;
        assert_eq!(SQLAllocEnv(&mut env), SQL_SUCCESS);
        assert_eq!(SQLAllocConnect(env, &mut dbc), SQL_SUCCESS);
        let name = b":memory:";
        assert_eq!(
            SQLConnect(
                dbc,
                name.as_ptr(),
                name.len() as SQLSMALLINT,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0
            ),
            SQL_SUCCESS
        );

        let mut supported = SQL_FALSE;
        assert_eq!(
            SQLGetFunctions(dbc, SQL_API_SQLBINDCOL, &mut supported),
            SQL_SUCCESS
        );
        assert_eq!(supported, SQL_TRUE);
        assert_eq!(
            SQLGetFunctions(dbc, SQL_API_SQLGETLENGTH, &mut supported),
            SQL_SUCCESS
        );
        assert_eq!(supported, SQL_FALSE);

        let feature = b"FEATURE";
        let id = b"E011";
        assert_eq!(
            SQLGetFeatureInfo(
                dbc,
                feature.as_ptr(),
                feature.len() as SQLSMALLINT,
                id.as_ptr(),
                id.len() as SQLSMALLINT,
                core::ptr::null(),
                0,
                &mut supported
            ),
            SQL_SUCCESS
        );
        assert_eq!(supported, SQL_TRUE);

        // A bound column, and a scrollable cursor.
        assert_eq!(SQLAllocStmt(dbc, &mut stmt), SQL_SUCCESS);
        let mut scrollable = SQL_SCROLLABLE;
        assert_eq!(
            SQLSetStmtAttr(
                stmt,
                SQL_ATTR_CURSOR_SCROLLABLE,
                &mut scrollable as *mut SQLINTEGER as SQLPOINTER,
                0
            ),
            SQL_SUCCESS
        );
        let sql = b"VALUES 1, 2, 3";
        assert_eq!(
            SQLExecDirect(stmt, sql.as_ptr(), sql.len() as SQLINTEGER),
            SQL_SUCCESS
        );
        let mut x: SQLINTEGER = 0;
        let mut ind = 0;
        assert_eq!(
            SQLBindCol(
                stmt,
                1,
                SQL_INTEGER,
                &mut x as *mut SQLINTEGER as SQLPOINTER,
                4,
                &mut ind
            ),
            SQL_SUCCESS
        );
        assert_eq!(SQLFetchScroll(stmt, SQL_FETCH_LAST, 0), SQL_SUCCESS);
        assert_eq!(x, 3);
        assert_eq!(SQLFetchScroll(stmt, SQL_FETCH_FIRST, 0), SQL_SUCCESS);
        assert_eq!(x, 1);
        assert_eq!(SQLFreeStmt(stmt, SQL_CLOSE_CURSOR), SQL_SUCCESS);

        // A catalog routine with SQL_ALL_SCHEMAS.
        assert_eq!(
            SQLTables(
                stmt,
                core::ptr::null(),
                0,
                SQL_ALL_SCHEMAS.as_ptr().cast(),
                SQL_NTS as SQLSMALLINT,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0
            ),
            SQL_SUCCESS
        );
        assert_eq!(SQLFetch(stmt), SQL_SUCCESS);

        assert_eq!(SQLFreeHandle(SQL_HANDLE_STMT, stmt), SQL_SUCCESS);
        assert_eq!(SQLEndTran(SQL_HANDLE_DBC, dbc, SQL_ROLLBACK), SQL_SUCCESS);
        assert_eq!(SQLDisconnect(dbc), SQL_SUCCESS);
        assert_eq!(SQLFreeConnect(dbc), SQL_SUCCESS);
        assert_eq!(SQLFreeEnv(env), SQL_SUCCESS);
    }
}

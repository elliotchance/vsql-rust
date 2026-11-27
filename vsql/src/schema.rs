//! The schema of a database (its tables, columns and keys), from the SQL/CLI
//! catalog routines.

use vsql_sys::*;

use crate::connection::Connection;
use crate::error::Result;
use crate::raw;
use crate::statement::{Row, Statement};
use crate::types::Type;

/// The type of a table, see [`Table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableType {
    /// A base table (`CREATE TABLE`).
    Table,
    /// A view.
    View,
    /// A table of the `INFORMATION_SCHEMA`.
    SystemTable,
}

impl TableType {
    fn parse(s: &str) -> TableType {
        match s {
            "VIEW" => TableType::View,
            "SYSTEM TABLE" => TableType::SystemTable,
            _ => TableType::Table,
        }
    }
}

/// A table, see [`Connection::tables`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Table {
    /// The schema of the table.
    pub schema: String,
    /// The name of the table. Names that weren't quoted are upper case.
    pub name: String,
    pub table_type: TableType,
}

/// A column of a table, see [`Connection::columns`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TableColumn {
    /// The name of the column. Names that weren't quoted are upper case.
    pub name: String,
    pub ty: Type,
    /// Whether the column can be NULL.
    pub nullable: bool,
    /// The maximum number of characters of a CHARACTER, CHARACTER VARYING or
    /// CHARACTER LARGE OBJECT, or octets of a BINARY, BINARY VARYING or
    /// BINARY LARGE OBJECT.
    pub length: Option<usize>,
    /// The precision and scale of a DECIMAL or NUMERIC.
    pub precision_scale: Option<(u32, u32)>,
    /// The default value, as SQL (such as `'x'` or `CURRENT_DATE`).
    pub default: Option<String>,
}

/// The PRIMARY KEY of a table, see [`Connection::primary_key`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PrimaryKey {
    /// The name of the constraint.
    pub name: String,
    /// The columns of the key, in order.
    pub columns: Vec<String>,
}

/// What a foreign key does when the row that it references is updated or
/// deleted, see [`ForeignKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ForeignKeyAction {
    Cascade,
    Restrict,
    SetNull,
    NoAction,
    SetDefault,
}

impl ForeignKeyAction {
    fn from_code(code: i16) -> ForeignKeyAction {
        match code {
            SQL_CASCADE => ForeignKeyAction::Cascade,
            SQL_RESTRICT => ForeignKeyAction::Restrict,
            SQL_SET_NULL => ForeignKeyAction::SetNull,
            SQL_SET_DEFAULT => ForeignKeyAction::SetDefault,
            _ => ForeignKeyAction::NoAction,
        }
    }
}

/// A foreign key of a table, see [`Connection::foreign_keys`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ForeignKey {
    /// The name of the constraint.
    pub name: String,
    /// The columns of the foreign key, in order.
    pub columns: Vec<String>,
    /// The schema of the table that it references.
    pub referenced_schema: String,
    /// The table that it references.
    pub referenced_table: String,
    /// The columns of the PRIMARY KEY or UNIQUE constraint that it references,
    /// in the same order as `columns`.
    pub referenced_columns: Vec<String>,
    pub on_update: ForeignKeyAction,
    pub on_delete: ForeignKeyAction,
}

/// A name as a quoted identifier.
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A name as a LIKE pattern (for SQLTables and SQLColumns) that only matches
/// itself.
fn pattern(name: &str) -> String {
    let mut p = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(c, '\\' | '%' | '_') {
            p.push('\\');
        }
        p.push(c);
    }

    p
}

/// The schema of a database. Each name is as it's stored (a name that wasn't
/// quoted is upper case), and a schema of `None` is the current schema
/// (`PUBLIC` unless it was changed with `SET SCHEMA`). A table that doesn't
/// exist has no columns or keys.
impl Connection {
    /// The schema, or the current schema for None.
    fn schema_name(&self, schema: Option<&str>) -> Result<String> {
        match schema {
            Some(schema) => Ok(schema.to_string()),
            None => self.query_row("VALUES CURRENT_SCHEMA", (), |row| row.get(0)),
        }
    }

    /// Maps each row of a catalog routine (that's given the schema) with `f`.
    /// A name can't be empty, so there are no rows for an empty schema or
    /// table (which the routines would treat as any name).
    fn catalog<T, R, F>(
        &self,
        schema: Option<&str>,
        table: &str,
        routine: R,
        mut f: F,
    ) -> Result<Vec<T>>
    where
        R: FnOnce(SQLHSTMT, &str) -> Result<()>,
        F: FnMut(&Row) -> Result<T>,
    {
        let schema = self.schema_name(schema)?;
        if schema.is_empty() || table.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = Statement::new(self)?;
        let rows = stmt.query_catalog(|stmt| routine(stmt, &schema))?;

        rows.map(|row| row.and_then(|row| f(&row))).collect()
    }

    /// The tables of a schema (`None` is the current schema), ordered by
    /// name.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    ///
    /// let tables = conn.tables(None)?;
    /// assert_eq!(tables[0].name, "FOO");
    /// # Ok(())
    /// # }
    /// ```
    pub fn tables(&self, schema: Option<&str>) -> Result<Vec<Table>> {
        self.catalog(
            schema,
            "%",
            |stmt, s| raw::tables(stmt, &pattern(s), "%"),
            |row| {
                Ok(Table {
                    schema: row.get("TABLE_SCHEM")?,
                    name: row.get("TABLE_NAME")?,
                    table_type: TableType::parse(&row.get::<_, String>("TABLE_TYPE")?),
                })
            },
        )
    }

    /// Whether a table exists in a schema (`None` is the current schema). The
    /// name is as it's stored, so a name that wasn't quoted is upper case.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    /// assert!(conn.table_exists(None, "FOO")?);
    /// assert!(!conn.table_exists(None, "foo")?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn table_exists(&self, schema: Option<&str>, table: &str) -> Result<bool> {
        let tables = self.catalog(
            schema,
            table,
            |stmt, s| raw::tables(stmt, &pattern(s), &pattern(table)),
            |_| Ok(()),
        )?;

        Ok(!tables.is_empty())
    }

    /// The columns of a table, in order.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER NOT NULL, y VARCHAR(10) DEFAULT 'a')", ())?;
    ///
    /// let columns = conn.columns(None, "FOO")?;
    /// assert_eq!(columns[1].name, "Y");
    /// assert_eq!(columns[1].ty, vsql::Type::CharacterVarying);
    /// assert_eq!(columns[1].length, Some(10));
    /// assert_eq!(columns[1].default.as_deref(), Some("'a'"));
    /// assert!(!columns[0].nullable);
    /// # Ok(())
    /// # }
    /// ```
    pub fn columns(&self, schema: Option<&str>, table: &str) -> Result<Vec<TableColumn>> {
        // SQLColumns doesn't have the type of some columns (such as a
        // BINARY), so the types are from the columns of a query of the table,
        // and SQLColumns has the defaults (and whether the table exists).
        let schema = self.schema_name(schema)?;
        let defaults = self.catalog(
            Some(&schema),
            table,
            |stmt, s| raw::columns(stmt, &pattern(s), &pattern(table)),
            |row| row.get::<_, Option<String>>("COLUMN_DEF"),
        )?;
        if defaults.is_empty() {
            return Ok(Vec::new());
        }
        let stmt = self.prepare(&format!(
            "SELECT * FROM {}.{}",
            quote(&schema),
            quote(table)
        ))?;

        stmt.describe()?
            .into_iter()
            .zip(defaults)
            .map(|(column, default)| {
                Ok(TableColumn {
                    name: column.name().to_string(),
                    ty: column.ty(),
                    nullable: column.is_nullable(),
                    length: column.length(),
                    precision_scale: column.precision_scale(),
                    default,
                })
            })
            .collect()
    }

    /// The PRIMARY KEY of a table, if it has one.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER, y INTEGER, PRIMARY KEY (y, x))", ())?;
    ///
    /// let pk = conn.primary_key(None, "FOO")?.unwrap();
    /// assert_eq!(pk.columns, ["Y", "X"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn primary_key(&self, schema: Option<&str>, table: &str) -> Result<Option<PrimaryKey>> {
        let mut columns = self.catalog(
            schema,
            table,
            |stmt, s| raw::primary_keys(stmt, s, table),
            |row| {
                Ok((
                    row.get::<_, i64>("ORDINAL_POSITION")?,
                    row.get::<_, String>("COLUMN_NAME")?,
                    row.get::<_, String>("PK_NAME")?,
                ))
            },
        )?;
        columns.sort();
        let name = match columns.first() {
            Some((_, _, name)) => name.clone(),
            None => return Ok(None),
        };

        Ok(Some(PrimaryKey {
            name,
            columns: columns.into_iter().map(|(_, column, _)| column).collect(),
        }))
    }

    /// The foreign keys of a table (that reference other tables), ordered by
    /// name.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (id INTEGER PRIMARY KEY)", ())?;
    /// conn.execute(
    ///     "CREATE TABLE bar (foo_id INTEGER, \
    ///      CONSTRAINT bar_foo FOREIGN KEY (foo_id) REFERENCES foo (id) ON DELETE CASCADE)",
    ///     (),
    /// )?;
    ///
    /// let fks = conn.foreign_keys(None, "BAR")?;
    /// assert_eq!(fks[0].name, "BAR_FOO");
    /// assert_eq!(fks[0].columns, ["FOO_ID"]);
    /// assert_eq!(fks[0].referenced_table, "FOO");
    /// assert_eq!(fks[0].referenced_columns, ["ID"]);
    /// assert_eq!(fks[0].on_delete, vsql::ForeignKeyAction::Cascade);
    /// # Ok(())
    /// # }
    /// ```
    pub fn foreign_keys(&self, schema: Option<&str>, table: &str) -> Result<Vec<ForeignKey>> {
        let mut rows = self.catalog(
            schema,
            table,
            |stmt, s| raw::foreign_keys(stmt, s, table),
            |row| {
                Ok((
                    row.get::<_, String>("FK_NAME")?,
                    row.get::<_, i64>("ORDINAL_POSITION")?,
                    row.get::<_, String>("FK_COLUMN_NAME")?,
                    row.get::<_, String>("UK_COLUMN_NAME")?,
                    ForeignKey {
                        name: row.get("FK_NAME")?,
                        columns: Vec::new(),
                        referenced_schema: row.get("UK_TABLE_SCHEM")?,
                        referenced_table: row.get("UK_TABLE_NAME")?,
                        referenced_columns: Vec::new(),
                        on_update: ForeignKeyAction::from_code(row.get("UPDATE_RULE")?),
                        on_delete: ForeignKeyAction::from_code(row.get("DELETE_RULE")?),
                    },
                ))
            },
        )?;
        rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));

        let mut keys: Vec<ForeignKey> = Vec::new();
        for (name, _, column, referenced_column, key) in rows {
            if keys.last().map(|k| &k.name) != Some(&name) {
                keys.push(key);
            }
            let key = keys.last_mut().unwrap();
            key.columns.push(column);
            key.referenced_columns.push(referenced_column);
        }

        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_types() {
        assert_eq!(TableType::parse("TABLE"), TableType::Table);
        assert_eq!(TableType::parse("VIEW"), TableType::View);
        assert_eq!(TableType::parse("SYSTEM TABLE"), TableType::SystemTable);
    }

    #[test]
    fn foreign_key_actions() {
        assert_eq!(
            ForeignKeyAction::from_code(SQL_CASCADE),
            ForeignKeyAction::Cascade
        );
        assert_eq!(
            ForeignKeyAction::from_code(SQL_RESTRICT),
            ForeignKeyAction::Restrict
        );
        assert_eq!(
            ForeignKeyAction::from_code(SQL_SET_NULL),
            ForeignKeyAction::SetNull
        );
        assert_eq!(
            ForeignKeyAction::from_code(SQL_NO_ACTION),
            ForeignKeyAction::NoAction
        );
        assert_eq!(
            ForeignKeyAction::from_code(SQL_SET_DEFAULT),
            ForeignKeyAction::SetDefault
        );
    }

    #[test]
    fn patterns() {
        assert_eq!(pattern("A_B%C\\D"), "A\\_B\\%C\\\\D");
        assert_eq!(quote("a\"b"), "\"a\"\"b\"");
    }
}

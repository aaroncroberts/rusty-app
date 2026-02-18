use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType,
    ForeignKeyInfo, IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo, TableInfo,
    TableMetadata, ViewInfo,
};
use crate::error::{DataError, Result};
use async_trait::async_trait;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Row, TypeInfo};
use tracing::{debug, info, instrument, warn};

/// SQLite database adapter using sqlx
pub struct SqliteAdapter {
    pool: Option<SqlitePool>,
}

impl SqliteAdapter {
    /// Create a new SQLite adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig) -> String {
        // For SQLite, the database field contains the file path
        // Special case: ":memory:" for in-memory database
        if config.database == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}", config.database)
        }
    }

    /// Convert a SQLite row to QueryValue vector
    fn row_to_values(row: &SqliteRow) -> Result<Vec<QueryValue>> {
        let mut values = Vec::new();

        for (i, column) in row.columns().iter().enumerate() {
            let type_info = column.type_info();
            let type_name = type_info.name();

            // SQLite has a simpler type system: NULL, INTEGER, REAL, TEXT, BLOB
            let value = match type_name {
                "BOOLEAN" | "BOOL" => {
                    let val: Option<bool> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bool value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bool(v),
                        None => QueryValue::Null,
                    }
                }
                "INTEGER" | "INT" | "TINYINT" | "SMALLINT" | "MEDIUMINT" | "BIGINT" => {
                    let val: Option<i64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get int value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Int(v),
                        None => QueryValue::Null,
                    }
                }
                "REAL" | "DOUBLE" | "FLOAT" => {
                    let val: Option<f64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get float value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Float(v),
                        None => QueryValue::Null,
                    }
                }
                "TEXT" | "VARCHAR" | "CHAR" | "CLOB" => {
                    let val: Option<String> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get text value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v),
                        None => QueryValue::Null,
                    }
                }
                "BLOB" => {
                    let val: Option<Vec<u8>> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bytes value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bytes(v),
                        None => QueryValue::Null,
                    }
                }
                "NULL" => QueryValue::Null,
                _ => {
                    // For unknown types, try to get as text
                    let val: Option<String> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!(
                            "Failed to get value for type {}: {}",
                            type_name, e
                        ))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v),
                        None => QueryValue::Null,
                    }
                }
            };

            values.push(value);
        }

        Ok(values)
    }
}

impl Default for SqliteAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    #[instrument(skip(self, _password), fields(db = %config.database))]
    async fn connect(&mut self, config: &ConnectionConfig, _password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::SQLite {
            return Err(DataError::Config(format!(
                "Invalid database type: expected SQLite, got {:?}",
                config.db_type
            )));
        }

        info!("Connecting to SQLite database");
        let connection_string = Self::build_connection_string(config);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to connect to SQLite");
                DataError::Connection(format!("Failed to connect: {}", e))
            })?;

        self.pool = Some(pool);
        info!("Successfully connected to SQLite");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            info!("Disconnecting from SQLite");
            pool.close().await;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.pool.is_some()
    }

    #[instrument(skip(self, query), fields(query_len = query.len()))]
    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        debug!("Executing query");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Query failed: {}", e)))?;

        if rows.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(0),
            });
        }

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        let mut result_rows = Vec::new();
        for row in &rows {
            let values = Self::row_to_values(row)?;
            result_rows.push(values);
        }

        Ok(QueryResult {
            columns,
            rows: result_rows,
            rows_affected: Some(rows.len() as u64),
        })
    }

    #[instrument(skip(self))]
    async fn list_databases(&self) -> Result<Vec<String>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        // SQLite doesn't have multiple databases in the same connection
        // Return attached databases if any
        let rows = sqlx::query("PRAGMA database_list")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list databases: {}", e)))?;

        let databases: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DataError::Query(format!("Failed to parse database names: {}", e)))?;

        Ok(databases)
    }

    #[instrument(skip(self))]
    async fn list_tables(&self, _schema: Option<&str>) -> Result<Vec<String>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        // SQLite uses sqlite_master to list tables
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list tables: {}", e)))?;

        let tables: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DataError::Query(format!("Failed to parse table names: {}", e)))?;

        Ok(tables)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn describe_table(&self, table_name: &str, _schema: Option<&str>) -> Result<TableInfo> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        // Use PRAGMA table_info to get column information
        let query = format!("PRAGMA table_info({})", table_name);

        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to describe table: {}", e)))?;

        let columns: Vec<ColumnInfo> = rows
            .iter()
            .map(|row| {
                Ok(ColumnInfo {
                    name: row.try_get("name").map_err(|e| {
                        DataError::Query(format!("Failed to get column name: {}", e))
                    })?,
                    data_type: row.try_get("type").map_err(|e| {
                        DataError::Query(format!("Failed to get data type: {}", e))
                    })?,
                    nullable: row
                        .try_get::<i64, _>("notnull")
                        .map_err(|e| {
                            DataError::Query(format!("Failed to get notnull flag: {}", e))
                        })?
                        == 0,
                    default_value: row.try_get("dflt_value").ok(),
                    is_primary_key: row
                        .try_get::<i64, _>("pk")
                        .map(|pk| pk > 0)
                        .unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: None, // SQLite doesn't use schemas in the traditional sense
            columns,
        })
    }

    async fn test_connection(&self, config: &ConnectionConfig, _password: Option<&str>) -> Result<bool> {
        let connection_string = Self::build_connection_string(config);

        match SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&connection_string)
            .await
        {
            Ok(pool) => {
                pool.close().await;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    fn database_type(&self) -> DatabaseType {
        DatabaseType::SQLite
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving SQLite server info");

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Get SQLite version
        let version_row = sqlx::query("SELECT sqlite_version() as version")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get SQLite version: {}", e)))?;

        let version: String = version_row
            .try_get("version")
            .map_err(|e| DataError::Query(format!("Failed to parse version: {}", e)))?;

        // Get compile options
        let compile_options = sqlx::query("PRAGMA compile_options")
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!("Failed to get compile options: {}", e))
            })?;

        let mut extra_info = std::collections::HashMap::new();
        extra_info.insert("full_version".to_string(), version.clone());

        for (i, row) in compile_options.iter().enumerate() {
            if let Ok(option) = row.try_get::<String, _>(0) {
                extra_info.insert(format!("compile_option_{}", i), option);
            }
        }

        Ok(ServerInfo {
            version,
            server_type: "SQLite".to_string(),
            extra_info,
        })
    }

    #[instrument(skip(self), fields(database = %database_name))]
    async fn get_database_metadata(&self, database_name: &str) -> Result<DatabaseMetadata> {
        info!("Retrieving metadata for database: {}", database_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // For SQLite, database_name is the file path
        // Get page size and page count to calculate file size
        let page_size_row = sqlx::query("PRAGMA page_size")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get page size: {}", e)))?;

        let page_count_row = sqlx::query("PRAGMA page_count")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get page count: {}", e)))?;

        let page_size: i64 = page_size_row
            .try_get(0)
            .map_err(|e| DataError::Query(format!("Failed to parse page size: {}", e)))?;

        let page_count: i64 = page_count_row
            .try_get(0)
            .map_err(|e| DataError::Query(format!("Failed to parse page count: {}", e)))?;

        let size_bytes = page_size * page_count;

        // Get encoding
        let encoding_row = sqlx::query("PRAGMA encoding")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get encoding: {}", e)))?;

        let encoding: String = encoding_row
            .try_get(0)
            .map_err(|e| DataError::Query(format!("Failed to parse encoding: {}", e)))?;

        let mut extra_info = std::collections::HashMap::new();
        extra_info.insert("page_size".to_string(), page_size.to_string());
        extra_info.insert("page_count".to_string(), page_count.to_string());

        Ok(DatabaseMetadata {
            name: database_name.to_string(),
            size_bytes: Some(size_bytes),
            owner: None, // SQLite is file-based, no owner concept
            encoding: Some(encoding),
            created_at: None,
            extra_info,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_table_metadata(&self, table_name: &str, _schema: Option<&str>) -> Result<TableMetadata> {
        info!("Retrieving metadata for table: {}", table_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Get row count
        let count_query = format!("SELECT COUNT(*) as count FROM {}", table_name);
        let count_row = sqlx::query(&count_query)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!("Failed to get row count for '{}': {}", table_name, e))
            })?;

        let row_count: i64 = count_row
            .try_get("count")
            .map_err(|e| DataError::Query(format!("Failed to parse row count: {}", e)))?;

        // Get table SQL from sqlite_master
        let table_info_query = "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?";
        let table_info_row = sqlx::query(table_info_query)
            .bind(table_name)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get table info for '{}': {}",
                    table_name, e
                ))
            })?;

        let table_sql: Option<String> = table_info_row.try_get("sql").ok();

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: None, // SQLite doesn't have schemas in the same way
            size_bytes: None, // Per-table size not easily available in SQLite
            row_count: Some(row_count),
            created_at: None,
            table_type: table_sql.map(|_| "table".to_string()),
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, _schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        info!("Retrieving indexes for table: {}", table_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Get indexes from sqlite_master
        let query = "
            SELECT name, sql
            FROM sqlite_master
            WHERE type = 'index' AND tbl_name = ?
        ";

        let rows = sqlx::query(query)
            .bind(table_name)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!("Failed to get indexes for '{}': {}", table_name, e))
            })?;

        let mut indexes = Vec::new();

        for row in rows {
            let index_name: String = row
                .try_get("name")
                .map_err(|e| DataError::Query(format!("Failed to get index name: {}", e)))?;

            // Get index details using PRAGMA index_info
            let info_query = format!("PRAGMA index_info({})", index_name);
            let info_rows = sqlx::query(&info_query)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    DataError::Query(format!("Failed to get index info for '{}': {}", index_name, e))
                })?;

            let mut columns = Vec::new();
            for info_row in info_rows {
                if let Ok(col_name) = info_row.try_get::<String, _>("name") {
                    columns.push(col_name);
                }
            }

            // Check if unique using PRAGMA index_list
            let list_query = format!("PRAGMA index_list({})", table_name);
            let list_rows = sqlx::query(&list_query)
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    DataError::Query(format!("Failed to get index list for '{}': {}", table_name, e))
                })?;

            let mut is_unique = false;
            let mut is_primary = false;
            for list_row in list_rows {
                if let Ok(name) = list_row.try_get::<String, _>("name") {
                    if name == index_name {
                        if let Ok(unique) = list_row.try_get::<i64, _>("unique") {
                            is_unique = unique == 1;
                        }
                        if let Ok(origin) = list_row.try_get::<String, _>("origin") {
                            is_primary = origin == "pk";
                        }
                        break;
                    }
                }
            }

            indexes.push(IndexInfo {
                name: index_name.clone(),
                table_name: table_name.to_string(),
                schema: None,
                columns,
                is_unique,
                is_primary,
                index_type: Some("BTREE".to_string()), // SQLite uses B-tree by default
            });
        }

        Ok(indexes)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_foreign_keys(&self, table_name: &str, _schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        info!("Retrieving foreign keys for table: {}", table_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Use PRAGMA foreign_key_list to get FKs
        let query = format!("PRAGMA foreign_key_list({})", table_name);
        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get foreign keys for '{}': {}",
                    table_name, e
                ))
            })?;

        // Group by foreign key ID
        let mut fk_map: std::collections::HashMap<i64, ForeignKeyInfo> = std::collections::HashMap::new();

        for row in rows {
            let id: i64 = row
                .try_get("id")
                .map_err(|e| DataError::Query(format!("Failed to get FK id: {}", e)))?;

            let from_col: String = row
                .try_get("from")
                .map_err(|e| DataError::Query(format!("Failed to get from column: {}", e)))?;

            let to_table: String = row
                .try_get("table")
                .map_err(|e| DataError::Query(format!("Failed to get referenced table: {}", e)))?;

            let to_col: String = row
                .try_get("to")
                .map_err(|e| DataError::Query(format!("Failed to get to column: {}", e)))?;

            let on_update: Option<String> = row.try_get("on_update").ok();
            let on_delete: Option<String> = row.try_get("on_delete").ok();

            if let Some(fk) = fk_map.get_mut(&id) {
                fk.columns.push(from_col);
                fk.referenced_columns.push(to_col);
            } else {
                fk_map.insert(
                    id,
                    ForeignKeyInfo {
                        name: format!("fk_{}_{}", table_name, id),
                        table_name: table_name.to_string(),
                        schema: None,
                        columns: vec![from_col],
                        referenced_table: to_table.clone(),
                        referenced_schema: None,
                        referenced_columns: vec![to_col],
                        on_delete,
                        on_update,
                    },
                );
            }
        }

        Ok(fk_map.into_values().collect())
    }

    #[instrument(skip(self))]
    async fn get_views(&self, _schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        info!("Retrieving views");

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "SELECT name, sql FROM sqlite_master WHERE type = 'view'";

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get views: {}", e)))?;

        let mut views = Vec::new();

        for row in rows {
            let name: String = row
                .try_get("name")
                .map_err(|e| DataError::Query(format!("Failed to get view name: {}", e)))?;

            let definition: Option<String> = row.try_get("sql").ok();

            views.push(ViewInfo {
                name,
                schema: None,
                definition,
            });
        }

        Ok(views)
    }

    #[instrument(skip(self), fields(view = %view_name))]
    async fn get_view_definition(&self, view_name: &str, _schema: Option<&str>) -> Result<Option<String>> {
        info!("Retrieving view definition for: {}", view_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "SELECT sql FROM sqlite_master WHERE type = 'view' AND name = ?";

        let row = sqlx::query(query)
            .bind(view_name)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get view definition for '{}': {}",
                    view_name, e
                ))
            })?;

        match row {
            Some(r) => Ok(r.try_get("sql").ok()),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, _schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        info!("Listing stored procedures");

        // SQLite does not support stored procedures in the traditional sense
        // Return empty list
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-sqlite".to_string(),
            name: "Test SQLite".to_string(),
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            database: ":memory:".to_string(),
            username: None,
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = SqliteAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::SQLite);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = SqliteAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_connection_string_memory() {
        let config = test_config();
        let conn_str = SqliteAdapter::build_connection_string(&config);
        assert_eq!(conn_str, "sqlite::memory:");
    }

    #[test]
    fn test_connection_string_file() {
        let mut config = test_config();
        config.database = "/tmp/test.db".to_string();
        let conn_str = SqliteAdapter::build_connection_string(&config);
        assert_eq!(conn_str, "sqlite:///tmp/test.db");
    }

    #[test]
    fn test_connection_string_relative_path() {
        let mut config = test_config();
        config.database = "data/mydb.sqlite".to_string();
        let conn_str = SqliteAdapter::build_connection_string(&config);
        assert_eq!(conn_str, "sqlite://data/mydb.sqlite");
    }

    #[tokio::test]
    async fn test_connect_and_disconnect() {
        let mut adapter = SqliteAdapter::new();
        let config = test_config();

        assert!(!adapter.is_connected());

        adapter.connect(&config, None).await.unwrap();
        assert!(adapter.is_connected());

        adapter.disconnect().await.unwrap();
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = SqliteAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = SqliteAdapter::new();
        let result = adapter.execute_query("SELECT 1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_databases_when_not_connected() {
        let adapter = SqliteAdapter::new();
        let result = adapter.list_databases().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_tables_when_not_connected() {
        let adapter = SqliteAdapter::new();
        let result = adapter.list_tables(None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_describe_table_when_not_connected() {
        let adapter = SqliteAdapter::new();
        let result = adapter.describe_table("users", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = SqliteAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::Postgres;
        let result = adapter.connect(&config, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_basic_query() {
        let mut adapter = SqliteAdapter::new();
        let config = test_config();

        adapter.connect(&config, None).await.unwrap();

        let result = adapter.execute_query("SELECT 1 as num, 'hello' as text").await.unwrap();
        assert_eq!(result.columns, vec!["num", "text"]);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows_affected, Some(1));
    }

    #[tokio::test]
    async fn test_list_databases() {
        let mut adapter = SqliteAdapter::new();
        let config = test_config();

        adapter.connect(&config, None).await.unwrap();

        let databases = adapter.list_databases().await.unwrap();
        // In-memory database should have at least "main"
        assert!(!databases.is_empty());
        assert!(databases.contains(&"main".to_string()));
    }

    #[tokio::test]
    async fn test_create_and_list_tables() {
        let mut adapter = SqliteAdapter::new();
        let config = test_config();

        adapter.connect(&config, None).await.unwrap();

        // Create a test table
        adapter
            .execute_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let tables = adapter.list_tables(None).await.unwrap();
        assert!(tables.contains(&"users".to_string()));
    }

    #[tokio::test]
    async fn test_describe_table() {
        let mut adapter = SqliteAdapter::new();
        let config = test_config();

        adapter.connect(&config, None).await.unwrap();

        // Create a test table
        adapter
            .execute_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)")
            .await
            .unwrap();

        let table_info = adapter.describe_table("users", None).await.unwrap();
        assert_eq!(table_info.name, "users");
        assert_eq!(table_info.columns.len(), 3);

        let id_col = &table_info.columns[0];
        assert_eq!(id_col.name, "id");
        assert_eq!(id_col.data_type, "INTEGER");
        assert!(id_col.is_primary_key);

        let name_col = &table_info.columns[1];
        assert_eq!(name_col.name, "name");
        assert_eq!(name_col.data_type, "TEXT");
        assert!(!name_col.nullable);
    }

    #[tokio::test]
    async fn test_test_connection() {
        let adapter = SqliteAdapter::new();
        let config = test_config();

        let result = adapter.test_connection(&config, None).await.unwrap();
        assert!(result);
    }
}

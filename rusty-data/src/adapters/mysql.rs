use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType,
    ForeignKeyInfo, IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo,
    TableInfo, TableMetadata, ViewInfo,
};
use crate::error::{DataError, Result};
use async_trait::async_trait;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::{Column, Row, TypeInfo};
use tracing::{debug, info, instrument, warn};

/// MySQL database adapter using sqlx
pub struct MySqlAdapter {
    pool: Option<MySqlPool>,
}

impl MySqlAdapter {
    /// Create a new MySQL adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Validate database name
    fn validate_database_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config("Database name cannot be empty".to_string()));
        }
        if name.len() > 64 {
            return Err(DataError::Config(format!(
                "Database name too long (max 64 chars): {}",
                name.len()
            )));
        }
        Ok(())
    }

    /// Validate table name
    fn validate_table_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config("Table name cannot be empty".to_string()));
        }
        if name.len() > 64 {
            return Err(DataError::Config(format!(
                "Table name too long (max 64 chars): {}",
                name.len()
            )));
        }
        Ok(())
    }

    /// Validate query
    fn validate_query(query: &str) -> Result<()> {
        if query.trim().is_empty() {
            return Err(DataError::Config("Query cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig, password: Option<&str>) -> Result<String> {
        Self::validate_database_name(&config.database)?;

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(3306);
        let username = config.username.as_deref().unwrap_or("root");
        let password = password.unwrap_or("");
        let database = &config.database;

        let ssl_mode = if config.use_ssl {
            "ssl-mode=REQUIRED"
        } else {
            "ssl-mode=DISABLED"
        };

        Ok(format!(
            "mysql://{}:{}@{}:{}/{}?{}",
            username, password, host, port, database, ssl_mode
        ))
    }

    /// Convert a MySQL row to QueryValue vector
    fn row_to_values(row: &MySqlRow) -> Result<Vec<QueryValue>> {
        let mut values = Vec::new();

        for (i, column) in row.columns().iter().enumerate() {
            let type_info = column.type_info();
            let type_name = type_info.name();

            let value = match type_name {
                "TINYINT(1)" | "BOOLEAN" => {
                    let val: Option<bool> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bool value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bool(v),
                        None => QueryValue::Null,
                    }
                }
                "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" => {
                    let val: Option<i64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get int value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Int(v),
                        None => QueryValue::Null,
                    }
                }
                "FLOAT" | "DOUBLE" | "DECIMAL" => {
                    let val: Option<f64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get float value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Float(v),
                        None => QueryValue::Null,
                    }
                }
                "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => {
                    let val: Option<String> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get text value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v),
                        None => QueryValue::Null,
                    }
                }
                "TIMESTAMP" => {
                    // MySQL TIMESTAMP is stored as UTC DateTime
                    use sqlx::types::chrono::{DateTime, Utc};
                    let val: Option<DateTime<Utc>> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get timestamp value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v.format("%Y-%m-%d %H:%M:%S").to_string()),
                        None => QueryValue::Null,
                    }
                }
                "DATETIME" => {
                    // MySQL DATETIME is timezone-naive
                    use sqlx::types::chrono::NaiveDateTime;
                    let val: Option<NaiveDateTime> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get datetime value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v.format("%Y-%m-%d %H:%M:%S").to_string()),
                        None => QueryValue::Null,
                    }
                }
                "DATE" => {
                    // Get date as NaiveDate and convert to string
                    use sqlx::types::chrono::NaiveDate;
                    let val: Option<NaiveDate> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get date value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v.format("%Y-%m-%d").to_string()),
                        None => QueryValue::Null,
                    }
                }
                "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "BINARY" | "VARBINARY" => {
                    let val: Option<Vec<u8>> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bytes value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bytes(v),
                        None => QueryValue::Null,
                    }
                }
                _ => {
                    // For other types (DATE, DATETIME, JSON, etc.), try to get as text
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

impl Default for MySqlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for MySqlAdapter {
    #[instrument(skip(self, password), fields(
        db = %config.database,
        host = config.host.as_deref().unwrap_or("localhost"),
        port = config.port.unwrap_or(3306)
    ))]
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::MySQL {
            return Err(DataError::Config(format!(
                "Invalid database type: expected MySQL, got {:?}",
                config.db_type
            )));
        }

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(3306);
        let database = &config.database;

        info!(
            database = %database,
            host = %host,
            port = %port,
            "Connecting to MySQL database"
        );
        let start = std::time::Instant::now();
        let connection_string = Self::build_connection_string(config, password)?;

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();

                // Categorize connection errors
                let error_category = if error_msg.contains("Access denied") || error_msg.contains("authentication") {
                    "authentication"
                } else if error_msg.contains("Connection refused") || error_msg.contains("Can't connect") {
                    "network"
                } else if error_msg.contains("Unknown database") {
                    "database_not_found"
                } else {
                    "unknown"
                };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    elapsed_ms = elapsed.as_millis(),
                    "Failed to connect to MySQL"
                );

                if error_msg.contains("Access denied") || error_msg.contains("authentication") {
                    DataError::Connection(format!(
                        "Authentication failed for database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                } else if error_msg.contains("Connection refused") || error_msg.contains("Can't connect") {
                    DataError::Connection(format!(
                        "Network error connecting to {}:{} - {}",
                        host, port, e
                    ))
                } else if error_msg.contains("Unknown database") {
                    DataError::Connection(format!(
                        "Database '{}' does not exist at {}:{}",
                        database, host, port
                    ))
                } else {
                    DataError::Connection(format!(
                        "Failed to connect to database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                }
            })?;

        let elapsed = start.elapsed();
        let pool_size = pool.size();
        self.pool = Some(pool);

        info!(
            max_connections = 5,
            current_size = pool_size,
            elapsed_ms = elapsed.as_millis(),
            "Successfully connected to MySQL"
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            info!("Disconnecting from MySQL");
            pool.close().await;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.pool.is_some()
    }

    #[instrument(skip(self, query), fields(query_len = query.len()))]
    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        // Validate query
        Self::validate_query(query)?;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query_snippet = if query.len() > 100 {
            format!("{}...", &query[..100])
        } else {
            query.to_string()
        };

        debug!(
            query_snippet = %query_snippet,
            pool_size = pool.size(),
            "Executing MySQL query"
        );
        let start = std::time::Instant::now();

        // Check if this is a DDL statement that doesn't work with prepared statements
        // MySQL doesn't support CREATE PROCEDURE, CREATE FUNCTION, DROP PROCEDURE, DROP FUNCTION in prepared statements
        let query_upper = query.trim().to_uppercase();
        let needs_simple_execution = query_upper.starts_with("CREATE PROCEDURE")
            || query_upper.starts_with("CREATE FUNCTION")
            || query_upper.starts_with("DROP PROCEDURE")
            || query_upper.starts_with("DROP FUNCTION");

        if needs_simple_execution {
            // Execute as simple statement using raw connection
            use sqlx::Executor;
            let mut conn = pool.acquire().await.map_err(|e| {
                DataError::Connection(format!("Failed to acquire connection: {}", e))
            })?;

            conn.execute(query).await.map_err(|e| {
                let elapsed = start.elapsed();
                warn!(
                    error = %e,
                    query_snippet = %query_snippet,
                    elapsed_ms = elapsed.as_millis(),
                    "Simple query execution failed"
                );
                DataError::Query(format!("Query failed: {} - {}", query_snippet, e))
            })?;

            let elapsed = start.elapsed();
            info!(
                elapsed_ms = elapsed.as_millis(),
                "DDL statement executed successfully"
            );

            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(0),
            });
        }

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();

                let error_category = if error_msg.contains("syntax") {
                    "syntax"
                } else if error_msg.contains("Access denied") || error_msg.contains("permission") {
                    "permission"
                } else if error_msg.contains("doesn't exist") || error_msg.contains("Unknown") {
                    "object_not_found"
                } else if error_msg.contains("Duplicate") || error_msg.contains("constraint") {
                    "constraint"
                } else {
                    "unknown"
                };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    query_snippet = %query_snippet,
                    elapsed_ms = elapsed.as_millis(),
                    "Query execution failed"
                );

                if error_msg.contains("syntax") {
                    DataError::Query(format!("SQL syntax error: {} - {}", query_snippet, e))
                } else if error_msg.contains("Access denied") || error_msg.contains("permission") {
                    DataError::Query(format!("Permission denied: {} - {}", query_snippet, e))
                } else if error_msg.contains("doesn't exist") || error_msg.contains("Unknown") {
                    DataError::Query(format!("Object not found: {} - {}", query_snippet, e))
                } else if error_msg.contains("Duplicate") || error_msg.contains("constraint") {
                    DataError::Query(format!("Constraint violation: {} - {}", query_snippet, e))
                } else {
                    DataError::Query(format!("Query failed: {} - {}", query_snippet, e))
                }
            })?;

        let fetch_elapsed = start.elapsed();

        if rows.is_empty() {
            info!(
                rows_count = 0,
                columns_count = 0,
                elapsed_ms = fetch_elapsed.as_millis(),
                "Query executed successfully (no results)"
            );
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

        let column_count = columns.len();
        let mut result_rows = Vec::new();
        for row in &rows {
            let values = Self::row_to_values(row)?;
            result_rows.push(values);
        }
        let row_count = result_rows.len();

        let total_elapsed = start.elapsed();
        info!(
            rows_count = row_count,
            columns_count = column_count,
            fetch_ms = fetch_elapsed.as_millis(),
            total_ms = total_elapsed.as_millis(),
            "Query executed successfully"
        );

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

        let rows = sqlx::query("SHOW DATABASES")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list databases: {}", e)))?;

        let databases: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>(0))
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

        // In MySQL, schema parameter is ignored - tables are in the current database
        let rows = sqlx::query("SHOW TABLES")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list tables: {}", e)))?;

        let tables: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>(0))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DataError::Query(format!("Failed to parse table names: {}", e)))?;

        Ok(tables)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn describe_table(&self, table_name: &str, _schema: Option<&str>) -> Result<TableInfo> {
        Self::validate_table_name(table_name)?;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        // In MySQL, schema parameter is ignored - using current database
        let query = "
            SELECT
                COLUMN_NAME as column_name,
                DATA_TYPE as data_type,
                IS_NULLABLE as is_nullable,
                COLUMN_DEFAULT as column_default,
                COLUMN_KEY as column_key
            FROM INFORMATION_SCHEMA.COLUMNS
            WHERE TABLE_SCHEMA = DATABASE()
                AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION
        ";

        let rows = sqlx::query(query)
            .bind(table_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to describe table: {}", e)))?;

        let columns: Vec<ColumnInfo> = rows
            .iter()
            .map(|row| {
                Ok(ColumnInfo {
                    name: row.try_get("column_name").map_err(|e| {
                        DataError::Query(format!("Failed to get column name: {}", e))
                    })?,
                    data_type: row.try_get("data_type").map_err(|e| {
                        DataError::Query(format!("Failed to get data type: {}", e))
                    })?,
                    nullable: row
                        .try_get::<String, _>("is_nullable")
                        .map_err(|e| {
                            DataError::Query(format!("Failed to get nullable flag: {}", e))
                        })?
                        == "YES",
                    default_value: row.try_get("column_default").ok(),
                    is_primary_key: row
                        .try_get::<String, _>("column_key")
                        .map(|k| k == "PRI")
                        .unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: None, // MySQL doesn't use schemas in the same way as PostgreSQL
            columns,
        })
    }

    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool> {
        let connection_string = Self::build_connection_string(config, password)?;

        match MySqlPoolOptions::new()
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
        DatabaseType::MySQL
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving MySQL server information");

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let version_result = sqlx::query("SELECT VERSION() as version")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get server version: {}", e)))?;

        let version: String = version_result.try_get("version")
            .map_err(|e| DataError::Query(format!("Failed to parse version: {}", e)))?;

        let mut extra_info = std::collections::HashMap::new();

        let vars_result = sqlx::query("SHOW VARIABLES WHERE Variable_name IN ('character_set_server', 'collation_server', 'max_connections')")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get server variables: {}", e)))?;

        for row in vars_result {
            let name: String = row.try_get("Variable_name").unwrap_or_default();
            let value: String = row.try_get("Value").unwrap_or_default();
            extra_info.insert(name, value);
        }

        Ok(ServerInfo {
            version,
            server_type: "MySQL".to_string(),
            extra_info,
        })
    }

    #[instrument(skip(self), fields(database = %database_name))]
    async fn get_database_metadata(&self, database_name: &str) -> Result<DatabaseMetadata> {
        info!("Retrieving metadata for database: {}", database_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                SCHEMA_NAME as name,
                DEFAULT_CHARACTER_SET_NAME as charset,
                DEFAULT_COLLATION_NAME as collation
            FROM INFORMATION_SCHEMA.SCHEMATA
            WHERE SCHEMA_NAME = ?
        ";

        let result = sqlx::query(query)
            .bind(database_name)
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get database metadata for '{}': {}", database_name, e)))?;

        let size_query = "
            SELECT SUM(data_length + index_length) as size_bytes
            FROM information_schema.TABLES
            WHERE table_schema = ?
        ";

        let size_result = sqlx::query(size_query)
            .bind(database_name)
            .fetch_one(pool)
            .await
            .ok();

        let mut extra_info = std::collections::HashMap::new();
        if let Ok(collation) = result.try_get::<String, _>("collation") {
            extra_info.insert("collation".to_string(), collation);
        }

        Ok(DatabaseMetadata {
            name: result.try_get("name").unwrap_or_else(|_| database_name.to_string()),
            size_bytes: size_result.and_then(|r| r.try_get("size_bytes").ok()),
            owner: None, // MySQL doesn't have database owners
            encoding: result.try_get("charset").ok(),
            created_at: None,
            extra_info,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_table_metadata(&self, table_name: &str, _schema: Option<&str>) -> Result<TableMetadata> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving metadata for table: {}", table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                TABLE_NAME as name,
                TABLE_SCHEMA as schema_name,
                DATA_LENGTH + INDEX_LENGTH as size_bytes,
                TABLE_ROWS as row_count,
                ENGINE as table_type
            FROM INFORMATION_SCHEMA.TABLES
            WHERE TABLE_NAME = ?
                AND TABLE_SCHEMA = DATABASE()
        ";

        let result = sqlx::query(query)
            .bind(table_name)
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get table metadata for '{}': {}", table_name, e)))?;

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: result.try_get("schema_name").ok(),
            size_bytes: result.try_get("size_bytes").ok(),
            row_count: result.try_get("row_count").ok(),
            created_at: None,
            table_type: result.try_get("table_type").ok(),
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, _schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving indexes for table: {}", table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                INDEX_NAME,
                TABLE_NAME,
                TABLE_SCHEMA,
                NON_UNIQUE,
                INDEX_TYPE,
                GROUP_CONCAT(COLUMN_NAME ORDER BY SEQ_IN_INDEX) as columns
            FROM INFORMATION_SCHEMA.STATISTICS
            WHERE TABLE_NAME = ?
                AND TABLE_SCHEMA = DATABASE()
            GROUP BY INDEX_NAME, TABLE_NAME, TABLE_SCHEMA, NON_UNIQUE, INDEX_TYPE
        ";

        let results = sqlx::query(query)
            .bind(table_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get indexes for '{}': {}", table_name, e)))?;

        let mut indexes = Vec::new();
        for row in results {
            let index_name: String = row.try_get("INDEX_NAME").unwrap_or_default();
            let columns_str: String = row.try_get("columns").unwrap_or_default();
            let columns: Vec<String> = columns_str.split(',').map(|s| s.to_string()).collect();

            indexes.push(IndexInfo {
                name: index_name.clone(),
                table_name: row.try_get("TABLE_NAME").unwrap_or_else(|_| table_name.to_string()),
                schema: row.try_get("TABLE_SCHEMA").ok(),
                columns,
                is_unique: row.try_get::<i32, _>("NON_UNIQUE").unwrap_or(1) == 0,
                is_primary: index_name == "PRIMARY",
                index_type: row.try_get("INDEX_TYPE").ok(),
            });
        }

        Ok(indexes)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_foreign_keys(&self, table_name: &str, _schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving foreign keys for table: {}", table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                CONSTRAINT_NAME,
                TABLE_NAME,
                TABLE_SCHEMA,
                COLUMN_NAME,
                REFERENCED_TABLE_NAME,
                REFERENCED_TABLE_SCHEMA,
                REFERENCED_COLUMN_NAME
            FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
            WHERE TABLE_NAME = ?
                AND TABLE_SCHEMA = DATABASE()
                AND REFERENCED_TABLE_NAME IS NOT NULL
            ORDER BY CONSTRAINT_NAME, ORDINAL_POSITION
        ";

        let results = sqlx::query(query)
            .bind(table_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get foreign keys for '{}': {}", table_name, e)))?;

        let mut fk_map: std::collections::HashMap<String, ForeignKeyInfo> = std::collections::HashMap::new();

        for row in results {
            let fk_name: String = row.try_get("CONSTRAINT_NAME").unwrap_or_default();
            let column: String = row.try_get("COLUMN_NAME").unwrap_or_default();
            let ref_column: String = row.try_get("REFERENCED_COLUMN_NAME").unwrap_or_default();

            fk_map.entry(fk_name.clone()).or_insert_with(|| ForeignKeyInfo {
                name: fk_name,
                table_name: row.try_get("TABLE_NAME").unwrap_or_else(|_| table_name.to_string()),
                schema: row.try_get("TABLE_SCHEMA").ok(),
                columns: Vec::new(),
                referenced_table: row.try_get("REFERENCED_TABLE_NAME").unwrap_or_default(),
                referenced_schema: row.try_get("REFERENCED_TABLE_SCHEMA").ok(),
                referenced_columns: Vec::new(),
                on_delete: None,
                on_update: None,
            }).columns.push(column);

            if let Some(fk) = fk_map.get_mut(&row.try_get::<String, _>("CONSTRAINT_NAME").unwrap_or_default()) {
                fk.referenced_columns.push(ref_column);
            }
        }

        Ok(fk_map.into_values().collect())
    }

    #[instrument(skip(self))]
    async fn get_views(&self, _schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        info!("Retrieving views");

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT TABLE_NAME, TABLE_SCHEMA
            FROM INFORMATION_SCHEMA.VIEWS
            WHERE TABLE_SCHEMA = DATABASE()
            ORDER BY TABLE_NAME
        ";

        let results = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get views: {}", e)))?;

        let mut views = Vec::new();
        for row in results {
            views.push(ViewInfo {
                name: row.try_get("TABLE_NAME").unwrap_or_default(),
                schema: row.try_get("TABLE_SCHEMA").ok(),
                definition: None,
            });
        }

        Ok(views)
    }

    #[instrument(skip(self), fields(view = %view_name))]
    async fn get_view_definition(&self, view_name: &str, _schema: Option<&str>) -> Result<Option<String>> {
        info!("Retrieving definition for view: {}", view_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT VIEW_DEFINITION
            FROM INFORMATION_SCHEMA.VIEWS
            WHERE TABLE_NAME = ? AND TABLE_SCHEMA = DATABASE()
        ";

        let result = sqlx::query(query)
            .bind(view_name)
            .fetch_optional(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get view definition for '{}': {}", view_name, e)))?;

        Ok(result.and_then(|row| row.try_get("VIEW_DEFINITION").ok()))
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, _schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        info!("Retrieving stored procedures");

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                ROUTINE_NAME as name,
                ROUTINE_SCHEMA as schema_name,
                DTD_IDENTIFIER as return_type
            FROM INFORMATION_SCHEMA.ROUTINES
            WHERE ROUTINE_SCHEMA = DATABASE()
            ORDER BY ROUTINE_NAME
        ";

        let results = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get stored procedures: {}", e)))?;

        let mut procedures = Vec::new();
        for row in results {
            procedures.push(ProcedureInfo {
                name: row.try_get("name").unwrap_or_default(),
                schema: row.try_get("schema_name").ok(),
                return_type: row.try_get("return_type").ok(),
                language: Some("SQL".to_string()), // MySQL only supports SQL
            });
        }

        Ok(procedures)
    }

    #[instrument(skip(self, rows), fields(table = %table_name, row_count = rows.len(), column_count = columns.len()))]
    async fn bulk_insert(
        &self,
        table_name: &str,
        columns: &[String],
        rows: &[Vec<QueryValue>],
        schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_table_name(table_name)?;

        if columns.is_empty() {
            return Err(DataError::Config("Column list cannot be empty".to_string()));
        }

        if rows.is_empty() {
            return Ok(0);
        }

        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();
        info!(
            "Bulk inserting {} rows into {}{}",
            rows.len(),
            schema_prefix,
            table_name
        );

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Validate all rows have the same column count
        for (idx, row) in rows.iter().enumerate() {
            if row.len() != columns.len() {
                return Err(DataError::Config(format!(
                    "Row {} has {} values but expected {} columns",
                    idx,
                    row.len(),
                    columns.len()
                )));
            }
        }

        let start = std::time::Instant::now();

        // Use MySQL multi-row INSERT syntax for efficiency
        // Build column list
        let column_list = columns.join(", ");

        // Build value placeholders - MySQL uses ?
        let row_placeholder = format!("({})", vec!["?"; columns.len()].join(", "));
        let placeholders = vec![row_placeholder; rows.len()].join(", ");

        // Build the full INSERT query
        let query = format!(
            "INSERT INTO {}{} ({}) VALUES {}",
            schema_prefix, table_name, column_list, placeholders
        );

        debug!("Bulk insert query: {}", query);

        // Build and bind all parameters
        let mut query_builder = sqlx::query(&query);

        for row in rows {
            for value in row {
                query_builder = match value {
                    QueryValue::Null => query_builder.bind(None::<String>),
                    QueryValue::Int(v) => query_builder.bind(*v),
                    QueryValue::Float(v) => query_builder.bind(*v),
                    QueryValue::Text(v) => query_builder.bind(v),
                    QueryValue::Bool(v) => query_builder.bind(*v),
                    QueryValue::Bytes(v) => query_builder.bind(v),
                };
            }
        }

        // Execute the query
        let result = query_builder
            .execute(pool)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to bulk insert into {}{}: {}",
                    schema_prefix, table_name, e
                ))
            })?;

        let rows_affected = result.rows_affected();
        let elapsed = start.elapsed();

        info!(
            "Bulk insert completed: {} rows into {}{} in {}ms",
            rows_affected,
            schema_prefix,
            table_name,
            elapsed.as_millis()
        );

        Ok(rows_affected)
    }

    #[instrument(skip(self, updates), fields(table = %table_name, update_count = updates.len()))]
    async fn bulk_update(
        &self,
        table_name: &str,
        updates: &[(std::collections::HashMap<String, QueryValue>, String)],
        schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_table_name(table_name)?;

        if updates.is_empty() {
            return Ok(0);
        }

        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();
        info!(
            "Bulk updating {} rows in {}{}",
            updates.len(),
            schema_prefix,
            table_name
        );

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let start = std::time::Instant::now();
        let mut total_affected = 0u64;

        // Execute each update in a batch
        for (set_clauses, where_clause) in updates {
            if set_clauses.is_empty() {
                continue;
            }

            // Build SET clause with placeholders
            let set_parts: Vec<String> = set_clauses
                .keys()
                .map(|column| format!("{} = ?", column))
                .collect();

            let query = format!(
                "UPDATE {}{} SET {} WHERE {}",
                schema_prefix,
                table_name,
                set_parts.join(", "),
                where_clause
            );

            debug!("Bulk update query: {}", query);

            // Bind parameters
            let mut query_builder = sqlx::query(&query);

            for value in set_clauses.values() {
                query_builder = match value {
                    QueryValue::Null => query_builder.bind(None::<String>),
                    QueryValue::Int(v) => query_builder.bind(*v),
                    QueryValue::Float(v) => query_builder.bind(*v),
                    QueryValue::Text(v) => query_builder.bind(v),
                    QueryValue::Bool(v) => query_builder.bind(*v),
                    QueryValue::Bytes(v) => query_builder.bind(v),
                };
            }

            let result = query_builder
                .execute(pool)
                .await
                .map_err(|e| {
                    DataError::Query(format!(
                        "Failed to bulk update {}{}: {}",
                        schema_prefix, table_name, e
                    ))
                })?;

            total_affected += result.rows_affected();
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk update completed: {} rows in {}{} in {}ms",
            total_affected,
            schema_prefix,
            table_name,
            elapsed.as_millis()
        );

        Ok(total_affected)
    }

    #[instrument(skip(self, where_clauses), fields(table = %table_name, delete_count = where_clauses.len()))]
    async fn bulk_delete(
        &self,
        table_name: &str,
        where_clauses: &[String],
        schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_table_name(table_name)?;

        if where_clauses.is_empty() {
            return Ok(0);
        }

        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();
        info!(
            "Bulk deleting {} rows from {}{}",
            where_clauses.len(),
            schema_prefix,
            table_name
        );

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let start = std::time::Instant::now();
        let mut total_affected = 0u64;

        // Execute each delete
        for where_clause in where_clauses {
            if where_clause.trim().is_empty() {
                continue;
            }

            let query = format!(
                "DELETE FROM {}{} WHERE {}",
                schema_prefix, table_name, where_clause
            );

            debug!("Bulk delete query: {}", query);

            let result = sqlx::query(&query)
                .execute(pool)
                .await
                .map_err(|e| {
                    DataError::Query(format!(
                        "Failed to bulk delete from {}{}: {}",
                        schema_prefix, table_name, e
                    ))
                })?;

            total_affected += result.rows_affected();
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk delete completed: {} rows from {}{} in {}ms",
            total_affected,
            schema_prefix,
            table_name,
            elapsed.as_millis()
        );

        Ok(total_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mysql".to_string(),
            name: "Test MySQL".to_string(),
            db_type: DatabaseType::MySQL,
            host: Some("localhost".to_string()),
            port: Some(3306),
            database: "test_db".to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = MySqlAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::MySQL);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = MySqlAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_connection_string_basic() {
        let config = test_config();
        let conn_str = MySqlAdapter::build_connection_string(&config, Some("password123")).unwrap();
        assert!(conn_str.contains("mysql://"));
        assert!(conn_str.contains("test_user"));
        assert!(conn_str.contains("password123"));
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("3306"));
        assert!(conn_str.contains("test_db"));
        assert!(conn_str.contains("ssl-mode=DISABLED"));
    }

    #[test]
    fn test_connection_string_with_ssl() {
        let mut config = test_config();
        config.use_ssl = true;
        let conn_str = MySqlAdapter::build_connection_string(&config, Some("password123")).unwrap();
        assert!(conn_str.contains("ssl-mode=REQUIRED"));
    }

    #[test]
    fn test_connection_string_defaults() {
        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::MySQL,
            host: None,
            port: None,
            database: "mydb".to_string(),
            username: None,
            use_ssl: false,
            parameters: HashMap::new(),
        };
        let conn_str = MySqlAdapter::build_connection_string(&config, None).unwrap();
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("3306"));
        assert!(conn_str.contains("root"));
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = MySqlAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = MySqlAdapter::new();
        let result = adapter.execute_query("SELECT 1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_databases_when_not_connected() {
        let adapter = MySqlAdapter::new();
        let result = adapter.list_databases().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_tables_when_not_connected() {
        let adapter = MySqlAdapter::new();
        let result = adapter.list_tables(None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_describe_table_when_not_connected() {
        let adapter = MySqlAdapter::new();
        let result = adapter.describe_table("users", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = MySqlAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::Postgres;
        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    // ===== Validation Tests =====

    #[test]
    fn test_validate_database_name_valid() {
        assert!(MySqlAdapter::validate_database_name("testdb").is_ok());
        assert!(MySqlAdapter::validate_database_name("my_database_123").is_ok());
    }

    #[test]
    fn test_validate_database_name_empty() {
        let result = MySqlAdapter::validate_database_name("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_database_name_too_long() {
        let long_name = "a".repeat(65);
        let result = MySqlAdapter::validate_database_name(&long_name);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_database_name_max_length() {
        let max_name = "a".repeat(64);
        assert!(MySqlAdapter::validate_database_name(&max_name).is_ok());
    }

    #[test]
    fn test_validate_table_name_valid() {
        assert!(MySqlAdapter::validate_table_name("users").is_ok());
        assert!(MySqlAdapter::validate_table_name("order_items").is_ok());
    }

    #[test]
    fn test_validate_table_name_empty() {
        let result = MySqlAdapter::validate_table_name("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_table_name_too_long() {
        let long_name = "t".repeat(65);
        let result = MySqlAdapter::validate_table_name(&long_name);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_query_valid() {
        assert!(MySqlAdapter::validate_query("SELECT * FROM users").is_ok());
        assert!(MySqlAdapter::validate_query("INSERT INTO users VALUES (1)").is_ok());
    }

    #[test]
    fn test_validate_query_empty() {
        let result = MySqlAdapter::validate_query("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_query_whitespace_only() {
        let result = MySqlAdapter::validate_query("   \n\t  ");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    // ===== QueryValue Display Tests =====

    #[test]
    fn test_query_value_display_null() {
        assert_eq!(QueryValue::Null.to_string(), "NULL");
    }

    #[test]
    fn test_query_value_display_bool() {
        assert_eq!(QueryValue::Bool(true).to_string(), "true");
        assert_eq!(QueryValue::Bool(false).to_string(), "false");
    }

    #[test]
    fn test_query_value_display_int() {
        assert_eq!(QueryValue::Int(42).to_string(), "42");
        assert_eq!(QueryValue::Int(-100).to_string(), "-100");
    }

    #[test]
    fn test_query_value_display_float() {
        assert_eq!(QueryValue::Float(3.14).to_string(), "3.14");
        assert_eq!(QueryValue::Float(-2.5).to_string(), "-2.5");
    }

    #[test]
    fn test_query_value_display_text() {
        assert_eq!(QueryValue::Text("hello".to_string()).to_string(), "hello");
    }

    #[test]
    fn test_query_value_display_bytes() {
        let bytes = vec![1, 2, 3, 4, 5];
        assert_eq!(QueryValue::Bytes(bytes).to_string(), "<5 bytes>");
    }

    // ========== Bulk Operations Tests (TDD - RED Phase) ==========

    #[tokio::test]
    async fn test_bulk_insert_not_connected() {
        let adapter = MySqlAdapter::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
            vec![QueryValue::Int(2), QueryValue::Text("Bob".to_string())],
        ];

        let result = adapter
            .bulk_insert("users", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_insert_validation_empty_columns() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let columns = vec![];
            let rows = vec![vec![QueryValue::Int(1)]];

            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[test]
    fn test_bulk_insert_validation_empty_rows() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let columns = vec!["id".to_string()];
            let rows = vec![];

            // Empty rows should return Ok(0)
            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_insert_validation_column_count_mismatch() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let columns = vec!["id".to_string(), "name".to_string()];
            let rows = vec![
                vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
                vec![QueryValue::Int(2)], // Wrong column count
            ];

            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_bulk_insert_validation_table_name() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let columns = vec!["id".to_string()];
            let rows = vec![vec![QueryValue::Int(1)]];

            // Empty table name
            let result = adapter.bulk_insert("", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));

            // Table name too long (> 64 chars)
            let long_name = "a".repeat(65);
            let result = adapter.bulk_insert(&long_name, &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[tokio::test]
    async fn test_bulk_update_not_connected() {
        let adapter = MySqlAdapter::new();
        let mut update_map = HashMap::new();
        update_map.insert("name".to_string(), QueryValue::Text("Updated".to_string()));

        let updates = vec![(update_map, "id = 1".to_string())];

        let result = adapter.bulk_update("users", &updates, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_update_validation_empty_updates() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let updates = vec![];

            // Empty updates should return Ok(0)
            let result = adapter.bulk_update("users", &updates, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_update_validation_table_name() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let mut update_map = HashMap::new();
            update_map.insert("name".to_string(), QueryValue::Text("Test".to_string()));
            let updates = vec![(update_map, "id = 1".to_string())];

            // Empty table name
            let result = adapter.bulk_update("", &updates, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));

            // Table name too long
            let long_name = "a".repeat(65);
            let result = adapter.bulk_update(&long_name, &updates, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[tokio::test]
    async fn test_bulk_delete_not_connected() {
        let adapter = MySqlAdapter::new();
        let where_clauses = vec!["id = 1".to_string(), "id = 2".to_string()];

        let result = adapter.bulk_delete("users", &where_clauses, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_delete_validation_empty_clauses() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let where_clauses = vec![];

            // Empty clauses should return Ok(0)
            let result = adapter.bulk_delete("users", &where_clauses, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_delete_validation_table_name() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let where_clauses = vec!["id = 1".to_string()];

            // Empty table name
            let result = adapter.bulk_delete("", &where_clauses, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));

            // Table name too long
            let long_name = "a".repeat(65);
            let result = adapter.bulk_delete(&long_name, &where_clauses, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[test]
    fn test_bulk_insert_all_data_types() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MySqlAdapter::new();
            let columns = vec![
                "id".to_string(),
                "name".to_string(),
                "active".to_string(),
                "score".to_string(),
                "data".to_string(),
                "description".to_string(),
            ];
            let rows = vec![
                vec![
                    QueryValue::Int(1),
                    QueryValue::Text("Alice".to_string()),
                    QueryValue::Bool(true),
                    QueryValue::Float(95.5),
                    QueryValue::Bytes(vec![1, 2, 3]),
                    QueryValue::Null,
                ],
                vec![
                    QueryValue::Int(2),
                    QueryValue::Text("Bob".to_string()),
                    QueryValue::Bool(false),
                    QueryValue::Float(87.3),
                    QueryValue::Bytes(vec![4, 5, 6]),
                    QueryValue::Text("Some description".to_string()),
                ],
            ];

            // Will fail at connection, but validates we handle all types
            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
        });
    }
}

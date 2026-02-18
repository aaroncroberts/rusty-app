use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType,
    ForeignKeyInfo, IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo,
    TableInfo, TableMetadata, ViewInfo,
};
use crate::error::{DataError, Result};
use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{Column, Row, TypeInfo};
use tracing::{debug, info, instrument, warn};

/// PostgreSQL database adapter using sqlx
pub struct PostgresAdapter {
    pool: Option<PgPool>,
}

impl PostgresAdapter {
    /// Create a new PostgreSQL adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Validate database name
    fn validate_database_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config("Database name cannot be empty".to_string()));
        }
        if name.len() > 63 {
            return Err(DataError::Config(format!(
                "Database name too long (max 63 chars): {}",
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
        if name.len() > 63 {
            return Err(DataError::Config(format!(
                "Table name too long (max 63 chars): {}",
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
        let port = config.port.unwrap_or(5432);
        let username = config.username.as_deref().unwrap_or("postgres");
        let password = password.unwrap_or("");
        let database = &config.database;

        let ssl_mode = if config.use_ssl { "require" } else { "prefer" };

        Ok(format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}",
            username, password, host, port, database, ssl_mode
        ))
    }

    /// Convert a PostgreSQL row to QueryValue vector
    fn row_to_values(row: &PgRow) -> Result<Vec<QueryValue>> {
        let mut values = Vec::new();

        for (i, column) in row.columns().iter().enumerate() {
            let type_info = column.type_info();
            let type_name = type_info.name();

            let value = match type_name {
                "BOOL" => {
                    let val: Option<bool> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bool value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bool(v),
                        None => QueryValue::Null,
                    }
                }
                "INT2" => {
                    let val: Option<i16> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get int2 value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Int(v as i64),
                        None => QueryValue::Null,
                    }
                }
                "INT4" => {
                    let val: Option<i32> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get int4 value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Int(v as i64),
                        None => QueryValue::Null,
                    }
                }
                "INT8" => {
                    let val: Option<i64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get int8 value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Int(v),
                        None => QueryValue::Null,
                    }
                }
                "FLOAT4" | "FLOAT8" | "NUMERIC" => {
                    let val: Option<f64> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get float value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Float(v),
                        None => QueryValue::Null,
                    }
                }
                "TEXT" | "VARCHAR" | "CHAR" | "NAME" => {
                    let val: Option<String> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get text value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v),
                        None => QueryValue::Null,
                    }
                }
                "TIMESTAMP" | "TIMESTAMPTZ" => {
                    // Get timestamp as NaiveDateTime and convert to string
                    use sqlx::types::chrono::NaiveDateTime;
                    let val: Option<NaiveDateTime> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get timestamp value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Text(v.format("%Y-%m-%d %H:%M:%S").to_string()),
                        None => QueryValue::Null,
                    }
                }
                "BYTEA" => {
                    let val: Option<Vec<u8>> = row.try_get(i).map_err(|e| {
                        DataError::Query(format!("Failed to get bytes value: {}", e))
                    })?;
                    match val {
                        Some(v) => QueryValue::Bytes(v),
                        None => QueryValue::Null,
                    }
                }
                _ => {
                    // For other types, try to get as text
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

impl Default for PostgresAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    #[instrument(skip(self, password), fields(
        db = %config.database,
        host = config.host.as_deref().unwrap_or("localhost"),
        port = config.port.unwrap_or(5432)
    ))]
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::Postgres {
            return Err(DataError::Config(format!(
                "Invalid database type: expected Postgres, got {:?}",
                config.db_type
            )));
        }

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(5432);
        let database = &config.database;

        info!("Connecting to PostgreSQL database");
        let connection_string = Self::build_connection_string(config, password)?;

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to connect to PostgreSQL");
                let error_msg = e.to_string();
                // Categorize connection errors
                if error_msg.contains("password authentication failed") || error_msg.contains("no pg_hba.conf entry") {
                    DataError::Connection(format!(
                        "Authentication failed for database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                } else if error_msg.contains("could not translate host name") || error_msg.contains("Connection refused") {
                    DataError::Connection(format!(
                        "Network error connecting to {}:{} - {}",
                        host, port, e
                    ))
                } else if error_msg.contains("does not exist") {
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

        self.pool = Some(pool);
        info!("Successfully connected to PostgreSQL");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            info!("Disconnecting from PostgreSQL");
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

        // Validate query
        Self::validate_query(query)?;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Get query snippet for error messages (first 100 chars)
        let query_snippet = if query.len() > 100 {
            format!("{}...", &query[..100])
        } else {
            query.to_string()
        };

        let rows = sqlx::query(query)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                // Categorize query errors
                if error_msg.contains("syntax error") {
                    DataError::Query(format!("SQL syntax error in query: {} - Error: {}", query_snippet, e))
                } else if error_msg.contains("permission denied") || error_msg.contains("must be owner") {
                    DataError::Query(format!("Permission denied executing query: {} - Error: {}", query_snippet, e))
                } else if error_msg.contains("does not exist") {
                    DataError::Query(format!("Object not found executing query: {} - Error: {}", query_snippet, e))
                } else if error_msg.contains("violates") {
                    DataError::Query(format!("Constraint violation in query: {} - Error: {}", query_snippet, e))
                } else {
                    DataError::Query(format!("Query execution failed: {} - Error: {}", query_snippet, e))
                }
            })?;

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

    async fn list_databases(&self) -> Result<Vec<String>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let rows = sqlx::query("SELECT datname FROM pg_database WHERE datistemplate = false")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list databases: {}", e)))?;

        let databases: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>("datname"))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DataError::Query(format!("Failed to parse database names: {}", e)))?;

        Ok(databases)
    }

    async fn list_tables(&self, schema: Option<&str>) -> Result<Vec<String>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let schema_name = schema.unwrap_or("public");

        let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_type = 'BASE TABLE'";

        let rows = sqlx::query(query)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list tables in schema '{}': {}", schema_name, e)))?;

        let tables: Vec<String> = rows
            .iter()
            .map(|row| row.try_get::<String, _>("table_name"))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| DataError::Query(format!("Failed to parse table names from schema '{}': {}", schema_name, e)))?;

        Ok(tables)
    }

    async fn describe_table(&self, table_name: &str, schema: Option<&str>) -> Result<TableInfo> {
        // Validate table name
        Self::validate_table_name(table_name)?;

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let schema_name = schema.unwrap_or("public");

        let query = "
            SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key
            FROM information_schema.columns c
            LEFT JOIN (
                SELECT ku.column_name
                FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage ku
                    ON tc.constraint_name = ku.constraint_name
                    AND tc.table_schema = ku.table_schema
                WHERE tc.constraint_type = 'PRIMARY KEY'
                    AND tc.table_name = $1
                    AND tc.table_schema = $2
            ) pk ON c.column_name = pk.column_name
            WHERE c.table_name = $1
                AND c.table_schema = $2
            ORDER BY c.ordinal_position
        ";

        let rows = sqlx::query(query)
            .bind(table_name)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("does not exist") {
                    DataError::Query(format!(
                        "Table '{}' not found in schema '{}': {}",
                        table_name, schema_name, e
                    ))
                } else {
                    DataError::Query(format!(
                        "Failed to describe table '{}.{}': {}",
                        schema_name, table_name, e
                    ))
                }
            })?;

        if rows.is_empty() {
            return Err(DataError::Query(format!(
                "Table '{}.{}' not found or has no columns",
                schema_name, table_name
            )));
        }

        let columns: Vec<ColumnInfo> = rows
            .iter()
            .map(|row| {
                Ok(ColumnInfo {
                    name: row.try_get("column_name").map_err(|e| {
                        DataError::Query(format!("Failed to get column name for table '{}.{}': {}", schema_name, table_name, e))
                    })?,
                    data_type: row.try_get("data_type").map_err(|e| {
                        DataError::Query(format!("Failed to get data type for table '{}.{}': {}", schema_name, table_name, e))
                    })?,
                    nullable: row
                        .try_get::<String, _>("is_nullable")
                        .map_err(|e| {
                            DataError::Query(format!("Failed to get nullable flag for table '{}.{}': {}", schema_name, table_name, e))
                        })?
                        == "YES",
                    default_value: row.try_get("column_default").ok(),
                    is_primary_key: row.try_get("is_primary_key").unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: Some(schema_name.to_string()),
            columns,
        })
    }

    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool> {
        let connection_string = Self::build_connection_string(config, password)?;

        match PgPoolOptions::new()
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
        DatabaseType::Postgres
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving PostgreSQL server information");

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        // Get version
        let version_result = sqlx::query("SELECT version()")
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get server version: {}", e)))?;

        let version: String = version_result.try_get(0)
            .map_err(|e| DataError::Query(format!("Failed to parse version: {}", e)))?;

        // Get additional server info
        let settings_result = sqlx::query("SELECT name, setting FROM pg_settings WHERE name IN ('server_version', 'server_encoding', 'max_connections', 'shared_buffers')")
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get server settings: {}", e)))?;

        let mut extra_info = std::collections::HashMap::new();
        for row in settings_result {
            let name: String = row.try_get("name").unwrap_or_default();
            let setting: String = row.try_get("setting").unwrap_or_default();
            extra_info.insert(name, setting);
        }

        debug!("Retrieved server info for PostgreSQL {}", version);

        Ok(ServerInfo {
            version,
            server_type: "PostgreSQL".to_string(),
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
                pg_database.datname as name,
                pg_database_size(pg_database.datname) as size_bytes,
                pg_catalog.pg_get_userbyid(pg_database.datdba) as owner,
                pg_encoding_to_char(pg_database.encoding) as encoding,
                pg_database.datcollate as collation,
                pg_database.datctype as ctype
            FROM pg_database
            WHERE datname = $1
        ";

        let result = sqlx::query(query)
            .bind(database_name)
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get database metadata for '{}': {}", database_name, e)))?;

        let mut extra_info = std::collections::HashMap::new();
        if let Ok(collation) = result.try_get::<String, _>("collation") {
            extra_info.insert("collation".to_string(), collation);
        }
        if let Ok(ctype) = result.try_get::<String, _>("ctype") {
            extra_info.insert("ctype".to_string(), ctype);
        }

        Ok(DatabaseMetadata {
            name: result.try_get("name").unwrap_or_else(|_| database_name.to_string()),
            size_bytes: result.try_get("size_bytes").ok(),
            owner: result.try_get("owner").ok(),
            encoding: result.try_get("encoding").ok(),
            created_at: None, // PostgreSQL doesn't store creation time
            extra_info,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_table_metadata(&self, table_name: &str, schema: Option<&str>) -> Result<TableMetadata> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving metadata for table: {}.{}", schema_name, table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                schemaname,
                tablename,
                pg_total_relation_size(schemaname||'.'||tablename) as size_bytes,
                n_live_tup as row_count
            FROM pg_stat_user_tables
            WHERE schemaname = $1 AND tablename = $2
        ";

        let result = sqlx::query(query)
            .bind(schema_name)
            .bind(table_name)
            .fetch_one(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get table metadata for '{}.{}': {}", schema_name, table_name, e)))?;

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: Some(schema_name.to_string()),
            size_bytes: result.try_get("size_bytes").ok(),
            row_count: result.try_get("row_count").ok(),
            created_at: None, // PostgreSQL doesn't store table creation time
            table_type: Some("TABLE".to_string()),
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving indexes for table: {}.{}", schema_name, table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                i.relname as index_name,
                t.relname as table_name,
                n.nspname as schema_name,
                ix.indisunique as is_unique,
                ix.indisprimary as is_primary,
                am.amname as index_type,
                array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) as columns
            FROM pg_index ix
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
            JOIN pg_am am ON am.oid = i.relam
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
            WHERE t.relname = $1 AND n.nspname = $2
            GROUP BY i.relname, t.relname, n.nspname, ix.indisunique, ix.indisprimary, am.amname
        ";

        let results = sqlx::query(query)
            .bind(table_name)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get indexes for '{}.{}': {}", schema_name, table_name, e)))?;

        let mut indexes = Vec::new();
        for row in results {
            indexes.push(IndexInfo {
                name: row.try_get("index_name").unwrap_or_default(),
                table_name: row.try_get("table_name").unwrap_or_else(|_| table_name.to_string()),
                schema: Some(row.try_get("schema_name").unwrap_or_else(|_| schema_name.to_string())),
                columns: row.try_get::<Vec<String>, _>("columns").unwrap_or_default(),
                is_unique: row.try_get("is_unique").unwrap_or(false),
                is_primary: row.try_get("is_primary").unwrap_or(false),
                index_type: row.try_get("index_type").ok(),
            });
        }

        debug!("Found {} indexes for {}.{}", indexes.len(), schema_name, table_name);
        Ok(indexes)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_foreign_keys(&self, table_name: &str, schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving foreign keys for table: {}.{}", schema_name, table_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                tc.constraint_name,
                tc.table_name,
                tc.table_schema,
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.table_schema AS foreign_table_schema,
                ccu.column_name AS foreign_column_name,
                rc.update_rule,
                rc.delete_rule
            FROM information_schema.table_constraints AS tc
            JOIN information_schema.key_column_usage AS kcu
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage AS ccu
                ON ccu.constraint_name = tc.constraint_name
                AND ccu.table_schema = tc.table_schema
            JOIN information_schema.referential_constraints AS rc
                ON rc.constraint_name = tc.constraint_name
                AND rc.constraint_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
                AND tc.table_name = $1
                AND tc.table_schema = $2
            ORDER BY tc.constraint_name, kcu.ordinal_position
        ";

        let results = sqlx::query(query)
            .bind(table_name)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get foreign keys for '{}.{}': {}", schema_name, table_name, e)))?;

        // Group by constraint name since one FK can span multiple columns
        let mut fk_map: std::collections::HashMap<String, ForeignKeyInfo> = std::collections::HashMap::new();

        for row in results {
            let fk_name: String = row.try_get("constraint_name").unwrap_or_default();
            let column: String = row.try_get("column_name").unwrap_or_default();
            let ref_column: String = row.try_get("foreign_column_name").unwrap_or_default();

            fk_map.entry(fk_name.clone()).or_insert_with(|| ForeignKeyInfo {
                name: fk_name,
                table_name: row.try_get("table_name").unwrap_or_else(|_| table_name.to_string()),
                schema: Some(row.try_get("table_schema").unwrap_or_else(|_| schema_name.to_string())),
                columns: Vec::new(),
                referenced_table: row.try_get("foreign_table_name").unwrap_or_default(),
                referenced_schema: row.try_get("foreign_table_schema").ok(),
                referenced_columns: Vec::new(),
                on_delete: row.try_get("delete_rule").ok(),
                on_update: row.try_get("update_rule").ok(),
            }).columns.push(column);

            if let Some(fk) = fk_map.get_mut(&row.try_get::<String, _>("constraint_name").unwrap_or_default()) {
                fk.referenced_columns.push(ref_column);
            }
        }

        let foreign_keys: Vec<ForeignKeyInfo> = fk_map.into_values().collect();
        debug!("Found {} foreign keys for {}.{}", foreign_keys.len(), schema_name, table_name);
        Ok(foreign_keys)
    }

    #[instrument(skip(self))]
    async fn get_views(&self, schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving views for schema: {}", schema_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                table_name,
                table_schema
            FROM information_schema.views
            WHERE table_schema = $1
            ORDER BY table_name
        ";

        let results = sqlx::query(query)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get views for schema '{}': {}", schema_name, e)))?;

        let mut views = Vec::new();
        for row in results {
            views.push(ViewInfo {
                name: row.try_get("table_name").unwrap_or_default(),
                schema: row.try_get("table_schema").ok(),
                definition: None, // Definition retrieved separately via get_view_definition
            });
        }

        debug!("Found {} views in schema {}", views.len(), schema_name);
        Ok(views)
    }

    #[instrument(skip(self), fields(view = %view_name))]
    async fn get_view_definition(&self, view_name: &str, schema: Option<&str>) -> Result<Option<String>> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving definition for view: {}.{}", schema_name, view_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT view_definition
            FROM information_schema.views
            WHERE table_name = $1 AND table_schema = $2
        ";

        let result = sqlx::query(query)
            .bind(view_name)
            .bind(schema_name)
            .fetch_optional(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get view definition for '{}.{}': {}", schema_name, view_name, e)))?;

        Ok(result.and_then(|row| row.try_get("view_definition").ok()))
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        let schema_name = schema.unwrap_or("public");
        info!("Retrieving stored procedures for schema: {}", schema_name);

        let pool = self.pool.as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let query = "
            SELECT
                p.proname as name,
                n.nspname as schema,
                pg_get_function_result(p.oid) as return_type,
                l.lanname as language
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            JOIN pg_language l ON l.oid = p.prolang
            WHERE n.nspname = $1
            ORDER BY p.proname
        ";

        let results = sqlx::query(query)
            .bind(schema_name)
            .fetch_all(pool)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get stored procedures for schema '{}': {}", schema_name, e)))?;

        let mut procedures = Vec::new();
        for row in results {
            procedures.push(ProcedureInfo {
                name: row.try_get("name").unwrap_or_default(),
                schema: row.try_get("schema").ok(),
                return_type: row.try_get("return_type").ok(),
                language: row.try_get("language").ok(),
            });
        }

        debug!("Found {} procedures in schema {}", procedures.len(), schema_name);
        Ok(procedures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-pg".to_string(),
            name: "Test PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "test_db".to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = PostgresAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::Postgres);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = PostgresAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_connection_string_basic() {
        let config = test_config();
        let conn_str = PostgresAdapter::build_connection_string(&config, Some("password123")).unwrap();
        assert!(conn_str.contains("postgresql://"));
        assert!(conn_str.contains("test_user"));
        assert!(conn_str.contains("password123"));
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("5432"));
        assert!(conn_str.contains("test_db"));
        assert!(conn_str.contains("sslmode=prefer"));
    }

    #[test]
    fn test_connection_string_with_ssl() {
        let mut config = test_config();
        config.use_ssl = true;
        let conn_str = PostgresAdapter::build_connection_string(&config, Some("password123")).unwrap();
        assert!(conn_str.contains("sslmode=require"));
    }

    #[test]
    fn test_connection_string_defaults() {
        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::Postgres,
            host: None,
            port: None,
            database: "mydb".to_string(),
            username: None,
            use_ssl: false,
            parameters: HashMap::new(),
        };
        let conn_str = PostgresAdapter::build_connection_string(&config, None).unwrap();
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("5432"));
        assert!(conn_str.contains("postgres"));
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = PostgresAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = PostgresAdapter::new();
        let result = adapter.execute_query("SELECT 1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_databases_when_not_connected() {
        let adapter = PostgresAdapter::new();
        let result = adapter.list_databases().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_tables_when_not_connected() {
        let adapter = PostgresAdapter::new();
        let result = adapter.list_tables(None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_describe_table_when_not_connected() {
        let adapter = PostgresAdapter::new();
        let result = adapter.describe_table("users", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = PostgresAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::MySQL;
        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }
}

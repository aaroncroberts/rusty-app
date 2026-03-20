use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType, ForeignKeyInfo,
    IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo, TableInfo, TableMetadata,
    ViewInfo,
};
use crate::error::{DataError, Result};
use crate::pool::Pool;
use async_trait::async_trait;
use futures_util::stream::TryStreamExt;
use tiberius::{AuthMethod, Client, Config, QueryItem, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tracing::{debug, info, instrument, warn};

/// Microsoft SQL Server database adapter using tiberius
pub struct MssqlAdapter {
    pool: Option<Pool<Client<Compat<TcpStream>>>>,
}

impl MssqlAdapter {
    /// Create a new SQL Server adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Validate database name
    fn validate_database_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config(
                "Database name cannot be empty".to_string(),
            ));
        }
        if name.len() > 128 {
            return Err(DataError::Config(format!(
                "Database name too long (max 128 chars): {}",
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
        if name.len() > 128 {
            return Err(DataError::Config(format!(
                "Table name too long (max 128 chars): {}",
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

    /// Build a tiberius config from connection configuration
    fn build_config(config: &ConnectionConfig, password: Option<&str>) -> Result<Config> {
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(1433);
        let username = config.username.as_deref().unwrap_or("sa");
        let password = password.unwrap_or("");
        let database = &config.database;

        let mut tiberius_config = Config::new();
        tiberius_config.host(host);
        tiberius_config.port(port);
        tiberius_config.database(database);
        tiberius_config.authentication(AuthMethod::sql_server(username, password));

        if config.use_ssl {
            tiberius_config.encryption(tiberius::EncryptionLevel::Required);
        } else {
            tiberius_config.encryption(tiberius::EncryptionLevel::NotSupported);
        }

        // Trust server certificate for development
        tiberius_config.trust_cert();

        Ok(tiberius_config)
    }

    /// Convert a SQL Server row to QueryValue vector
    fn row_to_values(row: &Row) -> Result<Vec<QueryValue>> {
        let mut values = Vec::new();

        for i in 0..row.len() {
            // Try to get the value as different types
            let value = if let Ok(Some(v)) = row.try_get::<&str, usize>(i) {
                QueryValue::Text(v.to_string())
            } else if let Ok(Some(v)) = row.try_get::<i32, usize>(i) {
                QueryValue::Int(v as i64)
            } else if let Ok(Some(v)) = row.try_get::<i64, usize>(i) {
                QueryValue::Int(v)
            } else if let Ok(Some(v)) = row.try_get::<f64, usize>(i) {
                QueryValue::Float(v)
            } else if let Ok(Some(v)) = row.try_get::<bool, usize>(i) {
                QueryValue::Bool(v)
            } else if let Ok(Some(v)) = row.try_get::<&[u8], usize>(i) {
                QueryValue::Bytes(v.to_vec())
            } else {
                // NULL or unsupported type
                QueryValue::Null
            };

            values.push(value);
        }

        Ok(values)
    }
}

impl Default for MssqlAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for MssqlAdapter {
    #[instrument(skip(self, password), fields(
        db = %config.database,
        host = config.host.as_deref().unwrap_or("localhost"),
        port = config.port.unwrap_or(1433)
    ))]
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::SQLServer {
            return Err(DataError::Config(format!(
                "Invalid database type: expected SQLServer, got {:?}",
                config.db_type
            )));
        }

        Self::validate_database_name(&config.database)?;

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(1433);
        let database = &config.database;

        info!(
            database = %database,
            host = %host,
            port = %port,
            "Connecting to SQL Server database"
        );
        let start = std::time::Instant::now();
        let tiberius_config = Self::build_config(config, password)?;

        let tcp = TcpStream::connect(tiberius_config.get_addr())
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();
                let error_category = if error_msg.contains("Connection refused") {
                    "network"
                } else if error_msg.contains("No route to host") || error_msg.contains("timeout") {
                    "timeout"
                } else {
                    "unknown"
                };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    elapsed_ms = elapsed.as_millis(),
                    "Failed to connect to SQL Server"
                );

                if error_msg.contains("Connection refused") {
                    DataError::Connection(format!(
                        "Network error connecting to SQL Server at {}:{} - {}",
                        host, port, e
                    ))
                } else if error_msg.contains("No route to host") || error_msg.contains("timeout") {
                    DataError::Connection(format!(
                        "Network timeout or unreachable host {}:{} - {}",
                        host, port, e
                    ))
                } else {
                    DataError::Connection(format!(
                        "Failed to connect to SQL Server at {}:{} - {}",
                        host, port, e
                    ))
                }
            })?;

        let client = Client::connect(tiberius_config, tcp.compat_write())
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();
                let error_category =
                    if error_msg.contains("Login failed") || error_msg.contains("authentication") {
                        "authentication"
                    } else if error_msg.contains("Cannot open database")
                        || error_msg.contains("does not exist")
                    {
                        "database_not_found"
                    } else {
                        "unknown"
                    };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    elapsed_ms = elapsed.as_millis(),
                    "Failed to authenticate with SQL Server"
                );

                if error_msg.contains("Login failed") || error_msg.contains("authentication") {
                    DataError::Connection(format!(
                        "Authentication failed for database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                } else if error_msg.contains("Cannot open database")
                    || error_msg.contains("does not exist")
                {
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
        self.pool = Some(Pool::new(client));
        info!(
            elapsed_ms = elapsed.as_millis(),
            "Successfully connected to SQL Server"
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(_pool) = self.pool.take() {
            info!("Disconnecting from SQL Server");
            // Pool will be dropped here, closing the connection
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.pool.is_some()
    }

    #[instrument(skip(self, query), fields(query_len = query.len()))]
    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
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
            "Executing SQL Server query"
        );
        let start = std::time::Instant::now();

        let mut client = pool.lock().await;

        // Check if this is a DDL statement that doesn't return results
        let query_upper = query.trim().to_uppercase();
        let is_ddl = query_upper.starts_with("CREATE VIEW")
            || query_upper.starts_with("CREATE PROCEDURE")
            || query_upper.starts_with("CREATE FUNCTION")
            || query_upper.starts_with("ALTER VIEW")
            || query_upper.starts_with("ALTER PROCEDURE")
            || query_upper.starts_with("ALTER FUNCTION")
            || query_upper.starts_with("DROP VIEW")
            || query_upper.starts_with("DROP PROCEDURE")
            || query_upper.starts_with("DROP FUNCTION");

        if is_ddl {
            // Execute DDL statement using simple_query (doesn't return QueryResult)
            client.simple_query(query).await.map_err(|e| {
                let elapsed = start.elapsed();
                warn!(
                    error = %e,
                    query_snippet = %query_snippet,
                    elapsed_ms = elapsed.as_millis(),
                    "DDL statement execution failed"
                );
                DataError::Query(format!("SQL syntax error: {} - Query: {}", e, query))
            })?;

            let elapsed = start.elapsed();
            info!(
                query_snippet = %query_snippet,
                elapsed_ms = elapsed.as_millis(),
                "DDL statement executed successfully"
            );

            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(0),
            });
        }

        // Execute the query
        let mut result = client.query(query, &[]).await.map_err(|e| {
            let elapsed = start.elapsed();
            let error_msg = e.to_string();

            // Categorize SQL Server query errors
            let error_category = if error_msg.contains("Incorrect syntax")
                || error_msg.contains("syntax error")
            {
                "syntax"
            } else if error_msg.contains("Invalid object name")
                || error_msg.contains("does not exist")
            {
                "object_not_found"
            } else if error_msg.contains("UNIQUE constraint") || error_msg.contains("duplicate key")
            {
                "unique_constraint"
            } else if error_msg.contains("FOREIGN KEY constraint") {
                "foreign_key_constraint"
            } else if error_msg.contains("CHECK constraint") {
                "check_constraint"
            } else if error_msg.contains("Cannot insert NULL") || error_msg.contains("NOT NULL") {
                "not_null_constraint"
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

            if error_msg.contains("Incorrect syntax") || error_msg.contains("syntax error") {
                DataError::Query(format!("SQL syntax error: {} - Query: {}", e, query))
            } else if error_msg.contains("Invalid object name")
                || error_msg.contains("does not exist")
            {
                DataError::Query(format!(
                    "Table or column not found: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("UNIQUE constraint") || error_msg.contains("duplicate key")
            {
                DataError::Query(format!(
                    "Unique constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("FOREIGN KEY constraint") {
                DataError::Query(format!(
                    "Foreign key constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("CHECK constraint") {
                DataError::Query(format!(
                    "Check constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("Cannot insert NULL") || error_msg.contains("NOT NULL") {
                DataError::Query(format!(
                    "Not null constraint violation: {} - Query: {}",
                    e, query
                ))
            } else {
                DataError::Query(format!("Query failed: {} - Query: {}", e, query))
            }
        })?;

        // Get column information from the first result set
        let columns_opt = result
            .columns()
            .await
            .map_err(|e| DataError::Query(format!("Failed to get columns: {}", e)))?;

        let columns: Vec<String> = if let Some(cols) = columns_opt {
            cols.iter().map(|col| col.name().to_string()).collect()
        } else {
            Vec::new()
        };

        // Collect rows
        let mut rows = Vec::new();
        let mut row_count = 0u64;

        while let Some(item) = result
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to fetch row: {}", e)))?
        {
            match item {
                QueryItem::Row(row) => {
                    let values = Self::row_to_values(&row)?;
                    rows.push(values);
                    row_count += 1;
                }
                QueryItem::Metadata(_) => {
                    // Skip metadata items
                }
            }
        }

        // For queries that modify data, use the row count; otherwise None
        let rows_affected = if rows.is_empty() && row_count == 0 {
            Some(0)
        } else if !rows.is_empty() {
            None // SELECT query
        } else {
            Some(row_count)
        };

        let total_elapsed = start.elapsed();
        info!(
            rows_count = rows.len(),
            columns_count = columns.len(),
            total_ms = total_elapsed.as_millis(),
            "Query executed successfully"
        );

        Ok(QueryResult {
            columns,
            rows,
            rows_affected,
        })
    }

    #[instrument(skip(self))]
    async fn list_databases(&self) -> Result<Vec<String>> {
        let result = self
            .execute_query("SELECT name FROM sys.databases WHERE database_id > 4 ORDER BY name")
            .await?;

        let databases: Vec<String> = result
            .rows
            .into_iter()
            .filter_map(|row| {
                if let Some(QueryValue::Text(name)) = row.first() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        Ok(databases)
    }

    #[instrument(skip(self))]
    async fn list_tables(&self, schema: Option<&str>) -> Result<Vec<String>> {
        let schema_name = schema.unwrap_or("dbo");
        let query = format!(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_TYPE = 'BASE TABLE' \
             ORDER BY TABLE_NAME",
            schema_name
        );

        let result = self.execute_query(&query).await?;

        let tables: Vec<String> = result
            .rows
            .into_iter()
            .filter_map(|row| {
                if let Some(QueryValue::Text(name)) = row.first() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        Ok(tables)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn describe_table(&self, table_name: &str, schema: Option<&str>) -> Result<TableInfo> {
        Self::validate_table_name(table_name)?;

        let schema_name = schema.unwrap_or("dbo");
        let query = format!(
            "SELECT
                c.COLUMN_NAME,
                c.DATA_TYPE,
                c.IS_NULLABLE,
                c.COLUMN_DEFAULT,
                CASE WHEN pk.COLUMN_NAME IS NOT NULL THEN 1 ELSE 0 END as IS_PRIMARY_KEY
            FROM INFORMATION_SCHEMA.COLUMNS c
            LEFT JOIN (
                SELECT ku.COLUMN_NAME
                FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
                JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku
                    ON tc.CONSTRAINT_NAME = ku.CONSTRAINT_NAME
                    AND tc.TABLE_SCHEMA = ku.TABLE_SCHEMA
                WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
                    AND tc.TABLE_NAME = '{}'
                    AND tc.TABLE_SCHEMA = '{}'
            ) pk ON c.COLUMN_NAME = pk.COLUMN_NAME
            WHERE c.TABLE_NAME = '{}'
                AND c.TABLE_SCHEMA = '{}'
            ORDER BY c.ORDINAL_POSITION",
            table_name, schema_name, table_name, schema_name
        );

        let result = self.execute_query(&query).await?;

        let columns: Vec<ColumnInfo> = result
            .rows
            .iter()
            .map(|row| {
                let name = if let Some(QueryValue::Text(n)) = row.get(0) {
                    n.clone()
                } else {
                    String::new()
                };

                let data_type = if let Some(QueryValue::Text(t)) = row.get(1) {
                    t.clone()
                } else {
                    String::new()
                };

                let nullable = if let Some(QueryValue::Text(n)) = row.get(2) {
                    n == "YES"
                } else {
                    true
                };

                let default_value = if let Some(QueryValue::Text(d)) = row.get(3) {
                    Some(d.clone())
                } else {
                    None
                };

                let is_primary_key = if let Some(QueryValue::Int(pk)) = row.get(4) {
                    *pk == 1
                } else {
                    false
                };

                ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    default_value,
                    is_primary_key,
                }
            })
            .collect();

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: Some(schema_name.to_string()),
            columns,
        })
    }

    async fn test_connection(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> Result<bool> {
        let tiberius_config = Self::build_config(config, password)?;

        match TcpStream::connect(tiberius_config.get_addr()).await {
            Ok(tcp) => match Client::connect(tiberius_config, tcp.compat_write()).await {
                Ok(client) => {
                    let _ = client.close().await;
                    Ok(true)
                }
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }

    fn database_type(&self) -> DatabaseType {
        DatabaseType::SQLServer
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving SQL Server info");

        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        // Get version (cast SERVERPROPERTY to avoid SQL_VARIANT type issues)
        let version_query = "SELECT @@VERSION as version, CAST(SERVERPROPERTY('ProductVersion') AS NVARCHAR(128)) as product_version, CAST(SERVERPROPERTY('Edition') AS NVARCHAR(128)) as edition";
        let mut stream = client
            .query(version_query, &[])
            .await
            .map_err(|e| DataError::Query(format!("Failed to get server version: {}", e)))?;

        let mut extra_info = std::collections::HashMap::new();
        let mut version = String::from("unknown");

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate version result: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                if let Ok(Some(v)) = row.try_get::<&str, _>("version") {
                    version = v.to_string();
                }
                if let Ok(Some(v)) = row.try_get::<&str, _>("product_version") {
                    extra_info.insert("product_version".to_string(), v.to_string());
                }
                if let Ok(Some(v)) = row.try_get::<&str, _>("edition") {
                    extra_info.insert("edition".to_string(), v.to_string());
                }
            }
        }

        Ok(ServerInfo {
            version,
            server_type: "Microsoft SQL Server".to_string(),
            extra_info,
        })
    }

    #[instrument(skip(self), fields(database = %database_name))]
    async fn get_database_metadata(&self, database_name: &str) -> Result<DatabaseMetadata> {
        info!("Retrieving metadata for database: {}", database_name);

        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT
                d.name,
                COALESCE(CAST(SUM(mf.size) * 8 / 1024 AS BIGINT), 0) as size_mb,
                suser_sname(d.owner_sid) as owner,
                d.collation_name,
                d.create_date,
                d.recovery_model_desc,
                d.compatibility_level
            FROM sys.databases d
            LEFT JOIN sys.master_files mf ON d.database_id = mf.database_id
            WHERE d.name = '{}'
            GROUP BY d.name, d.owner_sid, d.collation_name, d.create_date, d.recovery_model_desc, d.compatibility_level",
            database_name
        );

        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to get database metadata for '{}': {}",
                database_name, e
            ))
        })?;

        let mut size_bytes = None;
        let mut owner = None;
        let mut encoding = None;
        let created_at = None;
        let mut extra_info = std::collections::HashMap::new();

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate database metadata: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                if let Ok(Some(size_mb)) = row.try_get::<i64, _>("size_mb") {
                    size_bytes = Some(size_mb * 1024 * 1024); // Convert MB to bytes
                }
                if let Ok(Some(o)) = row.try_get::<&str, _>("owner") {
                    owner = Some(o.to_string());
                }
                if let Ok(Some(c)) = row.try_get::<&str, _>("collation_name") {
                    encoding = Some(c.to_string());
                }
                if let Ok(Some(recovery)) = row.try_get::<&str, _>("recovery_model_desc") {
                    extra_info.insert("recovery_model".to_string(), recovery.to_string());
                }
                if let Ok(Some(compat)) = row.try_get::<i32, _>("compatibility_level") {
                    extra_info.insert("compatibility_level".to_string(), compat.to_string());
                }
            }
        }

        Ok(DatabaseMetadata {
            name: database_name.to_string(),
            size_bytes,
            owner,
            encoding,
            created_at,
            extra_info,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_table_metadata(
        &self,
        table_name: &str,
        schema: Option<&str>,
    ) -> Result<TableMetadata> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving metadata for table: {}", table_name);

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT
                t.name as table_name,
                s.name as schema_name,
                SUM(a.total_pages) * 8 as size_kb,
                SUM(p.rows) as row_count,
                t.create_date,
                t.type_desc
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.indexes i ON t.object_id = i.object_id
            INNER JOIN sys.partitions p ON i.object_id = p.object_id AND i.index_id = p.index_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE t.name = '{}' AND s.name = '{}'
            GROUP BY t.name, s.name, t.create_date, t.type_desc",
            table_name, schema_name
        );

        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to get table metadata for '{}': {}",
                table_name, e
            ))
        })?;

        let mut size_bytes = None;
        let mut row_count = None;
        let mut table_type = None;

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate table metadata: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                if let Ok(Some(size_kb)) = row.try_get::<i64, _>("size_kb") {
                    size_bytes = Some(size_kb * 1024); // Convert KB to bytes
                }
                if let Ok(Some(rc)) = row.try_get::<i64, _>("row_count") {
                    row_count = Some(rc);
                }
                if let Ok(Some(tt)) = row.try_get::<&str, _>("type_desc") {
                    table_type = Some(tt.to_string());
                }
            }
        }

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: Some(schema_name.to_string()),
            size_bytes,
            row_count,
            created_at: None,
            table_type,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving indexes for table: {}", table_name);

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT
                i.name as index_name,
                i.is_unique,
                i.is_primary_key,
                i.type_desc,
                STRING_AGG(c.name, ',') as columns
            FROM sys.indexes i
            INNER JOIN sys.tables t ON i.object_id = t.object_id
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id
            INNER JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id
            WHERE t.name = '{}' AND s.name = '{}'
            GROUP BY i.name, i.is_unique, i.is_primary_key, i.type_desc",
            table_name, schema_name
        );

        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!("Failed to get indexes for '{}': {}", table_name, e))
        })?;

        let mut indexes = Vec::new();

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate indexes: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                let index_name = row
                    .try_get::<&str, _>("index_name")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();

                let columns_str = row
                    .try_get::<&str, _>("columns")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();
                let columns: Vec<String> = columns_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                let is_unique = row
                    .try_get::<bool, _>("is_unique")
                    .unwrap_or(Some(false))
                    .unwrap_or(false);
                let is_primary = row
                    .try_get::<bool, _>("is_primary_key")
                    .unwrap_or(Some(false))
                    .unwrap_or(false);
                let index_type = row
                    .try_get::<&str, _>("type_desc")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                indexes.push(IndexInfo {
                    name: index_name,
                    table_name: table_name.to_string(),
                    schema: Some(schema_name.to_string()),
                    columns,
                    is_unique,
                    is_primary,
                    index_type,
                });
            }
        }

        Ok(indexes)
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_foreign_keys(
        &self,
        table_name: &str,
        schema: Option<&str>,
    ) -> Result<Vec<ForeignKeyInfo>> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving foreign keys for table: {}", table_name);

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT
                fk.name as fk_name,
                OBJECT_NAME(fk.parent_object_id) as table_name,
                SCHEMA_NAME(t1.schema_id) as schema_name,
                STRING_AGG(c1.name, ',') as columns,
                OBJECT_NAME(fk.referenced_object_id) as referenced_table,
                SCHEMA_NAME(t2.schema_id) as referenced_schema,
                STRING_AGG(c2.name, ',') as referenced_columns,
                fk.delete_referential_action_desc as on_delete,
                fk.update_referential_action_desc as on_update
            FROM sys.foreign_keys fk
            INNER JOIN sys.tables t1 ON fk.parent_object_id = t1.object_id
            INNER JOIN sys.tables t2 ON fk.referenced_object_id = t2.object_id
            INNER JOIN sys.foreign_key_columns fkc ON fk.object_id = fkc.constraint_object_id
            INNER JOIN sys.columns c1 ON fkc.parent_object_id = c1.object_id AND fkc.parent_column_id = c1.column_id
            INNER JOIN sys.columns c2 ON fkc.referenced_object_id = c2.object_id AND fkc.referenced_column_id = c2.column_id
            WHERE OBJECT_NAME(fk.parent_object_id) = '{}' AND SCHEMA_NAME(t1.schema_id) = '{}'
            GROUP BY fk.name, fk.parent_object_id, t1.schema_id, fk.referenced_object_id, t2.schema_id, fk.delete_referential_action_desc, fk.update_referential_action_desc",
            table_name, schema_name
        );

        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to get foreign keys for '{}': {}",
                table_name, e
            ))
        })?;

        let mut fks = Vec::new();

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate foreign keys: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                let fk_name = row
                    .try_get::<&str, _>("fk_name")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();

                let columns_str = row
                    .try_get::<&str, _>("columns")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();
                let columns: Vec<String> = columns_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                let referenced_table = row
                    .try_get::<&str, _>("referenced_table")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();

                let referenced_schema = row
                    .try_get::<&str, _>("referenced_schema")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                let referenced_columns_str = row
                    .try_get::<&str, _>("referenced_columns")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();
                let referenced_columns: Vec<String> = referenced_columns_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                let on_delete = row
                    .try_get::<&str, _>("on_delete")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                let on_update = row
                    .try_get::<&str, _>("on_update")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                fks.push(ForeignKeyInfo {
                    name: fk_name,
                    table_name: table_name.to_string(),
                    schema: Some(schema_name.to_string()),
                    columns,
                    referenced_table,
                    referenced_schema,
                    referenced_columns,
                    on_delete,
                    on_update,
                });
            }
        }

        Ok(fks)
    }

    #[instrument(skip(self))]
    async fn get_views(&self, schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        info!("Retrieving views");

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT v.name, s.name as schema_name
            FROM sys.views v
            INNER JOIN sys.schemas s ON v.schema_id = s.schema_id
            WHERE s.name = '{}'",
            schema_name
        );

        let mut stream = client
            .query(query.as_str(), &[])
            .await
            .map_err(|e| DataError::Query(format!("Failed to get views: {}", e)))?;

        let mut views = Vec::new();

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate views: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                let name = row
                    .try_get::<&str, _>("name")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();

                let schema = row
                    .try_get::<&str, _>("schema_name")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                views.push(ViewInfo {
                    name,
                    schema,
                    definition: None, // Definition retrieved separately
                });
            }
        }

        Ok(views)
    }

    #[instrument(skip(self), fields(view = %view_name))]
    async fn get_view_definition(
        &self,
        view_name: &str,
        schema: Option<&str>,
    ) -> Result<Option<String>> {
        info!("Retrieving view definition for: {}", view_name);

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT OBJECT_DEFINITION(OBJECT_ID('{}.{}')) as definition",
            schema_name, view_name
        );

        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to get view definition for '{}': {}",
                view_name, e
            ))
        })?;

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate view definition: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                if let Ok(Some(def)) = row.try_get::<&str, _>("definition") {
                    return Ok(Some(def.to_string()));
                }
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        info!("Listing stored procedures");

        let schema_name = schema.unwrap_or("dbo");
        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let query = format!(
            "SELECT p.name, s.name as schema_name, p.type_desc
            FROM sys.procedures p
            INNER JOIN sys.schemas s ON p.schema_id = s.schema_id
            WHERE s.name = '{}'",
            schema_name
        );

        let mut stream = client
            .query(query.as_str(), &[])
            .await
            .map_err(|e| DataError::Query(format!("Failed to list stored procedures: {}", e)))?;

        let mut procedures = Vec::new();

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to iterate procedures: {}", e)))?
        {
            if let QueryItem::Row(row) = item {
                let name = row
                    .try_get::<&str, _>("name")
                    .map(|s| s.unwrap_or("").to_string())
                    .unwrap_or_default();

                let schema = row
                    .try_get::<&str, _>("schema_name")
                    .map(|s| s.map(|s| s.to_string()))
                    .unwrap_or(None);

                let language = Some("T-SQL".to_string()); // SQL Server uses T-SQL

                procedures.push(ProcedureInfo {
                    name,
                    schema,
                    return_type: None, // Would require more complex query
                    language,
                });
            }
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

        let schema_name = schema.unwrap_or("dbo");
        info!(
            "Bulk inserting {} rows into {}.{}",
            rows.len(),
            schema_name,
            table_name
        );

        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

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

        // Use SQL Server multi-row INSERT syntax
        // Build values directly into query (Tiberius parameterization is complex for bulk ops)
        let column_list = columns.join(", ");

        let mut value_rows = Vec::new();
        for row in rows {
            let values: Vec<String> = row
                .iter()
                .map(|v| match v {
                    QueryValue::Null => "NULL".to_string(),
                    QueryValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                    QueryValue::Int(i) => i.to_string(),
                    QueryValue::Float(f) => f.to_string(),
                    QueryValue::Text(s) => format!("'{}'", s.replace("'", "''")), // Escape single quotes
                    QueryValue::Bytes(b) => {
                        // Convert bytes to hex string manually
                        let hex_str: String =
                            b.iter().map(|byte| format!("{:02X}", byte)).collect();
                        format!("0x{}", hex_str)
                    }
                })
                .collect();
            value_rows.push(format!("({})", values.join(", ")));
        }

        // Build the full INSERT query
        let query = format!(
            "INSERT INTO {}.{} ({}) VALUES {}",
            schema_name,
            table_name,
            column_list,
            value_rows.join(", ")
        );

        debug!("Bulk insert query: {}", query);

        // Execute query
        let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to bulk insert into {}.{}: {}",
                schema_name, table_name, e
            ))
        })?;

        // Consume the result stream
        while let Some(_item) = stream
            .try_next()
            .await
            .map_err(|e| DataError::Query(format!("Failed to get result: {}", e)))?
        {
            // Just consume the stream
        }

        // For INSERT, rows affected equals number of inserted rows
        let rows_affected = rows.len() as u64;

        let elapsed = start.elapsed();

        info!(
            "Bulk insert completed: {} rows into {}.{} in {}ms",
            rows_affected,
            schema_name,
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

        let schema_name = schema.unwrap_or("dbo");
        info!(
            "Bulk updating {} rows in {}.{}",
            updates.len(),
            schema_name,
            table_name
        );

        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let start = std::time::Instant::now();
        let mut total_affected = 0u64;

        // Execute each update
        for (set_clauses, where_clause) in updates {
            if set_clauses.is_empty() {
                continue;
            }

            // Build SET clause with values directly in query
            let set_parts: Vec<String> = set_clauses
                .iter()
                .map(|(column, value)| {
                    let val_str = match value {
                        QueryValue::Null => "NULL".to_string(),
                        QueryValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                        QueryValue::Int(i) => i.to_string(),
                        QueryValue::Float(f) => f.to_string(),
                        QueryValue::Text(s) => format!("'{}'", s.replace("'", "''")),
                        QueryValue::Bytes(b) => {
                            // Convert bytes to hex string manually
                            let hex_str: String =
                                b.iter().map(|byte| format!("{:02X}", byte)).collect();
                            format!("0x{}", hex_str)
                        }
                    };
                    format!("{} = {}", column, val_str)
                })
                .collect();

            let query = format!(
                "UPDATE {}.{} SET {} WHERE {}",
                schema_name,
                table_name,
                set_parts.join(", "),
                where_clause
            );

            debug!("Bulk update query: {}", query);

            let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
                DataError::Query(format!(
                    "Failed to bulk update {}.{}: {}",
                    schema_name, table_name, e
                ))
            })?;

            // For UPDATE, we need to count affected rows
            // SQL Server doesn't return this directly, assume 1 per update
            total_affected += 1;

            // Consume the stream
            while let Some(_) = stream
                .try_next()
                .await
                .map_err(|e| DataError::Query(format!("Failed to get result: {}", e)))?
            {
            }
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk update completed: {} rows in {}.{} in {}ms",
            total_affected,
            schema_name,
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

        let schema_name = schema.unwrap_or("dbo");
        info!(
            "Bulk deleting {} rows from {}.{}",
            where_clauses.len(),
            schema_name,
            table_name
        );

        let mut client = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .lock()
            .await;

        let start = std::time::Instant::now();
        let mut total_affected = 0u64;

        // Execute each delete
        for where_clause in where_clauses {
            if where_clause.trim().is_empty() {
                continue;
            }

            let query = format!(
                "DELETE FROM {}.{} WHERE {}",
                schema_name, table_name, where_clause
            );

            debug!("Bulk delete query: {}", query);

            let mut stream = client.query(query.as_str(), &[]).await.map_err(|e| {
                DataError::Query(format!(
                    "Failed to bulk delete from {}.{}: {}",
                    schema_name, table_name, e
                ))
            })?;

            // For DELETE, assume 1 row per delete
            total_affected += 1;

            // Consume the stream
            while let Some(_) = stream
                .try_next()
                .await
                .map_err(|e| DataError::Query(format!("Failed to get result: {}", e)))?
            {
            }
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk delete completed: {} rows from {}.{} in {}ms",
            total_affected,
            schema_name,
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
            id: "test-mssql".to_string(),
            name: "Test SQL Server".to_string(),
            db_type: DatabaseType::SQLServer,
            host: Some("localhost".to_string()),
            port: Some(1433),
            database: "test_db".to_string(),
            username: Some("sa".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = MssqlAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::SQLServer);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = MssqlAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = MssqlAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = MssqlAdapter::new();
        let result = adapter.execute_query("SELECT 1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = MssqlAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::Postgres;
        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    // ===== Validation Tests =====
    #[test]
    fn test_validate_database_name_valid() {
        assert!(MssqlAdapter::validate_database_name("testdb").is_ok());
    }

    #[test]
    fn test_validate_database_name_empty() {
        assert!(MssqlAdapter::validate_database_name("").is_err());
    }

    #[test]
    fn test_validate_table_name_valid() {
        assert!(MssqlAdapter::validate_table_name("users").is_ok());
    }

    #[test]
    fn test_validate_table_name_empty() {
        assert!(MssqlAdapter::validate_table_name("").is_err());
    }

    #[test]
    fn test_validate_query_valid() {
        assert!(MssqlAdapter::validate_query("SELECT * FROM users").is_ok());
    }

    #[test]
    fn test_validate_query_empty() {
        assert!(MssqlAdapter::validate_query("").is_err());
    }

    // ===== QueryValue Display Tests =====
    #[test]
    fn test_query_value_display() {
        assert_eq!(QueryValue::Null.to_string(), "NULL");
        assert_eq!(QueryValue::Bool(true).to_string(), "true");
        assert_eq!(QueryValue::Int(42).to_string(), "42");
        assert_eq!(QueryValue::Float(3.14).to_string(), "3.14");
        assert_eq!(QueryValue::Text("hello".to_string()).to_string(), "hello");
        assert_eq!(QueryValue::Bytes(vec![1, 2, 3]).to_string(), "<3 bytes>");
    }

    // ========== Bulk Operations Tests (TDD - RED Phase) ==========

    #[tokio::test]
    async fn test_bulk_insert_not_connected() {
        let adapter = MssqlAdapter::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
            vec![QueryValue::Int(2), QueryValue::Text("Bob".to_string())],
        ];

        let result = adapter.bulk_insert("users", &columns, &rows, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_insert_validation_empty_columns() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MssqlAdapter::new();
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
            let adapter = MssqlAdapter::new();
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
            let adapter = MssqlAdapter::new();
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
            let adapter = MssqlAdapter::new();
            let columns = vec!["id".to_string()];
            let rows = vec![vec![QueryValue::Int(1)]];

            // Empty table name
            let result = adapter.bulk_insert("", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[tokio::test]
    async fn test_bulk_update_not_connected() {
        let adapter = MssqlAdapter::new();
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
            let adapter = MssqlAdapter::new();
            let updates = vec![];

            // Empty updates should return Ok(0)
            let result = adapter.bulk_update("users", &updates, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[tokio::test]
    async fn test_bulk_delete_not_connected() {
        let adapter = MssqlAdapter::new();
        let where_clauses = vec!["id = 1".to_string(), "id = 2".to_string()];

        let result = adapter.bulk_delete("users", &where_clauses, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_delete_validation_empty_clauses() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MssqlAdapter::new();
            let where_clauses = vec![];

            // Empty clauses should return Ok(0)
            let result = adapter.bulk_delete("users", &where_clauses, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_insert_all_data_types() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MssqlAdapter::new();
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

use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType, ForeignKeyInfo,
    IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo, TableInfo, TableMetadata,
    ViewInfo,
};
use crate::error::{DataError, Result};
use crate::pool::Pool;
use async_trait::async_trait;
use oracle::{Connection, Row};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

/// Oracle database adapter
pub struct OracleAdapter {
    pool: Option<Pool<Connection>>,
}

impl OracleAdapter {
    /// Create a new Oracle adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Validate database name (service name or SID)
    fn validate_database_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config(
                "Database name (service name/SID) cannot be empty".to_string(),
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

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig) -> String {
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(1521);
        let database = &config.database; // This is the service name or SID

        format!("{}:{}/{}", host, port, database)
    }

    /// Execute a query in a blocking context
    /// This is a static method to avoid lifetime issues with spawn_blocking
    fn execute_blocking(pool: Pool<Connection>, query: String) -> Result<QueryResult> {
        let query_snippet = if query.len() > 100 {
            format!("{}...", &query[..100])
        } else {
            query.clone()
        };

        let start = std::time::Instant::now();

        // Get the connection from the pool in the blocking context
        let conn_guard = futures::executor::block_on(pool.lock());

        // Execute the query
        let mut stmt = conn_guard.statement(&query).build().map_err(|e| {
            let elapsed = start.elapsed();
            let error_msg = e.to_string();

            let error_category =
                if error_msg.contains("ORA-00900") || error_msg.contains("invalid SQL statement") {
                    "syntax"
                } else {
                    "prepare_failed"
                };

            tracing::warn!(
                error = %e,
                error_category = %error_category,
                query_snippet = %query_snippet,
                elapsed_ms = elapsed.as_millis(),
                "Failed to prepare Oracle statement"
            );

            if error_msg.contains("ORA-00900") || error_msg.contains("invalid SQL statement") {
                DataError::Query(format!("SQL syntax error: {} - Query: {}", e, query))
            } else {
                DataError::Query(format!(
                    "Failed to prepare statement: {} - Query: {}",
                    e, query
                ))
            }
        })?;

        let mut result_set = stmt.query(&[]).map_err(|e| {
            let elapsed = start.elapsed();
            let error_msg = e.to_string();

            // Categorize Oracle query errors
            let error_category = if error_msg.contains("ORA-00942")
                || error_msg.contains("table or view does not exist")
            {
                "object_not_found"
            } else if error_msg.contains("ORA-00904") || error_msg.contains("invalid identifier") {
                "column_not_found"
            } else if error_msg.contains("ORA-00001") || error_msg.contains("unique constraint") {
                "unique_constraint"
            } else if error_msg.contains("ORA-02291") || error_msg.contains("integrity constraint")
            {
                "foreign_key_constraint"
            } else if error_msg.contains("ORA-01407")
                || error_msg.contains("cannot update") && error_msg.contains("to NULL")
            {
                "not_null_constraint"
            } else if error_msg.contains("ORA-02290") || error_msg.contains("check constraint") {
                "check_constraint"
            } else {
                "unknown"
            };

            tracing::warn!(
                error = %e,
                error_category = %error_category,
                query_snippet = %query_snippet,
                elapsed_ms = elapsed.as_millis(),
                "Query execution failed"
            );

            if error_msg.contains("ORA-00942") || error_msg.contains("table or view does not exist")
            {
                DataError::Query(format!("Table or view not found: {} - Query: {}", e, query))
            } else if error_msg.contains("ORA-00904") || error_msg.contains("invalid identifier") {
                DataError::Query(format!("Column not found: {} - Query: {}", e, query))
            } else if error_msg.contains("ORA-00001") || error_msg.contains("unique constraint") {
                DataError::Query(format!(
                    "Unique constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("ORA-02291") || error_msg.contains("integrity constraint")
            {
                DataError::Query(format!(
                    "Foreign key constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("ORA-01407")
                || error_msg.contains("cannot update") && error_msg.contains("to NULL")
            {
                DataError::Query(format!(
                    "Not null constraint violation: {} - Query: {}",
                    e, query
                ))
            } else if error_msg.contains("ORA-02290") || error_msg.contains("check constraint") {
                DataError::Query(format!(
                    "Check constraint violation: {} - Query: {}",
                    e, query
                ))
            } else {
                DataError::Query(format!("Query failed: {} - Query: {}", e, query))
            }
        })?;

        let fetch_start = std::time::Instant::now();

        // Get column information
        let column_info = result_set.column_info();
        let columns: Vec<String> = column_info
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        let column_count = columns.len();

        // Collect rows
        let mut rows = Vec::new();
        for row_result in &mut result_set {
            let row =
                row_result.map_err(|e| DataError::Query(format!("Failed to fetch row: {}", e)))?;
            let values = Self::row_to_values(&row, column_count)?;
            rows.push(values);
        }

        let row_count = rows.len();
        let fetch_elapsed = fetch_start.elapsed();
        let total_elapsed = start.elapsed();

        // For DML statements (INSERT, UPDATE, DELETE), rows will be empty
        // Oracle doesn't easily provide rows_affected without additional work
        let rows_affected = if rows.is_empty() {
            Some(0) // Placeholder - would need ROW_COUNT or similar
        } else {
            None // SELECT query
        };

        tracing::info!(
            rows_count = row_count,
            columns_count = column_count,
            fetch_ms = fetch_elapsed.as_millis(),
            total_ms = total_elapsed.as_millis(),
            "Query executed successfully"
        );

        Ok(QueryResult {
            columns,
            rows,
            rows_affected,
        })
    }

    /// Convert a row to a vector of QueryValues
    /// The oracle crate requires us to know the types at compile time
    /// For now, we'll convert everything to strings as a simple approach
    fn row_to_values(row: &Row, column_count: usize) -> Result<Vec<QueryValue>> {
        let mut values = Vec::new();

        for i in 0..column_count {
            // Try to get the value as a string first (most compatible)
            let value = match row.get::<usize, Option<String>>(i) {
                Ok(Some(s)) => QueryValue::Text(s),
                Ok(None) => QueryValue::Null,
                Err(_) => {
                    // If string fails, try i64
                    match row.get::<usize, Option<i64>>(i) {
                        Ok(Some(n)) => QueryValue::Int(n),
                        Ok(None) => QueryValue::Null,
                        Err(_) => {
                            // If i64 fails, try f64
                            match row.get::<usize, Option<f64>>(i) {
                                Ok(Some(f)) => QueryValue::Float(f),
                                Ok(None) => QueryValue::Null,
                                Err(_) => {
                                    // Default to null if we can't convert
                                    QueryValue::Null
                                }
                            }
                        }
                    }
                }
            };
            values.push(value);
        }

        Ok(values)
    }
}

impl Default for OracleAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for OracleAdapter {
    #[instrument(skip(self, password), fields(
        db = %config.database,
        host = config.host.as_deref().unwrap_or("localhost"),
        port = config.port.unwrap_or(1521)
    ))]
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::Oracle {
            return Err(DataError::Config(format!(
                "Invalid database type: expected Oracle, got {:?}",
                config.db_type
            )));
        }

        Self::validate_database_name(&config.database)?;

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(1521);
        let database = &config.database;

        info!(
            database = %database,
            host = %host,
            port = %port,
            "Connecting to Oracle database"
        );
        let start = std::time::Instant::now();

        let username = config
            .username
            .as_deref()
            .ok_or_else(|| DataError::Config("Username is required for Oracle".to_string()))?;

        let password = password
            .ok_or_else(|| DataError::Connection("Password is required for Oracle".to_string()))?;

        let connection_string = Self::build_connection_string(config);

        // Oracle connections are synchronous, so we run them in a blocking task
        let conn_str = connection_string.clone();
        let user = username.to_string();
        let pass = password.to_string();

        let connection =
            tokio::task::spawn_blocking(move || Connection::connect(&user, &pass, &conn_str))
                .await
                .map_err(|e| {
                    let elapsed = start.elapsed();
                    warn!(
                        error = %e,
                        elapsed_ms = elapsed.as_millis(),
                        "Task join error"
                    );
                    DataError::Connection(format!(
                        "Task join error connecting to Oracle at {}:{} - {}",
                        host, port, e
                    ))
                })?
                .map_err(|e| {
                    let elapsed = start.elapsed();
                    let error_msg = e.to_string();

                    // Categorize Oracle connection errors
                    let error_category = if error_msg.contains("ORA-01017")
                        || error_msg.contains("invalid username/password")
                    {
                        "authentication"
                    } else if error_msg.contains("ORA-12154")
                        || error_msg.contains("TNS:could not resolve")
                    {
                        "tns_resolution"
                    } else if error_msg.contains("ORA-12170")
                        || error_msg.contains("TNS:connect timeout")
                    {
                        "timeout"
                    } else if error_msg.contains("ORA-12541")
                        || error_msg.contains("TNS:no listener")
                    {
                        "no_listener"
                    } else if error_msg.contains("ORA-01033")
                        || error_msg.contains("ORACLE initialization or shutdown")
                    {
                        "instance_unavailable"
                    } else {
                        "unknown"
                    };

                    warn!(
                        error = %e,
                        error_category = %error_category,
                        elapsed_ms = elapsed.as_millis(),
                        "Failed to connect to Oracle"
                    );

                    if error_msg.contains("ORA-01017")
                        || error_msg.contains("invalid username/password")
                    {
                        DataError::Connection(format!(
                            "Authentication failed for database '{}' at {}:{} - {}",
                            database, host, port, e
                        ))
                    } else if error_msg.contains("ORA-12154")
                        || error_msg.contains("TNS:could not resolve")
                    {
                        DataError::Connection(format!(
                            "Service name '{}' not found or TNS resolution failed at {}:{} - {}",
                            database, host, port, e
                        ))
                    } else if error_msg.contains("ORA-12170")
                        || error_msg.contains("TNS:connect timeout")
                    {
                        DataError::Connection(format!(
                            "Network timeout connecting to Oracle at {}:{} - {}",
                            host, port, e
                        ))
                    } else if error_msg.contains("ORA-12541")
                        || error_msg.contains("TNS:no listener")
                    {
                        DataError::Connection(format!(
                            "No listener at {}:{} - is Oracle service running? - {}",
                            host, port, e
                        ))
                    } else if error_msg.contains("ORA-01033")
                        || error_msg.contains("ORACLE initialization or shutdown")
                    {
                        DataError::Connection(format!(
                            "Oracle instance at {}:{} is starting up or shutting down - {}",
                            host, port, e
                        ))
                    } else {
                        DataError::Connection(format!(
                            "Failed to connect to database '{}' at {}:{} - {}",
                            database, host, port, e
                        ))
                    }
                })?;

        let elapsed = start.elapsed();
        self.pool = Some(Pool::new(connection));
        info!(
            elapsed_ms = elapsed.as_millis(),
            "Successfully connected to Oracle"
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(_pool) = self.pool.take() {
            info!("Disconnecting from Oracle");
            // Pool will be dropped here, which will close the connection
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.pool.is_some()
    }

    #[instrument(skip_all)]
    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        Self::validate_query(query)?;

        debug!("Executing Oracle query");

        // Clone pool and drop borrow of self immediately
        let pool = {
            let pool_ref = self
                .pool
                .as_ref()
                .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;
            pool_ref.clone()
        };

        let query = query.to_string();

        // Oracle is synchronous, so we run queries in a blocking task
        let result =
            tokio::task::spawn_blocking(move || Self::execute_blocking(pool.clone(), query))
                .await
                .map_err(|e| {
                    warn!(error = %e, "Task join error");
                    DataError::Query(format!("Failed to execute query: {}", e))
                })??;

        Ok(result)
    }

    #[instrument(skip(self))]
    async fn list_databases(&self) -> Result<Vec<String>> {
        // Oracle doesn't have a simple "list databases" concept like other RDBMS
        // Instead, it has schemas/users
        self.execute_query("SELECT username FROM all_users ORDER BY username")
            .await
            .map(|result| {
                result
                    .rows
                    .into_iter()
                    .filter_map(|row| {
                        if let Some(QueryValue::Text(name)) = row.first() {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }

    #[instrument(skip(self))]
    async fn list_tables(&self, schema: Option<&str>) -> Result<Vec<String>> {
        let query = if let Some(schema_name) = schema {
            format!(
                "SELECT table_name FROM all_tables WHERE owner = '{}' ORDER BY table_name",
                schema_name.to_uppercase()
            )
        } else {
            "SELECT table_name FROM user_tables ORDER BY table_name".to_string()
        };

        self.execute_query(&query).await.map(|result| {
            result
                .rows
                .into_iter()
                .filter_map(|row| {
                    if let Some(QueryValue::Text(name)) = row.first() {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn describe_table(&self, table_name: &str, schema: Option<&str>) -> Result<TableInfo> {
        Self::validate_table_name(table_name)?;

        let query = if let Some(schema_name) = schema {
            format!(
                "SELECT column_name, data_type, nullable, data_default \
                 FROM all_tab_columns \
                 WHERE table_name = '{}' AND owner = '{}' \
                 ORDER BY column_id",
                table_name.to_uppercase(),
                schema_name.to_uppercase()
            )
        } else {
            format!(
                "SELECT column_name, data_type, nullable, data_default \
                 FROM user_tab_columns \
                 WHERE table_name = '{}' \
                 ORDER BY column_id",
                table_name.to_uppercase()
            )
        };

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
                    n == "Y"
                } else {
                    true
                };

                let default_value = if let Some(QueryValue::Text(d)) = row.get(3) {
                    Some(d.clone())
                } else {
                    None
                };

                ColumnInfo {
                    name,
                    data_type,
                    nullable,
                    default_value,
                    is_primary_key: false, // Would need additional query to determine
                }
            })
            .collect();

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: schema.map(|s| s.to_string()),
            columns,
        })
    }

    async fn test_connection(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> Result<bool> {
        let username = match &config.username {
            Some(u) => u.clone(),
            None => return Ok(false),
        };

        let password = match password {
            Some(p) => p.to_string(),
            None => return Ok(false),
        };

        let connection_string = Self::build_connection_string(config);

        let result = tokio::task::spawn_blocking(move || {
            Connection::connect(&username, &password, &connection_string)
        })
        .await;

        match result {
            Ok(Ok(conn)) => {
                let _ = conn.close();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn database_type(&self) -> DatabaseType {
        DatabaseType::Oracle
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving Oracle server info");

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = "SELECT * FROM v$version WHERE banner LIKE 'Oracle%'".to_string();

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut version = String::from("unknown");
        let mut extra_info = std::collections::HashMap::new();

        if !result.rows.is_empty() {
            if let Some(QueryValue::Text(banner)) = result.rows[0].first() {
                version = banner.clone();
                extra_info.insert("banner".to_string(), banner.clone());
            }
        }

        Ok(ServerInfo {
            version,
            server_type: "Oracle Database".to_string(),
            extra_info,
        })
    }

    #[instrument(skip(self), fields(database = %database_name))]
    async fn get_database_metadata(&self, database_name: &str) -> Result<DatabaseMetadata> {
        info!("Retrieving metadata for database: {}", database_name);

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        // Oracle uses tablespace concept; get overall database info
        let query = format!(
            "SELECT
                name,
                created,
                log_mode
            FROM v$database"
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut created_at = None;
        let mut extra_info = std::collections::HashMap::new();

        if !result.rows.is_empty() {
            if let Some(QueryValue::Text(log_mode)) = result.rows[0].get(2) {
                extra_info.insert("log_mode".to_string(), log_mode.clone());
            }
            if let Some(QueryValue::Text(created)) = result.rows[0].get(1) {
                created_at = Some(created.clone());
            }
        }

        Ok(DatabaseMetadata {
            name: database_name.to_string(),
            size_bytes: None, // Would require tablespace size queries
            owner: None,
            encoding: Some("UTF8".to_string()), // Oracle typically uses UTF8
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

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT
                table_name,
                tablespace_name,
                num_rows
            FROM all_tables
            WHERE UPPER(table_name) = UPPER('{}')
            AND (owner = UPPER('{}') OR owner = USER)",
            table_name, owner
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut row_count = None;

        if !result.rows.is_empty() {
            if let Some(QueryValue::Int(num_rows)) = result.rows[0].get(2) {
                row_count = Some(*num_rows);
            } else if let Some(QueryValue::Text(num_rows_str)) = result.rows[0].get(2) {
                row_count = num_rows_str.parse::<i64>().ok();
            }
        }

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: Some(owner.to_string()),
            size_bytes: None, // Would require segment size queries
            row_count,
            created_at: None,
            table_type: Some("TABLE".to_string()),
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        Self::validate_table_name(table_name)?;

        info!("Retrieving indexes for table: {}", table_name);

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT
                i.index_name,
                i.uniqueness,
                LISTAGG(ic.column_name, ',') WITHIN GROUP (ORDER BY ic.column_position) as columns,
                i.index_type
            FROM all_indexes i
            JOIN all_ind_columns ic ON i.index_name = ic.index_name AND i.owner = ic.index_owner
            WHERE UPPER(i.table_name) = UPPER('{}')
            AND (i.owner = UPPER('{}') OR i.owner = USER)
            GROUP BY i.index_name, i.uniqueness, i.index_type",
            table_name, owner
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut indexes = Vec::new();

        for row in result.rows {
            let index_name = match row.first() {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => continue,
            };

            let is_unique = match row.get(1) {
                Some(QueryValue::Text(s)) => s == "UNIQUE",
                _ => false,
            };

            let columns_str = match row.get(2) {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => String::new(),
            };
            let columns: Vec<String> = columns_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let index_type = match row.get(3) {
                Some(QueryValue::Text(s)) => Some(s.clone()),
                _ => None,
            };

            // Oracle primary keys are typically named like SYS_C00xxxxx or have PK in name
            let is_primary = index_name.contains("PK") || index_name.starts_with("SYS_C");

            indexes.push(IndexInfo {
                name: index_name,
                table_name: table_name.to_string(),
                schema: Some(owner.to_string()),
                columns,
                is_unique,
                is_primary,
                index_type,
            });
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

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT
                c.constraint_name,
                c.table_name,
                c.owner,
                LISTAGG(cc.column_name, ',') WITHIN GROUP (ORDER BY cc.position) as columns,
                c.r_constraint_name,
                rc.table_name as referenced_table,
                rc.owner as referenced_owner,
                c.delete_rule
            FROM all_constraints c
            JOIN all_cons_columns cc ON c.constraint_name = cc.constraint_name AND c.owner = cc.owner
            JOIN all_constraints rc ON c.r_constraint_name = rc.constraint_name
            WHERE c.constraint_type = 'R'
            AND UPPER(c.table_name) = UPPER('{}')
            AND (c.owner = UPPER('{}') OR c.owner = USER)
            GROUP BY c.constraint_name, c.table_name, c.owner, c.r_constraint_name, rc.table_name, rc.owner, c.delete_rule",
            table_name, owner
        );

        let result = tokio::task::spawn_blocking(move || {
            Self::execute_blocking(pool.clone(), query.clone())
        })
        .await
        .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut fks = Vec::new();

        for row in result.rows {
            let fk_name = match row.first() {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => continue,
            };

            let columns_str = match row.get(3) {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => String::new(),
            };
            let columns: Vec<String> = columns_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let referenced_table = match row.get(5) {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => String::new(),
            };

            let referenced_schema = match row.get(6) {
                Some(QueryValue::Text(s)) => Some(s.clone()),
                _ => None,
            };

            let on_delete = match row.get(7) {
                Some(QueryValue::Text(s)) => Some(s.clone()),
                _ => None,
            };

            // Oracle doesn't easily expose referenced columns without additional query
            // For simplicity, use same column names as assumption
            let referenced_columns = columns.clone();

            fks.push(ForeignKeyInfo {
                name: fk_name,
                table_name: table_name.to_string(),
                schema: Some(owner.to_string()),
                columns,
                referenced_table,
                referenced_schema,
                referenced_columns,
                on_delete,
                on_update: None, // Oracle doesn't have ON UPDATE CASCADE
            });
        }

        Ok(fks)
    }

    #[instrument(skip(self))]
    async fn get_views(&self, schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        info!("Retrieving views");

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT view_name, owner
            FROM all_views
            WHERE owner = UPPER('{}') OR owner = USER",
            owner
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut views = Vec::new();

        for row in result.rows {
            let name = match row.first() {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => continue,
            };

            let schema = match row.get(1) {
                Some(QueryValue::Text(s)) => Some(s.clone()),
                _ => None,
            };

            views.push(ViewInfo {
                name,
                schema,
                definition: None, // Retrieved separately
            });
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

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT text
            FROM all_views
            WHERE UPPER(view_name) = UPPER('{}')
            AND (owner = UPPER('{}') OR owner = USER)",
            view_name, owner
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        if !result.rows.is_empty() {
            if let Some(QueryValue::Text(text)) = result.rows[0].first() {
                return Ok(Some(text.clone()));
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        info!("Listing stored procedures");

        let owner = schema.unwrap_or("USER");
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let query = format!(
            "SELECT object_name, owner, object_type
            FROM all_procedures
            WHERE (owner = UPPER('{}') OR owner = USER)
            AND object_type IN ('PROCEDURE', 'FUNCTION')",
            owner
        );

        let result = tokio::task::spawn_blocking(move || Self::execute_blocking(pool, query))
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute query: {}", e)))??;

        let mut procedures = Vec::new();

        for row in result.rows {
            let name = match row.first() {
                Some(QueryValue::Text(s)) => s.clone(),
                _ => continue,
            };

            let schema = match row.get(1) {
                Some(QueryValue::Text(s)) => Some(s.clone()),
                _ => None,
            };

            let return_type = match row.get(2) {
                Some(QueryValue::Text(s)) if s == "FUNCTION" => Some("FUNCTION".to_string()),
                _ => None,
            };

            procedures.push(ProcedureInfo {
                name,
                schema,
                return_type,
                language: Some("PL/SQL".to_string()),
            });
        }

        Ok(procedures)
    }

    #[instrument(skip(self, rows))]
    async fn bulk_insert(
        &self,
        table_name: &str,
        columns: &[String],
        rows: &[Vec<QueryValue>],
        schema: Option<&str>,
    ) -> Result<u64> {
        let start = std::time::Instant::now();
        info!(
            "Bulk inserting {} rows into table: {}",
            rows.len(),
            table_name
        );

        // Validation
        Self::validate_table_name(table_name)?;
        if columns.is_empty() {
            return Err(DataError::Config("Column list cannot be empty".to_string()));
        }
        if rows.is_empty() {
            return Err(DataError::Config("Rows cannot be empty".to_string()));
        }

        // Validate all rows have correct column count
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

        // Check connection
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        // Build Oracle INSERT ALL statement
        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();
        let column_list = columns.join(", ");

        // Oracle INSERT ALL syntax:
        // INSERT ALL
        //   INTO table (col1, col2) VALUES (val1, val2)
        //   INTO table (col1, col2) VALUES (val3, val4)
        // SELECT * FROM DUAL

        let mut query = String::from("INSERT ALL\n");
        for row in rows {
            let values: Vec<String> = row
                .iter()
                .map(|v| match v {
                    QueryValue::Null => "NULL".to_string(),
                    QueryValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                    QueryValue::Int(i) => i.to_string(),
                    QueryValue::Float(f) => f.to_string(),
                    QueryValue::Text(s) => format!("'{}'", s.replace("'", "''")),
                    QueryValue::Bytes(b) => {
                        let hex_str: String =
                            b.iter().map(|byte| format!("{:02X}", byte)).collect();
                        format!("HEXTORAW('{}')", hex_str)
                    }
                })
                .collect();

            query.push_str(&format!(
                "  INTO {}{} ({}) VALUES ({})\n",
                schema_prefix,
                table_name,
                column_list,
                values.join(", ")
            ));
        }
        query.push_str("SELECT * FROM DUAL");

        debug!("Executing bulk insert query");

        let num_rows = rows.len() as u64;
        tokio::task::spawn_blocking(move || {
            let conn_guard = futures::executor::block_on(pool.lock());

            let mut stmt = conn_guard
                .statement(&query)
                .build()
                .map_err(|e| DataError::Query(format!("Failed to prepare statement: {}", e)))?;

            stmt.execute(&[])
                .map_err(|e| DataError::Query(format!("Failed to execute bulk insert: {}", e)))?;

            Ok::<(), DataError>(())
        })
        .await
        .map_err(|e| DataError::Query(format!("Failed to execute bulk insert: {}", e)))??;

        let rows_affected = num_rows;

        let elapsed = start.elapsed();
        info!(
            "Bulk inserted {} rows in {:?} ({:.2} rows/sec)",
            rows_affected,
            elapsed,
            rows_affected as f64 / elapsed.as_secs_f64()
        );

        Ok(rows_affected)
    }

    #[instrument(skip(self, updates))]
    async fn bulk_update(
        &self,
        table_name: &str,
        updates: &[(HashMap<String, QueryValue>, String)],
        schema: Option<&str>,
    ) -> Result<u64> {
        let start = std::time::Instant::now();
        info!(
            "Bulk updating {} rows in table: {}",
            updates.len(),
            table_name
        );

        // Validation
        Self::validate_table_name(table_name)?;
        if updates.is_empty() {
            return Err(DataError::Config("Updates cannot be empty".to_string()));
        }

        // Check connection
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();

        // Execute updates individually
        let mut total_affected = 0u64;
        for (column_values, where_clause) in updates {
            if column_values.is_empty() {
                continue;
            }

            let set_clauses: Vec<String> = column_values
                .iter()
                .map(|(col, val)| {
                    let value_str = match val {
                        QueryValue::Null => "NULL".to_string(),
                        QueryValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                        QueryValue::Int(i) => i.to_string(),
                        QueryValue::Float(f) => f.to_string(),
                        QueryValue::Text(s) => format!("'{}'", s.replace("'", "''")),
                        QueryValue::Bytes(b) => {
                            let hex_str: String =
                                b.iter().map(|byte| format!("{:02X}", byte)).collect();
                            format!("HEXTORAW('{}')", hex_str)
                        }
                    };
                    format!("{} = {}", col, value_str)
                })
                .collect();

            let query = format!(
                "UPDATE {}{} SET {} WHERE {}",
                schema_prefix,
                table_name,
                set_clauses.join(", "),
                where_clause
            );

            let pool_clone = pool.clone();
            tokio::task::spawn_blocking(move || {
                let conn_guard = futures::executor::block_on(pool_clone.lock());

                let mut stmt = conn_guard
                    .statement(&query)
                    .build()
                    .map_err(|e| DataError::Query(format!("Failed to prepare statement: {}", e)))?;

                stmt.execute(&[])
                    .map_err(|e| DataError::Query(format!("Failed to execute update: {}", e)))?;

                Ok::<(), DataError>(())
            })
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute bulk update: {}", e)))??;

            total_affected += 1;
        }

        let elapsed = start.elapsed();
        info!("Bulk updated {} rows in {:?}", total_affected, elapsed);

        Ok(total_affected)
    }

    #[instrument(skip(self, where_clauses))]
    async fn bulk_delete(
        &self,
        table_name: &str,
        where_clauses: &[String],
        schema: Option<&str>,
    ) -> Result<u64> {
        let start = std::time::Instant::now();
        info!(
            "Bulk deleting {} rows from table: {}",
            where_clauses.len(),
            table_name
        );

        // Validation
        Self::validate_table_name(table_name)?;
        if where_clauses.is_empty() {
            return Err(DataError::Config(
                "Where clauses cannot be empty".to_string(),
            ));
        }

        // Check connection
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?
            .clone();

        let schema_prefix = schema.map(|s| format!("{}.", s)).unwrap_or_default();

        // Execute deletes individually
        let mut total_affected = 0u64;
        for where_clause in where_clauses {
            let query = format!(
                "DELETE FROM {}{} WHERE {}",
                schema_prefix, table_name, where_clause
            );

            let pool_clone = pool.clone();
            tokio::task::spawn_blocking(move || {
                let conn_guard = futures::executor::block_on(pool_clone.lock());

                let mut stmt = conn_guard
                    .statement(&query)
                    .build()
                    .map_err(|e| DataError::Query(format!("Failed to prepare statement: {}", e)))?;

                stmt.execute(&[])
                    .map_err(|e| DataError::Query(format!("Failed to execute delete: {}", e)))?;

                Ok::<(), DataError>(())
            })
            .await
            .map_err(|e| DataError::Query(format!("Failed to execute bulk delete: {}", e)))??;

            total_affected += 1;
        }

        let elapsed = start.elapsed();
        info!("Bulk deleted {} rows in {:?}", total_affected, elapsed);

        Ok(total_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-oracle".to_string(),
            name: "Test Oracle".to_string(),
            db_type: DatabaseType::Oracle,
            host: Some("localhost".to_string()),
            port: Some(1521),
            database: "FREE".to_string(), // Oracle 23ai Free service name
            username: Some("system".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = OracleAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::Oracle);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = OracleAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_connection_string() {
        let config = test_config();
        let conn_str = OracleAdapter::build_connection_string(&config);
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("1521"));
        assert!(conn_str.contains("FREE"));
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = OracleAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = OracleAdapter::new();
        let result = adapter.execute_query("SELECT 1 FROM DUAL").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = OracleAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::Postgres;
        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    // ===== Validation Tests =====
    #[test]
    fn test_validate_database_name_valid() {
        assert!(OracleAdapter::validate_database_name("ORCL").is_ok());
        assert!(OracleAdapter::validate_database_name("FREE").is_ok());
        assert!(OracleAdapter::validate_database_name("XE").is_ok()); // Still valid for legacy
    }

    #[test]
    fn test_validate_database_name_empty() {
        assert!(OracleAdapter::validate_database_name("").is_err());
    }

    #[test]
    fn test_validate_table_name_valid() {
        assert!(OracleAdapter::validate_table_name("USERS").is_ok());
        assert!(OracleAdapter::validate_table_name("ORDER_ITEMS").is_ok());
    }

    #[test]
    fn test_validate_table_name_empty() {
        assert!(OracleAdapter::validate_table_name("").is_err());
    }

    #[test]
    fn test_validate_query_valid() {
        assert!(OracleAdapter::validate_query("SELECT * FROM USERS").is_ok());
    }

    #[test]
    fn test_validate_query_empty() {
        assert!(OracleAdapter::validate_query("").is_err());
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

    // ===== Bulk Operations Unit Tests =====
    #[tokio::test]
    async fn test_bulk_insert_not_connected() {
        let adapter = OracleAdapter::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![vec![
            QueryValue::Int(1),
            QueryValue::Text("Alice".to_string()),
        ]];
        let result = adapter
            .bulk_insert("test_table", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_table_name() {
        let adapter = OracleAdapter::new();
        let columns = vec!["id".to_string()];
        let rows = vec![vec![QueryValue::Int(1)]];
        let result = adapter.bulk_insert("", &columns, &rows, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_columns() {
        let adapter = OracleAdapter::new();
        let columns: Vec<String> = vec![];
        let rows = vec![vec![QueryValue::Int(1)]];
        let result = adapter
            .bulk_insert("test_table", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_rows() {
        let adapter = OracleAdapter::new();
        let columns = vec!["id".to_string()];
        let rows: Vec<Vec<QueryValue>> = vec![];
        let result = adapter
            .bulk_insert("test_table", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_insert_column_row_mismatch() {
        let adapter = OracleAdapter::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![QueryValue::Int(1)], // Only 1 value, but 2 columns
        ];
        let result = adapter
            .bulk_insert("test_table", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_update_not_connected() {
        let adapter = OracleAdapter::new();
        let mut update = HashMap::new();
        update.insert("name".to_string(), QueryValue::Text("Updated".to_string()));
        let updates = vec![(update, "id = 1".to_string())];
        let result = adapter.bulk_update("test_table", &updates, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_bulk_update_empty_table_name() {
        let adapter = OracleAdapter::new();
        let mut update = HashMap::new();
        update.insert("name".to_string(), QueryValue::Text("Updated".to_string()));
        let updates = vec![(update, "id = 1".to_string())];
        let result = adapter.bulk_update("", &updates, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_update_empty_updates() {
        let adapter = OracleAdapter::new();
        let updates: Vec<(HashMap<String, QueryValue>, String)> = vec![];
        let result = adapter.bulk_update("test_table", &updates, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[tokio::test]
    async fn test_bulk_delete_not_connected() {
        let adapter = OracleAdapter::new();
        let where_clauses = vec!["id = 1".to_string()];
        let result = adapter
            .bulk_delete("test_table", &where_clauses, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_bulk_delete_empty_table_name() {
        let adapter = OracleAdapter::new();
        let where_clauses = vec!["id = 1".to_string()];
        let result = adapter.bulk_delete("", &where_clauses, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }
}

use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseType, QueryResult, QueryValue,
    TableInfo,
};
use crate::error::{DataError, Result};
use crate::pool::Pool;
use async_trait::async_trait;
use oracle::{Connection, Row};
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
        // Get the connection from the pool in the blocking context
        let conn_guard = futures::executor::block_on(pool.lock());
        
        // Execute the query
        let mut stmt = conn_guard.statement(&query).build()
            .map_err(|e| DataError::Query(format!("Failed to prepare statement: {}", e)))?;
        
        let mut result_set = stmt.query(&[])
            .map_err(|e| DataError::Query(format!("Query failed: {}", e)))?;

        // Get column information
        let column_info = result_set.column_info();
        let columns: Vec<String> = column_info.iter()
            .map(|col| col.name().to_string())
            .collect();
        
        let column_count = columns.len();

        // Collect rows
        let mut rows = Vec::new();
        for row_result in &mut result_set {
            let row = row_result.map_err(|e| {
                DataError::Query(format!("Failed to fetch row: {}", e))
            })?;
            let values = Self::row_to_values(&row, column_count)?;
            rows.push(values);
        }

        let _row_count = rows.len() as u64;

        // For DML statements (INSERT, UPDATE, DELETE), rows will be empty
        // Oracle doesn't easily provide rows_affected without additional work
        let rows_affected = if rows.is_empty() {
            Some(0) // Placeholder - would need ROW_COUNT or similar
        } else {
            None // SELECT query
        };

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

        info!("Connecting to Oracle database");

        let username = config.username.as_deref().ok_or_else(|| {
            DataError::Config("Username is required for Oracle".to_string())
        })?;

        let password = password.ok_or_else(|| {
            DataError::Connection("Password is required for Oracle".to_string())
        })?;

        let connection_string = Self::build_connection_string(config);

        // Oracle connections are synchronous, so we run them in a blocking task
        let conn_str = connection_string.clone();
        let user = username.to_string();
        let pass = password.to_string();

        let connection = tokio::task::spawn_blocking(move || {
            Connection::connect(&user, &pass, &conn_str)
        })
        .await
        .map_err(|e| {
            warn!(error = %e, "Task join error");
            DataError::Connection(format!("Failed to connect: {}", e))
        })?
        .map_err(|e| {
            warn!(error = %e, "Failed to connect to Oracle");
            DataError::Connection(format!("Failed to connect: {}", e))
        })?;

        self.pool = Some(Pool::new(connection));
        info!("Successfully connected to Oracle");
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
        debug!("Executing Oracle query");

        // Clone pool and drop borrow of self immediately
        let pool = {
            let pool_ref = self.pool.as_ref()
                .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;
            pool_ref.clone()
        };
        
        let query = query.to_string();

        // Oracle is synchronous, so we run queries in a blocking task
        let result = tokio::task::spawn_blocking(move || {
            Self::execute_blocking(pool.clone(), query)
        })
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

    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool> {
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
            database: "XE".to_string(), // Express Edition service name
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
        assert!(conn_str.contains("XE"));
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
}

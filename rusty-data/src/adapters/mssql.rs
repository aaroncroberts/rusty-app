use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType,
    ForeignKeyInfo, IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo, TableInfo,
    TableMetadata, ViewInfo,
};
use crate::error::{DataError, Result};
use crate::pool::Pool;
use async_trait::async_trait;
use futures_util::stream::TryStreamExt;
use tiberius::{AuthMethod, Client, Config, QueryItem, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tracing::{info, instrument, warn};

/// Microsoft SQL Server database adapter using tiberius
pub struct MssqlAdapter {
    pool: Option<Pool<Client<Compat<TcpStream>>>>,
}

impl MssqlAdapter {
    /// Create a new SQL Server adapter
    pub fn new() -> Self {
        Self { pool: None }
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

        info!("Connecting to SQL Server database");
        let tiberius_config = Self::build_config(config, password)?;

        let tcp = TcpStream::connect(tiberius_config.get_addr())
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to connect to SQL Server");
                DataError::Connection(format!("Failed to connect: {}", e))
            })?;

        let client = Client::connect(tiberius_config, tcp.compat_write())
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to authenticate with SQL Server");
                DataError::Connection(format!("Failed to connect: {}", e))
            })?;

        self.pool = Some(Pool::new(client));
        info!("Successfully connected to SQL Server");
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
        let pool = self.pool.as_ref().ok_or_else(|| {
            DataError::Connection("Not connected to database".to_string())
        })?;

        let mut client = pool.lock().await;
        
        // Execute the query
        let mut result = client.query(query, &[]).await.map_err(|e| {
            warn!(error = %e, "Query execution failed");
            DataError::Query(format!("Query failed: {}", e))
        })?;

        // Get column information from the first result set
        let columns_opt = result.columns().await.map_err(|e| {
            DataError::Query(format!("Failed to get columns: {}", e))
        })?;
        
        let columns: Vec<String> = if let Some(cols) = columns_opt {
            cols.iter().map(|col| col.name().to_string()).collect()
        } else {
            Vec::new()
        };

        // Collect rows
        let mut rows = Vec::new();
        let mut row_count = 0u64;
        
        while let Some(item) = result.try_next().await.map_err(|e| {
            DataError::Query(format!("Failed to fetch row: {}", e))
        })? {
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

    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool> {
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
}

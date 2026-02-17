use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseType, QueryResult, QueryValue,
    TableInfo,
};
use crate::error::{DataError, Result};
use async_trait::async_trait;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::{Column, Row, TypeInfo};

/// MySQL database adapter using sqlx
pub struct MySqlAdapter {
    pool: Option<MySqlPool>,
}

impl MySqlAdapter {
    /// Create a new MySQL adapter
    pub fn new() -> Self {
        Self { pool: None }
    }

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig, password: Option<&str>) -> String {
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

        format!(
            "mysql://{}:{}@{}:{}/{}?{}",
            username, password, host, port, database, ssl_mode
        )
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
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::MySQL {
            return Err(DataError::Config(format!(
                "Invalid database type: expected MySQL, got {:?}",
                config.db_type
            )));
        }

        let connection_string = Self::build_connection_string(config, password);

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .map_err(|e| DataError::Connection(format!("Failed to connect: {}", e)))?;

        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.pool.is_some()
    }

    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
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

    async fn describe_table(&self, table_name: &str, _schema: Option<&str>) -> Result<TableInfo> {
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
        let connection_string = Self::build_connection_string(config, password);

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
        let conn_str = MySqlAdapter::build_connection_string(&config, Some("password123"));
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
        let conn_str = MySqlAdapter::build_connection_string(&config, Some("password123"));
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
        let conn_str = MySqlAdapter::build_connection_string(&config, None);
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
}

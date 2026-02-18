use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Unique identifier for this connection
    pub id: String,
    /// Display name for the connection
    pub name: String,
    /// Database type (postgres, mysql, sqlite, etc.)
    pub db_type: DatabaseType,
    /// Host address (not used for file-based databases like SQLite)
    pub host: Option<String>,
    /// Port number
    pub port: Option<u16>,
    /// Database name
    pub database: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Whether to use SSL/TLS
    pub use_ssl: bool,
    /// Additional connection parameters
    pub parameters: HashMap<String, String>,
}

/// Supported database types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    MySQL,
    SQLite,
    MongoDB,
    SQLServer,
    Oracle,
}

impl DatabaseType {
    /// Returns the default port for this database type
    pub fn default_port(&self) -> Option<u16> {
        match self {
            DatabaseType::Postgres => Some(5432),
            DatabaseType::MySQL => Some(3306),
            DatabaseType::SQLite => None,
            DatabaseType::MongoDB => Some(27017),
            DatabaseType::SQLServer => Some(1433),
            DatabaseType::Oracle => Some(1521),
        }
    }
}

/// Result row from a query
#[derive(Debug, Clone)]
pub struct QueryRow {
    pub columns: Vec<String>,
    pub values: Vec<QueryValue>,
}

/// Represents a value in a query result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
}

/// Result of a query execution
#[derive(Debug)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<QueryValue>>,
    pub rows_affected: Option<u64>,
}

/// Schema information about a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub columns: Vec<ColumnInfo>,
}

/// Column information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub server_type: String,
    pub extra_info: HashMap<String, String>,
}

/// Database metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetadata {
    pub name: String,
    pub size_bytes: Option<i64>,
    pub owner: Option<String>,
    pub encoding: Option<String>,
    pub created_at: Option<String>,
    pub extra_info: HashMap<String, String>,
}

/// Index information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub schema: Option<String>,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: Option<String>,
}

/// Foreign key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub name: String,
    pub table_name: String,
    pub schema: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_schema: Option<String>,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// View information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewInfo {
    pub name: String,
    pub schema: Option<String>,
    pub definition: Option<String>,
}

/// Stored procedure/function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureInfo {
    pub name: String,
    pub schema: Option<String>,
    pub return_type: Option<String>,
    pub language: Option<String>,
}

/// Enhanced table metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub schema: Option<String>,
    pub size_bytes: Option<i64>,
    pub row_count: Option<i64>,
    pub created_at: Option<String>,
    pub table_type: Option<String>,
}

/// Main trait that all database adapters must implement
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Connect to the database using the provided configuration and credentials
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()>;

    /// Disconnect from the database
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if currently connected
    fn is_connected(&self) -> bool;

    /// Execute a query and return results
    async fn execute_query(&self, query: &str) -> Result<QueryResult>;

    /// List all databases on the server
    async fn list_databases(&self) -> Result<Vec<String>>;

    /// List all tables in the current database
    async fn list_tables(&self, schema: Option<&str>) -> Result<Vec<String>>;

    /// Get detailed information about a table
    async fn describe_table(&self, table_name: &str, schema: Option<&str>) -> Result<TableInfo>;

    /// Test the connection without fully connecting
    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool>;

    /// Get the database type this adapter handles
    fn database_type(&self) -> DatabaseType;

    // ===== Server & Database Introspection Methods =====

    /// Get server version and configuration information
    async fn get_server_info(&self) -> Result<ServerInfo> {
        // Default implementation returns basic info
        Ok(ServerInfo {
            version: "Unknown".to_string(),
            server_type: format!("{:?}", self.database_type()),
            extra_info: HashMap::new(),
        })
    }

    /// Get metadata about a specific database
    async fn get_database_metadata(&self, _database_name: &str) -> Result<DatabaseMetadata> {
        // Default implementation returns minimal info
        Ok(DatabaseMetadata {
            name: _database_name.to_string(),
            size_bytes: None,
            owner: None,
            encoding: None,
            created_at: None,
            extra_info: HashMap::new(),
        })
    }

    /// Get metadata about a specific table
    async fn get_table_metadata(&self, _table_name: &str, _schema: Option<&str>) -> Result<TableMetadata> {
        // Default implementation returns minimal info
        Ok(TableMetadata {
            name: _table_name.to_string(),
            schema: _schema.map(|s| s.to_string()),
            size_bytes: None,
            row_count: None,
            created_at: None,
            table_type: None,
        })
    }

    /// Get all indexes for a table
    async fn get_indexes(&self, _table_name: &str, _schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    /// Get all foreign keys for a table
    async fn get_foreign_keys(&self, _table_name: &str, _schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    /// List all views in a schema
    async fn get_views(&self, _schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    /// Get view definition
    async fn get_view_definition(&self, _view_name: &str, _schema: Option<&str>) -> Result<Option<String>> {
        // Default implementation returns None
        Ok(None)
    }

    /// List all stored procedures/functions in a schema
    async fn list_stored_procedures(&self, _schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock database adapter for testing
    pub struct MockAdapter {
        connected: bool,
        db_type: DatabaseType,
        fail_on_connect: bool,
        fail_on_query: bool,
    }

    impl MockAdapter {
        /// Create a new mock adapter
        pub fn new(db_type: DatabaseType) -> Self {
            Self {
                connected: false,
                db_type,
                fail_on_connect: false,
                fail_on_query: false,
            }
        }

        /// Configure the mock to fail on connect
        pub fn with_connect_failure(mut self) -> Self {
            self.fail_on_connect = true;
            self
        }

        /// Configure the mock to fail on queries
        pub fn with_query_failure(mut self) -> Self {
            self.fail_on_query = true;
            self
        }
    }

    #[async_trait]
    impl DatabaseAdapter for MockAdapter {
        async fn connect(&mut self, _config: &ConnectionConfig, _password: Option<&str>) -> Result<()> {
            if self.fail_on_connect {
                return Err(crate::error::DataError::Connection(
                    "Mock connection failure".to_string(),
                ));
            }
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<()> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn execute_query(&self, _query: &str) -> Result<QueryResult> {
            if self.fail_on_query {
                return Err(crate::error::DataError::Query("Mock query failure".to_string()));
            }

            Ok(QueryResult {
                columns: vec!["id".to_string(), "name".to_string()],
                rows: vec![
                    vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
                    vec![QueryValue::Int(2), QueryValue::Text("Bob".to_string())],
                ],
                rows_affected: Some(2),
            })
        }

        async fn list_databases(&self) -> Result<Vec<String>> {
            Ok(vec![
                "database1".to_string(),
                "database2".to_string(),
                "database3".to_string(),
            ])
        }

        async fn list_tables(&self, _schema: Option<&str>) -> Result<Vec<String>> {
            Ok(vec![
                "users".to_string(),
                "posts".to_string(),
                "comments".to_string(),
            ])
        }

        async fn describe_table(&self, table_name: &str, _schema: Option<&str>) -> Result<TableInfo> {
            Ok(TableInfo {
                name: table_name.to_string(),
                schema: Some("public".to_string()),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                        is_primary_key: true,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        default_value: Some("''".to_string()),
                        is_primary_key: false,
                    },
                ],
            })
        }

        async fn test_connection(&self, _config: &ConnectionConfig, _password: Option<&str>) -> Result<bool> {
            Ok(!self.fail_on_connect)
        }

        fn database_type(&self) -> DatabaseType {
            self.db_type
        }
    }

    /// Helper to create a test connection config
    pub fn test_connection_config(db_type: DatabaseType) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-connection".to_string(),
            name: "Test Database".to_string(),
            db_type,
            host: Some("localhost".to_string()),
            port: db_type.default_port(),
            database: "test_db".to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_mock_adapter_connect() {
        let mut adapter = MockAdapter::new(DatabaseType::Postgres);
        let config = test_connection_config(DatabaseType::Postgres);

        assert!(!adapter.is_connected());

        adapter.connect(&config, Some("password")).await.unwrap();
        assert!(adapter.is_connected());

        adapter.disconnect().await.unwrap();
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn test_mock_adapter_connect_failure() {
        let mut adapter = MockAdapter::new(DatabaseType::Postgres).with_connect_failure();
        let config = test_connection_config(DatabaseType::Postgres);

        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn test_mock_adapter_query() {
        let mut adapter = MockAdapter::new(DatabaseType::Postgres);
        let config = test_connection_config(DatabaseType::Postgres);

        adapter.connect(&config, Some("password")).await.unwrap();

        let result = adapter.execute_query("SELECT * FROM users").await.unwrap();
        assert_eq!(result.columns, vec!["id", "name"]);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows_affected, Some(2));
    }

    #[tokio::test]
    async fn test_mock_adapter_query_failure() {
        let mut adapter = MockAdapter::new(DatabaseType::Postgres).with_query_failure();
        let config = test_connection_config(DatabaseType::Postgres);

        adapter.connect(&config, Some("password")).await.unwrap();

        let result = adapter.execute_query("SELECT * FROM users").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_adapter_list_databases() {
        let adapter = MockAdapter::new(DatabaseType::Postgres);
        let databases = adapter.list_databases().await.unwrap();

        assert_eq!(databases.len(), 3);
        assert!(databases.contains(&"database1".to_string()));
    }

    #[tokio::test]
    async fn test_mock_adapter_list_tables() {
        let adapter = MockAdapter::new(DatabaseType::Postgres);
        let tables = adapter.list_tables(None).await.unwrap();

        assert_eq!(tables.len(), 3);
        assert!(tables.contains(&"users".to_string()));
    }

    #[tokio::test]
    async fn test_mock_adapter_describe_table() {
        let adapter = MockAdapter::new(DatabaseType::Postgres);
        let table_info = adapter.describe_table("users", None).await.unwrap();

        assert_eq!(table_info.name, "users");
        assert_eq!(table_info.schema, Some("public".to_string()));
        assert_eq!(table_info.columns.len(), 2);
        assert!(table_info.columns[0].is_primary_key);
    }

    #[tokio::test]
    async fn test_mock_adapter_test_connection() {
        let adapter = MockAdapter::new(DatabaseType::Postgres);
        let config = test_connection_config(DatabaseType::Postgres);

        let result = adapter.test_connection(&config, Some("password")).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_database_type() {
        let adapter = MockAdapter::new(DatabaseType::MySQL);
        assert_eq!(adapter.database_type(), DatabaseType::MySQL);
    }

    #[test]
    fn test_database_type_default_ports() {
        assert_eq!(DatabaseType::Postgres.default_port(), Some(5432));
        assert_eq!(DatabaseType::MySQL.default_port(), Some(3306));
        assert_eq!(DatabaseType::SQLite.default_port(), None);
        assert_eq!(DatabaseType::MongoDB.default_port(), Some(27017));
        assert_eq!(DatabaseType::SQLServer.default_port(), Some(1433));
        assert_eq!(DatabaseType::Oracle.default_port(), Some(1521));
    }

    #[test]
    fn test_connection_config_creation() {
        let config = test_connection_config(DatabaseType::Postgres);

        assert_eq!(config.id, "test-connection");
        assert_eq!(config.name, "Test Database");
        assert_eq!(config.db_type, DatabaseType::Postgres);
        assert_eq!(config.host, Some("localhost".to_string()));
        assert_eq!(config.port, Some(5432));
        assert_eq!(config.database, "test_db");
        assert_eq!(config.username, Some("test_user".to_string()));
        assert!(!config.use_ssl);
    }

    #[test]
    fn test_query_value_variants() {
        let _null = QueryValue::Null;
        let _bool = QueryValue::Bool(true);
        let _int = QueryValue::Int(42);
        let _float = QueryValue::Float(3.14);
        let _text = QueryValue::Text("hello".to_string());
        let _bytes = QueryValue::Bytes(vec![1, 2, 3]);
    }
}

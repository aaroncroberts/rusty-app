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
    // Future support
    // MongoDB,
    // Redis,
    // SQLServer,
}

impl DatabaseType {
    /// Returns the default port for this database type
    pub fn default_port(&self) -> Option<u16> {
        match self {
            DatabaseType::Postgres => Some(5432),
            DatabaseType::MySQL => Some(3306),
            DatabaseType::SQLite => None,
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
}

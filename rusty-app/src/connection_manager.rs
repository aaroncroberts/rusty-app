//! Connection manager for tracking active database connections
//!
//! This module provides structures and functionality for managing active database
//! connections throughout the application session. It handles connection lifecycle,
//! status tracking, and adapter instance management.

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::QueryResult;
use std::collections::HashMap;

/// Status of a database connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Successfully connected and ready for queries
    Connected,
    /// Not currently connected
    Disconnected,
    /// Connection encountered an error
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// Wrapper enum for different adapter types
///
/// This enum allows us to store adapters of different types in a uniform collection.
/// Each variant corresponds to a database type and holds the appropriate adapter.
pub enum AdapterInstance {
    #[cfg(feature = "postgres")]
    Postgres(rusty_data::adapters::postgres::PostgresAdapter),

    #[cfg(feature = "mysql")]
    MySQL(rusty_data::adapters::mysql::MySqlAdapter),

    #[cfg(feature = "sqlite")]
    SQLite(rusty_data::adapters::sqlite::SqliteAdapter),

    #[cfg(feature = "mongodb")]
    MongoDB(rusty_data::adapters::mongodb::MongoDbAdapter),

    #[cfg(feature = "mssql")]
    SQLServer(rusty_data::adapters::mssql::MssqlAdapter),

    #[cfg(feature = "oracle")]
    Oracle(rusty_data::adapters::oracle::OracleAdapter),
}

impl AdapterInstance {
    /// Create a new adapter instance for the given database type
    pub fn new(db_type: DatabaseType) -> Self {
        match db_type {
            #[cfg(feature = "postgres")]
            DatabaseType::Postgres => {
                Self::Postgres(rusty_data::adapters::postgres::PostgresAdapter::new())
            }
            #[cfg(feature = "mysql")]
            DatabaseType::MySQL => {
                Self::MySQL(rusty_data::adapters::mysql::MySqlAdapter::new())
            }
            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => {
                Self::SQLite(rusty_data::adapters::sqlite::SqliteAdapter::new())
            }
            #[cfg(feature = "mongodb")]
            DatabaseType::MongoDB => {
                Self::MongoDB(rusty_data::adapters::mongodb::MongoDbAdapter::new())
            }
            #[cfg(feature = "mssql")]
            DatabaseType::SQLServer => {
                Self::SQLServer(rusty_data::adapters::mssql::MssqlAdapter::new())
            }
            #[cfg(feature = "oracle")]
            DatabaseType::Oracle => {
                Self::Oracle(rusty_data::adapters::oracle::OracleAdapter::new())
            }
            #[cfg(not(feature = "postgres"))]
            DatabaseType::Postgres => panic!("PostgreSQL support not compiled in"),
            #[cfg(not(feature = "mysql"))]
            DatabaseType::MySQL => panic!("MySQL support not compiled in"),
            #[cfg(not(feature = "sqlite"))]
            DatabaseType::SQLite => panic!("SQLite support not compiled in"),
            #[cfg(not(feature = "mongodb"))]
            DatabaseType::MongoDB => panic!("MongoDB support not compiled in"),
            #[cfg(not(feature = "mssql"))]
            DatabaseType::SQLServer => panic!("SQL Server support not compiled in"),
            #[cfg(not(feature = "oracle"))]
            DatabaseType::Oracle => panic!("Oracle support not compiled in"),
        }
    }

    /// Connect to the database
    pub async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> rusty_data::Result<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(adapter) => adapter.connect(config, password).await,
            #[cfg(feature = "mysql")]
            Self::MySQL(adapter) => adapter.connect(config, password).await,
            #[cfg(feature = "sqlite")]
            Self::SQLite(adapter) => adapter.connect(config, password).await,
            #[cfg(feature = "mongodb")]
            Self::MongoDB(adapter) => adapter.connect(config, password).await,
            #[cfg(feature = "mssql")]
            Self::SQLServer(adapter) => adapter.connect(config, password).await,
            #[cfg(feature = "oracle")]
            Self::Oracle(adapter) => adapter.connect(config, password).await,
        }
    }

    /// Disconnect from the database
    pub async fn disconnect(&mut self) -> rusty_data::Result<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(adapter) => adapter.disconnect().await,
            #[cfg(feature = "mysql")]
            Self::MySQL(adapter) => adapter.disconnect().await,
            #[cfg(feature = "sqlite")]
            Self::SQLite(adapter) => adapter.disconnect().await,
            #[cfg(feature = "mongodb")]
            Self::MongoDB(adapter) => adapter.disconnect().await,
            #[cfg(feature = "mssql")]
            Self::SQLServer(adapter) => adapter.disconnect().await,
            #[cfg(feature = "oracle")]
            Self::Oracle(adapter) => adapter.disconnect().await,
        }
    }

    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(adapter) => adapter.is_connected(),
            #[cfg(feature = "mysql")]
            Self::MySQL(adapter) => adapter.is_connected(),
            #[cfg(feature = "sqlite")]
            Self::SQLite(adapter) => adapter.is_connected(),
            #[cfg(feature = "mongodb")]
            Self::MongoDB(adapter) => adapter.is_connected(),
            #[cfg(feature = "mssql")]
            Self::SQLServer(adapter) => adapter.is_connected(),
            #[cfg(feature = "oracle")]
            Self::Oracle(adapter) => adapter.is_connected(),
        }
    }

    /// Execute a query
    pub async fn execute_query(&mut self, query: &str) -> rusty_data::Result<QueryResult> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(adapter) => adapter.execute_query(query).await,
            #[cfg(feature = "mysql")]
            Self::MySQL(adapter) => adapter.execute_query(query).await,
            #[cfg(feature = "sqlite")]
            Self::SQLite(adapter) => adapter.execute_query(query).await,
            #[cfg(feature = "mongodb")]
            Self::MongoDB(adapter) => adapter.execute_query(query).await,
            #[cfg(feature = "mssql")]
            Self::SQLServer(adapter) => adapter.execute_query(query).await,
            #[cfg(feature = "oracle")]
            Self::Oracle(adapter) => adapter.execute_query(query).await,
        }
    }
}

/// Represents an active database connection with its associated state
pub struct ActiveConnection {
    /// Unique identifier for this connection
    pub id: String,
    /// Connection configuration
    pub config: ConnectionConfig,
    /// The database adapter instance
    pub adapter: AdapterInstance,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Optional password (stored in memory for reconnection)
    pub password: Option<String>,
}

impl ActiveConnection {
    /// Create a new active connection
    pub fn new(config: ConnectionConfig, password: Option<String>) -> Self {
        let id = config.id.clone();
        let adapter = AdapterInstance::new(config.db_type);

        Self {
            id,
            config,
            adapter,
            status: ConnectionStatus::Disconnected,
            password,
        }
    }

    /// Attempt to establish connection
    pub async fn connect(&mut self) -> Result<(), String> {
        let password_ref = self.password.as_deref();

        match self.adapter.connect(&self.config, password_ref).await {
            Ok(()) => {
                self.status = ConnectionStatus::Connected;
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.status = ConnectionStatus::Error(error_msg.clone());
                Err(error_msg)
            }
        }
    }

    /// Disconnect from the database
    pub async fn disconnect(&mut self) -> Result<(), String> {
        match self.adapter.disconnect().await {
            Ok(()) => {
                self.status = ConnectionStatus::Disconnected;
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.status = ConnectionStatus::Error(error_msg.clone());
                Err(error_msg)
            }
        }
    }

    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        self.adapter.is_connected() && self.status == ConnectionStatus::Connected
    }

    /// Execute a query on this connection
    pub async fn execute_query(&mut self, query: &str) -> Result<QueryResult, String> {
        if !self.is_connected() {
            return Err("Not connected to database".to_string());
        }

        self.adapter.execute_query(query)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Manager for all active database connections
#[derive(Default)]
pub struct ConnectionManager {
    /// Map of connection ID to active connection
    connections: HashMap<String, ActiveConnection>,
    /// Currently selected connection ID
    selected_id: Option<String>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new connection
    pub fn add_connection(&mut self, connection: ActiveConnection) {
        let id = connection.id.clone();
        self.connections.insert(id, connection);
    }

    /// Remove a connection by ID
    pub fn remove_connection(&mut self, id: &str) -> Option<ActiveConnection> {
        // If removing the selected connection, clear the selection
        if self.selected_id.as_deref() == Some(id) {
            self.selected_id = None;
        }
        self.connections.remove(id)
    }

    /// Get a connection by ID
    pub fn get_connection(&self, id: &str) -> Option<&ActiveConnection> {
        self.connections.get(id)
    }

    /// Get a mutable reference to a connection by ID
    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut ActiveConnection> {
        self.connections.get_mut(id)
    }

    /// Get all connection IDs
    pub fn connection_ids(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    /// Get the currently selected connection ID
    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    /// Set the selected connection
    pub fn set_selected(&mut self, id: Option<String>) {
        self.selected_id = id;
    }

    /// Get the currently selected connection
    pub fn selected_connection(&self) -> Option<&ActiveConnection> {
        self.selected_id.as_ref()
            .and_then(|id| self.connections.get(id))
    }

    /// Get a mutable reference to the currently selected connection
    pub fn selected_connection_mut(&mut self) -> Option<&mut ActiveConnection> {
        if let Some(id) = &self.selected_id {
            self.connections.get_mut(id)
        } else {
            None
        }
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Disconnect all connections
    pub async fn disconnect_all(&mut self) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();

        for (id, connection) in self.connections.iter_mut() {
            let result = connection.disconnect().await;
            results.push((id.clone(), result));
        }

        results
    }
}

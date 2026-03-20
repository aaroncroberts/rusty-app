//! Connection manager for tracking active database connections

use arni::adapter::DbAdapter;
use arni::{ConnectionConfig, DatabaseType, QueryResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Status of a database connection
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    Connected,
    #[default]
    Disconnected,
    Error(String),
}

/// Create an arni adapter for the given database type and config
fn make_adapter(config: ConnectionConfig) -> Box<dyn DbAdapter + Send + Sync> {
    match config.db_type {
        #[cfg(feature = "postgres")]
        DatabaseType::Postgres => Box::new(arni::adapters::postgres::PostgresAdapter::new(config)),
        #[cfg(feature = "mysql")]
        DatabaseType::MySQL => Box::new(arni::adapters::mysql::MySqlAdapter::new(config)),
        #[cfg(feature = "sqlite")]
        DatabaseType::SQLite => Box::new(arni::adapters::sqlite::SqliteAdapter::new(config)),
        #[cfg(feature = "mongodb")]
        DatabaseType::MongoDB => Box::new(arni::adapters::mongodb::MongoDbAdapter::new(config)),
        #[cfg(feature = "mssql")]
        DatabaseType::SQLServer => Box::new(arni::adapters::mssql::SqlServerAdapter::new(config)),
        #[cfg(feature = "oracle")]
        DatabaseType::Oracle => Box::new(arni::adapters::oracle::OracleAdapter::new(config)),
        #[allow(unreachable_patterns)]
        _ => panic!("Database type {:?} not compiled in", config.db_type),
    }
}

/// Represents an active database connection with its associated state
pub struct ActiveConnection {
    pub id: String,
    pub config: ConnectionConfig,
    pub adapter: Arc<Mutex<Box<dyn DbAdapter + Send + Sync>>>,
    pub status: ConnectionStatus,
    pub password: Option<String>,
}

impl ActiveConnection {
    pub fn new(config: ConnectionConfig, password: Option<String>) -> Self {
        let id = config.id.clone();
        let adapter = Arc::new(Mutex::new(make_adapter(config.clone())));
        Self {
            id,
            config,
            adapter,
            status: ConnectionStatus::Disconnected,
            password,
        }
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        let password_ref = self.password.as_deref();
        let mut adapter = self.adapter.lock().await;
        match adapter.connect(&self.config, password_ref).await {
            Ok(()) => {
                self.status = ConnectionStatus::Connected;
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                self.status = ConnectionStatus::Error(msg.clone());
                Err(msg)
            }
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), String> {
        let mut adapter = self.adapter.lock().await;
        match adapter.disconnect().await {
            Ok(()) => {
                self.status = ConnectionStatus::Disconnected;
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                self.status = ConnectionStatus::Error(msg.clone());
                Err(msg)
            }
        }
    }

    pub async fn is_connected(&self) -> bool {
        let adapter = self.adapter.lock().await;
        adapter.is_connected() && self.status == ConnectionStatus::Connected
    }

    pub async fn execute_query(&self, query: &str) -> Result<QueryResult, String> {
        let adapter = self.adapter.lock().await;
        adapter
            .execute_query(query)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Manager for all active database connections
#[derive(Default)]
pub struct ConnectionManager {
    connections: HashMap<String, ActiveConnection>,
    selected_id: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_connection(&mut self, connection: ActiveConnection) {
        let id = connection.id.clone();
        self.connections.insert(id, connection);
    }

    pub fn remove_connection(&mut self, id: &str) -> Option<ActiveConnection> {
        if self.selected_id.as_deref() == Some(id) {
            self.selected_id = None;
        }
        self.connections.remove(id)
    }

    pub fn get_connection(&self, id: &str) -> Option<&ActiveConnection> {
        self.connections.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut ActiveConnection> {
        self.connections.get_mut(id)
    }

    pub fn connection_ids(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    pub fn set_selected(&mut self, id: Option<String>) {
        self.selected_id = id;
    }

    pub fn selected_connection(&self) -> Option<&ActiveConnection> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.connections.get(id))
    }

    pub fn selected_connection_mut(&mut self) -> Option<&mut ActiveConnection> {
        if let Some(id) = &self.selected_id {
            self.connections.get_mut(id)
        } else {
            None
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub async fn disconnect_all(&mut self) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();
        for (id, connection) in self.connections.iter_mut() {
            let result = connection.disconnect().await;
            results.push((id.clone(), result));
        }
        results
    }
}

//! rusty-data - Data access library for rusty-app
//!
//! This library provides a unified interface for connecting to multiple database systems
//! using the adapter pattern. It handles configuration, credential encryption, and
//! database operations for PostgreSQL, MySQL, and SQLite.
//!
//! # Features
//!
//! - **Adapter Pattern**: Common `DatabaseAdapter` trait for all database types
//! - **Async-First**: All operations use `tokio` for non-blocking I/O
//! - **Secure Storage**: AES-GCM encryption for credentials
//! - **TOML Configuration**: Simple, human-readable config files
//!
//! # Example
//!
//! ```no_run
//! use rusty_data::{ConnectionConfig, DatabaseType, config::ConfigManager};
//!
//! # async fn example() -> rusty_data::Result<()> {
//! // Load configurations
//! let config_manager = ConfigManager::new("~/.config/rusty-app")?;
//! let connections = config_manager.load_connections()?;
//!
//! // Create a connection
//! let conn_config = ConnectionConfig {
//!     id: "my-postgres".to_string(),
//!     name: "My Database".to_string(),
//!     db_type: DatabaseType::Postgres,
//!     host: Some("localhost".to_string()),
//!     port: Some(5432),
//!     database: "mydb".to_string(),
//!     username: Some("user".to_string()),
//!     use_ssl: false,
//!     parameters: Default::default(),
//! };
//! # Ok(())
//! # }
//! ```

// Re-export main types
pub use adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseType, QueryResult, QueryValue,
    TableInfo,
};
pub use error::{DataError, Result};

// Modules
pub mod adapter;
pub mod config;
pub mod error;

// Optional database adapter modules (enabled via features)
#[cfg(feature = "postgres")]
pub mod adapters;

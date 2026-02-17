/// Integration tests for database adapters
///
/// These tests require actual database instances running:
/// - PostgreSQL on localhost:5432 (rusty_test database)
/// - MySQL on localhost:3306 (rusty_test database)
///
/// Run: podman-compose up -d (from repository root)
/// Then: cargo test --package rusty-data --features all-databases -- --test-threads=1

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::error::Result;

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;
    use rusty_data::adapters::postgres::PostgresAdapter;

    fn get_postgres_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-postgres".to_string(),
            name: "Test PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "rusty_test".to_string(),
            username: Some("rusty_user".to_string()),
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_postgres_connect_disconnect() -> Result<()> {
        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config();

        // Connect
        adapter.connect(&config, Some("rusty_pass")).await?;
        assert!(adapter.is_connected());

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_execute_query() -> Result<()> {
        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        // Query test data
        let result = adapter.execute_query("SELECT * FROM users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5); // id, username, email, created_at, is_active
        assert_eq!(result.rows.len(), 3); // 3 test users

        // Check first user
        let first_row = &result.rows[0];
        assert_eq!(first_row.len(), 5);

        adapter.disconnect().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_list_tables() -> Result<()> {
        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        let tables = adapter.list_tables(Some("public")).await?;

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"products".to_string()));
        assert!(tables.contains(&"orders".to_string()));

        adapter.disconnect().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_describe_table() -> Result<()> {
        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        let table_info = adapter.describe_table("users", Some("public")).await?;

        assert_eq!(table_info.name, "users");
        assert!(table_info.columns.len() >= 5);

        // Check for expected columns
        let column_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"username".to_string()));
        assert!(column_names.contains(&"email".to_string()));

        adapter.disconnect().await?;
        Ok(())
    }
}

#[cfg(feature = "mysql")]
mod mysql_tests {
    use super::*;
    use rusty_data::adapters::mysql::MySqlAdapter;

    fn get_mysql_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mysql".to_string(),
            name: "Test MySQL".to_string(),
            db_type: DatabaseType::MySQL,
            host: Some("localhost".to_string()),
            port: Some(3306),
            database: "rusty_test".to_string(),
            username: Some("rusty_user".to_string()),
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_mysql_connect_disconnect() -> Result<()> {
        let mut adapter = MySqlAdapter::new();
        let config = get_mysql_config();

        // Connect
        adapter.connect(&config, Some("rusty_pass")).await?;
        assert!(adapter.is_connected());

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());

        Ok(())
    }

    #[tokio::test]
    async fn test_mysql_execute_query() -> Result<()> {
        let mut adapter = MySqlAdapter::new();
        let config = get_mysql_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        // Query test data
        let result = adapter.execute_query("SELECT * FROM users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5); // id, username, email, created_at, is_active
        assert_eq!(result.rows.len(), 3); // 3 test users

        adapter.disconnect().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mysql_list_tables() -> Result<()> {
        let mut adapter = MySqlAdapter::new();
        let config = get_mysql_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"products".to_string()));
        assert!(tables.contains(&"orders".to_string()));

        adapter.disconnect().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mysql_describe_table() -> Result<()> {
        let mut adapter = MySqlAdapter::new();
        let config = get_mysql_config();

        adapter.connect(&config, Some("rusty_pass")).await?;

        let table_info = adapter.describe_table("users", None).await?;

        assert_eq!(table_info.name, "users");
        assert!(table_info.columns.len() >= 5);

        // Check for expected columns
        let column_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"username".to_string()));
        assert!(column_names.contains(&"email".to_string()));

        adapter.disconnect().await?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use super::*;
    use rusty_data::adapters::sqlite::SqliteAdapter;
    use std::fs;

    fn get_sqlite_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-sqlite".to_string(),
            name: "Test SQLite".to_string(),
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            database: "./test-data/test.db".to_string(),
            username: None,
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    async fn setup_sqlite_test_db() -> Result<()> {
        // Create test database with sample data
        let config = get_sqlite_config();
        let mut adapter = SqliteAdapter::new();

        adapter.connect(&config, None).await?;

        // Create tables
        adapter.execute_query("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                is_active INTEGER DEFAULT 1
            )
        ").await?;

        adapter.execute_query("
            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                price REAL NOT NULL,
                stock INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT OR IGNORE INTO users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;

        adapter.disconnect().await?;
        Ok(())
    }

    fn cleanup_sqlite_test_db() {
        let _ = fs::remove_file("./test-data/test.db");
    }

    #[tokio::test]
    async fn test_sqlite_connect_disconnect() -> Result<()> {
        cleanup_sqlite_test_db();
        setup_sqlite_test_db().await?;

        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config();

        // Connect
        adapter.connect(&config, None).await?;
        assert!(adapter.is_connected());

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());

        cleanup_sqlite_test_db();
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_execute_query() -> Result<()> {
        cleanup_sqlite_test_db();
        setup_sqlite_test_db().await?;

        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config();

        adapter.connect(&config, None).await?;

        // Query test data
        let result = adapter.execute_query("SELECT * FROM users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);

        adapter.disconnect().await?;
        cleanup_sqlite_test_db();
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_list_tables() -> Result<()> {
        cleanup_sqlite_test_db();
        setup_sqlite_test_db().await?;

        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config();

        adapter.connect(&config, None).await?;

        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"products".to_string()));

        adapter.disconnect().await?;
        cleanup_sqlite_test_db();
        Ok(())
    }
}

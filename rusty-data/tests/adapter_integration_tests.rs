/// Integration tests for database adapters
///
/// These tests are fully self-contained and encapsulated:
/// - Each test creates its own unique test database
/// - All test data is created during test execution
/// - Complete cleanup of databases and objects after test
/// - Comprehensive logging via tracing to track execution
///
/// Tests require database instances running via podman-compose:
/// Run: podman-compose up -d (from repository root)
/// Wait for containers to be healthy
/// Then: cargo test --package rusty-data --features all-databases -- --test-threads=1 --ignored
///
/// Tests are marked with #[ignore] so they don't run by default.
/// Use --ignored or --include-ignored to run them.

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::error::Result;
use tracing::{info, debug};

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;
    use rusty_data::adapters::postgres::PostgresAdapter;
    use std::sync::atomic::{AtomicBool, Ordering};

    const TEST_PASSWORD: &str = "test_password";
    const TEST_DB_NAME: &str = "rusty_test_postgres";

    static INIT: AtomicBool = AtomicBool::new(false);

    fn get_postgres_config(database: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-postgres".to_string(),
            name: "Test PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: database.to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    /// Ensure test database exists (idempotent, called once per suite)
    async fn ensure_test_database() -> Result<()> {
        let mut adapter = PostgresAdapter::new();
        let postgres_config = get_postgres_config("postgres");

        adapter.connect(&postgres_config, Some(TEST_PASSWORD)).await?;

        info!("Creating PostgreSQL test database: {}", TEST_DB_NAME);

        // Create database if it doesn't exist (will error if exists, we ignore)
        let _ = adapter.execute_query(&format!("CREATE DATABASE {}", TEST_DB_NAME)).await;

        adapter.disconnect().await?;
        debug!("PostgreSQL test database ready: {}", TEST_DB_NAME);
        Ok(())
    }

    /// Setup function called before each test
    async fn setup() -> Result<PostgresAdapter> {
        // Ensure database exists (happens once)
        if !INIT.load(Ordering::Relaxed) {
            ensure_test_database().await?;
            INIT.store(true, Ordering::Relaxed);
        }

        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config(TEST_DB_NAME);

        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        Ok(adapter)
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_connect_disconnect() -> Result<()> {
        info!("Starting test: test_postgres_connect_disconnect");

        let mut adapter = setup().await?;
        assert!(adapter.is_connected());
        debug!("Connected to test database");

        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        info!("Test completed: test_postgres_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_execute_query() -> Result<()> {
        info!("Starting test: test_postgres_execute_query");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_query_users (
                id SERIAL PRIMARY KEY,
                username VARCHAR(50),
                email VARCHAR(100),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT true
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT INTO test_query_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;
        debug!("Test data inserted");

        // Query test data
        info!("Testing execute_query");
        let result = adapter.execute_query("SELECT * FROM test_query_users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);

        let first_row = &result.rows[0];
        assert_eq!(first_row.len(), 5);
        debug!("Query results validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_query_users").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_execute_query");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_list_tables() -> Result<()> {
        info!("Starting test: test_postgres_list_tables");

        let mut adapter = setup().await?;

        // Create test tables
        info!("Creating test tables");
        adapter.execute_query("CREATE TABLE test_list_users (id SERIAL PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_products (id SERIAL PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_orders (id SERIAL PRIMARY KEY)").await?;
        debug!("Test tables created");

        // List tables
        info!("Testing list_tables");
        let tables = adapter.list_tables(Some("public")).await?;

        assert!(tables.contains(&"test_list_users".to_string()));
        assert!(tables.contains(&"test_list_products".to_string()));
        assert!(tables.contains(&"test_list_orders".to_string()));
        debug!("Found all expected tables");

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE test_list_users").await?;
        adapter.execute_query("DROP TABLE test_list_products").await?;
        adapter.execute_query("DROP TABLE test_list_orders").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_list_tables");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_describe_table() -> Result<()> {
        info!("Starting test: test_postgres_describe_table");

        let mut adapter = setup().await?;

        // Create a test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_describe_table (
                id SERIAL PRIMARY KEY,
                username VARCHAR(50) NOT NULL,
                email VARCHAR(100)
            )
        ").await?;
        debug!("Test table created");

        // Describe table
        info!("Testing describe_table");
        let table_info = adapter.describe_table("test_describe_table", None).await?;

        assert_eq!(table_info.name, "test_describe_table");
        assert_eq!(table_info.columns.len(), 3);

        let column_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"username".to_string()));
        assert!(column_names.contains(&"email".to_string()));
        debug!("Table structure validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_describe_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_describe_table");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_crud_operations() -> Result<()> {
        info!("Starting test: test_postgres_crud_operations");

        let mut adapter = setup().await?;

        // CREATE TABLE
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_crud_users (
                id SERIAL PRIMARY KEY,
                username VARCHAR(50) NOT NULL UNIQUE,
                email VARCHAR(100) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        ").await?;

        // INSERT
        info!("Testing INSERT");
        let insert_result = adapter.execute_query("
            INSERT INTO test_crud_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;
        assert!(insert_result.rows_affected.is_some());
        debug!("Inserted 3 users");

        // SELECT
        info!("Testing SELECT");
        let select_result = adapter.execute_query("SELECT * FROM test_crud_users ORDER BY id").await?;
        assert_eq!(select_result.columns.len(), 4);
        assert_eq!(select_result.rows.len(), 3);
        assert_eq!(select_result.columns, vec!["id", "username", "email", "created_at"]);
        debug!("SELECT validated");

        // UPDATE
        info!("Testing UPDATE");
        adapter.execute_query("UPDATE test_crud_users SET email = 'alice@newdomain.com' WHERE username = 'alice'").await?;
        let updated = adapter.execute_query("SELECT email FROM test_crud_users WHERE username = 'alice'").await?;
        assert_eq!(updated.rows.len(), 1);
        debug!("UPDATE validated");

        // DELETE
        info!("Testing DELETE");
        adapter.execute_query("DELETE FROM test_crud_users WHERE username = 'bob'").await?;
        let remaining = adapter.execute_query("SELECT * FROM test_crud_users ORDER BY id").await?;
        assert_eq!(remaining.rows.len(), 2);
        debug!("DELETE validated");

        // Cleanup: Drop test table
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_crud_users").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_crud_operations");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_list_databases() -> Result<()> {
        info!("Starting test: test_postgres_list_databases");

        let mut adapter = setup().await?;

        // List databases
        info!("Testing list_databases");
        let databases = adapter.list_databases().await?;

        // Should at least have postgres, template0, template1, and our test database
        assert!(!databases.is_empty());
        assert!(databases.contains(&TEST_DB_NAME.to_string()));
        assert!(databases.contains(&"postgres".to_string()));
        debug!("Found expected databases");

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_list_databases");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_error_invalid_credentials() -> Result<()> {
        info!("Starting test: test_postgres_error_invalid_credentials");

        let mut adapter = PostgresAdapter::new();
        let config = get_postgres_config(TEST_DB_NAME);

        // Try to connect with wrong password
        info!("Testing invalid credentials");
        let result = adapter.connect(&config, Some("wrong_password")).await;

        // Should fail with connection error
        assert!(result.is_err());
        assert!(!adapter.is_connected());
        debug!("Invalid credentials correctly rejected");

        info!("Test completed: test_postgres_error_invalid_credentials");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_error_invalid_query() -> Result<()> {
        info!("Starting test: test_postgres_error_invalid_query");

        let mut adapter = setup().await?;

        // Try to query non-existent table
        info!("Testing query on non-existent table");
        let result = adapter.execute_query("SELECT * FROM non_existent_table").await;
        assert!(result.is_err());
        debug!("Non-existent table query correctly failed");

        // Try invalid SQL syntax
        info!("Testing invalid SQL syntax");
        let result = adapter.execute_query("INVALID SQL SYNTAX").await;
        assert!(result.is_err());
        debug!("Invalid SQL syntax correctly failed");

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_error_invalid_query");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_connection_pooling() -> Result<()> {
        info!("Starting test: test_postgres_connection_pooling");

        let mut adapter = setup().await?;

        // Execute multiple queries sequentially (using connection pool)
        info!("Testing connection pooling with multiple queries");
        for i in 0..5 {
            let result = adapter.execute_query(&format!("SELECT {} as value", i)).await?;
            assert_eq!(result.rows.len(), 1);
            debug!("Query {} completed", i);
        }
        debug!("All pooled queries completed successfully");

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_connection_pooling");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_list_tables_with_schemas() -> Result<()> {
        info!("Starting test: test_postgres_list_tables_with_schemas");

        let mut adapter = setup().await?;

        // Create a test schema and table
        info!("Creating test schema and table");
        adapter.execute_query("CREATE SCHEMA test_schema").await?;
        adapter.execute_query("
            CREATE TABLE test_schema.test_table (
                id SERIAL PRIMARY KEY,
                data VARCHAR(100)
            )
        ").await?;
        debug!("Test schema and table created");

        // List tables in test_schema
        info!("Testing list_tables with schema");
        let tables = adapter.list_tables(Some("test_schema")).await?;
        assert!(tables.contains(&"test_table".to_string()));
        debug!("Found table in test schema");

        // Cleanup
        info!("Cleaning up test schema and table");
        adapter.execute_query("DROP TABLE test_schema.test_table").await?;
        adapter.execute_query("DROP SCHEMA test_schema").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_list_tables_with_schemas");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_server_info() -> Result<()> {
        info!("Starting test: test_postgres_get_server_info");

        let mut adapter = setup().await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "PostgreSQL");
        assert!(!server_info.version.is_empty());
        assert!(server_info.version.contains("PostgreSQL"));
        debug!("Server version: {}", server_info.version);
        debug!("Server settings: {:?}", server_info.extra_info);

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_server_info");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_database_metadata() -> Result<()> {
        info!("Starting test: test_postgres_get_database_metadata");

        let mut adapter = setup().await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata(TEST_DB_NAME).await?;

        assert_eq!(db_meta.name, TEST_DB_NAME);
        assert!(db_meta.size_bytes.is_some());
        assert!(db_meta.owner.is_some());
        assert!(db_meta.encoding.is_some());
        debug!("Database size: {:?} bytes", db_meta.size_bytes);
        debug!("Database owner: {:?}", db_meta.owner);
        debug!("Database encoding: {:?}", db_meta.encoding);

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_table_metadata() -> Result<()> {
        info!("Starting test: test_postgres_get_table_metadata");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_metadata_table (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL
            )
        ").await?;

        // Insert some rows for row count
        adapter.execute_query("INSERT INTO test_metadata_table (name) VALUES ('test1'), ('test2'), ('test3')").await?;

        // Get table metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter.get_table_metadata("test_metadata_table", None).await?;

        assert_eq!(table_meta.name, "test_metadata_table");
        assert_eq!(table_meta.schema, Some("public".to_string()));
        assert!(table_meta.size_bytes.is_some());
        assert!(table_meta.row_count.is_some());
        debug!("Table size: {:?} bytes", table_meta.size_bytes);
        debug!("Row count: {:?}", table_meta.row_count);

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_metadata_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_indexes() -> Result<()> {
        info!("Starting test: test_postgres_get_indexes");

        let mut adapter = setup().await?;

        // Create test table with indexes
        info!("Creating test table with indexes");
        adapter.execute_query("
            CREATE TABLE test_indexes_table (
                id SERIAL PRIMARY KEY,
                email VARCHAR(100) UNIQUE,
                name TEXT,
                status TEXT
            )
        ").await?;

        adapter.execute_query("CREATE INDEX idx_name ON test_indexes_table(name)").await?;
        adapter.execute_query("CREATE INDEX idx_status ON test_indexes_table(status)").await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter.get_indexes("test_indexes_table", None).await?;

        assert!(indexes.len() >= 3); // PRIMARY KEY, UNIQUE, and our indexes

        // Find specific indexes
        let primary_idx = indexes.iter().find(|i| i.is_primary);
        assert!(primary_idx.is_some());

        let unique_idx = indexes.iter().find(|i| i.is_unique && !i.is_primary);
        assert!(unique_idx.is_some());

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!("Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary);
        }

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_indexes_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_indexes");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_postgres_get_foreign_keys");

        let mut adapter = setup().await?;

        // Create parent and child tables with FK
        info!("Creating tables with foreign key");
        adapter.execute_query("
            CREATE TABLE test_fk_parent (
                id SERIAL PRIMARY KEY,
                name TEXT
            )
        ").await?;

        adapter.execute_query("
            CREATE TABLE test_fk_child (
                id SERIAL PRIMARY KEY,
                parent_id INTEGER REFERENCES test_fk_parent(id) ON DELETE CASCADE,
                data TEXT
            )
        ").await?;

        // Get foreign keys
        info!("Testing get_foreign_keys");
        let fks = adapter.get_foreign_keys("test_fk_child", None).await?;

        assert_eq!(fks.len(), 1);
        let fk = &fks[0];
        assert_eq!(fk.table_name, "test_fk_child");
        assert_eq!(fk.referenced_table, "test_fk_parent");
        assert!(fk.columns.contains(&"parent_id".to_string()));
        assert!(fk.referenced_columns.contains(&"id".to_string()));
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
        debug!("Foreign key: {} -> {}", fk.name, fk.referenced_table);

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE test_fk_child").await?;
        adapter.execute_query("DROP TABLE test_fk_parent").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_get_views() -> Result<()> {
        info!("Starting test: test_postgres_get_views");

        let mut adapter = setup().await?;

        // Create test table and view
        info!("Creating test table and view");
        adapter.execute_query("
            CREATE TABLE test_view_source (
                id SERIAL PRIMARY KEY,
                value INTEGER
            )
        ").await?;

        adapter.execute_query("
            CREATE VIEW test_my_view AS
            SELECT id, value * 2 AS doubled
            FROM test_view_source
        ").await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(None).await?;

        let test_view = views.iter().find(|v| v.name == "test_my_view");
        assert!(test_view.is_some());
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter.get_view_definition("test_my_view", None).await?;
        assert!(definition.is_some());
        assert!(definition.unwrap().contains("test_view_source"));
        debug!("View definition retrieved");

        // Cleanup
        info!("Cleaning up test view and table");
        adapter.execute_query("DROP VIEW test_my_view").await?;
        adapter.execute_query("DROP TABLE test_view_source").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_get_views");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_postgres_list_stored_procedures");

        let mut adapter = setup().await?;

        // Create test function
        info!("Creating test function");
        adapter.execute_query("
            CREATE FUNCTION test_add_numbers(a INTEGER, b INTEGER)
            RETURNS INTEGER AS $$
            BEGIN
                RETURN a + b;
            END;
            $$ LANGUAGE plpgsql
        ").await?;

        // List procedures
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(None).await?;

        let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
        assert!(test_proc.is_some());
        let proc = test_proc.unwrap();
        assert_eq!(proc.language, Some("plpgsql".to_string()));
        debug!("Found function: {} (language: {:?})", proc.name, proc.language);

        // Cleanup
        info!("Cleaning up test function");
        adapter.execute_query("DROP FUNCTION test_add_numbers").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_postgres_list_stored_procedures");
        Ok(())
    }

    /// Cleanup test - drops the test database after all tests complete
    /// Named with zzz prefix to run last (tests run alphabetically)
    #[tokio::test]
    #[ignore]
    async fn test_zzz_cleanup_postgres_database() -> Result<()> {
        info!("Cleaning up PostgreSQL test database: {}", TEST_DB_NAME);

        let mut adapter = PostgresAdapter::new();
        let postgres_config = get_postgres_config("postgres");

        adapter.connect(&postgres_config, Some(TEST_PASSWORD)).await?;

        // Drop test database
        adapter.execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME)).await?;

        adapter.disconnect().await?;

        info!("PostgreSQL test database cleanup complete");
        Ok(())
    }
}

#[cfg(feature = "mysql")]
mod mysql_tests {
    use super::*;
    use rusty_data::adapters::mysql::MySqlAdapter;
    use std::sync::Once;

    const TEST_PASSWORD: &str = "test_password";
    const TEST_DB_NAME: &str = "rusty_test_mysql";

    static INIT: Once = Once::new();

    fn get_mysql_config(database: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mysql".to_string(),
            name: "Test MySQL".to_string(),
            db_type: DatabaseType::MySQL,
            host: Some("localhost".to_string()),
            port: Some(3306),
            database: database.to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    /// Ensure test database exists (idempotent, called once per suite)
    async fn ensure_test_database() -> Result<()> {
        let mut adapter = MySqlAdapter::new();
        let mysql_config = get_mysql_config("mysql");

        adapter.connect(&mysql_config, Some(TEST_PASSWORD)).await?;

        info!("Creating MySQL test database: {}", TEST_DB_NAME);

        // Create database if it doesn't exist
        let _ = adapter.execute_query(&format!("CREATE DATABASE IF NOT EXISTS {}", TEST_DB_NAME)).await?;

        adapter.disconnect().await?;
        debug!("MySQL test database ready: {}", TEST_DB_NAME);
        Ok(())
    }

    /// Setup function called before each test
    async fn setup() -> Result<MySqlAdapter> {
        // Ensure database exists (happens once)
        INIT.call_once(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(ensure_test_database())
                .unwrap();
        });

        let mut adapter = MySqlAdapter::new();
        let config = get_mysql_config(TEST_DB_NAME);

        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        Ok(adapter)
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_connect_disconnect() -> Result<()> {
        info!("Starting test: test_mysql_connect_disconnect");

        let mut adapter = setup().await?;
        assert!(adapter.is_connected());
        debug!("Connected to test database");

        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        info!("Test completed: test_mysql_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_execute_query() -> Result<()> {
        info!("Starting test: test_mysql_execute_query");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_query_users (
                id INT AUTO_INCREMENT PRIMARY KEY,
                username VARCHAR(50),
                email VARCHAR(100),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT true
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT INTO test_query_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;
        debug!("Test data inserted");

        // Query test data
        info!("Testing execute_query");
        let result = adapter.execute_query("SELECT * FROM test_query_users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);
        debug!("Query results validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_query_users").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_execute_query");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_list_tables() -> Result<()> {
        info!("Starting test: test_mysql_list_tables");

        let mut adapter = setup().await?;

        // Create test tables
        info!("Creating test tables");
        adapter.execute_query("CREATE TABLE test_list_users (id INT AUTO_INCREMENT PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_products (id INT AUTO_INCREMENT PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_orders (id INT AUTO_INCREMENT PRIMARY KEY)").await?;
        debug!("Test tables created");

        // List tables
        info!("Testing list_tables");
        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"test_list_users".to_string()));
        assert!(tables.contains(&"test_list_products".to_string()));
        assert!(tables.contains(&"test_list_orders".to_string()));
        debug!("Found all expected tables");

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE test_list_users").await?;
        adapter.execute_query("DROP TABLE test_list_products").await?;
        adapter.execute_query("DROP TABLE test_list_orders").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_list_tables");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_describe_table() -> Result<()> {
        info!("Starting test: test_mysql_describe_table");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_describe_table (
                id INT AUTO_INCREMENT PRIMARY KEY,
                username VARCHAR(50) NOT NULL,
                email VARCHAR(100)
            )
        ").await?;
        debug!("Test table created");

        // Describe table
        info!("Testing describe_table");
        let table_info = adapter.describe_table("test_describe_table", None).await?;

        assert_eq!(table_info.name, "test_describe_table");
        assert_eq!(table_info.columns.len(), 3);

        let column_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"username".to_string()));
        assert!(column_names.contains(&"email".to_string()));
        debug!("Table structure validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_describe_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_describe_table");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_server_info() -> Result<()> {
        info!("Starting test: test_mysql_get_server_info");

        let mut adapter = setup().await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "MySQL");
        assert!(!server_info.version.is_empty());
        assert!(server_info.version.contains("8.") || server_info.version.contains("5."));
        debug!("Server version: {}", server_info.version);
        debug!("Server settings: {:?}", server_info.extra_info);

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_server_info");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_database_metadata() -> Result<()> {
        info!("Starting test: test_mysql_get_database_metadata");

        let mut adapter = setup().await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata(TEST_DB_NAME).await?;

        assert_eq!(db_meta.name, TEST_DB_NAME);
        assert!(db_meta.size_bytes.is_some());
        debug!("Database size: {:?} bytes", db_meta.size_bytes);
        debug!("Database encoding: {:?}", db_meta.encoding);

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_table_metadata() -> Result<()> {
        info!("Starting test: test_mysql_get_table_metadata");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_mysql_metadata_table (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(100) NOT NULL
            ) ENGINE=InnoDB
        ").await?;

        // Insert some rows for row count
        adapter.execute_query("INSERT INTO test_mysql_metadata_table (name) VALUES ('test1'), ('test2'), ('test3')").await?;

        // Get table metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter.get_table_metadata("test_mysql_metadata_table", None).await?;

        assert_eq!(table_meta.name, "test_mysql_metadata_table");
        assert!(table_meta.size_bytes.is_some());
        assert!(table_meta.row_count.is_some());
        assert_eq!(table_meta.table_type, Some("InnoDB".to_string()));
        debug!("Table size: {:?} bytes", table_meta.size_bytes);
        debug!("Row count: {:?}", table_meta.row_count);
        debug!("Table engine: {:?}", table_meta.table_type);

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_mysql_metadata_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_indexes() -> Result<()> {
        info!("Starting test: test_mysql_get_indexes");

        let mut adapter = setup().await?;

        // Create test table with indexes
        info!("Creating test table with indexes");
        adapter.execute_query("
            CREATE TABLE test_mysql_indexes_table (
                id INT AUTO_INCREMENT PRIMARY KEY,
                email VARCHAR(100) UNIQUE,
                name VARCHAR(100),
                status VARCHAR(50)
            )
        ").await?;

        adapter.execute_query("CREATE INDEX idx_name ON test_mysql_indexes_table(name)").await?;
        adapter.execute_query("CREATE INDEX idx_status ON test_mysql_indexes_table(status)").await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter.get_indexes("test_mysql_indexes_table", None).await?;

        assert!(indexes.len() >= 3); // PRIMARY KEY, UNIQUE, and our indexes

        // Find specific indexes
        let primary_idx = indexes.iter().find(|i| i.is_primary);
        assert!(primary_idx.is_some());
        assert_eq!(primary_idx.unwrap().name, "PRIMARY");

        let unique_idx = indexes.iter().find(|i| i.is_unique && !i.is_primary);
        assert!(unique_idx.is_some());

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!("Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary);
        }

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_mysql_indexes_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_indexes");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_mysql_get_foreign_keys");

        let mut adapter = setup().await?;

        // Create parent and child tables with FK
        info!("Creating tables with foreign key");
        adapter.execute_query("
            CREATE TABLE test_mysql_fk_parent (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(100)
            )
        ").await?;

        adapter.execute_query("
            CREATE TABLE test_mysql_fk_child (
                id INT AUTO_INCREMENT PRIMARY KEY,
                parent_id INT,
                data VARCHAR(100),
                FOREIGN KEY (parent_id) REFERENCES test_mysql_fk_parent(id) ON DELETE CASCADE
            )
        ").await?;

        // Get foreign keys
        info!("Testing get_foreign_keys");
        let fks = adapter.get_foreign_keys("test_mysql_fk_child", None).await?;

        assert_eq!(fks.len(), 1);
        let fk = &fks[0];
        assert_eq!(fk.table_name, "test_mysql_fk_child");
        assert_eq!(fk.referenced_table, "test_mysql_fk_parent");
        assert!(fk.columns.contains(&"parent_id".to_string()));
        assert!(fk.referenced_columns.contains(&"id".to_string()));
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
        debug!("Foreign key: {} -> {}", fk.name, fk.referenced_table);

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE test_mysql_fk_child").await?;
        adapter.execute_query("DROP TABLE test_mysql_fk_parent").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_get_views() -> Result<()> {
        info!("Starting test: test_mysql_get_views");

        let mut adapter = setup().await?;

        // Create test table and view
        info!("Creating test table and view");
        adapter.execute_query("
            CREATE TABLE test_mysql_view_source (
                id INT AUTO_INCREMENT PRIMARY KEY,
                value INT
            )
        ").await?;

        adapter.execute_query("
            CREATE VIEW test_mysql_my_view AS
            SELECT id, value * 2 AS doubled
            FROM test_mysql_view_source
        ").await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(None).await?;

        let test_view = views.iter().find(|v| v.name == "test_mysql_my_view");
        assert!(test_view.is_some());
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter.get_view_definition("test_mysql_my_view", None).await?;
        assert!(definition.is_some());
        assert!(definition.unwrap().to_lowercase().contains("test_mysql_view_source"));
        debug!("View definition retrieved");

        // Cleanup
        info!("Cleaning up test view and table");
        adapter.execute_query("DROP VIEW test_mysql_my_view").await?;
        adapter.execute_query("DROP TABLE test_mysql_view_source").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_get_views");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mysql_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_mysql_list_stored_procedures");

        let mut adapter = setup().await?;

        // Create test stored procedure
        info!("Creating test stored procedure");
        adapter.execute_query("
            CREATE PROCEDURE test_add_numbers(IN a INT, IN b INT, OUT result INT)
            BEGIN
                SET result = a + b;
            END
        ").await?;

        // List procedures
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(None).await?;

        let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
        assert!(test_proc.is_some());
        let proc = test_proc.unwrap();
        assert_eq!(proc.language, Some("SQL".to_string()));
        debug!("Found procedure: {} (language: {:?})", proc.name, proc.language);

        // Cleanup
        info!("Cleaning up test procedure");
        adapter.execute_query("DROP PROCEDURE test_add_numbers").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mysql_list_stored_procedures");
        Ok(())
    }

    /// Cleanup test - drops the test database after all tests complete
    /// Named with zzz prefix to run last (tests run alphabetically)
    #[tokio::test]
    #[ignore]
    async fn test_zzz_cleanup_mysql_database() -> Result<()> {
        info!("Cleaning up MySQL test database: {}", TEST_DB_NAME);

        let mut adapter = MySqlAdapter::new();
        let mysql_config = get_mysql_config("mysql");

        adapter.connect(&mysql_config, Some(TEST_PASSWORD)).await?;

        // Drop test database
        adapter.execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME)).await?;

        adapter.disconnect().await?;

        info!("MySQL test database cleanup complete");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use super::*;
    use rusty_data::adapters::sqlite::SqliteAdapter;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate unique database filename for each test
    fn unique_db_path() -> String {
        let id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("./test-data/rusty_test_sqlite_{}.db", id)
    }

    fn get_sqlite_config(db_path: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-sqlite".to_string(),
            name: "Test SQLite".to_string(),
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            use_ssl: false,
            parameters: Default::default(),
        }
    }

    /// Ensure test-data directory exists
    fn ensure_test_dir() {
        let _ = fs::create_dir_all("./test-data");
    }

    /// Cleanup SQLite database file
    fn cleanup_db_file(db_path: &str) {
        let _ = fs::remove_file(db_path);
        debug!("Cleaned up database file: {}", db_path);
    }

    #[tokio::test]
    #[ignore]
    async fn test_sqlite_connect_disconnect() -> Result<()> {
        info!("Starting test: test_sqlite_connect_disconnect");
        ensure_test_dir();

        let db_path = unique_db_path();
        info!("Using database file: {}", db_path);

        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        // Connect
        adapter.connect(&config, None).await?;
        assert!(adapter.is_connected());
        debug!("Connected to SQLite database");

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        // Cleanup
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_sqlite_execute_query() -> Result<()> {
        info!("Starting test: test_sqlite_execute_query");
        ensure_test_dir();

        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                is_active INTEGER DEFAULT 1
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT INTO test_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;
        debug!("Test data inserted");

        // Query test data
        info!("Testing execute_query");
        let result = adapter.execute_query("SELECT * FROM test_users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);
        debug!("Query results validated");

        adapter.disconnect().await?;

        // Cleanup
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_execute_query");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_list_tables() -> Result<()> {
        info!("Starting test: test_sqlite_list_tables");
        ensure_test_dir();

        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Create test tables
        info!("Creating test tables");
        adapter.execute_query("CREATE TABLE test_users (id INTEGER PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_products (id INTEGER PRIMARY KEY)").await?;
        debug!("Test tables created");

        // List tables
        info!("Testing list_tables");
        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"test_users".to_string()));
        assert!(tables.contains(&"test_products".to_string()));
        debug!("Found all expected tables");

        adapter.disconnect().await?;

        // Cleanup
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_list_tables");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_server_info() -> Result<()> {
        info!("Starting test: test_sqlite_get_server_info");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "SQLite");
        assert!(!server_info.version.is_empty());
        assert!(server_info.version.starts_with("3.")); // SQLite version 3.x
        debug!("Server version: {}", server_info.version);
        debug!("Compile options: {:?}", server_info.extra_info);

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_server_info");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_database_metadata() -> Result<()> {
        info!("Starting test: test_sqlite_get_database_metadata");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata(&db_path).await?;

        assert_eq!(db_meta.name, db_path);
        assert!(db_meta.size_bytes.is_some());
        assert!(db_meta.size_bytes.unwrap() > 0);
        assert!(db_meta.encoding.is_some());
        debug!("Database size: {:?} bytes", db_meta.size_bytes);
        debug!("Database encoding: {:?}", db_meta.encoding);
        debug!("Extra info: {:?}", db_meta.extra_info);

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_table_metadata() -> Result<()> {
        info!("Starting test: test_sqlite_get_table_metadata");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Create test table
        info!("Creating test table");
        adapter
            .execute_query(
                "CREATE TABLE test_sqlite_metadata_table (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL
            )",
            )
            .await?;

        // Insert some rows for row count
        adapter
            .execute_query("INSERT INTO test_sqlite_metadata_table (name) VALUES ('test1'), ('test2'), ('test3')")
            .await?;

        // Get table metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter
            .get_table_metadata("test_sqlite_metadata_table", None)
            .await?;

        assert_eq!(table_meta.name, "test_sqlite_metadata_table");
        assert!(table_meta.row_count.is_some());
        assert_eq!(table_meta.row_count.unwrap(), 3);
        debug!("Row count: {:?}", table_meta.row_count);

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_indexes() -> Result<()> {
        info!("Starting test: test_sqlite_get_indexes");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Create test table with indexes
        info!("Creating test table with indexes");
        adapter
            .execute_query(
                "CREATE TABLE test_sqlite_indexes_table (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE,
                name TEXT,
                status TEXT
            )",
            )
            .await?;

        adapter
            .execute_query("CREATE INDEX idx_name ON test_sqlite_indexes_table(name)")
            .await?;
        adapter
            .execute_query("CREATE INDEX idx_status ON test_sqlite_indexes_table(status)")
            .await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter
            .get_indexes("test_sqlite_indexes_table", None)
            .await?;

        assert!(indexes.len() >= 2); // Our indexes (UNIQUE constraint may also create an index)

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!(
                "Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary
            );
        }

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_indexes");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_sqlite_get_foreign_keys");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Enable foreign keys (SQLite requires this)
        adapter.execute_query("PRAGMA foreign_keys = ON").await?;

        // Create parent and child tables with FK
        info!("Creating tables with foreign key");
        adapter
            .execute_query(
                "CREATE TABLE test_sqlite_fk_parent (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT
            )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE TABLE test_sqlite_fk_child (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id INTEGER,
                data TEXT,
                FOREIGN KEY (parent_id) REFERENCES test_sqlite_fk_parent(id) ON DELETE CASCADE
            )",
            )
            .await?;

        // Get foreign keys
        info!("Testing get_foreign_keys");
        let fks = adapter
            .get_foreign_keys("test_sqlite_fk_child", None)
            .await?;

        assert_eq!(fks.len(), 1);
        let fk = &fks[0];
        assert_eq!(fk.table_name, "test_sqlite_fk_child");
        assert_eq!(fk.referenced_table, "test_sqlite_fk_parent");
        assert!(fk.columns.contains(&"parent_id".to_string()));
        assert!(fk.referenced_columns.contains(&"id".to_string()));
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
        debug!("Foreign key: {} -> {}", fk.name, fk.referenced_table);

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_get_views() -> Result<()> {
        info!("Starting test: test_sqlite_get_views");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // Create test table and view
        info!("Creating test table and view");
        adapter
            .execute_query(
                "CREATE TABLE test_sqlite_view_source (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value INTEGER
            )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE VIEW test_sqlite_my_view AS
            SELECT id, value * 2 AS doubled
            FROM test_sqlite_view_source",
            )
            .await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(None).await?;

        let test_view = views.iter().find(|v| v.name == "test_sqlite_my_view");
        assert!(test_view.is_some());
        let view = test_view.unwrap();
        assert!(view.definition.is_some());
        assert!(view
            .definition
            .as_ref()
            .unwrap()
            .contains("test_sqlite_view_source"));
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter
            .get_view_definition("test_sqlite_my_view", None)
            .await?;
        assert!(definition.is_some());
        assert!(definition.unwrap().contains("test_sqlite_view_source"));
        debug!("View definition retrieved");

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_get_views");
        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_sqlite_list_stored_procedures");

        ensure_test_dir();
        let db_path = unique_db_path();
        let mut adapter = SqliteAdapter::new();
        let config = get_sqlite_config(&db_path);

        adapter.connect(&config, None).await?;

        // List procedures (should be empty for SQLite)
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(None).await?;

        assert_eq!(procedures.len(), 0); // SQLite doesn't support stored procedures
        debug!("SQLite correctly reports no stored procedures");

        adapter.disconnect().await?;
        cleanup_db_file(&db_path);

        info!("Test completed: test_sqlite_list_stored_procedures");
        Ok(())
    }
}

// ============================================================================
// MongoDB Integration Tests
// ============================================================================

#[cfg(feature = "mongodb")]
mod mongodb_tests {
    use super::*;
    use rusty_data::adapters::mongodb::MongoDbAdapter;
    use std::sync::Once;
    use tracing::{debug, info};

    // MongoDB test database - shared across all tests
    const TEST_DB_NAME: &str = "rusty_test_mongodb";
    const TEST_USER: &str = "test_user";
    const TEST_PASSWORD: &str = "test_password";

    // One-time database creation
    static INIT: Once = Once::new();

    fn get_mongodb_config(database: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mongodb".to_string(),
            name: "Test MongoDB".to_string(),
            db_type: DatabaseType::MongoDB,
            host: Some("localhost".to_string()),
            port: Some(27017),
            database: database.to_string(),
            username: Some(TEST_USER.to_string()),
            use_ssl: false,
            parameters: std::collections::HashMap::new(),
        }
    }

    async fn ensure_test_database() -> Result<()> {
        let mut adapter = MongoDbAdapter::new();
        let admin_config = get_mongodb_config("admin");

        adapter.connect(&admin_config, Some(TEST_PASSWORD)).await?;
        info!("Creating MongoDB test database: {}", TEST_DB_NAME);

        // MongoDB creates databases implicitly when first used, but we'll ensure it exists
        // by executing a command against it
        let use_db_query = format!("use {}", TEST_DB_NAME);
        let _ = adapter.execute_query(&use_db_query).await;

        adapter.disconnect().await?;
        debug!("MongoDB test database ready: {}", TEST_DB_NAME);
        Ok(())
    }

    async fn setup() -> Result<MongoDbAdapter> {
        INIT.call_once(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(ensure_test_database())
                .unwrap();
        });

        let mut adapter = MongoDbAdapter::new();
        let config = get_mongodb_config(TEST_DB_NAME);
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        Ok(adapter)
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_connect_disconnect() -> Result<()> {
        info!("Starting test: test_mongodb_connect_disconnect");

        let mut adapter = MongoDbAdapter::new();
        let config = get_mongodb_config(TEST_DB_NAME);

        // Connect
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        assert!(adapter.is_connected());
        debug!("Connected to MongoDB database");

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        info!("Test completed: test_mongodb_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_execute_query_insert_find() -> Result<()> {
        info!("Starting test: test_mongodb_execute_query_insert_find");

        let mut adapter = setup().await?;

        // Create unique collection name for this test
        let collection_name = "test_insert_users";

        // Insert documents
        info!("Inserting test documents into collection: {}", collection_name);
        let insert_query = format!(
            r#"db.{}.insertMany([
                {{ username: "alice", email: "alice@example.com", age: 30 }},
                {{ username: "bob", email: "bob@example.com", age: 25 }},
                {{ username: "charlie", email: "charlie@example.com", age: 35 }}
            ])"#,
            collection_name
        );
        adapter.execute_query(&insert_query).await?;
        debug!("Test documents inserted");

        // Find documents
        info!("Testing execute_query with find");
        let find_query = format!(r#"db.{}.find({{}})"#, collection_name);
        let result = adapter.execute_query(&find_query).await?;

        assert_eq!(result.rows.len(), 3);
        debug!("Query results validated: found {} documents", result.rows.len());

        // Cleanup - drop collection
        info!("Cleaning up test collection: {}", collection_name);
        let drop_query = format!("db.{}.drop()", collection_name);
        adapter.execute_query(&drop_query).await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_execute_query_insert_find");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_list_databases() -> Result<()> {
        info!("Starting test: test_mongodb_list_databases");

        let mut adapter = setup().await?;

        // List databases
        info!("Testing list_databases");
        let databases = adapter.list_databases().await?;

        // Should at least contain our test database and admin
        assert!(databases.contains(&TEST_DB_NAME.to_string()));
        debug!("Found test database in list: {}", TEST_DB_NAME);

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_list_databases");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_list_tables() -> Result<()> {
        info!("Starting test: test_mongodb_list_tables");

        let mut adapter = setup().await?;

        // Create test collections
        let collections = vec!["test_list_users", "test_list_products", "test_list_orders"];

        info!("Creating test collections");
        for collection in &collections {
            let create_query = format!(r#"db.{}.insertOne({{ _init: true }})"#, collection);
            adapter.execute_query(&create_query).await?;
        }
        debug!("Test collections created");

        // List collections (tables in MongoDB)
        info!("Testing list_tables (collections)");
        let tables = adapter.list_tables(None).await?;

        for collection in &collections {
            assert!(
                tables.contains(&collection.to_string()),
                "Expected to find collection: {}",
                collection
            );
        }
        debug!("Found all expected collections");

        // Cleanup - drop test collections
        info!("Cleaning up test collections");
        for collection in &collections {
            let drop_query = format!("db.{}.drop()", collection);
            adapter.execute_query(&drop_query).await?;
        }

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_list_tables");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_describe_table() -> Result<()> {
        info!("Starting test: test_mongodb_describe_table");

        let mut adapter = setup().await?;

        let collection_name = "test_describe_collection";

        // Create collection with sample documents
        info!("Creating test collection with documents");
        let insert_query = format!(
            r#"db.{}.insertMany([
                {{ username: "alice", email: "alice@example.com", age: 30, is_active: true }},
                {{ username: "bob", email: "bob@example.com", age: 25, is_active: false }}
            ])"#,
            collection_name
        );
        adapter.execute_query(&insert_query).await?;
        debug!("Test collection created with sample documents");

        // Describe collection (in MongoDB this typically means getting schema info)
        info!("Testing describe_table for collection");
        let table_info = adapter.describe_table(collection_name, None).await?;

        assert_eq!(table_info.name, collection_name);
        // MongoDB is schemaless, so column info might be based on sampling
        // Just verify we got some structure back
        assert!(!table_info.columns.is_empty());
        debug!("Collection schema info retrieved");

        // Cleanup - drop collection
        info!("Cleaning up test collection");
        let drop_query = format!("db.{}.drop()", collection_name);
        adapter.execute_query(&drop_query).await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_describe_table");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_crud_operations() -> Result<()> {
        info!("Starting test: test_mongodb_crud_operations");

        let mut adapter = setup().await?;

        let collection_name = "test_crud_users";

        // CREATE - Insert documents
        info!("Testing INSERT (insertMany)");
        let insert_query = format!(
            r#"db.{}.insertMany([
                {{ username: "alice", email: "alice@example.com", status: "active" }},
                {{ username: "bob", email: "bob@example.com", status: "active" }}
            ])"#,
            collection_name
        );
        adapter.execute_query(&insert_query).await?;
        debug!("Documents inserted");

        // READ - Find documents
        info!("Testing READ (find)");
        let find_query = format!(r#"db.{}.find({{}})"#, collection_name);
        let result = adapter.execute_query(&find_query).await?;
        assert_eq!(result.rows.len(), 2);
        debug!("Found {} documents", result.rows.len());

        // UPDATE - Update a document
        info!("Testing UPDATE (updateOne)");
        let update_query = format!(
            r#"db.{}.updateOne(
                {{ username: "alice" }},
                {{ $set: {{ status: "inactive" }} }}
            )"#,
            collection_name
        );
        adapter.execute_query(&update_query).await?;
        debug!("Document updated");

        // Verify update
        let verify_query = format!(r#"db.{}.find({{ username: "alice" }})"#, collection_name);
        let verify_result = adapter.execute_query(&verify_query).await?;
        assert_eq!(verify_result.rows.len(), 1);
        debug!("Update verified");

        // DELETE - Delete a document
        info!("Testing DELETE (deleteOne)");
        let delete_query = format!(r#"db.{}.deleteOne({{ username: "bob" }})"#, collection_name);
        adapter.execute_query(&delete_query).await?;
        debug!("Document deleted");

        // Verify deletion
        let final_query = format!(r#"db.{}.find({{}})"#, collection_name);
        let final_result = adapter.execute_query(&final_query).await?;
        assert_eq!(final_result.rows.len(), 1);
        debug!("Deletion verified: {} document remaining", final_result.rows.len());

        // Cleanup - drop collection
        info!("Cleaning up test collection");
        let drop_query = format!("db.{}.drop()", collection_name);
        adapter.execute_query(&drop_query).await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_crud_operations");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_server_info() -> Result<()> {
        info!("Starting test: test_mongodb_get_server_info");

        let mut adapter = setup().await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "MongoDB");
        assert!(!server_info.version.is_empty());
        assert!(server_info.version.starts_with("7.") || server_info.version.starts_with("6.") || server_info.version.starts_with("5."));
        debug!("Server version: {}", server_info.version);
        debug!("Server info: {:?}", server_info.extra_info);

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_server_info");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_database_metadata() -> Result<()> {
        info!("Starting test: test_mongodb_get_database_metadata");

        let mut adapter = setup().await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata(TEST_DB_NAME).await?;

        assert_eq!(db_meta.name, TEST_DB_NAME);
        assert!(db_meta.size_bytes.is_some());
        assert_eq!(db_meta.encoding, Some("UTF-8".to_string()));
        debug!("Database size: {:?} bytes", db_meta.size_bytes);
        debug!("Database metadata: {:?}", db_meta.extra_info);

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_table_metadata() -> Result<()> {
        info!("Starting test: test_mongodb_get_table_metadata");

        let mut adapter = setup().await?;

        // Create test collection
        let collection_name = "test_mongodb_metadata_coll";
        info!("Creating test collection");
        adapter
            .execute_query(&format!(
                "db.{}.insertMany([{{name: 'test1'}}, {{name: 'test2'}}, {{name: 'test3'}}])",
                collection_name
            ))
            .await?;

        // Get collection metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter.get_table_metadata(collection_name, None).await?;

        assert_eq!(table_meta.name, collection_name);
        assert!(table_meta.row_count.is_some());
        assert!(table_meta.row_count.unwrap() >= 3);
        assert_eq!(table_meta.table_type, Some("collection".to_string()));
        debug!("Collection size: {:?} bytes", table_meta.size_bytes);
        debug!("Document count: {:?}", table_meta.row_count);

        // Cleanup
        info!("Cleaning up test collection");
        adapter
            .execute_query(&format!("db.{}.drop()", collection_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_indexes() -> Result<()> {
        info!("Starting test: test_mongodb_get_indexes");

        let mut adapter = setup().await?;

        // Create test collection with indexes
        let collection_name = "test_mongodb_indexes_coll";
        info!("Creating test collection with indexes");
        adapter
            .execute_query(&format!("db.{}.insertOne({{name: 'test', email: 'test@example.com'}})", collection_name))
            .await?;

        // Create indexes
        adapter
            .execute_query(&format!("db.{}.createIndex({{name: 1}})", collection_name))
            .await?;
        adapter
            .execute_query(&format!("db.{}.createIndex({{email: 1}}, {{unique: true}})", collection_name))
            .await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter.get_indexes(collection_name, None).await?;

        assert!(indexes.len() >= 3); // _id (default), name, email indexes

        // Find the _id index (MongoDB's default primary key)
        let id_idx = indexes.iter().find(|i| i.name == "_id_");
        assert!(id_idx.is_some());
        assert!(id_idx.unwrap().is_primary);

        // Find the unique email index
        let email_idx = indexes.iter().find(|i| i.columns.contains(&"email".to_string()));
        assert!(email_idx.is_some());
        assert!(email_idx.unwrap().is_unique);

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!(
                "Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary
            );
        }

        // Cleanup
        info!("Cleaning up test collection");
        adapter
            .execute_query(&format!("db.{}.drop()", collection_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_indexes");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_mongodb_get_foreign_keys");

        let mut adapter = setup().await?;

        // Get foreign keys (should be empty for MongoDB)
        info!("Testing get_foreign_keys");
        let fks = adapter.get_foreign_keys("any_collection", None).await?;

        assert_eq!(fks.len(), 0); // MongoDB doesn't support foreign keys
        debug!("MongoDB correctly reports no foreign keys");

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_get_views() -> Result<()> {
        info!("Starting test: test_mongodb_get_views");

        let mut adapter = setup().await?;

        // Create test collection and view
        let collection_name = "test_mongodb_view_source";
        let view_name = "test_mongodb_my_view";
        info!("Creating test collection and view");
        adapter
            .execute_query(&format!(
                "db.{}.insertOne({{value: 10}})",
                collection_name
            ))
            .await?;

        adapter
            .execute_query(&format!(
                "db.createView('{}', '{}', [{{$project: {{doubled: {{$multiply: ['$value', 2]}}}}}}])",
                view_name, collection_name
            ))
            .await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(None).await?;

        let test_view = views.iter().find(|v| v.name == view_name);
        assert!(test_view.is_some());
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter.get_view_definition(view_name, None).await?;
        assert!(definition.is_some());
        debug!("View definition: {:?}", definition);

        // Cleanup
        info!("Cleaning up test view and collection");
        adapter
            .execute_query(&format!("db.{}.drop()", view_name))
            .await?;
        adapter
            .execute_query(&format!("db.{}.drop()", collection_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_get_views");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mongodb_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_mongodb_list_stored_procedures");

        let mut adapter = setup().await?;

        // List procedures (should be empty for MongoDB)
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(None).await?;

        assert_eq!(procedures.len(), 0); // MongoDB doesn't support stored procedures
        debug!("MongoDB correctly reports no stored procedures");

        adapter.disconnect().await?;
        info!("Test completed: test_mongodb_list_stored_procedures");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_zzz_cleanup_mongodb_database() -> Result<()> {
        info!("Cleaning up MongoDB test database: {}", TEST_DB_NAME);

        let mut adapter = MongoDbAdapter::new();
        let admin_config = get_mongodb_config("admin");

        adapter.connect(&admin_config, Some(TEST_PASSWORD)).await?;

        // Drop the test database
        let drop_query = format!("use {}; db.dropDatabase()", TEST_DB_NAME);
        let _ = adapter.execute_query(&drop_query).await;

        adapter.disconnect().await?;
        info!("MongoDB test database cleanup complete");
        Ok(())
    }
}

// ============================================================================
// Microsoft SQL Server Integration Tests
// ============================================================================

#[cfg(feature = "mssql")]
mod mssql_tests {
    use super::*;
    use rusty_data::adapters::mssql::MssqlAdapter;
    use std::sync::Once;
    use tracing::{debug, info};

    // MSSQL test database - shared across all tests
    const TEST_DB_NAME: &str = "rusty_test_mssql";
    const TEST_USER: &str = "sa";
    const TEST_PASSWORD: &str = "Test_Password123!";

    // One-time database creation
    static INIT: Once = Once::new();

    fn get_mssql_config(database: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mssql".to_string(),
            name: "Test SQL Server".to_string(),
            db_type: DatabaseType::SQLServer,
            host: Some("localhost".to_string()),
            port: Some(1433),
            database: database.to_string(),
            username: Some(TEST_USER.to_string()),
            use_ssl: false,
            parameters: std::collections::HashMap::new(),
        }
    }

    async fn ensure_test_database() -> Result<()> {
        let mut adapter = MssqlAdapter::new();
        let master_config = get_mssql_config("master");

        adapter.connect(&master_config, Some(TEST_PASSWORD)).await?;
        info!("Creating MSSQL test database: {}", TEST_DB_NAME);

        // Drop database if exists, then create
        let _ = adapter.execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME)).await;
        adapter.execute_query(&format!("CREATE DATABASE {}", TEST_DB_NAME)).await?;

        adapter.disconnect().await?;
        debug!("MSSQL test database ready: {}", TEST_DB_NAME);
        Ok(())
    }

    async fn setup() -> Result<MssqlAdapter> {
        INIT.call_once(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(ensure_test_database())
                .unwrap();
        });

        let mut adapter = MssqlAdapter::new();
        let config = get_mssql_config(TEST_DB_NAME);
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        Ok(adapter)
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_connect_disconnect() -> Result<()> {
        info!("Starting test: test_mssql_connect_disconnect");

        let mut adapter = MssqlAdapter::new();
        let config = get_mssql_config(TEST_DB_NAME);

        // Connect
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        assert!(adapter.is_connected());
        debug!("Connected to MSSQL database");

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        info!("Test completed: test_mssql_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_execute_query() -> Result<()> {
        info!("Starting test: test_mssql_execute_query");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_query_users (
                id INT IDENTITY(1,1) PRIMARY KEY,
                username NVARCHAR(50),
                email NVARCHAR(100),
                created_at DATETIME DEFAULT GETDATE(),
                is_active BIT DEFAULT 1
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT INTO test_query_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com'),
            ('charlie', 'charlie@example.com')
        ").await?;
        debug!("Test data inserted");

        // Query test data
        info!("Testing execute_query");
        let result = adapter.execute_query("SELECT * FROM test_query_users ORDER BY id").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);
        debug!("Query results validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_query_users").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_execute_query");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_list_databases() -> Result<()> {
        info!("Starting test: test_mssql_list_databases");

        let mut adapter = setup().await?;

        // List databases
        info!("Testing list_databases");
        let databases = adapter.list_databases().await?;

        // Should contain our test database (system databases excluded by adapter)
        assert!(databases.contains(&TEST_DB_NAME.to_string()));
        debug!("Found test database in list: {}", TEST_DB_NAME);

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_list_databases");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_list_tables() -> Result<()> {
        info!("Starting test: test_mssql_list_tables");

        let mut adapter = setup().await?;

        // Create test tables
        info!("Creating test tables");
        adapter.execute_query("CREATE TABLE test_list_users (id INT IDENTITY(1,1) PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_products (id INT IDENTITY(1,1) PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE test_list_orders (id INT IDENTITY(1,1) PRIMARY KEY)").await?;
        debug!("Test tables created");

        // List tables
        info!("Testing list_tables");
        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"test_list_users".to_string()));
        assert!(tables.contains(&"test_list_products".to_string()));
        assert!(tables.contains(&"test_list_orders".to_string()));
        debug!("Found all expected tables");

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE test_list_users").await?;
        adapter.execute_query("DROP TABLE test_list_products").await?;
        adapter.execute_query("DROP TABLE test_list_orders").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_list_tables");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_describe_table() -> Result<()> {
        info!("Starting test: test_mssql_describe_table");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_describe_table (
                id INT IDENTITY(1,1) PRIMARY KEY,
                username NVARCHAR(50) NOT NULL,
                email NVARCHAR(100)
            )
        ").await?;
        debug!("Test table created");

        // Describe table
        info!("Testing describe_table");
        let table_info = adapter.describe_table("test_describe_table", None).await?;

        assert_eq!(table_info.name, "test_describe_table");
        assert_eq!(table_info.columns.len(), 3);

        // Verify column details
        let id_col = table_info.columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.is_primary_key);

        let username_col = table_info.columns.iter().find(|c| c.name == "username").unwrap();
        assert!(!username_col.nullable);

        debug!("Table schema validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_describe_table").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_describe_table");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_crud_operations() -> Result<()> {
        info!("Starting test: test_mssql_crud_operations");

        let mut adapter = setup().await?;

        // CREATE table and INSERT data
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE test_crud_users (
                id INT IDENTITY(1,1) PRIMARY KEY,
                username NVARCHAR(50) NOT NULL,
                email NVARCHAR(100),
                status NVARCHAR(20) DEFAULT 'active'
            )
        ").await?;

        info!("Testing INSERT");
        adapter.execute_query("
            INSERT INTO test_crud_users (username, email) VALUES
            ('alice', 'alice@example.com'),
            ('bob', 'bob@example.com')
        ").await?;
        debug!("Rows inserted");

        // READ
        info!("Testing SELECT");
        let result = adapter.execute_query("SELECT * FROM test_crud_users ORDER BY id").await?;
        assert_eq!(result.rows.len(), 2);
        debug!("Found {} rows", result.rows.len());

        // UPDATE
        info!("Testing UPDATE");
        adapter.execute_query("
            UPDATE test_crud_users
            SET status = 'inactive'
            WHERE username = 'alice'
        ").await?;
        debug!("Row updated");

        // Verify update
        let verify_result = adapter.execute_query("
            SELECT status FROM test_crud_users WHERE username = 'alice'
        ").await?;
        assert_eq!(verify_result.rows.len(), 1);
        debug!("Update verified");

        // DELETE
        info!("Testing DELETE");
        adapter.execute_query("DELETE FROM test_crud_users WHERE username = 'bob'").await?;
        debug!("Row deleted");

        // Verify deletion
        let final_result = adapter.execute_query("SELECT * FROM test_crud_users").await?;
        assert_eq!(final_result.rows.len(), 1);
        debug!("Deletion verified: {} row remaining", final_result.rows.len());

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE test_crud_users").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_crud_operations");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_server_info() -> Result<()> {
        info!("Starting test: test_mssql_get_server_info");

        let mut adapter = setup().await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "Microsoft SQL Server");
        assert!(!server_info.version.is_empty());
        debug!("Server version: {}", server_info.version);
        debug!("Server info: {:?}", server_info.extra_info);

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_server_info");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_database_metadata() -> Result<()> {
        info!("Starting test: test_mssql_get_database_metadata");

        let mut adapter = setup().await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata(TEST_DB_NAME).await?;

        assert_eq!(db_meta.name, TEST_DB_NAME);
        assert!(db_meta.size_bytes.is_some());
        debug!("Database size: {:?} bytes", db_meta.size_bytes);
        debug!("Recovery model: {:?}", db_meta.extra_info.get("recovery_model"));

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_table_metadata() -> Result<()> {
        info!("Starting test: test_mssql_get_table_metadata");

        let mut adapter = setup().await?;

        // Create test table
        let table_name = "test_mssql_metadata_table";
        info!("Creating test table");
        adapter
            .execute_query(&format!(
                "CREATE TABLE {} (id INT PRIMARY KEY, name NVARCHAR(100))",
                table_name
            ))
            .await?;

        adapter
            .execute_query(&format!(
                "INSERT INTO {} (id, name) VALUES (1, 'test1'), (2, 'test2'), (3, 'test3')",
                table_name
            ))
            .await?;

        // Get table metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter.get_table_metadata(table_name, Some("dbo")).await?;

        assert_eq!(table_meta.name, table_name);
        assert_eq!(table_meta.schema, Some("dbo".to_string()));
        assert!(table_meta.row_count.is_some());
        debug!("Table size: {:?} bytes", table_meta.size_bytes);
        debug!("Row count: {:?}", table_meta.row_count);

        // Cleanup
        adapter
            .execute_query(&format!("DROP TABLE {}", table_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_indexes() -> Result<()> {
        info!("Starting test: test_mssql_get_indexes");

        let mut adapter = setup().await?;

        // Create test table with indexes
        let table_name = "test_mssql_indexes_table";
        info!("Creating test table with indexes");
        adapter
            .execute_query(&format!(
                "CREATE TABLE {} (
                    id INT PRIMARY KEY,
                    email NVARCHAR(100) UNIQUE,
                    name NVARCHAR(100),
                    status NVARCHAR(50)
                )",
                table_name
            ))
            .await?;

        adapter
            .execute_query(&format!(
                "CREATE INDEX idx_name ON {}(name)",
                table_name
            ))
            .await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter.get_indexes(table_name, Some("dbo")).await?;

        assert!(indexes.len() >= 2); // PRIMARY KEY, UNIQUE

        let primary_idx = indexes.iter().find(|i| i.is_primary);
        assert!(primary_idx.is_some());

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!(
                "Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary
            );
        }

        // Cleanup
        adapter
            .execute_query(&format!("DROP TABLE {}", table_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_indexes");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_mssql_get_foreign_keys");

        let mut adapter = setup().await?;

        // Create parent and child tables with FK
        info!("Creating tables with foreign key");
        adapter
            .execute_query(
                "CREATE TABLE test_mssql_fk_parent (
                    id INT PRIMARY KEY,
                    name NVARCHAR(100)
                )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE TABLE test_mssql_fk_child (
                    id INT PRIMARY KEY,
                    parent_id INT,
                    data NVARCHAR(100),
                    FOREIGN KEY (parent_id) REFERENCES test_mssql_fk_parent(id) ON DELETE CASCADE
                )",
            )
            .await?;

        // Get foreign keys
        info!("Testing get_foreign_keys");
        let fks = adapter
            .get_foreign_keys("test_mssql_fk_child", Some("dbo"))
            .await?;

        assert_eq!(fks.len(), 1);
        let fk = &fks[0];
        assert_eq!(fk.table_name, "test_mssql_fk_child");
        assert_eq!(fk.referenced_table, "test_mssql_fk_parent");
        assert!(fk.columns.contains(&"parent_id".to_string()));
        debug!("Foreign key: {} -> {}", fk.name, fk.referenced_table);

        // Cleanup
        adapter
            .execute_query("DROP TABLE test_mssql_fk_child")
            .await?;
        adapter
            .execute_query("DROP TABLE test_mssql_fk_parent")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_get_views() -> Result<()> {
        info!("Starting test: test_mssql_get_views");

        let mut adapter = setup().await?;

        // Create test table and view
        info!("Creating test table and view");
        adapter
            .execute_query(
                "CREATE TABLE test_mssql_view_source (
                    id INT PRIMARY KEY,
                    value INT
                )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE VIEW test_mssql_my_view AS
                SELECT id, value * 2 AS doubled
                FROM test_mssql_view_source",
            )
            .await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(Some("dbo")).await?;

        let test_view = views.iter().find(|v| v.name == "test_mssql_my_view");
        assert!(test_view.is_some());
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter
            .get_view_definition("test_mssql_my_view", Some("dbo"))
            .await?;
        assert!(definition.is_some());
        assert!(definition.unwrap().contains("test_mssql_view_source"));
        debug!("View definition retrieved");

        // Cleanup
        adapter
            .execute_query("DROP VIEW test_mssql_my_view")
            .await?;
        adapter
            .execute_query("DROP TABLE test_mssql_view_source")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_get_views");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_mssql_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_mssql_list_stored_procedures");

        let mut adapter = setup().await?;

        // Create test stored procedure
        info!("Creating test stored procedure");
        adapter
            .execute_query(
                "CREATE PROCEDURE test_add_numbers
                    @a INT,
                    @b INT
                AS
                BEGIN
                    SELECT @a + @b AS result
                END",
            )
            .await?;

        // List procedures
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(Some("dbo")).await?;

        let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
        assert!(test_proc.is_some());
        let proc = test_proc.unwrap();
        assert_eq!(proc.language, Some("T-SQL".to_string()));
        debug!("Found procedure: {} (language: {:?})", proc.name, proc.language);

        // Cleanup
        adapter
            .execute_query("DROP PROCEDURE test_add_numbers")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_mssql_list_stored_procedures");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_zzz_cleanup_mssql_database() -> Result<()> {
        info!("Cleaning up MSSQL test database: {}", TEST_DB_NAME);

        let mut adapter = MssqlAdapter::new();
        let master_config = get_mssql_config("master");

        adapter.connect(&master_config, Some(TEST_PASSWORD)).await?;

        // Drop the test database
        adapter.execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME)).await?;

        adapter.disconnect().await?;
        info!("MSSQL test database cleanup complete");
        Ok(())
    }
}

// ============================================================================
// Oracle Integration Tests
// ============================================================================

#[cfg(feature = "oracle")]
mod oracle_tests {
    use super::*;
    use rusty_data::adapters::oracle::OracleAdapter;
    use std::sync::Once;
    use tracing::{debug, info};

    // Oracle test database - shared across all tests
    // For Oracle, we use a schema/user instead of creating a new database
    const TEST_DB_NAME: &str = "XE"; // Service name
    const TEST_USER: &str = "system";
    const TEST_PASSWORD: &str = "Test_Password123!";

    // One-time database setup
    static INIT: Once = Once::new();

    fn get_oracle_config(database: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: "test-oracle".to_string(),
            name: "Test Oracle".to_string(),
            db_type: DatabaseType::Oracle,
            host: Some("localhost".to_string()),
            port: Some(1521),
            database: database.to_string(),
            username: Some(TEST_USER.to_string()),
            use_ssl: false,
            parameters: std::collections::HashMap::new(),
        }
    }

    async fn ensure_test_database() -> Result<()> {
        // For Oracle, the XE database already exists
        // We just need to ensure we can connect
        let mut adapter = OracleAdapter::new();
        let config = get_oracle_config(TEST_DB_NAME);

        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        info!("Connected to Oracle database: {}", TEST_DB_NAME);

        adapter.disconnect().await?;
        debug!("Oracle test database ready: {}", TEST_DB_NAME);
        Ok(())
    }

    async fn setup() -> Result<OracleAdapter> {
        INIT.call_once(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(ensure_test_database())
                .unwrap();
        });

        let mut adapter = OracleAdapter::new();
        let config = get_oracle_config(TEST_DB_NAME);
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        Ok(adapter)
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_connect_disconnect() -> Result<()> {
        info!("Starting test: test_oracle_connect_disconnect");

        let mut adapter = OracleAdapter::new();
        let config = get_oracle_config(TEST_DB_NAME);

        // Connect
        adapter.connect(&config, Some(TEST_PASSWORD)).await?;
        assert!(adapter.is_connected());
        debug!("Connected to Oracle database");

        // Disconnect
        adapter.disconnect().await?;
        assert!(!adapter.is_connected());
        debug!("Successfully disconnected");

        info!("Test completed: test_oracle_connect_disconnect");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_execute_query() -> Result<()> {
        info!("Starting test: test_oracle_execute_query");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE TEST_QUERY_USERS (
                ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                USERNAME VARCHAR2(50),
                EMAIL VARCHAR2(100),
                CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                IS_ACTIVE NUMBER(1) DEFAULT 1
            )
        ").await?;

        // Insert test data
        adapter.execute_query("
            INSERT ALL
            INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('alice', 'alice@example.com')
            INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('bob', 'bob@example.com')
            INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('charlie', 'charlie@example.com')
            SELECT * FROM DUAL
        ").await?;
        debug!("Test data inserted");

        // Query test data
        info!("Testing execute_query");
        let result = adapter.execute_query("SELECT * FROM TEST_QUERY_USERS ORDER BY ID").await?;

        assert_eq!(result.columns.len(), 5);
        assert_eq!(result.rows.len(), 3);
        debug!("Query results validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE TEST_QUERY_USERS").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_execute_query");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_list_databases() -> Result<()> {
        info!("Starting test: test_oracle_list_databases");

        let mut adapter = setup().await?;

        // List databases (schemas/users in Oracle)
        info!("Testing list_databases");
        let databases = adapter.list_databases().await?;

        // Should contain system schemas
        assert!(databases.contains(&"SYSTEM".to_string()));
        debug!("Found SYSTEM schema in list");

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_list_databases");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_list_tables() -> Result<()> {
        info!("Starting test: test_oracle_list_tables");

        let mut adapter = setup().await?;

        // Create test tables
        info!("Creating test tables");
        adapter.execute_query("CREATE TABLE TEST_LIST_USERS (ID NUMBER PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE TEST_LIST_PRODUCTS (ID NUMBER PRIMARY KEY)").await?;
        adapter.execute_query("CREATE TABLE TEST_LIST_ORDERS (ID NUMBER PRIMARY KEY)").await?;
        debug!("Test tables created");

        // List tables
        info!("Testing list_tables");
        let tables = adapter.list_tables(None).await?;

        assert!(tables.contains(&"TEST_LIST_USERS".to_string()));
        assert!(tables.contains(&"TEST_LIST_PRODUCTS".to_string()));
        assert!(tables.contains(&"TEST_LIST_ORDERS".to_string()));
        debug!("Found all expected tables");

        // Cleanup
        info!("Cleaning up test tables");
        adapter.execute_query("DROP TABLE TEST_LIST_USERS").await?;
        adapter.execute_query("DROP TABLE TEST_LIST_PRODUCTS").await?;
        adapter.execute_query("DROP TABLE TEST_LIST_ORDERS").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_list_tables");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_describe_table() -> Result<()> {
        info!("Starting test: test_oracle_describe_table");

        let mut adapter = setup().await?;

        // Create test table
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE TEST_DESCRIBE_TABLE (
                ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                USERNAME VARCHAR2(50) NOT NULL,
                EMAIL VARCHAR2(100)
            )
        ").await?;
        debug!("Test table created");

        // Describe table
        info!("Testing describe_table");
        let table_info = adapter.describe_table("TEST_DESCRIBE_TABLE", None).await?;

        assert_eq!(table_info.name, "TEST_DESCRIBE_TABLE");
        assert_eq!(table_info.columns.len(), 3);

        // Verify column details
        let username_col = table_info.columns.iter().find(|c| c.name == "USERNAME").unwrap();
        assert!(!username_col.nullable);

        debug!("Table schema validated");

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE TEST_DESCRIBE_TABLE").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_describe_table");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_crud_operations() -> Result<()> {
        info!("Starting test: test_oracle_crud_operations");

        let mut adapter = setup().await?;

        // CREATE table and INSERT data
        info!("Creating test table");
        adapter.execute_query("
            CREATE TABLE TEST_CRUD_USERS (
                ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                USERNAME VARCHAR2(50) NOT NULL,
                EMAIL VARCHAR2(100),
                STATUS VARCHAR2(20) DEFAULT 'active'
            )
        ").await?;

        info!("Testing INSERT");
        adapter.execute_query("
            INSERT ALL
            INTO TEST_CRUD_USERS (USERNAME, EMAIL) VALUES ('alice', 'alice@example.com')
            INTO TEST_CRUD_USERS (USERNAME, EMAIL) VALUES ('bob', 'bob@example.com')
            SELECT * FROM DUAL
        ").await?;
        debug!("Rows inserted");

        // READ
        info!("Testing SELECT");
        let result = adapter.execute_query("SELECT * FROM TEST_CRUD_USERS ORDER BY ID").await?;
        assert_eq!(result.rows.len(), 2);
        debug!("Found {} rows", result.rows.len());

        // UPDATE
        info!("Testing UPDATE");
        adapter.execute_query("
            UPDATE TEST_CRUD_USERS
            SET STATUS = 'inactive'
            WHERE USERNAME = 'alice'
        ").await?;
        debug!("Row updated");

        // Verify update
        let verify_result = adapter.execute_query("
            SELECT STATUS FROM TEST_CRUD_USERS WHERE USERNAME = 'alice'
        ").await?;
        assert_eq!(verify_result.rows.len(), 1);
        debug!("Update verified");

        // DELETE
        info!("Testing DELETE");
        adapter.execute_query("DELETE FROM TEST_CRUD_USERS WHERE USERNAME = 'bob'").await?;
        debug!("Row deleted");

        // Verify deletion
        let final_result = adapter.execute_query("SELECT * FROM TEST_CRUD_USERS").await?;
        assert_eq!(final_result.rows.len(), 1);
        debug!("Deletion verified: {} row remaining", final_result.rows.len());

        // Cleanup
        info!("Cleaning up test table");
        adapter.execute_query("DROP TABLE TEST_CRUD_USERS").await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_crud_operations");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_server_info() -> Result<()> {
        info!("Starting test: test_oracle_get_server_info");

        let mut adapter = setup().await?;

        info!("Testing get_server_info");
        let server_info = adapter.get_server_info().await?;

        assert_eq!(server_info.server_type, "Oracle Database");
        assert!(!server_info.version.is_empty());
        assert!(server_info.version.contains("Oracle"));
        debug!("Server version: {}", server_info.version);

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_server_info");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_database_metadata() -> Result<()> {
        info!("Starting test: test_oracle_get_database_metadata");

        let mut adapter = setup().await?;

        info!("Testing get_database_metadata");
        let db_meta = adapter.get_database_metadata("XE").await?;

        assert_eq!(db_meta.name, "XE");
        debug!("Database encoding: {:?}", db_meta.encoding);
        debug!("Database log mode: {:?}", db_meta.extra_info.get("log_mode"));

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_database_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_table_metadata() -> Result<()> {
        info!("Starting test: test_oracle_get_table_metadata");

        let mut adapter = setup().await?;

        // Create test table
        let table_name = "test_oracle_metadata_table";
        info!("Creating test table");
        adapter
            .execute_query(&format!(
                "CREATE TABLE {} (id NUMBER PRIMARY KEY, name VARCHAR2(100))",
                table_name
            ))
            .await?;

        adapter
            .execute_query(&format!(
                "INSERT INTO {} (id, name) VALUES (1, 'test1')",
                table_name
            ))
            .await?;
        adapter
            .execute_query(&format!(
                "INSERT INTO {} (id, name) VALUES (2, 'test2')",
                table_name
            ))
            .await?;

        // Get table metadata
        info!("Testing get_table_metadata");
        let table_meta = adapter.get_table_metadata(table_name, None).await?;

        assert_eq!(table_meta.name, table_name);
        debug!("Row count: {:?}", table_meta.row_count);

        // Cleanup
        adapter
            .execute_query(&format!("DROP TABLE {}", table_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_table_metadata");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_indexes() -> Result<()> {
        info!("Starting test: test_oracle_get_indexes");

        let mut adapter = setup().await?;

        // Create test table with indexes
        let table_name = "test_oracle_indexes_table";
        info!("Creating test table with indexes");
        adapter
            .execute_query(&format!(
                "CREATE TABLE {} (
                    id NUMBER PRIMARY KEY,
                    email VARCHAR2(100) UNIQUE,
                    name VARCHAR2(100)
                )",
                table_name
            ))
            .await?;

        adapter
            .execute_query(&format!(
                "CREATE INDEX idx_name ON {}(name)",
                table_name
            ))
            .await?;

        // Get indexes
        info!("Testing get_indexes");
        let indexes = adapter.get_indexes(table_name, None).await?;

        assert!(indexes.len() >= 2); // PRIMARY KEY, UNIQUE

        debug!("Found {} indexes", indexes.len());
        for idx in &indexes {
            debug!(
                "Index: {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary
            );
        }

        // Cleanup
        adapter
            .execute_query(&format!("DROP TABLE {}", table_name))
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_indexes");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_foreign_keys() -> Result<()> {
        info!("Starting test: test_oracle_get_foreign_keys");

        let mut adapter = setup().await?;

        // Create parent and child tables with FK
        info!("Creating tables with foreign key");
        adapter
            .execute_query(
                "CREATE TABLE test_oracle_fk_parent (
                    id NUMBER PRIMARY KEY,
                    name VARCHAR2(100)
                )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE TABLE test_oracle_fk_child (
                    id NUMBER PRIMARY KEY,
                    parent_id NUMBER,
                    data VARCHAR2(100),
                    FOREIGN KEY (parent_id) REFERENCES test_oracle_fk_parent(id) ON DELETE CASCADE
                )",
            )
            .await?;

        // Get foreign keys
        info!("Testing get_foreign_keys");
        let fks = adapter
            .get_foreign_keys("test_oracle_fk_child", None)
            .await?;

        assert_eq!(fks.len(), 1);
        let fk = &fks[0];
        assert_eq!(fk.table_name, "test_oracle_fk_child");
        assert_eq!(fk.referenced_table, "test_oracle_fk_parent");
        assert!(fk.columns.contains(&"PARENT_ID".to_string()));
        debug!("Foreign key: {} -> {}", fk.name, fk.referenced_table);

        // Cleanup
        adapter
            .execute_query("DROP TABLE test_oracle_fk_child")
            .await?;
        adapter
            .execute_query("DROP TABLE test_oracle_fk_parent")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_foreign_keys");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_get_views() -> Result<()> {
        info!("Starting test: test_oracle_get_views");

        let mut adapter = setup().await?;

        // Create test table and view
        info!("Creating test table and view");
        adapter
            .execute_query(
                "CREATE TABLE test_oracle_view_source (
                    id NUMBER PRIMARY KEY,
                    value NUMBER
                )",
            )
            .await?;

        adapter
            .execute_query(
                "CREATE VIEW test_oracle_my_view AS
                SELECT id, value * 2 AS doubled
                FROM test_oracle_view_source",
            )
            .await?;

        // Get views
        info!("Testing get_views");
        let views = adapter.get_views(None).await?;

        let test_view = views.iter().find(|v| v.name == "test_oracle_my_view");
        assert!(test_view.is_some());
        debug!("Found {} views", views.len());

        // Get view definition
        info!("Testing get_view_definition");
        let definition = adapter
            .get_view_definition("test_oracle_my_view", None)
            .await?;
        assert!(definition.is_some());
        debug!("View definition retrieved");

        // Cleanup
        adapter
            .execute_query("DROP VIEW test_oracle_my_view")
            .await?;
        adapter
            .execute_query("DROP TABLE test_oracle_view_source")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_get_views");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_oracle_list_stored_procedures() -> Result<()> {
        info!("Starting test: test_oracle_list_stored_procedures");

        let mut adapter = setup().await?;

        // Create test stored procedure
        info!("Creating test stored procedure");
        adapter
            .execute_query(
                "CREATE OR REPLACE PROCEDURE test_add_numbers (
                    a IN NUMBER,
                    b IN NUMBER,
                    result OUT NUMBER
                ) AS
                BEGIN
                    result := a + b;
                END test_add_numbers;",
            )
            .await?;

        // List procedures
        info!("Testing list_stored_procedures");
        let procedures = adapter.list_stored_procedures(None).await?;

        let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
        assert!(test_proc.is_some());
        let proc = test_proc.unwrap();
        assert_eq!(proc.language, Some("PL/SQL".to_string()));
        debug!("Found procedure: {} (language: {:?})", proc.name, proc.language);

        // Cleanup
        adapter
            .execute_query("DROP PROCEDURE test_add_numbers")
            .await?;

        adapter.disconnect().await?;
        info!("Test completed: test_oracle_list_stored_procedures");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_zzz_cleanup_oracle_database() -> Result<()> {
        info!("Oracle cleanup: Using existing XE database, no database drop needed");

        // For Oracle, we don't drop the database as it's a shared service
        // The individual tests clean up their own tables
        // This is just a placeholder cleanup test

        info!("Oracle test cleanup complete");
        Ok(())
    }
}

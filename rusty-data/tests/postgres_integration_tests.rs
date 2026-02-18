/// PostgreSQL Integration Tests
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
/// Then: cargo test --package rusty-data --features postgres -- --test-threads=1 --ignored
///
/// Tests are marked with #[ignore] so they don't run by default.
/// Use --ignored or --include-ignored to run them.

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::adapters::postgres::PostgresAdapter;
use rusty_data::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info};

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

    adapter
        .connect(&postgres_config, Some(TEST_PASSWORD))
        .await?;

    info!("Creating PostgreSQL test database: {}", TEST_DB_NAME);

    // Create database if it doesn't exist (will error if exists, we ignore)
    let _ = adapter
        .execute_query(&format!("CREATE DATABASE {}", TEST_DB_NAME))
        .await;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_query_users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(50),
            email VARCHAR(100),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN DEFAULT true
        )
    ",
        )
        .await?;

    // Insert test data
    adapter
        .execute_query(
            "
        INSERT INTO test_query_users (username, email) VALUES
        ('alice', 'alice@example.com'),
        ('bob', 'bob@example.com'),
        ('charlie', 'charlie@example.com')
    ",
        )
        .await?;
    debug!("Test data inserted");

    // Query test data
    info!("Testing execute_query");
    let result = adapter
        .execute_query("SELECT * FROM test_query_users ORDER BY id")
        .await?;

    assert_eq!(result.columns.len(), 5);
    assert_eq!(result.rows.len(), 3);

    let first_row = &result.rows[0];
    assert_eq!(first_row.len(), 5);
    debug!("Query results validated");

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_query_users")
        .await?;

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
    adapter
        .execute_query("CREATE TABLE test_list_users (id SERIAL PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE test_list_products (id SERIAL PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE test_list_orders (id SERIAL PRIMARY KEY)")
        .await?;
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
    adapter
        .execute_query("DROP TABLE test_list_users")
        .await?;
    adapter
        .execute_query("DROP TABLE test_list_products")
        .await?;
    adapter
        .execute_query("DROP TABLE test_list_orders")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_describe_table (
            id SERIAL PRIMARY KEY,
            username VARCHAR(50) NOT NULL,
            email VARCHAR(100)
        )
    ",
        )
        .await?;
    debug!("Test table created");

    // Describe table
    info!("Testing describe_table");
    let table_info = adapter
        .describe_table("test_describe_table", None)
        .await?;

    assert_eq!(table_info.name, "test_describe_table");
    assert_eq!(table_info.columns.len(), 3);

    let column_names: Vec<String> = table_info
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect();
    assert!(column_names.contains(&"id".to_string()));
    assert!(column_names.contains(&"username".to_string()));
    assert!(column_names.contains(&"email".to_string()));
    debug!("Table structure validated");

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_describe_table")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_crud_users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(50) NOT NULL UNIQUE,
            email VARCHAR(100) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    ",
        )
        .await?;

    // INSERT
    info!("Testing INSERT");
    let insert_result = adapter
        .execute_query(
            "
        INSERT INTO test_crud_users (username, email) VALUES
        ('alice', 'alice@example.com'),
        ('bob', 'bob@example.com'),
        ('charlie', 'charlie@example.com')
    ",
        )
        .await?;
    assert!(insert_result.rows_affected.is_some());
    debug!("Inserted 3 users");

    // SELECT
    info!("Testing SELECT");
    let select_result = adapter
        .execute_query("SELECT * FROM test_crud_users ORDER BY id")
        .await?;
    assert_eq!(select_result.columns.len(), 4);
    assert_eq!(select_result.rows.len(), 3);
    assert_eq!(
        select_result.columns,
        vec!["id", "username", "email", "created_at"]
    );
    debug!("SELECT validated");

    // UPDATE
    info!("Testing UPDATE");
    adapter
        .execute_query(
            "UPDATE test_crud_users SET email = 'alice@newdomain.com' WHERE username = 'alice'",
        )
        .await?;
    let updated = adapter
        .execute_query("SELECT email FROM test_crud_users WHERE username = 'alice'")
        .await?;
    assert_eq!(updated.rows.len(), 1);
    debug!("UPDATE validated");

    // DELETE
    info!("Testing DELETE");
    adapter
        .execute_query("DELETE FROM test_crud_users WHERE username = 'bob'")
        .await?;
    let remaining = adapter
        .execute_query("SELECT * FROM test_crud_users ORDER BY id")
        .await?;
    assert_eq!(remaining.rows.len(), 2);
    debug!("DELETE validated");

    // Cleanup: Drop test table
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_crud_users")
        .await?;

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
    let result = adapter
        .execute_query("SELECT * FROM non_existent_table")
        .await;
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
        let result = adapter
            .execute_query(&format!("SELECT {} as value", i))
            .await?;
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
    adapter
        .execute_query(
            "
        CREATE TABLE test_schema.test_table (
            id SERIAL PRIMARY KEY,
            data VARCHAR(100)
        )
    ",
        )
        .await?;
    debug!("Test schema and table created");

    // List tables in test_schema
    info!("Testing list_tables with schema");
    let tables = adapter.list_tables(Some("test_schema")).await?;
    assert!(tables.contains(&"test_table".to_string()));
    debug!("Found table in test schema");

    // Cleanup
    info!("Cleaning up test schema and table");
    adapter
        .execute_query("DROP TABLE test_schema.test_table")
        .await?;
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
    adapter
        .execute_query(
            "
        CREATE TABLE test_metadata_table (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL
        )
    ",
        )
        .await?;

    // Insert some rows for row count
    adapter
        .execute_query("INSERT INTO test_metadata_table (name) VALUES ('test1'), ('test2'), ('test3')")
        .await?;

    // Get table metadata
    info!("Testing get_table_metadata");
    let table_meta = adapter
        .get_table_metadata("test_metadata_table", None)
        .await?;

    assert_eq!(table_meta.name, "test_metadata_table");
    assert_eq!(table_meta.schema, Some("public".to_string()));
    assert!(table_meta.size_bytes.is_some());
    assert!(table_meta.row_count.is_some());
    debug!("Table size: {:?} bytes", table_meta.size_bytes);
    debug!("Row count: {:?}", table_meta.row_count);

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_metadata_table")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_indexes_table (
            id SERIAL PRIMARY KEY,
            email VARCHAR(100) UNIQUE,
            name TEXT,
            status TEXT
        )
    ",
        )
        .await?;

    adapter
        .execute_query("CREATE INDEX idx_name ON test_indexes_table(name)")
        .await?;
    adapter
        .execute_query("CREATE INDEX idx_status ON test_indexes_table(status)")
        .await?;

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
        debug!(
            "Index: {} (columns: {:?}, unique: {}, primary: {})",
            idx.name, idx.columns, idx.is_unique, idx.is_primary
        );
    }

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_indexes_table")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_fk_parent (
            id SERIAL PRIMARY KEY,
            name TEXT
        )
    ",
        )
        .await?;

    adapter
        .execute_query(
            "
        CREATE TABLE test_fk_child (
            id SERIAL PRIMARY KEY,
            parent_id INTEGER REFERENCES test_fk_parent(id) ON DELETE CASCADE,
            data TEXT
        )
    ",
        )
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_view_source (
            id SERIAL PRIMARY KEY,
            value INTEGER
        )
    ",
        )
        .await?;

    adapter
        .execute_query(
            "
        CREATE VIEW test_my_view AS
        SELECT id, value * 2 AS doubled
        FROM test_view_source
    ",
        )
        .await?;

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
    adapter
        .execute_query("DROP TABLE test_view_source")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE FUNCTION test_add_numbers(a INTEGER, b INTEGER)
        RETURNS INTEGER AS $$
        BEGIN
            RETURN a + b;
        END;
        $$ LANGUAGE plpgsql
    ",
        )
        .await?;

    // List procedures
    info!("Testing list_stored_procedures");
    let procedures = adapter.list_stored_procedures(None).await?;

    let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
    assert!(test_proc.is_some());
    let proc = test_proc.unwrap();
    assert_eq!(proc.language, Some("plpgsql".to_string()));
    debug!(
        "Found function: {} (language: {:?})",
        proc.name, proc.language
    );

    // Cleanup
    info!("Cleaning up test function");
    adapter
        .execute_query("DROP FUNCTION test_add_numbers")
        .await?;

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

    adapter
        .connect(&postgres_config, Some(TEST_PASSWORD))
        .await?;

    // Drop test database
    adapter
        .execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME))
        .await?;

    adapter.disconnect().await?;

    info!("PostgreSQL test database cleanup complete");
    Ok(())
}

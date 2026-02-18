// ============================================================================
// MSSQL Integration Tests
// ============================================================================
//
// These tests validate the MssqlAdapter implementation by exercising all major
// database operations against a real SQL Server instance.
//
// Prerequisites:
// - SQL Server instance running on localhost:1433
// - SA user with password: Test_Password123!
//
// To run: cargo test --test mssql_integration_tests --features mssql -- --ignored --nocapture

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::adapters::mssql::MssqlAdapter;
use rusty_data::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info};

// MSSQL test database - shared across all tests
const TEST_DB_NAME: &str = "rusty_test_mssql";
const TEST_USER: &str = "sa";
const TEST_PASSWORD: &str = "Test_Password123!";

// One-time database creation
static INIT: AtomicBool = AtomicBool::new(false);

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

    // Try to set database to single user mode and drop it (ignore errors if it doesn't exist)
    let _ = adapter.execute_query(&format!(
        "IF EXISTS (SELECT name FROM sys.databases WHERE name = '{}')
         BEGIN
             ALTER DATABASE {} SET SINGLE_USER WITH ROLLBACK IMMEDIATE;
             DROP DATABASE {};
         END",
        TEST_DB_NAME, TEST_DB_NAME, TEST_DB_NAME
    )).await;

    // Create the database
    adapter.execute_query(&format!("CREATE DATABASE {}", TEST_DB_NAME)).await?;

    adapter.disconnect().await?;
    debug!("MSSQL test database ready: {}", TEST_DB_NAME);
    Ok(())
}

async fn setup() -> Result<MssqlAdapter> {
    if !INIT.load(Ordering::Relaxed) {
        ensure_test_database().await?;
        INIT.store(true, Ordering::Relaxed);
    }

    let mut adapter = MssqlAdapter::new();
    let config = get_mssql_config(TEST_DB_NAME);
    adapter.connect(&config, Some(TEST_PASSWORD)).await?;
    Ok(adapter)
}

#[tokio::test]
#[ignore]
async fn test_mssql_connect_disconnect() -> Result<()> {
    info!("Starting test: test_mssql_connect_disconnect");

    // Ensure database exists
    ensure_test_database().await?;

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
        .execute_query("CREATE VIEW dbo.test_mssql_my_view AS SELECT id, value * 2 AS doubled FROM dbo.test_mssql_view_source")
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
        .execute_query("CREATE PROCEDURE dbo.test_add_numbers @a INT, @b INT AS BEGIN SELECT @a + @b AS result END")
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

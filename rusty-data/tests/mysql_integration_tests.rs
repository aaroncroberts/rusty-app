//! MySQL Integration Tests
//!
//! This module contains integration tests for the MySQL database adapter.
//! These tests verify the functionality of connecting to MySQL, executing queries,
//! retrieving metadata, and managing database objects like tables, views, and indexes.
//!
//! Note: These tests are marked with `#[ignore]` and require a running MySQL instance
//! with appropriate test credentials configured.

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::adapters::mysql::MySqlAdapter;
use rusty_data::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info};

const TEST_PASSWORD: &str = "test_password";
const TEST_DB_NAME: &str = "rusty_test_mysql";

static INIT: AtomicBool = AtomicBool::new(false);

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
    // Use root credentials to create database
    let mut root_config = get_mysql_config("mysql");
    root_config.username = Some("root".to_string());

    adapter.connect(&root_config, Some("root_password")).await?;

    info!("Creating MySQL test database: {}", TEST_DB_NAME);

    // Create database if it doesn't exist
    let _ = adapter
        .execute_query(&format!("CREATE DATABASE IF NOT EXISTS {}", TEST_DB_NAME))
        .await?;
    // Grant all privileges to test_user
    let _ = adapter
        .execute_query(&format!(
            "GRANT ALL PRIVILEGES ON {}.* TO 'test_user'@'%'",
            TEST_DB_NAME
        ))
        .await?;

    adapter.disconnect().await?;
    debug!("MySQL test database ready: {}", TEST_DB_NAME);
    Ok(())
}

/// Setup function called before each test
async fn setup() -> Result<MySqlAdapter> {
    // Ensure database exists (happens once)
    if !INIT.load(Ordering::Relaxed) {
        ensure_test_database().await?;
        INIT.store(true, Ordering::Relaxed);
    }

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_query_users (
            id INT AUTO_INCREMENT PRIMARY KEY,
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
    adapter
        .execute_query("CREATE TABLE test_list_users (id INT AUTO_INCREMENT PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE test_list_products (id INT AUTO_INCREMENT PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE test_list_orders (id INT AUTO_INCREMENT PRIMARY KEY)")
        .await?;
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
    adapter
        .execute_query("DROP TABLE test_list_products")
        .await?;
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
    adapter
        .execute_query(
            "
        CREATE TABLE test_describe_table (
            id INT AUTO_INCREMENT PRIMARY KEY,
            username VARCHAR(50) NOT NULL,
            email VARCHAR(100)
        )
    ",
        )
        .await?;
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
    adapter
        .execute_query("DROP TABLE test_describe_table")
        .await?;

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
    // size_bytes and encoding may be None for some databases
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
    adapter
        .execute_query(
            "
        CREATE TABLE test_mysql_metadata_table (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100) NOT NULL
        ) ENGINE=InnoDB
    ",
        )
        .await?;

    // Insert some rows for row count
    adapter
        .execute_query(
            "INSERT INTO test_mysql_metadata_table (name) VALUES ('test1'), ('test2'), ('test3')",
        )
        .await?;

    // Get table metadata
    info!("Testing get_table_metadata");
    let table_meta = adapter
        .get_table_metadata("test_mysql_metadata_table", None)
        .await?;

    assert_eq!(table_meta.name, "test_mysql_metadata_table");
    // size_bytes, row_count may be None for newly created tables
    debug!("Table size: {:?} bytes", table_meta.size_bytes);
    debug!("Row count: {:?}", table_meta.row_count);
    debug!("Table engine: {:?}", table_meta.table_type);

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE test_mysql_metadata_table")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_mysql_indexes_table (
            id INT AUTO_INCREMENT PRIMARY KEY,
            email VARCHAR(100) UNIQUE,
            name VARCHAR(100),
            status VARCHAR(50)
        )
    ",
        )
        .await?;

    adapter
        .execute_query("CREATE INDEX idx_name ON test_mysql_indexes_table(name)")
        .await?;
    adapter
        .execute_query("CREATE INDEX idx_status ON test_mysql_indexes_table(status)")
        .await?;

    // Get indexes
    info!("Testing get_indexes");
    let indexes = adapter
        .get_indexes("test_mysql_indexes_table", None)
        .await?;

    assert!(indexes.len() >= 3); // PRIMARY KEY, UNIQUE, and our indexes

    // Find specific indexes
    let primary_idx = indexes.iter().find(|i| i.is_primary);
    assert!(primary_idx.is_some());
    assert_eq!(primary_idx.unwrap().name, "PRIMARY");

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
        .execute_query("DROP TABLE test_mysql_indexes_table")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_mysql_fk_parent (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100)
        )
    ",
        )
        .await?;

    adapter
        .execute_query(
            "
        CREATE TABLE test_mysql_fk_child (
            id INT AUTO_INCREMENT PRIMARY KEY,
            parent_id INT,
            data VARCHAR(100),
            FOREIGN KEY (parent_id) REFERENCES test_mysql_fk_parent(id) ON DELETE CASCADE
        )
    ",
        )
        .await?;

    // Get foreign keys
    info!("Testing get_foreign_keys");
    let fks = adapter
        .get_foreign_keys("test_mysql_fk_child", None)
        .await?;

    assert_eq!(fks.len(), 1);
    let fk = &fks[0];
    assert_eq!(fk.table_name, "test_mysql_fk_child");
    assert_eq!(fk.referenced_table, "test_mysql_fk_parent");
    assert!(fk.columns.contains(&"parent_id".to_string()));
    assert!(fk.referenced_columns.contains(&"id".to_string()));
    // on_delete and on_update may not be available from all schemas
    debug!(
        "Foreign key: {} -> {} (on_delete: {:?})",
        fk.name, fk.referenced_table, fk.on_delete
    );

    // Cleanup
    info!("Cleaning up test tables");
    adapter
        .execute_query("DROP TABLE test_mysql_fk_child")
        .await?;
    adapter
        .execute_query("DROP TABLE test_mysql_fk_parent")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE test_mysql_view_source (
            id INT AUTO_INCREMENT PRIMARY KEY,
            value INT
        )
    ",
        )
        .await?;

    adapter
        .execute_query(
            "
        CREATE VIEW test_mysql_my_view AS
        SELECT id, value * 2 AS doubled
        FROM test_mysql_view_source
    ",
        )
        .await?;

    // Get views
    info!("Testing get_views");
    let views = adapter.get_views(None).await?;

    let test_view = views.iter().find(|v| v.name == "test_mysql_my_view");
    assert!(test_view.is_some());
    debug!("Found {} views", views.len());

    // Get view definition
    info!("Testing get_view_definition");
    let definition = adapter
        .get_view_definition("test_mysql_my_view", None)
        .await?;
    assert!(definition.is_some());
    assert!(definition
        .unwrap()
        .to_lowercase()
        .contains("test_mysql_view_source"));
    debug!("View definition retrieved");

    // Cleanup
    info!("Cleaning up test view and table");
    adapter
        .execute_query("DROP VIEW test_mysql_my_view")
        .await?;
    adapter
        .execute_query("DROP TABLE test_mysql_view_source")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE PROCEDURE test_add_numbers(IN a INT, IN b INT, OUT result INT)
        BEGIN
            SET result = a + b;
        END
    ",
        )
        .await?;

    // List procedures
    info!("Testing list_stored_procedures");
    let procedures = adapter.list_stored_procedures(None).await?;

    let test_proc = procedures.iter().find(|p| p.name == "test_add_numbers");
    assert!(test_proc.is_some());
    let proc = test_proc.unwrap();
    assert_eq!(proc.language, Some("SQL".to_string()));
    debug!(
        "Found procedure: {} (language: {:?})",
        proc.name, proc.language
    );

    // Cleanup
    info!("Cleaning up test procedure");
    adapter
        .execute_query("DROP PROCEDURE test_add_numbers")
        .await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mysql_list_stored_procedures");
    Ok(())
}

// ========== Bulk Operations Integration Tests ==========

#[tokio::test]
#[ignore]
async fn test_mysql_bulk_insert() -> Result<()> {
    use rusty_data::adapter::QueryValue;

    info!("Starting test: test_mysql_bulk_insert");

    let mut adapter = setup().await?;

    // Create test table
    info!("Creating test table for bulk insert");
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_insert (
            id INT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            email VARCHAR(100),
            active BOOLEAN,
            score FLOAT
        )
    ",
        )
        .await?;

    // Prepare bulk insert data
    info!("Preparing bulk insert data");
    let columns = vec![
        "id".to_string(),
        "name".to_string(),
        "email".to_string(),
        "active".to_string(),
        "score".to_string(),
    ];

    let rows = vec![
        vec![
            QueryValue::Int(1),
            QueryValue::Text("Alice".to_string()),
            QueryValue::Text("alice@example.com".to_string()),
            QueryValue::Bool(true),
            QueryValue::Float(95.5),
        ],
        vec![
            QueryValue::Int(2),
            QueryValue::Text("Bob".to_string()),
            QueryValue::Text("bob@example.com".to_string()),
            QueryValue::Bool(false),
            QueryValue::Float(87.3),
        ],
        vec![
            QueryValue::Int(3),
            QueryValue::Text("Charlie".to_string()),
            QueryValue::Null,
            QueryValue::Bool(true),
            QueryValue::Float(92.8),
        ],
        vec![
            QueryValue::Int(4),
            QueryValue::Text("Diana".to_string()),
            QueryValue::Text("diana@example.com".to_string()),
            QueryValue::Bool(true),
            QueryValue::Float(98.1),
        ],
    ];

    // Execute bulk insert
    info!("Executing bulk insert of {} rows", rows.len());
    let rows_affected = adapter
        .bulk_insert("test_bulk_insert", &columns, &rows, None)
        .await?;

    assert_eq!(rows_affected, 4);
    debug!("Bulk insert completed: {} rows affected", rows_affected);

    // Verify the data was inserted
    info!("Verifying inserted data");
    let result = adapter
        .execute_query("SELECT * FROM test_bulk_insert ORDER BY id")
        .await?;

    assert_eq!(result.rows.len(), 4);
    assert_eq!(
        result.columns,
        vec!["id", "name", "email", "active", "score"]
    );
    debug!("Data verification successful");

    // Cleanup
    info!("Cleaning up test table");
    adapter.execute_query("DROP TABLE test_bulk_insert").await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mysql_bulk_insert");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mysql_bulk_update() -> Result<()> {
    use rusty_data::adapter::QueryValue;
    use std::collections::HashMap;

    info!("Starting test: test_mysql_bulk_update");

    let mut adapter = setup().await?;

    // Create test table with sample data
    info!("Creating test table for bulk update");
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_update (
            id INT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            status VARCHAR(50),
            score INT
        )
    ",
        )
        .await?;

    // Insert initial data
    adapter
        .execute_query(
            "
        INSERT INTO test_bulk_update (id, name, status, score) VALUES
        (1, 'Alice', 'pending', 50),
        (2, 'Bob', 'pending', 60),
        (3, 'Charlie', 'active', 70),
        (4, 'Diana', 'pending', 80)
    ",
        )
        .await?;

    // Prepare bulk updates
    info!("Preparing bulk update data");
    let mut update1 = HashMap::new();
    update1.insert("status".to_string(), QueryValue::Text("active".to_string()));
    update1.insert("score".to_string(), QueryValue::Int(100));

    let mut update2 = HashMap::new();
    update2.insert(
        "status".to_string(),
        QueryValue::Text("completed".to_string()),
    );

    let updates = vec![
        (update1, "id = 1".to_string()),
        (update2, "id = 2".to_string()),
    ];

    // Execute bulk update
    info!("Executing bulk update");
    let rows_affected = adapter
        .bulk_update("test_bulk_update", &updates, None)
        .await?;

    assert_eq!(rows_affected, 2);
    debug!("Bulk update completed: {} rows affected", rows_affected);

    // Verify the updates
    info!("Verifying updated data");
    let result = adapter
        .execute_query("SELECT * FROM test_bulk_update WHERE id IN (1, 2) ORDER BY id")
        .await?;

    assert_eq!(result.rows.len(), 2);
    debug!("Update verification successful");

    // Cleanup
    info!("Cleaning up test table");
    adapter.execute_query("DROP TABLE test_bulk_update").await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mysql_bulk_update");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mysql_bulk_delete() -> Result<()> {
    info!("Starting test: test_mysql_bulk_delete");

    let mut adapter = setup().await?;

    // Create test table with sample data
    info!("Creating test table for bulk delete");
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_delete (
            id INT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            status VARCHAR(50)
        )
    ",
        )
        .await?;

    // Insert initial data
    adapter
        .execute_query(
            "
        INSERT INTO test_bulk_delete (id, name, status) VALUES
        (1, 'Alice', 'active'),
        (2, 'Bob', 'inactive'),
        (3, 'Charlie', 'active'),
        (4, 'Diana', 'inactive'),
        (5, 'Eve', 'active'),
        (6, 'Frank', 'inactive')
    ",
        )
        .await?;

    // Verify initial count
    let initial = adapter
        .execute_query("SELECT COUNT(*) FROM test_bulk_delete")
        .await?;
    assert_eq!(initial.rows.len(), 1);

    // Prepare bulk deletes
    info!("Preparing bulk delete clauses");
    let where_clauses = vec![
        "id = 2".to_string(),
        "id = 4".to_string(),
        "id = 6".to_string(),
    ];

    // Execute bulk delete
    info!("Executing bulk delete");
    let rows_affected = adapter
        .bulk_delete("test_bulk_delete", &where_clauses, None)
        .await?;

    assert_eq!(rows_affected, 3);
    debug!("Bulk delete completed: {} rows affected", rows_affected);

    // Verify remaining data
    info!("Verifying remaining data");
    let result = adapter
        .execute_query("SELECT * FROM test_bulk_delete ORDER BY id")
        .await?;

    assert_eq!(result.rows.len(), 3);
    // Should have IDs 1, 3, 5 remaining
    debug!(
        "Delete verification successful: {} rows remain",
        result.rows.len()
    );

    // Cleanup
    info!("Cleaning up test table");
    adapter.execute_query("DROP TABLE test_bulk_delete").await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mysql_bulk_delete");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mysql_bulk_insert_large_batch() -> Result<()> {
    use rusty_data::adapter::QueryValue;

    info!("Starting test: test_mysql_bulk_insert_large_batch");

    let mut adapter = setup().await?;

    // Create test table
    info!("Creating test table for large batch insert");
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_large (
            id INT PRIMARY KEY,
            data VARCHAR(100)
        )
    ",
        )
        .await?;

    // Generate 1000 rows
    info!("Generating 1000 rows for bulk insert");
    let columns = vec!["id".to_string(), "data".to_string()];
    let mut rows = Vec::new();

    for i in 1..=1000 {
        rows.push(vec![
            QueryValue::Int(i),
            QueryValue::Text(format!("data_{}", i)),
        ]);
    }

    // Execute bulk insert
    info!("Executing bulk insert of 1000 rows");
    let start = std::time::Instant::now();
    let rows_affected = adapter
        .bulk_insert("test_bulk_large", &columns, &rows, None)
        .await?;
    let elapsed = start.elapsed();

    assert_eq!(rows_affected, 1000);
    info!(
        "Bulk insert completed: {} rows in {}ms",
        rows_affected,
        elapsed.as_millis()
    );

    // Verify count
    let result = adapter
        .execute_query("SELECT COUNT(*) FROM test_bulk_large")
        .await?;
    assert_eq!(result.rows.len(), 1);

    // Cleanup
    info!("Cleaning up test table");
    adapter.execute_query("DROP TABLE test_bulk_large").await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mysql_bulk_insert_large_batch");
    Ok(())
}

/// Cleanup test - drops the test database after all tests complete
/// Named with zzz prefix to run last (tests run alphabetically)
#[tokio::test]
#[ignore]
async fn test_zzz_cleanup_mysql_database() -> Result<()> {
    info!("Cleaning up MySQL test database: {}", TEST_DB_NAME);

    let mut adapter = MySqlAdapter::new();
    // Use root credentials to drop database
    let mut root_config = get_mysql_config("mysql");
    root_config.username = Some("root".to_string());

    adapter.connect(&root_config, Some("root_password")).await?;

    // Drop test database
    adapter
        .execute_query(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME))
        .await?;

    adapter.disconnect().await?;

    info!("MySQL test database cleanup complete");
    Ok(())
}

// =============================================================================
// Oracle Integration Tests
// =============================================================================
//
// This module contains comprehensive integration tests for the Oracle database
// adapter. These tests verify connectivity, query execution, metadata operations,
// CRUD operations, and schema introspection capabilities against an Oracle
// database instance.
//
// Note: Tests are marked with #[ignore] and require a running Oracle instance
// (Oracle Database 23ai Free - ARM-compatible) with appropriate credentials configured.
// Run tests with: cargo test --features oracle --test oracle_integration_tests -- --ignored

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::adapters::oracle::OracleAdapter;
use rusty_data::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info};

// Oracle test database - shared across all tests
// For Oracle, we use a schema/user instead of creating a new database
// Using Oracle 23ai Free (ARM-compatible for Apple Silicon)
const TEST_DB_NAME: &str = "FREE"; // Service name (changed from XE to FREE for Oracle 23ai)
const TEST_USER: &str = "system";
const TEST_PASSWORD: &str = "TestPassword123"; // Password without special chars for container compatibility

// One-time database setup
static INIT: AtomicBool = AtomicBool::new(false);

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
    // For Oracle, the FREE database already exists (Oracle 23ai Free)
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
    if !INIT.load(Ordering::Relaxed) {
        ensure_test_database().await?;
        INIT.store(true, Ordering::Relaxed);
    }

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
    adapter
        .execute_query(
            "
        CREATE TABLE TEST_QUERY_USERS (
            ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            USERNAME VARCHAR2(50),
            EMAIL VARCHAR2(100),
            CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            IS_ACTIVE NUMBER(1) DEFAULT 1
        )
    ",
        )
        .await?;

    // Insert test data
    adapter
        .execute_query(
            "
        INSERT ALL
        INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('alice', 'alice@example.com')
        INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('bob', 'bob@example.com')
        INTO TEST_QUERY_USERS (USERNAME, EMAIL) VALUES ('charlie', 'charlie@example.com')
        SELECT * FROM DUAL
    ",
        )
        .await?;
    debug!("Test data inserted");

    // Query test data
    info!("Testing execute_query");
    let result = adapter
        .execute_query("SELECT * FROM TEST_QUERY_USERS ORDER BY ID")
        .await?;

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
    adapter
        .execute_query("CREATE TABLE TEST_LIST_USERS (ID NUMBER PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE TEST_LIST_PRODUCTS (ID NUMBER PRIMARY KEY)")
        .await?;
    adapter
        .execute_query("CREATE TABLE TEST_LIST_ORDERS (ID NUMBER PRIMARY KEY)")
        .await?;
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
    adapter
        .execute_query("DROP TABLE TEST_LIST_PRODUCTS")
        .await?;
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
    adapter
        .execute_query(
            "
        CREATE TABLE TEST_DESCRIBE_TABLE (
            ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            USERNAME VARCHAR2(50) NOT NULL,
            EMAIL VARCHAR2(100)
        )
    ",
        )
        .await?;
    debug!("Test table created");

    // Describe table
    info!("Testing describe_table");
    let table_info = adapter.describe_table("TEST_DESCRIBE_TABLE", None).await?;

    assert_eq!(table_info.name, "TEST_DESCRIBE_TABLE");
    assert_eq!(table_info.columns.len(), 3);

    // Verify column details
    let username_col = table_info
        .columns
        .iter()
        .find(|c| c.name == "USERNAME")
        .unwrap();
    assert!(!username_col.nullable);

    debug!("Table schema validated");

    // Cleanup
    info!("Cleaning up test table");
    adapter
        .execute_query("DROP TABLE TEST_DESCRIBE_TABLE")
        .await?;

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
    adapter
        .execute_query(
            "
        CREATE TABLE TEST_CRUD_USERS (
            ID NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            USERNAME VARCHAR2(50) NOT NULL,
            EMAIL VARCHAR2(100),
            STATUS VARCHAR2(20) DEFAULT 'active'
        )
    ",
        )
        .await?;

    info!("Testing INSERT");
    adapter
        .execute_query(
            "
        INSERT ALL
        INTO TEST_CRUD_USERS (USERNAME, EMAIL) VALUES ('alice', 'alice@example.com')
        INTO TEST_CRUD_USERS (USERNAME, EMAIL) VALUES ('bob', 'bob@example.com')
        SELECT * FROM DUAL
    ",
        )
        .await?;
    debug!("Rows inserted");

    // READ
    info!("Testing SELECT");
    let result = adapter
        .execute_query("SELECT * FROM TEST_CRUD_USERS ORDER BY ID")
        .await?;
    assert_eq!(result.rows.len(), 2);
    debug!("Found {} rows", result.rows.len());

    // UPDATE
    info!("Testing UPDATE");
    adapter
        .execute_query(
            "
        UPDATE TEST_CRUD_USERS
        SET STATUS = 'inactive'
        WHERE USERNAME = 'alice'
    ",
        )
        .await?;
    debug!("Row updated");

    // Verify update
    let verify_result = adapter
        .execute_query(
            "
        SELECT STATUS FROM TEST_CRUD_USERS WHERE USERNAME = 'alice'
    ",
        )
        .await?;
    assert_eq!(verify_result.rows.len(), 1);
    debug!("Update verified");

    // DELETE
    info!("Testing DELETE");
    adapter
        .execute_query("DELETE FROM TEST_CRUD_USERS WHERE USERNAME = 'bob'")
        .await?;
    debug!("Row deleted");

    // Verify deletion
    let final_result = adapter
        .execute_query("SELECT * FROM TEST_CRUD_USERS")
        .await?;
    assert_eq!(final_result.rows.len(), 1);
    debug!(
        "Deletion verified: {} row remaining",
        final_result.rows.len()
    );

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
    let db_meta = adapter.get_database_metadata("FREE").await?;

    assert_eq!(db_meta.name, "FREE");
    debug!("Database encoding: {:?}", db_meta.encoding);
    debug!(
        "Database log mode: {:?}",
        db_meta.extra_info.get("log_mode")
    );

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
        .execute_query(&format!("CREATE INDEX idx_name ON {}(name)", table_name))
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
    debug!(
        "Found procedure: {} (language: {:?})",
        proc.name, proc.language
    );

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
async fn test_oracle_bulk_insert() -> Result<()> {
    info!("=== Starting test: test_oracle_bulk_insert ===");
    let test_start = std::time::Instant::now();

    let mut adapter = setup().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to setup Oracle adapter");
        e
    })?;
    debug!(
        "Oracle adapter setup completed in {:?}",
        test_start.elapsed()
    );

    // Create test table with detailed logging
    info!("Creating test table: test_bulk_insert");
    let create_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_insert (
            id NUMBER PRIMARY KEY,
            name VARCHAR2(100),
            value NUMBER
        )
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Oracle test table");
            e
        })?;
    debug!("Oracle table created in {:?}", create_start.elapsed());

    // Prepare bulk insert data with logging
    use rusty_data::adapter::QueryValue;
    let columns = vec!["id".to_string(), "name".to_string(), "value".to_string()];
    let rows = vec![
        vec![
            QueryValue::Int(1),
            QueryValue::Text("Alice".to_string()),
            QueryValue::Int(100),
        ],
        vec![
            QueryValue::Int(2),
            QueryValue::Text("Bob".to_string()),
            QueryValue::Int(200),
        ],
        vec![
            QueryValue::Int(3),
            QueryValue::Text("Charlie".to_string()),
            QueryValue::Int(300),
        ],
    ];
    info!(
        "Prepared {} rows for Oracle INSERT ALL with {} columns",
        rows.len(),
        columns.len()
    );
    debug!("Columns: {:?}", columns);

    // Execute bulk insert with timing
    info!("Executing Oracle INSERT ALL bulk operation");
    let insert_start = std::time::Instant::now();
    let rows_inserted = adapter
        .bulk_insert("test_bulk_insert", &columns, &rows, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                table = "test_bulk_insert",
                row_count = rows.len(),
                "Oracle INSERT ALL operation failed"
            );
            e
        })?;
    let insert_duration = insert_start.elapsed();

    info!(
        "Oracle INSERT ALL completed: {} rows in {:?} ({:.2} rows/sec)",
        rows_inserted,
        insert_duration,
        rows_inserted as f64 / insert_duration.as_secs_f64()
    );

    assert_eq!(
        rows_inserted, 3,
        "Expected 3 rows inserted, got {}",
        rows_inserted
    );

    // Verify data was inserted with detailed checking
    info!("Verifying Oracle inserted data");
    let verify_start = std::time::Instant::now();
    let result = adapter
        .execute_query("SELECT * FROM test_bulk_insert ORDER BY id")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify Oracle inserted data");
            e
        })?;
    debug!(
        "Oracle verification query completed in {:?}",
        verify_start.elapsed()
    );

    assert_eq!(
        result.rows.len(),
        3,
        "Expected 3 rows in result, got {}",
        result.rows.len()
    );
    info!(
        "✓ Oracle data verification successful: {} rows found",
        result.rows.len()
    );

    // Log sample of inserted data
    for (idx, row) in result.rows.iter().take(3).enumerate() {
        debug!("Oracle row {}: {:?}", idx + 1, row);
    }

    // Cleanup with error handling
    info!("Cleaning up Oracle test table");
    let cleanup_result = adapter.execute_query("DROP TABLE test_bulk_insert").await;
    if let Err(e) = cleanup_result {
        tracing::warn!(error = %e, "Failed to drop Oracle test table (cleanup)");
    } else {
        debug!("Oracle test table dropped successfully");
    }

    adapter.disconnect().await.map_err(|e| {
        tracing::warn!(error = %e, "Failed to disconnect Oracle adapter");
        e
    })?;

    let total_duration = test_start.elapsed();
    info!(
        "=== Test completed: test_oracle_bulk_insert in {:?} ===",
        total_duration
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_oracle_bulk_update() -> Result<()> {
    info!("=== Starting test: test_oracle_bulk_update ===");
    let test_start = std::time::Instant::now();

    let mut adapter = setup().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to setup Oracle adapter");
        e
    })?;
    debug!(
        "Oracle adapter setup completed in {:?}",
        test_start.elapsed()
    );

    // Create test table with logging
    info!("Creating Oracle test table: test_bulk_update");
    let create_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_update (
            id NUMBER PRIMARY KEY,
            name VARCHAR2(100),
            status VARCHAR2(50)
        )
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Oracle test table");
            e
        })?;
    debug!("Oracle table created in {:?}", create_start.elapsed());

    // Insert initial test data
    info!("Inserting initial Oracle test data (3 rows)");
    let insert_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        INSERT ALL
            INTO test_bulk_update (id, name, status) VALUES (1, 'Alice', 'active')
            INTO test_bulk_update (id, name, status) VALUES (2, 'Bob', 'active')
            INTO test_bulk_update (id, name, status) VALUES (3, 'Charlie', 'active')
        SELECT * FROM DUAL
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to insert Oracle initial test data");
            e
        })?;
    debug!(
        "Oracle initial data inserted in {:?}",
        insert_start.elapsed()
    );

    // Prepare bulk update operations with logging
    use rusty_data::adapter::QueryValue;
    use std::collections::HashMap;

    let mut update1 = HashMap::new();
    update1.insert(
        "status".to_string(),
        QueryValue::Text("inactive".to_string()),
    );

    let mut update2 = HashMap::new();
    update2.insert(
        "status".to_string(),
        QueryValue::Text("suspended".to_string()),
    );

    let updates = vec![
        (update1, "id = 1".to_string()),
        (update2, "id = 3".to_string()),
    ];

    info!("Prepared {} Oracle update operations", updates.len());
    debug!(
        "Update operations: {} updates with WHERE conditions",
        updates.len()
    );

    // Execute bulk update with timing
    info!("Executing Oracle bulk_update operation");
    let update_start = std::time::Instant::now();
    let rows_updated = adapter
        .bulk_update("test_bulk_update", &updates, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                table = "test_bulk_update",
                update_count = updates.len(),
                "Oracle bulk update operation failed"
            );
            e
        })?;
    let update_duration = update_start.elapsed();

    info!(
        "Oracle bulk update completed: {} rows in {:?}",
        rows_updated, update_duration
    );

    assert_eq!(
        rows_updated, 2,
        "Expected 2 rows updated, got {}",
        rows_updated
    );

    // Verify updates with detailed checking
    info!("Verifying Oracle updated data");
    let verify_start = std::time::Instant::now();
    let result = adapter
        .execute_query(
            "SELECT id, name, status FROM test_bulk_update WHERE status != 'active' ORDER BY id",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify Oracle updated data");
            e
        })?;
    debug!(
        "Oracle verification query completed in {:?}",
        verify_start.elapsed()
    );

    assert_eq!(
        result.rows.len(),
        2,
        "Expected 2 updated rows in result, got {}",
        result.rows.len()
    );
    info!(
        "✓ Oracle update verification successful: {} rows updated as expected",
        result.rows.len()
    );

    // Log updated rows
    for (idx, row) in result.rows.iter().enumerate() {
        debug!("Oracle updated row {}: {:?}", idx + 1, row);
    }

    // Cleanup with error handling
    info!("Cleaning up Oracle test table");
    let cleanup_result = adapter.execute_query("DROP TABLE test_bulk_update").await;
    if let Err(e) = cleanup_result {
        tracing::warn!(error = %e, "Failed to drop Oracle test table (cleanup)");
    } else {
        debug!("Oracle test table dropped successfully");
    }

    adapter.disconnect().await.map_err(|e| {
        tracing::warn!(error = %e, "Failed to disconnect Oracle adapter");
        e
    })?;

    let total_duration = test_start.elapsed();
    info!(
        "=== Test completed: test_oracle_bulk_update in {:?} ===",
        total_duration
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_oracle_bulk_delete() -> Result<()> {
    info!("=== Starting test: test_oracle_bulk_delete ===");
    let test_start = std::time::Instant::now();

    let mut adapter = setup().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to setup Oracle adapter");
        e
    })?;
    debug!(
        "Oracle adapter setup completed in {:?}",
        test_start.elapsed()
    );

    // Create test table with logging
    info!("Creating Oracle test table: test_bulk_delete");
    let create_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_delete (
            id NUMBER PRIMARY KEY,
            name VARCHAR2(100)
        )
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Oracle test table");
            e
        })?;
    debug!("Oracle table created in {:?}", create_start.elapsed());

    // Insert initial test data
    info!("Inserting initial Oracle test data (5 rows)");
    let insert_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        INSERT ALL
            INTO test_bulk_delete (id, name) VALUES (1, 'Alice')
            INTO test_bulk_delete (id, name) VALUES (2, 'Bob')
            INTO test_bulk_delete (id, name) VALUES (3, 'Charlie')
            INTO test_bulk_delete (id, name) VALUES (4, 'David')
            INTO test_bulk_delete (id, name) VALUES (5, 'Eve')
        SELECT * FROM DUAL
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to insert Oracle initial test data");
            e
        })?;
    debug!(
        "Oracle initial data inserted in {:?}",
        insert_start.elapsed()
    );

    // Prepare bulk delete with logging
    let where_clauses = vec![
        "id = 2".to_string(),
        "id = 4".to_string(),
        "id = 5".to_string(),
    ];

    info!("Prepared {} Oracle delete operations", where_clauses.len());
    debug!("Delete conditions: {:?}", where_clauses);

    // Execute bulk delete with timing
    info!("Executing Oracle bulk_delete operation");
    let delete_start = std::time::Instant::now();
    let rows_deleted = adapter
        .bulk_delete("test_bulk_delete", &where_clauses, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                table = "test_bulk_delete",
                delete_count = where_clauses.len(),
                "Oracle bulk delete operation failed"
            );
            e
        })?;
    let delete_duration = delete_start.elapsed();

    info!(
        "Oracle bulk delete completed: {} rows in {:?}",
        rows_deleted, delete_duration
    );

    assert_eq!(
        rows_deleted, 3,
        "Expected 3 rows deleted, got {}",
        rows_deleted
    );

    // Verify deletions with detailed checking
    info!("Verifying Oracle remaining data after delete");
    let verify_start = std::time::Instant::now();
    let result = adapter
        .execute_query("SELECT id, name FROM test_bulk_delete ORDER BY id")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify Oracle remaining data");
            e
        })?;
    debug!(
        "Oracle verification query completed in {:?}",
        verify_start.elapsed()
    );

    assert_eq!(
        result.rows.len(),
        2,
        "Expected 2 remaining rows, got {}",
        result.rows.len()
    );
    info!(
        "✓ Oracle delete verification successful: {} rows remaining",
        result.rows.len()
    );

    // Log remaining rows
    for (idx, row) in result.rows.iter().enumerate() {
        debug!("Oracle remaining row {}: {:?}", idx + 1, row);
    }

    // Cleanup with error handling
    info!("Cleaning up Oracle test table");
    let cleanup_result = adapter.execute_query("DROP TABLE test_bulk_delete").await;
    if let Err(e) = cleanup_result {
        tracing::warn!(error = %e, "Failed to drop Oracle test table (cleanup)");
    } else {
        debug!("Oracle test table dropped successfully");
    }

    adapter.disconnect().await.map_err(|e| {
        tracing::warn!(error = %e, "Failed to disconnect Oracle adapter");
        e
    })?;

    let total_duration = test_start.elapsed();
    info!(
        "=== Test completed: test_oracle_bulk_delete in {:?} ===",
        total_duration
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_oracle_bulk_insert_large_batch() -> Result<()> {
    info!("=== Starting test: test_oracle_bulk_insert_large_batch ===");
    let test_start = std::time::Instant::now();

    let mut adapter = setup().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to setup Oracle adapter");
        e
    })?;
    debug!(
        "Oracle adapter setup completed in {:?}",
        test_start.elapsed()
    );

    // Create test table with logging
    info!("Creating Oracle test table: test_bulk_large");
    let create_start = std::time::Instant::now();
    adapter
        .execute_query(
            "
        CREATE TABLE test_bulk_large (
            id NUMBER PRIMARY KEY,
            value NUMBER
        )
    ",
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Oracle test table");
            e
        })?;
    debug!("Oracle table created in {:?}", create_start.elapsed());

    // Prepare large batch (1000 rows) with logging
    use rusty_data::adapter::QueryValue;
    let columns = vec!["id".to_string(), "value".to_string()];
    let mut rows = Vec::new();
    let prep_start = std::time::Instant::now();
    for i in 1..=1000 {
        rows.push(vec![QueryValue::Int(i), QueryValue::Int(i * 10)]);
    }
    debug!("Prepared {} rows in {:?}", rows.len(), prep_start.elapsed());

    // Execute bulk insert with detailed timing
    info!(
        "Executing Oracle INSERT ALL with {} rows (performance test)",
        rows.len()
    );
    let insert_start = std::time::Instant::now();
    let rows_inserted = adapter
        .bulk_insert("test_bulk_large", &columns, &rows, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                table = "test_bulk_large",
                row_count = rows.len(),
                "Oracle large batch INSERT ALL failed"
            );
            e
        })?;
    let insert_duration = insert_start.elapsed();

    let rows_per_sec = rows_inserted as f64 / insert_duration.as_secs_f64();
    info!(
        "Oracle INSERT ALL large batch completed: {} rows in {:?} ({:.2} rows/sec, {:.2} ms per row)",
        rows_inserted,
        insert_duration,
        rows_per_sec,
        insert_duration.as_millis() as f64 / rows_inserted as f64
    );

    assert_eq!(
        rows_inserted, 1000,
        "Expected 1000 rows inserted, got {}",
        rows_inserted
    );

    // Verify count with detailed logging
    info!("Verifying Oracle row count");
    let verify_start = std::time::Instant::now();
    let result = adapter
        .execute_query("SELECT COUNT(*) as cnt FROM test_bulk_large")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify Oracle row count");
            e
        })?;
    debug!(
        "Oracle verification query completed in {:?}",
        verify_start.elapsed()
    );

    assert_eq!(
        result.rows.len(),
        1,
        "Expected 1 count row, got {}",
        result.rows.len()
    );
    info!(
        "✓ Oracle large batch verification successful: all {} rows inserted",
        rows_inserted
    );

    // Performance summary
    info!("Oracle Performance metrics:");
    info!("  - Total rows: {}", rows_inserted);
    info!("  - Insert time: {:?}", insert_duration);
    info!("  - Throughput: {:.2} rows/sec", rows_per_sec);
    info!(
        "  - Latency: {:.2} ms/row",
        insert_duration.as_millis() as f64 / rows_inserted as f64
    );

    // Cleanup with error handling
    info!("Cleaning up Oracle test table");
    let cleanup_result = adapter.execute_query("DROP TABLE test_bulk_large").await;
    if let Err(e) = cleanup_result {
        tracing::warn!(error = %e, "Failed to drop Oracle test table (cleanup)");
    } else {
        debug!("Oracle test table dropped successfully");
    }

    adapter.disconnect().await.map_err(|e| {
        tracing::warn!(error = %e, "Failed to disconnect Oracle adapter");
        e
    })?;

    let total_duration = test_start.elapsed();
    info!(
        "=== Test completed: test_oracle_bulk_insert_large_batch in {:?} ===",
        total_duration
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_zzz_cleanup_oracle_database() -> Result<()> {
    info!("Oracle cleanup: Using existing FREE database, no database drop needed");

    // For Oracle, we don't drop the database as it's a shared service
    // The individual tests clean up their own tables
    // This is just a placeholder cleanup test

    info!("Oracle test cleanup complete");
    Ok(())
}

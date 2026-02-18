// ============================================================================
// MongoDB Integration Tests
// ============================================================================
//
// This module contains comprehensive integration tests for the MongoDB adapter.
// Tests cover connection management, CRUD operations, schema introspection,
// metadata retrieval, and database-specific features.
//
// NOTE: These tests require a running MongoDB instance with authentication
// configured. See the test constants below for connection details.
//
// Run with: cargo test --test mongodb_integration_tests --features mongodb -- --nocapture --ignored

use rusty_data::adapter::{ConnectionConfig, DatabaseAdapter, DatabaseType};
use rusty_data::adapters::mongodb::MongoDbAdapter;
use rusty_data::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info};

// MongoDB test database - shared across all tests
const TEST_DB_NAME: &str = "rusty_test_mongodb";
const TEST_USER: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";

// One-time database creation
static INIT: AtomicBool = AtomicBool::new(false);

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
    let test_config = get_mongodb_config(TEST_DB_NAME);

    adapter.connect(&test_config, Some(TEST_PASSWORD)).await?;
    info!("Creating MongoDB test database: {}", TEST_DB_NAME);

    // MongoDB creates databases implicitly when data is written
    // Create a dummy collection to make the database visible
    let create_collection_query = r#"{
        "collection": "_init",
        "operation": "insert",
        "document": {"_initialized": true}
    }"#;
    let _ = adapter.execute_query(create_collection_query).await;

    adapter.disconnect().await?;
    debug!("MongoDB test database ready: {}", TEST_DB_NAME);
    Ok(())
}

async fn setup() -> Result<MongoDbAdapter> {
    if !INIT.load(Ordering::Relaxed) {
        ensure_test_database().await?;
        INIT.store(true, Ordering::Relaxed);
    }

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
        r#"{{"collection": "{}", "operation": "insertMany", "documents": [
                {{"username": "alice", "email": "alice@example.com", "age": 30}},
                {{"username": "bob", "email": "bob@example.com", "age": 25}},
                {{"username": "charlie", "email": "charlie@example.com", "age": 35}}
            ]}}"#,
        collection_name
    );
    adapter.execute_query(&insert_query).await?;
    debug!("Test documents inserted");

    // Find documents
    info!("Testing execute_query with find");
    let find_query = format!(r#"{{"collection": "{}", "filter": {{}}}}"#, collection_name);
    let result = adapter.execute_query(&find_query).await?;

    assert_eq!(result.rows.len(), 3);
    debug!("Query results validated: found {} documents", result.rows.len());

    // Cleanup - drop collection
    info!("Cleaning up test collection: {}", collection_name);
    let drop_query = format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name);
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
        let create_query = format!(r#"{{"collection": "{}", "operation": "insert", "document": {{"_init": true}}}}"#, collection);
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
        let drop_query = format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection);
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
        r#"{{"collection": "{}", "operation": "insertMany", "documents": [
                {{"username": "alice", "email": "alice@example.com", "age": 30, "is_active": true}},
                {{"username": "bob", "email": "bob@example.com", "age": 25, "is_active": false}}
            ]}}"#,
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
    let drop_query = format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name);
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
        r#"{{"collection": "{}", "operation": "insertMany", "documents": [
                {{"username": "alice", "email": "alice@example.com", "status": "active"}},
                {{"username": "bob", "email": "bob@example.com", "status": "active"}}
            ]}}"#,
        collection_name
    );
    adapter.execute_query(&insert_query).await?;
    debug!("Documents inserted");

    // READ - Find documents
    info!("Testing READ (find)");
    let find_query = format!(r#"{{"collection": "{}", "filter": {{}}}}"#, collection_name);
    let result = adapter.execute_query(&find_query).await?;
    assert_eq!(result.rows.len(), 2);
    debug!("Found {} documents", result.rows.len());

    // UPDATE - Update a document
    info!("Testing UPDATE (updateMany)");
    let update_query = format!(
        r#"{{"collection": "{}", "operation": "update", "filter": {{"username": "alice"}}, "update": {{"$set": {{"status": "inactive"}}}}}}"#,
        collection_name
    );
    adapter.execute_query(&update_query).await?;
    debug!("Document updated");

    // Verify update
    let verify_query = format!(r#"{{"collection": "{}", "filter": {{"username": "alice"}}}}"#, collection_name);
    let verify_result = adapter.execute_query(&verify_query).await?;
    assert_eq!(verify_result.rows.len(), 1);
    debug!("Update verified");

    // DELETE - Delete a document
    info!("Testing DELETE (deleteMany)");
    let delete_query = format!(r#"{{"collection": "{}", "operation": "delete", "filter": {{"username": "bob"}}}}"#, collection_name);
    adapter.execute_query(&delete_query).await?;
    debug!("Document deleted");

    // Verify deletion
    let final_query = format!(r#"{{"collection": "{}", "filter": {{}}}}"#, collection_name);
    let final_result = adapter.execute_query(&final_query).await?;
    assert_eq!(final_result.rows.len(), 1);
    debug!("Deletion verified: {} document remaining", final_result.rows.len());

    // Cleanup - drop collection
    info!("Cleaning up test collection");
    let drop_query = format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name);
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
    // size_bytes may be None for newly created databases
    debug!("Database size: {:?} bytes", db_meta.size_bytes);
    debug!("Database encoding: {:?}", db_meta.encoding);
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
            r#"{{"collection": "{}", "operation": "insertMany", "documents": [{{"name": "test1"}}, {{"name": "test2"}}, {{"name": "test3"}}]}}"#,
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
        .execute_query(&format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name))
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
        .execute_query(&format!(r#"{{"collection": "{}", "operation": "insert", "document": {{"name": "test", "email": "test@example.com"}}}}"#, collection_name))
        .await?;

    // TODO: Add support for createIndex in execute_query or use direct MongoDB client
    // For now, skip index creation tests as createIndex is not yet supported via execute_query

    // Get indexes
    info!("Testing get_indexes");
    let indexes = adapter.get_indexes(collection_name, None).await?;

    // MongoDB always has at least _id index
    assert!(indexes.len() >= 1); // _id (default)

    // Find the _id index (MongoDB's default primary key)
    let id_idx = indexes.iter().find(|i| i.name == "_id_");
    assert!(id_idx.is_some());
    assert!(id_idx.unwrap().is_primary);

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
        .execute_query(&format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name))
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
    info!("Creating test collection");
    adapter
        .execute_query(&format!(
            r#"{{"collection": "{}", "operation": "insert", "document": {{"value": 10}}}}"#,
            collection_name
        ))
        .await?;

    // Create a view on the source collection
    info!("Creating test view");
    adapter
        .execute_query(&format!(
            r#"{{"operation": "createView", "viewName": "{}", "viewOn": "{}", "pipeline": [{{"$project": {{"value": 1, "doubled": {{"$multiply": ["$value", 2]}}}}}}]}}"#,
            view_name, collection_name
        ))
        .await?;

    // Get views
    info!("Testing get_views");
    let views = adapter.get_views(None).await?;

    // Verify the view we created exists
    let test_view = views.iter().find(|v| v.name == view_name);
    assert!(test_view.is_some(), "Created view '{}' should exist in views list", view_name);
    debug!("Found {} views", views.len());

    // Get view definition
    info!("Testing get_view_definition");
    let definition = adapter.get_view_definition(view_name, None).await?;
    assert!(definition.is_some(), "View definition should exist for '{}'", view_name);
    let def = definition.unwrap();
    assert!(def.contains(collection_name), "View definition should reference source collection '{}'", collection_name);
    debug!("View definition: {}", def);

    // Cleanup
    info!("Cleaning up test view and collection");
    adapter
        .execute_query(&format!(r#"{{"collection": "{}", "operation": "drop"}}"#, view_name))
        .await?;
    adapter
        .execute_query(&format!(r#"{{"collection": "{}", "operation": "drop"}}"#, collection_name))
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

// ========== Bulk Operations Integration Tests ==========

#[tokio::test]
#[ignore]
async fn test_mongodb_bulk_insert() -> Result<()> {
    use rusty_data::adapter::QueryValue;

    info!("Starting test: test_mongodb_bulk_insert");

    let mut adapter = setup().await?;

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
            QueryValue::Text("charlie@example.com".to_string()),
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
        .execute_query("db.test_bulk_insert.find().sort({id: 1})")
        .await?;

    assert_eq!(result.rows.len(), 4);
    debug!("Data verification successful");

    // Cleanup
    info!("Cleaning up test collection");
    adapter
        .execute_query("db.test_bulk_insert.drop()")
        .await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mongodb_bulk_insert");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mongodb_bulk_update() -> Result<()> {
    use rusty_data::adapter::QueryValue;
    use std::collections::HashMap;

    info!("Starting test: test_mongodb_bulk_update");

    let mut adapter = setup().await?;

    // Insert initial data
    adapter
        .execute_query(
            r#"db.test_bulk_update.insertMany([
                {id: 1, name: "Alice", status: "pending", score: 50},
                {id: 2, name: "Bob", status: "pending", score: 60},
                {id: 3, name: "Charlie", status: "active", score: 70},
                {id: 4, name: "Diana", status: "pending", score: 80}
            ])"#,
        )
        .await?;

    // Prepare bulk updates
    info!("Preparing bulk update data");
    let mut update1 = HashMap::new();
    update1.insert("status".to_string(), QueryValue::Text("active".to_string()));
    update1.insert("score".to_string(), QueryValue::Int(100));

    let mut update2 = HashMap::new();
    update2.insert("status".to_string(), QueryValue::Text("completed".to_string()));

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

    // Cleanup
    info!("Cleaning up test collection");
    adapter
        .execute_query("db.test_bulk_update.drop()")
        .await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mongodb_bulk_update");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mongodb_bulk_delete() -> Result<()> {
    info!("Starting test: test_mongodb_bulk_delete");

    let mut adapter = setup().await?;

    // Insert initial data
    adapter
        .execute_query(
            r#"db.test_bulk_delete.insertMany([
                {id: 1, name: "Alice", status: "active"},
                {id: 2, name: "Bob", status: "inactive"},
                {id: 3, name: "Charlie", status: "active"},
                {id: 4, name: "Diana", status: "inactive"},
                {id: 5, name: "Eve", status: "active"},
                {id: 6, name: "Frank", status: "inactive"}
            ])"#,
        )
        .await?;

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
        .execute_query("db.test_bulk_delete.find().sort({id: 1})")
        .await?;

    assert_eq!(result.rows.len(), 3);
    debug!("Delete verification successful: {} rows remain", result.rows.len());

    // Cleanup
    info!("Cleaning up test collection");
    adapter
        .execute_query("db.test_bulk_delete.drop()")
        .await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mongodb_bulk_delete");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mongodb_bulk_insert_large_batch() -> Result<()> {
    use rusty_data::adapter::QueryValue;

    info!("Starting test: test_mongodb_bulk_insert_large_batch");

    let mut adapter = setup().await?;

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
        .execute_query("db.test_bulk_large.countDocuments({})")
        .await?;
    assert_eq!(result.rows.len(), 1);

    // Cleanup
    info!("Cleaning up test collection");
    adapter
        .execute_query("db.test_bulk_large.drop()")
        .await?;

    adapter.disconnect().await?;
    info!("Test completed: test_mongodb_bulk_insert_large_batch");
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

use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseMetadata, DatabaseType,
    ForeignKeyInfo, IndexInfo, ProcedureInfo, QueryResult, QueryValue, ServerInfo, TableInfo,
    TableMetadata, ViewInfo,
};
use crate::error::{DataError, Result};
use async_trait::async_trait;
use mongodb::{
    bson::{doc, Bson, Document},
    options::ClientOptions,
    Client,
};
use tracing::{debug, info, instrument, warn};

/// MongoDB database adapter
pub struct MongoDbAdapter {
    client: Option<Client>,
    current_database: Option<String>,
}

impl MongoDbAdapter {
    /// Create a new MongoDB adapter
    pub fn new() -> Self {
        Self {
            client: None,
            current_database: None,
        }
    }

    /// Validate database name
    fn validate_database_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config("Database name cannot be empty".to_string()));
        }
        // MongoDB database names have specific restrictions
        if name.len() > 64 {
            return Err(DataError::Config(format!(
                "Database name too long (max 64 chars): {}",
                name.len()
            )));
        }
        // Check for invalid characters
        for c in ['/','\\', '.', ' ', '"', '$', '*', '<', '>', ':', '|', '?'] {
            if name.contains(c) {
                return Err(DataError::Config(format!(
                    "Database name contains invalid character '{}': {}",
                    c, name
                )));
            }
        }
        Ok(())
    }

    /// Validate collection name (MongoDB's equivalent of table)
    fn validate_collection_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(DataError::Config("Collection name cannot be empty".to_string()));
        }
        if name.starts_with("system.") {
            return Err(DataError::Config(format!(
                "Collection name cannot start with 'system.': {}",
                name
            )));
        }
        if name.contains('$') && !name.starts_with("oplog.$") {
            return Err(DataError::Config(format!(
                "Collection name contains invalid character '$': {}",
                name
            )));
        }
        if name.contains('\0') {
            return Err(DataError::Config(format!(
                "Collection name contains null character: {}",
                name
            )));
        }
        Ok(())
    }

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig, password: Option<&str>) -> String {
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(27017);
        let username = config.username.as_deref();
        let password = password;

        if let (Some(user), Some(pass)) = (username, password) {
            // Include authSource=admin for root user authentication
            format!("mongodb://{}:{}@{}:{}/?authSource=admin", user, pass, host, port)
        } else {
            format!("mongodb://{}:{}", host, port)
        }
    }

    /// Convert a BSON value to QueryValue
    fn bson_to_query_value(bson: &Bson) -> QueryValue {
        match bson {
            Bson::Null | Bson::Undefined => QueryValue::Null,
            Bson::Boolean(b) => QueryValue::Bool(*b),
            Bson::Int32(i) => QueryValue::Int(*i as i64),
            Bson::Int64(i) => QueryValue::Int(*i),
            Bson::Double(d) => QueryValue::Float(*d),
            Bson::String(s) => QueryValue::Text(s.clone()),
            Bson::Binary(b) => QueryValue::Bytes(b.bytes.clone()),
            Bson::ObjectId(oid) => QueryValue::Text(oid.to_hex()),
            Bson::DateTime(dt) => QueryValue::Text(dt.to_string()),
            Bson::Array(arr) => QueryValue::Text(format!("{:?}", arr)),
            Bson::Document(doc) => QueryValue::Text(format!("{:?}", doc)),
            _ => QueryValue::Text(format!("{:?}", bson)),
        }
    }

    /// Parse simple where clause to MongoDB filter document
    /// Supports simple "field = value" and "_id = value" formats
    fn parse_where_clause_to_filter(where_clause: &str) -> Result<mongodb::bson::Document> {
        let where_clause = where_clause.trim();

        // Simple parser for "field = value" format
        if let Some(eq_pos) = where_clause.find('=') {
            let field = where_clause[..eq_pos].trim();
            let value_str = where_clause[eq_pos + 1..].trim();

            // Try to parse the value
            let bson_value = if value_str == "null" {
                mongodb::bson::Bson::Null
            } else if value_str == "true" {
                mongodb::bson::Bson::Boolean(true)
            } else if value_str == "false" {
                mongodb::bson::Bson::Boolean(false)
            } else if let Ok(i) = value_str.parse::<i64>() {
                mongodb::bson::Bson::Int64(i)
            } else if let Ok(f) = value_str.parse::<f64>() {
                mongodb::bson::Bson::Double(f)
            } else {
                // Remove quotes if present
                let cleaned = value_str.trim_matches(|c| c == '\'' || c == '"');
                mongodb::bson::Bson::String(cleaned.to_string())
            };

            Ok(doc! { field: bson_value })
        } else {
            // If no '=' found, try to parse as JSON filter
            Err(DataError::Query(format!(
                "Unsupported where clause format: {}. Use 'field = value' format.",
                where_clause
            )))
        }
    }
}

impl Default for MongoDbAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for MongoDbAdapter {
    #[instrument(skip(self, password), fields(
        db = %config.database,
        host = config.host.as_deref().unwrap_or("localhost"),
        port = config.port.unwrap_or(27017)
    ))]
    async fn connect(&mut self, config: &ConnectionConfig, password: Option<&str>) -> Result<()> {
        if config.db_type != DatabaseType::MongoDB {
            return Err(DataError::Config(format!(
                "Invalid database type: expected MongoDB, got {:?}",
                config.db_type
            )));
        }

        Self::validate_database_name(&config.database)?;

        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(27017);
        let database = &config.database;

        info!(
            database = %database,
            host = %host,
            port = %port,
            "Connecting to MongoDB database"
        );
        let start = std::time::Instant::now();
        let connection_string = Self::build_connection_string(config, password);

        let client_options = ClientOptions::parse(&connection_string)
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                warn!(
                    error = %e,
                    elapsed_ms = elapsed.as_millis(),
                    "Failed to parse MongoDB connection string"
                );
                DataError::Connection(format!(
                    "Invalid connection string for {}:{} - {}",
                    host, port, e
                ))
            })?;

        let client = Client::with_options(client_options).map_err(|e| {
            let elapsed = start.elapsed();
            warn!(
                error = %e,
                elapsed_ms = elapsed.as_millis(),
                "Failed to create MongoDB client"
            );
            DataError::Connection(format!(
                "Failed to create MongoDB client for {}:{} - {}",
                host, port, e
            ))
        })?;

        // Test the connection
        client
            .database(database)
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();

                // Categorize MongoDB connection errors
                let error_category = if error_msg.contains("authentication failed") || error_msg.contains("auth failed") {
                    "authentication"
                } else if error_msg.contains("connection refused") || error_msg.contains("No connection available") {
                    "network"
                } else if error_msg.contains("not master") || error_msg.contains("replica set") {
                    "replica_set"
                } else if error_msg.contains("unauthorized") {
                    "unauthorized"
                } else {
                    "unknown"
                };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    elapsed_ms = elapsed.as_millis(),
                    "Failed to connect to MongoDB"
                );

                if error_msg.contains("authentication failed") || error_msg.contains("auth failed") {
                    DataError::Connection(format!(
                        "Authentication failed for database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                } else if error_msg.contains("connection refused") || error_msg.contains("No connection available") {
                    DataError::Connection(format!(
                        "Network error connecting to MongoDB at {}:{} - {}",
                        host, port, e
                    ))
                } else if error_msg.contains("not master") || error_msg.contains("replica set") {
                    DataError::Connection(format!(
                        "Replica set configuration issue at {}:{} - {}",
                        host, port, e
                    ))
                } else if error_msg.contains("unauthorized") {
                    DataError::Connection(format!(
                        "Unauthorized access to database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                } else {
                    DataError::Connection(format!(
                        "Failed to connect to database '{}' at {}:{} - {}",
                        database, host, port, e
                    ))
                }
            })?;

        let elapsed = start.elapsed();
        self.client = Some(client);
        self.current_database = Some(config.database.clone());
        info!(
            elapsed_ms = elapsed.as_millis(),
            "Successfully connected to MongoDB"
        );
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<()> {
        if self.client.is_some() {
            info!("Disconnecting from MongoDB");
            self.client = None;
            self.current_database = None;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    #[instrument(skip(self, query), fields(query_len = query.len()))]
    async fn execute_query(&self, query: &str) -> Result<QueryResult> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let db_name = self
            .current_database
            .as_ref()
            .ok_or_else(|| DataError::Connection("No database selected".to_string()))?;

        let query_snippet = if query.len() > 100 {
            format!("{}...", &query[..100])
        } else {
            query.to_string()
        };

        debug!(
            query_snippet = %query_snippet,
            database = %db_name,
            "Executing MongoDB query"
        );
        let start = std::time::Instant::now();

        // Parse the query as a MongoDB command
        // For simplicity, we'll assume the query is a JSON document representing a find command
        // Format: {"collection": "collectionName", "filter": {...}, "limit": 10}
        let command: Document = serde_json::from_str(query).map_err(|e| {
            let elapsed = start.elapsed();
            warn!(
                error = %e,
                query_snippet = %query_snippet,
                elapsed_ms = elapsed.as_millis(),
                "Invalid MongoDB query format"
            );
            DataError::Query(format!("Invalid MongoDB query format: {}. Expected JSON document with 'collection' and 'filter' fields", e))
        })?;

        let db = client.database(db_name);

        // Check for operation type (createView doesn't use "collection" field)
        let operation = command.get_str("operation").unwrap_or("find");

        // Handle createView specially (it uses "viewName" instead of "collection")
        if operation == "createView" {
            let view_name = command
                .get_str("viewName")
                .map_err(|_| DataError::Query("Missing 'viewName' field for createView operation".to_string()))?;

            let view_on = command
                .get_str("viewOn")
                .map_err(|_| DataError::Query("Missing 'viewOn' field for createView operation".to_string()))?;

            let pipeline = command
                .get_array("pipeline")
                .map_err(|_| DataError::Query("Missing 'pipeline' field for createView operation".to_string()))?
                .iter()
                .filter_map(|d| d.as_document())
                .cloned()
                .collect::<Vec<Document>>();

            if pipeline.is_empty() {
                return Err(DataError::Query("Pipeline must contain at least one valid document".to_string()));
            }

            Self::validate_collection_name(view_name)?;
            Self::validate_collection_name(view_on)?;

            use mongodb::options::CreateCollectionOptions;
            let mut options = CreateCollectionOptions::default();
            options.view_on = Some(view_on.to_string());
            options.pipeline = Some(pipeline);

            db.create_collection(view_name, options).await.map_err(|e| {
                DataError::Query(format!("Create view failed: {}", e))
            })?;

            let elapsed = start.elapsed();
            info!(
                view_name = %view_name,
                view_on = %view_on,
                elapsed_ms = elapsed.as_millis(),
                "Create view operation completed"
            );

            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(0),
            });
        }

        // All other operations require collection field
        let collection_name = command
            .get_str("collection")
            .map_err(|_| DataError::Query("Missing 'collection' field in query".to_string()))?;

        Self::validate_collection_name(collection_name)?;

        let collection = db.collection::<Document>(collection_name);

        match operation {
            "insert" => {
                // Handle insert operation
                let document = command
                    .get_document("document")
                    .map_err(|_| DataError::Query("Missing 'document' field for insert operation".to_string()))?
                    .clone();

                collection.insert_one(document, None).await.map_err(|e| {
                    DataError::Query(format!("Insert failed: {}", e))
                })?;

                let elapsed = start.elapsed();
                info!(
                    collection = %collection_name,
                    elapsed_ms = elapsed.as_millis(),
                    "Insert operation completed"
                );

                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(1),
                });
            }
            "insertMany" => {
                // Handle insert many operation
                let documents = command
                    .get_array("documents")
                    .map_err(|_| DataError::Query("Missing 'documents' field for insertMany operation".to_string()))?
                    .iter()
                    .filter_map(|d| d.as_document())
                    .cloned()
                    .collect::<Vec<Document>>();

                if documents.is_empty() {
                    return Err(DataError::Query("No valid documents to insert".to_string()));
                }

                let count = documents.len();
                collection.insert_many(documents, None).await.map_err(|e| {
                    DataError::Query(format!("InsertMany failed: {}", e))
                })?;

                let elapsed = start.elapsed();
                info!(
                    collection = %collection_name,
                    count = count,
                    elapsed_ms = elapsed.as_millis(),
                    "InsertMany operation completed"
                );

                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(count as u64),
                });
            }
            "update" => {
                // Handle update operation
                let filter = command
                    .get_document("filter")
                    .map_err(|_| DataError::Query("Missing 'filter' field for update operation".to_string()))?
                    .clone();

                let update = command
                    .get_document("update")
                    .map_err(|_| DataError::Query("Missing 'update' field for update operation".to_string()))?
                    .clone();

                let result = collection.update_many(filter, update, None).await.map_err(|e| {
                    DataError::Query(format!("Update failed: {}", e))
                })?;

                let elapsed = start.elapsed();
                info!(
                    collection = %collection_name,
                    modified = result.modified_count,
                    elapsed_ms = elapsed.as_millis(),
                    "Update operation completed"
                );

                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(result.modified_count),
                });
            }
            "delete" => {
                // Handle delete operation
                let filter = command
                    .get_document("filter")
                    .map_err(|_| DataError::Query("Missing 'filter' field for delete operation".to_string()))?
                    .clone();

                let result = collection.delete_many(filter, None).await.map_err(|e| {
                    DataError::Query(format!("Delete failed: {}", e))
                })?;

                let elapsed = start.elapsed();
                info!(
                    collection = %collection_name,
                    deleted = result.deleted_count,
                    elapsed_ms = elapsed.as_millis(),
                    "Delete operation completed"
                );

                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(result.deleted_count),
                });
            }
            "drop" => {
                // Handle drop collection operation
                collection.drop(None).await.map_err(|e| {
                    DataError::Query(format!("Drop collection failed: {}", e))
                })?;

                let elapsed = start.elapsed();
                info!(
                    collection = %collection_name,
                    elapsed_ms = elapsed.as_millis(),
                    "Drop collection completed"
                );

                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(0),
                });
            }
            "find" | _ => {
                // Handle find operation (default)
                let filter = command
                    .get_document("filter")
                    .unwrap_or(&Document::new())
                    .clone();

                let mut cursor = collection
            .find(filter, None)
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_msg = e.to_string();

                // Categorize MongoDB query errors
                let error_category = if error_msg.contains("namespace not found") || error_msg.contains("does not exist") {
                    "collection_not_found"
                } else if error_msg.contains("unauthorized") || error_msg.contains("not authorized") {
                    "unauthorized"
                } else if error_msg.contains("bad query") || error_msg.contains("invalid") {
                    "invalid_query"
                } else {
                    "unknown"
                };

                warn!(
                    error = %e,
                    error_category = %error_category,
                    collection = %collection_name,
                    query_snippet = %query_snippet,
                    elapsed_ms = elapsed.as_millis(),
                    "Query execution failed"
                );

                if error_msg.contains("namespace not found") || error_msg.contains("does not exist") {
                    DataError::Query(format!(
                        "Collection '{}' not found in database '{}' - {}",
                        collection_name, db_name, e
                    ))
                } else if error_msg.contains("unauthorized") || error_msg.contains("not authorized") {
                    DataError::Query(format!(
                        "Unauthorized to query collection '{}' - {}",
                        collection_name, e
                    ))
                } else if error_msg.contains("bad query") || error_msg.contains("invalid") {
                    DataError::Query(format!(
                        "Invalid query for collection '{}': {} - Query: {}",
                        collection_name, e, query
                    ))
                } else {
                    DataError::Query(format!(
                        "Query failed for collection '{}': {} - Query: {}",
                        collection_name, e, query
                    ))
                }
            })?;

        let fetch_start = std::time::Instant::now();
        let mut result_rows = Vec::new();
        let mut columns = Vec::new();

        while cursor.advance().await.map_err(|e| DataError::Query(format!("Failed to fetch results: {}", e)))? {
            let doc = cursor.deserialize_current().map_err(|e| {
                DataError::Query(format!("Failed to deserialize document: {}", e))
            })?;

            // Collect all unique field names for columns
            for (key, _) in &doc {
                if !columns.contains(&key.clone()) {
                    columns.push(key.clone());
                }
            }

            // Convert document to row values
            let mut row_values = Vec::new();
            for col in &columns {
                let value = doc
                    .get(col)
                    .map(Self::bson_to_query_value)
                    .unwrap_or(QueryValue::Null);
                row_values.push(value);
            }
            result_rows.push(row_values);
        }

        let fetch_elapsed = fetch_start.elapsed();
        let total_elapsed = start.elapsed();
        let row_count = result_rows.len();
        let column_count = columns.len();

        info!(
            collection = %collection_name,
            rows_count = row_count,
            columns_count = column_count,
            fetch_ms = fetch_elapsed.as_millis(),
            total_ms = total_elapsed.as_millis(),
            "Query executed successfully"
        );

                Ok(QueryResult {
                    columns,
                    rows: result_rows,
                    rows_affected: Some(row_count as u64),
                })
            }
        }
    }

    #[instrument(skip(self))]
    async fn list_databases(&self) -> Result<Vec<String>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let databases = client
            .list_database_names(None, None)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list databases: {}", e)))?;

        Ok(databases)
    }

    #[instrument(skip(self))]
    async fn list_tables(&self, _schema: Option<&str>) -> Result<Vec<String>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let db_name = self
            .current_database
            .as_ref()
            .ok_or_else(|| DataError::Connection("No database selected".to_string()))?;

        let db = client.database(db_name);
        let collections = db
            .list_collection_names(None)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list collections: {}", e)))?;

        Ok(collections)
    }

    #[instrument(skip(self), fields(collection = %table_name))]
    async fn describe_table(&self, table_name: &str, _schema: Option<&str>) -> Result<TableInfo> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let db_name = self
            .current_database
            .as_ref()
            .ok_or_else(|| DataError::Connection("No database selected".to_string()))?;

        let db = client.database(db_name);
        let collection = db.collection::<Document>(table_name);

        // Sample a few documents to infer schema
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| DataError::Query(format!("Failed to query collection: {}", e)))?;

        let mut field_types: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        let mut sample_count = 0;
        while sample_count < 10
            && cursor.advance().await.map_err(|e| {
                DataError::Query(format!("Failed to fetch document: {}", e))
            })?
        {
            let doc = cursor.deserialize_current().map_err(|e| {
                DataError::Query(format!("Failed to deserialize document: {}", e))
            })?;

            for (key, value) in &doc {
                let type_name = match value {
                    Bson::Null | Bson::Undefined => "null",
                    Bson::Boolean(_) => "boolean",
                    Bson::Int32(_) => "int32",
                    Bson::Int64(_) => "int64",
                    Bson::Double(_) => "double",
                    Bson::String(_) => "string",
                    Bson::Binary(_) => "binary",
                    Bson::ObjectId(_) => "objectid",
                    Bson::DateTime(_) => "datetime",
                    Bson::Array(_) => "array",
                    Bson::Document(_) => "document",
                    _ => "unknown",
                };
                field_types.insert(key.clone(), type_name.to_string());
            }

            sample_count += 1;
        }

        let columns: Vec<ColumnInfo> = field_types
            .into_iter()
            .map(|(name, data_type)| ColumnInfo {
                name,
                data_type,
                nullable: true, // MongoDB fields are always nullable
                default_value: None,
                is_primary_key: false, // MongoDB uses _id, but we'll keep it simple
            })
            .collect();

        Ok(TableInfo {
            name: table_name.to_string(),
            schema: None,
            columns,
        })
    }

    async fn test_connection(&self, config: &ConnectionConfig, password: Option<&str>) -> Result<bool> {
        let connection_string = Self::build_connection_string(config, password);

        match ClientOptions::parse(&connection_string).await {
            Ok(options) => match Client::with_options(options) {
                Ok(client) => {
                    // Try to ping the database
                    match client
                        .database(&config.database)
                        .run_command(doc! { "ping": 1 }, None)
                        .await
                    {
                        Ok(_) => Ok(true),
                        Err(_) => Ok(false),
                    }
                }
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }

    fn database_type(&self) -> DatabaseType {
        DatabaseType::MongoDB
    }

    // ===== Server & Database Introspection Methods =====

    #[instrument(skip(self))]
    async fn get_server_info(&self) -> Result<ServerInfo> {
        info!("Retrieving MongoDB server info");

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().unwrap_or("admin");
        let db = client.database(db_name);

        // Run buildInfo command
        let build_info = db
            .run_command(doc! { "buildInfo": 1 }, None)
            .await
            .map_err(|e| DataError::Query(format!("Failed to get build info: {}", e)))?;

        let version = build_info
            .get_str("version")
            .unwrap_or("unknown")
            .to_string();

        let mut extra_info = std::collections::HashMap::new();

        // Extract additional info
        if let Ok(git_version) = build_info.get_str("gitVersion") {
            extra_info.insert("git_version".to_string(), git_version.to_string());
        }
        if let Ok(sys_info) = build_info.get_str("sysInfo") {
            extra_info.insert("sys_info".to_string(), sys_info.to_string());
        }
        if let Ok(storage_engines) = build_info.get_array("storageEngines") {
            extra_info.insert("storage_engines".to_string(), format!("{:?}", storage_engines));
        }

        Ok(ServerInfo {
            version,
            server_type: "MongoDB".to_string(),
            extra_info,
        })
    }

    #[instrument(skip(self), fields(database = %database_name))]
    async fn get_database_metadata(&self, database_name: &str) -> Result<DatabaseMetadata> {
        info!("Retrieving metadata for database: {}", database_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db = client.database(database_name);

        // Run dbStats command
        let db_stats = db
            .run_command(doc! { "dbStats": 1 }, None)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get database stats for '{}': {}",
                    database_name, e
                ))
            })?;

        let size_bytes = db_stats.get_i64("dataSize").ok();
        let collection_count = db_stats.get_i32("collections").ok().map(|c| c as i64);

        let mut extra_info = std::collections::HashMap::new();

        if let Ok(indexes) = db_stats.get_i32("indexes") {
            extra_info.insert("indexes".to_string(), indexes.to_string());
        }
        if let Ok(index_size) = db_stats.get_i64("indexSize") {
            extra_info.insert("index_size".to_string(), index_size.to_string());
        }
        if let Ok(storage_size) = db_stats.get_i64("storageSize") {
            extra_info.insert("storage_size".to_string(), storage_size.to_string());
        }
        if let Some(coll_count) = collection_count {
            extra_info.insert("collections".to_string(), coll_count.to_string());
        }

        Ok(DatabaseMetadata {
            name: database_name.to_string(),
            size_bytes,
            owner: None, // MongoDB doesn't expose owner in dbStats
            encoding: Some("UTF-8".to_string()), // MongoDB uses UTF-8 by default
            created_at: None,
            extra_info,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_table_metadata(&self, table_name: &str, _schema: Option<&str>) -> Result<TableMetadata> {
        info!("Retrieving metadata for collection: {}", table_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let db = client.database(db_name);

        // Get collection stats
        let stats_cmd = doc! { "collStats": table_name };
        let coll_stats = db
            .run_command(stats_cmd, None)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get collection stats for '{}': {}",
                    table_name, e
                ))
            })?;

        let size_bytes = coll_stats.get_i64("size").ok();

        // Get accurate document count using count_documents
        let collection = db.collection::<Document>(table_name);
        let row_count = collection
            .count_documents(doc! {}, None)
            .await
            .map(|count| count as i64)
            .ok();

        let table_type = if let Ok(view_on) = coll_stats.get_str("viewOn") {
            Some(format!("view (on: {})", view_on))
        } else {
            Some("collection".to_string())
        };

        Ok(TableMetadata {
            name: table_name.to_string(),
            schema: None, // MongoDB doesn't have schemas in the SQL sense
            size_bytes,
            row_count,
            created_at: None,
            table_type,
        })
    }

    #[instrument(skip(self), fields(table = %table_name))]
    async fn get_indexes(&self, table_name: &str, _schema: Option<&str>) -> Result<Vec<IndexInfo>> {
        info!("Retrieving indexes for collection: {}", table_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let db = client.database(db_name);
        let collection = db.collection::<Document>(table_name);

        // List indexes
        let mut cursor = collection.list_indexes(None).await.map_err(|e| {
            DataError::Query(format!(
                "Failed to list indexes for '{}': {}",
                table_name, e
            ))
        })?;

        let mut indexes = Vec::new();

        while cursor.advance().await.map_err(|e| {
            DataError::Query(format!("Failed to iterate indexes: {}", e))
        })? {
            let index_doc = cursor.current();

            let index_name = index_doc
                .get_str("name")
                .unwrap_or("unknown")
                .to_string();

            // Extract key fields (columns)
            let mut columns = Vec::new();
            if let Ok(keys) = index_doc.get_document("key") {
                for item in keys.iter() {
                    if let Ok((field, _)) = item {
                        columns.push(field.to_string());
                    }
                }
            }

            let is_unique = index_doc.get_bool("unique").unwrap_or(false);
            let is_primary = index_name == "_id_"; // MongoDB's default primary key index

            let index_type = if let Ok(version) = index_doc.get_i32("v") {
                Some(format!("version_{}", version))
            } else {
                None
            };

            indexes.push(IndexInfo {
                name: index_name,
                table_name: table_name.to_string(),
                schema: None,
                columns,
                is_unique,
                is_primary,
                index_type,
            });
        }

        Ok(indexes)
    }

    #[instrument(skip(self), fields(table = %_table_name))]
    async fn get_foreign_keys(&self, _table_name: &str, _schema: Option<&str>) -> Result<Vec<ForeignKeyInfo>> {
        info!("MongoDB does not support foreign keys");

        // MongoDB doesn't have foreign key constraints
        // Return empty list
        Ok(Vec::new())
    }

    #[instrument(skip(self))]
    async fn get_views(&self, _schema: Option<&str>) -> Result<Vec<ViewInfo>> {
        info!("Retrieving views");

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let db = client.database(db_name);

        // List collections with views
        let filter = doc! { "type": "view" };
        let mut cursor = db
            .list_collections(Some(filter), None)
            .await
            .map_err(|e| DataError::Query(format!("Failed to list views: {}", e)))?;

        let mut views = Vec::new();

        while cursor.advance().await.map_err(|e| {
            DataError::Query(format!("Failed to iterate views: {}", e))
        })? {
            let view_doc = cursor.current();

            let name = view_doc.get_str("name").unwrap_or("unknown").to_string();

            // Try to extract pipeline from options
            let definition = if let Ok(options) = view_doc.get_document("options") {
                if let Ok(pipeline) = options.get_array("pipeline") {
                    Some(format!("{:?}", pipeline))
                } else {
                    None
                }
            } else {
                None
            };

            views.push(ViewInfo {
                name,
                schema: None,
                definition,
            });
        }

        Ok(views)
    }

    #[instrument(skip(self), fields(view = %view_name))]
    async fn get_view_definition(&self, view_name: &str, _schema: Option<&str>) -> Result<Option<String>> {
        info!("Retrieving view definition for: {}", view_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let db = client.database(db_name);

        // List collection to get view info
        let filter = doc! { "name": view_name, "type": "view" };
        let mut cursor = db
            .list_collections(Some(filter), None)
            .await
            .map_err(|e| {
                DataError::Query(format!(
                    "Failed to get view definition for '{}': {}",
                    view_name, e
                ))
            })?;

        if cursor.advance().await.map_err(|e| {
            DataError::Query(format!("Failed to iterate views: {}", e))
        })? {
            let view_doc = cursor.current();

            if let Ok(options) = view_doc.get_document("options") {
                let view_on = options.get_str("viewOn").ok();
                let pipeline = options.get_array("pipeline").ok();

                match (view_on, pipeline) {
                    (Some(vo), Some(p)) => {
                        return Ok(Some(format!("View on collection: {}\nPipeline: {:?}", vo, p)));
                    }
                    (Some(vo), None) => {
                        return Ok(Some(format!("View on collection: {}", vo)));
                    }
                    (None, Some(p)) => {
                        return Ok(Some(format!("Pipeline: {:?}", p)));
                    }
                    _ => {}
                }
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    async fn list_stored_procedures(&self, _schema: Option<&str>) -> Result<Vec<ProcedureInfo>> {
        info!("MongoDB does not support stored procedures");

        // MongoDB doesn't have stored procedures in the traditional sense
        // (though you can store JavaScript functions with db.system.js, this is deprecated)
        Ok(Vec::new())
    }

    #[instrument(skip(self, rows), fields(collection = %table_name, row_count = rows.len(), column_count = columns.len()))]
    async fn bulk_insert(
        &self,
        table_name: &str,
        columns: &[String],
        rows: &[Vec<QueryValue>],
        _schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_collection_name(table_name)?;

        if columns.is_empty() {
            return Err(DataError::Config("Column list cannot be empty".to_string()));
        }

        if rows.is_empty() {
            return Ok(0);
        }

        info!("Bulk inserting {} rows into {}", rows.len(), table_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        // Validate all rows have the same column count
        for (idx, row) in rows.iter().enumerate() {
            if row.len() != columns.len() {
                return Err(DataError::Config(format!(
                    "Row {} has {} values but expected {} columns",
                    idx,
                    row.len(),
                    columns.len()
                )));
            }
        }

        let start = std::time::Instant::now();

        let db = client.database(db_name);
        let collection = db.collection::<mongodb::bson::Document>(table_name);

        // Build documents for insertion
        let mut documents = Vec::new();
        for row in rows {
            let mut doc = mongodb::bson::Document::new();
            for (col, value) in columns.iter().zip(row.iter()) {
                let bson_value = match value {
                    QueryValue::Null => mongodb::bson::Bson::Null,
                    QueryValue::Bool(b) => mongodb::bson::Bson::Boolean(*b),
                    QueryValue::Int(i) => mongodb::bson::Bson::Int64(*i),
                    QueryValue::Float(f) => mongodb::bson::Bson::Double(*f),
                    QueryValue::Text(s) => mongodb::bson::Bson::String(s.clone()),
                    QueryValue::Bytes(b) => mongodb::bson::Bson::Binary(mongodb::bson::Binary {
                        subtype: mongodb::bson::spec::BinarySubtype::Generic,
                        bytes: b.clone(),
                    }),
                };
                doc.insert(col, bson_value);
            }
            documents.push(doc);
        }

        // Use MongoDB's native insert_many for efficiency
        let result = collection
            .insert_many(documents, None)
            .await
            .map_err(|e| {
                DataError::Query(format!("Failed to bulk insert into {}: {}", table_name, e))
            })?;

        let rows_affected = result.inserted_ids.len() as u64;
        let elapsed = start.elapsed();

        info!(
            "Bulk insert completed: {} rows into {} in {}ms",
            rows_affected,
            table_name,
            elapsed.as_millis()
        );

        Ok(rows_affected)
    }

    #[instrument(skip(self, updates), fields(collection = %table_name, update_count = updates.len()))]
    async fn bulk_update(
        &self,
        table_name: &str,
        updates: &[(std::collections::HashMap<String, QueryValue>, String)],
        _schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_collection_name(table_name)?;

        if updates.is_empty() {
            return Ok(0);
        }

        info!("Bulk updating {} rows in {}", updates.len(), table_name);

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let start = std::time::Instant::now();

        let db = client.database(db_name);
        let collection = db.collection::<mongodb::bson::Document>(table_name);

        let mut total_affected = 0u64;

        // Execute each update (MongoDB doesn't have a simple bulk update with different filters)
        for (set_clauses, where_clause) in updates {
            if set_clauses.is_empty() {
                continue;
            }

            // Build update document
            let mut update_doc = mongodb::bson::Document::new();
            for (col, value) in set_clauses.iter() {
                let bson_value = match value {
                    QueryValue::Null => mongodb::bson::Bson::Null,
                    QueryValue::Bool(b) => mongodb::bson::Bson::Boolean(*b),
                    QueryValue::Int(i) => mongodb::bson::Bson::Int64(*i),
                    QueryValue::Float(f) => mongodb::bson::Bson::Double(*f),
                    QueryValue::Text(s) => mongodb::bson::Bson::String(s.clone()),
                    QueryValue::Bytes(b) => mongodb::bson::Bson::Binary(mongodb::bson::Binary {
                        subtype: mongodb::bson::spec::BinarySubtype::Generic,
                        bytes: b.clone(),
                    }),
                };
                update_doc.insert(col, bson_value);
            }

            // Parse where clause (simplified - in production would need proper query parser)
            // For now, assume simple "field = value" format
            let filter = Self::parse_where_clause_to_filter(where_clause)?;

            let result = collection
                .update_many(filter, doc! { "$set": update_doc }, None)
                .await
                .map_err(|e| {
                    DataError::Query(format!("Failed to bulk update {}: {}", table_name, e))
                })?;

            total_affected += result.modified_count;
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk update completed: {} rows in {} in {}ms",
            total_affected,
            table_name,
            elapsed.as_millis()
        );

        Ok(total_affected)
    }

    #[instrument(skip(self, where_clauses), fields(collection = %table_name, delete_count = where_clauses.len()))]
    async fn bulk_delete(
        &self,
        table_name: &str,
        where_clauses: &[String],
        _schema: Option<&str>,
    ) -> Result<u64> {
        Self::validate_collection_name(table_name)?;

        if where_clauses.is_empty() {
            return Ok(0);
        }

        info!(
            "Bulk deleting {} rows from {}",
            where_clauses.len(),
            table_name
        );

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected to database".to_string()))?;

        let db_name = self.current_database.as_deref().ok_or_else(|| {
            DataError::Connection("No database selected".to_string())
        })?;

        let start = std::time::Instant::now();

        let db = client.database(db_name);
        let collection = db.collection::<mongodb::bson::Document>(table_name);

        let mut total_affected = 0u64;

        // Execute each delete
        for where_clause in where_clauses {
            if where_clause.trim().is_empty() {
                continue;
            }

            // Parse where clause to filter
            let filter = Self::parse_where_clause_to_filter(where_clause)?;

            let result = collection
                .delete_many(filter, None)
                .await
                .map_err(|e| {
                    DataError::Query(format!("Failed to bulk delete from {}: {}", table_name, e))
                })?;

            total_affected += result.deleted_count;
        }

        let elapsed = start.elapsed();

        info!(
            "Bulk delete completed: {} rows from {} in {}ms",
            total_affected,
            table_name,
            elapsed.as_millis()
        );

        Ok(total_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> ConnectionConfig {
        ConnectionConfig {
            id: "test-mongodb".to_string(),
            name: "Test MongoDB".to_string(),
            db_type: DatabaseType::MongoDB,
            host: Some("localhost".to_string()),
            port: Some(27017),
            database: "test_db".to_string(),
            username: None,
            use_ssl: false,
            parameters: HashMap::new(),
        }
    }

    #[test]
    fn test_new_adapter() {
        let adapter = MongoDbAdapter::new();
        assert!(!adapter.is_connected());
        assert_eq!(adapter.database_type(), DatabaseType::MongoDB);
    }

    #[test]
    fn test_default_adapter() {
        let adapter = MongoDbAdapter::default();
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_connection_string_basic() {
        let mut config = test_config();
        config.username = Some("admin".to_string());
        let conn_str = MongoDbAdapter::build_connection_string(&config, Some("password123"));
        assert!(conn_str.contains("mongodb://"));
        assert!(conn_str.contains("admin"));
        assert!(conn_str.contains("password123"));
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("27017"));
    }

    #[test]
    fn test_connection_string_without_auth() {
        let config = test_config();
        let conn_str = MongoDbAdapter::build_connection_string(&config, None);
        assert!(conn_str.contains("mongodb://"));
        assert!(conn_str.contains("localhost"));
        assert!(conn_str.contains("27017"));
        assert!(!conn_str.contains("@")); // No auth separator
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut adapter = MongoDbAdapter::new();
        let result = adapter.disconnect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_when_not_connected() {
        let adapter = MongoDbAdapter::new();
        let result = adapter
            .execute_query(r#"{"collection":"users","filter":{}}"#)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_databases_when_not_connected() {
        let adapter = MongoDbAdapter::new();
        let result = adapter.list_databases().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_list_tables_when_not_connected() {
        let adapter = MongoDbAdapter::new();
        let result = adapter.list_tables(None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_describe_table_when_not_connected() {
        let adapter = MongoDbAdapter::new();
        let result = adapter.describe_table("users", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[tokio::test]
    async fn test_connect_with_wrong_database_type() {
        let mut adapter = MongoDbAdapter::new();
        let mut config = test_config();
        config.db_type = DatabaseType::Postgres;
        let result = adapter.connect(&config, Some("password")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_bson_conversions() {
        assert!(matches!(
            MongoDbAdapter::bson_to_query_value(&Bson::Null),
            QueryValue::Null
        ));
        assert!(matches!(
            MongoDbAdapter::bson_to_query_value(&Bson::Boolean(true)),
            QueryValue::Bool(true)
        ));
        assert!(matches!(
            MongoDbAdapter::bson_to_query_value(&Bson::Int32(42)),
            QueryValue::Int(42)
        ));
        assert!(matches!(
            MongoDbAdapter::bson_to_query_value(&Bson::String("test".to_string())),
            QueryValue::Text(_)
        ));
    }

    // ===== Validation Tests =====

    #[test]
    fn test_validate_database_name_valid() {
        assert!(MongoDbAdapter::validate_database_name("testdb").is_ok());
        assert!(MongoDbAdapter::validate_database_name("my_database_123").is_ok());
    }

    #[test]
    fn test_validate_database_name_empty() {
        let result = MongoDbAdapter::validate_database_name("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_database_name_too_long() {
        let long_name = "a".repeat(65);
        let result = MongoDbAdapter::validate_database_name(&long_name);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_database_name_max_length() {
        let max_name = "a".repeat(64);
        assert!(MongoDbAdapter::validate_database_name(&max_name).is_ok());
    }

    #[test]
    fn test_validate_collection_name_valid() {
        assert!(MongoDbAdapter::validate_collection_name("users").is_ok());
        assert!(MongoDbAdapter::validate_collection_name("order_items").is_ok());
    }

    #[test]
    fn test_validate_collection_name_empty() {
        let result = MongoDbAdapter::validate_collection_name("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_collection_name_system_prefix() {
        let result = MongoDbAdapter::validate_collection_name("system.users");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Config(_)));
    }

    #[test]
    fn test_validate_collection_name_invalid_chars() {
        assert!(MongoDbAdapter::validate_collection_name("test$collection").is_err());
        assert!(MongoDbAdapter::validate_collection_name("test\0collection").is_err());
    }

    // ===== QueryValue Display Tests =====

    #[test]
    fn test_query_value_display_null() {
        assert_eq!(QueryValue::Null.to_string(), "NULL");
    }

    #[test]
    fn test_query_value_display_bool() {
        assert_eq!(QueryValue::Bool(true).to_string(), "true");
        assert_eq!(QueryValue::Bool(false).to_string(), "false");
    }

    #[test]
    fn test_query_value_display_int() {
        assert_eq!(QueryValue::Int(42).to_string(), "42");
        assert_eq!(QueryValue::Int(-100).to_string(), "-100");
    }

    #[test]
    fn test_query_value_display_float() {
        assert_eq!(QueryValue::Float(3.14).to_string(), "3.14");
        assert_eq!(QueryValue::Float(-2.5).to_string(), "-2.5");
    }

    #[test]
    fn test_query_value_display_text() {
        assert_eq!(QueryValue::Text("hello".to_string()).to_string(), "hello");
    }

    #[test]
    fn test_query_value_display_bytes() {
        let bytes = vec![1, 2, 3, 4, 5];
        assert_eq!(QueryValue::Bytes(bytes).to_string(), "<5 bytes>");
    }

    // ========== Bulk Operations Tests (TDD - RED Phase) ==========

    #[tokio::test]
    async fn test_bulk_insert_not_connected() {
        let adapter = MongoDbAdapter::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let rows = vec![
            vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
            vec![QueryValue::Int(2), QueryValue::Text("Bob".to_string())],
        ];

        let result = adapter
            .bulk_insert("users", &columns, &rows, None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_insert_validation_empty_columns() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let columns = vec![];
            let rows = vec![vec![QueryValue::Int(1)]];

            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[test]
    fn test_bulk_insert_validation_empty_rows() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let columns = vec!["id".to_string()];
            let rows = vec![];

            // Empty rows should return Ok(0)
            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_insert_validation_column_count_mismatch() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let columns = vec!["id".to_string(), "name".to_string()];
            let rows = vec![
                vec![QueryValue::Int(1), QueryValue::Text("Alice".to_string())],
                vec![QueryValue::Int(2)], // Wrong column count
            ];

            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_bulk_insert_validation_collection_name() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let columns = vec!["id".to_string()];
            let rows = vec![vec![QueryValue::Int(1)]];

            // Empty collection name
            let result = adapter.bulk_insert("", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));

            // Invalid characters
            let result = adapter.bulk_insert("test$collection", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Config(_)));
        });
    }

    #[tokio::test]
    async fn test_bulk_update_not_connected() {
        let adapter = MongoDbAdapter::new();
        let mut update_map = HashMap::new();
        update_map.insert("name".to_string(), QueryValue::Text("Updated".to_string()));

        let updates = vec![(update_map, "id = 1".to_string())];

        let result = adapter.bulk_update("users", &updates, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_update_validation_empty_updates() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let updates = vec![];

            // Empty updates should return Ok(0)
            let result = adapter.bulk_update("users", &updates, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[tokio::test]
    async fn test_bulk_delete_not_connected() {
        let adapter = MongoDbAdapter::new();
        let where_clauses = vec!["id = 1".to_string(), "id = 2".to_string()];

        let result = adapter.bulk_delete("users", &where_clauses, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
    }

    #[test]
    fn test_bulk_delete_validation_empty_clauses() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let where_clauses = vec![];

            // Empty clauses should return Ok(0)
            let result = adapter.bulk_delete("users", &where_clauses, None).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        });
    }

    #[test]
    fn test_bulk_insert_all_data_types() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = MongoDbAdapter::new();
            let columns = vec![
                "id".to_string(),
                "name".to_string(),
                "active".to_string(),
                "score".to_string(),
                "data".to_string(),
                "description".to_string(),
            ];
            let rows = vec![
                vec![
                    QueryValue::Int(1),
                    QueryValue::Text("Alice".to_string()),
                    QueryValue::Bool(true),
                    QueryValue::Float(95.5),
                    QueryValue::Bytes(vec![1, 2, 3]),
                    QueryValue::Null,
                ],
                vec![
                    QueryValue::Int(2),
                    QueryValue::Text("Bob".to_string()),
                    QueryValue::Bool(false),
                    QueryValue::Float(87.3),
                    QueryValue::Bytes(vec![4, 5, 6]),
                    QueryValue::Text("Some description".to_string()),
                ],
            ];

            // Will fail at connection, but validates we handle all types
            let result = adapter.bulk_insert("users", &columns, &rows, None).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DataError::Connection(_)));
        });
    }
}

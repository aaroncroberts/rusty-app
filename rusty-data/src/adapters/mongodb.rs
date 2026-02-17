use crate::adapter::{
    ColumnInfo, ConnectionConfig, DatabaseAdapter, DatabaseType, QueryResult, QueryValue,
    TableInfo,
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

    /// Build a connection string from configuration
    fn build_connection_string(config: &ConnectionConfig, password: Option<&str>) -> String {
        let host = config.host.as_deref().unwrap_or("localhost");
        let port = config.port.unwrap_or(27017);
        let username = config.username.as_deref();
        let password = password;

        if let (Some(user), Some(pass)) = (username, password) {
            format!("mongodb://{}:{}@{}:{}", user, pass, host, port)
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

        info!("Connecting to MongoDB database");
        let connection_string = Self::build_connection_string(config, password);

        let client_options = ClientOptions::parse(&connection_string)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to parse MongoDB connection string");
                DataError::Connection(format!("Failed to parse connection string: {}", e))
            })?;

        let client = Client::with_options(client_options).map_err(|e| {
            warn!(error = %e, "Failed to create MongoDB client");
            DataError::Connection(format!("Failed to create client: {}", e))
        })?;

        // Test the connection
        client
            .database(&config.database)
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to connect to MongoDB");
                DataError::Connection(format!("Failed to connect: {}", e))
            })?;

        self.client = Some(client);
        self.current_database = Some(config.database.clone());
        info!("Successfully connected to MongoDB");
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
        debug!("Executing MongoDB query");
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DataError::Connection("Not connected".to_string()))?;

        let db_name = self
            .current_database
            .as_ref()
            .ok_or_else(|| DataError::Connection("No database selected".to_string()))?;

        // Parse the query as a MongoDB command
        // For simplicity, we'll assume the query is a JSON document representing a find command
        // Format: {"collection": "collectionName", "filter": {...}, "limit": 10}
        let command: Document = serde_json::from_str(query).map_err(|e| {
            DataError::Query(format!("Invalid MongoDB query format: {}. Expected JSON document with 'collection' and 'filter' fields", e))
        })?;

        let collection_name = command
            .get_str("collection")
            .map_err(|_| DataError::Query("Missing 'collection' field in query".to_string()))?;

        let filter = command
            .get_document("filter")
            .unwrap_or(&Document::new())
            .clone();

        let db = client.database(db_name);
        let collection = db.collection::<Document>(collection_name);

        let mut cursor = collection
            .find(filter, None)
            .await
            .map_err(|e| DataError::Query(format!("Query failed: {}", e)))?;

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

        let rows_affected = result_rows.len() as u64;
        Ok(QueryResult {
            columns,
            rows: result_rows,
            rows_affected: Some(rows_affected),
        })
    }

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
}

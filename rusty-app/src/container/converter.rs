//! Converter for transforming ContainerInfo into ConnectionConfig

use super::types::ContainerInfo;
use rusty_data::adapter::{ConnectionConfig, DatabaseType};
use std::collections::HashMap;

/// Container credentials for local development containers
struct ContainerCredentials {
    username: &'static str,
    password: &'static str,
    database: &'static str,
}

impl ContainerCredentials {
    /// Get credentials for a specific container database type
    fn for_database(db_type: DatabaseType) -> Self {
        match db_type {
            DatabaseType::Postgres => Self {
                username: "test_user",
                password: "test_password",
                database: "test_db",
            },
            DatabaseType::MySQL => Self {
                username: "test_user",
                password: "test_password",
                database: "test_db",
            },
            DatabaseType::MongoDB => Self {
                username: "test_user",
                password: "test_password",
                database: "test_db",
            },
            DatabaseType::SQLServer => Self {
                username: "sa",
                password: "TestPassword123!",
                database: "master",
            },
            DatabaseType::Oracle => Self {
                username: "test_user",
                password: "test_password",
                database: "XE", // SID for Oracle XE
            },
            DatabaseType::SQLite => Self {
                username: "",
                password: "",
                database: "memory.db",
            },
        }
    }
}

/// Convert container database type name to DatabaseType enum
///
/// # Example
/// ```
/// use rusty_app::container::converter::container_db_type_to_enum;
/// use rusty_data::adapter::DatabaseType;
///
/// assert_eq!(container_db_type_to_enum("postgres"), Some(DatabaseType::Postgres));
/// assert_eq!(container_db_type_to_enum("mysql"), Some(DatabaseType::MySQL));
/// assert_eq!(container_db_type_to_enum("invalid"), None);
/// ```
pub fn container_db_type_to_enum(db_type: &str) -> Option<DatabaseType> {
    match db_type {
        "postgres" => Some(DatabaseType::Postgres),
        "mysql" => Some(DatabaseType::MySQL),
        "mongodb" => Some(DatabaseType::MongoDB),
        "mssql" => Some(DatabaseType::SQLServer),
        "oracle" => Some(DatabaseType::Oracle),
        "sqlite" => Some(DatabaseType::SQLite),
        _ => None,
    }
}

/// Convert ContainerInfo to ConnectionConfig
///
/// Creates a connection configuration for a running container using:
/// - localhost as host
/// - Default port for the database type
/// - Test credentials from container configuration
/// - Connection ID prefixed with "local-" for grouping
///
/// # Arguments
/// * `container` - Container information from ContainerManager
///
/// # Returns
/// * `Some(ConnectionConfig)` if the container is a recognized database type
/// * `None` if the container type cannot be mapped to a database
///
/// # Example
/// ```
/// use rusty_app::container::{ContainerInfo, ContainerStatus};
/// use rusty_app::container::converter::container_to_connection;
///
/// let container = ContainerInfo::new(
///     "icitadel-dev-postgres".to_string(),
///     ContainerStatus::Running,
///     "5432".to_string(),
/// );
///
/// let connection = container_to_connection(&container).unwrap();
/// assert_eq!(connection.id, "local-postgres");
/// assert_eq!(connection.host, Some("localhost".to_string()));
/// assert_eq!(connection.port, Some(5432));
/// ```
pub fn container_to_connection(container: &ContainerInfo) -> Option<ConnectionConfig> {
    // Extract database type from container name
    let db_type_str = container.database_type()?;
    let db_type = container_db_type_to_enum(db_type_str)?;

    // Get credentials for this database type
    let credentials = ContainerCredentials::for_database(db_type);

    // Generate connection ID and name
    let connection_id = format!("local-{}", db_type_str);
    let connection_name = format!(
        "Local {} (Container)",
        match db_type {
            DatabaseType::Postgres => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
            DatabaseType::MongoDB => "MongoDB",
            DatabaseType::SQLServer => "SQL Server",
            DatabaseType::Oracle => "Oracle",
            DatabaseType::SQLite => "SQLite",
        }
    );

    // Build connection config
    Some(ConnectionConfig {
        id: connection_id,
        name: connection_name,
        db_type,
        host: Some("localhost".to_string()),
        port: db_type.default_port(),
        database: credentials.database.to_string(),
        username: Some(credentials.username.to_string()),
        use_ssl: false, // Local containers don't use SSL
        parameters: HashMap::new(),
    })
}

/// Sync connections with running containers
///
/// This function compares running containers with existing connections and:
/// 1. Creates connections for new running containers
/// 2. Removes connections for stopped containers
/// 3. Preserves non-local connections
///
/// # Arguments
/// * `containers` - List of running containers from ContainerManager
/// * `existing_connections` - Current connection configurations
///
/// # Returns
/// A new vector of connections with local connections synced to container state
///
/// # Example
/// ```
/// use rusty_app::container::{ContainerInfo, ContainerStatus};
/// use rusty_app::container::converter::sync_connections_with_containers;
///
/// let containers = vec![
///     ContainerInfo::new(
///         "icitadel-dev-postgres".to_string(),
///         ContainerStatus::Running,
///         "5432".to_string(),
///     ),
/// ];
///
/// let connections = sync_connections_with_containers(&containers, &[]);
/// assert_eq!(connections.len(), 1);
/// assert_eq!(connections[0].id, "local-postgres");
/// ```
pub fn sync_connections_with_containers(
    containers: &[ContainerInfo],
    existing_connections: &[ConnectionConfig],
) -> Vec<ConnectionConfig> {
    let mut synced_connections = Vec::new();

    // Keep all non-local connections
    for conn in existing_connections {
        if !conn.id.starts_with("local-") {
            synced_connections.push(conn.clone());
        }
    }

    // Add connections for running containers
    for container in containers {
        if let Some(connection) = container_to_connection(container) {
            // Only add if not already present
            if !synced_connections.iter().any(|c| c.id == connection.id) {
                synced_connections.push(connection);
            }
        }
    }

    synced_connections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerStatus;

    #[test]
    fn test_container_db_type_to_enum() {
        assert_eq!(
            container_db_type_to_enum("postgres"),
            Some(DatabaseType::Postgres)
        );
        assert_eq!(
            container_db_type_to_enum("mysql"),
            Some(DatabaseType::MySQL)
        );
        assert_eq!(
            container_db_type_to_enum("mongodb"),
            Some(DatabaseType::MongoDB)
        );
        assert_eq!(
            container_db_type_to_enum("mssql"),
            Some(DatabaseType::SQLServer)
        );
        assert_eq!(
            container_db_type_to_enum("oracle"),
            Some(DatabaseType::Oracle)
        );
        assert_eq!(container_db_type_to_enum("invalid"), None);
    }

    #[test]
    fn test_container_to_connection_postgres() {
        let container = ContainerInfo::new(
            "icitadel-dev-postgres".to_string(),
            ContainerStatus::Running,
            "5432".to_string(),
        );

        let connection = container_to_connection(&container).unwrap();

        assert_eq!(connection.id, "local-postgres");
        assert_eq!(connection.name, "Local PostgreSQL (Container)");
        assert_eq!(connection.db_type, DatabaseType::Postgres);
        assert_eq!(connection.host, Some("localhost".to_string()));
        assert_eq!(connection.port, Some(5432));
        assert_eq!(connection.database, "test_db");
        assert_eq!(connection.username, Some("test_user".to_string()));
        assert!(!connection.use_ssl);
    }

    #[test]
    fn test_container_to_connection_mysql() {
        let container = ContainerInfo::new(
            "icitadel-dev-mysql".to_string(),
            ContainerStatus::Running,
            "3306".to_string(),
        );

        let connection = container_to_connection(&container).unwrap();

        assert_eq!(connection.id, "local-mysql");
        assert_eq!(connection.name, "Local MySQL (Container)");
        assert_eq!(connection.db_type, DatabaseType::MySQL);
        assert_eq!(connection.port, Some(3306));
        assert_eq!(connection.database, "test_db");
        assert_eq!(connection.username, Some("test_user".to_string()));
    }

    #[test]
    fn test_container_to_connection_mongodb() {
        let container = ContainerInfo::new(
            "icitadel-dev-mongodb".to_string(),
            ContainerStatus::Running,
            "27017".to_string(),
        );

        let connection = container_to_connection(&container).unwrap();

        assert_eq!(connection.id, "local-mongodb");
        assert_eq!(connection.name, "Local MongoDB (Container)");
        assert_eq!(connection.db_type, DatabaseType::MongoDB);
        assert_eq!(connection.port, Some(27017));
        assert_eq!(connection.database, "test_db");
    }

    #[test]
    fn test_container_to_connection_mssql() {
        let container = ContainerInfo::new(
            "icitadel-dev-mssql".to_string(),
            ContainerStatus::Running,
            "1433".to_string(),
        );

        let connection = container_to_connection(&container).unwrap();

        assert_eq!(connection.id, "local-mssql");
        assert_eq!(connection.name, "Local SQL Server (Container)");
        assert_eq!(connection.db_type, DatabaseType::SQLServer);
        assert_eq!(connection.port, Some(1433));
        assert_eq!(connection.database, "master"); // SQL Server uses master
        assert_eq!(connection.username, Some("sa".to_string())); // SQL Server uses sa
    }

    #[test]
    fn test_container_to_connection_oracle() {
        let container = ContainerInfo::new(
            "icitadel-dev-oracle".to_string(),
            ContainerStatus::Running,
            "1521".to_string(),
        );

        let connection = container_to_connection(&container).unwrap();

        assert_eq!(connection.id, "local-oracle");
        assert_eq!(connection.name, "Local Oracle (Container)");
        assert_eq!(connection.db_type, DatabaseType::Oracle);
        assert_eq!(connection.port, Some(1521));
        assert_eq!(connection.database, "XE"); // Oracle uses XE SID
    }

    #[test]
    fn test_container_to_connection_invalid() {
        let container = ContainerInfo::new(
            "other-container".to_string(),
            ContainerStatus::Running,
            "8080".to_string(),
        );

        let connection = container_to_connection(&container);
        assert!(connection.is_none());
    }

    #[test]
    fn test_sync_connections_with_containers_add_new() {
        let containers = vec![ContainerInfo::new(
            "icitadel-dev-postgres".to_string(),
            ContainerStatus::Running,
            "5432".to_string(),
        )];

        let synced = sync_connections_with_containers(&containers, &[]);

        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].id, "local-postgres");
    }

    #[test]
    fn test_sync_connections_with_containers_remove_stopped() {
        let containers = vec![]; // No running containers

        let existing = vec![ConnectionConfig {
            id: "local-postgres".to_string(),
            name: "Local PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "test_db".to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }];

        let synced = sync_connections_with_containers(&containers, &existing);

        // Local connection should be removed when container is stopped
        assert_eq!(synced.len(), 0);
    }

    #[test]
    fn test_sync_connections_preserve_non_local() {
        let containers = vec![];

        let existing = vec![ConnectionConfig {
            id: "production-db".to_string(),
            name: "Production Database".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("prod.example.com".to_string()),
            port: Some(5432),
            database: "prod_db".to_string(),
            username: Some("prod_user".to_string()),
            use_ssl: true,
            parameters: HashMap::new(),
        }];

        let synced = sync_connections_with_containers(&containers, &existing);

        // Non-local connections should be preserved
        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].id, "production-db");
    }

    #[test]
    fn test_sync_connections_mixed() {
        let containers = vec![
            ContainerInfo::new(
                "icitadel-dev-postgres".to_string(),
                ContainerStatus::Running,
                "5432".to_string(),
            ),
            ContainerInfo::new(
                "icitadel-dev-mysql".to_string(),
                ContainerStatus::Running,
                "3306".to_string(),
            ),
        ];

        let existing = vec![
            ConnectionConfig {
                id: "production-db".to_string(),
                name: "Production".to_string(),
                db_type: DatabaseType::Postgres,
                host: Some("prod.example.com".to_string()),
                port: Some(5432),
                database: "prod".to_string(),
                username: Some("prod_user".to_string()),
                use_ssl: true,
                parameters: HashMap::new(),
            },
            ConnectionConfig {
                id: "local-oracle".to_string(),
                name: "Local Oracle".to_string(),
                db_type: DatabaseType::Oracle,
                host: Some("localhost".to_string()),
                port: Some(1521),
                database: "XE".to_string(),
                username: Some("test_user".to_string()),
                use_ssl: false,
                parameters: HashMap::new(),
            },
        ];

        let synced = sync_connections_with_containers(&containers, &existing);

        // Should have: production-db + local-postgres + local-mysql
        // local-oracle should be removed (container not running)
        assert_eq!(synced.len(), 3);

        let ids: Vec<&str> = synced.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"production-db"));
        assert!(ids.contains(&"local-postgres"));
        assert!(ids.contains(&"local-mysql"));
        assert!(!ids.contains(&"local-oracle"));
    }

    #[test]
    fn test_sync_connections_no_duplicates() {
        let containers = vec![ContainerInfo::new(
            "icitadel-dev-postgres".to_string(),
            ContainerStatus::Running,
            "5432".to_string(),
        )];

        let existing = vec![ConnectionConfig {
            id: "local-postgres".to_string(),
            name: "Local PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "test_db".to_string(),
            username: Some("test_user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        }];

        let synced = sync_connections_with_containers(&containers, &existing);

        // Should not duplicate existing local connection
        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].id, "local-postgres");
    }
}

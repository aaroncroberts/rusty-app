use crate::adapter::ConnectionConfig;
use crate::error::{DataError, Result};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, warn};

/// Manages application configuration and connection settings
pub struct ConfigManager {
    config_dir: PathBuf,
}

/// Encrypted credential storage
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedCredentials {
    /// Encrypted password data
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    /// Salt used for key derivation
    pub salt: String,
}

/// Wrapper for serializing connection list to TOML
#[derive(Debug, Serialize, Deserialize)]
struct ConnectionsFile {
    connections: Vec<ConnectionConfig>,
}

impl ConfigManager {
    /// Create a new configuration manager
    #[instrument(skip(config_dir), fields(config_dir = %config_dir.as_ref().display()))]
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        let config_dir = expand_home_dir(config_dir.as_ref())?;

        if !config_dir.exists() {
            info!("Creating config directory");
            fs::create_dir_all(&config_dir).map_err(|e| {
                DataError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }

        debug!("ConfigManager initialized");
        Ok(Self { config_dir })
    }

    /// Get the path to the connections configuration file
    pub fn connections_file(&self) -> PathBuf {
        self.config_dir.join("connections.toml")
    }

    /// Load connection configurations from file
    #[instrument(skip(self))]
    pub fn load_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let path = self.connections_file();

        if !path.exists() {
            debug!("Connections file does not exist, returning empty list");
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&path)?;
        let file: ConnectionsFile = toml::from_str(&contents)?;

        info!(count = file.connections.len(), "Loaded connections");
        Ok(file.connections)
    }

    /// Save connection configurations to file
    #[instrument(skip(self, connections), fields(count = connections.len()))]
    pub fn save_connections(&self, connections: &[ConnectionConfig]) -> Result<()> {
        // Validate all connections before saving
        Self::validate_connections(connections)?;

        let file = ConnectionsFile {
            connections: connections.to_vec(),
        };
        let contents = toml::to_string_pretty(&file)?;
        let path = self.connections_file();

        fs::write(&path, contents)?;
        info!("Saved connections");

        Ok(())
    }

    /// Validate a list of connections
    pub fn validate_connections(connections: &[ConnectionConfig]) -> Result<()> {
        // Check for duplicate IDs
        let mut seen_ids = std::collections::HashSet::new();
        for conn in connections {
            if !seen_ids.insert(&conn.id) {
                return Err(DataError::Config(format!(
                    "Duplicate connection ID: {}",
                    conn.id
                )));
            }

            // Validate individual connection
            Self::validate_connection(conn)?;
        }

        Ok(())
    }

    /// Validate a single connection configuration
    pub fn validate_connection(config: &ConnectionConfig) -> Result<()> {
        use crate::adapter::DatabaseType;

        // Validate ID is not empty
        if config.id.trim().is_empty() {
            return Err(DataError::Config("Connection ID cannot be empty".to_string()));
        }

        // Validate database name is not empty
        if config.database.trim().is_empty() {
            return Err(DataError::Config("Database name cannot be empty".to_string()));
        }

        // For non-file-based databases, host is required
        match config.db_type {
            DatabaseType::SQLite => {
                // SQLite uses database field as file path, host/port are optional
            }
            DatabaseType::Postgres | DatabaseType::MySQL | DatabaseType::MongoDB | DatabaseType::SQLServer | DatabaseType::Oracle => {
                if config.host.is_none() || config.host.as_ref().unwrap().trim().is_empty() {
                    return Err(DataError::Config(format!(
                        "{:?} requires a host address",
                        config.db_type
                    )));
                }
            }
        }

        // Validate port range if provided
        if let Some(port) = config.port {
            if port == 0 {
                return Err(DataError::Config(
                    "Invalid port number: 0. Must be between 1 and 65535".to_string()
                ));
            }
        }

        // For server databases, port should be specified
        match config.db_type {
            DatabaseType::Postgres | DatabaseType::MySQL | DatabaseType::MongoDB | DatabaseType::SQLServer | DatabaseType::Oracle => {
                if config.port.is_none() {
                    return Err(DataError::Config(format!(
                        "{:?} requires a port number",
                        config.db_type
                    )));
                }
            }
            DatabaseType::SQLite => {
                // Port not required for SQLite
            }
        }

        Ok(())
    }

    /// Encrypt a password using AES-GCM with Argon2 key derivation
    pub fn encrypt_password(password: &str, master_password: &str) -> Result<EncryptedCredentials> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();
        let key_hash = argon2
            .hash_password(master_password.as_bytes(), &salt)
            .map_err(|e| DataError::Encryption(format!("Key derivation failed: {}", e)))?;

        let key_bytes = key_hash.hash.ok_or_else(|| {
            DataError::Encryption("Failed to extract key from hash".to_string())
        })?;
        let key = key_bytes.as_bytes();

        if key.len() < 32 {
            return Err(DataError::Encryption(
                "Derived key too short for AES-256".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&key[..32])
            .map_err(|e| DataError::Encryption(format!("Cipher creation failed: {}", e)))?;

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, password.as_bytes())
            .map_err(|e| DataError::Encryption(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedCredentials {
            ciphertext,
            nonce: nonce.to_vec(),
            salt: salt.to_string(),
        })
    }

    /// Decrypt a password using AES-GCM with Argon2 key derivation
    pub fn decrypt_password(
        encrypted: &EncryptedCredentials,
        master_password: &str,
    ) -> Result<String> {
        let salt = SaltString::from_b64(&encrypted.salt)
            .map_err(|e| DataError::Encryption(format!("Invalid salt: {}", e)))?;

        let argon2 = Argon2::default();
        let key_hash = argon2
            .hash_password(master_password.as_bytes(), &salt)
            .map_err(|e| DataError::Encryption(format!("Key derivation failed: {}", e)))?;

        let key_bytes = key_hash.hash.ok_or_else(|| {
            DataError::Encryption("Failed to extract key from hash".to_string())
        })?;
        let key = key_bytes.as_bytes();

        if key.len() < 32 {
            return Err(DataError::Encryption(
                "Derived key too short for AES-256".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&key[..32])
            .map_err(|e| DataError::Encryption(format!("Cipher creation failed: {}", e)))?;

        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| DataError::Encryption(format!("Decryption failed (wrong password?): {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| DataError::Encryption(format!("Invalid UTF-8 in decrypted data: {}", e)))
    }
}

/// Expand ~ in path to home directory
fn expand_home_dir(path: &Path) -> Result<PathBuf> {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join(&path_str[2..]));
            }
        }
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = "my_secret_password";
        let master = "master_password_123";

        let encrypted = ConfigManager::encrypt_password(password, master).unwrap();
        let decrypted = ConfigManager::decrypt_password(&encrypted, master).unwrap();

        assert_eq!(password, decrypted);
    }

    #[test]
    fn test_wrong_master_password_fails() {
        let password = "my_secret_password";
        let master = "master_password_123";
        let wrong_master = "wrong_password";

        let encrypted = ConfigManager::encrypt_password(password, master).unwrap();
        let result = ConfigManager::decrypt_password(&encrypted, wrong_master);

        assert!(result.is_err());
    }

    #[test]
    fn test_expand_home_dir() {
        let path = Path::new("~/test/config");
        let expanded = expand_home_dir(path).unwrap();

        assert!(!expanded.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_config_manager_creation() {
        use std::env;
        let temp_dir = env::temp_dir().join("rusty-data-test-config");

        let _manager = ConfigManager::new(&temp_dir).unwrap();
        assert!(temp_dir.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_connections_missing_file() {
        use std::env;
        let temp_dir = env::temp_dir().join("rusty-data-test-missing");

        let manager = ConfigManager::new(&temp_dir).unwrap();
        let connections = manager.load_connections().unwrap();

        assert_eq!(connections.len(), 0);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_load_connections() {
        use std::env;
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let temp_dir = env::temp_dir().join("rusty-data-test-save-load");

        let manager = ConfigManager::new(&temp_dir).unwrap();

        // Create test connections
        let connections = vec![
            ConnectionConfig {
                id: "test-pg".to_string(),
                name: "Test Postgres".to_string(),
                db_type: DatabaseType::Postgres,
                host: Some("localhost".to_string()),
                port: Some(5432),
                database: "testdb".to_string(),
                username: Some("testuser".to_string()),
                use_ssl: false,
                parameters: HashMap::new(),
            },
            ConnectionConfig {
                id: "test-mysql".to_string(),
                name: "Test MySQL".to_string(),
                db_type: DatabaseType::MySQL,
                host: Some("127.0.0.1".to_string()),
                port: Some(3306),
                database: "mydb".to_string(),
                username: Some("root".to_string()),
                use_ssl: true,
                parameters: HashMap::new(),
            },
        ];

        // Save connections
        manager.save_connections(&connections).unwrap();

        // Load connections
        let loaded = manager.load_connections().unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "test-pg");
        assert_eq!(loaded[0].db_type, DatabaseType::Postgres);
        assert_eq!(loaded[1].id, "test-mysql");
        assert_eq!(loaded[1].use_ssl, true);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_connections_file_path() {
        use std::env;
        let temp_dir = env::temp_dir().join("rusty-data-test-path");

        let _manager = ConfigManager::new(&temp_dir).unwrap();
        let path = _manager.connections_file();

        assert_eq!(path, temp_dir.join("connections.toml"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_empty_id() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID cannot be empty"));
    }

    #[test]
    fn test_validate_empty_database() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Database name cannot be empty"));
    }

    #[test]
    fn test_validate_missing_host_for_postgres() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::Postgres,
            host: None,
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a host"));
    }

    #[test]
    fn test_validate_missing_port_for_mysql() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::MySQL,
            host: Some("localhost".to_string()),
            port: None,
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a port"));
    }

    #[test]
    fn test_validate_invalid_port() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(0), // Port 0 is invalid
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port"));
    }

    #[test]
    fn test_validate_sqlite_no_host_required() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "test".to_string(),
            name: "Test SQLite".to_string(),
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            database: "/path/to/database.db".to_string(),
            username: None,
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_duplicate_ids() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;
        use std::env;

        let temp_dir = env::temp_dir().join("rusty-data-test-duplicate");
        let manager = ConfigManager::new(&temp_dir).unwrap();

        let connections = vec![
            ConnectionConfig {
                id: "duplicate".to_string(),
                name: "First".to_string(),
                db_type: DatabaseType::Postgres,
                host: Some("localhost".to_string()),
                port: Some(5432),
                database: "db1".to_string(),
                username: Some("user".to_string()),
                use_ssl: false,
                parameters: HashMap::new(),
            },
            ConnectionConfig {
                id: "duplicate".to_string(),
                name: "Second".to_string(),
                db_type: DatabaseType::MySQL,
                host: Some("localhost".to_string()),
                port: Some(3306),
                database: "db2".to_string(),
                username: Some("user".to_string()),
                use_ssl: false,
                parameters: HashMap::new(),
            },
        ];

        let result = manager.save_connections(&connections);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate connection ID"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_valid_postgres_connection() {
        use std::collections::HashMap;
        use crate::adapter::DatabaseType;

        let config = ConnectionConfig {
            id: "valid-pg".to_string(),
            name: "Valid Postgres".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
        };

        let result = ConfigManager::validate_connection(&config);
        assert!(result.is_ok());
    }
}

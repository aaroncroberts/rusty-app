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

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        let config_dir = expand_home_dir(config_dir.as_ref())?;

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                DataError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }

        Ok(Self { config_dir })
    }

    /// Get the path to the connections configuration file
    pub fn connections_file(&self) -> PathBuf {
        self.config_dir.join("connections.toml")
    }

    /// Load connection configurations from file
    pub fn load_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let path = self.connections_file();

        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&path)?;
        let connections: Vec<ConnectionConfig> = toml::from_str(&contents)?;

        Ok(connections)
    }

    /// Save connection configurations to file
    pub fn save_connections(&self, connections: &[ConnectionConfig]) -> Result<()> {
        let contents = toml::to_string_pretty(connections)?;
        let path = self.connections_file();

        fs::write(&path, contents)?;

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
}

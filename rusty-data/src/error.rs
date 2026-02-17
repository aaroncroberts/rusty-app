use thiserror::Error;

/// Result type for rusty-data operations
pub type Result<T> = std::result::Result<T, DataError>;

/// Main error type for data operations
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Adapter not found: {0}")]
    AdapterNotFound(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<toml::de::Error> for DataError {
    fn from(err: toml::de::Error) -> Self {
        DataError::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for DataError {
    fn from(err: toml::ser::Error) -> Self {
        DataError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for DataError {
    fn from(err: serde_json::Error) -> Self {
        DataError::Serialization(err.to_string())
    }
}

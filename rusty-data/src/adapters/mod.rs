// Database adapter implementations
// Each adapter module will be added here as they are implemented

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "mssql")]
pub mod mssql;

#[cfg(feature = "mongodb")]
pub mod mongodb;

#[cfg(feature = "oracle")]
pub mod oracle;

//! rusty-logging - Centralized logging infrastructure for rusty-app
//!
//! This crate provides structured, async-aware logging using the `tracing` ecosystem
//! with support for multiple output formats (console and files) and flexible configuration.
//!
//! # Features
//!
//! - **Structured Logging**: Key-value fields, spans, and events
//! - **Multiple Outputs**: Console (pretty/compact) and files (.log/.jsonl)
//! - **Dual Output**: Simultaneous console + file logging
//! - **Log Levels**: TRACE, DEBUG, INFO, WARN, ERROR
//! - **Categories**: Via tracing targets and spans
//! - **Environment Config**: RUST_LOG environment variable
//! - **File Rotation**: Size or time-based automatic rotation
//!
//! # Quick Start
//!
//! ```no_run
//! use rusty_logging::LoggingConfig;
//!
//! // Simple console logging
//! rusty_logging::init_default();
//!
//! // Custom configuration
//! LoggingConfig::builder()
//!     .with_console_pretty()
//!     .build()
//!     .expect("Failed to initialize logging");
//! ```

use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

pub mod config;
pub mod error;

pub use config::{LoggingConfig, LoggingConfigBuilder};
pub use error::{LoggingError, Result};

/// Initialize logging with default settings (pretty console output, INFO level)
///
/// This is a convenience function that sets up logging with sensible defaults:
/// - Pretty console format (colorized)
/// - INFO log level (can be overridden with RUST_LOG environment variable)
/// - Output to stderr
///
/// # Examples
///
/// ```no_run
/// rusty_logging::init_default();
/// tracing::info!("Application started");
/// ```
///
/// # Panics
///
/// Panics if logging has already been initialized or if initialization fails.
pub fn init_default() {
    init_default_with_filter("info").expect("Failed to initialize logging");
}

/// Initialize logging with default settings and a custom filter
///
/// # Arguments
///
/// * `filter` - Log level filter (e.g., "debug", "info", "warn")
///
/// # Examples
///
/// ```no_run
/// rusty_logging::init_default_with_filter("debug").unwrap();
/// tracing::debug!("Debug message");
/// ```
pub fn init_default_with_filter(filter: &str) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(filter))
        .map_err(|e| LoggingError::ConfigError(format!("Invalid filter: {}", e)))?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .try_init()
        .map_err(|e| LoggingError::InitError(format!("Failed to initialize subscriber: {}", e)))?;

    Ok(())
}

/// Initialize logging with a custom configuration
///
/// # Arguments
///
/// * `config` - Custom logging configuration
///
/// # Examples
///
/// ```no_run
/// use rusty_logging::LoggingConfig;
///
/// let config = LoggingConfig::builder()
///     .with_console_compact()
///     .build()
///     .unwrap();
///
/// rusty_logging::init(config).unwrap();
/// ```
pub fn init(config: LoggingConfig) -> Result<()> {
    config.apply()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_filter_parsing() {
        // Test valid filter levels
        let filter = EnvFilter::try_new("info");
        assert!(filter.is_ok());

        let filter = EnvFilter::try_new("debug");
        assert!(filter.is_ok());

        let filter = EnvFilter::try_new("warn");
        assert!(filter.is_ok());
    }

    #[test]
    fn test_env_filter_with_target() {
        // Test filter with target specification
        let filter = EnvFilter::try_new("rusty_data=debug,info");
        assert!(filter.is_ok());
    }

    #[test]
    fn test_logging_config_builder() {
        // Test that config builder can be created
        let builder = LoggingConfig::builder();
        assert!(builder.build().is_ok());
    }
}

use crate::error::{LoggingError, Result};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

/// Logging configuration for rusty-logging
///
/// This struct holds the configuration for logging initialization,
/// including output format, log levels, and other settings.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Environment filter for log levels and targets
    pub(crate) filter: String,

    /// Console output format
    pub(crate) console_format: ConsoleFormat,

    /// Console output writer (stdout/stderr)
    pub(crate) console_writer: ConsoleWriter,

    /// Whether console output is enabled
    pub(crate) console_enabled: bool,
}

/// Console output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleFormat {
    /// Pretty format with colors and full information (development)
    Pretty,

    /// Compact format with minimal output (production)
    Compact,
}

/// Console output writer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleWriter {
    /// Write to stdout
    Stdout,

    /// Write to stderr (default)
    Stderr,
}

impl LoggingConfig {
    /// Create a new builder for logging configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_console_pretty()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> LoggingConfigBuilder {
        LoggingConfigBuilder::default()
    }

    /// Apply this configuration and initialize logging
    ///
    /// # Errors
    ///
    /// Returns an error if logging has already been initialized or if
    /// the configuration is invalid.
    pub fn apply(self) -> Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.filter))
            .map_err(|e| LoggingError::FilterError(format!("Invalid filter '{}': {}", self.filter, e)))?;

        let registry = tracing_subscriber::registry().with(env_filter);

        if self.console_enabled {
            match (self.console_format, self.console_writer) {
                (ConsoleFormat::Pretty, ConsoleWriter::Stdout) => {
                    let layer = fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(std::io::stdout);

                    registry
                        .with(layer)
                        .try_init()
                        .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                }
                (ConsoleFormat::Pretty, ConsoleWriter::Stderr) => {
                    let layer = fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(std::io::stderr);

                    registry
                        .with(layer)
                        .try_init()
                        .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                }
                (ConsoleFormat::Compact, ConsoleWriter::Stdout) => {
                    let layer = fmt::layer()
                        .compact()
                        .with_target(true)
                        .with_writer(std::io::stdout);

                    registry
                        .with(layer)
                        .try_init()
                        .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                }
                (ConsoleFormat::Compact, ConsoleWriter::Stderr) => {
                    let layer = fmt::layer()
                        .compact()
                        .with_target(true)
                        .with_writer(std::io::stderr);

                    registry
                        .with(layer)
                        .try_init()
                        .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                }
            }
        } else {
            // No console output - just initialize registry
            registry
                .try_init()
                .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
        }

        Ok(())
    }
}

/// Builder for logging configuration
///
/// Provides a fluent API for constructing logging configurations.
#[derive(Debug, Clone)]
pub struct LoggingConfigBuilder {
    filter: String,
    console_format: ConsoleFormat,
    console_writer: ConsoleWriter,
    console_enabled: bool,
}

impl Default for LoggingConfigBuilder {
    fn default() -> Self {
        Self {
            filter: "info".to_string(),
            console_format: ConsoleFormat::Pretty,
            console_writer: ConsoleWriter::Stderr,
            console_enabled: true,
        }
    }
}

impl LoggingConfigBuilder {
    /// Set the log level filter
    ///
    /// # Arguments
    ///
    /// * `filter` - Log level filter (e.g., "debug", "info", "warn")
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_filter("debug")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self
    }

    /// Enable pretty console output (colorized, development-friendly)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_console_pretty()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_pretty(mut self) -> Self {
        self.console_format = ConsoleFormat::Pretty;
        self.console_enabled = true;
        self
    }

    /// Enable compact console output (minimal, production-optimized)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_console_compact()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_compact(mut self) -> Self {
        self.console_format = ConsoleFormat::Compact;
        self.console_enabled = true;
        self
    }

    /// Set console output to stdout
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_console_stdout()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_stdout(mut self) -> Self {
        self.console_writer = ConsoleWriter::Stdout;
        self.console_enabled = true;
        self
    }

    /// Set console output to stderr (default)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_console_stderr()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_stderr(mut self) -> Self {
        self.console_writer = ConsoleWriter::Stderr;
        self.console_enabled = true;
        self
    }

    /// Disable console output
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .without_console()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn without_console(mut self) -> Self {
        self.console_enabled = false;
        self
    }

    /// Build the logging configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn build(self) -> Result<LoggingConfig> {
        // Validate filter
        EnvFilter::try_new(&self.filter)
            .map_err(|e| LoggingError::FilterError(format!("Invalid filter '{}': {}", self.filter, e)))?;

        Ok(LoggingConfig {
            filter: self.filter,
            console_format: self.console_format,
            console_writer: self.console_writer,
            console_enabled: self.console_enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let config = LoggingConfig::builder().build().unwrap();
        assert_eq!(config.filter, "info");
        assert_eq!(config.console_format, ConsoleFormat::Pretty);
        assert!(config.console_enabled);
    }

    #[test]
    fn test_builder_with_filter() {
        let config = LoggingConfig::builder()
            .with_filter("debug")
            .build()
            .unwrap();
        assert_eq!(config.filter, "debug");
    }

    #[test]
    fn test_builder_with_compact() {
        let config = LoggingConfig::builder()
            .with_console_compact()
            .build()
            .unwrap();
        assert_eq!(config.console_format, ConsoleFormat::Compact);
        assert!(config.console_enabled);
    }

    #[test]
    fn test_builder_without_console() {
        let config = LoggingConfig::builder()
            .without_console()
            .build()
            .unwrap();
        assert!(!config.console_enabled);
    }

    #[test]
    fn test_builder_invalid_filter() {
        let result = LoggingConfig::builder()
            .with_filter("invalid[[[")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_console_format_variants() {
        assert_eq!(ConsoleFormat::Pretty, ConsoleFormat::Pretty);
        assert_eq!(ConsoleFormat::Compact, ConsoleFormat::Compact);
        assert_ne!(ConsoleFormat::Pretty, ConsoleFormat::Compact);
    }
}

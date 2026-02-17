use crate::error::{LoggingError, Result};
use std::path::PathBuf;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

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

    /// Whether file output is enabled
    pub(crate) file_enabled: bool,

    /// File output format
    pub(crate) file_format: FileFormat,

    /// Directory for log files
    pub(crate) file_directory: PathBuf,

    /// Prefix for log file names
    pub(crate) file_prefix: String,

    /// File rotation policy
    pub(crate) rotation_policy: RotationPolicy,
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

/// File output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Human-readable text format (.log)
    Text,

    /// JSON Lines format (.jsonl)
    Json,
}

/// File rotation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationPolicy {
    /// Rotate daily at midnight
    Daily,

    /// Rotate hourly
    Hourly,

    /// Rotate every minute (for testing)
    Minutely,

    /// Never rotate (append to single file)
    Never,
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
        use std::fs;

        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.filter))
            .map_err(|e| LoggingError::FilterError(format!("Invalid filter '{}': {}", self.filter, e)))?;

        let registry = tracing_subscriber::registry().with(env_filter);

        // Build layers based on configuration
        match (self.console_enabled, self.file_enabled) {
            // Both console and file output
            (true, true) => {
                // Create log directory if it doesn't exist
                fs::create_dir_all(&self.file_directory)?;

                // Convert rotation policy to tracing_appender Rotation
                let rotation = match self.rotation_policy {
                    RotationPolicy::Daily => Rotation::DAILY,
                    RotationPolicy::Hourly => Rotation::HOURLY,
                    RotationPolicy::Minutely => Rotation::MINUTELY,
                    RotationPolicy::Never => Rotation::NEVER,
                };

                let file_appender = RollingFileAppender::new(
                    rotation,
                    &self.file_directory,
                    &self.file_prefix,
                );

                // Handle all combinations of console and file formats
                match (self.console_format, self.console_writer, self.file_format) {
                    // Pretty + Stdout + Text
                    (ConsoleFormat::Pretty, ConsoleWriter::Stdout, FileFormat::Text) => {
                        let console_layer = fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(std::io::stdout);

                        let file_layer = fmt::layer()
                            .with_ansi(false)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Pretty + Stdout + Json
                    (ConsoleFormat::Pretty, ConsoleWriter::Stdout, FileFormat::Json) => {
                        let console_layer = fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(std::io::stdout);

                        let file_layer = fmt::layer()
                            .json()
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Pretty + Stderr + Text
                    (ConsoleFormat::Pretty, ConsoleWriter::Stderr, FileFormat::Text) => {
                        let console_layer = fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(std::io::stderr);

                        let file_layer = fmt::layer()
                            .with_ansi(false)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Pretty + Stderr + Json
                    (ConsoleFormat::Pretty, ConsoleWriter::Stderr, FileFormat::Json) => {
                        let console_layer = fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(std::io::stderr);

                        let file_layer = fmt::layer()
                            .json()
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Compact + Stdout + Text
                    (ConsoleFormat::Compact, ConsoleWriter::Stdout, FileFormat::Text) => {
                        let console_layer = fmt::layer()
                            .compact()
                            .with_target(true)
                            .with_writer(std::io::stdout);

                        let file_layer = fmt::layer()
                            .with_ansi(false)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Compact + Stdout + Json
                    (ConsoleFormat::Compact, ConsoleWriter::Stdout, FileFormat::Json) => {
                        let console_layer = fmt::layer()
                            .compact()
                            .with_target(true)
                            .with_writer(std::io::stdout);

                        let file_layer = fmt::layer()
                            .json()
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Compact + Stderr + Text
                    (ConsoleFormat::Compact, ConsoleWriter::Stderr, FileFormat::Text) => {
                        let console_layer = fmt::layer()
                            .compact()
                            .with_target(true)
                            .with_writer(std::io::stderr);

                        let file_layer = fmt::layer()
                            .with_ansi(false)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    // Compact + Stderr + Json
                    (ConsoleFormat::Compact, ConsoleWriter::Stderr, FileFormat::Json) => {
                        let console_layer = fmt::layer()
                            .compact()
                            .with_target(true)
                            .with_writer(std::io::stderr);

                        let file_layer = fmt::layer()
                            .json()
                            .with_writer(file_appender);

                        registry
                            .with(console_layer)
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                }
            }
            // Console only
            (true, false) => {
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
            }
            // File only
            (false, true) => {
                // Create log directory if it doesn't exist
                fs::create_dir_all(&self.file_directory)?;

                // Convert rotation policy
                let rotation = match self.rotation_policy {
                    RotationPolicy::Daily => Rotation::DAILY,
                    RotationPolicy::Hourly => Rotation::HOURLY,
                    RotationPolicy::Minutely => Rotation::MINUTELY,
                    RotationPolicy::Never => Rotation::NEVER,
                };

                let file_appender = RollingFileAppender::new(
                    rotation,
                    &self.file_directory,
                    &self.file_prefix,
                );

                // Build file layer based on format
                match self.file_format {
                    FileFormat::Text => {
                        let file_layer = fmt::layer()
                            .with_ansi(false)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                            .with_writer(file_appender);

                        registry
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                    FileFormat::Json => {
                        let file_layer = fmt::layer()
                            .json()
                            .with_writer(file_appender);

                        registry
                            .with(file_layer)
                            .try_init()
                            .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
                    }
                }
            }
            // No output
            (false, false) => {
                registry
                    .try_init()
                    .map_err(|e| LoggingError::InitError(format!("Failed to initialize: {}", e)))?;
            }
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
    file_enabled: bool,
    file_format: FileFormat,
    file_directory: PathBuf,
    file_prefix: String,
    rotation_policy: RotationPolicy,
}

impl Default for LoggingConfigBuilder {
    fn default() -> Self {
        Self {
            filter: "info".to_string(),
            console_format: ConsoleFormat::Pretty,
            console_writer: ConsoleWriter::Stderr,
            console_enabled: true,
            file_enabled: false,
            file_format: FileFormat::Text,
            file_directory: PathBuf::from("logs"),
            file_prefix: "app".to_string(),
            rotation_policy: RotationPolicy::Daily,
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

    /// Enable file output with text format (.log)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_file_text()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_file_text(mut self) -> Self {
        self.file_enabled = true;
        self.file_format = FileFormat::Text;
        self
    }

    /// Enable file output with JSON Lines format (.jsonl)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_file_json()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_file_json(mut self) -> Self {
        self.file_enabled = true;
        self.file_format = FileFormat::Json;
        self
    }

    /// Set the directory for log files
    ///
    /// # Arguments
    ///
    /// * `directory` - Path to the log directory
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_file_directory("./my-logs")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_file_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.file_directory = directory.into();
        self
    }

    /// Set the prefix for log file names
    ///
    /// # Arguments
    ///
    /// * `prefix` - File name prefix (e.g., "app" creates "app.log")
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_file_prefix("myapp")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_file_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.file_prefix = prefix.into();
        self
    }

    /// Set the file rotation policy
    ///
    /// # Arguments
    ///
    /// * `policy` - Rotation policy (Daily, Hourly, Minutely, Never)
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::{LoggingConfig, config::RotationPolicy};
    ///
    /// let config = LoggingConfig::builder()
    ///     .with_rotation_policy(RotationPolicy::Hourly)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_rotation_policy(mut self, policy: RotationPolicy) -> Self {
        self.rotation_policy = policy;
        self
    }

    /// Disable file output
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_logging::LoggingConfig;
    ///
    /// let config = LoggingConfig::builder()
    ///     .without_file()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn without_file(mut self) -> Self {
        self.file_enabled = false;
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
            file_enabled: self.file_enabled,
            file_format: self.file_format,
            file_directory: self.file_directory,
            file_prefix: self.file_prefix,
            rotation_policy: self.rotation_policy,
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

    #[test]
    fn test_builder_with_file_text() {
        let config = LoggingConfig::builder()
            .with_file_text()
            .build()
            .unwrap();
        assert!(config.file_enabled);
        assert_eq!(config.file_format, FileFormat::Text);
    }

    #[test]
    fn test_builder_with_file_json() {
        let config = LoggingConfig::builder()
            .with_file_json()
            .build()
            .unwrap();
        assert!(config.file_enabled);
        assert_eq!(config.file_format, FileFormat::Json);
    }

    #[test]
    fn test_builder_with_file_directory() {
        let config = LoggingConfig::builder()
            .with_file_directory("./test-logs")
            .build()
            .unwrap();
        assert_eq!(config.file_directory, PathBuf::from("./test-logs"));
    }

    #[test]
    fn test_builder_with_file_prefix() {
        let config = LoggingConfig::builder()
            .with_file_prefix("testapp")
            .build()
            .unwrap();
        assert_eq!(config.file_prefix, "testapp");
    }

    #[test]
    fn test_builder_with_rotation_policy() {
        let config = LoggingConfig::builder()
            .with_rotation_policy(RotationPolicy::Hourly)
            .build()
            .unwrap();
        assert_eq!(config.rotation_policy, RotationPolicy::Hourly);
    }

    #[test]
    fn test_builder_without_file() {
        let config = LoggingConfig::builder()
            .without_file()
            .build()
            .unwrap();
        assert!(!config.file_enabled);
    }

    #[test]
    fn test_file_format_variants() {
        assert_eq!(FileFormat::Text, FileFormat::Text);
        assert_eq!(FileFormat::Json, FileFormat::Json);
        assert_ne!(FileFormat::Text, FileFormat::Json);
    }

    #[test]
    fn test_rotation_policy_variants() {
        assert_eq!(RotationPolicy::Daily, RotationPolicy::Daily);
        assert_eq!(RotationPolicy::Hourly, RotationPolicy::Hourly);
        assert_eq!(RotationPolicy::Minutely, RotationPolicy::Minutely);
        assert_eq!(RotationPolicy::Never, RotationPolicy::Never);
    }

    #[test]
    fn test_dual_output_console_and_file() {
        let config = LoggingConfig::builder()
            .with_console_pretty()
            .with_file_text()
            .build()
            .unwrap();
        assert!(config.console_enabled);
        assert!(config.file_enabled);
        assert_eq!(config.console_format, ConsoleFormat::Pretty);
        assert_eq!(config.file_format, FileFormat::Text);
    }

    #[test]
    fn test_file_only_no_console() {
        let config = LoggingConfig::builder()
            .without_console()
            .with_file_json()
            .build()
            .unwrap();
        assert!(!config.console_enabled);
        assert!(config.file_enabled);
        assert_eq!(config.file_format, FileFormat::Json);
    }
}

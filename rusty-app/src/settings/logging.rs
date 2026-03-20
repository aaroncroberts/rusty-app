//! Logging integration for rusty-app settings
//!
//! Converts settings from the settings file into rusty-logging configuration.

use crate::settings::types::{
    ConsoleFormat, ConsoleWriter, FileFormat, LoggingSettings, RotationPolicy,
};
use rusty_logging::LoggingConfig;
use std::path::PathBuf;

/// Convert settings types to rusty-logging types and build configuration
///
/// This function maps the LoggingSettings from our settings file to the
/// rusty-logging LoggingConfig, handling all format conversions and
/// applying the builder pattern.
///
/// # Arguments
///
/// * `settings` - Logging settings from the settings file
///
/// # Examples
///
/// ```no_run
/// use rusty_app::settings::{LoggingSettings, SettingsManager};
/// use rusty_app::settings::logging::build_logging_config;
///
/// let manager = SettingsManager::new("~/.rusty-app").unwrap();
/// let config = build_logging_config(&manager.settings().logging).unwrap();
/// config.apply().unwrap();
/// ```
pub fn build_logging_config(
    settings: &LoggingSettings,
) -> Result<LoggingConfig, rusty_logging::LoggingError> {
    let mut builder = LoggingConfig::builder();

    // Set global filter
    builder = builder.with_filter(&settings.filter);

    // Configure console output
    if settings.console_enabled {
        // Set console format
        builder = match settings.console_format {
            ConsoleFormat::Pretty => builder.with_console_pretty(),
            ConsoleFormat::Compact => builder.with_console_compact(),
        };

        // Set console writer
        builder = match settings.console_writer {
            ConsoleWriter::Stdout => builder.with_console_stdout(),
            ConsoleWriter::Stderr => builder.with_console_stderr(),
        };

        // Set console-specific filter if provided
        if let Some(ref console_filter) = settings.console_filter {
            builder = builder.with_console_filter(console_filter);
        }
    } else {
        builder = builder.without_console();
    }

    // Configure file output
    if settings.file_enabled {
        // Set file format
        builder = match settings.file_format {
            FileFormat::Text => builder.with_file_text(),
            FileFormat::Json => builder.with_file_json(),
        };

        // Expand home directory in file_directory path
        let log_dir = expand_tilde(&settings.file_directory);
        builder = builder.with_file_directory(log_dir);
        builder = builder.with_file_prefix(&settings.file_prefix);

        // Set rotation policy
        builder = builder.with_rotation_policy(convert_rotation_policy(settings.rotation_policy));

        // Set file-specific filter if provided
        if let Some(ref file_filter) = settings.file_filter {
            builder = builder.with_file_filter(file_filter);
        }
    } else {
        builder = builder.without_file();
    }

    builder.build()
}

/// Convert settings RotationPolicy to rusty-logging RotationPolicy
fn convert_rotation_policy(policy: RotationPolicy) -> rusty_logging::config::RotationPolicy {
    match policy {
        RotationPolicy::Daily => rusty_logging::config::RotationPolicy::Daily,
        RotationPolicy::Hourly => rusty_logging::config::RotationPolicy::Hourly,
        RotationPolicy::Minutely => rusty_logging::config::RotationPolicy::Minutely,
        RotationPolicy::Never => rusty_logging::config::RotationPolicy::Never,
    }
}

/// Expand ~ in path to home directory
///
/// This is a simplified version that handles the common case of
/// paths starting with ~/
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_logging_config_defaults() {
        let settings = LoggingSettings::default();
        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_console_pretty() {
        let mut settings = LoggingSettings::default();
        settings.console_format = ConsoleFormat::Pretty;
        settings.console_enabled = true;
        settings.file_enabled = false;

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_console_compact() {
        let mut settings = LoggingSettings::default();
        settings.console_format = ConsoleFormat::Compact;
        settings.console_enabled = true;
        settings.file_enabled = false;

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_file_text() {
        let mut settings = LoggingSettings::default();
        settings.console_enabled = false;
        settings.file_enabled = true;
        settings.file_format = FileFormat::Text;
        settings.file_directory = "/tmp/test-logs".to_string();
        settings.file_prefix = "test".to_string();

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_file_json() {
        let mut settings = LoggingSettings::default();
        settings.console_enabled = false;
        settings.file_enabled = true;
        settings.file_format = FileFormat::Json;
        settings.file_directory = "/tmp/test-logs".to_string();
        settings.file_prefix = "test".to_string();

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_dual_output() {
        let mut settings = LoggingSettings::default();
        settings.console_enabled = true;
        settings.console_format = ConsoleFormat::Compact;
        settings.file_enabled = true;
        settings.file_format = FileFormat::Text;
        settings.file_directory = "/tmp/test-logs".to_string();

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_with_filters() {
        let mut settings = LoggingSettings::default();
        settings.filter = "debug".to_string();
        settings.console_filter = Some("info".to_string());
        settings.file_filter = Some("trace".to_string());
        settings.file_directory = "/tmp/test-logs".to_string();

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }

    #[test]
    fn test_build_logging_config_invalid_filter() {
        let mut settings = LoggingSettings::default();
        settings.filter = "invalid[[[".to_string();

        let config = build_logging_config(&settings);
        assert!(config.is_err());
    }

    #[test]
    fn test_convert_rotation_policy() {
        use rusty_logging::config::RotationPolicy as LogRotationPolicy;

        assert_eq!(
            convert_rotation_policy(RotationPolicy::Daily),
            LogRotationPolicy::Daily
        );
        assert_eq!(
            convert_rotation_policy(RotationPolicy::Hourly),
            LogRotationPolicy::Hourly
        );
        assert_eq!(
            convert_rotation_policy(RotationPolicy::Minutely),
            LogRotationPolicy::Minutely
        );
        assert_eq!(
            convert_rotation_policy(RotationPolicy::Never),
            LogRotationPolicy::Never
        );
    }

    #[test]
    fn test_expand_tilde() {
        // Should expand ~/path
        let expanded = expand_tilde("~/test/path");
        assert!(!expanded.to_string_lossy().contains('~'));

        // Should not modify absolute paths
        let absolute = expand_tilde("/absolute/path");
        assert_eq!(absolute, PathBuf::from("/absolute/path"));

        // Should not modify relative paths without ~
        let relative = expand_tilde("relative/path");
        assert_eq!(relative, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_rotation_policies() {
        let settings_with_policy = |policy| {
            let mut s = LoggingSettings::default();
            s.rotation_policy = policy;
            s.file_enabled = true;
            s.console_enabled = false;
            s.file_directory = "/tmp/test".to_string();
            s
        };

        assert!(build_logging_config(&settings_with_policy(RotationPolicy::Daily)).is_ok());
        assert!(build_logging_config(&settings_with_policy(RotationPolicy::Hourly)).is_ok());
        assert!(build_logging_config(&settings_with_policy(RotationPolicy::Minutely)).is_ok());
        assert!(build_logging_config(&settings_with_policy(RotationPolicy::Never)).is_ok());
    }

    #[test]
    fn test_console_writers() {
        let mut settings = LoggingSettings::default();
        settings.file_enabled = false;

        settings.console_writer = ConsoleWriter::Stdout;
        assert!(build_logging_config(&settings).is_ok());

        settings.console_writer = ConsoleWriter::Stderr;
        assert!(build_logging_config(&settings).is_ok());
    }

    #[test]
    fn test_no_output() {
        let mut settings = LoggingSettings::default();
        settings.console_enabled = false;
        settings.file_enabled = false;

        let config = build_logging_config(&settings);
        assert!(config.is_ok());
    }
}

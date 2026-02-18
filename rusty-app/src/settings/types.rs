//! Settings type definitions

use rusty_data::adapter::ConnectionConfig;
use serde::{Deserialize, Serialize};

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingSettings,

    /// Database connections
    #[serde(default)]
    pub connections: Vec<ConnectionConfig>,

    /// UI preferences
    #[serde(default)]
    pub ui_preferences: UiPreferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            logging: LoggingSettings::default(),
            connections: Vec::new(),
            ui_preferences: UiPreferences::default(),
        }
    }
}

/// Logging configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// Global filter level
    #[serde(default = "default_filter")]
    pub filter: String,

    /// Console-specific filter (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_filter: Option<String>,

    /// Console output format
    #[serde(default)]
    pub console_format: ConsoleFormat,

    /// Console output writer
    #[serde(default)]
    pub console_writer: ConsoleWriter,

    /// Enable console logging
    #[serde(default = "default_true")]
    pub console_enabled: bool,

    /// File-specific filter (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_filter: Option<String>,

    /// Enable file logging
    #[serde(default = "default_true")]
    pub file_enabled: bool,

    /// File output format
    #[serde(default)]
    pub file_format: FileFormat,

    /// Directory for log files
    #[serde(default = "default_log_directory")]
    pub file_directory: String,

    /// Prefix for log file names
    #[serde(default = "default_log_prefix")]
    pub file_prefix: String,

    /// File rotation policy
    #[serde(default)]
    pub rotation_policy: RotationPolicy,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            filter: default_filter(),
            console_filter: None,
            console_format: ConsoleFormat::default(),
            console_writer: ConsoleWriter::default(),
            console_enabled: true,
            file_filter: None,
            file_enabled: true,
            file_format: FileFormat::default(),
            file_directory: default_log_directory(),
            file_prefix: default_log_prefix(),
            rotation_policy: RotationPolicy::default(),
        }
    }
}

/// Console output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleFormat {
    Pretty,
    Compact,
}

impl Default for ConsoleFormat {
    fn default() -> Self {
        ConsoleFormat::Compact
    }
}

/// Console output writer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleWriter {
    Stdout,
    Stderr,
}

impl Default for ConsoleWriter {
    fn default() -> Self {
        ConsoleWriter::Stderr
    }
}

/// File output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Text,
    Json,
}

impl Default for FileFormat {
    fn default() -> Self {
        FileFormat::Text
    }
}

/// File rotation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationPolicy {
    Daily,
    Hourly,
    Minutely,
    Never,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        RotationPolicy::Daily
    }
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    /// UI theme
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Left panel width in pixels
    #[serde(default = "default_panel_width")]
    pub panel_width: u32,

    /// Show left panel on startup
    #[serde(default = "default_true")]
    pub show_left_panel: bool,

    /// Active component on startup
    #[serde(default = "default_active_component")]
    pub active_component: String,

    /// Window width in pixels
    #[serde(default = "default_window_width")]
    pub window_width: u32,

    /// Window height in pixels
    #[serde(default = "default_window_height")]
    pub window_height: u32,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            panel_width: default_panel_width(),
            show_left_panel: true,
            active_component: default_active_component(),
            window_width: default_window_width(),
            window_height: default_window_height(),
        }
    }
}

// Default value functions
fn default_filter() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_log_directory() -> String {
    "~/.rusty-app/logs".to_string()
}

fn default_log_prefix() -> String {
    "rusty-app".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_panel_width() -> u32 {
    250
}

fn default_active_component() -> String {
    "server_list".to_string()
}

fn default_window_width() -> u32 {
    1280
}

fn default_window_height() -> u32 {
    800
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.logging.filter, "info");
        assert!(settings.logging.console_enabled);
        assert!(settings.logging.file_enabled);
        assert_eq!(settings.connections.len(), 0);
        assert_eq!(settings.ui_preferences.theme, "dark");
    }

    #[test]
    fn test_logging_settings_default() {
        let logging = LoggingSettings::default();
        assert_eq!(logging.filter, "info");
        assert_eq!(logging.console_format, ConsoleFormat::Compact);
        assert_eq!(logging.console_writer, ConsoleWriter::Stderr);
        assert!(logging.console_enabled);
        assert_eq!(logging.file_format, FileFormat::Text);
        assert_eq!(logging.file_directory, "~/.rusty-app/logs");
        assert_eq!(logging.file_prefix, "rusty-app");
        assert_eq!(logging.rotation_policy, RotationPolicy::Daily);
    }

    #[test]
    fn test_ui_preferences_default() {
        let ui = UiPreferences::default();
        assert_eq!(ui.theme, "dark");
        assert_eq!(ui.panel_width, 250);
        assert!(ui.show_left_panel);
        assert_eq!(ui.active_component, "server_list");
        assert_eq!(ui.window_width, 1280);
        assert_eq!(ui.window_height, 800);
    }

    #[test]
    fn test_console_format_serialization() {
        assert_eq!(
            serde_json::to_string(&ConsoleFormat::Pretty).unwrap(),
            "\"pretty\""
        );
        assert_eq!(
            serde_json::to_string(&ConsoleFormat::Compact).unwrap(),
            "\"compact\""
        );
    }

    #[test]
    fn test_rotation_policy_serialization() {
        assert_eq!(
            serde_json::to_string(&RotationPolicy::Daily).unwrap(),
            "\"daily\""
        );
        assert_eq!(
            serde_json::to_string(&RotationPolicy::Hourly).unwrap(),
            "\"hourly\""
        );
    }
}

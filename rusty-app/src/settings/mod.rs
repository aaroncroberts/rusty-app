//! Settings management for rusty-app
//!
//! Provides centralized configuration management with atomic saves and reload support.

pub mod logging;
mod types;

pub use types::{
    ConsoleFormat, ConsoleWriter, FileFormat, LoggingSettings, RotationPolicy, Settings,
    UiPreferences,
};

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info};

/// Errors that can occur during settings operations
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Settings validation error: {0}")]
    Validation(String),

    #[error("Path expansion error: {0}")]
    PathExpansion(String),
}

pub type Result<T> = std::result::Result<T, SettingsError>;

/// Settings manager for loading, saving, and reloading application settings
pub struct SettingsManager {
    /// Path to settings directory (e.g., ~/.rusty-app)
    settings_dir: PathBuf,
    /// Path to settings file (e.g., ~/.rusty-app/settings.toml)
    settings_path: PathBuf,
    /// Current settings in memory
    settings: Settings,
}

impl SettingsManager {
    /// Create a new SettingsManager
    ///
    /// # Arguments
    /// * `settings_dir` - Directory containing settings.toml (e.g., "~/.rusty-app")
    ///
    /// # Example
    /// ```no_run
    /// use rusty_app::settings::SettingsManager;
    ///
    /// let manager = SettingsManager::new("~/.rusty-app").unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(settings_dir: P) -> Result<Self> {
        let settings_dir = expand_home_dir(settings_dir.as_ref())?;
        let settings_path = settings_dir.join("settings.toml");

        // Create directory if missing
        if !settings_dir.exists() {
            info!("Creating settings directory: {}", settings_dir.display());
            fs::create_dir_all(&settings_dir)?;
        }

        // Load or create default settings
        let settings = if settings_path.exists() {
            debug!("Loading settings from: {}", settings_path.display());
            Self::load_from_file(&settings_path)?
        } else {
            info!("Settings file not found, creating default");
            let default_settings = Settings::default();
            // Save default settings to file
            Self::write_to_file(&settings_path, &default_settings)?;
            default_settings
        };

        Ok(Self {
            settings_dir,
            settings_path,
            settings,
        })
    }

    /// Get the current settings
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Get mutable reference to settings
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Get the settings directory path
    pub fn settings_dir(&self) -> &Path {
        &self.settings_dir
    }

    /// Get the settings file path
    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    /// Reload settings from disk
    ///
    /// This discards in-memory changes and reloads from the file.
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::settings::SettingsManager;
    /// # let mut manager = SettingsManager::new("~/.rusty-app").unwrap();
    /// manager.reload().unwrap();
    /// ```
    pub fn reload(&mut self) -> Result<()> {
        debug!("Reloading settings from: {}", self.settings_path.display());
        self.settings = Self::load_from_file(&self.settings_path)?;
        info!("Settings reloaded successfully");
        Ok(())
    }

    /// Save current settings to disk
    ///
    /// Settings are saved atomically using a temporary file.
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::settings::SettingsManager;
    /// # let mut manager = SettingsManager::new("~/.rusty-app").unwrap();
    /// manager.settings_mut().ui_preferences.theme = "light".to_string();
    /// manager.save().unwrap();
    /// ```
    pub fn save(&self) -> Result<()> {
        debug!("Saving settings to: {}", self.settings_path.display());
        Self::write_to_file(&self.settings_path, &self.settings)?;
        info!("Settings saved successfully");
        Ok(())
    }

    /// Validate settings
    ///
    /// Checks that settings contain valid values.
    pub fn validate(&self) -> Result<()> {
        Self::validate_settings(&self.settings)
    }

    // Private helper methods

    /// Load settings from a TOML file
    fn load_from_file(path: &Path) -> Result<Settings> {
        let contents = fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&contents)?;
        Self::validate_settings(&settings)?;
        Ok(settings)
    }

    /// Write settings to a file atomically
    fn write_to_file(path: &Path, settings: &Settings) -> Result<()> {
        // Validate before saving
        Self::validate_settings(settings)?;

        // Serialize to TOML
        let contents = toml::to_string_pretty(settings)?;

        // Write atomically: write to temp file, then rename
        let temp_path = path.with_extension("toml.tmp");

        fs::write(&temp_path, contents)?;

        // Atomic rename
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Validate settings
    fn validate_settings(settings: &Settings) -> Result<()> {
        // Validate UI preferences
        let ui = &settings.ui_preferences;

        // Panel width range
        if ui.panel_width < 200 || ui.panel_width > 600 {
            return Err(SettingsError::Validation(format!(
                "panel_width must be between 200 and 600, got {}",
                ui.panel_width
            )));
        }

        // Theme validation
        if ui.theme != "dark" && ui.theme != "light" {
            return Err(SettingsError::Validation(format!(
                "theme must be 'dark' or 'light', got '{}'",
                ui.theme
            )));
        }

        // Active component validation
        let valid_components = ["server_list", "table_list", "properties"];
        if !valid_components.contains(&ui.active_component.as_str()) {
            return Err(SettingsError::Validation(format!(
                "active_component must be one of {:?}, got '{}'",
                valid_components, ui.active_component
            )));
        }

        // Window dimensions
        if ui.window_width == 0 || ui.window_height == 0 {
            return Err(SettingsError::Validation(
                "window dimensions must be positive".to_string(),
            ));
        }

        // Logging validation
        let logging = &settings.logging;

        if logging.file_prefix.trim().is_empty() {
            return Err(SettingsError::Validation(
                "file_prefix cannot be empty".to_string(),
            ));
        }

        if logging.file_directory.trim().is_empty() {
            return Err(SettingsError::Validation(
                "file_directory cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Expand ~ in path to home directory
fn expand_home_dir(path: &Path) -> Result<PathBuf> {
    if let Some(path_str) = path.to_str() {
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join(stripped));
            } else {
                return Err(SettingsError::PathExpansion(
                    "Could not determine home directory".to_string(),
                ));
            }
        }
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arni::{ConnectionConfig, DatabaseType};
    use std::collections::HashMap;

    fn temp_settings_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rusty-app-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn test_settings_manager_new() {
        let dir = temp_settings_dir();
        let manager = SettingsManager::new(&dir).unwrap();

        assert!(dir.exists());
        assert!(manager.settings_path().exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_manager_default_values() {
        let dir = temp_settings_dir();
        let manager = SettingsManager::new(&dir).unwrap();

        let settings = manager.settings();
        assert_eq!(settings.logging.filter, "info");
        assert_eq!(settings.ui_preferences.theme, "dark");
        assert_eq!(settings.connections.len(), 0);

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_manager_save_and_reload() {
        let dir = temp_settings_dir();
        let mut manager = SettingsManager::new(&dir).unwrap();

        // Modify settings
        manager.settings_mut().ui_preferences.theme = "light".to_string();
        manager.settings_mut().logging.filter = "debug".to_string();

        // Save
        manager.save().unwrap();

        // Reload
        manager.reload().unwrap();

        // Verify changes persisted
        assert_eq!(manager.settings().ui_preferences.theme, "light");
        assert_eq!(manager.settings().logging.filter, "debug");

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_manager_atomic_save() {
        let dir = temp_settings_dir();
        let manager = SettingsManager::new(&dir).unwrap();

        // Save should create temp file then rename
        manager.save().unwrap();

        // Temp file should not exist after save
        let temp_path = manager.settings_path().with_extension("toml.tmp");
        assert!(!temp_path.exists());

        // Settings file should exist
        assert!(manager.settings_path().exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_validation_panel_width() {
        let dir = temp_settings_dir();
        let mut manager = SettingsManager::new(&dir).unwrap();

        // Invalid panel width (too small)
        manager.settings_mut().ui_preferences.panel_width = 100;
        let result = manager.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("panel_width must be between 200 and 600"));

        // Invalid panel width (too large)
        manager.settings_mut().ui_preferences.panel_width = 700;
        let result = manager.validate();
        assert!(result.is_err());

        // Valid panel width
        manager.settings_mut().ui_preferences.panel_width = 300;
        let result = manager.validate();
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_validation_theme() {
        let dir = temp_settings_dir();
        let mut manager = SettingsManager::new(&dir).unwrap();

        // Invalid theme
        manager.settings_mut().ui_preferences.theme = "invalid".to_string();
        let result = manager.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("theme must be 'dark' or 'light'"));

        // Valid themes
        manager.settings_mut().ui_preferences.theme = "dark".to_string();
        assert!(manager.validate().is_ok());

        manager.settings_mut().ui_preferences.theme = "light".to_string();
        assert!(manager.validate().is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_validation_active_component() {
        let dir = temp_settings_dir();
        let mut manager = SettingsManager::new(&dir).unwrap();

        // Invalid component
        manager.settings_mut().ui_preferences.active_component = "invalid".to_string();
        let result = manager.validate();
        assert!(result.is_err());

        // Valid components
        for component in &["server_list", "table_list", "properties"] {
            manager.settings_mut().ui_preferences.active_component = component.to_string();
            assert!(manager.validate().is_ok());
        }

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_with_connections() {
        let dir = temp_settings_dir();
        let mut manager = SettingsManager::new(&dir).unwrap();

        // Add a connection
        let conn = ConnectionConfig {
            id: "test-pg".to_string(),
            name: "Test PostgreSQL".to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("testuser".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
            pool_config: None,
        };

        manager.settings_mut().connections.push(conn);

        // Save and reload
        manager.save().unwrap();
        manager.reload().unwrap();

        // Verify connection persisted
        assert_eq!(manager.settings().connections.len(), 1);
        assert_eq!(manager.settings().connections[0].id, "test-pg");
        assert_eq!(
            manager.settings().connections[0].db_type,
            DatabaseType::Postgres
        );

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_expand_home_dir() {
        let path = Path::new("~/test/path");
        let expanded = expand_home_dir(path).unwrap();

        assert!(!expanded.to_string_lossy().contains('~'));

        // Non-tilde paths should pass through
        let regular_path = Path::new("/absolute/path");
        let expanded = expand_home_dir(regular_path).unwrap();
        assert_eq!(expanded, regular_path);
    }

    #[test]
    fn test_settings_manager_missing_directory_created() {
        let dir = temp_settings_dir();

        // Ensure directory doesn't exist
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }

        // Create manager should create directory
        let _manager = SettingsManager::new(&dir).unwrap();

        assert!(dir.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}

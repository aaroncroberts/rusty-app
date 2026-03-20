//! Settings editor UI for modifying application configuration
//!
//! Provides a form interface for editing logging settings and UI preferences.

use crate::settings::{
    ConsoleFormat, ConsoleWriter, FileFormat, LoggingSettings, RotationPolicy, SettingsManager,
    UiPreferences,
};
use crate::theme::ThemeColors;
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Alignment, Element, Fill, Length};

/// Settings editor data (holds editable copies of settings)
#[derive(Debug, Clone)]
pub struct SettingsEditorData {
    pub logging: LoggingSettings,
    pub ui_preferences: UiPreferences,
}

impl SettingsEditorData {
    /// Create new settings editor data from manager
    pub fn from_manager(manager: &SettingsManager) -> Self {
        Self {
            logging: manager.settings().logging.clone(),
            ui_preferences: manager.settings().ui_preferences.clone(),
        }
    }
}

/// Settings editor messages
#[derive(Debug, Clone)]
pub enum SettingsEditorMessage {
    // Logging settings
    FilterChanged(String),
    ConsoleFilterChanged(String),
    FileFilterChanged(String),
    ConsoleFormatChanged(ConsoleFormat),
    ConsoleWriterChanged(ConsoleWriter),
    ConsoleEnabledToggled,
    FileFormatChanged(FileFormat),
    FileEnabledToggled,
    FileDirectoryChanged(String),
    FilePrefixChanged(String),
    RotationPolicyChanged(RotationPolicy),

    // UI preferences
    ThemeChanged(String),
    PanelWidthChanged(String),
    ShowLeftPanelToggled,

    // Actions
    Save,
    Reload,
    Cancel,
}

/// Settings editor component
pub struct SettingsEditor {
    theme: ThemeColors,
}

impl SettingsEditor {
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        data: &SettingsEditorData,
        validation_error: &Option<String>,
        success_message: &Option<String>,
        on_message: impl Fn(SettingsEditorMessage) -> Message + 'a + Copy,
    ) -> Element<'a, Message> {
        let title = text("Application Settings").size(20).color(self.theme.text);

        // Messages section
        let mut messages = column![].spacing(10);

        if let Some(ref error) = validation_error {
            let error_container = container(
                text(format!("❌ {}", error))
                    .size(12)
                    .color([1.0, 0.3, 0.3]),
            )
            .padding(10)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from([0.3, 0.1, 0.1]).into()),
                border: iced::Border {
                    color: [1.0, 0.3, 0.3].into(),
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });
            messages = messages.push(error_container);
        }

        if let Some(ref success) = success_message {
            let success_container = container(
                text(format!("✓ {}", success))
                    .size(12)
                    .color([0.3, 1.0, 0.3]),
            )
            .padding(10)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from([0.1, 0.3, 0.1]).into()),
                border: iced::Border {
                    color: [0.3, 1.0, 0.3].into(),
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });
            messages = messages.push(success_container);
        }

        // Logging settings section
        let logging_section = self.build_logging_section(data, on_message);

        // UI preferences section
        let ui_section = self.build_ui_section(data, on_message);

        // Action buttons
        let buttons = row![
            button(text("Save").size(14))
                .padding(8)
                .on_press(on_message(SettingsEditorMessage::Save)),
            button(text("Reload").size(14))
                .padding(8)
                .on_press(on_message(SettingsEditorMessage::Reload)),
            button(text("Cancel").size(14))
                .padding(8)
                .on_press(on_message(SettingsEditorMessage::Cancel)),
        ]
        .spacing(10);

        let content = column![title, messages, logging_section, ui_section, buttons]
            .spacing(20)
            .padding(20);

        container(scrollable(content).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(self.theme.background.into()),
                ..Default::default()
            })
            .into()
    }

    fn build_logging_section<'a, Message: 'a + Clone>(
        &'a self,
        data: &SettingsEditorData,
        on_message: impl Fn(SettingsEditorMessage) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let section_title = text("Logging").size(16).color(self.theme.text);

        // Global filter
        let global_filter_row = row![
            text("Global Filter:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("info", &data.logging.filter)
                .on_input(move |s| on_message(SettingsEditorMessage::FilterChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Console settings
        let console_title = text("Console Output").size(14).color(self.theme.text);

        let console_enabled = row![
            text("Enabled:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            button(if data.logging.console_enabled {
                "✓ On"
            } else {
                "○ Off"
            })
            .padding(5)
            .on_press(on_message(SettingsEditorMessage::ConsoleEnabledToggled)),
        ]
        .spacing(10);

        let console_format_options = vec![ConsoleFormat::Pretty, ConsoleFormat::Compact];
        let console_format = row![
            text("Format:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            pick_list(
                console_format_options,
                Some(data.logging.console_format),
                move |f| on_message(SettingsEditorMessage::ConsoleFormatChanged(f))
            )
            .padding(5),
        ]
        .spacing(10);

        let console_writer_options = vec![ConsoleWriter::Stdout, ConsoleWriter::Stderr];
        let console_writer = row![
            text("Writer:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            pick_list(
                console_writer_options,
                Some(data.logging.console_writer),
                move |w| on_message(SettingsEditorMessage::ConsoleWriterChanged(w))
            )
            .padding(5),
        ]
        .spacing(10);

        let console_filter_value = data.logging.console_filter.as_deref().unwrap_or("");
        let console_filter = row![
            text("Filter Override:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("(use global)", console_filter_value)
                .on_input(move |s| on_message(SettingsEditorMessage::ConsoleFilterChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        // File settings
        let file_title = text("File Output").size(14).color(self.theme.text);

        let file_enabled = row![
            text("Enabled:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            button(if data.logging.file_enabled {
                "✓ On"
            } else {
                "○ Off"
            })
            .padding(5)
            .on_press(on_message(SettingsEditorMessage::FileEnabledToggled)),
        ]
        .spacing(10);

        let file_format_options = vec![FileFormat::Text, FileFormat::Json];
        let file_format = row![
            text("Format:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            pick_list(
                file_format_options,
                Some(data.logging.file_format),
                move |f| on_message(SettingsEditorMessage::FileFormatChanged(f))
            )
            .padding(5),
        ]
        .spacing(10);

        let file_directory = row![
            text("Directory:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("~/.rusty-app/logs", &data.logging.file_directory)
                .on_input(move |s| on_message(SettingsEditorMessage::FileDirectoryChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        let file_prefix = row![
            text("File Prefix:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("rusty-app", &data.logging.file_prefix)
                .on_input(move |s| on_message(SettingsEditorMessage::FilePrefixChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        let rotation_options = vec![
            RotationPolicy::Daily,
            RotationPolicy::Hourly,
            RotationPolicy::Minutely,
            RotationPolicy::Never,
        ];
        let rotation_policy = row![
            text("Rotation:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            pick_list(
                rotation_options,
                Some(data.logging.rotation_policy),
                move |p| on_message(SettingsEditorMessage::RotationPolicyChanged(p))
            )
            .padding(5),
        ]
        .spacing(10);

        let file_filter_value = data.logging.file_filter.as_deref().unwrap_or("");
        let file_filter = row![
            text("Filter Override:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("(use global)", file_filter_value)
                .on_input(move |s| on_message(SettingsEditorMessage::FileFilterChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        let logging_content = column![
            section_title,
            text("──────────────────────────")
                .size(10)
                .color(self.theme.border),
            global_filter_row,
            console_title,
            console_enabled,
            console_format,
            console_writer,
            console_filter,
            file_title,
            file_enabled,
            file_format,
            file_directory,
            file_prefix,
            rotation_policy,
            file_filter,
        ]
        .spacing(8);

        container(logging_content)
            .padding(10)
            .style(|_theme| container::Style {
                background: Some(self.theme.background_secondary.into()),
                border: iced::Border {
                    color: self.theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn build_ui_section<'a, Message: 'a + Clone>(
        &'a self,
        data: &SettingsEditorData,
        on_message: impl Fn(SettingsEditorMessage) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let section_title = text("UI Preferences").size(16).color(self.theme.text);

        let theme_row = row![
            text("Theme:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("dark", &data.ui_preferences.theme)
                .on_input(move |s| on_message(SettingsEditorMessage::ThemeChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        let panel_width_row = row![
            text("Panel Width:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            text_input("250", &data.ui_preferences.panel_width.to_string())
                .on_input(move |s| on_message(SettingsEditorMessage::PanelWidthChanged(s)))
                .padding(5)
                .size(12),
        ]
        .spacing(10);

        let show_panel_row = row![
            text("Show Left Panel:")
                .size(12)
                .color(self.theme.text)
                .width(Length::Fixed(150.0)),
            button(if data.ui_preferences.show_left_panel {
                "✓ On"
            } else {
                "○ Off"
            })
            .padding(5)
            .on_press(on_message(SettingsEditorMessage::ShowLeftPanelToggled)),
        ]
        .spacing(10);

        let ui_content = column![
            section_title,
            text("──────────────────────────")
                .size(10)
                .color(self.theme.border),
            theme_row,
            panel_width_row,
            show_panel_row,
        ]
        .spacing(8);

        container(ui_content)
            .padding(10)
            .style(|_theme| container::Style {
                background: Some(self.theme.background_secondary.into()),
                border: iced::Border {
                    color: self.theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}

// Implement Display for pick_list types
impl std::fmt::Display for ConsoleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsoleFormat::Pretty => write!(f, "Pretty"),
            ConsoleFormat::Compact => write!(f, "Compact"),
        }
    }
}

impl std::fmt::Display for ConsoleWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsoleWriter::Stdout => write!(f, "Stdout"),
            ConsoleWriter::Stderr => write!(f, "Stderr"),
        }
    }
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Text => write!(f, "Text"),
            FileFormat::Json => write!(f, "JSON"),
        }
    }
}

impl std::fmt::Display for RotationPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotationPolicy::Daily => write!(f, "Daily"),
            RotationPolicy::Hourly => write!(f, "Hourly"),
            RotationPolicy::Minutely => write!(f, "Minutely"),
            RotationPolicy::Never => write!(f, "Never"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_settings_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rusty-app-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn test_settings_editor_data_from_manager() {
        let dir = temp_settings_dir();
        let manager = SettingsManager::new(&dir).unwrap();
        let data = SettingsEditorData::from_manager(&manager);

        assert_eq!(data.logging.filter, "info");
        assert_eq!(data.ui_preferences.theme, "dark");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_editor_creation() {
        let theme = ThemeColors::dark();
        let editor = SettingsEditor::new(theme);
        // Just verify it compiles and creates
        let _ = editor;
    }

    #[test]
    fn test_display_implementations() {
        assert_eq!(format!("{}", ConsoleFormat::Pretty), "Pretty");
        assert_eq!(format!("{}", ConsoleFormat::Compact), "Compact");
        assert_eq!(format!("{}", ConsoleWriter::Stdout), "Stdout");
        assert_eq!(format!("{}", ConsoleWriter::Stderr), "Stderr");
        assert_eq!(format!("{}", FileFormat::Text), "Text");
        assert_eq!(format!("{}", FileFormat::Json), "JSON");
        assert_eq!(format!("{}", RotationPolicy::Daily), "Daily");
        assert_eq!(format!("{}", RotationPolicy::Never), "Never");
    }
}

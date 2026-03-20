//! SQLite connection form component
//!
//! Provides a form interface specifically for SQLite connections with
//! connection testing functionality.

use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Border, Element, Fill};

/// SQLite connection form data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SQLiteConnectionFormData {
    pub name: String,
    pub file_path: String,
}

impl SQLiteConnectionFormData {
    /// Create a new SQLite connection form with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the form data
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Connection name is required".to_string());
        }
        if self.file_path.trim().is_empty() {
            return Err("File path is required".to_string());
        }
        Ok(())
    }
}

/// Messages for the SQLite connection form
#[derive(Debug, Clone)]
pub enum SQLiteConnectionFormMessage {
    NameChanged(String),
    FilePathChanged(String),
    TestConnection,
    Save,
    Cancel,
}

/// SQLite connection form component
#[derive(Debug, Clone)]
pub struct SQLiteConnectionForm {
    theme: ThemeColors,
}

impl SQLiteConnectionForm {
    /// Create a new SQLite connection form with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the SQLite connection form
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        data: &'a SQLiteConnectionFormData,
        testing_connection: bool,
        test_result: &'a Option<Result<(), String>>,
        on_message: impl Fn(SQLiteConnectionFormMessage) -> Message + 'a + Copy,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        let title = text("New SQLite Connection").size(18).color(theme.text);

        let mut form_fields = column![
            title,
            // Connection name
            self.form_field(
                "Connection Name",
                text_input("My SQLite DB", &data.name)
                    .on_input(move |s| on_message(SQLiteConnectionFormMessage::NameChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // File path
            self.form_field(
                "Database File Path",
                text_input("/path/to/database.db", &data.file_path)
                    .on_input(move |s| on_message(SQLiteConnectionFormMessage::FilePathChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
        ]
        .spacing(15);

        // Test result display
        if let Some(result) = test_result {
            let result_text = match result {
                Ok(_) => text("✓ Connection successful!")
                    .size(14)
                    .color([0.3, 1.0, 0.3]),
                Err(e) => text(format!("✗ Connection failed: {}", e))
                    .size(14)
                    .color([1.0, 0.3, 0.3]),
            };
            form_fields = form_fields.push(result_text);
        }

        // Action buttons
        let test_button = if testing_connection {
            button(text("Testing...").size(14))
                .padding([8, 16])
                .style(move |_theme, _status| button::Style {
                    background: Some(theme.background_secondary.into()),
                    text_color: theme.text_secondary,
                    border: Border {
                        color: theme.border,
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                })
        } else {
            button(text("Test Connection").size(14))
                .padding([8, 16])
                .on_press(on_message(SQLiteConnectionFormMessage::TestConnection))
                .style(move |_theme, status| button::Style {
                    background: Some(theme.background_secondary.into()),
                    text_color: theme.text,
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            theme.accent
                        } else {
                            theme.border
                        },
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                })
        };

        let buttons = row![
            button(text("Cancel").size(14))
                .padding([8, 16])
                .on_press(on_message(SQLiteConnectionFormMessage::Cancel))
                .style(move |_theme, status| button::Style {
                    background: Some(theme.background_secondary.into()),
                    text_color: theme.text,
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            theme.accent
                        } else {
                            theme.border
                        },
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            test_button,
            button(text("Save Connection").size(14))
                .padding([8, 16])
                .on_press(on_message(SQLiteConnectionFormMessage::Save))
                .style(move |_theme, status| button::Style {
                    background: Some(theme.accent.into()),
                    text_color: theme.text,
                    border: Border {
                        color: if matches!(status, button::Status::Hovered) {
                            theme.text
                        } else {
                            theme.accent
                        },
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        ]
        .spacing(10);

        form_fields = form_fields.push(buttons);

        container(form_fields)
            .width(Fill)
            .padding(20)
            .style(move |_theme| container::Style {
                background: Some(theme.background.into()),
                ..Default::default()
            })
            .into()
    }

    /// Create a labeled form field
    fn form_field<'a, Message: 'a>(
        &'a self,
        label: &'a str,
        input: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        column![
            text(label).size(12).color(self.theme.text_secondary),
            input.into(),
        ]
        .spacing(5)
        .into()
    }

    /// Style for text inputs
    fn input_style(&self, _theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
        let theme = self.theme;
        text_input::Style {
            background: theme.background_secondary.into(),
            border: Border {
                color: if matches!(status, text_input::Status::Focused) {
                    theme.accent
                } else {
                    theme.border
                },
                width: 1.0,
                ..Default::default()
            },
            icon: theme.text_secondary,
            placeholder: theme.text_secondary,
            value: theme.text,
            selection: theme.accent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_connection_form_data_default() {
        let data = SQLiteConnectionFormData::default();
        assert!(data.name.is_empty());
        assert!(data.file_path.is_empty());
    }

    #[test]
    fn test_sqlite_connection_form_data_new() {
        let data = SQLiteConnectionFormData::new();
        assert!(data.name.is_empty());
    }

    #[test]
    fn test_validate_sqlite_valid() {
        let data = SQLiteConnectionFormData {
            name: "Test".to_string(),
            file_path: "/path/to/db.sqlite".to_string(),
        };
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_validate_sqlite_missing_name() {
        let data = SQLiteConnectionFormData {
            name: "".to_string(),
            file_path: "/path/to/db.sqlite".to_string(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "Connection name is required");
    }

    #[test]
    fn test_validate_sqlite_missing_path() {
        let data = SQLiteConnectionFormData {
            name: "Test".to_string(),
            file_path: "".to_string(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "File path is required");
    }

    #[test]
    fn test_sqlite_connection_form_creation() {
        let theme = ThemeColors::dark();
        let form = SQLiteConnectionForm::new(theme);
        assert_eq!(form.theme, theme);
    }
}

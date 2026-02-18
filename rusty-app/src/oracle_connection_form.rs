//! Oracle connection form component
//!
//! Provides a form interface specifically for Oracle connections with
//! connection testing functionality.

use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Border, Element, Fill};

/// Oracle connection form data
#[derive(Debug, Clone, PartialEq)]
pub struct OracleConnectionFormData {
    pub name: String,
    pub host: String,
    pub port: String,
    pub database: String, // SID or Service Name
    pub username: String,
    pub password: String,
}

impl Default for OracleConnectionFormData {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: "localhost".to_string(),
            port: "1521".to_string(),
            database: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl OracleConnectionFormData {
    /// Create a new Oracle connection form with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the form data
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Connection name is required".to_string());
        }
        if self.host.trim().is_empty() {
            return Err("Host is required".to_string());
        }
        if self.database.trim().is_empty() {
            return Err("SID/Service Name is required".to_string());
        }
        if self.port.trim().is_empty() {
            return Err("Port is required".to_string());
        }
        if self.port.parse::<u16>().is_err() {
            return Err("Port must be a valid number (1-65535)".to_string());
        }
        Ok(())
    }
}

/// Messages for the Oracle connection form
#[derive(Debug, Clone)]
pub enum OracleConnectionFormMessage {
    NameChanged(String),
    HostChanged(String),
    PortChanged(String),
    DatabaseChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    TestConnection,
    Save,
    Cancel,
}

/// Oracle connection form component
#[derive(Debug, Clone)]
pub struct OracleConnectionForm {
    theme: ThemeColors,
}

impl OracleConnectionForm {
    /// Create a new Oracle connection form with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the Oracle connection form
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        data: &'a OracleConnectionFormData,
        testing_connection: bool,
        test_result: &'a Option<Result<(), String>>,
        on_message: impl Fn(OracleConnectionFormMessage) -> Message + 'a + Copy,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        let title = text("New Oracle Connection")
            .size(18)
            .color(theme.text);

        let mut form_fields = column![
            title,
            // Connection name
            self.form_field(
                "Connection Name",
                text_input("My Oracle DB", &data.name)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::NameChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Host
            self.form_field(
                "Host",
                text_input("localhost", &data.host)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::HostChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Port
            self.form_field(
                "Port",
                text_input("1521", &data.port)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::PortChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Database (SID or Service Name)
            self.form_field(
                "SID/Service Name",
                text_input("ORCL", &data.database)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::DatabaseChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Username
            self.form_field(
                "Username",
                text_input("system", &data.username)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::UsernameChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Password
            self.form_field(
                "Password",
                text_input("password", &data.password)
                    .on_input(move |s| on_message(OracleConnectionFormMessage::PasswordChanged(s)))
                    .padding(8)
                    .secure(true)
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
                .on_press(on_message(OracleConnectionFormMessage::TestConnection))
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
                .on_press(on_message(OracleConnectionFormMessage::Cancel))
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
                .on_press(on_message(OracleConnectionFormMessage::Save))
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
    fn test_oracle_connection_form_data_default() {
        let data = OracleConnectionFormData::default();
        assert_eq!(data.host, "localhost");
        assert_eq!(data.port, "1521");
        assert!(data.name.is_empty());
        assert!(data.database.is_empty());
    }

    #[test]
    fn test_oracle_connection_form_data_new() {
        let data = OracleConnectionFormData::new();
        assert_eq!(data.port, "1521");
    }

    #[test]
    fn test_validate_oracle_valid() {
        let data = OracleConnectionFormData {
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: "1521".to_string(),
            database: "ORCL".to_string(),
            username: "system".to_string(),
            password: "pass".to_string(),
        };
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_validate_oracle_missing_name() {
        let data = OracleConnectionFormData {
            name: "".to_string(),
            host: "localhost".to_string(),
            port: "1521".to_string(),
            database: "ORCL".to_string(),
            username: "system".to_string(),
            password: "pass".to_string(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "Connection name is required");
    }

    #[test]
    fn test_oracle_connection_form_creation() {
        let theme = ThemeColors::dark();
        let form = OracleConnectionForm::new(theme);
        assert_eq!(form.theme, theme);
    }
}

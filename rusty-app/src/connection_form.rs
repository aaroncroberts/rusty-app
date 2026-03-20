//! Connection form component for creating database connections
//!
//! Provides a form interface for entering connection details including
//! database type selection and connection parameters.

use crate::theme::ThemeColors;
use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Border, Element, Fill};
use arni::DatabaseType;

/// Wrapper for DatabaseType to implement Display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DisplayableDatabaseType(DatabaseType);

impl From<DatabaseType> for DisplayableDatabaseType {
    fn from(db_type: DatabaseType) -> Self {
        Self(db_type)
    }
}

impl From<DisplayableDatabaseType> for DatabaseType {
    fn from(dt: DisplayableDatabaseType) -> Self {
        dt.0
    }
}

impl std::fmt::Display for DisplayableDatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            DatabaseType::Postgres => write!(f, "PostgreSQL"),
            DatabaseType::MySQL => write!(f, "MySQL"),
            DatabaseType::SQLite => write!(f, "SQLite"),
            DatabaseType::MongoDB => write!(f, "MongoDB"),
            DatabaseType::SQLServer => write!(f, "SQL Server"),
            DatabaseType::Oracle => write!(f, "Oracle"),
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
        }
    }
}

/// Connection form data
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionFormData {
    pub db_type: DatabaseType,
    pub name: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub file_path: String, // For SQLite
}

impl Default for ConnectionFormData {
    fn default() -> Self {
        Self {
            db_type: DatabaseType::Postgres,
            name: String::new(),
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: String::new(),
            username: String::new(),
            password: String::new(),
            file_path: String::new(),
        }
    }
}

impl ConnectionFormData {
    /// Create a new connection form with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the database type and adjust default port
    pub fn with_db_type(mut self, db_type: DatabaseType) -> Self {
        self.db_type = db_type;
        if let Some(port) = db_type.default_port() {
            self.port = port.to_string();
        }
        self
    }

    /// Validate the form data
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Connection name is required".to_string());
        }

        match self.db_type {
            DatabaseType::SQLite => {
                if self.file_path.trim().is_empty() {
                    return Err("File path is required for SQLite".to_string());
                }
            }
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
            DatabaseType::Postgres
            | DatabaseType::MySQL
            | DatabaseType::MongoDB
            | DatabaseType::SQLServer
            | DatabaseType::Oracle => {
                if self.host.trim().is_empty() {
                    return Err("Host is required".to_string());
                }
                if self.database.trim().is_empty() {
                    return Err("Database is required".to_string());
                }
                if self.port.trim().is_empty() {
                    return Err("Port is required".to_string());
                }
                if self.port.parse::<u16>().is_err() {
                    return Err("Port must be a valid number (1-65535)".to_string());
                }
            }
        }

        Ok(())
    }
}

/// Messages for the connection form
#[derive(Debug, Clone)]
pub enum ConnectionFormMessage {
    NameChanged(String),
    DbTypeChanged(DatabaseType),
    HostChanged(String),
    PortChanged(String),
    DatabaseChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    FilePathChanged(String),
    TestConnection,
    Save,
    Cancel,
}

/// Connection form component
#[derive(Debug, Clone)]
pub struct ConnectionForm {
    theme: ThemeColors,
}

impl ConnectionForm {
    /// Create a new connection form with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the connection form
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        data: &'a ConnectionFormData,
        on_message: impl Fn(ConnectionFormMessage) -> Message + 'a + Copy,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Database type picker
        let db_type_picker = pick_list(
            vec![
                DisplayableDatabaseType(DatabaseType::Postgres),
                DisplayableDatabaseType(DatabaseType::MySQL),
                DisplayableDatabaseType(DatabaseType::SQLite),
            ],
            Some(DisplayableDatabaseType(data.db_type)),
            move |dt: DisplayableDatabaseType| {
                on_message(ConnectionFormMessage::DbTypeChanged(dt.0))
            },
        )
        .padding(8)
        .style(move |_theme, status| pick_list::Style {
            text_color: theme.text,
            placeholder_color: theme.text_secondary,
            handle_color: theme.text_secondary,
            background: theme.background_secondary.into(),
            border: Border {
                color: if matches!(
                    status,
                    pick_list::Status::Active | pick_list::Status::Hovered
                ) {
                    theme.accent
                } else {
                    theme.border
                },
                width: 1.0,
                ..Default::default()
            },
        });

        let mut form_fields = column![
            // Connection name
            self.form_field(
                "Connection Name",
                text_input("My Connection", &data.name)
                    .on_input(move |s| on_message(ConnectionFormMessage::NameChanged(s)))
                    .padding(8)
                    .style(|theme, status| self.input_style(theme, status))
            ),
            // Database type
            self.form_field("Database Type", db_type_picker),
        ]
        .spacing(15);

        // Show different fields based on database type
        match data.db_type {
            DatabaseType::SQLite => {
                form_fields = form_fields.push(
                    self.form_field(
                        "File Path",
                        text_input("/path/to/database.db", &data.file_path)
                            .on_input(move |s| {
                                on_message(ConnectionFormMessage::FilePathChanged(s))
                            })
                            .padding(8)
                            .style(|theme, status| self.input_style(theme, status)),
                    ),
                );
            }
            DatabaseType::Postgres
            | DatabaseType::MySQL
            | DatabaseType::MongoDB
            | DatabaseType::SQLServer
            | DatabaseType::Oracle => {
                form_fields = form_fields
                    .push(
                        self.form_field(
                            "Host",
                            text_input("localhost", &data.host)
                                .on_input(move |s| {
                                    on_message(ConnectionFormMessage::HostChanged(s))
                                })
                                .padding(8)
                                .style(|theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            "Port",
                            text_input("5432", &data.port)
                                .on_input(move |s| {
                                    on_message(ConnectionFormMessage::PortChanged(s))
                                })
                                .padding(8)
                                .style(|theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            "Database",
                            text_input("mydb", &data.database)
                                .on_input(move |s| {
                                    on_message(ConnectionFormMessage::DatabaseChanged(s))
                                })
                                .padding(8)
                                .style(|theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            "Username",
                            text_input("user", &data.username)
                                .on_input(move |s| {
                                    on_message(ConnectionFormMessage::UsernameChanged(s))
                                })
                                .padding(8)
                                .style(|theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            "Password",
                            text_input("password", &data.password)
                                .on_input(move |s| {
                                    on_message(ConnectionFormMessage::PasswordChanged(s))
                                })
                                .padding(8)
                                .secure(true)
                                .style(|theme, status| self.input_style(theme, status)),
                        ),
                    );
            }
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
        }

        // Action buttons
        let buttons = row![
            button(text("Cancel").size(14))
                .padding([8, 16])
                .on_press(on_message(ConnectionFormMessage::Cancel))
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
            button(text("Test Connection").size(14))
                .padding([8, 16])
                .on_press(on_message(ConnectionFormMessage::TestConnection))
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
            button(text("Save Connection").size(14))
                .padding([8, 16])
                .on_press(on_message(ConnectionFormMessage::Save))
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
    fn test_connection_form_data_default() {
        let data = ConnectionFormData::default();
        assert_eq!(data.db_type, DatabaseType::Postgres);
        assert_eq!(data.host, "localhost");
        assert_eq!(data.port, "5432");
        assert!(data.name.is_empty());
        assert!(data.database.is_empty());
    }

    #[test]
    fn test_connection_form_data_new() {
        let data = ConnectionFormData::new();
        assert_eq!(data.db_type, DatabaseType::Postgres);
    }

    #[test]
    fn test_with_db_type_postgres() {
        let data = ConnectionFormData::new().with_db_type(DatabaseType::Postgres);
        assert_eq!(data.db_type, DatabaseType::Postgres);
        assert_eq!(data.port, "5432");
    }

    #[test]
    fn test_with_db_type_mysql() {
        let data = ConnectionFormData::new().with_db_type(DatabaseType::MySQL);
        assert_eq!(data.db_type, DatabaseType::MySQL);
        assert_eq!(data.port, "3306");
    }

    #[test]
    fn test_with_db_type_sqlite() {
        let data = ConnectionFormData::new().with_db_type(DatabaseType::SQLite);
        assert_eq!(data.db_type, DatabaseType::SQLite);
    }

    #[test]
    fn test_validate_postgres_valid() {
        let data = ConnectionFormData {
            db_type: DatabaseType::Postgres,
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            file_path: String::new(),
        };
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_validate_postgres_missing_name() {
        let data = ConnectionFormData {
            db_type: DatabaseType::Postgres,
            name: "".to_string(),
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            file_path: String::new(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "Connection name is required");
    }

    #[test]
    fn test_validate_postgres_missing_host() {
        let data = ConnectionFormData {
            db_type: DatabaseType::Postgres,
            name: "Test".to_string(),
            host: "".to_string(),
            port: "5432".to_string(),
            database: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            file_path: String::new(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "Host is required");
    }

    #[test]
    fn test_validate_postgres_missing_database() {
        let data = ConnectionFormData {
            db_type: DatabaseType::Postgres,
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            file_path: String::new(),
        };
        assert!(data.validate().is_err());
        assert_eq!(data.validate().unwrap_err(), "Database is required");
    }

    #[test]
    fn test_validate_postgres_invalid_port() {
        let data = ConnectionFormData {
            db_type: DatabaseType::Postgres,
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: "invalid".to_string(),
            database: "testdb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            file_path: String::new(),
        };
        assert!(data.validate().is_err());
        assert_eq!(
            data.validate().unwrap_err(),
            "Port must be a valid number (1-65535)"
        );
    }

    #[test]
    fn test_validate_sqlite_valid() {
        let data = ConnectionFormData {
            db_type: DatabaseType::SQLite,
            name: "Test".to_string(),
            host: String::new(),
            port: String::new(),
            database: String::new(),
            username: String::new(),
            password: String::new(),
            file_path: "/path/to/db.sqlite".to_string(),
        };
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_validate_sqlite_missing_path() {
        let data = ConnectionFormData {
            db_type: DatabaseType::SQLite,
            name: "Test".to_string(),
            host: String::new(),
            port: String::new(),
            database: String::new(),
            username: String::new(),
            password: String::new(),
            file_path: "".to_string(),
        };
        assert!(data.validate().is_err());
        assert_eq!(
            data.validate().unwrap_err(),
            "File path is required for SQLite"
        );
    }

    #[test]
    fn test_connection_form_creation() {
        let theme = ThemeColors::dark();
        let form = ConnectionForm::new(theme);
        assert_eq!(form.theme, theme);
    }

    #[test]
    fn test_database_type_display() {
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::Postgres).to_string(),
            "PostgreSQL"
        );
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::MySQL).to_string(),
            "MySQL"
        );
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::SQLite).to_string(),
            "SQLite"
        );
    }
}

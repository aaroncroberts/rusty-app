//! Connection form component for creating database connections

use crate::components::{Component, ComponentAction, ComponentId, ConnectionFormAction};
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

/// Connection form component with internal state
#[derive(Debug, Clone)]
pub struct ConnectionFormComponent {
    data: ConnectionFormData,
}

impl ConnectionFormComponent {
    /// Create a new connection form component
    pub fn new() -> Self {
        Self {
            data: ConnectionFormData::new(),
        }
    }

    /// Get the current form data
    pub fn data(&self) -> &ConnectionFormData {
        &self.data
    }

    /// Get mutable reference to form data
    pub fn data_mut(&mut self) -> &mut ConnectionFormData {
        &mut self.data
    }

    /// Reset the form to default values
    pub fn reset(&mut self) {
        self.data = ConnectionFormData::new();
    }

    /// Update the component based on an action
    pub fn update(&mut self, action: ConnectionFormAction) {
        match action {
            ConnectionFormAction::NameChanged(name) => {
                self.data.name = name;
            }
            ConnectionFormAction::DbTypeChanged(db_type) => {
                self.data = self.data.clone().with_db_type(db_type);
            }
            ConnectionFormAction::HostChanged(host) => {
                self.data.host = host;
            }
            ConnectionFormAction::PortChanged(port) => {
                self.data.port = port;
            }
            ConnectionFormAction::DatabaseChanged(database) => {
                self.data.database = database;
            }
            ConnectionFormAction::UsernameChanged(username) => {
                self.data.username = username;
            }
            ConnectionFormAction::PasswordChanged(password) => {
                self.data.password = password;
            }
            ConnectionFormAction::FilePathChanged(file_path) => {
                self.data.file_path = file_path;
            }
            // Save, Cancel, TestConnection handled by parent
            ConnectionFormAction::Save
            | ConnectionFormAction::Cancel
            | ConnectionFormAction::TestConnection => {}
        }
    }

    /// Render a labeled form field
    fn form_field<'a>(
        &self,
        theme: ThemeColors,
        label: &'a str,
        input: impl Into<Element<'a, ComponentAction>>,
    ) -> Element<'a, ComponentAction> {
        column![
            text(label).size(12).color(theme.text_secondary),
            input.into(),
        ]
        .spacing(5)
        .into()
    }

    /// Style for text inputs
    fn input_style(&self, theme: ThemeColors, status: text_input::Status) -> text_input::Style {
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

impl Default for ConnectionFormComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ConnectionFormComponent {
    fn id(&self) -> ComponentId {
        ComponentId::ConnectionForm
    }

    fn view(&self, theme: ThemeColors) -> Element<ComponentAction> {
        let data = &self.data;

        // Database type picker
        let db_type_picker = pick_list(
            vec![
                DisplayableDatabaseType(DatabaseType::Postgres),
                DisplayableDatabaseType(DatabaseType::MySQL),
                DisplayableDatabaseType(DatabaseType::SQLite),
            ],
            Some(DisplayableDatabaseType(data.db_type)),
            |dt: DisplayableDatabaseType| {
                ComponentAction::ConnectionForm(ConnectionFormAction::DbTypeChanged(dt.0))
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
                theme,
                "Connection Name",
                text_input("My Connection", &data.name)
                    .on_input(|s| ComponentAction::ConnectionForm(
                        ConnectionFormAction::NameChanged(s)
                    ))
                    .padding(8)
                    .style(move |_theme, status| self.input_style(theme, status))
            ),
            // Database type
            self.form_field(theme, "Database Type", db_type_picker),
        ]
        .spacing(15);

        // Show different fields based on database type
        match data.db_type {
            DatabaseType::SQLite => {
                form_fields = form_fields.push(
                    self.form_field(
                        theme,
                        "File Path",
                        text_input("/path/to/database.db", &data.file_path)
                            .on_input(|s| {
                                ComponentAction::ConnectionForm(
                                    ConnectionFormAction::FilePathChanged(s),
                                )
                            })
                            .padding(8)
                            .style(move |_theme, status| self.input_style(theme, status)),
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
                            theme,
                            "Host",
                            text_input("localhost", &data.host)
                                .on_input(|s| {
                                    ComponentAction::ConnectionForm(
                                        ConnectionFormAction::HostChanged(s),
                                    )
                                })
                                .padding(8)
                                .style(move |_theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            theme,
                            "Port",
                            text_input("5432", &data.port)
                                .on_input(|s| {
                                    ComponentAction::ConnectionForm(
                                        ConnectionFormAction::PortChanged(s),
                                    )
                                })
                                .padding(8)
                                .style(move |_theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            theme,
                            "Database",
                            text_input("mydb", &data.database)
                                .on_input(|s| {
                                    ComponentAction::ConnectionForm(
                                        ConnectionFormAction::DatabaseChanged(s),
                                    )
                                })
                                .padding(8)
                                .style(move |_theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            theme,
                            "Username",
                            text_input("user", &data.username)
                                .on_input(|s| {
                                    ComponentAction::ConnectionForm(
                                        ConnectionFormAction::UsernameChanged(s),
                                    )
                                })
                                .padding(8)
                                .style(move |_theme, status| self.input_style(theme, status)),
                        ),
                    )
                    .push(
                        self.form_field(
                            theme,
                            "Password",
                            text_input("password", &data.password)
                                .on_input(|s| {
                                    ComponentAction::ConnectionForm(
                                        ConnectionFormAction::PasswordChanged(s),
                                    )
                                })
                                .padding(8)
                                .secure(true)
                                .style(move |_theme, status| self.input_style(theme, status)),
                        ),
                    );
            }
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
        }

        // Action buttons
        let buttons = row![
            button(text("Cancel").size(14))
                .padding([8, 16])
                .on_press(ComponentAction::ConnectionForm(
                    ConnectionFormAction::Cancel
                ))
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
                .on_press(ComponentAction::ConnectionForm(
                    ConnectionFormAction::TestConnection
                ))
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
                .on_press(ComponentAction::ConnectionForm(ConnectionFormAction::Save))
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
    fn test_connection_form_component_new() {
        let component = ConnectionFormComponent::new();
        assert_eq!(component.id(), ComponentId::ConnectionForm);
        assert_eq!(component.data().db_type, DatabaseType::Postgres);
    }

    #[test]
    fn test_connection_form_component_update() {
        let mut component = ConnectionFormComponent::new();

        component.update(ConnectionFormAction::NameChanged("Test".to_string()));
        assert_eq!(component.data().name, "Test");

        component.update(ConnectionFormAction::HostChanged("192.168.1.1".to_string()));
        assert_eq!(component.data().host, "192.168.1.1");
    }

    #[test]
    fn test_connection_form_component_reset() {
        let mut component = ConnectionFormComponent::new();
        component.update(ConnectionFormAction::NameChanged("Test".to_string()));
        assert_eq!(component.data().name, "Test");

        component.reset();
        assert!(component.data().name.is_empty());
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
}

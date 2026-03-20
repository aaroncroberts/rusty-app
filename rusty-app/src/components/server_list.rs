//! Server list component for displaying database connections

use crate::button_styles;
use crate::components::{Component, ComponentAction, ComponentId, ServerListAction};
use crate::icons;
use crate::theme::ThemeColors;
use iced::widget::{button, column, container, horizontal_space, row, scrollable, text};
use iced::{Border, Element, Fill};
use arni::ConnectionConfig;

/// Server list component with internal connection list
#[derive(Debug, Clone)]
pub struct ServerListComponent {
    connections: Vec<ConnectionConfig>,
}

impl ServerListComponent {
    /// Create a new server list component with empty connections
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    /// Get the current connections
    pub fn connections(&self) -> &[ConnectionConfig] {
        &self.connections
    }

    /// Set the connections list
    pub fn set_connections(&mut self, connections: Vec<ConnectionConfig>) {
        self.connections = connections;
    }

    /// Add a connection to the list
    pub fn add_connection(&mut self, connection: ConnectionConfig) {
        self.connections.push(connection);
    }

    /// Remove a connection by ID
    pub fn remove_connection(&mut self, id: &str) -> bool {
        if let Some(pos) = self.connections.iter().position(|c| c.id == id) {
            self.connections.remove(pos);
            true
        } else {
            false
        }
    }

    /// Update the component based on an action
    pub fn update(&mut self, action: ServerListAction) {
        match action {
            ServerListAction::SelectServer(id) => {
                // Selection handled by parent - this is a read-only display component
                _ = id;
            }
            ServerListAction::NewConnection => {
                // NewConnection handled by parent
            }
            ServerListAction::RefreshList => {
                // Refresh handled by parent
            }
        }
    }
}

impl Default for ServerListComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ServerListComponent {
    fn id(&self) -> ComponentId {
        ComponentId::ServerList
    }

    fn view(&self, theme: ThemeColors) -> Element<ComponentAction> {
        // New connection button
        let new_conn_btn = button(text("+ New Connection").size(12))
            .padding([6, 10])
            .style(button_styles::primary(theme))
            .on_press(ComponentAction::ServerList(ServerListAction::NewConnection));

        let mut content_col = column![
            row![
                text(icons::server()).font(icons::font()).size(14).color(theme.accent),
                text("Servers").size(12).color(theme.text),
                horizontal_space(),
                new_conn_btn,
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            text("─────────").size(10).color(theme.border),
        ]
        .spacing(10);

        if self.connections.is_empty() {
            content_col = content_col.push(
                text("(No connections)")
                    .size(11)
                    .color(theme.text_secondary),
            );
        } else {
            for conn in &self.connections {
                let conn_id = conn.id.clone();
                content_col = content_col.push(
                    button(text(&conn.name).size(11).color(theme.text))
                        .padding([4, 8])
                        .style(move |_theme, status| button::Style {
                            background: Some(if matches!(status, button::Status::Hovered) {
                                theme.background_secondary.into()
                            } else {
                                theme.background.into()
                            }),
                            text_color: theme.text,
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(ComponentAction::ServerList(ServerListAction::SelectServer(
                            conn_id,
                        ))),
                );
            }
        }

        container(scrollable(content_col.padding(15)).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arni::DatabaseType;

    fn create_test_connection(id: &str, name: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: id.to_string(),
            name: name.to_string(),
            db_type: DatabaseType::Postgres,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: std::collections::HashMap::new(),
            pool_config: None,
        }
    }

    #[test]
    fn test_server_list_component_new() {
        let component = ServerListComponent::new();
        assert_eq!(component.id(), ComponentId::ServerList);
        assert_eq!(component.connections().len(), 0);
    }

    #[test]
    fn test_server_list_add_connection() {
        let mut component = ServerListComponent::new();
        let conn = create_test_connection("test1", "Test Connection 1");

        component.add_connection(conn);
        assert_eq!(component.connections().len(), 1);
        assert_eq!(component.connections()[0].name, "Test Connection 1");
    }

    #[test]
    fn test_server_list_set_connections() {
        let mut component = ServerListComponent::new();
        let connections = vec![
            create_test_connection("test1", "Test 1"),
            create_test_connection("test2", "Test 2"),
        ];

        component.set_connections(connections);
        assert_eq!(component.connections().len(), 2);
    }

    #[test]
    fn test_server_list_remove_connection() {
        let mut component = ServerListComponent::new();
        component.add_connection(create_test_connection("test1", "Test 1"));
        component.add_connection(create_test_connection("test2", "Test 2"));

        assert_eq!(component.connections().len(), 2);

        assert!(component.remove_connection("test1"));
        assert_eq!(component.connections().len(), 1);
        assert_eq!(component.connections()[0].id, "test2");

        assert!(!component.remove_connection("nonexistent"));
        assert_eq!(component.connections().len(), 1);
    }
}

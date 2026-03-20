//! Connection list component for displaying saved database connections
//!
//! Displays a scrollable list of saved connections with adapter type icons,
//! connection names, and status indicators (connected/disconnected).

use crate::theme::ThemeColors;
use iced::widget::{column, container, horizontal_space, row, scrollable, text};
use iced::{Border, Element, Fill};
use arni::{ConnectionConfig, DatabaseType};
use std::collections::HashMap;

/// Connection status for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl ConnectionStatus {
    /// Get the display indicator for this status
    pub fn indicator(&self) -> &'static str {
        match self {
            ConnectionStatus::Connected => "●",
            ConnectionStatus::Disconnected => "○",
        }
    }

    /// Get the color for this status indicator
    pub fn color(&self, theme: ThemeColors) -> iced::Color {
        match self {
            ConnectionStatus::Connected => theme.success,
            ConnectionStatus::Disconnected => theme.text_secondary,
        }
    }
}

/// Connection list component displaying saved connections
#[derive(Debug, Clone)]
pub struct ConnectionList {
    theme: ThemeColors,
}

impl ConnectionList {
    /// Create a new connection list with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Get the icon/emoji for a database type
    fn database_icon(db_type: &DatabaseType) -> &'static str {
        match db_type {
            DatabaseType::Postgres => "🐘",  // PostgreSQL elephant
            DatabaseType::MySQL => "🐬",     // MySQL dolphin
            DatabaseType::SQLite => "💾",    // SQLite file-based
            DatabaseType::MongoDB => "🍃",   // MongoDB leaf
            DatabaseType::SQLServer => "⚡", // SQL Server
            DatabaseType::Oracle => "🔷",    // Oracle
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
        }
    }

    /// Render the connection list component
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        connections: &'a [ConnectionConfig],
        connection_statuses: &'a HashMap<String, ConnectionStatus>,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Header with title
        let header = row![
            text("Connections").size(14).color(theme.text),
            horizontal_space(),
        ]
        .spacing(5);

        let mut content_col =
            column![header, text("─────────").size(10).color(theme.border)].spacing(10);

        if connections.is_empty() {
            // Empty state
            content_col = content_col.push(
                text("No saved connections")
                    .size(12)
                    .color(theme.text_secondary),
            );
        } else {
            // Display each connection
            for conn in connections {
                let status = connection_statuses
                    .get(&conn.id)
                    .copied()
                    .unwrap_or(ConnectionStatus::Disconnected);

                let connection_row = row![
                    // Status indicator
                    text(status.indicator()).size(14).color(status.color(theme)),
                    // Database type icon
                    text(Self::database_icon(&conn.db_type)).size(14),
                    // Connection name
                    text(&conn.name).size(12).color(theme.text),
                    horizontal_space(),
                ]
                .spacing(8)
                .padding([4, 8]);

                content_col =
                    content_col.push(container(connection_row).width(Fill).style(move |_theme| {
                        container::Style {
                            background: Some(theme.background_secondary.into()),
                            border: Border {
                                color: theme.border,
                                width: 1.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }));
            }
        }

        // Wrap in scrollable container
        container(scrollable(content_col.padding(15)).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
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
    use std::collections::HashMap;

    fn create_test_connection(id: &str, name: &str, db_type: DatabaseType) -> ConnectionConfig {
        ConnectionConfig {
            id: id.to_string(),
            name: name.to_string(),
            db_type,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: "testdb".to_string(),
            username: Some("user".to_string()),
            use_ssl: false,
            parameters: HashMap::new(),
            pool_config: None,
        }
    }

    #[test]
    fn test_connection_list_creation() {
        let theme = ThemeColors::dark();
        let list = ConnectionList::new(theme);

        assert_eq!(list.theme, theme);
    }

    #[test]
    fn test_connection_status_indicator() {
        assert_eq!(ConnectionStatus::Connected.indicator(), "●");
        assert_eq!(ConnectionStatus::Disconnected.indicator(), "○");
    }

    #[test]
    fn test_connection_status_color() {
        let theme = ThemeColors::dark();

        let connected_color = ConnectionStatus::Connected.color(theme);
        let disconnected_color = ConnectionStatus::Disconnected.color(theme);

        assert_eq!(connected_color, theme.success);
        assert_eq!(disconnected_color, theme.text_secondary);
    }

    #[test]
    fn test_database_icon_mapping() {
        assert_eq!(ConnectionList::database_icon(&DatabaseType::Postgres), "🐘");
        assert_eq!(ConnectionList::database_icon(&DatabaseType::MySQL), "🐬");
        assert_eq!(ConnectionList::database_icon(&DatabaseType::SQLite), "💾");
        assert_eq!(ConnectionList::database_icon(&DatabaseType::MongoDB), "🍃");
        assert_eq!(
            ConnectionList::database_icon(&DatabaseType::SQLServer),
            "⚡"
        );
        assert_eq!(ConnectionList::database_icon(&DatabaseType::Oracle), "🔷");
    }

    #[test]
    fn test_connection_list_with_empty_connections() {
        let theme = ThemeColors::dark();
        let list = ConnectionList::new(theme);
        let connections: Vec<ConnectionConfig> = vec![];
        let statuses = HashMap::new();

        // Should not panic with empty connections
        let _view = list.view::<()>(&connections, &statuses);
    }

    #[test]
    fn test_connection_list_with_connections() {
        let theme = ThemeColors::dark();
        let list = ConnectionList::new(theme);

        let connections = vec![
            create_test_connection("conn1", "PostgreSQL Local", DatabaseType::Postgres),
            create_test_connection("conn2", "MySQL Dev", DatabaseType::MySQL),
            create_test_connection("conn3", "MongoDB Atlas", DatabaseType::MongoDB),
        ];

        let mut statuses = HashMap::new();
        statuses.insert("conn1".to_string(), ConnectionStatus::Connected);
        statuses.insert("conn2".to_string(), ConnectionStatus::Disconnected);
        // conn3 not in map - should default to Disconnected

        // Should not panic with multiple connections
        let _view = list.view::<()>(&connections, &statuses);
    }

    #[test]
    fn test_connection_status_equality() {
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_eq!(
            ConnectionStatus::Disconnected,
            ConnectionStatus::Disconnected
        );
        assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_connection_status_debug() {
        let status = ConnectionStatus::Connected;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Connected"));
    }

    #[test]
    fn test_connection_list_cloneable() {
        let theme = ThemeColors::dark();
        let list = ConnectionList::new(theme);
        let cloned = list.clone();

        assert_eq!(list.theme, cloned.theme);
    }

    #[test]
    fn test_connection_list_debug() {
        let theme = ThemeColors::dark();
        let list = ConnectionList::new(theme);

        let debug_str = format!("{:?}", list);
        assert!(debug_str.contains("ConnectionList"));
    }
}

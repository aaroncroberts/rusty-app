//! Table list component for displaying database tables

use crate::components::{Component, ComponentAction, ComponentId, TableListAction};
use crate::icons;
use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Element, Fill};

/// Metadata for a database table
#[derive(Debug, Clone, PartialEq)]
pub struct TableMetadata {
    pub schema: String,
    pub name: String,
    pub row_count: Option<u64>,
}

/// Table list component with internal table list
#[derive(Debug, Clone)]
pub struct TableListComponent {
    tables: Vec<TableMetadata>,
    selected_server: Option<String>,
}

impl TableListComponent {
    /// Create a new table list component
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            selected_server: None,
        }
    }

    /// Get the current tables
    pub fn tables(&self) -> &[TableMetadata] {
        &self.tables
    }

    /// Set the tables list
    pub fn set_tables(&mut self, tables: Vec<TableMetadata>) {
        self.tables = tables;
    }

    /// Set the selected server
    pub fn set_selected_server(&mut self, server: Option<String>) {
        let is_none = server.is_none();
        self.selected_server = server;
        // Clear tables when server changes
        if is_none {
            self.tables.clear();
        }
    }

    /// Get the selected server
    pub fn selected_server(&self) -> Option<&str> {
        self.selected_server.as_deref()
    }

    /// Clear all tables
    pub fn clear(&mut self) {
        self.tables.clear();
    }

    /// Update the component based on an action
    pub fn update(&mut self, action: TableListAction) {
        match action {
            TableListAction::SelectTable(_table_name) => {
                // Selection handled by parent
            }
            TableListAction::RefreshTables => {
                // Refresh handled by parent
            }
        }
    }
}

impl Default for TableListComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TableListComponent {
    fn id(&self) -> ComponentId {
        ComponentId::TableList
    }

    fn view(&self, theme: ThemeColors) -> Element<'_, ComponentAction> {
        let mut content_col = column![
            row![
                text(icons::table())
                    .font(icons::font())
                    .size(14)
                    .color(theme.accent),
                text("Tables").size(12).color(theme.text),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text("─────────").size(10).color(theme.border),
        ]
        .spacing(10);

        if self.selected_server.is_none() {
            content_col = content_col.push(
                text("(Select a server)")
                    .size(11)
                    .color(theme.text_secondary),
            );
        } else if self.tables.is_empty() {
            content_col =
                content_col.push(text("(No tables)").size(11).color(theme.text_secondary));
        } else {
            for table in &self.tables {
                let table_name = format!("{}.{}", table.schema, table.name);
                let display_text = if let Some(count) = table.row_count {
                    format!("{} ({})", table_name, count)
                } else {
                    table_name.clone()
                };

                content_col = content_col.push(
                    button(text(display_text).size(11).color(theme.text))
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
                        .on_press(ComponentAction::TableList(TableListAction::SelectTable(
                            table_name,
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

    fn create_test_table(schema: &str, name: &str, row_count: Option<u64>) -> TableMetadata {
        TableMetadata {
            schema: schema.to_string(),
            name: name.to_string(),
            row_count,
        }
    }

    #[test]
    fn test_table_list_component_new() {
        let component = TableListComponent::new();
        assert_eq!(component.id(), ComponentId::TableList);
        assert_eq!(component.tables().len(), 0);
        assert_eq!(component.selected_server(), None);
    }

    #[test]
    fn test_table_list_set_tables() {
        let mut component = TableListComponent::new();
        let tables = vec![
            create_test_table("public", "users", Some(100)),
            create_test_table("public", "orders", Some(250)),
        ];

        component.set_tables(tables);
        assert_eq!(component.tables().len(), 2);
        assert_eq!(component.tables()[0].name, "users");
    }

    #[test]
    fn test_table_list_set_selected_server() {
        let mut component = TableListComponent::new();

        component.set_selected_server(Some("server1".to_string()));
        assert_eq!(component.selected_server(), Some("server1"));

        component.set_selected_server(None);
        assert_eq!(component.selected_server(), None);
    }

    #[test]
    fn test_table_list_clear_on_server_change() {
        let mut component = TableListComponent::new();
        component.set_selected_server(Some("server1".to_string()));
        component.set_tables(vec![create_test_table("public", "users", None)]);

        assert_eq!(component.tables().len(), 1);

        component.set_selected_server(None);
        assert_eq!(component.tables().len(), 0);
    }

    #[test]
    fn test_table_list_clear() {
        let mut component = TableListComponent::new();
        component.set_tables(vec![
            create_test_table("public", "users", None),
            create_test_table("public", "orders", None),
        ]);

        assert_eq!(component.tables().len(), 2);

        component.clear();
        assert_eq!(component.tables().len(), 0);
    }
}

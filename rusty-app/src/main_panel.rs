//! Main panel component for query editor and data viewer
//!
//! Provides a tabbed interface for managing multiple query sessions,
//! displaying results, and interacting with the database.

use crate::query_editor::QueryEditor;
use crate::result_grid::ResultGrid;
use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, text};
use iced::{Border, Element, Fill};
use rusty_data::adapter::QueryResult;
use std::collections::HashMap;

/// Identifier for a tab
pub type TabId = usize;

/// Represents a tab in the main panel
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
}

impl Tab {
    /// Create a new tab with the given ID and title
    pub fn new(id: TabId, title: String) -> Self {
        Self { id, title }
    }
}

/// Main panel component that displays query editor and results
pub struct MainPanel {
    theme: ThemeColors,
    tabs: Vec<Tab>,
    active_tab_id: Option<TabId>,
    next_tab_id: TabId,
    query_editors: HashMap<TabId, QueryEditor>,
    result_grid: ResultGrid,
}

impl MainPanel {
    /// Create a new main panel with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self {
            theme,
            tabs: Vec::new(),
            active_tab_id: None,
            next_tab_id: 0,
            query_editors: HashMap::new(),
            result_grid: ResultGrid::new(theme),
        }
    }

    /// Add a new tab with the given title
    pub fn add_tab(&mut self, title: String) -> TabId {
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = Tab::new(id, title);
        self.tabs.push(tab);
        self.active_tab_id = Some(id);

        // Create a query editor for this tab
        let editor = QueryEditor::new(self.theme);
        self.query_editors.insert(id, editor);

        id
    }

    /// Set the active tab by ID
    pub fn set_active_tab(&mut self, id: TabId) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_tab_id = Some(id);
        }
    }

    /// Close a tab by ID
    pub fn close_tab(&mut self, id: TabId) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);

            // Remove the query editor for this tab
            self.query_editors.remove(&id);

            // If we closed the active tab, activate another one
            if self.active_tab_id == Some(id) {
                self.active_tab_id = if !self.tabs.is_empty() {
                    // Activate the tab at the same position, or the last one if we removed the last tab
                    let new_pos = pos.min(self.tabs.len().saturating_sub(1));
                    self.tabs.get(new_pos).map(|t| t.id)
                } else {
                    None
                };
            }
        }
    }

    /// Get the currently active tab
    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab_id
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
    }

    /// Get all tabs
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get the query text for a specific tab
    pub fn get_query_text(&self, tab_id: TabId) -> Option<String> {
        self.query_editors.get(&tab_id).map(|editor| editor.text())
    }

    /// Render the main panel component
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        on_new_tab: Message,
        on_tab_click: impl Fn(TabId) -> Message + 'a,
        on_tab_close: impl Fn(TabId) -> Message + 'a,
        on_query_editor: impl Fn(TabId, crate::query_editor::QueryEditorMessage) -> Message + 'a + Copy,
        query_results: &'a HashMap<TabId, Option<QueryResult>>,
        query_errors: &'a HashMap<TabId, Option<String>>,
    ) -> Element<'a, Message> {
        let theme = self.theme;
        let on_new_tab_clone = on_new_tab.clone();

        // Map query editor messages to the parent message type
        let tab_id = self.active_tab_id;
        let mapped_content = if let Some(id) = tab_id {
            self.tab_content(id, query_results, query_errors).map(move |msg| on_query_editor(id, msg))
        } else {
            // No active tab - show empty state
            container(
                column![
                    text("No tabs open")
                        .size(16)
                        .color(theme.text_secondary),
                    text("Click + to create a new tab")
                        .size(12)
                        .color(theme.text_secondary),
                ]
                .spacing(10)
            )
            .width(Fill)
            .height(Fill)
            .padding(20)
            .style(move |_theme| container::Style {
                background: Some(theme.background.into()),
                ..Default::default()
            })
            .into()
        };

        let content = column![
            self.tab_bar(on_new_tab_clone, on_tab_click, on_tab_close),
            mapped_content,
        ]
        .spacing(0);

        container(content)
            .width(Fill)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(theme.background.into()),
                ..Default::default()
            })
            .into()
    }

    /// Render the tab bar
    fn tab_bar<'a, Message: 'a + Clone>(
        &'a self,
        on_new_tab: Message,
        on_tab_click: impl Fn(TabId) -> Message + 'a,
        on_tab_close: impl Fn(TabId) -> Message + 'a,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        let mut tabs_row = row![].spacing(2);

        // Render each tab
        for tab in &self.tabs {
            let is_active = self.active_tab_id == Some(tab.id);
            let tab_id = tab.id;

            let bg_color = if is_active {
                theme.background
            } else {
                theme.background_secondary
            };

            let text_color = if is_active {
                theme.text
            } else {
                theme.text_secondary
            };

            // Wrap both buttons in a container with border for consistent height
            let tab_content = container(
                row![
                    button(text(&tab.title).size(12).color(text_color))
                        .on_press(on_tab_click(tab_id))
                        .padding([6, 12])
                        .style(move |_theme, _status| button::Style {
                            background: Some(bg_color.into()),
                            text_color,
                            border: Border::default(),
                            ..Default::default()
                        }),
                    button(text("×").size(12).color(text_color))
                        .on_press(on_tab_close(tab_id))
                        .padding([6, 8])
                        .style(move |_theme, status| button::Style {
                            background: Some(bg_color.into()),
                            text_color: if matches!(status, button::Status::Hovered) {
                                theme.accent
                            } else {
                                text_color
                            },
                            border: Border::default(),
                            ..Default::default()
                        }),
                ]
                .spacing(0)
            )
            .style(move |_theme| container::Style {
                background: None,
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });

            tabs_row = tabs_row.push(tab_content);
        }

        // New tab button (perfectly aligned height with tabs)
        let new_tab_btn = button(text("+").size(12))
            .on_press(on_new_tab)
            .padding([6, 12])
            .style(move |_theme, status| button::Style {
                background: Some(theme.background_secondary.into()),
                text_color: if matches!(status, button::Status::Hovered) {
                    theme.accent
                } else {
                    theme.text_secondary
                },
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });

        tabs_row = tabs_row.push(new_tab_btn);

        container(tabs_row)
            .width(Fill)
            .padding([8, 12])
            .style(move |_theme| container::Style {
                background: Some(theme.background_secondary.into()),
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    /// Render the content area for the active tab
    fn tab_content<'a>(
        &'a self,
        tab_id: TabId,
        query_results: &'a HashMap<TabId, Option<QueryResult>>,
        query_errors: &'a HashMap<TabId, Option<String>>,
    ) -> Element<'a, crate::query_editor::QueryEditorMessage> {
        let theme = self.theme;

        // Display the query editor for this tab
        if let Some(editor) = self.query_editors.get(&tab_id) {
            // Get result and error for this tab - clone to owned values
            let result_opt = query_results.get(&tab_id).cloned().flatten();
            let error_opt = query_errors.get(&tab_id).cloned().flatten();

            // Create a column with editor on top, result grid on bottom
            let editor_view = editor.view();
            let result_view: Element<'a, crate::query_editor::QueryEditorMessage> =
                self.result_grid.view(result_opt, error_opt);

            container(
                column![editor_view, result_view]
                    .spacing(0)
                    .height(Fill)
            )
            .width(Fill)
            .height(Fill)
            .into()
        } else {
            // Editor not found (shouldn't happen)
            container(
                text("Error: Query editor not found")
                    .size(16)
                    .color(theme.text_secondary)
            )
            .width(Fill)
            .height(Fill)
            .padding(20)
            .style(move |_theme| container::Style {
                background: Some(theme.background.into()),
                ..Default::default()
            })
            .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_panel_creation() {
        let theme = ThemeColors::dark();
        let panel = MainPanel::new(theme);

        assert_eq!(panel.theme, theme);
        assert_eq!(panel.tabs.len(), 0);
        assert_eq!(panel.active_tab_id, None);
        assert_eq!(panel.next_tab_id, 0);
    }

    // Note: MainPanel doesn't implement Clone or Debug because QueryEditor
    // contains text_editor::Content which doesn't implement Clone

    #[test]
    fn test_add_tab() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        let id = panel.add_tab("Query 1".to_string());
        assert_eq!(id, 0);
        assert_eq!(panel.tabs.len(), 1);
        assert_eq!(panel.active_tab_id, Some(0));
        assert_eq!(panel.tabs[0].title, "Query 1");
    }

    #[test]
    fn test_add_multiple_tabs() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        let id1 = panel.add_tab("Query 1".to_string());
        let id2 = panel.add_tab("Query 2".to_string());
        let id3 = panel.add_tab("Query 3".to_string());

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
        assert_eq!(panel.tabs.len(), 3);
        assert_eq!(panel.active_tab_id, Some(2)); // Last added tab is active
    }

    #[test]
    fn test_set_active_tab() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        panel.add_tab("Query 1".to_string());
        panel.add_tab("Query 2".to_string());
        panel.set_active_tab(0);

        assert_eq!(panel.active_tab_id, Some(0));
    }

    #[test]
    fn test_set_active_tab_invalid_id() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        panel.add_tab("Query 1".to_string());
        panel.set_active_tab(99); // Invalid ID

        assert_eq!(panel.active_tab_id, Some(0)); // Should not change
    }

    #[test]
    fn test_close_tab() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        let id1 = panel.add_tab("Query 1".to_string());
        let id2 = panel.add_tab("Query 2".to_string());

        panel.close_tab(id1);

        assert_eq!(panel.tabs.len(), 1);
        assert_eq!(panel.tabs[0].id, id2);
        assert_eq!(panel.active_tab_id, Some(id2)); // Active tab should shift
    }

    #[test]
    fn test_close_active_tab_activates_next() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        let _id1 = panel.add_tab("Query 1".to_string());
        let id2 = panel.add_tab("Query 2".to_string());
        let _id3 = panel.add_tab("Query 3".to_string());

        panel.set_active_tab(id2);
        panel.close_tab(id2);

        // Should activate the tab at the same position (id3)
        assert_eq!(panel.tabs.len(), 2);
        assert_eq!(panel.active_tab_id, Some(panel.tabs[1].id));
    }

    #[test]
    fn test_close_last_tab() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        let id = panel.add_tab("Query 1".to_string());
        panel.close_tab(id);

        assert_eq!(panel.tabs.len(), 0);
        assert_eq!(panel.active_tab_id, None);
    }

    #[test]
    fn test_active_tab() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        assert_eq!(panel.active_tab(), None);

        panel.add_tab("Query 1".to_string());
        let active = panel.active_tab();
        assert!(active.is_some());
        assert_eq!(active.unwrap().title, "Query 1");
    }

    #[test]
    fn test_tabs() {
        let theme = ThemeColors::dark();
        let mut panel = MainPanel::new(theme);

        panel.add_tab("Query 1".to_string());
        panel.add_tab("Query 2".to_string());

        let tabs = panel.tabs();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].title, "Query 1");
        assert_eq!(tabs[1].title, "Query 2");
    }

    #[test]
    fn test_tab_creation() {
        let tab = Tab::new(0, "Test".to_string());
        assert_eq!(tab.id, 0);
        assert_eq!(tab.title, "Test");
    }

    #[test]
    fn test_tab_cloneable() {
        let tab = Tab::new(0, "Test".to_string());
        let cloned = tab.clone();
        assert_eq!(tab, cloned);
    }

    #[test]
    fn test_tab_debug() {
        let tab = Tab::new(0, "Test".to_string());
        let debug_str = format!("{:?}", tab);
        assert!(debug_str.contains("Tab"));
        assert!(debug_str.contains("Test"));
    }
}

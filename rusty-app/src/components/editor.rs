//! Editor component for query editing with tab management

use crate::components::{Component, ComponentAction, ComponentId, EditorAction};
use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Border, Element, Fill};

/// A query tab in the editor
#[derive(Debug, Clone, PartialEq)]
pub struct QueryTab {
    pub id: usize,
    pub title: String,
    pub query: String,
}

impl QueryTab {
    /// Create a new query tab
    pub fn new(id: usize, title: String) -> Self {
        Self {
            id,
            title,
            query: String::new(),
        }
    }
}

/// Editor component with tab management
#[derive(Debug, Clone)]
pub struct EditorComponent {
    tabs: Vec<QueryTab>,
    active_tab: Option<usize>,
    next_tab_id: usize,
}

impl EditorComponent {
    /// Create a new editor component
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: None,
            next_tab_id: 0,
        }
    }

    /// Get all tabs
    pub fn tabs(&self) -> &[QueryTab] {
        &self.tabs
    }

    /// Get the active tab ID
    pub fn active_tab(&self) -> Option<usize> {
        self.active_tab
    }

    /// Get the active tab
    pub fn active_tab_mut(&mut self) -> Option<&mut QueryTab> {
        self.active_tab
            .and_then(|id| self.tabs.iter_mut().find(|t| t.id == id))
    }

    /// Add a new tab
    pub fn add_tab(&mut self, title: String) -> usize {
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        self.tabs.push(QueryTab::new(id, title));
        self.active_tab = Some(id);
        id
    }

    /// Close a tab by ID
    pub fn close_tab(&mut self, tab_id: usize) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(pos);

            // Update active tab if the closed tab was active
            if self.active_tab == Some(tab_id) {
                self.active_tab = if !self.tabs.is_empty() {
                    Some(self.tabs[pos.min(self.tabs.len() - 1)].id)
                } else {
                    None
                };
            }
            true
        } else {
            false
        }
    }

    /// Select a tab by ID
    pub fn select_tab(&mut self, tab_id: usize) -> bool {
        if self.tabs.iter().any(|t| t.id == tab_id) {
            self.active_tab = Some(tab_id);
            true
        } else {
            false
        }
    }

    /// Update the query content of the active tab
    pub fn set_query(&mut self, query: String) {
        if let Some(tab) = self.active_tab_mut() {
            tab.query = query;
        }
    }

    /// Get the query from the active tab
    pub fn query(&self) -> Option<&str> {
        self.active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
            .map(|t| t.query.as_str())
    }

    /// Update the component based on an action
    pub fn update(&mut self, action: EditorAction) {
        match action {
            EditorAction::NewTab => {
                let count = self.tabs.len() + 1;
                self.add_tab(format!("Query {}", count));
            }
            EditorAction::CloseTab(tab_id) => {
                self.close_tab(tab_id);
            }
            EditorAction::SelectTab(tab_id) => {
                self.select_tab(tab_id);
            }
            EditorAction::QueryChanged(query) => {
                self.set_query(query);
            }
            EditorAction::ExecuteQuery => {
                // Execution handled by parent
            }
        }
    }
}

impl Default for EditorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for EditorComponent {
    fn id(&self) -> ComponentId {
        ComponentId::Editor
    }

    fn view(&self, theme: ThemeColors) -> Element<'_, ComponentAction> {
        // Tab bar
        let mut tab_row = row![].spacing(2);

        for tab in &self.tabs {
            let is_active = Some(tab.id) == self.active_tab;
            let tab_id = tab.id;

            let tab_button = button(
                row![
                    text(&tab.title).size(12),
                    button(text("×").size(14))
                        .padding([0, 4])
                        .style(move |_theme, status| button::Style {
                            background: None,
                            text_color: if matches!(status, button::Status::Hovered) {
                                theme.accent
                            } else {
                                theme.text_secondary
                            },
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(ComponentAction::Editor(EditorAction::CloseTab(tab_id))),
                ]
                .spacing(8),
            )
            .padding([6, 12])
            .style(move |_theme, status| button::Style {
                background: Some(if is_active {
                    theme.background.into()
                } else if matches!(status, button::Status::Hovered) {
                    theme.background_secondary.into()
                } else {
                    theme.background.into()
                }),
                text_color: theme.text,
                border: Border {
                    color: if is_active {
                        theme.accent
                    } else {
                        theme.border
                    },
                    width: if is_active { 2.0 } else { 1.0 },
                    ..Default::default()
                },
                ..Default::default()
            })
            .on_press(ComponentAction::Editor(EditorAction::SelectTab(tab_id)));

            tab_row = tab_row.push(tab_button);
        }

        // New tab button
        let new_tab_btn = button(text("+ New Query").size(12))
            .padding([6, 12])
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
            })
            .on_press(ComponentAction::Editor(EditorAction::NewTab));

        tab_row = tab_row.push(new_tab_btn);

        // Query editor area - simplified for component pattern
        let editor_text = if let Some(tab) = self
            .active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
        {
            if tab.query.is_empty() {
                format!("{}: (Enter SQL query...)", tab.title)
            } else {
                format!("{}: {}", tab.title, tab.query)
            }
        } else {
            "(No tabs open)".to_string()
        };

        let editor_content =
            container(text(editor_text).size(12).color(theme.text_secondary)).padding(15);

        let content = column![
            container(scrollable(tab_row))
                .padding(5)
                .style(move |_theme| container::Style {
                    background: Some(theme.background_secondary.into()),
                    border: Border {
                        color: theme.border,
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            container(editor_content)
                .width(Fill)
                .height(Fill)
                .padding(10),
        ]
        .spacing(0);

        container(content).width(Fill).height(Fill).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_component_new() {
        let component = EditorComponent::new();
        assert_eq!(component.id(), ComponentId::Editor);
        assert_eq!(component.tabs().len(), 0);
        assert_eq!(component.active_tab(), None);
    }

    #[test]
    fn test_editor_add_tab() {
        let mut component = EditorComponent::new();
        let tab_id = component.add_tab("Query 1".to_string());

        assert_eq!(component.tabs().len(), 1);
        assert_eq!(component.active_tab(), Some(tab_id));
        assert_eq!(component.tabs()[0].title, "Query 1");
    }

    #[test]
    fn test_editor_close_tab() {
        let mut component = EditorComponent::new();
        let tab1 = component.add_tab("Query 1".to_string());
        let tab2 = component.add_tab("Query 2".to_string());

        assert_eq!(component.tabs().len(), 2);
        assert_eq!(component.active_tab(), Some(tab2));

        assert!(component.close_tab(tab1));
        assert_eq!(component.tabs().len(), 1);
        assert_eq!(component.active_tab(), Some(tab2));
    }

    #[test]
    fn test_editor_select_tab() {
        let mut component = EditorComponent::new();
        let tab1 = component.add_tab("Query 1".to_string());
        let tab2 = component.add_tab("Query 2".to_string());

        assert_eq!(component.active_tab(), Some(tab2));

        assert!(component.select_tab(tab1));
        assert_eq!(component.active_tab(), Some(tab1));
    }

    #[test]
    fn test_editor_set_query() {
        let mut component = EditorComponent::new();
        component.add_tab("Query 1".to_string());

        component.set_query("SELECT * FROM users".to_string());
        assert_eq!(component.query(), Some("SELECT * FROM users"));
    }

    #[test]
    fn test_editor_close_active_tab_selects_next() {
        let mut component = EditorComponent::new();
        let tab1 = component.add_tab("Query 1".to_string());
        let tab2 = component.add_tab("Query 2".to_string());

        assert_eq!(component.active_tab(), Some(tab2));

        component.close_tab(tab2);
        assert_eq!(component.active_tab(), Some(tab1));
    }
}

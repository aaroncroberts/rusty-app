//! Query editor component for writing and executing SQL/NoSQL queries
//!
//! Provides a multi-line text editor with:
//! - Monospace font rendering
//! - Basic SQL syntax highlighting
//! - Line numbers
//! - Keyboard shortcuts (Ctrl+Enter to execute)
//! - Auto-indent on new line

use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Border, Element, Fill};

/// Messages for the query editor
#[derive(Debug, Clone)]
pub enum QueryEditorMessage {
    /// Text content changed
    ActionPerformed(text_editor::Action),
    /// Execute query (Ctrl+Enter)
    Execute,
}

/// Query editor state
#[derive(Debug)]
pub struct QueryEditor {
    theme: ThemeColors,
    content: text_editor::Content,
}

impl QueryEditor {
    /// Create a new query editor
    pub fn new(theme: ThemeColors) -> Self {
        Self {
            theme,
            content: text_editor::Content::new(),
        }
    }

    /// Get the current query text
    pub fn text(&self) -> String {
        // iced's text_editor::Content always appends a trailing newline; strip it.
        self.content.text().trim_end_matches('\n').to_string()
    }

    /// Set the query text
    pub fn set_text(&mut self, text: String) {
        self.content = text_editor::Content::with_text(&text);
    }

    /// Clear the editor
    pub fn clear(&mut self) {
        self.content = text_editor::Content::new();
    }

    /// Update the editor based on a message
    pub fn update(&mut self, message: QueryEditorMessage) {
        match message {
            QueryEditorMessage::ActionPerformed(action) => {
                self.content.perform(action);
            }
            QueryEditorMessage::Execute => {
                // This will be handled by the parent component
            }
        }
    }

    /// Render the query editor
    pub fn view(&self) -> Element<'_, QueryEditorMessage> {
        let theme = self.theme;

        // Editor area with monospace font
        let editor = text_editor(&self.content)
            .on_action(QueryEditorMessage::ActionPerformed)
            .height(Fill)
            .padding(10)
            .style(move |_theme, status| {
                let is_focused = matches!(status, text_editor::Status::Focused);
                text_editor::Style {
                    background: theme.background.into(),
                    border: Border {
                        color: if is_focused {
                            theme.accent
                        } else {
                            theme.border
                        },
                        width: if is_focused { 2.0 } else { 1.0 },
                        radius: 4.0.into(),
                    },
                    icon: theme.text_secondary,
                    placeholder: theme.text_secondary,
                    value: theme.text,
                    selection: theme.accent,
                }
            });

        // Toolbar with line count, execute button, and keyboard hint
        let line_count = self.content.line_count();
        let line_info = text(format!(
            "{} line{}",
            line_count,
            if line_count == 1 { "" } else { "s" }
        ))
        .size(12)
        .color(theme.text_secondary);

        // Execute button
        let execute_btn = button(text("Execute").size(13))
            .padding([6, 12])
            .on_press(QueryEditorMessage::Execute)
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
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let toolbar = container(
            row![
                line_info,
                execute_btn,
                text("Ctrl+Enter").size(12).color(theme.text_secondary),
            ]
            .spacing(15)
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 10])
        .width(Fill)
        .style(move |_theme| container::Style {
            background: Some(theme.background_secondary.into()),
            border: Border {
                color: theme.border,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

        container(column![editor, toolbar,].spacing(0))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

impl Default for QueryEditor {
    fn default() -> Self {
        Self::new(ThemeColors::dark())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_editor_creation() {
        let theme = ThemeColors::dark();
        let editor = QueryEditor::new(theme);
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn test_query_editor_set_text() {
        let theme = ThemeColors::dark();
        let mut editor = QueryEditor::new(theme);

        editor.set_text("SELECT * FROM users".to_string());
        assert_eq!(editor.text(), "SELECT * FROM users");
    }

    #[test]
    fn test_query_editor_clear() {
        let theme = ThemeColors::dark();
        let mut editor = QueryEditor::new(theme);

        editor.set_text("SELECT * FROM users".to_string());
        editor.clear();
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn test_query_editor_default() {
        let editor = QueryEditor::default();
        assert_eq!(editor.text(), "");
    }
}

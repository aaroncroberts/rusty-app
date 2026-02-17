//! Status bar component for the Database IDE
//!
//! Provides a horizontal status bar that spans the full width of the window
//! at the bottom, displaying application state and contextual information.

use crate::theme::ThemeColors;
use iced::widget::{container, row, text};
use iced::{Border, Element, Fill};

/// Height of the status bar in pixels
pub const STATUS_BAR_HEIGHT: f32 = 30.0;

/// Status bar component that displays at the bottom of the window
#[derive(Debug, Clone)]
pub struct StatusBar {
    theme: ThemeColors,
}

impl StatusBar {
    /// Create a new status bar with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the status bar component
    pub fn view<'a, Message: 'a + Clone>(&'a self) -> Element<'a, Message> {
        let theme = self.theme;

        // Status bar content (placeholder)
        let content = row![
            text("Ready")
                .size(12)
                .color(theme.text_secondary),
        ]
        .padding([6, 15])
        .spacing(20);

        container(content)
            .width(Fill)
            .height(STATUS_BAR_HEIGHT)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_creation() {
        let theme = ThemeColors::dark();
        let status_bar = StatusBar::new(theme);

        assert_eq!(status_bar.theme, theme);
    }

    #[test]
    fn test_status_bar_cloneable() {
        let theme = ThemeColors::dark();
        let status_bar = StatusBar::new(theme);
        let cloned = status_bar.clone();

        assert_eq!(status_bar.theme, cloned.theme);
    }

    #[test]
    fn test_status_bar_debug() {
        let theme = ThemeColors::dark();
        let status_bar = StatusBar::new(theme);

        let debug_str = format!("{:?}", status_bar);
        assert!(debug_str.contains("StatusBar"));
    }

    #[test]
    fn test_status_bar_height_constant() {
        assert_eq!(STATUS_BAR_HEIGHT, 30.0);
    }

    #[test]
    fn test_status_bar_has_correct_theme() {
        let theme = ThemeColors::dark();
        let status_bar = StatusBar::new(theme);

        assert_eq!(
            status_bar.theme.background_secondary,
            theme.background_secondary
        );
    }
}

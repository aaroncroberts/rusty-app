//! Left panel component for database object navigation
//!
//! Provides a resizable panel (120-400px) for viewing database objects
//! like servers, tables, and properties.

use crate::theme::ThemeColors;
use iced::widget::{column, container, horizontal_space, row, text};
use iced::{Border, Element, Fill, Length};

/// Minimum width for the left panel in pixels
pub const MIN_WIDTH: f32 = 120.0;

/// Maximum width for the left panel in pixels
pub const MAX_WIDTH: f32 = 400.0;

/// Default width for the left panel in pixels
pub const DEFAULT_WIDTH: f32 = 200.0;

/// Width of the resize handle in pixels
const RESIZE_HANDLE_WIDTH: f32 = 4.0;

/// Left panel component with resizable width
#[derive(Debug, Clone)]
pub struct LeftPanel {
    theme: ThemeColors,
}

impl LeftPanel {
    /// Create a new left panel with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the left panel with current width
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        width: f32,
        _on_resize_start: Message,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Panel content
        let content = column![
            text("Servers").size(12).color(theme.text),
            text("─────────").size(10).color(theme.border),
            text("(No connections)")
                .size(11)
                .color(theme.text_secondary),
        ]
        .spacing(10)
        .padding(15);

        // Resize handle (vertical bar on right edge)
        let resize_handle = container(horizontal_space())
            .width(RESIZE_HANDLE_WIDTH)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(theme.border.into()),
                ..Default::default()
            });

        // Combine content and resize handle
        let panel_with_handle = row![
            container(content)
                .width(Length::Fixed(width - RESIZE_HANDLE_WIDTH))
                .height(Fill)
                .style(move |_theme| container::Style {
                    background: Some(theme.background.into()),
                    border: Border {
                        color: theme.border,
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            resize_handle,
        ]
        .spacing(0);

        container(panel_with_handle)
            .width(Length::Fixed(width))
            .height(Fill)
            .into()
    }
}

/// Constrain width to valid range (MIN_WIDTH to MAX_WIDTH)
pub fn constrain_width(width: f32) -> f32 {
    width.clamp(MIN_WIDTH, MAX_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_panel_creation() {
        let theme = ThemeColors::dark();
        let panel = LeftPanel::new(theme);

        assert_eq!(panel.theme, theme);
    }

    #[test]
    fn test_left_panel_cloneable() {
        let theme = ThemeColors::dark();
        let panel = LeftPanel::new(theme);
        let cloned = panel.clone();

        assert_eq!(panel.theme, cloned.theme);
    }

    #[test]
    fn test_left_panel_debug() {
        let theme = ThemeColors::dark();
        let panel = LeftPanel::new(theme);

        let debug_str = format!("{:?}", panel);
        assert!(debug_str.contains("LeftPanel"));
    }

    #[test]
    fn test_constrain_width_min_boundary() {
        assert_eq!(constrain_width(50.0), MIN_WIDTH);
        assert_eq!(constrain_width(MIN_WIDTH - 10.0), MIN_WIDTH);
        assert_eq!(constrain_width(MIN_WIDTH), MIN_WIDTH);
    }

    #[test]
    fn test_constrain_width_max_boundary() {
        assert_eq!(constrain_width(500.0), MAX_WIDTH);
        assert_eq!(constrain_width(MAX_WIDTH + 10.0), MAX_WIDTH);
        assert_eq!(constrain_width(MAX_WIDTH), MAX_WIDTH);
    }

    #[test]
    fn test_constrain_width_within_range() {
        assert_eq!(constrain_width(150.0), 150.0);
        assert_eq!(constrain_width(200.0), 200.0);
        assert_eq!(constrain_width(300.0), 300.0);
    }

    #[test]
    fn test_default_width_is_valid() {
        assert!(DEFAULT_WIDTH >= MIN_WIDTH);
        assert!(DEFAULT_WIDTH <= MAX_WIDTH);
        assert_eq!(constrain_width(DEFAULT_WIDTH), DEFAULT_WIDTH);
    }

    #[test]
    fn test_width_constants() {
        assert_eq!(MIN_WIDTH, 120.0);
        assert_eq!(MAX_WIDTH, 400.0);
        assert_eq!(DEFAULT_WIDTH, 200.0);
    }
}

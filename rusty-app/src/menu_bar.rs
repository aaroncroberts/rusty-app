//! Menu bar component for the Database IDE
//!
//! Provides a horizontal menu bar that spans the full width of the window
//! with a dark theme background and clickable menu items.

use crate::theme::ThemeColors;
use iced::widget::{button, container, row, text};
use iced::{Border, Element, Fill};

/// Available menu items in the menu bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    File,
    Edit,
    View,
    Tools,
    NewConnection,
}

impl MenuItem {
    /// Get the display name for this menu item
    pub fn name(&self) -> &'static str {
        match self {
            MenuItem::File => "File",
            MenuItem::Edit => "Edit",
            MenuItem::View => "View",
            MenuItem::Tools => "Tools",
            MenuItem::NewConnection => "File > New > Connection",
        }
    }

    /// Get all top-level menu items in order
    pub fn all() -> &'static [MenuItem] {
        &[MenuItem::File, MenuItem::Edit, MenuItem::View, MenuItem::Tools]
    }
}

/// Menu bar component that renders at the top of the window
#[derive(Debug, Clone)]
pub struct MenuBar {
    theme: ThemeColors,
}

impl MenuBar {
    /// Create a new menu bar with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the menu bar component with menu items
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        on_click: impl Fn(MenuItem) -> Message + 'a,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Create menu item buttons
        let menu_items = MenuItem::all()
            .iter()
            .fold(row![].spacing(0), |row, item| {
                let item_button = button(
                    text(item.name())
                        .size(14)
                        .color(theme.text)
                )
                .padding([6, 12])
                .style(move |_theme, status| {
                    let background = match status {
                        button::Status::Hovered => Some(theme.accent.into()),
                        _ => None,
                    };

                    button::Style {
                        background,
                        text_color: theme.text,
                        border: Border::default(),
                        ..Default::default()
                    }
                })
                .on_press(on_click(*item));

                row.push(item_button)
            });

        container(menu_items)
            .width(Fill)
            .height(40.0)
            .padding([0, 10])
            .center_y(40.0)
            .style(move |_| container::Style {
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
    fn test_menu_bar_creation() {
        let theme = ThemeColors::dark();
        let menu_bar = MenuBar::new(theme);

        assert_eq!(menu_bar.theme, theme);
    }

    #[test]
    fn test_menu_bar_has_correct_theme() {
        let theme = ThemeColors::dark();
        let menu_bar = MenuBar::new(theme);

        assert_eq!(
            menu_bar.theme.background_secondary,
            theme.background_secondary
        );
    }

    #[test]
    fn test_menu_bar_cloneable() {
        let theme = ThemeColors::dark();
        let menu_bar = MenuBar::new(theme);
        let cloned = menu_bar.clone();

        assert_eq!(menu_bar.theme, cloned.theme);
    }

    #[test]
    fn test_menu_bar_debug() {
        let theme = ThemeColors::dark();
        let menu_bar = MenuBar::new(theme);

        let debug_str = format!("{:?}", menu_bar);
        assert!(debug_str.contains("MenuBar"));
    }

    #[test]
    fn test_menu_item_names() {
        assert_eq!(MenuItem::File.name(), "File");
        assert_eq!(MenuItem::Edit.name(), "Edit");
        assert_eq!(MenuItem::View.name(), "View");
        assert_eq!(MenuItem::Tools.name(), "Tools");
        assert_eq!(MenuItem::NewConnection.name(), "File > New > Connection");
    }

    #[test]
    fn test_menu_item_all() {
        let all = MenuItem::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], MenuItem::File);
        assert_eq!(all[1], MenuItem::Edit);
        assert_eq!(all[2], MenuItem::View);
        assert_eq!(all[3], MenuItem::Tools);
    }

    #[test]
    fn test_menu_item_equality() {
        assert_eq!(MenuItem::File, MenuItem::File);
        assert_ne!(MenuItem::File, MenuItem::Edit);
    }

    #[test]
    fn test_menu_item_debug() {
        let item = MenuItem::File;
        let debug_str = format!("{:?}", item);
        assert!(debug_str.contains("File"));
    }
}

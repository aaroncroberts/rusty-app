//! Menu bar component for the Database IDE
//!
//! Provides a horizontal menu bar that spans the full width of the window
//! with a dark theme background and clickable menu items with dropdown submenus.

use crate::theme::ThemeColors;
use iced::widget::{button, column, container, row, text};
use iced::{Border, Element, Fill, Length};

/// Top-level menu items in the menu bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    File,
    Edit,
    View,
    Tools,
}

impl MenuItem {
    /// Get the display name for this menu item
    pub fn name(&self) -> &'static str {
        match self {
            MenuItem::File => "File",
            MenuItem::Edit => "Edit",
            MenuItem::View => "View",
            MenuItem::Tools => "Tools",
        }
    }

    /// Get all top-level menu items in order
    pub fn all() -> &'static [MenuItem] {
        &[MenuItem::File, MenuItem::Edit, MenuItem::View, MenuItem::Tools]
    }
}

/// Submenu items for the File menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuItem {
    NewConnection,
    OpenConnection,
    SaveQuery,
    Exit,
}

impl FileMenuItem {
    pub fn name(&self) -> &'static str {
        match self {
            FileMenuItem::NewConnection => "New Connection",
            FileMenuItem::OpenConnection => "Open Connection...",
            FileMenuItem::SaveQuery => "Save Query",
            FileMenuItem::Exit => "Exit",
        }
    }

    pub fn all() -> &'static [FileMenuItem] {
        &[
            FileMenuItem::NewConnection,
            FileMenuItem::OpenConnection,
            FileMenuItem::SaveQuery,
            FileMenuItem::Exit,
        ]
    }
}

/// Submenu items for the Edit menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMenuItem {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
}

impl EditMenuItem {
    pub fn name(&self) -> &'static str {
        match self {
            EditMenuItem::Undo => "Undo",
            EditMenuItem::Redo => "Redo",
            EditMenuItem::Cut => "Cut",
            EditMenuItem::Copy => "Copy",
            EditMenuItem::Paste => "Paste",
        }
    }

    pub fn all() -> &'static [EditMenuItem] {
        &[
            EditMenuItem::Undo,
            EditMenuItem::Redo,
            EditMenuItem::Cut,
            EditMenuItem::Copy,
            EditMenuItem::Paste,
        ]
    }
}

/// Submenu items for the View menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMenuItem {
    ToggleLeftPanel,
    Servers,
    Tables,
    Properties,
    QueryEditor,
    ConnectionManager,
    Settings,
}

impl ViewMenuItem {
    pub fn name(&self) -> &'static str {
        match self {
            ViewMenuItem::ToggleLeftPanel => "Toggle Left Panel",
            ViewMenuItem::Servers => "Servers",
            ViewMenuItem::Tables => "Tables",
            ViewMenuItem::Properties => "Properties",
            ViewMenuItem::QueryEditor => "Query Editor",
            ViewMenuItem::ConnectionManager => "Connection Manager",
            ViewMenuItem::Settings => "Settings...",
        }
    }

    pub fn all() -> &'static [ViewMenuItem] {
        &[
            ViewMenuItem::ToggleLeftPanel,
            ViewMenuItem::Servers,
            ViewMenuItem::Tables,
            ViewMenuItem::Properties,
            ViewMenuItem::QueryEditor,
            ViewMenuItem::ConnectionManager,
            ViewMenuItem::Settings,
        ]
    }

    /// Map ViewMenuItem to ComponentId
    ///
    /// Returns None for ToggleLeftPanel and Settings (not components),
    /// returns Some(ComponentId) for all component views.
    pub fn as_component_id(&self) -> Option<crate::components::ComponentId> {
        use crate::components::ComponentId;
        match self {
            ViewMenuItem::ToggleLeftPanel => None,
            ViewMenuItem::Servers => Some(ComponentId::ServerList),
            ViewMenuItem::Tables => Some(ComponentId::TableList),
            ViewMenuItem::Properties => Some(ComponentId::Properties),
            ViewMenuItem::QueryEditor => Some(ComponentId::Editor),
            ViewMenuItem::ConnectionManager => Some(ComponentId::ConnectionForm),
            ViewMenuItem::Settings => None,
        }
    }
}

/// Submenu items for the Tools menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsMenuItem {
    Preferences,
    ExportData,
    ImportData,
}

impl ToolsMenuItem {
    pub fn name(&self) -> &'static str {
        match self {
            ToolsMenuItem::Preferences => "Preferences...",
            ToolsMenuItem::ExportData => "Export Data",
            ToolsMenuItem::ImportData => "Import Data",
        }
    }

    pub fn all() -> &'static [ToolsMenuItem] {
        &[
            ToolsMenuItem::Preferences,
            ToolsMenuItem::ExportData,
            ToolsMenuItem::ImportData,
        ]
    }
}

/// All possible menu actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    File(FileMenuItem),
    Edit(EditMenuItem),
    View(ViewMenuItem),
    Tools(ToolsMenuItem),
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

    /// Render the menu bar component with menu items and optional submenu
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        open_menu: Option<MenuItem>,
        view_registry: &'a crate::views::ViewRegistry,
        on_menu_toggle: impl Fn(MenuItem) -> Message + 'a + Copy,
        on_menu_action: impl Fn(MenuAction) -> Message + 'a + Copy,
        on_close_menu: Message,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Create menu item buttons
        let menu_items = MenuItem::all()
            .iter()
            .fold(row![].spacing(0), |row, item| {
                let is_open = open_menu == Some(*item);

                let item_button = button(
                    text(item.name())
                        .size(14)
                        .color(theme.text)
                )
                .padding([6, 12])
                .style(move |_theme, status| {
                    let background = if is_open {
                        Some(theme.accent.into())
                    } else {
                        match status {
                            button::Status::Hovered => Some(theme.background.into()),
                            _ => None,
                        }
                    };

                    button::Style {
                        background,
                        text_color: theme.text,
                        border: Border::default(),
                        ..Default::default()
                    }
                })
                .on_press(on_menu_toggle(*item));

                row.push(item_button)
            });

        let menu_bar = container(menu_items)
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
            });

        // If a menu is open, render the submenu
        if let Some(menu) = open_menu {
            let submenu = self.render_submenu(menu, view_registry, on_menu_action, on_close_menu);
            column![menu_bar, submenu].spacing(0).into()
        } else {
            menu_bar.into()
        }
    }

    /// Render a submenu for the given menu item
    fn render_submenu<'a, Message: 'a + Clone>(
        &'a self,
        menu: MenuItem,
        view_registry: &'a crate::views::ViewRegistry,
        on_action: impl Fn(MenuAction) -> Message + 'a + Copy,
        _on_close: Message,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        let submenu_items = match menu {
            MenuItem::File => {
                FileMenuItem::all()
                    .iter()
                    .fold(column![].spacing(0), |col, item| {
                        let item_button = button(
                            text(item.name())
                                .size(13)
                                .color(theme.text)
                        )
                        .width(Length::Fixed(200.0))
                        .padding([8, 16])
                        .style(move |_theme, status| button::Style {
                            background: Some(if matches!(status, button::Status::Hovered) {
                                theme.accent.into()
                            } else {
                                theme.background_secondary.into()
                            }),
                            text_color: theme.text,
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(on_action(MenuAction::File(*item)));

                        col.push(item_button)
                    })
            }
            MenuItem::Edit => {
                EditMenuItem::all()
                    .iter()
                    .fold(column![].spacing(0), |col, item| {
                        let item_button = button(
                            text(item.name())
                                .size(13)
                                .color(theme.text)
                        )
                        .width(Length::Fixed(200.0))
                        .padding([8, 16])
                        .style(move |_theme, status| button::Style {
                            background: Some(if matches!(status, button::Status::Hovered) {
                                theme.accent.into()
                            } else {
                                theme.background_secondary.into()
                            }),
                            text_color: theme.text,
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(on_action(MenuAction::Edit(*item)));

                        col.push(item_button)
                    })
            }
            MenuItem::View => {
                ViewMenuItem::all()
                    .iter()
                    .fold(column![].spacing(0), |col, item| {
                        // Check if this view is enabled
                        let is_enabled = if let Some(component_id) = item.as_component_id() {
                            view_registry.is_enabled(component_id)
                        } else {
                            false // ToggleLeftPanel doesn't have a checkmark
                        };

                        // Build button text with checkmark if enabled
                        let button_text = if item.as_component_id().is_some() {
                            if is_enabled {
                                format!("✓ {}", item.name())
                            } else {
                                format!("  {}", item.name())
                            }
                        } else {
                            item.name().to_string()
                        };

                        let item_button = button(
                            text(button_text)
                                .size(13)
                                .color(theme.text)
                        )
                        .width(Length::Fixed(200.0))
                        .padding([8, 16])
                        .style(move |_theme, status| button::Style {
                            background: Some(if matches!(status, button::Status::Hovered) {
                                theme.accent.into()
                            } else {
                                theme.background_secondary.into()
                            }),
                            text_color: theme.text,
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(on_action(MenuAction::View(*item)));

                        col.push(item_button)
                    })
            }
            MenuItem::Tools => {
                ToolsMenuItem::all()
                    .iter()
                    .fold(column![].spacing(0), |col, item| {
                        let item_button = button(
                            text(item.name())
                                .size(13)
                                .color(theme.text)
                        )
                        .width(Length::Fixed(200.0))
                        .padding([8, 16])
                        .style(move |_theme, status| button::Style {
                            background: Some(if matches!(status, button::Status::Hovered) {
                                theme.accent.into()
                            } else {
                                theme.background_secondary.into()
                            }),
                            text_color: theme.text,
                            border: Border::default(),
                            ..Default::default()
                        })
                        .on_press(on_action(MenuAction::Tools(*item)));

                        col.push(item_button)
                    })
            }
        };

        // Render submenu as a dropdown
        container(submenu_items)
            .width(Length::Fixed(200.0))
            .padding(2)
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
    }

    #[test]
    fn test_file_menu_items() {
        assert_eq!(FileMenuItem::NewConnection.name(), "New Connection");
        assert_eq!(FileMenuItem::OpenConnection.name(), "Open Connection...");
        assert_eq!(FileMenuItem::SaveQuery.name(), "Save Query");
        assert_eq!(FileMenuItem::Exit.name(), "Exit");
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

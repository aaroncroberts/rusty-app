//! Database adapter selector component
//!
//! Provides a dropdown interface for selecting which database adapter to use
//! when creating a new connection.

use crate::button_styles;
use crate::theme::ThemeColors;
use arni::DatabaseType;
use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Border, Element, Fill};

/// Wrapper for DatabaseType to implement Display for the picker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayableDatabaseType(pub DatabaseType);

impl From<DatabaseType> for DisplayableDatabaseType {
    fn from(db_type: DatabaseType) -> Self {
        Self(db_type)
    }
}

impl From<DisplayableDatabaseType> for DatabaseType {
    fn from(dt: DisplayableDatabaseType) -> Self {
        dt.0
    }
}

impl std::fmt::Display for DisplayableDatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            DatabaseType::Postgres => write!(f, "PostgreSQL"),
            DatabaseType::MySQL => write!(f, "MySQL"),
            DatabaseType::SQLite => write!(f, "SQLite"),
            DatabaseType::MongoDB => write!(f, "MongoDB"),
            DatabaseType::SQLServer => write!(f, "SQL Server"),
            DatabaseType::Oracle => write!(f, "Oracle"),
            DatabaseType::DuckDB => unreachable!("DuckDB not used in rusty-app"),
        }
    }
}

/// Messages for the adapter selector
#[derive(Debug, Clone)]
pub enum AdapterSelectorMessage {
    AdapterSelected(DatabaseType),
    Continue,
    Cancel,
}

/// Adapter selector component
#[derive(Debug, Clone)]
pub struct AdapterSelector {
    theme: ThemeColors,
}

impl AdapterSelector {
    /// Create a new adapter selector with the given theme
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the adapter selector
    pub fn view<'a, Message: 'a + Clone>(
        &'a self,
        selected_adapter: &'a Option<DatabaseType>,
        on_message: impl Fn(AdapterSelectorMessage) -> Message + 'a + Copy,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        let title = text("Select Database Adapter").size(20).color(theme.text);

        let description = text("Choose the type of database you want to connect to:")
            .size(14)
            .color(theme.text_secondary);

        // Available adapters
        let available_adapters = vec![
            DisplayableDatabaseType(DatabaseType::Postgres),
            DisplayableDatabaseType(DatabaseType::MySQL),
            DisplayableDatabaseType(DatabaseType::SQLite),
            DisplayableDatabaseType(DatabaseType::MongoDB),
            DisplayableDatabaseType(DatabaseType::SQLServer),
            DisplayableDatabaseType(DatabaseType::Oracle),
        ];

        let adapter_picker = pick_list(
            available_adapters,
            selected_adapter.map(DisplayableDatabaseType::from),
            move |dt: DisplayableDatabaseType| {
                on_message(AdapterSelectorMessage::AdapterSelected(dt.0))
            },
        )
        .padding(8)
        .style(move |_theme, status| pick_list::Style {
            text_color: theme.text,
            placeholder_color: theme.text_secondary,
            handle_color: theme.text_secondary,
            background: theme.background_secondary.into(),
            border: Border {
                color: if matches!(
                    status,
                    pick_list::Status::Active | pick_list::Status::Hovered
                ) {
                    theme.accent
                } else {
                    theme.border
                },
                width: 1.0,
                ..Default::default()
            },
        });

        let picker_row = row![
            text("Database Type:").size(14).color(theme.text),
            adapter_picker,
        ]
        .spacing(15)
        .align_y(iced::Alignment::Center);

        // Buttons
        let can_continue = selected_adapter.is_some();
        let continue_button = if can_continue {
            button(text("Continue").size(14))
                .padding([8, 14])
                .on_press(on_message(AdapterSelectorMessage::Continue))
                .style(button_styles::primary(theme))
        } else {
            button(text("Continue").size(14))
                .padding([8, 14])
                .style(button_styles::disabled(theme))
        };

        let buttons = row![
            button(text("Cancel").size(14))
                .padding([8, 14])
                .on_press(on_message(AdapterSelectorMessage::Cancel))
                .style(button_styles::secondary(theme)),
            continue_button,
        ]
        .spacing(10);

        let content = column![title, description, picker_row, buttons]
            .spacing(20)
            .padding(20);

        container(content)
            .width(Fill)
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

    #[test]
    fn test_displayable_database_type_display() {
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::Postgres).to_string(),
            "PostgreSQL"
        );
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::MySQL).to_string(),
            "MySQL"
        );
        assert_eq!(
            DisplayableDatabaseType(DatabaseType::SQLite).to_string(),
            "SQLite"
        );
    }

    #[test]
    fn test_displayable_database_type_conversions() {
        let db_type = DatabaseType::Postgres;
        let displayable = DisplayableDatabaseType::from(db_type);
        assert_eq!(displayable.0, db_type);

        let converted_back = DatabaseType::from(displayable);
        assert_eq!(converted_back, db_type);
    }

    #[test]
    fn test_adapter_selector_creation() {
        let theme = ThemeColors::dark();
        let selector = AdapterSelector::new(theme);
        assert_eq!(selector.theme, theme);
    }
}

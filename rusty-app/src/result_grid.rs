//! Result grid component for displaying query results
//!
//! Provides a table view for query results with:
//! - Column headers
//! - Scrollable row data
//! - Row count and metadata display
//! - Error display

use crate::theme::ThemeColors;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Border, Element, Fill, Length};
use arni::QueryResult;

/// Result grid component for displaying query results
pub struct ResultGrid {
    theme: ThemeColors,
}

impl ResultGrid {
    /// Create a new result grid
    pub fn new(theme: ThemeColors) -> Self {
        Self { theme }
    }

    /// Render the result grid with query results
    pub fn view<'a, Message: 'a>(
        &self,
        result: Option<QueryResult>,
        error: Option<String>,
    ) -> Element<'a, Message> {
        let theme = self.theme;

        // Display error if present
        if let Some(err_msg) = error {
            return self.view_error(&err_msg);
        }

        // Display results if present
        if let Some(result) = result {
            return self.view_result(&result);
        }

        // Empty state - no results yet
        container(
            text("No results to display")
                .size(14)
                .color(theme.text_secondary),
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

    /// Render error message
    fn view_error<'a, Message: 'a>(&self, error_msg: &str) -> Element<'a, Message> {
        let theme = self.theme;

        container(
            column![
                text("Query Error")
                    .size(16)
                    .color(theme.accent)
                    .style(|_theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.9, 0.3, 0.3)),
                    }),
                text(error_msg.to_string()).size(13).color(theme.text),
            ]
            .spacing(10),
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

    /// Render query results in a table
    fn view_result<'a, Message: 'a>(&self, result: &QueryResult) -> Element<'a, Message> {
        let theme = self.theme;

        // Build header row
        let mut header_row = row![].spacing(1);
        for col in &result.columns {
            let header_cell = container(text(col.clone()).size(12).color(theme.text).style(
                |_theme| text::Style {
                    color: Some(iced::Color::from_rgb(1.0, 1.0, 1.0)),
                },
            ))
            .width(Length::Fixed(150.0))
            .padding([6, 10])
            .style(move |_theme| container::Style {
                background: Some(theme.accent.into()),
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });

            header_row = header_row.push(header_cell);
        }

        let header = container(header_row)
            .width(Fill)
            .style(move |_theme| container::Style {
                background: Some(theme.background_secondary.into()),
                ..Default::default()
            });

        // Build data rows
        let mut rows_column = column![].spacing(0);
        for row_data in &result.rows {
            let mut data_row = row![].spacing(1);
            for value in row_data {
                let cell_text = format!("{}", value);
                let data_cell = container(text(cell_text).size(12).color(theme.text))
                    .width(Length::Fixed(150.0))
                    .padding([6, 10])
                    .style(move |_theme| container::Style {
                        background: Some(theme.background.into()),
                        border: Border {
                            color: theme.border,
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    });

                data_row = data_row.push(data_cell);
            }
            rows_column = rows_column.push(data_row);
        }

        let data_scroll = scrollable(rows_column).height(Fill);

        // Footer with row count
        let row_count = result.rows.len();
        let footer_text = if let Some(affected) = result.rows_affected {
            format!(
                "{} row(s) returned, {} row(s) affected",
                row_count, affected
            )
        } else {
            format!("{} row(s) returned", row_count)
        };

        let footer = container(text(footer_text).size(12).color(theme.text_secondary))
            .width(Fill)
            .padding([6, 10])
            .style(move |_theme| container::Style {
                background: Some(theme.background_secondary.into()),
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });

        container(column![header, data_scroll, footer].spacing(0).height(Fill))
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
    use arni::QueryValue;

    #[test]
    fn test_result_grid_creation() {
        let theme = ThemeColors::dark();
        let grid = ResultGrid::new(theme);
        assert_eq!(grid.theme, theme);
    }

    #[test]
    fn test_query_value_display() {
        assert_eq!(format!("{}", QueryValue::Null), "NULL");
        assert_eq!(format!("{}", QueryValue::Bool(true)), "true");
        assert_eq!(format!("{}", QueryValue::Int(42)), "42");
        assert_eq!(format!("{}", QueryValue::Float(3.14)), "3.14");
        assert_eq!(
            format!("{}", QueryValue::Text("hello".to_string())),
            "hello"
        );
        assert_eq!(format!("{}", QueryValue::Bytes(vec![1, 2, 3])), "<3 bytes>");
    }
}

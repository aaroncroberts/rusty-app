//! Reusable button styling for consistent UI across the application
//!
//! Provides style functions for primary, secondary, and text button variants
//! with proper padding, rounded corners, and smooth hover transitions.

use crate::theme::ThemeColors;
use iced::widget::button;
use iced::{Border, Color};

/// Standard border radius for buttons (5px rounded corners)
const BUTTON_BORDER_RADIUS: f32 = 5.0;

/// Primary button style - accent background with prominent appearance
///
/// Use for main actions like "Continue", "Save", "Submit"
pub fn primary(theme: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let is_hovered = matches!(status, button::Status::Hovered);

        button::Style {
            background: Some(theme.accent.into()),
            text_color: theme.text,
            border: Border {
                color: if is_hovered { theme.text } else { theme.accent },
                width: 1.0,
                radius: BUTTON_BORDER_RADIUS.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Secondary button style - subtle background for less prominent actions
///
/// Use for "Cancel", "Back", or alternative actions
pub fn secondary(theme: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let is_hovered = matches!(status, button::Status::Hovered);

        button::Style {
            background: Some(
                if is_hovered {
                    theme.border.into()
                } else {
                    theme.background_secondary.into()
                }
            ),
            text_color: if is_hovered {
                theme.text
            } else {
                theme.text_secondary
            },
            border: Border {
                color: theme.border,
                width: 1.0,
                radius: BUTTON_BORDER_RADIUS.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Text button style - minimal styling for inline actions
///
/// Use for links, inline actions, or non-primary controls
pub fn text(theme: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let is_hovered = matches!(status, button::Status::Hovered);

        button::Style {
            background: if is_hovered {
                Some(Color::from_rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.1).into())
            } else {
                None
            },
            text_color: if is_hovered {
                theme.accent
            } else {
                theme.text_secondary
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: BUTTON_BORDER_RADIUS.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Danger button style - error color background for destructive actions
///
/// Use for "Delete", "Remove", or other destructive operations
pub fn danger(theme: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let is_hovered = matches!(status, button::Status::Hovered);

        button::Style {
            background: Some(theme.error.into()),
            text_color: theme.text,
            border: Border {
                color: if is_hovered { theme.text } else { theme.error },
                width: 1.0,
                radius: BUTTON_BORDER_RADIUS.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

/// Disabled button style - muted appearance for inactive buttons
///
/// Use when button action is not currently available
pub fn disabled(theme: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, _status| {
        button::Style {
            background: Some(theme.background_secondary.into()),
            text_color: theme.text_secondary,
            border: Border {
                color: theme.border,
                width: 1.0,
                radius: BUTTON_BORDER_RADIUS.into(),
            },
            shadow: iced::Shadow::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_border_radius() {
        let theme = ThemeColors::dark();

        let primary_style = primary(theme)(&iced::Theme::default(), button::Status::Active);
        assert_eq!(primary_style.border.radius, BUTTON_BORDER_RADIUS.into());

        let secondary_style = secondary(theme)(&iced::Theme::default(), button::Status::Active);
        assert_eq!(secondary_style.border.radius, BUTTON_BORDER_RADIUS.into());

        let text_style = text(theme)(&iced::Theme::default(), button::Status::Active);
        assert_eq!(text_style.border.radius, BUTTON_BORDER_RADIUS.into());
    }

    #[test]
    fn test_primary_button_uses_accent() {
        let theme = ThemeColors::dark();
        let style = primary(theme)(&iced::Theme::default(), button::Status::Active);

        assert_eq!(style.background, Some(theme.accent.into()));
        assert_eq!(style.text_color, theme.text);
    }

    #[test]
    fn test_primary_button_hover_state() {
        let theme = ThemeColors::dark();

        let normal = primary(theme)(&iced::Theme::default(), button::Status::Active);
        let hovered = primary(theme)(&iced::Theme::default(), button::Status::Hovered);

        // Border color should change on hover
        assert_eq!(normal.border.color, theme.accent);
        assert_eq!(hovered.border.color, theme.text);
    }

    #[test]
    fn test_secondary_button_hover_state() {
        let theme = ThemeColors::dark();

        let normal = secondary(theme)(&iced::Theme::default(), button::Status::Active);
        let hovered = secondary(theme)(&iced::Theme::default(), button::Status::Hovered);

        // Background should change on hover
        assert_eq!(normal.background, Some(theme.background_secondary.into()));
        assert_eq!(hovered.background, Some(theme.border.into()));

        // Text color should brighten on hover
        assert_eq!(normal.text_color, theme.text_secondary);
        assert_eq!(hovered.text_color, theme.text);
    }

    #[test]
    fn test_text_button_minimal_styling() {
        let theme = ThemeColors::dark();
        let style = text(theme)(&iced::Theme::default(), button::Status::Active);

        // Should have no background when not hovered
        assert_eq!(style.background, None);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.width, 0.0);
    }

    #[test]
    fn test_text_button_hover_subtle_background() {
        let theme = ThemeColors::dark();
        let hovered = text(theme)(&iced::Theme::default(), button::Status::Hovered);

        // Should show subtle accent background on hover
        assert!(hovered.background.is_some());

        // Text should change to accent color on hover
        assert_eq!(hovered.text_color, theme.accent);
    }

    #[test]
    fn test_danger_button_uses_error_color() {
        let theme = ThemeColors::dark();
        let style = danger(theme)(&iced::Theme::default(), button::Status::Active);

        assert_eq!(style.background, Some(theme.error.into()));
        assert_eq!(style.text_color, theme.text);
    }

    #[test]
    fn test_disabled_button_muted_appearance() {
        let theme = ThemeColors::dark();
        let style = disabled(theme)(&iced::Theme::default(), button::Status::Active);

        assert_eq!(style.background, Some(theme.background_secondary.into()));
        assert_eq!(style.text_color, theme.text_secondary);
        assert_eq!(style.border.color, theme.border);
    }

    #[test]
    fn test_all_buttons_have_consistent_border_radius() {
        let theme = ThemeColors::dark();

        let buttons = vec![
            primary(theme)(&iced::Theme::default(), button::Status::Active),
            secondary(theme)(&iced::Theme::default(), button::Status::Active),
            text(theme)(&iced::Theme::default(), button::Status::Active),
            danger(theme)(&iced::Theme::default(), button::Status::Active),
            disabled(theme)(&iced::Theme::default(), button::Status::Active),
        ];

        for button_style in buttons {
            assert_eq!(button_style.border.radius, BUTTON_BORDER_RADIUS.into());
        }
    }
}

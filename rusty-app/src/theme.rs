//! Dark theme color system for the Database IDE
//!
//! Defines a consistent color palette for the entire application following
//! a dark, sleek, modern aesthetic.

use iced::Color;

/// Main color palette for the dark theme
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    /// Main background color (#23272e)
    pub background: Color,
    /// Secondary background for panels (#2d323b)
    pub background_secondary: Color,
    /// Border color (#353b45)
    pub border: Color,
    /// Accent color for highlights and selections (#5fb3a6)
    pub accent: Color,
    /// Primary text color (#f5f6fa)
    pub text: Color,
    /// Secondary text color (dimmed) (#9ca0a8)
    pub text_secondary: Color,
    /// Success color (#50c878)
    pub success: Color,
    /// Warning color (#f39c12)
    pub warning: Color,
    /// Error color (#e74c3c)
    pub error: Color,
}

impl ThemeColors {
    /// Create the default dark theme palette
    pub const fn dark() -> Self {
        Self {
            background: Color::from_rgb(
                0x23 as f32 / 255.0,
                0x27 as f32 / 255.0,
                0x2e as f32 / 255.0,
            ),
            background_secondary: Color::from_rgb(
                0x2d as f32 / 255.0,
                0x32 as f32 / 255.0,
                0x3b as f32 / 255.0,
            ),
            border: Color::from_rgb(
                0x35 as f32 / 255.0,
                0x3b as f32 / 255.0,
                0x45 as f32 / 255.0,
            ),
            accent: Color::from_rgb(
                0x5f as f32 / 255.0,
                0xb3 as f32 / 255.0,
                0xa6 as f32 / 255.0,
            ),
            text: Color::from_rgb(
                0xf5 as f32 / 255.0,
                0xf6 as f32 / 255.0,
                0xfa as f32 / 255.0,
            ),
            text_secondary: Color::from_rgb(
                0x9c as f32 / 255.0,
                0xa0 as f32 / 255.0,
                0xa8 as f32 / 255.0,
            ),
            success: Color::from_rgb(
                0x50 as f32 / 255.0,
                0xc8 as f32 / 255.0,
                0x78 as f32 / 255.0,
            ),
            warning: Color::from_rgb(
                0xf3 as f32 / 255.0,
                0x9c as f32 / 255.0,
                0x12 as f32 / 255.0,
            ),
            error: Color::from_rgb(
                0xe7 as f32 / 255.0,
                0x4c as f32 / 255.0,
                0x3c as f32 / 255.0,
            ),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_colors_creation() {
        let theme = ThemeColors::dark();

        // Verify background color matches specification (#23272e)
        assert_eq!(
            (theme.background.r * 255.0) as u8,
            0x23,
            "Background red component should be 0x23"
        );
        assert_eq!(
            (theme.background.g * 255.0) as u8,
            0x27,
            "Background green component should be 0x27"
        );
        assert_eq!(
            (theme.background.b * 255.0) as u8,
            0x2e,
            "Background blue component should be 0x2e"
        );
    }

    #[test]
    fn test_border_color() {
        let theme = ThemeColors::dark();

        // Verify border color (#353b45)
        assert_eq!((theme.border.r * 255.0) as u8, 0x35);
        assert_eq!((theme.border.g * 255.0) as u8, 0x3b);
        assert_eq!((theme.border.b * 255.0) as u8, 0x45);
    }

    #[test]
    fn test_text_color() {
        let theme = ThemeColors::dark();

        // Verify text color (#f5f6fa)
        assert_eq!((theme.text.r * 255.0) as u8, 0xf5);
        assert_eq!((theme.text.g * 255.0) as u8, 0xf6);
        assert_eq!((theme.text.b * 255.0) as u8, 0xfa);
    }

    #[test]
    fn test_default_is_dark() {
        let default_theme = ThemeColors::default();
        let dark_theme = ThemeColors::dark();

        assert_eq!(
            default_theme, dark_theme,
            "Default theme should be dark theme"
        );
    }

    #[test]
    fn test_colors_are_valid_rgb() {
        let theme = ThemeColors::dark();

        // All color components should be between 0.0 and 1.0
        let colors = [
            theme.background,
            theme.background_secondary,
            theme.border,
            theme.accent,
            theme.text,
            theme.text_secondary,
            theme.success,
            theme.warning,
            theme.error,
        ];

        for color in colors.iter() {
            assert!(
                color.r >= 0.0 && color.r <= 1.0,
                "Red component out of range"
            );
            assert!(
                color.g >= 0.0 && color.g <= 1.0,
                "Green component out of range"
            );
            assert!(
                color.b >= 0.0 && color.b <= 1.0,
                "Blue component out of range"
            );
        }
    }
}

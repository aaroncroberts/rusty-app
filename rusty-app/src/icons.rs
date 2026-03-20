//! Icon helpers — Nerd Font (Codicons subset) for use with iced text widgets.
//!
//! Load the font at app startup by chaining `.font(icons::FONT_BYTES)` onto the
//! `iced::application()` builder (see `main.rs`).
//!
//! Usage in view code:
//! ```ignore
//! use crate::icons;
//! text(icons::database()).font(icons::font()).size(14).color(t.accent)
//! ```

use nerd_font::categories::Cod;

/// Raw font bytes — pass to `.font(icons::FONT_BYTES)` on the iced application builder.
pub const FONT_BYTES: &[u8] = nerd_font::NerdFont::FONT_BYTES;

/// The `iced::Font` definition for the Nerd Font family.
///
/// Built with the correct typographic family name so iced's fontdb resolves it.
pub fn font() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        weight: iced::font::Weight::Medium,
        ..Default::default()
    }
}

// ── Database / connection icons ────────────────────────────────────────────────

/// Database / server glyph — for the Servers panel header.
pub fn database() -> String {
    Cod::Database.to_string()
}

/// Server / host glyph.
pub fn server() -> String {
    Cod::Server.to_string()
}

/// Table / grid glyph — for the Tables panel header.
pub fn table() -> String {
    Cod::Table.to_string()
}

/// Symbol key/info glyph — for the Properties panel header.
pub fn symbol_key() -> String {
    Cod::SymbolKey.to_string()
}

// ── Navigation / panel layout icons ──────────────────────────────────────────

/// Chevron pointing right — collapse direction indicator.
pub fn chevron_right() -> String {
    Cod::ChevronRight.to_string()
}

/// Chevron pointing left — expand direction indicator.
pub fn chevron_left() -> String {
    Cod::ChevronLeft.to_string()
}

/// Chevron pointing down — expand/open tree node.
pub fn chevron_down() -> String {
    Cod::ChevronDown.to_string()
}

/// Expand pane — fill screen with this pane.
pub fn expand() -> String {
    Cod::ScreenFull.to_string()
}

/// Split/restore — restore two-panel split view.
pub fn split() -> String {
    Cod::SplitHorizontal.to_string()
}

// ── Action / toolbar icons ────────────────────────────────────────────────────

/// Add / plus glyph — for "New Connection" button.
pub fn add() -> String {
    Cod::Add.to_string()
}

/// Refresh / reload glyph.
pub fn refresh() -> String {
    Cod::Refresh.to_string()
}

/// Gear / settings glyph.
pub fn settings() -> String {
    Cod::SettingsGear.to_string()
}

/// Close / X glyph.
pub fn close() -> String {
    Cod::Close.to_string()
}

/// Trash / delete glyph.
pub fn trash() -> String {
    Cod::Trash.to_string()
}

/// Search / find glyph.
pub fn search() -> String {
    Cod::Search.to_string()
}

/// Play / run glyph — for Execute Query button.
pub fn play() -> String {
    Cod::Play.to_string()
}

// ── File / folder icons ───────────────────────────────────────────────────────

/// Closed folder glyph.
pub fn folder() -> String {
    Cod::Folder.to_string()
}

/// Open folder glyph.
pub fn folder_open() -> String {
    Cod::FolderOpened.to_string()
}

/// File glyph.
pub fn file() -> String {
    Cod::File.to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn font_bytes_non_empty() {
        assert!(!FONT_BYTES.is_empty());
    }

    #[test]
    fn all_icon_strings_non_empty() {
        assert!(!database().is_empty());
        assert!(!server().is_empty());
        assert!(!table().is_empty());
        assert!(!symbol_key().is_empty());
        assert!(!chevron_right().is_empty());
        assert!(!chevron_left().is_empty());
        assert!(!chevron_down().is_empty());
        assert!(!expand().is_empty());
        assert!(!split().is_empty());
        assert!(!add().is_empty());
        assert!(!refresh().is_empty());
        assert!(!settings().is_empty());
        assert!(!close().is_empty());
        assert!(!trash().is_empty());
        assert!(!search().is_empty());
        assert!(!play().is_empty());
        assert!(!folder().is_empty());
        assert!(!folder_open().is_empty());
        assert!(!file().is_empty());
    }
}

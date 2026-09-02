//! The TUI colour palette, selected from the user's configured theme (design GU-07 applied to the
//! terminal; handoff §2 `theme.rs`).
//!
//! Kept deliberately small this increment: a handful of named roles, a light and a dark variant, and
//! a monochrome fallback so nothing depends on colour alone (design NFR-A03/TU-10). Every status the
//! UI shows also carries text or shape, so a monochrome terminal loses no information.

use ratatui::style::Color;

use stikk_state::config::Theme as ConfigTheme;

/// The colour roles the TUI renders with. Chosen by role, not by literal, so the light/dark/mono
/// variants stay consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Primary foreground for body text.
    pub fg: Color,
    /// Dimmed text (labels, secondary detail).
    pub dim: Color,
    /// Accent for headings and the focused ref.
    pub accent: Color,
    /// A "good / ready" signal (a ready signing role).
    pub ok: Color,
    /// A "warning / attention" signal (a torn WAL tail, an unsupported prikk).
    pub warn: Color,
}

impl Palette {
    /// Build a palette from the configured theme. `System` uses the dark variant as a safe default for
    /// a terminal (most terminals are dark); an explicit light/dark choice wins.
    #[must_use]
    pub fn from_theme(theme: ConfigTheme) -> Self {
        match theme {
            ConfigTheme::Light => Self::light(),
            ConfigTheme::Dark | ConfigTheme::System => Self::dark(),
        }
    }

    /// The dark-terminal palette.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            fg: Color::Gray,
            dim: Color::DarkGray,
            accent: Color::Cyan,
            ok: Color::Green,
            warn: Color::Yellow,
        }
    }

    /// The light-terminal palette.
    #[must_use]
    pub fn light() -> Self {
        Self {
            fg: Color::Black,
            dim: Color::DarkGray,
            accent: Color::Blue,
            ok: Color::Green,
            warn: Color::Rgb(0xa0, 0x60, 0x00),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::dark()
    }
}

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

    /// The dark-terminal palette. `fg` and `dim` are fixed RGB rather than the named `Gray`/`DarkGray`
    /// so secondary text (labels) stays legible regardless of the terminal's own palette — named
    /// `DarkGray` renders near-invisible on many dark themes, which failed the contrast bar (NFR-A03).
    #[must_use]
    pub fn dark() -> Self {
        Self {
            fg: Color::Rgb(0xd8, 0xde, 0xe4),
            dim: Color::Rgb(0x9e, 0xa6, 0xb0),
            accent: Color::Cyan,
            ok: Color::Green,
            warn: Color::Yellow,
        }
    }

    /// The light-terminal palette. `fg`/`dim` are fixed RGB for the same contrast reason as [`dark`].
    #[must_use]
    pub fn light() -> Self {
        Self {
            fg: Color::Rgb(0x1b, 0x20, 0x24),
            dim: Color::Rgb(0x5a, 0x62, 0x6a),
            accent: Color::Blue,
            ok: Color::Rgb(0x18, 0x7a, 0x3c),
            warn: Color::Rgb(0xa0, 0x60, 0x00),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::dark()
    }
}

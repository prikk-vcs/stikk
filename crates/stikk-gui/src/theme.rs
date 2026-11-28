//! **Which preset draws, and who decided** (RFC 069 rule 3, RFC 070 decision 6).
//!
//! Three facts, in this order of authority:
//! 1. **what the user asked for in stikk's own configuration**, which is remembered because it is
//!    their file (`stikk_state::Config`'s `theme` key, the same key the terminal reads);
//! 2. **what the desktop says**, through `iced::system::theme()` at launch and
//!    `system::theme_changes()` while the app is open;
//! 3. **`light_aaa`**, when neither answers — *"for many older eyes"* (knotra).
//!
//! **A preset the user chose wins, and the desktop never overrides it** (rule 3). The desktop
//! decides **only what the configuration left open**, which is how all four presets are reachable
//! from a configuration file whose `theme` key has no high-contrast-dark value: `high-contrast`
//! chooses the contrast level, and the desktop's own light-or-dark setting chooses the mode within
//! it. A change of that setting is the user's own choice, made on their own desktop.
//!
//! **No new configuration key.** `theme = light | dark | high-contrast | system | plain` is the
//! key both front ends already read; adding a GUI-only value would be a change to `stikk-state`,
//! which this handoff does not sanction, and a key the terminal would then have to ignore.

use stikk_gui_design::Preset;
use stikk_state::config::Theme as Configured;

/// The preset choice, and where each half of it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The contrast level the configuration asked for: `Some(true)` is high contrast, `Some(false)`
    /// is the AAA pair, `None` is "the configuration did not say", which means the AAA pair.
    high_contrast: Option<bool>,
    /// The mode the configuration asked for, if it named one.
    configured_dark: Option<bool>,
    /// The mode the desktop last reported, if it has reported one.
    desktop_dark: Option<bool>,
}

impl Choice {
    /// **The choice as the user's configuration leaves it**, before the desktop has said anything.
    ///
    /// - `system` — nothing chosen; the desktop decides the mode.
    /// - `light` / `dark` — the mode is chosen, and wins.
    /// - `high-contrast` — the contrast level is chosen and wins; the desktop decides the mode
    ///   within it, because the key has no way to say which high-contrast preset.
    /// - `plain` — the terminal's *"no colour of stikk's own"* (RFC 059 decision 7), which has no
    ///   meaning in a window that must paint a background. Treated as `system`, so a user who set
    ///   it for the terminal gets the desktop's own setting here rather than a surprise.
    #[must_use]
    pub fn from_config(configured: Configured) -> Self {
        let (high_contrast, configured_dark) = match configured {
            Configured::System | Configured::Plain => (None, None),
            Configured::Light => (Some(false), Some(false)),
            Configured::Dark => (Some(false), Some(true)),
            Configured::HighContrast => (Some(true), None),
        };
        Self {
            high_contrast,
            configured_dark,
            desktop_dark: None,
        }
    }

    /// **The preset to draw with.**
    #[must_use]
    pub fn preset(self) -> Preset {
        let dark = self.configured_dark.or(self.desktop_dark).unwrap_or(false);
        Preset::of(self.high_contrast.unwrap_or(false), dark)
    }

    /// Whether the configuration chose the mode — the half the desktop must never override.
    #[must_use]
    pub fn mode_is_chosen(self) -> bool {
        self.configured_dark.is_some()
    }

    /// **What the desktop said**, at launch or while the app is open.
    ///
    /// `Mode::None` is *"no preference"*, which leaves the fallback in place rather than forcing
    /// light: a desktop that stops reporting has not asked for anything.
    ///
    /// Returns whether the preset changed, so the caller can record it.
    pub fn desktop_says(&mut self, mode: iced::theme::Mode) -> bool {
        let before = self.preset();
        let dark = match mode {
            iced::theme::Mode::Dark => Some(true),
            iced::theme::Mode::Light => Some(false),
            iced::theme::Mode::None => None,
        };
        // **The chosen mode wins** (rule 3). Recorded even so, because the desktop's answer belongs
        // in the record whether or not it is used — and because a version that let it through here
        // is this rule's failing run.
        self.desktop_dark = dark;
        before != self.preset()
    }

    /// What the desktop last reported, for the record and for the test that shows what a version
    /// without rule 3's precedence would have drawn.
    #[must_use]
    pub fn desktop_reported_dark(self) -> Option<bool> {
        self.desktop_dark
    }

    /// **Who decided the preset that is drawing**, for Orientation's own row and for the records.
    #[must_use]
    pub fn source(self) -> Source {
        match (self.high_contrast, self.configured_dark) {
            (None, _) => Source::Desktop,
            (Some(_), Some(_)) => Source::Configured,
            (Some(_), None) => Source::ConfiguredLevelOnly,
        }
    }
}

/// Who decided the preset that is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The configuration said nothing, so the desktop's own setting decides.
    Desktop,
    /// The configuration named a preset, and it wins whole.
    Configured,
    /// The configuration named the contrast level only; the desktop decides the mode within it.
    ConfiguredLevelOnly,
}

#[cfg(test)]
mod tests;

//! The four presets, and stikk's overrides over Snora's defaults (RFC 069 rules 3, 4 and 5).

use snora::design::Tokens;

/// **The four presets the desktop app draws in** (RFC 069 rule 3), as RFC 065 measured them.
///
/// There is no fifth, and no palette of stikk's own: every colour comes from one of these four, so
/// a contrast floor holds for the whole app by holding for four token bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Snora's `light_aaa`: light, every declared text pair at 7 : 1. **The default when the
    /// desktop reports no preference** (knotra: *"for many older eyes"*).
    LightAaa,
    /// Snora's `dark_aaa`: dark, every declared text pair at 7 : 1.
    DarkAaa,
    /// Snora's `high_contrast_light`.
    HcLight,
    /// Snora's `high_contrast_dark`.
    HcDark,
}

impl Preset {
    /// The preset's name, as the records and the contributing docs write it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::LightAaa => "light_aaa",
            Self::DarkAaa => "dark_aaa",
            Self::HcLight => "hc_light",
            Self::HcDark => "hc_dark",
        }
    }

    /// Every preset, in the order the test matrix runs them.
    #[must_use]
    pub fn every() -> [Self; 4] {
        [Self::LightAaa, Self::DarkAaa, Self::HcLight, Self::HcDark]
    }

    /// Whether this is one of the two high-contrast presets, whose floors are one step higher
    /// (rule 3's table: labels at 7 : 1 rather than 4.5 : 1, marks at 4.5 : 1 rather than 3 : 1).
    #[must_use]
    pub fn is_high_contrast(self) -> bool {
        matches!(self, Self::HcLight | Self::HcDark)
    }

    /// Whether this preset is a dark one — which is what a desktop's light-or-dark setting chooses
    /// between, within whichever pair the user's own configuration asked for.
    #[must_use]
    pub fn is_dark(self) -> bool {
        matches!(self, Self::DarkAaa | Self::HcDark)
    }

    /// The same contrast level, in the other mode. Used when the desktop's setting changes while
    /// the app is open and no preset was chosen (rule 3).
    #[must_use]
    pub fn with_dark(self, dark: bool) -> Self {
        Self::of(self.is_high_contrast(), dark)
    }

    /// The same mode, at the other contrast level.
    #[must_use]
    pub fn with_contrast(self, high_contrast: bool) -> Self {
        Self::of(high_contrast, self.is_dark())
    }

    /// The one preset at a contrast level and a mode. **The four are a two-by-two**, named once
    /// here so no caller writes the table again.
    #[must_use]
    pub fn of(high_contrast: bool, dark: bool) -> Self {
        match (high_contrast, dark) {
            (false, false) => Self::LightAaa,
            (false, true) => Self::DarkAaa,
            (true, false) => Self::HcLight,
            (true, true) => Self::HcDark,
        }
    }
}

/// **stikk's overrides over Snora's own preset values** (RFC 069 rules 3, 4 and 5).
///
/// Each field is a floor Snora's defaults do not meet, measured in RFC 065. They are switches
/// rather than constants so that **each one's own failing run is a test and not a code edit**: a
/// test builds the tree with one override off, and the guard that rule names must fail on it. A
/// shipped build uses [`Overrides::stikk`], which has every one on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overrides {
    /// Rule 5: `targets.min = 44`, where Snora's presets all say 24 and its buttons measure
    /// 27.5–28.3 px (RFC 065 M4).
    pub targets_44: bool,
    /// Rule 4: a scrollable reserves its bar's room, where iced's `Scrollbar::default()` has
    /// `spacing: None` and paints the bar over the content (RFC 065 M5).
    pub scrollbar_spacing: bool,
    /// Rule 3: an unavailable control's label meets 4.5 : 1 and its outline 3 : 1 (7 : 1 and
    /// 4.5 : 1 in the high-contrast presets), where Snora's disabled style multiplies the alpha by
    /// 0.45 and measures 1.48–2.90 : 1 (RFC 065 M4).
    pub disabled_contrast: bool,
}

impl Overrides {
    /// Every override on: what a shipped build draws with.
    #[must_use]
    pub fn stikk() -> Self {
        Self {
            targets_44: true,
            scrollbar_spacing: true,
            disabled_contrast: true,
        }
    }

    /// Rule 4's failing run: the same tree with the scrollbar's room not reserved.
    #[must_use]
    pub fn without_scrollbar_spacing(self) -> Self {
        Self {
            scrollbar_spacing: false,
            ..self
        }
    }

    /// Rule 3's failing run: the same tree with Snora's own disabled style.
    #[must_use]
    pub fn without_disabled_contrast(self) -> Self {
        Self {
            disabled_contrast: false,
            ..self
        }
    }

    /// Rule 5's failing run: the same tree with Snora's own 24 px target floor.
    #[must_use]
    pub fn without_targets_44(self) -> Self {
        Self {
            targets_44: false,
            ..self
        }
    }
}

impl Default for Overrides {
    fn default() -> Self {
        Self::stikk()
    }
}

/// **The tokens a preset draws with**, with `overrides` applied.
///
/// Snora's `Tokens` is `#[non_exhaustive]` and not `Copy`, so this returns an owned bundle; it is
/// cheap to clone and holds no allocation.
#[must_use]
pub fn tokens(preset: Preset, overrides: Overrides) -> Tokens {
    let mut tokens = match preset {
        Preset::LightAaa => Tokens::light_aaa(),
        Preset::DarkAaa => Tokens::dark_aaa(),
        Preset::HcLight => Tokens::high_contrast_light(),
        Preset::HcDark => Tokens::high_contrast_dark(),
    };
    if overrides.targets_44 {
        // Rule 5, set explicitly: the platform guidelines' figure, and the one knotra uses.
        tokens.targets.min = 44.0;
    }
    tokens
}

/// The `iced::Theme` a preset draws with, so stock iced widgets and the window's own background
/// follow the same palette as Snora's primitives (Snora's RFC-038).
#[must_use]
pub fn theme(preset: Preset, overrides: Overrides) -> iced::Theme {
    snora::design::theme(&tokens(preset, overrides))
}

#[cfg(test)]
mod tests;

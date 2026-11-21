//! The TUI colour palette (design GU-07, NFR-A03; **RFC 059**).
//!
//! **Every colour stikk chooses reaches 7 : 1** (WCAG AAA for body text) on every reference
//! background of its theme, in 24-bit colour and on a 256-colour terminal, and
//! `every_chosen_colour_reaches_seven_to_one_on_its_references` computes the ratios from the values
//! below rather than trusting a comment. Body text itself is **the terminal's own colour**
//! ([`Color::Reset`]): the terminal already chose a readable pair for its own background, and stikk
//! paints no background of its own in `dark`, `light` and `plain` (RFC 059 decision 2).
//!
//! **Three types, and the order they come in.** A [`Scheme`] is a theme as RFC 059 decision 3 writes
//! it: five roles, each carrying *both* depths' values at once, so a role cannot gain a 24-bit value
//! and silently keep an unchecked 256-colour one. [`Depth`] is which of the two a terminal gets
//! (decision 4). [`Scheme::for_depth`] resolves the first by the second into a [`Palette`] — one
//! `Color` per role — which is what every renderer draws with and all a renderer ever sees.
//!
//! **No named ANSI colour is used.** `Cyan` is whatever the terminal's scheme says it is, which is
//! exactly the contrast stikk cannot compute (RFC 059 F1). The 256-colour values are **chosen** from
//! the xterm cube, never rounded to it (F3): a value stikk picks has a known ratio, and a value the
//! terminal rounds does not.

use ratatui::style::Color;

use stikk_state::config::Theme as ConfigTheme;

/// The reference backgrounds the dark palette promises 7 : 1 on (RFC 059 decision 3).
///
/// Common terminal defaults, plus the one measured here: `#2e3440` is what Alacritty 0.17.0 answered
/// (`stikk-probe-059-alacritty-0.17.0-wayland.txt`), and it is the worst case of the six.
#[cfg(test)]
pub(crate) const DARK_REFERENCES: [(u8, u8, u8); 6] = [
    (0x00, 0x00, 0x00), // black
    (0x0c, 0x0c, 0x0c), // Windows Terminal Campbell
    (0x1e, 0x1e, 0x1e), // VS Code Dark+
    (0x28, 0x2c, 0x34), // One Dark
    (0x2e, 0x34, 0x40), // Alacritty, measured
    (0x00, 0x2b, 0x36), // Solarized Dark
];

/// The reference backgrounds the light palette promises 7 : 1 on (RFC 059 decision 3).
///
/// `#ffffff` is macOS Terminal's default, which is the background RFC 059 F1 found stikk unreadable
/// on.
#[cfg(test)]
pub(crate) const LIGHT_REFERENCES: [(u8, u8, u8); 3] = [
    (0xff, 0xff, 0xff), // white: macOS Terminal Basic, Tango Light
    (0xfa, 0xfa, 0xfa), // One Half Light
    (0xfd, 0xf6, 0xe3), // Solarized Light
];

/// The contrast every colour stikk chooses must reach: WCAG AAA for body text (NFR-A03, as RFC 059
/// decision 12 amends it). WCAG AA's 4.5 : 1 is the bar stikk 0.20.0 aimed at and missed in every
/// column of F1's table.
#[cfg(test)]
pub(crate) const MINIMUM_CONTRAST: f64 = 7.0;

/// Which colour depth stikk draws at (RFC 059 decision 4).
///
/// Not a terminal capability stikk discovers, but the one it is told about: `COLORTERM`, `WT_SESSION`
/// and the platform decide it, and a terminal that promises nothing gets the 256-colour values —
/// macOS Terminal is the common case (F3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// 24-bit colour: the role's `rgb` value, written as an RGB escape.
    TrueColour,
    /// 256 colours: the role's **chosen** palette index.
    ///
    /// Constructed by the resolver RFC 059 decision 4 asks for, which is this handoff's next commit;
    /// until then only the palette's own test draws at this depth.
    #[allow(
        dead_code,
        reason = "the resolver that picks a depth is the next commit"
    )]
    Ansi256,
}

/// Which palette a [`Palette`] came from. **There is no `System`**: the configured `system` resolves
/// to one of these, and what is drawn is always a decided palette.
///
/// Carried on the drawn palette so a test can ask a rendered screen which backgrounds it must hold up
/// against (`Self::references`). `high-contrast` and `plain` join it with their own commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeKind {
    /// The dark palette.
    Dark,
    /// The light palette.
    Light,
}

impl SchemeKind {
    /// The backgrounds this palette promises 7 : 1 on.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn references(self) -> &'static [(u8, u8, u8)] {
        match self {
            Self::Dark => &DARK_REFERENCES,
            Self::Light => &LIGHT_REFERENCES,
        }
    }

    /// This palette's name, as the Orientation row and a failing test print it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// One role's colour **at both depths at once** (RFC 059 decision 3).
///
/// The two travel together so they cannot drift: a role is chosen once, for a 24-bit terminal and for
/// a 256-colour one, and `Scheme::for_depth` is the only thing that picks between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleColour {
    /// The 24-bit value.
    pub rgb: Color,
    /// The **chosen** xterm-256 index — never a rounding of `rgb` (F3).
    pub indexed: Color,
}

impl RoleColour {
    /// A role stikk chooses: a 24-bit value and the 256-colour index picked for the same role.
    const fn chosen(r: u8, g: u8, b: u8, index: u8) -> Self {
        Self {
            rgb: Color::Rgb(r, g, b),
            indexed: Color::Indexed(index),
        }
    }

    /// The terminal's own colour, at either depth — stikk chooses nothing, so there is nothing to
    /// check (decision 2).
    const fn terminals_own() -> Self {
        Self {
            rgb: Color::Reset,
            indexed: Color::Reset,
        }
    }

    /// The colour to draw with at `depth`.
    #[must_use]
    pub const fn at(self, depth: Depth) -> Color {
        match depth {
            Depth::TrueColour => self.rgb,
            Depth::Ansi256 => self.indexed,
        }
    }
}

/// A theme's five roles, each at both depths, and the background the theme paints.
///
/// The values are RFC 059 decision 3's table, verified against every reference background by this
/// module's own test and cross-checked once against `palette059.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme {
    /// Which palette this is.
    pub kind: SchemeKind,
    /// Body text. [`Color::Reset`] — the terminal's own — in every theme but `high-contrast`.
    pub fg: RoleColour,
    /// The background stikk paints over the whole frame, or `None` where it paints none. Only
    /// `high-contrast` has one (decision 8).
    pub bg: Option<RoleColour>,
    /// Dimmed text (labels, secondary detail).
    pub dim: RoleColour,
    /// Accent for headings and the focused ref.
    pub accent: RoleColour,
    /// A "good / ready" signal (a ready signing role).
    pub ok: RoleColour,
    /// A "warning / attention" signal (a torn WAL tail, an unsupported prikk).
    pub warn: RoleColour,
}

impl Scheme {
    /// The dark palette (decision 3's `dark` column).
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            kind: SchemeKind::Dark,
            fg: RoleColour::terminals_own(),
            bg: None,
            dim: RoleColour::chosen(0xc1, 0xc5, 0xca, 251),
            accent: RoleColour::chosen(0x97, 0xcb, 0xf0, 117),
            ok: RoleColour::chosen(0x6d, 0xdb, 0x92, 114),
            warn: RoleColour::chosen(0xf7, 0xba, 0x64, 222),
        }
    }

    /// The light palette (decision 3's `light` column).
    #[must_use]
    pub const fn light() -> Self {
        Self {
            kind: SchemeKind::Light,
            fg: RoleColour::terminals_own(),
            bg: None,
            dim: RoleColour::chosen(0x4d, 0x53, 0x5a, 239),
            accent: RoleColour::chosen(0x13, 0x56, 0x86, 18),
            ok: RoleColour::chosen(0x18, 0x5e, 0x2f, 22),
            warn: RoleColour::chosen(0x76, 0x48, 0x06, 88),
        }
    }

    /// The colours to draw with at `depth`: `Color::Rgb` at 24-bit, `Color::Indexed` at 256.
    #[must_use]
    pub const fn for_depth(&self, depth: Depth) -> Palette {
        Palette {
            kind: self.kind,
            fg: self.fg.at(depth),
            bg: match self.bg {
                Some(bg) => Some(bg.at(depth)),
                None => None,
            },
            dim: self.dim.at(depth),
            accent: self.accent.at(depth),
            ok: self.ok.at(depth),
            warn: self.warn.at(depth),
        }
    }
}

/// The colour roles the TUI renders with, **at one depth** — one `Color` per role, which is all a
/// renderer ever sees. Produced by `Scheme::for_depth`.
///
/// Chosen by role, not by literal, so the palettes stay consistent: a renderer asks for `warn`, never
/// for a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Which palette this came from.
    pub kind: SchemeKind,
    /// Primary foreground for body text.
    pub fg: Color,
    /// The background painted over the whole frame, or `None` where stikk paints none.
    pub bg: Option<Color>,
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
    /// Every role that is **stikk's own choice**, with its name, in one list — what the contrast tests
    /// walk. `fg` is here too: it is `Color::Reset` in every theme but `high-contrast`, and the tests
    /// skip `Reset` because it is the terminal's own pair, not a colour stikk picked.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn roles(&self) -> [(&'static str, Color); 5] {
        [
            ("fg", self.fg),
            ("dim", self.dim),
            ("accent", self.accent),
            ("ok", self.ok),
            ("warn", self.warn),
        ]
    }

    /// Build a palette from the configured theme, at 24-bit colour.
    ///
    /// **Superseded within this RFC** by `theme::resolve` (RFC 059 decision 5), which is where the
    /// terminal's answer, `NO_COLOR` and the depth are weighed. It stays for this one commit so the
    /// palette's values can move before the resolver that chooses between them exists.
    #[must_use]
    pub fn from_theme(theme: ConfigTheme) -> Self {
        match theme {
            ConfigTheme::Light => Scheme::light().for_depth(Depth::TrueColour),
            ConfigTheme::Dark | ConfigTheme::System => Scheme::dark().for_depth(Depth::TrueColour),
        }
    }

    /// The light palette at 24-bit, for the render tests that capture a view in both themes.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn light() -> Self {
        Scheme::light().for_depth(Depth::TrueColour)
    }
}

impl Default for Palette {
    /// The dark palette at 24-bit, which is what the README visuals and the render tests draw with
    /// (decision 11) — never a resolution of the terminal's own answer.
    fn default() -> Self {
        Scheme::dark().for_depth(Depth::TrueColour)
    }
}

/// The relative luminance of an sRGB colour, as WCAG 2 defines it — the same formula, written once,
/// that `palette059.py` uses and that the headless matrix reuses on rendered cells.
#[cfg(test)]
#[must_use]
pub(crate) fn relative_luminance((r, g, b): (u8, u8, u8)) -> f64 {
    fn channel(value: u8) -> f64 {
        let v = f64::from(value) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// The WCAG contrast ratio between two sRGB colours: 1.0 for a colour against itself, 21.0 for black
/// against white.
#[cfg(test)]
#[must_use]
pub(crate) fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (x, y) = (relative_luminance(a), relative_luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The RGB an xterm-256 index stands for, or `None` for 0–15.
///
/// 16–231 are the 6 × 6 × 6 cube and 232–255 the grey ramp, both standard: **this is why a chosen
/// index has a known contrast** (F3). **0–15 have none** — they are the terminal's own sixteen, the
/// same unknown as a named colour — so they answer `None` and the contrast tests refuse them.
#[cfg(test)]
#[must_use]
pub(crate) fn xterm_rgb(index: u8) -> Option<(u8, u8, u8)> {
    /// The cube's six levels per channel.
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        0..=15 => None,
        16..=231 => {
            let n = usize::from(index - 16);
            let level = |i: usize| LEVELS.get(i).copied().unwrap_or(0);
            Some((level(n / 36), level((n % 36) / 6), level(n % 6)))
        }
        232..=255 => {
            let grey = 8 + 10 * (index - 232);
            Some((grey, grey, grey))
        }
    }
}

/// What a rendered `Color` is worth to a contrast check.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checkable {
    /// A colour stikk chose, with a known value.
    Known((u8, u8, u8)),
    /// The terminal's own pair — nothing of stikk's to check, and nothing stikk could check.
    TerminalsOwn,
    /// A colour whose value only the terminal knows: a named ANSI colour, or one of the low sixteen.
    /// **RFC 059 leaves none of these in the palette**, so a cell carrying one is a fault, not a pass.
    TerminalsGuess,
}

/// Classify a rendered colour for a contrast check.
#[cfg(test)]
#[must_use]
pub(crate) fn checkable(colour: Color) -> Checkable {
    match colour {
        Color::Reset => Checkable::TerminalsOwn,
        Color::Rgb(r, g, b) => Checkable::Known((r, g, b)),
        Color::Indexed(i) => match xterm_rgb(i) {
            Some(rgb) => Checkable::Known(rgb),
            None => Checkable::TerminalsGuess,
        },
        _ => Checkable::TerminalsGuess,
    }
}

#[cfg(test)]
mod tests;

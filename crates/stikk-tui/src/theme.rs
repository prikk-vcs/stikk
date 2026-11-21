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
    /// No colour of stikk's own (decision 7) — **and decision 5's fallback**, which is why it arrives
    /// with the resolver rather than with `high-contrast`.
    Plain,
}

impl SchemeKind {
    /// The backgrounds this palette promises 7 : 1 on.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn references(self) -> &'static [(u8, u8, u8)] {
        match self {
            Self::Dark => &DARK_REFERENCES,
            Self::Light => &LIGHT_REFERENCES,
            // Nothing to promise: `plain` chooses no colour at all.
            Self::Plain => &[],
        }
    }

    /// This palette's name, as the Orientation row and a failing test print it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Plain => "plain",
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

    /// **No colour of stikk's own** (decision 7): every role is the terminal's own text colour, and no
    /// background is painted.
    ///
    /// Safe on any background, which is why it is also where `system` lands when the terminal does not
    /// answer (decision 5). Nothing is lost: every status carries text or a shape (NFR-A03), and
    /// emphasis keeps bold and reverse video.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            kind: SchemeKind::Plain,
            fg: RoleColour::terminals_own(),
            bg: None,
            dim: RoleColour::terminals_own(),
            accent: RoleColour::terminals_own(),
            ok: RoleColour::terminals_own(),
            warn: RoleColour::terminals_own(),
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

/// What the terminal answered (RFC 059 decision 5).
///
/// Both halves are used: the mode decides which palette, and the background itself is what decision
/// 6's guard re-checks every chosen colour against — the palette's references cover the common
/// backgrounds, and this covers the rest, such as a mid-grey or a tinted scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    /// Whether the terminal's background is the light one, by the crate's own `theme_mode()` — which
    /// compares the terminal's foreground and background rather than guessing from the background
    /// alone.
    pub light: bool,
    /// The background itself, scaled to 8 bits per channel.
    pub background: (u8, u8, u8),
}

/// Everything [`resolve`] is told about the world. **It reads nothing itself** (Handoff A trap 2): the
/// reads happen in [`crate::run`] and arrive here as data, so every band below is a unit test with no
/// terminal and no environment variable.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// The configured theme.
    pub theme: ConfigTheme,
    /// `NO_COLOR`, as the environment holds it. **Present and non-empty** is what counts
    /// (no-color.org), so the empty string is not a request for anything.
    pub no_color: Option<String>,
    /// `COLORTERM`.
    pub colorterm: Option<String>,
    /// Whether `WT_SESSION` is set — Windows Terminal.
    pub wt_session: bool,
    /// `COLORFGBG`, whose last `;`-separated field some terminals use to say which way round they are.
    pub colorfgbg: Option<String>,
    /// Whether the host is Windows.
    pub windows: bool,
    /// What the terminal answered, or `None` — **which is both "no answer" and "not asked"**, and the
    /// two need no telling apart: a theme that sends no query does not look here.
    pub answer: Option<Answer>,
}

impl Environment {
    /// The configured theme and nothing else — no `NO_COLOR`, no `COLORTERM`, no answer. What a render
    /// test means when it names a theme.
    #[cfg(test)]
    #[must_use]
    pub fn configured(theme: ConfigTheme) -> Self {
        Self {
            theme,
            ..Self::default()
        }
    }

    /// Whether `NO_COLOR` applies: *"present and not an empty string"* (no-color.org).
    fn no_color(&self) -> bool {
        self.no_color
            .as_ref()
            .is_some_and(|value| !value.is_empty())
    }

    /// Whether the resolver would use an answer from the terminal — which is the only reason to send a
    /// query.
    ///
    /// **An explicit theme never queries** (decision 5), so a user can always choose one to avoid it,
    /// and `NO_COLOR` with `system` does not either: the answer could not change what it resolves to.
    fn wants_an_answer(&self) -> bool {
        match self.theme {
            ConfigTheme::System => !self.no_color(),
            ConfigTheme::Light | ConfigTheme::Dark => false,
        }
    }

    /// Which way round `COLORFGBG` says the terminal is, or `None` for no answer (decision 5).
    ///
    /// The last `;`-separated field is the background: `7` or `15` is light, `0`–`6` or `8` is dark.
    /// **Anything else counts as no answer**, including `default`, a colour index above 15 and a field
    /// that is not a number at all — a terminal that writes something else has not told stikk which
    /// way round it is.
    fn colorfgbg_is_light(&self) -> Option<bool> {
        let value = self.colorfgbg.as_deref()?;
        let last = value.rsplit(';').next()?.trim();
        match last.parse::<u8>() {
            Ok(7 | 15) => Some(true),
            Ok(0..=6 | 8) => Some(false),
            Ok(_) | Err(_) => None,
        }
    }

    /// Which depth to draw at (decision 4).
    ///
    /// 24-bit where a terminal promises it (`COLORTERM`), where Windows Terminal is running
    /// (`WT_SESSION`), or on Windows at all — Microsoft's console blog (2016-09-22) announced 24-bit
    /// colour in the Windows console from Windows 10 Insider build 14931. **256 colours otherwise**,
    /// which is macOS Terminal, the common case (F3).
    fn depth(&self) -> Depth {
        let promised = matches!(self.colorterm.as_deref(), Some("truecolor" | "24bit"));
        if promised || self.wt_session || self.windows {
            Depth::TrueColour
        } else {
            Depth::Ansi256
        }
    }
}

/// Why the palette in a [`Resolved`] is the one in use — the Orientation row's own words (decision
/// 10), and the support line when a user says the colours look wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The terminal answered, and this is which way round it is.
    Detected {
        /// Whether the answer was the light background.
        light: bool,
    },
    /// The terminal did not answer, but `COLORFGBG` said which way round it is.
    FromColorFgBg {
        /// Whether `COLORFGBG`'s background field was a light one.
        light: bool,
    },
    /// No answer, and no `COLORFGBG` to fall back on: `plain`, which is safe on any background.
    NoAnswer,
    /// `NO_COLOR` is set, and the theme is `system`.
    NoColor,
    /// The config named this palette, and an explicit choice wins — over `NO_COLOR` too, as
    /// no-color.org's own FAQ asks (decision 9).
    FromConfig(SchemeKind),
}

/// The outcome of [`resolve`]: what to draw with, at which depth, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    /// The colours to draw with — already at `depth`, and already past decision 6's guard.
    pub palette: Palette,
    /// Which depth they were resolved at.
    pub depth: Depth,
    /// Why this palette (decision 10).
    #[allow(
        dead_code,
        reason = "the Orientation row that shows it is this handoff's commit 4"
    )]
    pub reason: Reason,
    /// How many roles decision 6's guard sent back to the terminal's own colour.
    #[allow(
        dead_code,
        reason = "the Orientation row that shows it is this handoff's commit 4"
    )]
    pub fell_back: usize,
    /// Whether the resolver would have used an answer from the terminal.
    ///
    /// **This is what decides whether a query is sent at all**, and [`crate::run`] asks it of this
    /// same function before asking again with the answer — so what the tests assert about querying is
    /// what production does, not a second rule beside it.
    pub wants_an_answer: bool,
}

#[cfg(test)]
impl Resolved {
    /// A resolution that was not one: the palette a render test hands in, as if the config had named
    /// it. No guard ran, because no terminal answered.
    pub(crate) fn drawing_with(palette: Palette) -> Self {
        Self {
            palette,
            depth: Depth::TrueColour,
            reason: Reason::FromConfig(palette.kind),
            fell_back: 0,
            wants_an_answer: false,
        }
    }
}

/// Resolve the configured theme, the environment and the terminal's answer into the colours to draw
/// with (RFC 059 decisions 4, 5, 6 and 9). **Pure**: it reads nothing, sends nothing, and waits for
/// nothing.
#[must_use]
pub fn resolve(environment: &Environment) -> Resolved {
    let depth = environment.depth();
    let wants_an_answer = environment.wants_an_answer();
    let (scheme, reason) = match environment.theme {
        ConfigTheme::Dark => (Scheme::dark(), Reason::FromConfig(SchemeKind::Dark)),
        ConfigTheme::Light => (Scheme::light(), Reason::FromConfig(SchemeKind::Light)),
        ConfigTheme::System if environment.no_color() => (Scheme::plain(), Reason::NoColor),
        ConfigTheme::System => match environment.answer {
            Some(answer) => {
                let scheme = if answer.light {
                    Scheme::light()
                } else {
                    Scheme::dark()
                };
                // **Checked again on the background the terminal reported** (decision 6), which is the
                // one background the palette's references could not cover.
                let (palette, fell_back) = guard(scheme.for_depth(depth), answer.background);
                return Resolved {
                    palette,
                    depth,
                    reason: Reason::Detected {
                        light: answer.light,
                    },
                    fell_back,
                    wants_an_answer,
                };
            }
            None => match environment.colorfgbg_is_light() {
                Some(true) => (Scheme::light(), Reason::FromColorFgBg { light: true }),
                Some(false) => (Scheme::dark(), Reason::FromColorFgBg { light: false }),
                None => (Scheme::plain(), Reason::NoAnswer),
            },
        },
    };
    Resolved {
        palette: scheme.for_depth(depth),
        depth,
        reason,
        fell_back: 0,
        wants_an_answer,
    }
}

/// **Decision 6's guard.** Any role whose colour is below 7 : 1 on the background the terminal
/// reported shows in the terminal's own text colour instead, and the count of those comes back.
///
/// Applied **at the chosen depth**, because that is the colour the user will actually see: a role can
/// clear the bar in 24-bit and miss it as a cube entry, or the other way round.
fn guard(mut palette: Palette, background: (u8, u8, u8)) -> (Palette, usize) {
    let mut fell_back = 0;
    for role in [
        &mut palette.fg,
        &mut palette.dim,
        &mut palette.accent,
        &mut palette.ok,
        &mut palette.warn,
    ] {
        if let Checkable::Known(rgb) = checkable(*role)
            && contrast_ratio(rgb, background) < MINIMUM_CONTRAST
        {
            *role = Color::Reset;
            fell_back += 1;
        }
    }
    (palette, fell_back)
}

/// **Ask the terminal what its background is** (RFC 059 decision 5) — the one impure step, and the
/// only one in this module.
///
/// `None` is *"it did not answer"*, which [`resolve`] turns into `plain`. A terminal that does not
/// know `OSC 11` still answers `DA1`, so it is recognised as not answering at once rather than after
/// the wait (F2); the wait itself is the crate's default 1 s, which matters only where neither query
/// is answered and where latency is real, such as over SSH.
///
/// **Called before the alternate screen and raw mode**, once, by [`crate::run`] — never inside the UI
/// loop, and never a second time (trap 1). It needs stdin and stdout to be a terminal, which stikk
/// has already required of itself before it starts the TUI at all.
#[must_use]
pub fn detect() -> Option<Answer> {
    let answered =
        terminal_colorsaurus::color_palette(terminal_colorsaurus::QueryOptions::default()).ok()?;
    Some(Answer {
        light: answered.theme_mode() == terminal_colorsaurus::ThemeMode::Light,
        background: answered.background.scale_to_8bit(),
    })
}

#[cfg(test)]
mod tests;

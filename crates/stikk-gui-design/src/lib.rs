//! **The desktop app's design crate** (RFC 069 rule 1, RFC 070 decision 1).
//!
//! The only place in stikk that names a colour, a size, a font, a spacing or a Snora style. The
//! views live in [`stikk-gui`](../stikk_gui/index.html) and draw through this crate; they import
//! neither `snora::design` nor `iced::Color`, which is a check over their source (rule 1's test)
//! rather than a convention.
//!
//! **Why a crate and not a module:** one place to change, and a boundary a tool can hold. knotra's
//! 212 hand-written size sites were kept consistent only by a guard that scanned names (knotra's
//! analysis, §4), and a scan cannot see a size composed at run time.
//!
//! # What is here, and which rule asks for it
//!
//! | Item | Rule |
//! |---|---|
//! | [`body`], [`meta`], [`title`], [`heading`] — text by purpose, never by size | 2 |
//! | [`interface_font`] — the system's own interface font, named | 2 |
//! | [`Preset`] and [`theme`] — the four presets, with stikk's [`Overrides`] | 3, 4, 5 |
//! | [`isolate`], [`identifier`], [`meta_identifier`] — text that is not words | 12 |
//! | [`start`] — the start edge, aligned here and in no view | 4 |
//! | [`line()`] — lines, not panels | 4, 10 |
//! | [`render`] — the card on its scrim, and the shell around it | 4, 10 |
//! | [`scrollbar_spacing`] — every scrollable reserves its bar's room | 4 |
//! | [`button_primary`], [`button_secondary`] — Snora's buttons, with the unavailable floors | 3 |
//!
//! **No animation or shadow token beyond the card's** (rule 10). The shadow that marks a floating
//! card is Snora's own, applied by [`render`]; this crate exposes no other, and no duration.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod font;
mod preset;
mod style;
mod text;

pub use font::{interface_font, interface_font_name};
pub use preset::{Overrides, Preset, theme, tokens};
pub use style::{
    button_primary, button_secondary, card_width, line, render, scrollbar_spacing, start,
};
pub use text::{
    body, control_label, heading, identifier, isolate, meta, meta_identifier, role_sizes, strong,
    title,
};

/// The token bundle a preset resolves to, re-exported so a view never names `snora::design`.
///
/// It is Snora's own type: this crate adds stikk's [`Overrides`] to the preset's values rather than
/// wrapping them, so a Snora style function can still be given the tokens the app is drawing with.
pub type Tokens = snora::design::Tokens;

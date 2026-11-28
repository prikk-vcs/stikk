//! **The shell** (RFC 070 decision 3): one header line, the body, a status area, and the key line.
//!
//! The `GU-01` structure, drawn by RFC 069's rules. Snora composes the four slots; stikk fills
//! them. A card floats on the scrim in the dialog slot, which blocks the screen beneath for input
//! and for the pointer (knotra's RFC-034 lesson).

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, sensor};
use iced::{Element, Length};
use snora::AppLayout;
use stikk_gui_design as design;
use stikk_gui_design::Tokens;

use crate::words::Say;
use crate::{Message, Stikk, card, log, text, view};

/// The whole window.
///
/// **The sensor around everything is rule 8's third fact**: the size the shell's own root was laid
/// out at, which is the viewport the app actually used. Reported on arrival and on every change,
/// so a window that changes under the app withdraws its own confirmation.
pub fn view(stikk: &Stikk) -> Element<'_, Message> {
    view_with(stikk, Draws::shipped())
}

/// **Which parts of the shell are drawn** — the switches two guards need their failing run to
/// turn.
///
/// A shipped build is [`Draws::shipped`], and [`view()`] is the only caller that uses it. Nothing
/// else in the crate turns one off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draws {
    /// Rule 9: the status area draws every banner the program sets. **Off is rule 9's failing
    /// run** (RFC 070 §4.1: *"Run it with the status area's banner drawing removed, and it must
    /// fail"*).
    pub banner: bool,
    /// C2: each key-and-action pair is one unbreakable unit, and the line wraps between pairs
    /// only. **Off is that guard's failing run**: the key line as one string, which is the shape
    /// a real window broke between `v` and `Verify the repository`.
    pub key_pairs: bool,
}

impl Draws {
    /// Everything drawn, which is what a shipped build does.
    #[must_use]
    pub fn shipped() -> Self {
        Self {
            banner: true,
            key_pairs: true,
        }
    }
}

/// The whole window, with one of [`Draws`]'s parts turned off for a guard's failing run.
pub fn view_with(stikk: &Stikk, draws: Draws) -> Element<'_, Message> {
    let tokens = stikk.tokens();
    let mut layout = AppLayout::new(view::body(stikk, &tokens))
        .header(header(stikk, &tokens))
        .footer(footer(stikk, &tokens, draws));
    if let Some((overlay, surface)) = stikk.app().top_card() {
        layout = layout.dialog(snora::Dialog::new(card::view(
            overlay, surface, stikk, &tokens,
        )));
    }
    sensor(design::render(layout, &tokens))
        .on_show(Message::LaidOut)
        .on_resize(Message::LaidOut)
        .into()
}

/// **One header line:** the repository, and the screen's name.
fn header<'a>(stikk: &'a Stikk, tokens: &Tokens) -> Element<'a, Message> {
    let repository = text::identifier(&stikk.app().repo().display().to_string());
    let screen = log::screen_name(&stikk.app().focus());
    design::start(
        column![
            design::title(tokens, Say::ScreenName.fill(&[("screen", screen)])),
            design::meta_identifier(tokens, &repository),
        ]
        .spacing(tokens.spacing.xs),
    )
    .padding(tokens.spacing.md)
    .into()
}

/// **The status area and the key line.**
///
/// The status area draws **every banner the program sets** (rule 9) until the program clears it,
/// and its faults. RFC 065's W4 is why the rule exists: prikk refused *"nothing to commit"*, the
/// program closed the card and set a banner, and the prototype drew nothing — so the screen
/// silently did nothing, which is the confusion this project exists to prevent.
fn footer<'a>(stikk: &'a Stikk, tokens: &Tokens, draws: Draws) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(tokens.spacing.xs);
    if let Some(fault) = stikk.app().fault().filter(|_| draws.banner) {
        rows = rows.push(design::body(tokens, text::inert(fault)));
    }
    if let Some(banner) = stikk.app().banner().filter(|_| draws.banner) {
        rows = rows.push(design::body(tokens, text::inert(banner)));
    }
    rows = rows.push(design::line(tokens));
    rows = rows.push(key_line(stikk, tokens, draws.key_pairs));
    design::start(rows).padding(tokens.spacing.md).into()
}

/// **The key line: one unbreakable text per key-and-action pair, wrapping between pairs** (C2).
///
/// A key and its action are one unit. Laid out as one long string the line broke between `v` and
/// `Verify the repository` on a real window at 1280 × 720, so the `v` read as belonging to nothing
/// and the next line's first action read as having no key. Each pair is its own text with
/// `Wrapping::None`, so iced can only break the row **between** them.
fn key_line<'a>(stikk: &'a Stikk, tokens: &Tokens, pairs: bool) -> Element<'a, Message> {
    let here = stikk.app().keys_here();
    if !pairs {
        // The shape before C2, which this guard's failing run draws.
        return design::meta(tokens, crate::words::key_line_as_one_string(&here)).into();
    }
    let mut line = Row::new().spacing(tokens.spacing.xs);
    for entry in crate::words::key_line_entries(&here) {
        line = line.push(design::meta(tokens, entry).wrapping(Wrapping::None));
    }
    line.wrap().into()
}

/// The room a scrollable's own bar is given, as every scrollable in the app is given it (rule 4).
pub fn scrollable<'a>(
    content: impl Into<Element<'a, Message>>,
    id: &'static str,
    stikk: &Stikk,
    tokens: &Tokens,
    on_scroll: impl Fn(iced::widget::scrollable::Viewport) -> Message + 'a,
) -> iced::widget::Scrollable<'a, Message> {
    let overrides = stikk.overrides();
    let styling = tokens.clone();
    let scrollable = iced::widget::scrollable(content)
        .id(id)
        .height(Length::Fill)
        .on_scroll(on_scroll)
        // **The bar's thumb is a mark a reader has to be able to see** (rule 3), and iced's own
        // measured 1.45 : 1. The design crate chooses it from the preset's palette by measuring.
        .style(move |theme, status| design::scrollbar(&styling, overrides, theme, status));
    match design::scrollbar_spacing(tokens, overrides) {
        Some(spacing) => scrollable.spacing(spacing),
        None => scrollable,
    }
}

#[cfg(test)]
mod tests;

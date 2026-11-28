//! **The shell** (RFC 070 decision 3): one header line, the body, a status area, and the key line.
//!
//! The `GU-01` structure, drawn by RFC 069's rules. Snora composes the four slots; stikk fills
//! them. A card floats on the scrim in the dialog slot, which blocks the screen beneath for input
//! and for the pointer (knotra's RFC-034 lesson).

use iced::widget::{Column, column, sensor};
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
    view_with(stikk, true)
}

/// The whole window, with the status area's banner drawing on or off.
///
/// **`draws_banner` is rule 9's own failing run** (RFC 070 §4.1: *"Run it with the status area's
/// banner drawing removed, and it must fail"*). A shipped build is [`view()`], which passes `true`;
/// nothing else in the crate calls this with `false`.
pub fn view_with(stikk: &Stikk, draws_banner: bool) -> Element<'_, Message> {
    let tokens = stikk.tokens();
    let mut layout = AppLayout::new(view::body(stikk, &tokens))
        .header(header(stikk, &tokens))
        .footer(footer(stikk, &tokens, draws_banner));
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
fn footer<'a>(stikk: &'a Stikk, tokens: &Tokens, draws_banner: bool) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(tokens.spacing.xs);
    if let Some(fault) = stikk.app().fault().filter(|_| draws_banner) {
        rows = rows.push(design::body(tokens, text::inert(fault)));
    }
    if let Some(banner) = stikk.app().banner().filter(|_| draws_banner) {
        rows = rows.push(design::body(tokens, text::inert(banner)));
    }
    rows = rows.push(design::line(tokens));
    rows = rows.push(design::meta(tokens, keys_line(stikk)));
    design::start(rows).padding(tokens.spacing.md).into()
}

/// **The key line, from the catalog** (rule 7): a key exists only where the catalog binds it, so a
/// line that lists a key that does nothing is impossible. The words are the template table's.
#[must_use]
pub fn keys_line(stikk: &Stikk) -> String {
    crate::words::key_line(&stikk.app().keys_here())
}

/// The room a scrollable's own bar is given, as every scrollable in the app is given it (rule 4).
pub fn scrollable<'a>(
    content: impl Into<Element<'a, Message>>,
    id: &'static str,
    stikk: &Stikk,
    tokens: &Tokens,
    on_scroll: impl Fn(iced::widget::scrollable::Viewport) -> Message + 'a,
) -> iced::widget::Scrollable<'a, Message> {
    let scrollable = iced::widget::scrollable(content)
        .id(id)
        .height(Length::Fill)
        .on_scroll(on_scroll);
    match design::scrollbar_spacing(tokens, stikk.overrides()) {
        Some(spacing) => scrollable.spacing(spacing),
        None => scrollable,
    }
}

#[cfg(test)]
mod tests;

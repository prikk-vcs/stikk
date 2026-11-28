//! **The body: one screen at a time** (RFC 070 §4.4).
//!
//! Handoff A draws **Orientation** and **History**. Every other screen the program can focus says
//! so in one sentence, from the template table, rather than drawing nothing — a screen that is
//! blank for a reason nobody states is the confusion rule 9 exists to prevent.

pub mod history;
pub mod orientation;

use iced::Element;
use stikk_app::Focus;
use stikk_gui_design as design;
use stikk_gui_design::Tokens;

use crate::words::Say;
use crate::{Message, Stikk, log, shell};

/// The screen the program has focused.
pub fn body<'a>(stikk: &'a Stikk, tokens: &Tokens) -> Element<'a, Message> {
    let content: Element<'a, Message> = match stikk.app().focus() {
        Focus::Orientation(state) => orientation::view(state, stikk, tokens),
        Focus::History(view, cursor) => history::view(view, cursor, stikk, tokens),
        Focus::Loading(what) => {
            design::body(tokens, Say::LoadingScreen.fill(&[("screen", what)])).into()
        }
        other => design::body(
            tokens,
            Say::NotDrawnYet.fill(&[("screen", log::screen_name(&other))]),
        )
        .into(),
    };
    // **The page's padding is outside the scrollable, and the scrollable's content fills it.**
    //
    // The other way round — a padded container inside the scrollable — insets the content by the
    // page's own padding and so hides whether the bar's room was reserved at all: the bar would
    // be painted over 16 px of empty margin and rule 4's guard would never fire. With the padding
    // outside, the bar sits at the panel's own edge, which is where it belongs, and
    // `Scrollbar::spacing` is what keeps text out from under it (RFC 065 M5).
    iced::widget::container(shell::scrollable(
        design::start(content),
        crate::BODY_SCROLL,
        stikk,
        tokens,
        |_| Message::Ignored,
    ))
    .padding(tokens.spacing.md)
    .into()
}

/// A `label  value` row, the shape both screens lay their facts out in.
///
/// **The label is above its value when space is short** (rule 4), which is what a `Column` of
/// `Row`s cannot do and `iced`'s own wrapping does not reach — so the row is built as two
/// elements in a column whose first is the label, and the layout is the same at every width. No
/// layout here depends on English text length.
pub fn row<'a>(
    tokens: &Tokens,
    label: &'a str,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    iced::widget::column![design::meta(tokens, label), value]
        .spacing(tokens.spacing.xs)
        .into()
}

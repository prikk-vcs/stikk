//! **The generic card frame, and the Glossary drawn through it** (RFC 070 §4.4).
//!
//! One frame for every card: Snora's dialog slot, which the design crate's renderer gives its one
//! card chrome and its scrim. **The scrim blocks the screen beneath for input and for the
//! pointer** (knotra's RFC-034 lesson), so a key cannot reach a screen a card is covering.
//!
//! **Why the Glossary is in this handoff** and not with the rest of the cards: rule 8's
//! real-window reading sequence needs a card longer than the window, and the commit confirmation
//! — the other long card — is a writing card, which the desktop app does not open yet.
//!
//! **A card's entry is never a control that starts a change** (rule 6). Every card here is
//! read-only, so the entry is the way out.

use iced::widget::Column;
use iced::{Element, Length};
use stikk_app::{Overlay, Surface};
use stikk_gui_design as design;
use stikk_gui_design::Tokens;

use crate::words::Say;
use crate::{CARD_EXIT, CARD_FRAME, CARD_SCROLL, Message, Stikk, shell, text};

/// **One card.**
///
/// The two sensors are rule 8's measuring instruments: one on the first line, which says the
/// surface is on screen, and one past the last, which says its end is in view. Each names the
/// surface it was drawn for, so a late event from a card that has closed names a surface that is no
/// longer current and the program ignores it.
pub fn view<'a>(
    overlay: &'a Overlay,
    surface: Surface,
    stikk: &'a Stikk,
    tokens: &Tokens,
) -> Element<'a, Message> {
    let top = iced::widget::sensor(design::title(tokens, overlay.title().trim().to_string()))
        .key(surface)
        .on_show(move |_| Message::TopShown(surface));
    let mut lines = Column::new().spacing(tokens.spacing.sm).push(top);
    lines = lines.push(body(overlay, tokens));
    // A zero-height sentinel past the last line: when it is laid out, the end of the card is in
    // view. `on_hide` is as important as `on_show` — a card scrolled back up is not read.
    let end = iced::widget::sensor(design::meta(tokens, " "))
        .key(surface)
        .on_show(move |_| Message::EndVisible(surface, true))
        .on_hide(Message::EndVisible(surface, false));
    let scrolling = shell::scrollable(
        lines.push(end),
        CARD_SCROLL,
        stikk,
        tokens,
        move |viewport| Message::Scrolled(surface, viewport),
    );
    let hint = if stikk.app().at_end() {
        Say::CardWayOut
    } else {
        Say::CardMoreBelow
    };
    let mut inside = Column::new()
        .spacing(tokens.spacing.md)
        .push(scrolling)
        .push(design::meta(tokens, hint.template()));
    // **The way out, and the card's focus entry** (rule 6). One primary action per area (rule 10),
    // and it is the one that changes nothing.
    //
    // **Only on a card that has one.** A control that has focus takes `Enter` for itself — that is
    // iced's own button behaviour, through Snora's `Control` — so a card that is *chosen from*
    // must not have a way out as its entry, or `Enter` would close it instead of running the row
    // the cursor is on. Measured in the real window: the first keyboard pass reached the action
    // list, pressed `Enter`, and the card closed (2026-11-28). So a card this handoff does not
    // draw gets its sentence and no control, the catalog's keys keep reaching the program, and
    // Handoff C gives each card the entry it actually has.
    if entry_of(overlay).is_some() {
        inside = inside.push(design::button_primary(
            tokens,
            stikk.overrides(),
            CARD_EXIT,
            "Close",
            Some(Message::Press(stikk_core::Key::Back)),
        ));
    }
    design::start(inside)
        .id(CARD_FRAME)
        .width(Length::Fixed(design::card_width()))
        .padding(tokens.spacing.md)
        .into()
}

/// A card's own text.
///
/// Handoff A draws the **Glossary**; every other card says so in one sentence rather than drawing
/// an empty frame.
fn body<'a>(overlay: &'a Overlay, tokens: &Tokens) -> Element<'a, Message> {
    match overlay {
        Overlay::Glossary => glossary(tokens),
        other => design::body(
            tokens,
            Say::NotDrawnYet.fill(&[("screen", &text::inert(other.title().trim()))]),
        )
        .into(),
    }
}

/// **The Glossary**, from the product asset and the action catalog — **never a list of its own**.
///
/// The terminal's Keys section is the list that drifted furthest before RFC 061 (it still taught
/// five keys that had changed), so both front ends build it from `actions`. The terminology, the
/// codes and stikk's own notation come from `stikk_core::glossary`, which is a product asset
/// versioned with the release.
fn glossary<'a>(tokens: &Tokens) -> Element<'a, Message> {
    let mut rows = Column::new()
        .spacing(tokens.spacing.md)
        .push(design::meta(tokens, Say::GlossaryClaim.template()));

    rows = rows.push(design::title(tokens, Say::GlossaryKeys.template()));
    rows = rows.push(design::meta(tokens, Say::GlossaryLetters.template()));
    for action in stikk_core::actions::all() {
        let Some(key) = action.key else { continue };
        rows = rows.push(design::body(
            tokens,
            Say::GlossaryKeyRow.fill(&[("keys", &key.every()), ("action", action.name)]),
        ));
    }
    rows = rows.push(design::meta(tokens, Say::GlossaryByName.template()));

    rows = rows.push(design::title(tokens, Say::GlossaryTerms.template()));
    for term in stikk_core::glossary::terminology() {
        rows = rows.push(
            Column::new()
                .spacing(tokens.spacing.xs)
                .push(design::body(
                    tokens,
                    Say::GlossaryTermRow.fill(&[("git", term.git), ("prikk", term.prikk)]),
                ))
                .push(design::meta(tokens, term.note)),
        );
    }

    rows = rows.push(design::title(tokens, Say::GlossaryCodes.template()));
    for entry in stikk_core::glossary::code_entries() {
        rows = rows.push(
            Column::new()
                .spacing(tokens.spacing.xs)
                // prikk's own code, isolated: it is a name, not a word (rule 12).
                .push(design::identifier(tokens, &text::identifier(entry.code)))
                .push(design::strong(tokens, entry.title))
                .push(design::body(tokens, entry.explanation)),
        );
    }

    rows = rows.push(design::title(tokens, Say::GlossaryNotation.template()));
    for notation in stikk_core::glossary::notations() {
        rows = rows.push(
            Column::new()
                .spacing(tokens.spacing.xs)
                .push(design::strong(tokens, notation.title))
                .push(design::body(tokens, notation.note)),
        );
    }
    rows.into()
}

/// **The widget id a card's focus settles on** (rule 6), or `None` for a card that draws no
/// control.
///
/// Named here because the card is what draws it, and read by the one focus step: *"no open or
/// close site touches focus"*.
#[must_use]
pub fn entry_of(overlay: &Overlay) -> Option<&'static str> {
    match overlay {
        // A read-only card: its entry is the way out, which is never a control that starts a
        // change (rule 6).
        Overlay::Glossary => Some(CARD_EXIT),
        // A card this handoff draws as one sentence has no control at all, so focus settles
        // nowhere and every catalog key still reaches the program.
        _ => None,
    }
}

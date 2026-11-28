//! **Rule 8: the reading contract's front-end half.**
//!
//! RFC 067 Handoff C's four cases, on the desktop's own report, and the condition that makes the
//! desktop's half different from the terminal's: **nothing is reported until the viewport is
//! confirmed from the window.**

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iced::Size;
use stikk_app::{Reading, Surface};
use stikk_gui_design::Preset;

use super::{Report, Viewport};
use crate::harness::{self, Screen};

/// A viewport that has heard from the window and agrees with what was laid out.
fn confirmed() -> Viewport {
    Viewport {
        window: Some(Size::new(1280.0, 720.0)),
        scale: Some(1.0),
        laid_out: Some(Size::new(1280.0, 720.0)),
        app_scale: 1.0,
    }
}

/// **Confirmed needs all three facts, and the two sizes must agree.**
#[test]
fn a_viewport_is_confirmed_only_when_the_window_has_answered() {
    assert!(confirmed().confirmed());
    assert!(
        !Viewport::new(1.0).confirmed(),
        "a viewport that knows nothing is not confirmed"
    );
    for missing in [
        Viewport {
            window: None,
            ..confirmed()
        },
        Viewport {
            scale: None,
            ..confirmed()
        },
        Viewport {
            laid_out: None,
            ..confirmed()
        },
    ] {
        assert!(!missing.confirmed(), "{missing:?}");
    }
    // **The size the app laid out against must be the size the window says it has.**
    let disagreeing = Viewport {
        laid_out: Some(Size::new(1280.0, 600.0)),
        ..confirmed()
    };
    assert!(
        !disagreeing.confirmed(),
        "a layout against a different size is not this window's"
    );
    // A sub-pixel difference is not a disagreement about which window this is.
    let rounded = Viewport {
        laid_out: Some(Size::new(1279.6, 720.4)),
        ..confirmed()
    };
    assert!(rounded.confirmed());
}

/// The physical size is derived from both scales, and the line names every number.
#[test]
fn the_physical_size_is_the_logical_size_times_both_scales() {
    let viewport = Viewport {
        scale: Some(2.0),
        app_scale: 1.5,
        ..confirmed()
    };
    let physical = viewport.physical().unwrap();
    assert_eq!(physical.width, 1280.0 * 2.0 * 1.5);
    let line = viewport.line();
    for wanted in [
        "window 1280x720",
        "scale 2",
        "app_scale 1.5",
        "confirmed true",
    ] {
        assert!(line.contains(wanted), "{line:?} names {wanted:?}");
    }
}

/// **Nothing is reported until the viewport is confirmed** — and nothing is remembered either.
#[test]
fn a_report_before_the_window_has_answered_is_refused() {
    let mut report = Report::default();
    let surface = Surface::for_test(1);
    let unconfirmed = Viewport::new(1.0);
    assert!(
        report
            .measure(&unconfirmed, surface, Some(true), Some(false))
            .is_none(),
        "the end was in view and the window had not answered: nothing is reported"
    );
    assert!(
        report.measured().is_none(),
        "and nothing is kept, so it cannot be reported one message later"
    );
    // Once the window has answered, the same measurement is reported.
    let reading = report
        .measure(&confirmed(), surface, Some(true), Some(false))
        .expect("a confirmed viewport reports");
    assert_eq!(reading.surface, surface);
    assert!(reading.at_end);
}

/// **Each half of a measurement is remembered without claiming the other.**
#[test]
fn a_scroll_claims_nothing_about_the_end_and_a_sensor_nothing_about_the_top() {
    let mut report = Report::default();
    let surface = Surface::for_test(1);
    let first = report
        .measure(&confirmed(), surface, Some(true), None)
        .unwrap();
    assert!(first.at_end && !first.more_above);
    let second = report
        .measure(&confirmed(), surface, None, Some(true))
        .unwrap();
    assert!(
        second.at_end && second.more_above,
        "the end is still in view, and now something is above it"
    );
    // A different surface starts from nothing, so a closed card's reading cannot leak into it.
    let other = report
        .measure(&confirmed(), Surface::for_test(2), None, Some(true))
        .unwrap();
    assert!(!other.at_end, "a new surface has not been read");
}

/// **The whole app, with the window withheld:** a card drawn to its end stays unread.
///
/// This is rule 8's own failing case, both halves in one test: with the window's facts withheld the
/// card is never latched however it is drawn, and the same reading offered to the program directly
/// — *"the prototype's report, which reports at once"* — latches it. So the condition is what
/// refuses, and not an accident of the harness.
#[test]
fn a_card_drawn_to_its_end_is_unread_until_the_window_has_answered() {
    let settings = harness::drawing(Preset::LightAaa);
    // Tall enough that the Glossary's own end is laid out.
    let size = Size::new(1280.0, 8000.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.press(stikk_core::Key::Help);
    screen.draw();
    assert!(
        screen.stikk().app().top_overlay().is_some(),
        "the Glossary is open"
    );
    assert!(
        !screen.stikk().viewport().confirmed(),
        "the window has answered nothing yet: {}",
        screen.stikk().viewport().line()
    );
    assert!(
        !screen.stikk().app().seen_whole_card(),
        "nothing was reported, so the program's latch is closed — which is the safe failure"
    );

    // **The same reading, offered the way a front end with no condition would offer it.**
    let surface = screen.stikk().app().top_card().map(|(_, s)| s).unwrap();
    screen
        .stikk_mut()
        .report_reading_without_the_condition(Reading {
            surface,
            more_above: false,
            at_end: true,
        });
    assert!(
        screen.stikk().app().seen_whole_card(),
        "a report that skips the condition latches at once, which is what the condition prevents"
    );
}

/// **And once the window has answered, a card drawn to its end is read.**
#[test]
fn a_card_drawn_to_its_end_is_read_once_the_window_has_answered() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = Size::new(1280.0, 8000.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);
    screen.draw();
    assert!(screen.stikk().viewport().confirmed());
    assert!(
        screen.stikk().app().at_end(),
        "the whole card was laid out, and the drawing said so: {}",
        screen.stikk().state_line()
    );
    assert!(screen.stikk().app().seen_whole_card());
}

/// **And on a viewport too short for it, the same card is unread.**
///
/// The pair is the gate: the only difference between the two runs is how much of the card a real
/// layout put on screen.
#[test]
fn a_card_taller_than_the_window_is_unread() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);
    screen.draw();
    assert!(screen.stikk().viewport().confirmed());
    assert!(
        !screen.stikk().app().at_end(),
        "the Glossary is longer than the window: {}",
        screen.stikk().state_line()
    );
    assert!(!screen.stikk().app().seen_whole_card());
}

/// **A surface that was never drawn reports nothing** (RFC 067 Handoff C's second case).
#[test]
fn a_card_that_was_never_drawn_reports_nothing() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    // A card put on the stack without a draw: the program has a surface, and the drawing has
    // measured nothing for it.
    screen
        .stikk_mut()
        .push_card_for_test(stikk_app::Overlay::Glossary);
    assert!(
        !screen.stikk().app().seen_whole_card(),
        "no report means not read, which is the safe failure"
    );
    assert!(
        !screen.stikk().app().more_above(),
        "and nothing is claimed about the top either"
    );
    // **`at_end` is deliberately the other way round in the program**: *"no report means at the
    // end"*, because that is what an undrawn card meant when the renderer owned the offsets
    // (`App::at_end`'s own documentation). It is `seen_whole_card` that gates a confirmation, and
    // it is the one that fails safe — which is why rule 8 is written about the latch and not about
    // the offsets.
    assert!(screen.stikk().app().at_end());
}

/// **A card that replaces a read one starts unread** (RFC 067 Handoff C's first case).
///
/// Every card that reaches the top has a surface no earlier card had, so a reading reported for the
/// one before it cannot be read as this one's.
#[test]
fn a_card_replacing_a_read_card_starts_unread() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = Size::new(1280.0, 8000.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);
    screen.draw();
    assert!(
        screen.stikk().app().seen_whole_card(),
        "the first card is read"
    );
    let first = screen.stikk().app().top_card().map(|(_, s)| s).unwrap();

    screen
        .stikk_mut()
        .push_card_for_test(stikk_app::Overlay::Loading {
            what: "refs",
            seq: 1,
        });
    let second = screen.stikk().app().top_card().map(|(_, s)| s).unwrap();
    assert_ne!(first, second, "the new card has a surface of its own");
    assert!(
        !screen.stikk().app().seen_whole_card(),
        "and it starts unread, however well the card under it was read"
    );
}

/// **A reading for a surface that is no longer current is ignored** (the third and fourth cases:
/// a card replaced below the top keeps the top's reading, and the two latches agree through a
/// swap).
#[test]
fn a_late_reading_for_a_closed_card_is_ignored() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);
    let closed = screen.stikk().app().top_card().map(|(_, s)| s).unwrap();
    screen.key(harness::named(iced::keyboard::key::Named::Escape));
    assert!(screen.stikk().app().top_overlay().is_none());

    screen
        .stikk_mut()
        .push_card_for_test(stikk_app::Overlay::Glossary);
    let current = screen.stikk().app().top_card().map(|(_, s)| s).unwrap();
    assert_ne!(closed, current);

    // The sensor of the card that closed fires late, naming its own surface.
    screen.stikk_mut().offer_reading(Reading {
        surface: closed,
        more_above: false,
        at_end: true,
    });
    assert!(
        !screen.stikk().app().seen_whole_card(),
        "the card on top was not read by a report that names a surface that has gone: {}",
        screen.stikk().state_line()
    );
    assert!(
        !screen.stikk().app().more_above(),
        "and nothing of the closed card's position leaked into it: {}",
        screen.stikk().state_line()
    );
}

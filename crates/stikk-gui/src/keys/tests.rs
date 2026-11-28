//! **Rules 6 and 7:** the keys are the catalog's, an input method's own key is no action, and focus
//! settles after every update.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iced::keyboard::{Modifiers, key::Named};
use stikk_app::Intent;
use stikk_core::{Key, actions};
use stikk_gui_design::Preset;

use super::dispatch;
use crate::harness::{self, Screen, character, named};

/// **Rule 6: `Named(Process)` is no action.**
///
/// RFC 065's W3 measured Windows sending `Process` keys while an input method composes. Taken as a
/// letter it would act, which is how a Japanese user would find a card opening under their
/// composition.
#[test]
fn an_input_methods_own_key_is_no_action() {
    for text_focus in [false, true] {
        assert_eq!(
            dispatch(&named(Named::Process), Modifiers::empty(), text_focus),
            Intent::None,
            "a Process key with text_focus {text_focus}"
        );
    }
}

/// **A `Process` key changes no state**, through the whole app and not only the table.
#[test]
fn a_process_key_changes_no_state() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    let before = screen.stikk().state_line();
    screen.key(named(Named::Process));
    assert_eq!(screen.stikk().state_line(), before);
}

/// **Rule 7: the desktop app binds exactly the terminal's keys, from the catalog.**
///
/// Every key the catalog binds resolves to that key's own action, and the aliases are the ones
/// `stikk_core::Key` itself documents.
#[test]
fn every_key_the_catalog_binds_runs_its_own_action() {
    let run = |key: Key| actions::by_key(key).map(Intent::Run);
    let cases: Vec<(iced::keyboard::Key, Option<Intent>)> = vec![
        (named(Named::Enter), run(Key::Enter)),
        (named(Named::Escape), run(Key::Back)),
        (character("q"), run(Key::Back)),
        (character("Q"), run(Key::Back)),
        (named(Named::ArrowRight), run(Key::Right)),
        (named(Named::Space), run(Key::Space)),
        (character("?"), run(Key::Help)),
        (character(":"), run(Key::Palette)),
    ];
    for (pressed, expected) in cases {
        let Some(expected) = expected else { continue };
        assert_eq!(
            dispatch(&pressed, Modifiers::empty(), false),
            expected,
            "{pressed:?}"
        );
    }
    // The two halves of `app.move`, and the gentler half of `app.back`.
    assert_eq!(
        dispatch(&named(Named::ArrowUp), Modifiers::empty(), false),
        Intent::Up
    );
    assert_eq!(
        dispatch(&character("k"), Modifiers::empty(), false),
        Intent::Up
    );
    assert_eq!(
        dispatch(&character("J"), Modifiers::empty(), false),
        Intent::Down
    );
    assert_eq!(
        dispatch(&named(Named::ArrowLeft), Modifiers::empty(), false),
        Intent::BackWithoutQuit,
        "the left arrow goes back and never quits"
    );
}

/// **Every letter the catalog binds is folded**, so Caps Lock on is the same keyboard.
#[test]
fn a_letter_means_one_action_in_either_case() {
    for action in actions::all() {
        let Some(letter) = action.key.and_then(Key::letter) else {
            continue;
        };
        let lower: String = letter.to_lowercase().collect();
        let upper: String = letter.to_uppercase().collect();
        assert_eq!(
            dispatch(&character(&lower), Modifiers::empty(), false),
            dispatch(&character(&upper), Modifiers::empty(), false),
            "{letter:?} means one action in either case"
        );
    }
}

/// **Ctrl-C always quits**, whatever is open — the terminal's own first line.
#[test]
fn control_c_quits() {
    assert_eq!(
        dispatch(&character("c"), Modifiers::CTRL, false),
        Intent::Quit
    );
    assert_eq!(
        dispatch(&character("c"), Modifiers::CTRL, true),
        Intent::Quit,
        "even with a text field focused"
    );
}

/// **Rule 6: a letter never acts while a text field has focus.**
///
/// There is no text field in the app until Handoff C, so the only source of `text_focus` today is
/// the program's own `wants_text_input`; the table is already shaped for the second.
#[test]
fn a_letter_never_acts_while_a_text_field_has_focus() {
    for letter in ["c", "h", "q", "?", ":", "j", "k"] {
        let with = dispatch(&character(letter), Modifiers::empty(), true);
        assert_eq!(with, Intent::None, "{letter:?} acts nothing in a field");
    }
    // The keys a field does not take still reach the program.
    assert_eq!(
        dispatch(&named(Named::Backspace), Modifiers::empty(), true),
        Intent::Backspace
    );
    assert_eq!(
        dispatch(&named(Named::Escape), Modifiers::empty(), true),
        actions::by_key(Key::Back).map(Intent::Run).unwrap()
    );
}

/// **Rule 6, H: every catalog action Orientation and History offer is reachable by its key.**
#[test]
fn every_action_offered_here_is_reachable_by_its_key() {
    for fixture in [
        harness::on_orientation as fn() -> stikk_app::App,
        harness::on_history as fn() -> stikk_app::App,
    ] {
        let settings = harness::drawing(Preset::LightAaa);
        let size = harness::logical(harness::WINDOW, 1.0);
        let screen = Screen::open(fixture(), &settings, size);
        for action in screen.stikk().app().keys_here() {
            let Some(key) = action.key else { continue };
            let pressed = match key {
                Key::Letter(c) => character(&c.to_string()),
                Key::Enter => named(Named::Enter),
                Key::Right => named(Named::ArrowRight),
                Key::Back => named(Named::Escape),
                Key::Move => named(Named::ArrowDown),
                Key::Help => character("?"),
                Key::Palette => character(":"),
                Key::Space => named(Named::Space),
            };
            let intent = dispatch(&pressed, Modifiers::empty(), false);
            assert_ne!(
                intent,
                Intent::None,
                "{:?} is offered here and {pressed:?} reaches nothing",
                action.name
            );
        }
    }
}

/// **Rule 6: focus settles after every update** — a card takes focus on its own entry, and gives
/// it back where it was when it closes.
///
/// **knotra never proved this.** The assertion is on the app's own focus target, which is the one
/// thing both the headless level and the real window agree about; the ring itself is the R pass's.
#[test]
fn focus_returns_where_it_was_when_a_card_closes() {
    for fixture in [
        harness::on_orientation as fn() -> stikk_app::App,
        harness::on_history as fn() -> stikk_app::App,
    ] {
        let settings = harness::drawing(Preset::LightAaa);
        let size = harness::logical(harness::WINDOW, 1.0);
        let mut screen = Screen::open(fixture(), &settings, size);
        screen.window_says(size, 1.0);
        let before = screen.stikk().focus();
        assert_eq!(before, None, "no control has focus on a reading screen");

        screen.press(stikk_core::Key::Help);
        assert!(
            screen.stikk().app().top_overlay().is_some(),
            "the Glossary opened"
        );
        assert_eq!(
            screen.stikk().focus(),
            Some(crate::CARD_EXIT),
            "the card's focus is on its own way out, which is never a control that starts a change"
        );

        screen.key(named(Named::Escape));
        assert!(
            screen.stikk().app().top_overlay().is_none(),
            "the Glossary closed"
        );
        assert_eq!(
            screen.stikk().focus(),
            before,
            "focus is where it was before the card opened"
        );
    }
}

/// **The run that fails it:** without the settle step, focus stays on a control that is no longer
/// drawn.
///
/// The step is one function, so the counterfactual is that function not being called — which is
/// what this asserts, against the same state the test above walks.
#[test]
fn without_the_settle_step_focus_stays_on_a_closed_cards_control() {
    let settings = harness::drawing(Preset::LightAaa);
    let size = harness::logical(harness::WINDOW, 1.0);
    let mut screen = Screen::open(harness::on_orientation(), &settings, size);
    screen.window_says(size, 1.0);
    screen.press(stikk_core::Key::Help);
    assert_eq!(screen.stikk().focus(), Some(crate::CARD_EXIT));

    // Close the card **without** the update step that settles focus: the program's own close,
    // reached directly, which is what an open or close site touching focus itself would look like.
    screen.stikk_mut().close_card_without_settling_for_test();
    assert!(screen.stikk().app().top_overlay().is_none());
    assert_eq!(
        screen.stikk().focus(),
        Some(crate::CARD_EXIT),
        "with no settle step, focus is still on the control the closed card drew — which is the \
         defect the step exists to prevent"
    );
}

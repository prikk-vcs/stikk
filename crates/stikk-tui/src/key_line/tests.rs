//! **The key line's own tests** (RFC 061 decision 5).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_core::{RowKind, ScreenKind, actions};

use super::*;

fn palette() -> Palette {
    crate::theme::resolve(&crate::theme::Environment {
        theme: stikk_state::config::Theme::Dark,
        ..crate::theme::Environment::default()
    })
    .palette
}

fn text(actions: &[&'static Action], width: usize) -> String {
    line(actions, &palette(), width)
        .spans
        .iter()
        .map(|span| span.content.to_string())
        .collect()
}

/// **RFC 061 decision 5's own example**, on the root.
///
/// The example is **86 columns wide**, which is wider than the 80 the RFC's prose shows it in: it is
/// the form, not the fit. At 80, `s seal` is the lowest-ranked entry left and goes, leaving 78.
#[test]
fn the_root_is_the_rfcs_line() {
    let here = actions::keys_here(None, Some(ScreenKind::Orientation));
    let example = "Enter actions \u{00b7} \u{2191}\u{2193} move \u{00b7} b branches \u{00b7} \
                   w changes \u{00b7} c commit \u{00b7} s seal \u{00b7} ? help \u{00b7} q back";
    assert_eq!(example.chars().count(), 86);
    assert_eq!(text(&here, 86), example);
    assert_eq!(
        text(&here, 80),
        "Enter actions \u{00b7} \u{2191}\u{2193} move \u{00b7} b branches \u{00b7} w changes \
         \u{00b7} c commit \u{00b7} ? help \u{00b7} q back"
    );
    assert_eq!(text(&here, 80).chars().count(), 77);
}

/// **`? help` and `q back` survive every width**, and so does the floor.
#[test]
fn help_and_back_are_there_at_every_width() {
    for screen in [
        ScreenKind::Orientation,
        ScreenKind::History,
        ScreenKind::Changes,
        ScreenKind::Queue,
        ScreenKind::Trust,
        ScreenKind::PatchDetail,
        ScreenKind::Verify,
        ScreenKind::Bundle,
    ] {
        let here = actions::keys_here(Some(RowKind::Block), Some(screen));
        for width in [20, 40, 60, 72, 80, 120, 200] {
            let drawn = text(&here, width);
            assert!(drawn.contains("? help"), "{screen:?} at {width}: {drawn}");
            assert!(drawn.contains("q back"), "{screen:?} at {width}: {drawn}");
            // **Whole entries, never cut** — the line is as wide as it says it is.
            assert!(
                drawn.chars().count() <= width || width < 16,
                "{screen:?} at {width} draws {} columns: {drawn}",
                drawn.chars().count()
            );
        }
    }
}

/// **Entries drop by rank, lowest first**, and *"Enter actions"* goes last of the rest.
#[test]
fn the_lowest_rank_goes_first_and_enter_goes_last() {
    let here = actions::keys_here(None, Some(ScreenKind::Orientation));
    let wide = text(&here, 200);
    // At 200 every entry is there, including the lowest-ranked.
    assert!(wide.contains(": all actions"), "{wide}");
    // Narrowing drops it before anything that outranks it.
    let narrower = text(&here, 100);
    assert!(!narrower.contains(": all actions"), "{narrower}");
    assert!(narrower.contains("b branches"), "{narrower}");
    // At the floor, Enter is the last to go before the two that never drop.
    let floor = text(&here, 31);
    assert_eq!(floor, "Enter actions \u{00b7} ? help \u{00b7} q back");
    assert_eq!(floor.chars().count(), 31);
    let tighter = text(&here, 30);
    assert_eq!(tighter, "? help \u{00b7} q back");
}

/// **A screen's own keys are on it**, and not on a screen they do not apply to.
#[test]
fn a_screens_own_keys_are_on_its_line() {
    let changes = text(&actions::keys_here(None, Some(ScreenKind::Changes)), 200);
    assert!(changes.contains("u untracked"), "{changes}");
    assert!(changes.contains("d compare"), "{changes}");
    let queue = text(&actions::keys_here(None, Some(ScreenKind::Queue)), 200);
    assert!(!queue.contains("u untracked"), "{queue}");
    assert!(!queue.contains("f fields"), "{queue}");
    // A block row carries `m` and `d`, which belong to the row and not to History itself.
    let block = text(
        &actions::keys_here(Some(RowKind::Block), Some(ScreenKind::History)),
        200,
    );
    assert!(block.contains("m mark"), "{block}");
    assert!(block.contains("d compare"), "{block}");
}

/// **Every entry on the line is a key the catalog binds** (RFC 061 trap 1): the line names nothing
/// of its own.
#[test]
fn every_entry_comes_from_the_catalog() {
    let here = actions::keys_here(Some(RowKind::Ref), Some(ScreenKind::History));
    for action in &here {
        let key = action.key.expect("a key line entry has a key");
        let hint = action.hint.expect("and a word");
        assert!(
            text(&here, 200).contains(&format!("{} {hint}", key.label())),
            "{} is not on the line",
            action.id
        );
    }
}

//! Tests for the launcher's one-shot orientation print formatting (design C-T2a; review finding M1,
//! RFC 009).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn queued_line_without_a_target_shows_the_bare_count() {
    assert_eq!(queued_line(3, None), "  queued:      3");
}

#[test]
fn queued_line_with_a_target_shows_it() {
    assert_eq!(
        queued_line(1, Some("heads/main")),
        "  queued:      1 · targeting heads/main"
    );
}

#[test]
fn a_hostile_queued_target_is_rendered_inert() {
    // C-T2a: `queued_target` is repository-sourced (prikk's active-ref metadata); this one-shot path
    // has no raw terminal mode to protect it, but the line can still reach a real terminal (piped
    // through `less`, or a redirected file `cat`-ed later), so the same control applies as in the TUI.
    let line = queued_line(1, Some("\u{1b}[2Jheads/main"));
    assert!(!line.contains('\u{1b}'), "the ESC must be neutralized");
    assert!(line.contains('\u{FFFD}'));
}

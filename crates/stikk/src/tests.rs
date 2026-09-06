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
fn support_line_reads_the_validated_through_field_never_a_hardcoded_ceiling() {
    // RFC 015 C1: the TUI's copy of this exact sentence drifted silently for a release because it
    // hardcoded the ceiling instead of reading it; asserting against a sentinel here (never against
    // whatever the real ceiling currently is) is what makes that specific mistake impossible to repeat.
    let line = support_line(true, false, "0.99");
    assert!(line.contains("validated through 0.99"));
    assert!(line.contains("have not been checked"));
}

#[test]
fn support_line_below_the_floor_says_outside_the_validated_range() {
    let line = support_line(false, false, "0.99");
    assert!(line.contains("OUTSIDE"));
    assert!(!line.contains("0.99")); // the ceiling is irrelevant below the floor
}

#[test]
fn support_line_within_range_just_says_supported() {
    assert_eq!(support_line(true, true, "0.99"), "supported");
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

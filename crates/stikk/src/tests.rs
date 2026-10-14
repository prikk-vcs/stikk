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
fn a_hostile_queued_target_is_rendered_as_an_identifier() {
    // C-T2a: `queued_target` is repository-sourced (prikk's active-ref metadata); this one-shot path
    // has no raw terminal mode to protect it, but the line can still reach a real terminal (piped
    // through `less`, or a redirected file `cat`-ed later), so the same rules apply as in the TUI.
    //
    // **RFC 036 changed what this asserts.** It used to require `U+FFFD`, which said *something was
    // here* and not *what*. A ref name is an identifier, so the escape is written out and the whole
    // value is quoted — the ESC forces the quoted form.
    let line = queued_line(1, Some("\u{1b}[2Jheads/main"));
    assert!(
        !line.contains('\u{1b}'),
        "the ESC must not reach a terminal"
    );
    assert!(
        !line.contains('\u{FFFD}'),
        "and stikk no longer produces U+FFFD: one on screen is prikk's"
    );
    assert!(
        line.contains(r#""\u001b[2Jheads/main""#),
        "the ref, quoted, with the ESC spelled out: {line}"
    );
}

#[test]
fn a_ref_name_with_nothing_odd_in_it_prints_plainly() {
    // The common case costs nothing visually: no quotes, no escapes (RFC 036 decision 3).
    assert_eq!(
        queued_line(2, Some("heads/main")),
        "  queued:      2 · targeting heads/main"
    );
}

#[test]
fn the_printout_renders_the_root_the_version_and_the_id_too() {
    // RFC 036 decision 5: the three values that used to print raw. Driven through the same two
    // functions the TUI uses, so one rule cannot be true on one surface and false on the other.
    assert_eq!(
        stikk_tui::text::identifier("/home/u/my  repo"),
        r#""/home/u/my\u0020\u0020repo""#,
        "two spaces in a row force the quoted form, and neither can be collapsed downstream"
    );
    assert_eq!(
        stikk_tui::text::identifier("/home/u/my repo"),
        "/home/u/my repo",
        "one interior space is not ambiguous, so it stays plain (RFC 036 decision 3)"
    );
    assert_eq!(
        stikk_tui::text::identifier("/home/u/repo"),
        "/home/u/repo",
        "an ordinary root is plain"
    );
    assert_eq!(
        stikk_tui::text::inert("prikk 0.51.0\u{202e}"),
        r"prikk 0.51.0\u202e",
        "prikk's version line is prose: escaped in place, not quoted"
    );
}

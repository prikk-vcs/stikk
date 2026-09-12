//! Tests for the inert-text primitive (threat model C-T2a).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn plain_text_is_unchanged() {
    assert_eq!(inert("heads/main"), "heads/main");
    assert_eq!(inert("prikk 0.27.1"), "prikk 0.27.1");
}

#[test]
fn control_sequences_are_neutralized() {
    // An ANSI colour escape must not survive to the terminal.
    let hostile = "\u{1b}[31mred\u{1b}[0m";
    let safe = inert(hostile);
    assert!(!safe.contains('\u{1b}'), "the ESC byte must be replaced");
    assert!(safe.contains("red"), "the visible text is preserved");
    assert!(
        safe.contains('\u{FFFD}'),
        "control chars become the replacement char"
    );
}

#[test]
fn newlines_and_tabs_and_del_are_neutralized() {
    for hostile in ["a\nb", "a\tb", "a\u{7f}b", "a\rb"] {
        let safe = inert(hostile);
        assert!(
            !safe.chars().any(char::is_control),
            "no control char survives: {hostile:?}"
        );
    }
}

#[test]
fn empty_string_is_empty() {
    assert_eq!(inert(""), "");
}

#[test]
fn wrapping_never_exceeds_the_width_and_never_loses_a_word() {
    let text = "prikk stores repository paths with forward slashes on every platform and refuses \
                any path containing a backslash.";
    let lines = super::wrap_indented(text, 40, "    ");
    assert!(
        lines.len() > 1,
        "this text must actually wrap at 40: {lines:?}"
    );
    for line in &lines {
        assert!(
            line.chars().count() <= 40,
            "line exceeds the width: {line:?} ({} chars)",
            line.chars().count()
        );
        assert!(
            line.starts_with("    "),
            "every line carries the indent: {line:?}"
        );
    }
    // Nothing is dropped: the words come back in order, exactly once each.
    let round_trip: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
    let original: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(round_trip, original);
}

#[test]
fn a_word_longer_than_the_line_is_split_rather_than_clipped() {
    // The paragraph no longer wraps, so an over-long word would be silently cut off at the border —
    // the "truncated but present becomes absent" shape RFC 018 reverted, in miniature.
    let lines = super::wrap_indented("short aaaaaaaaaaaaaaaaaaaaaaaaa end", 14, "  ");
    for line in &lines {
        assert!(line.chars().count() <= 14, "{line:?}");
    }
    let joined: String = lines.join("").replace(' ', "");
    assert!(joined.contains("aaaaaaaaaaaaaaaaaaaaaaaaa"), "{lines:?}");
    assert!(joined.contains("end"), "{lines:?}");
}

#[test]
fn empty_text_is_still_one_line() {
    assert_eq!(super::wrap_indented("", 20, "  "), vec!["  ".to_string()]);
}

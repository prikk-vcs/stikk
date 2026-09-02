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

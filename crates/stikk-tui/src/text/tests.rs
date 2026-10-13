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

// ---------------------------------------------------------------------------------------------
// `wrap_verbatim` (RFC 035 review v1 §2.1): the wrap repository content needs.
// ---------------------------------------------------------------------------------------------

/// **Nothing is collapsed, trimmed or moved.** This is the whole point: `wrap_indented` rebuilds a line
/// from its whitespace-separated words, and content put through it renders a whitespace edit as no
/// edit at all.
#[test]
fn wrap_verbatim_changes_no_byte_of_its_input() {
    for text in [
        "\"  x\\n\"",
        "\"    x\\n\"",
        "\"a  b\"",
        "\"a b\"",
        "\"\\tx\"",
        "\"x  \"",
        "\"   \"",
    ] {
        assert_eq!(
            wrap_verbatim(text, 40, "  "),
            vec![format!("  {text}")],
            "a row that fits comes back as itself"
        );
    }
    // And the same texts through the word-wrap, for the contrast this exists to fix: two of them
    // become the same row.
    assert_eq!(
        wrap_indented("\"a  b\"", 40, ""),
        wrap_indented("\"a b\"", 40, "")
    );
}

/// A row wider than the viewport is **hard-split**, with every byte kept and every piece indented.
#[test]
fn wrap_verbatim_hard_splits_without_losing_anything() {
    let text = "0123456789abcdefghij";
    let rows = wrap_verbatim(text, 10, "  ");
    // Width 10 less a 2-column indent leaves 8 columns per row.
    assert_eq!(
        rows,
        vec![
            "  01234567".to_string(),
            "  89abcdef".to_string(),
            "  ghij".to_string()
        ]
    );
    let rejoined: String = rows
        .iter()
        .map(|row| row.trim_start_matches("  "))
        .collect();
    assert_eq!(rejoined, text, "every byte survives the split");
}

/// A multi-byte character is never split through the middle, and a narrow viewport still makes
/// progress rather than looping.
#[test]
fn wrap_verbatim_splits_on_char_boundaries_and_terminates() {
    let text = "日本語のテキスト";
    let rows = wrap_verbatim(text, 5, "  ");
    assert!(rows.len() > 1, "{rows:?}");
    let rejoined: String = rows
        .iter()
        .map(|row| row.trim_start_matches("  "))
        .collect();
    assert_eq!(rejoined, text);
    // An indent as wide as the width still leaves one column, so the wrap cannot spin.
    assert_eq!(wrap_verbatim("ab", 2, "  ").len(), 2);
}

/// An empty row stays a row, so a blank line in content is still a line.
#[test]
fn wrap_verbatim_keeps_an_empty_row() {
    assert_eq!(wrap_verbatim("", 20, "  "), vec!["  ".to_string()]);
}

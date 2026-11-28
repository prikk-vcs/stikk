//! **The two rules for text stikk did not write** (threat model C-T2a, `T-T2`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{compose, identifier, inert};

/// **Prose keeps its quotation marks and its backslashes**, and nothing is quoted.
#[test]
fn prose_is_left_as_prikk_wrote_it() {
    assert_eq!(inert(r#"prikk said "no""#), r#"prikk said "no""#);
    assert_eq!(inert("a path with a \\ in it"), "a path with a \\ in it");
    // Visible text is returned unchanged.
    assert_eq!(inert("é 日本 🙂"), "é 日本 🙂");
}

/// **Nothing invisible or reordering reaches the window.**
#[test]
fn prose_escapes_what_does_not_show_as_itself() {
    for (input, wanted) in [
        ("one\ntwo", "one\\ntwo"),
        ("one\ttwo", "one\\ttwo"),
        // A right-to-left override, which could reorder what is read.
        ("name\u{202e}txt.exe", "name\\u202etxt.exe"),
        // A zero-width space, which hides itself.
        ("he\u{200b}ads/main", "he\\u200bads/main"),
    ] {
        assert_eq!(inert(input), wanted, "{input:?}");
    }
}

/// **An ordinary identifier is shown as itself**, which is how a quoted one stands out.
#[test]
fn an_ordinary_identifier_is_plain() {
    for plain in ["heads/main", "src/main.rs", "237d0681aa11", "a b.txt"] {
        assert_eq!(identifier(plain), plain, "{plain:?}");
    }
}

/// **Two different identifiers never look alike**, and the quoted form holds no whitespace at all.
#[test]
fn an_identifier_that_cannot_be_shown_plainly_is_quoted() {
    for awkward in [
        "a  b.txt",
        " lead.txt",
        "trail.txt ",
        "say\"hi.txt",
        "back\\slash.txt",
        "",
    ] {
        let shown = identifier(awkward);
        assert!(
            shown.starts_with('"') && shown.ends_with('"'),
            "{awkward:?} is quoted: {shown:?}"
        );
        let inside = &shown[1..shown.len() - 1];
        assert!(
            !inside.chars().any(char::is_whitespace),
            "{awkward:?}: the quoted form holds no whitespace, so no layout can change it: \
             {shown:?}"
        );
    }
    // **And they stay different.** `a  b.txt` and `a b.txt` would read alike if whitespace were
    // left to the layout; prikk accepts both as ordinary paths.
    assert_ne!(identifier("a  b.txt"), identifier("a b.txt"));
}

/// **The one place the two rules meet:** core decides the words and which pieces are names.
#[test]
fn a_composed_sentence_takes_each_rule_where_it_belongs() {
    let pieces = vec![
        stikk_core::Piece::Said("2 patches are queued for ".to_string()),
        stikk_core::Piece::Name("a  branch.txt".to_string()),
    ];
    let sentence = compose(&pieces);
    assert!(sentence.starts_with("2 patches are queued for "));
    assert!(
        sentence.contains("\\u0020"),
        "the name took the identifier rule: {sentence:?}"
    );
}

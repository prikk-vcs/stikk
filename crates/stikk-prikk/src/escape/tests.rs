//! Tests for the one escape table (RFC 035 review v2 §2).
//!
//! **The claim under test is an invariant, not a spelling**: two pieces of repository text that
//! differ in any byte must not render identically. The table in the review result lists the four
//! families that broke it when only category `Cc` was escaped, and there is a case here for each.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

/// Row 1 of the review's table: bidi controls. Each one is escaped, so the row a reader sees is in
/// the order of the bytes prikk reported — a terminal has no override left to act on.
#[test]
fn bidi_controls_are_escaped() {
    for ch in [
        '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}',
        '\u{2068}', '\u{2069}', '\u{200E}', '\u{200F}', '\u{061C}',
    ] {
        assert!(
            needs_unicode_escape(ch),
            "U+{:04X} reorders displayed text and must not pass through",
            u32::from(ch)
        );
    }
}

/// A Trojan Source span (CVE-2021-42574): the bytes say one thing, a terminal honouring the override
/// would show another. stikk shows the bytes.
#[test]
fn a_bidi_override_renders_in_byte_order() {
    // `if access_level != "user\u{202E} \u{2066}// Check if admin\u{2069} \u{2066}"` — the shape the
    // CVE is usually demonstrated with.
    let span = "if access_level != \"user\u{202E} \u{2066}// Check if admin\u{2069} \u{2066}\" {";
    let rendered = quoted(span);
    assert!(
        !rendered.contains('\u{202E}') && !rendered.contains('\u{2066}'),
        "no override may survive into the rendered row: {rendered}"
    );
    assert_eq!(
        rendered,
        "\"if access_level != \\\"user\\u202e \\u2066// Check if admin\\u2069 \\u2066\\\" {\""
    );
    // The visible characters are still in the order prikk reported them.
    let visible: String = rendered.chars().filter(|ch| ch.is_alphabetic()).collect();
    assert!(visible.starts_with("ifaccess"), "got {visible}");
}

/// Row 2: zero-width characters. `"x"` and `"x\u{200b}"` looked alike before this.
#[test]
fn zero_width_characters_are_escaped_and_break_no_look_alike() {
    for ch in [
        '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}', '\u{00AD}',
    ] {
        assert!(needs_unicode_escape(ch), "U+{:04X}", u32::from(ch));
    }
    assert_ne!(quoted("x"), quoted("x\u{200b}"));
    assert_eq!(quoted("x\u{200b}"), "\"x\\u200b\"");
}

/// Row 3: whitespace that is not `U+0020`. A non-breaking space looks like a space; `U+2028` breaks
/// the row.
#[test]
fn whitespace_other_than_the_space_is_escaped() {
    for ch in [
        '\u{00A0}', '\u{2000}', '\u{2001}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{3000}',
        '\u{1680}', '\u{2028}', '\u{2029}',
    ] {
        assert!(needs_unicode_escape(ch), "U+{:04X}", u32::from(ch));
    }
    assert!(
        !needs_unicode_escape(' '),
        "`U+0020` is the one whitespace character that shows as itself"
    );
    assert_ne!(quoted("a b"), quoted("a\u{a0}b"));
    assert_eq!(quoted("a\u{a0}b"), "\"a\\u00a0b\"");
}

/// Row 4: tag characters, which display nothing at all, and sit above `U+FFFF`.
#[test]
fn tag_characters_are_escaped_as_surrogate_pairs() {
    assert!(needs_unicode_escape('\u{E0001}'));
    assert!(needs_unicode_escape('\u{E0041}'));
    assert_eq!(quoted("\u{E0001}"), "\"\\udb40\\udc01\"");
}

/// `\u{:04x}` alone was not injective above `U+FFFF`: `U+E0001` printed `\ue0001`, which also reads
/// as `U+E000` followed by `1`. The surrogate-pair form is the fix, and the two spellings differ.
#[test]
fn an_astral_escape_cannot_be_read_as_a_bmp_escape_and_a_digit() {
    assert_ne!(quoted("\u{E0001}"), quoted("\u{E000}1"));
    assert_eq!(
        quoted("\u{E000}1"),
        "\"\u{E000}1\"",
        "a private-use character shows as itself"
    );
}

/// Visible non-ASCII is content, not a hazard: it shows as itself and stays as itself.
#[test]
fn ordinary_visible_characters_pass_through() {
    for text in ["é", "日本語", "🦀", "Ωmega", "naïve café"] {
        assert_eq!(
            quoted(text),
            format!("\"{text}\""),
            "{text} must not be escaped"
        );
    }
}

/// Control characters keep the behaviour that was already right, with JSON's short spellings where
/// JSON has one.
#[test]
fn controls_keep_their_escapes() {
    assert_eq!(
        quoted("a\nb\tc\rd\u{1b}e\u{8}f\u{c}g\0h"),
        "\"a\\nb\\tc\\rd\\u001be\\bf\\fg\\u0000h\""
    );
    assert_eq!(quoted("say \"hi\"\\"), "\"say \\\"hi\\\"\\\\\"");
}

/// The invariant itself, over the whole look-alike set at once: every one of these differs from
/// every other in at least one byte, so no two may render the same.
#[test]
fn no_two_distinct_spans_render_identically() {
    let spans = [
        "x",
        "x ",
        "x\u{a0}",
        "x\u{200b}",
        "x\u{2060}",
        "x\u{feff}",
        "x\n",
        "x\r\n",
        "x\t",
        "x  ",
        "  x",
        "\tx",
        "x\u{202e}",
        "x\u{e0001}",
        "x\u{ad}",
        "x\u{3000}",
    ];
    for (at, left) in spans.iter().enumerate() {
        for right in spans.iter().skip(at + 1) {
            assert_ne!(
                quoted(left),
                quoted(right),
                "{left:?} and {right:?} render alike"
            );
        }
    }
}

/// The range table has to stay sorted and disjoint, because [`needs_unicode_escape`] binary-searches
/// it. A test rather than a comment: an inserted range in the wrong place would silently stop
/// escaping everything above it.
#[test]
fn the_ignorable_table_is_sorted_and_disjoint() {
    let mut previous_high = None;
    for &(low, high) in IGNORABLE {
        assert!(low <= high, "range U+{low:04X}..=U+{high:04X} is inverted");
        if let Some(previous) = previous_high {
            assert!(
                previous < low,
                "U+{low:04X} does not follow U+{previous:04X}"
            );
        }
        previous_high = Some(high);
    }
}

/// `escape_into` appends, so a caller can build a row around it; `quoted` is it with the quotes.
#[test]
fn escape_into_appends_without_quotes() {
    let mut out = String::from("prefix: ");
    escape_into(&mut out, "a\tb");
    assert_eq!(out, "prefix: a\\tb");
}

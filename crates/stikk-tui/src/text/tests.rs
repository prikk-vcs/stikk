//! Tests for the inert-text primitive (threat model C-T2a).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

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
    assert!(!safe.contains('\u{1b}'), "the ESC byte must not survive");
    assert!(safe.contains("red"), "the visible text is preserved");
    // **RFC 036 changed what this asserts.** It used to require `U+FFFD`, which said *something was
    // here* without saying what. The escape says which character it was, and `U+FFFD` is now
    // prikk's alone.
    assert_eq!(safe, r"\u001b[31mred\u001b[0m");
    assert!(!safe.contains('\u{FFFD}'), "stikk produces no U+FFFD");
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

/// A row wider than the viewport is **hard-split**, with every byte kept and every piece
/// indented — and the pieces are **as equal as possible** (RFC 056 Handoff B §1).
#[test]
fn wrap_verbatim_hard_splits_evenly_without_losing_anything() {
    let text = "0123456789abcdefghij";
    let rows = wrap_verbatim(text, 10, "  ");
    // Width 10 less a 2-column indent leaves 8 columns per row, so 20 characters need three
    // rows — and three rows of 7, 7 and 6 rather than 8, 8 and 4.
    assert_eq!(
        rows,
        vec![
            "  0123456".to_string(),
            "  789abcd".to_string(),
            "  efghij".to_string()
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

// =============================================================================================
// RFC 036 — the two rules.
//
// **The claims under test are invariants.** Prose: nothing that does not show as itself reaches a
// cell, and `"`/`\` are untouched. Identifier: the same, **plus** two different identifiers never
// render alike, and no layout pass can change one.

/// Each trigger for the quoted form, alone, so a future change to one cannot hide behind another.
#[test]
fn each_trigger_quotes_and_nothing_else_does() {
    for (what, value, expected) in [
        ("a bidi override", "ev\u{202e}il", "\"ev\\u202eil\""),
        ("a zero-width space", "a\u{200b}b", "\"a\\u200bb\""),
        ("a non-breaking space", "a\u{a0}b", "\"a\\u00a0b\""),
        ("a tab", "a\tb", "\"a\\tb\""),
        ("a quotation mark", "say\"hi", "\"say\\\"hi\""),
        ("a backslash", "a\\b", "\"a\\\\b\""),
        ("a leading space", " lead", "\"\\u0020lead\""),
        ("a trailing space", "trail ", "\"trail\\u0020\""),
        ("two spaces", "a  b", "\"a\\u0020\\u0020b\""),
        ("the empty value", "", "\"\""),
    ] {
        assert_eq!(identifier(value), expected, "{what}");
    }
    for plain in [
        "heads/main",
        "src/main.rs",
        "a b.txt",
        "é",
        "日本語",
        "🦀",
        "4db1669a176b",
        "author",
    ] {
        assert_eq!(identifier(plain), plain, "{plain} must stay plain");
    }
}

/// **The identifier invariant**: RFC 035's sixteen look-alike spans, plus the three whitespace
/// shapes prikk's own path validator accepts, no two of which may render the same.
#[test]
fn no_two_distinct_identifiers_render_alike() {
    let values = [
        "x",
        "x ",
        " x",
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
        "a b",
        "a  b",
        " a b",
        "",
    ];
    for (at, left) in values.iter().enumerate() {
        for right in values.iter().skip(at + 1) {
            assert_ne!(
                identifier(left),
                identifier(right),
                "{left:?} and {right:?} render alike"
            );
        }
    }
}

/// **Prose escapes every row of RFC 035 A3's table, and leaves `"` and `\` alone.** The second half
/// is the difference between the two rules: a refusal prikk wrote with a quotation mark in it must
/// read as prikk wrote it (`ER-02`).
#[test]
fn prose_escapes_the_invisible_and_keeps_the_rest() {
    for (what, value, expected) in [
        ("a bidi override", "ev\u{202e}il", "ev\\u202eil"),
        ("a zero-width space", "a\u{200b}b", "a\\u200bb"),
        ("a non-breaking space", "a\u{a0}b", "a\\u00a0b"),
        ("an ideographic space", "a\u{3000}b", "a\\u3000b"),
        ("a tag character", "A\u{e0041}", "A\\udb40\\udc41"),
        ("an ESC", "\u{1b}[2J", r"\u001b[2J"),
        ("a newline", "a\nb", "a\\nb"),
    ] {
        assert_eq!(inert(value), expected, "{what}");
    }
    assert_eq!(
        inert(r#"prikk said "no" at a\b"#),
        r#"prikk said "no" at a\b"#,
        "prose is never quoted and never escapes a quote or a backslash"
    );
    for visible in ["é", "日本語", "🦀", "plain words"] {
        assert_eq!(inert(visible), visible, "{visible} must pass through");
    }
}

/// **Trap 4: a quoted identifier survives `wrap_indented` unchanged**, at every width, because it
/// holds no whitespace for `split_whitespace` to act on.
#[test]
fn a_quoted_identifier_survives_wrapping_at_every_width() {
    let quoted = identifier("a  b\u{202e}c.txt");
    assert!(quoted.starts_with('"'), "{quoted}");
    assert!(
        !quoted.chars().any(char::is_whitespace),
        "the quoted form holds no whitespace: {quoted}"
    );
    for width in [20, 30, 40, 60, 80, 120] {
        let sentence = format!("create-file {quoted} — prikk reported it");
        let rows = wrap_indented(&sentence, width, "  ");
        let joined: String = rows
            .iter()
            .map(|row| row.trim_start().to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            joined.contains(&quoted) || rows.len() > 1,
            "width {width}: {rows:?}"
        );
        // Whatever the width, every character of the identifier is still present, in order.
        let all: String = rows.concat();
        let mut haystack = all.chars().filter(|c| !c.is_whitespace());
        for wanted in quoted.chars() {
            assert!(
                haystack.any(|c| c == wanted),
                "width {width}: {wanted:?} lost from {rows:?}"
            );
        }
    }
}

/// **`U+FFFD` is prikk's now** (trap 3). stikk produces none, and one that arrives from prikk — the
/// character it substitutes for filename bytes it cannot show — reaches the screen as itself, which
/// is what makes `stikk_core::would_refuse_next_steps`'s extra step exact.
#[test]
fn a_replacement_character_from_prikk_is_shown_as_itself() {
    let name = "bad\u{FFFD}name.txt";
    assert_eq!(identifier(name), name, "it needs no escape and no quoting");
    assert_eq!(inert(name), name);
    assert!(
        stikk_core::would_refuse_next_steps(&[stikk_core::RefusedPath {
            path: name.to_string(),
            kind: stikk_core::ChangeKind::Unsupported,
            reason: "invalid name".to_string(),
        }])
        .iter()
        .any(|step| step.contains('\u{FFFD}')),
        "the next step still fires on prikk's own substitution"
    );
}

/// **One value splits into equal rows** (RFC 056 Handoff B §1), not a full row and a remainder:
/// a 64-hex key reads as 32 + 32, which can be compared against another key, where 63 + 1 reads
/// as a defect.
///
/// **The row count is the same** as filling each row would have used — this changes where the
/// characters sit, never how many rows they need.
#[test]
fn one_value_splits_into_rows_as_equal_as_possible() {
    let hex = "a1b2c3d4".repeat(8); // 64 characters, as a public key is
    assert_eq!(hex.chars().count(), 64);
    // 65 columns less a two-column indent leaves 63, so two rows — of 32 and 32.
    let rows = wrap_verbatim(&hex, 65, "  ");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].trim_start().chars().count(), 32);
    assert_eq!(rows[1].trim_start().chars().count(), 32);

    // 120 characters over the same 63 columns: two rows of 60.
    let long = "a1b2c3d4".repeat(15);
    assert_eq!(long.chars().count(), 120);
    let rows = wrap_verbatim(&long, 65, "  ");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].trim_start().chars().count(), 60);
    assert_eq!(rows[1].trim_start().chars().count(), 60);

    // Exactly the available width is one row, untouched.
    let exact: String = "x".repeat(63);
    assert_eq!(wrap_verbatim(&exact, 65, "  "), vec![format!("  {exact}")]);

    // And nothing is lost, in any of them.
    let rejoined: String = wrap_verbatim(&hex, 65, "  ")
        .iter()
        .map(|row| row.trim_start_matches(' ').to_string())
        .collect();
    assert_eq!(rejoined, hex);
}

/// **Characters, not bytes** — the same rule the helpers already measure by, so a multi-byte
/// value splits where the terminal grid would, and never mid-character.
#[test]
fn an_even_split_counts_characters_not_bytes() {
    // Each of these is three bytes and one column's worth of `char`.
    let text = "ねこ".repeat(10); // 20 characters, 60 bytes
    assert_eq!(text.chars().count(), 20);
    let rows = wrap_verbatim(&text, 10, "");
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0].chars().count(), 10);
    assert_eq!(rows[1].chars().count(), 10);
    assert_eq!(
        rows.concat(),
        text,
        "nothing is lost, and nothing is cut in half"
    );
}

/// **`wrap_indented`'s hard-split is even too**, and the text around the over-long word still
/// wraps on whitespace as it always did.
#[test]
fn wrap_indented_hard_splits_an_over_long_word_evenly() {
    let word = "x".repeat(20);
    let rows = wrap_indented(&format!("see {word} now"), 10, "");
    // "see" on its own row, then the word in two equal halves, then "now".
    assert_eq!(rows[0], "see");
    assert_eq!(rows[1].chars().count(), 10);
    assert_eq!(rows[2].chars().count(), 10);
    assert_eq!(rows[3], "now");
    assert_eq!(rows[1..3].concat(), word, "nothing is lost");
}

/// **An escape is never cut in half** (RFC 056 Handoff B §4 trap 2): `\uXXXX` is one piece, and
/// a surrogate pair is one piece, because half of either reads as neither the escape nor the
/// character it stands for.
#[test]
fn an_escape_at_the_split_point_stays_whole_on_one_row() {
    // A value whose escape falls exactly where an even split would otherwise cut: eight
    // ordinary characters, the escape, then eight more, split over two rows.
    let escaped = format!("{}\\u202e{}", "a".repeat(8), "b".repeat(8));
    let rows = wrap_verbatim(&escaped, 11, "");
    assert!(rows.len() >= 2, "{rows:?}");
    assert!(
        rows.iter().any(|row| row.contains("\\u202e")),
        "the escape is whole on one row: {rows:?}"
    );
    for row in &rows {
        assert!(
            !row.ends_with('\\') && !row.ends_with("\\u") && !row.ends_with("\\u2"),
            "no row ends inside an escape: {rows:?}"
        );
    }
    assert_eq!(rows.concat(), escaped, "and nothing is lost");

    // A surrogate pair is one piece too: `😀` is one character's escape.
    let pair = format!("{}\\ud83d\\ude00{}", "a".repeat(6), "b".repeat(6));
    let rows = wrap_verbatim(&pair, 10, "");
    assert!(
        rows.iter().any(|row| row.contains("\\ud83d\\ude00")),
        "the pair is whole on one row: {rows:?}"
    );
    assert_eq!(rows.concat(), pair);
}

/// **No row is ever wider than the available width** (RFC 056 Handoff B review v1): a piece is
/// one `char` **or one whole escape**, and an escape is six columns, so rows balanced by piece
/// *count* could come out far wider than the row they sit in.
///
/// The review's own example: 63 pieces, ten of them escapes, at 63 columns available. Counting
/// pieces, that is "63 ≤ 63" and one row of 113 columns, which ratatui clips at the border —
/// and **the ids most likely to be full of escapes are the adversarial ones**, since a bundle's
/// sealer id is chosen by whoever sealed it (RFC 054).
#[test]
fn no_row_is_wider_than_the_available_width_however_many_escapes() {
    let escaped = "\\u202e".repeat(10) + &"a".repeat(53); // 63 pieces, 113 columns
    assert_eq!(escaped.chars().count(), 113);
    let rows = wrap_verbatim(&escaped, 63, "");
    for row in &rows {
        assert!(
            row.chars().count() <= 63,
            "a row of {} columns: {rows:?}",
            row.chars().count()
        );
    }
    // Every escape is still whole, and the rows rejoin to the input.
    assert_eq!(rows.concat(), escaped, "nothing is lost");
    assert_eq!(
        rows.iter()
            .map(|row| row.matches("\\u202e").count())
            .sum::<usize>(),
        10,
        "every escape is whole on some row: {rows:?}"
    );
}

/// **A value made only of escapes** balances by width too: twenty escapes are 120 columns, so
/// at 63 available they come out as two rows of ten.
#[test]
fn a_value_of_only_escapes_balances_by_width() {
    let escaped = "\\u202e".repeat(20);
    assert_eq!(escaped.chars().count(), 120);
    let rows = wrap_verbatim(&escaped, 63, "");
    assert_eq!(rows.len(), 2, "{rows:?}");
    for row in &rows {
        assert_eq!(row.chars().count(), 60, "{rows:?}");
        assert_eq!(row.matches("\\u202e").count(), 10, "{rows:?}");
    }
    assert_eq!(rows.concat(), escaped);
}

/// **A surrogate pair at a row boundary stays whole** — twelve columns that mean one character,
/// and nothing by halves.
#[test]
fn a_surrogate_pair_at_a_row_boundary_stays_whole() {
    // Six columns of plain text, then the pair, then more: at a width of 10 the pair cannot sit
    // on the first row, so it must start the second rather than be cut.
    let text = format!("{}\\ud83d\\ude00{}", "a".repeat(6), "b".repeat(6));
    let rows = wrap_verbatim(&text, 10, "");
    assert!(
        rows.iter().any(|row| row.contains("\\ud83d\\ude00")),
        "{rows:?}"
    );
    for row in &rows {
        assert!(row.chars().count() <= 12, "{rows:?}");
    }
    assert_eq!(rows.concat(), text);
}

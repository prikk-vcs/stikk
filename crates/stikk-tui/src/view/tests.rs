//! Tests for [`super::wrap_spans`] (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::{wrap_hanging, wrap_spans};

/// The row's text, rows joined by `|`, each continuation still carrying its indent.
fn shape(rows: &[ratatui::text::Line<'_>]) -> String {
    rows.iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn a_row_that_fits_is_one_line() {
    let rows = wrap_spans(vec![Span::raw("abcdef")], 10, "  ");
    assert_eq!(shape(&rows), "abcdef");
}

#[test]
fn a_long_row_continues_under_the_indent() {
    let rows = wrap_spans(vec![Span::raw("abcdefghij")], 6, "  ");
    // 6 columns on the first row, then 4 under a 2-column indent.
    assert_eq!(shape(&rows), "abcdef|  ghij");
}

#[test]
fn every_character_survives_in_order() {
    let spans = vec![
        Span::raw("one "),
        Span::raw("two "),
        Span::raw("three "),
        Span::raw("four"),
    ];
    let rows = wrap_spans(spans, 7, "   ");
    let rebuilt: String = rows
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            // Drop the indent every row but the first adds.
            if i == 0 {
                text
            } else {
                text.strip_prefix("   ").unwrap().to_string()
            }
        })
        .collect();
    assert_eq!(rebuilt, "one two three four");
}

#[test]
fn a_split_keeps_each_spans_own_style() {
    let red = Style::default().fg(Color::Red);
    let blue = Style::default().fg(Color::Blue);
    let rows = wrap_spans(
        vec![Span::styled("aaaa", red), Span::styled("bbbb", blue)],
        3,
        "",
    );
    // Walk every rendered character and assert the colour it was given.
    let mut seen: Vec<(char, Option<Color>)> = Vec::new();
    for line in &rows {
        for span in &line.spans {
            for ch in span.content.chars() {
                seen.push((ch, span.style.fg));
            }
        }
    }
    let expected: Vec<(char, Option<Color>)> = "aaaa"
        .chars()
        .map(|c| (c, Some(Color::Red)))
        .chain("bbbb".chars().map(|c| (c, Some(Color::Blue))))
        .collect();
    assert_eq!(seen, expected);
}

#[test]
fn a_multi_byte_character_is_never_split_in_half() {
    // Each glyph is three bytes; a byte split would render replacement glyphs instead.
    let rows = wrap_spans(vec![Span::raw("\u{25b6}\u{25cf}\u{2190}")], 2, "");
    assert_eq!(shape(&rows), "\u{25b6}\u{25cf}|\u{2190}");
}

#[test]
fn an_indent_is_never_left_as_a_row_of_its_own() {
    // The text ends exactly on a row boundary: 5 columns, then 3 under a 2-column indent.
    let rows = wrap_spans(vec![Span::raw("abcdefgh")], 5, "  ");
    assert_eq!(shape(&rows), "abcde|  fgh");
    assert_eq!(rows.len(), 2);
}

#[test]
fn an_empty_span_sequence_renders_no_rows() {
    assert!(wrap_spans(Vec::new(), 10, "  ").is_empty());
}

// ---------------------------------------------------------------------------------------------
// `wrap_hanging`: each row gets the room its own indent leaves.

#[test]
fn a_hanging_wrap_fills_the_first_row_to_its_own_width() {
    // Wrapping the whole sentence to the *hanging* room would stop the first row short of the
    // border for no reason a reader can see.
    // Width 11: the first row has 9 columns to spend and takes two words; the hanging rows have
    // 5 and take one each. Wrapped to the hanging room throughout, the first row would hold only
    // `aaa`.
    let rows = wrap_hanging("aaa bbb ccc ddd eee", 11, "  ", "      ");
    assert_eq!(
        rows,
        vec!["  aaa bbb", "      ccc", "      ddd", "      eee"]
    );
}

#[test]
fn a_hanging_wrap_never_lays_a_continuation_wider_than_it_has() {
    // The fault this helper exists for: a continuation laid out to the first row's room overflows
    // its own indent and clips at the border.
    let rows = wrap_hanging(
        "one two three four five six seven",
        20,
        "  ",
        "            ",
    );
    for row in &rows {
        assert!(
            row.chars().count() <= 20,
            "{row:?} is wider than 20: {rows:?}"
        );
    }
}

#[test]
fn a_hanging_wrap_does_not_repeat_a_marker_down_the_side() {
    // An opening indent can end in a character a trim would leave behind (`    ! `), which used
    // to print the marker on every continuation.
    let rows = wrap_hanging("alpha beta gamma delta epsilon zeta", 18, "  ! ", "    ");
    assert!(rows.len() > 1, "{rows:?}");
    for row in &rows[1..] {
        assert!(
            !row.contains('!'),
            "the marker is only on the first row: {rows:?}"
        );
        assert!(row.starts_with("    "), "{row:?}");
    }
}

#[test]
fn a_hanging_wrap_keeps_every_word_once_and_in_order() {
    let text = "the quick brown fox jumps over the lazy dog";
    let rows = wrap_hanging(text, 16, "  ", "     ");
    let rejoined: Vec<&str> = rows
        .iter()
        .flat_map(|r| r.split_whitespace())
        .filter(|w| *w != "!")
        .collect();
    assert_eq!(rejoined.join(" "), text, "{rows:?}");
}

#[test]
fn a_hanging_wrap_of_a_short_text_is_one_row() {
    assert_eq!(wrap_hanging("short", 40, "  ", "    "), vec!["  short"]);
}

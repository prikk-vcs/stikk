//! Tests for [`super::wrap_spans`] (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::wrap_spans;

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

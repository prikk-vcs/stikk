//! Tests for the History view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::HistoryView;
use stikk_prikk::BlockRow;

use super::*;
use crate::test_util::buffer_text;

fn row(id: &str, seq: u64, kind: &str) -> BlockRow {
    BlockRow {
        block_id: id.to_string(),
        ref_state_id: format!("rs-{id}"),
        update_seq: seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents: if seq > 1 { 1 } else { 0 },
        patches: 1,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: if seq > 1 { Some("prev".into()) } else { None },
    }
}

/// **80 columns, the width the handoffs name** (RFC 036 review v2's note 1). It was 90 — wider than
/// the terminal every capture in this project is read at, which is how the queued tier's clip went
/// unnoticed here while the same clip in Changes was caught.
fn draw(view: &HistoryView, cursor: usize) -> String {
    draw_at(view, cursor, 80, 20)
}

fn draw_at(view: &HistoryView, cursor: usize, width: u16, height: u16) -> String {
    draw_marked(view, cursor, None, width, height)
}

fn draw_marked(
    view: &HistoryView,
    cursor: usize,
    marked: Option<&str>,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(view, cursor, marked, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn renders_queue_tier_and_blocks_newest_first() {
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 2,
        queued_target: None,
        blocks: vec![row("bbbb0000", 2, "Normal"), row("aaaa0000", 1, "Root")],
    };
    let text = draw(&view, 0);
    assert!(text.contains("queued"));
    assert!(text.contains("2 patch(es)"));
    assert!(text.contains("Normal"));
    assert!(text.contains("Root"));
    assert!(text.contains("tip")); // only the first (newest) block is the tip
}

#[test]
fn empty_ref_says_so() {
    let view = HistoryView {
        reff: "heads/topic".into(),
        queued: 0,
        queued_target: None,
        blocks: Vec::new(),
    };
    let text = draw(&view, 0);
    assert!(text.contains("no sealed blocks"));
}

#[test]
fn hostile_ref_name_and_kind_are_rendered_inert() {
    // Both the ref (in the title) and a block kind reach cells only through inert() (C-T2a).
    let view = HistoryView {
        reff: "heads/\u{1b}[31mmain".into(),
        queued: 0,
        queued_target: None,
        blocks: vec![row("cccc0000", 1, "R\u{7}oot")],
    };
    let text = draw(&view, 0);
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{7}'));
    assert!(
        text.contains("\\u001b"),
        "RFC 036: the ESC is written as its escape, not as U+FFFD — {text:?}"
    );
}

/// **RFC 036: a ref name in a title is an identifier.** §1's measurement found prikk 0.51 accepts a
/// branch name carrying a bidi override, so this is a name a real repository can hold.
#[test]
fn a_ref_name_with_an_override_is_quoted_in_the_title() {
    let view = HistoryView {
        reff: "heads/ev\u{202e}il".to_string(),
        queued: 0,
        queued_target: None,
        blocks: vec![row("b1", 1, "commit")],
    };
    let text = draw(&view, 0);
    println!("--- RFC 036: a branch name prikk accepts, in History's title\n{text}");
    assert!(
        !text.contains('\u{202e}'),
        "no override may reach a cell: {text:?}"
    );
    assert!(
        text.lines().any(|row| row.contains("u202eil")),
        "the override is written out, in the title: {text}"
    );
    assert!(
        text.lines().any(|row| row.contains("History")),
        "and the title is still there: {text}"
    );
}

/// **RFC 036 review v1 §2.3: the queued tier names its target ref as an identifier.** Core returns
/// the sentence as pieces, so a ref with a double space in it keeps both inside the sentence.
#[test]
fn the_queued_tiers_target_ref_is_an_identifier() {
    let view = HistoryView {
        reff: "heads/other".to_string(),
        queued: 2,
        queued_target: Some("heads/ev\u{202e}il".to_string()),
        blocks: vec![row("b1", 1, "commit")],
    };
    let text = draw(&view, 0);
    println!("--- RFC 036 §2.3: the queued tier's target ref\n{text}");
    assert!(!text.contains('\u{202e}'), "{text:?}");
    assert!(
        text.lines().any(|r| r.contains("u202eil")),
        "the override is written out inside core's sentence: {text}"
    );
    assert!(
        text.lines()
            .any(|r| r.contains("the active WAL holds 2 patch(es) for")),
        "and core's words are unchanged: {text}"
    );
}

/// **The queued tier wraps; it never clips** (RFC 024, carried into RFC 037 Handoff A §5 from
/// RFC 036's review). The tier was one unwrapped `Line`, so at 80 columns the sentence was cut at
/// the border mid-word — *"…not this ref's hist"* — and a long ref name in it was cut with it.
///
/// The assertion is the strong one the handoff asks for: **every character of a 60-character ref
/// name is on screen, once, in order.**
#[test]
fn the_queued_tier_wraps_and_a_long_ref_name_survives_whole() {
    // 60 characters, and a valid ref name. Spelled out rather than generated, so the assertion
    // below is comparing against something a reader can see.
    let long_ref = "heads/a-very-long-branch-name-that-does-not-fit-on-one-row-x".to_string();
    assert_eq!(long_ref.chars().count(), 60, "{long_ref}");

    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 7,
        queued_target: Some(long_ref.clone()),
        blocks: vec![row("bbbb0000", 2, "Normal")],
    };
    let text = draw(&view, 0);

    // Nothing is cut at the border: the sentence's own last words are present.
    assert!(
        text.contains("Q: Queue"),
        "the tier's tail is on screen: {text}"
    );

    // The name, whole and in order, across however many rows it took. The panel's side borders and
    // the row padding come out first; what is left is the name's characters in the order drawn.
    let drawn: String = text
        .lines()
        .map(|line| line.trim_matches(|c| c == '\u{2502}' || c == ' '))
        .collect::<Vec<_>>()
        .join("");
    assert!(
        drawn.contains(&long_ref),
        "every character of the 60-character ref name, once and in order.\nname: {long_ref}\n{text}"
    );
}

/// The wrapped tier still leaves the cursor arithmetic exact: block rows begin below however many
/// rows the tier took, so a selected row is not scrolled to the wrong line (Handoff A §5).
#[test]
fn a_wrapped_tier_does_not_move_the_block_rows_out_from_under_the_cursor() {
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 7,
        queued_target: Some(format!("heads/{}", "x".repeat(54))),
        blocks: vec![
            row("bbbb0000", 3, "Normal"),
            row("aaaa0000", 2, "Normal"),
            row("9999dddd", 1, "Root"),
        ],
    };
    // A short panel, so the scroll is doing real work: the selected row must still be visible.
    let text = draw_at(&view, 2, 80, 10);
    assert!(
        text.contains("9999dddd"),
        "the selected row is on screen below the wrapped tier: {text}"
    );
}

/// **RFC 037 Handoff A §5's capture, at exactly 80 columns**: the queued tier wrapped, with a
/// 60-character ref name. Reproduce with
/// `cargo test -p stikk-tui --lib capture_ — --nocapture`.
#[test]
fn capture_rfc037_queued_tier_at_80_columns() {
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 7,
        queued_target: Some("heads/a-very-long-branch-name-that-does-not-fit-on-one-row-x".into()),
        blocks: vec![
            row("56e95d7b7657", 2, "Normal"),
            row("2b35728e3550", 1, "Root"),
        ],
    };
    let text = draw(&view, 0);
    println!("--- History's queued tier, wrapped (80 columns) ---\n{text}");
    assert!(text.contains("Q: Queue"));
}

/// A row carrying every marker at once: the widest a block row gets.
fn maximal_row() -> BlockRow {
    let mut r = row("4db1669a176b1111", 123_456, "merge");
    r.rollback_block = true;
    r.required_attestations = 3;
    r.patches = 9;
    r
}

/// Each rendered row's text with the borders stripped, so a column can be asserted on.
fn inner_rows(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix(BORDER))
        .filter_map(|l| l.strip_suffix(BORDER))
        .map(str::to_string)
        .collect()
}

const BORDER: char = '│';

#[test]
fn a_block_row_too_long_for_the_terminal_wraps_under_the_short_id() {
    // **Measured at 80 columns** (RFC 037 Handoff C §1). This row is 85 columns, so it used to be
    // cut at the border: `needs 3 attestation(` and no tip marker at all, which is a wrong picture
    // of the lineage, not just a narrow one (RFC 024).
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        queued_target: None,
        blocks: vec![maximal_row()],
    };
    let text = draw(&view, 0);
    println!("--- RFC 037 Handoff C §1: a maximal History row at 80 columns\n{text}");
    let rows = inner_rows(&text);

    // Nothing is cut: the tip marker, the last field on the row, is on screen.
    assert!(text.contains("← tip"), "{text}");

    // The row wrapped, and its continuation starts at the short id's own column — four columns in,
    // past the cursor and mark columns — not at the left edge.
    let first = rows
        .iter()
        .position(|r| r.contains("4db1669a176b"))
        .expect("the block row is on screen");
    let continuation = &rows[first + 1];
    assert!(
        continuation.starts_with("    ") && !continuation.starts_with("     "),
        "continuation must start at column 4, got {continuation:?}"
    );

    // Every character of the row is present once and in order: joining the rows back up, with each
    // continuation's indent removed, gives exactly the fields in their order.
    let rejoined = format!(
        "{}{}",
        rows[first].trim_end(),
        continuation.trim_end().strip_prefix("    ").unwrap()
    );
    assert_eq!(
        rejoined,
        "▶   4db1669a176b  #123456merge   9 patch(es)  [rollback]  needs 3 attestation(s)  ← tip"
    );
}

#[test]
fn the_cursor_still_selects_the_right_row_below_a_wrapped_one() {
    // **The scroll is no longer arithmetic on the cursor** (RFC 023 F2): a wrapped row occupies two
    // lines, so a count of rows above the cursor no longer gives its line. Three maximal rows all
    // wrap; the marker must sit on the one `cursor` names, and the row must be whole on screen.
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        queued_target: None,
        blocks: vec![
            row("aaaaaaaaaaaa1111", 3, "ordinary"),
            maximal_row(),
            row("cccccccccccc3333", 1, "ordinary"),
        ],
    };
    for (cursor, id) in [
        (0, "aaaaaaaaaaaa"),
        (1, "4db1669a176b"),
        (2, "cccccccccccc"),
    ] {
        let text = draw(&view, cursor);
        let marked: Vec<&str> = text.lines().filter(|l| l.contains("▶ ")).collect();
        assert_eq!(
            marked.len(),
            1,
            "exactly one row carries the cursor: {text}"
        );
        assert!(
            marked[0].contains(id),
            "cursor {cursor} must mark {id}, got {:?}",
            marked[0]
        );
    }
}

#[test]
fn a_selected_wrapped_row_is_whole_on_screen_when_it_is_the_last() {
    // The scroll pins the selected row's **last** line, so selecting a wrapped row at the bottom
    // shows both of its lines rather than scrolling its continuation off.
    let mut blocks: Vec<BlockRow> = (0..8)
        .map(|i| {
            row(
                &format!("{i}{i}{i}{i}{i}{i}{i}{i}{i}{i}{i}{i}"),
                8 - i,
                "ordinary",
            )
        })
        .collect();
    blocks.push(maximal_row());
    let last = blocks.len() - 1;
    let view = HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        queued_target: None,
        blocks,
    };
    // A short terminal, so the lineage cannot all fit and the view must scroll.
    let text = draw_at(&view, last, 80, 8);
    let rows = inner_rows(&text);
    let first = rows
        .iter()
        .position(|r| r.contains("4db1669a176b"))
        .expect("the selected row is on screen");
    // Its continuation is on screen too, below it, rather than scrolled past the bottom border.
    let continuation = rows
        .get(first + 1)
        .expect("the selected row's continuation is on screen");
    assert!(
        continuation.starts_with("    ") && !continuation.trim().is_empty(),
        "both lines of the selected row: {text}"
    );
}

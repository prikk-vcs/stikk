//! Tests for the History view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

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

fn draw(view: &HistoryView, cursor: usize) -> String {
    let backend = TestBackend::new(90, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(view, cursor, &Palette::default(), f, f.area()))
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

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
        blocks: vec![row("cccc0000", 1, "R\u{7}oot")],
    };
    let text = draw(&view, 0);
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{7}'));
    assert!(text.contains('\u{FFFD}'));
}

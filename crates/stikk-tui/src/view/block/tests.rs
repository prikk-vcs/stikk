//! Tests for the Block-detail view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::BlockDetailView;
use stikk_prikk::{BlockRow, StateFiles};

use super::*;
use crate::test_util::buffer_text;

fn row() -> BlockRow {
    BlockRow {
        block_id: "bbbbbbbbbbbbbbbb".into(),
        ref_state_id: "rs-bbbb".into(),
        update_seq: 2,
        kind: "Normal".into(),
        rollback_block: false,
        parents: 1,
        patches: 4,
        rollback_patches: 0,
        required_attestations: 0,
        previous_ref_state: Some("rs-aaaa".into()),
    }
}

fn draw(detail: &BlockDetailView) -> String {
    let backend = TestBackend::new(90, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(detail, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn tip_detail_lists_state_files() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: Some(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec!["readme.txt".into(), "src/lib.rs".into()],
            total_bytes: 200,
        }),
    };
    let text = draw(&detail);
    assert!(text.contains("tip")); // title marks the tip
    assert!(text.contains("readme.txt"));
    assert!(text.contains("src/lib.rs"));
    assert!(text.contains("2 file(s)"));
    assert!(text.contains("UD-09")); // the honest per-patch ceiling note
}

#[test]
fn non_tip_detail_explains_the_missing_state() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(text.contains("replays only to the ref tip"));
    assert!(!text.contains("readme")); // no file set for an older block
}

#[test]
fn hostile_file_path_is_rendered_inert() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: Some(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec!["evil\u{1b}[2Jfile.txt".into()],
            total_bytes: 10,
        }),
    };
    let text = draw(&detail);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

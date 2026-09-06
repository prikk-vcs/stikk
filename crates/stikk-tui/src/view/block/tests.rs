//! Tests for the Block-detail view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::BlockDetailView;
use stikk_prikk::{BlockRow, PatchMessage, StateFiles};

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
        messages: Vec::new(),
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
fn a_disagreeing_patch_count_and_message_list_is_explained_in_the_rendered_buffer() {
    // RFC 015 F4 / decision 2 — the acceptance-critical case: `row()` reports 4 patches and lists no
    // messages (a block sealed entirely below prikk 0.32). The buffer must say so, not merely list 0.
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(text.contains("4"));
    assert!(text.contains("0 with a message"));
    assert!(text.contains("before prikk 0.32 carry"));
}

#[test]
fn an_agreeing_patch_count_and_message_list_shows_the_bare_count() {
    let mut agreeing = row();
    agreeing.patches = 1;
    agreeing.messages = vec![PatchMessage {
        patch_id: "a".repeat(64),
        message: "a real message".to_string(),
    }];
    let detail = BlockDetailView {
        row: agreeing,
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(text.contains("a real message"));
    // No disagreement, so no explanation is fabricated for an honest match.
    assert!(!text.contains("carry none"));
}

#[test]
fn messages_never_render_without_the_count_beside_them() {
    // RFC 015 decision 2: the two must always travel together — this test just confirms the count
    // field is present on the same screen as the message list, not asserting a specific ordering.
    let mut with_messages = row();
    with_messages.patches = 2;
    with_messages.messages = vec![PatchMessage {
        patch_id: "b".repeat(64),
        message: "second message".to_string(),
    }];
    let detail = BlockDetailView {
        row: with_messages,
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(text.contains("patches"));
    assert!(text.contains("second message"));
}

#[test]
fn a_hostile_message_renders_inert_and_forges_no_field() {
    // C-T2a/C-T2b: a message is repository content — never a next-step, never chrome.
    let mut hostile = row();
    hostile.patches = 1;
    hostile.messages = vec![PatchMessage {
        patch_id: "c".repeat(64),
        message: "\u{1b}[2J kind: FORGED".to_string(),
    }];
    let detail = BlockDetailView {
        row: hostile,
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn a_message_with_a_colon_renders_intact() {
    let mut with_colon = row();
    with_colon.patches = 1;
    with_colon.messages = vec![PatchMessage {
        patch_id: "d".repeat(64),
        message: "first: with a colon".to_string(),
    }];
    let detail = BlockDetailView {
        row: with_colon,
        is_tip: false,
        state: None,
    };
    let text = draw(&detail);
    assert!(text.contains("first: with a colon"));
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

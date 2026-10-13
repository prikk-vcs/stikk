//! Tests for the Block-detail view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{BlockDetailView, BlockPatches};
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
        .draw(|f| render(detail, 0, &Palette::default(), f, f.area()))
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
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(text.contains("tip")); // title marks the tip
    assert!(text.contains("readme.txt"));
    assert!(text.contains("src/lib.rs"));
    assert!(text.contains("2 file(s)"));
    // RFC 035 §4.1: where the UD-09 ceiling note used to be, the Patches section now says — in
    // stikk's own voice, not prikk's — from which prikk a patch's content is read.
    assert!(
        text.contains("stikk reads a patch's content from prikk 0.39 on"),
        "{text}"
    );
    assert!(!text.contains("UD-09"), "the retired note is gone: {text}");
}

#[test]
fn non_tip_detail_explains_the_missing_state() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: None,
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
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
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

// ---------------------------------------------------------------------------------------------
// RFC 035 §4.1/§6 — the Patches section.
// ---------------------------------------------------------------------------------------------

const PATCH_ONE: &str = "4db1669a176b64136838975ef7e2724e2161ad78b89c6dabc4efa24cd6818f66";
const PATCH_TWO: &str = "c23a5d7d4d5d639b3e41e76806158e4bc475f113f3ed85858501010c2d830999";

fn draw_at(detail: &BlockDetailView, cursor: usize, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
    terminal
        .draw(|f| render(detail, cursor, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

fn shown(id: &str, operations: usize) -> stikk_prikk::ShownPatch {
    stikk_prikk::ShownPatch {
        patch_id: id.to_string(),
        queued: false,
        operations: (0..operations)
            .map(|_| stikk_prikk::PatchOperation {
                kind: "edit-text".to_string(),
                paths: vec![stikk_prikk::QueuedPath::Path("a.txt".to_string())],
                content: stikk_prikk::OperationContent::TextEdit {
                    old_span: "one".to_string(),
                    replacement: "two".to_string(),
                },
                raw: "{}".to_string(),
            })
            .collect(),
    }
}

/// **§6's capture: Block detail's Patches section at prikk 0.51, at 80 columns.** One row per patch in
/// prikk's order, with the message joined by id, the operation count, and the cursor on one of them.
#[test]
fn the_patches_section_lists_each_patch_with_its_joined_message() {
    let mut r = row();
    r.patches = 2;
    r.messages = vec![PatchMessage {
        patch_id: PATCH_TWO.to_string(),
        message: "patch two".to_string(),
    }];
    let detail = BlockDetailView {
        row: r,
        is_tip: false,
        state: None,
        patches: BlockPatches::Listed(vec![
            stikk_core::BlockPatchRow {
                patch: shown(PATCH_ONE, 3),
                message: stikk_core::PatchMessageState::NotReported,
            },
            stikk_core::BlockPatchRow {
                patch: shown(PATCH_TWO, 1),
                message: stikk_core::PatchMessageState::Text("patch two".to_string()),
            },
        ]),
    };
    let text = draw_at(&detail, 1, 80);
    println!("--- RFC 035 §6: Block detail's Patches section at 0.51, 80 columns\n{text}");
    assert!(
        text.contains("patches — 2 listed · Enter opens one:"),
        "{text}"
    );
    // The short id, the message in its own state, and the operation count — on one row each.
    assert!(
        text.contains("4db1669a176b  (no message reported)"),
        "{text}"
    );
    assert!(text.contains("c23a5d7d4d5d  patch two"), "{text}");
    assert!(
        text.contains("3 operation(s)") && text.contains("1 operation(s)"),
        "{text}"
    );
    // The cursor marks the second row, and only it.
    let marked: Vec<&str> = text.lines().filter(|line| line.contains('▸')).collect();
    let [marked] = marked.as_slice() else {
        panic!("exactly one row marked: {text}")
    };
    assert!(marked.contains("c23a5d7d4d5d"), "{text}");
    // The retired UD-09 ceiling note is gone, and nothing claims prikk reports nothing.
    assert!(!text.contains("UD-09"), "{text}");
}

/// **§6's other capture: the below-0.39 sentence**, in stikk's own voice — never *"prikk does not
/// report it"*, which would be false at 0.36–0.38 (`C-T4d`).
#[test]
fn below_the_json_floor_the_section_names_stikks_own_limit() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: None,
        patches: BlockPatches::Unreported(30),
    };
    let text = draw_at(&detail, 0, 80);
    println!("--- RFC 035 §6: Block detail's Patches section below prikk 0.39, 80 columns\n{text}");
    assert!(
        text.contains("stikk reads a patch's content from prikk 0.39 on"),
        "{text}"
    );
    assert!(!text.contains("prikk does not report"), "{text}");
    assert!(!text.contains('▸'), "nothing to select: {text}");
}

/// A patch message is repository content and reaches the screen inert (`C-T2a`).
#[test]
fn a_patch_rows_message_is_rendered_inert() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: None,
        patches: BlockPatches::Listed(vec![stikk_core::BlockPatchRow {
            patch: shown(PATCH_ONE, 1),
            message: stikk_core::PatchMessageState::Text("evil\u{1b}[2Jmessage".to_string()),
        }]),
    };
    let text = draw_at(&detail, 0, 80);
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert!(text.contains('\u{FFFD}'), "{text:?}");
}

/// **A failed `show` costs the Patches section and nothing else** (review v1 §2.3): the block's fields
/// and the tip's state files are still on screen, and the section says what failed, in prikk's own
/// words where the failure was prikk's.
#[test]
fn a_failed_patches_read_leaves_the_rest_of_the_block_on_screen() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: Some(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec!["readme.txt".into()],
            total_bytes: 12,
        }),
        patches: BlockPatches::Failed(
            "refusal: precondition not met: no object bbbb in the object store".to_string(),
        ),
    };
    let text = draw_at(&detail, 0, 80);
    println!(
        "--- RFC 035 review v1 §2.3: Block detail with a failed patches read, 80 columns\n{text}"
    );
    assert!(
        text.contains("bbbbbbbbbbbbbbbb"),
        "the block itself: {text}"
    );
    assert!(text.contains("readme.txt"), "the tip's state files: {text}");
    assert!(
        text.contains("could not read this block's patches"),
        "{text}"
    );
    assert!(text.contains("no object bbbb"), "prikk's own words: {text}");
    assert!(!text.contains('▸'), "nothing to select: {text}");
}

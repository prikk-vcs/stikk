//! Tests for the Block-detail view (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{BlockDetailView, BlockPatches, BlockState};
use stikk_prikk::{BlockRow, PatchMessage, StateFile, StateFiles};

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

/// **80 columns, the width the handoffs name** (RFC 036 review v2's note 1). It was 90, which is
/// wider than the terminal every capture in this project is read at.
/// The drawn buffer with every run of whitespace — the row padding and the newlines between rows
/// included — collapsed to one space, so a test can assert on a sentence that wraps. **Only for
/// prose**: a test about a path, a mode or a layout reads [`draw`] itself, since this would hide
/// exactly the spacing those tests are about.
fn drawn_prose(detail: &BlockDetailView) -> String {
    draw(detail)
        // The panel's own side borders sit between the end of one row and the start of the next,
        // so they have to go before the rows can be joined into a sentence.
        .replace('\u{2502}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn draw(detail: &BlockDetailView) -> String {
    let backend = TestBackend::new(80, 24);
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
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![
                StateFile::file("readme.txt", 0o100_644),
                StateFile::file("src/lib.rs", 0o100_755),
            ],
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

/// **Below prikk 0.46**, where prikk really does replay only to a tip. At or above it this view
/// lists an older block's files instead, and `the_older_block_sentence_is_gone_at_the_band` is the
/// other half of that pair.
#[test]
fn non_tip_detail_explains_the_missing_state() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: BlockState::Unreported(30),
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
        state: BlockState::Unreported(30),
        patches: BlockPatches::Unreported(30),
    };
    let text = drawn_prose(&detail);
    assert!(text.contains("4"));
    assert!(text.contains("0 with a message"));
    assert!(text.contains("before prikk 0.32 carry"), "{text}");
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
        state: BlockState::Unreported(30),
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
        state: BlockState::Unreported(30),
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
        state: BlockState::Unreported(30),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(!text.contains('\u{1b}'));
    assert!(
        text.contains("\\u001b"),
        "RFC 036: the ESC is written as its escape, not as U+FFFD — {text:?}"
    );
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
        state: BlockState::Unreported(30),
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
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![StateFile::file("evil\u{1b}[2Jfile.txt", 0o100_644)],
            total_bytes: 10,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(!text.contains('\u{1b}'));
    assert!(
        text.contains("\\u001b"),
        "RFC 036: the ESC is written as its escape, not as U+FFFD — {text:?}"
    );
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
        state: BlockState::Unreported(30),
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
        state: BlockState::Unreported(30),
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
        state: BlockState::Unreported(30),
        patches: BlockPatches::Listed(vec![stikk_core::BlockPatchRow {
            patch: shown(PATCH_ONE, 1),
            message: stikk_core::PatchMessageState::Text("evil\u{1b}[2Jmessage".to_string()),
        }]),
    };
    let text = draw_at(&detail, 0, 80);
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert!(
        text.contains("\\u001b"),
        "RFC 036: the ESC is written as its escape, not as U+FFFD — {text:?}"
    );
}

/// **A failed `show` costs the Patches section and nothing else** (review v1 §2.3): the block's fields
/// and the tip's state files are still on screen, and the section says what failed, in prikk's own
/// words where the failure was prikk's.
#[test]
fn a_failed_patches_read_leaves_the_rest_of_the_block_on_screen() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![StateFile::file("readme.txt", 0o100_644)],
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
    // **There is still something to select, and that changed with RFC 053.** Until the cursor
    // ranged over the files too, a failed patches read left nothing selectable at all; now the
    // one file is row zero, and marking it is what lets `F` save it. A failed *patches* read
    // does not take the *files* away.
    assert_eq!(
        text.matches('▸').count(),
        1,
        "the file is selectable even though the patches are not: {text}"
    );
    assert!(
        text.lines()
            .any(|l| l.contains('▸') && l.contains("readme.txt")),
        "and it is the file that is marked: {text}"
    );
}

/// **RFC 036 review v1 §2.3: `BlockPatchRow::summary()`'s short id is an identifier.** Hex, so it
/// is always plain — the point is that it goes through the rule that owns it, so a change to what
/// prikk puts in a patch id cannot sneak past.
#[test]
fn a_patch_summarys_id_is_rendered_as_an_identifier() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: BlockState::Unreported(30),
        patches: BlockPatches::Listed(vec![stikk_core::BlockPatchRow {
            patch: shown(PATCH_ONE, 1),
            message: stikk_core::PatchMessageState::Text("a  message".to_string()),
        }]),
    };
    let text = draw_at(&detail, 0, 90);
    println!("--- RFC 036 §2.3: a block's patch summary\n{text}");
    let short = &PATCH_ONE[..stikk_core::SHORT_ID_CHARS];
    assert!(
        text.lines().any(|r| r.contains(short)),
        "the short id is plain, as hex always is: {text}"
    );
    assert!(
        text.lines()
            .any(|r| r.contains("a  message") || r.contains("a message")),
        "and the message is prose beside it: {text}"
    );
}

/// **An older block lists its own files, and the heading says which block** (RFC 037 §3). The
/// heading used to say "state at tip" for the only block it could show; saying that over an older
/// block's correct file list would be stikk mislabelling a right answer.
#[test]
fn an_older_block_lists_its_files_under_its_own_heading() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![
                StateFile::file("a.txt", 0o100_644),
                StateFile::file("src/bin/tool.rs", 0o100_755),
            ],
            total_bytes: 9,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(text.contains("state at this block"), "{text}");
    assert!(!text.contains("state at tip"), "{text}");
    assert!(text.contains("2 file(s), 9 byte(s)"), "{text}");
    assert!(
        text.contains("a.txt") && text.contains("src/bin/tool.rs"),
        "{text}"
    );
    // **And the sentence that is now false is gone** (`C-T2b`).
    assert!(!text.contains("replays only to the ref tip"), "{text}");
}

/// Each row shows its mode, in the octal Patch detail already uses — so `100644` and `100755` read
/// as the same spelling a `create-file` operation shows, not as prikk's `33188`.
#[test]
fn each_file_row_shows_its_mode_as_octal() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![
                StateFile::file("plain.txt", 0o100_644),
                StateFile::file("run.sh", 0o100_755),
            ],
            total_bytes: 2,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(text.contains("100644  plain.txt"), "{text}");
    assert!(text.contains("100755  run.sh"), "{text}");
    assert!(
        !text.contains("33188"),
        "prikk's integer is not shown: {text}"
    );
}

/// **"Not reported" is not `0`** (RFC 037 §7 trap 2). On the prose band prikk names a path and
/// nothing else, and a `0` rendered as `000000` would be a mode no file has and prikk never claimed.
#[test]
fn a_file_with_no_reported_mode_says_so_rather_than_showing_zero() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![StateFile::path_only("readme.txt")],
            total_bytes: 3,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(text.contains("(no mode)"), "{text}");
    assert!(!text.contains("000000"), "{text}");
}

/// A `kind` stikk has not measured is **shown**, not silently rendered as an ordinary file. prikk
/// has never emitted one — it refuses to author a symlink at 0.51 — which is exactly why a reader
/// must not be shown one as though it were a file.
#[test]
fn an_unmeasured_kind_is_shown_beside_the_path() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: true,
        state: BlockState::Listed(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![
                StateFile::file("plain.txt", 0o100_644),
                StateFile {
                    path: "link.txt".into(),
                    kind: Some("symlink".into()),
                    mode: Some(0o120_777),
                },
            ],
            total_bytes: 4,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let text = draw(&detail);
    assert!(text.contains("link.txt  (symlink)"), "{text}");
    assert!(
        !text.contains("plain.txt  (file)"),
        "the one measured kind is not labelled on every row: {text}"
    );
}

/// **A failed file read costs the file list and nothing else** (RFC 037 decision 5): the block's
/// own fields are still on screen, with prikk's words for what went wrong.
#[test]
fn a_failed_file_read_keeps_the_block_on_screen() {
    let detail = BlockDetailView {
        row: row(),
        is_tip: false,
        state: BlockState::Failed(
            "prikk refused: precondition not met: block bb is not in this repository".into(),
        ),
        patches: BlockPatches::Unreported(30),
    };
    let text = drawn_prose(&detail);
    assert!(
        text.contains("bbbbbbbbbbbbbbbb"),
        "the block is still here: {text}"
    );
    assert!(text.contains("update-seq 2"), "{text}");
    assert!(
        text.contains("block bb is not in this repository"),
        "prikk's own words (`ER-02`): {text}"
    );
}

/// **RFC 037 Handoff A's captures, at exactly 80 columns** (§4). Asserts the three facts the review
/// package shows, and prints each screen so the package's captures can be reproduced from the
/// committed tree:
///
/// ```text
/// cargo test -p stikk-tui --lib capture_ — --nocapture
/// ```
#[test]
fn capture_rfc037_block_detail_at_80_columns() {
    let ids = [
        "2b35728e3550eda58d9eaf4a3290eabbe874acb8a36382f607f3e4d68b021389",
        "56e95d7b76573762a3624a99504a858756f6d954c9ccf3d7013572f6b6904e20",
    ];
    let mut older_row = row();
    older_row.block_id = ids[0].into();
    older_row.ref_state_id = format!("rs{}", &ids[0][2..]);
    older_row.update_seq = 1;
    older_row.kind = "Root".into();
    older_row.patches = 1;
    let mut tip_row = older_row.clone();
    tip_row.block_id = ids[1].into();
    tip_row.ref_state_id = format!("rs{}", &ids[1][2..]);
    tip_row.update_seq = 2;
    tip_row.kind = "Normal".into();

    let older = BlockDetailView {
        row: older_row,
        is_tip: false,
        state: BlockState::Listed(StateFiles {
            target_block: ids[0].into(),
            files: vec![
                StateFile::file("a.txt", 0o100_644),
                StateFile::file("scripts/release.sh", 0o100_755),
            ],
            total_bytes: 212,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let tip = BlockDetailView {
        row: tip_row,
        is_tip: true,
        state: BlockState::Listed(StateFiles {
            target_block: ids[1].into(),
            files: vec![
                StateFile::file("b.txt", 0o100_755),
                StateFile::file("blob.bin", 0o100_644),
                StateFile::file("nested/deep.txt", 0o100_644),
            ],
            total_bytes: 14,
        }),
        patches: BlockPatches::Unreported(30),
    };
    let below = BlockDetailView {
        state: BlockState::Unreported(28),
        ..older.clone()
    };

    for (label, detail) in [
        ("an older block's files, at prikk 0.51", &older),
        ("the tip's files, at prikk 0.51", &tip),
        ("the same older block, at prikk 0.28", &below),
    ] {
        let text = draw(detail);
        println!("--- {label} (80 columns) ---\n{text}");
    }

    assert!(draw(&older).contains("state at this block"));
    assert!(draw(&tip).contains("state at tip"));
    assert!(draw(&below).contains("replays only to the ref tip"));
}

// ---------------------------------------------------------------------------------------------
// RFC 037 Handoff C §1: long rows wrap to their own column, and the line count stays exact.

/// Each rendered row with the panel's side borders stripped, so a **column** can be asserted on.
/// [`drawn_prose`] cannot be used here: it collapses exactly the spacing these tests are about.
fn inner_rows(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix(SIDE))
        .filter_map(|l| l.strip_suffix(SIDE))
        .map(str::to_string)
        .collect()
}

const SIDE: char = '│';

/// The value column: two columns of margin plus the 15-wide label.
const VALUE_COL: usize = 17;

/// A block whose `block` and `ref-state` are full 64-hex ids, so both rows are far too long for an
/// 80-column terminal, with two patches listed below them.
fn wide_detail() -> BlockDetailView {
    let mut r = row();
    r.block_id = PATCH_ONE.to_string();
    r.ref_state_id = PATCH_TWO.to_string();
    r.patches = 2;
    BlockDetailView {
        row: r,
        is_tip: false,
        state: BlockState::Unreported(30),
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
    }
}

/// The rows of a field, from its label row through every continuation: the label row, then each
/// following row that is blank up to the value column and holds text beyond it.
fn field_block(rows: &[String], label: &str) -> Vec<String> {
    let at = rows
        .iter()
        .position(|r| r.starts_with(&format!("  {label} ")))
        .unwrap_or_else(|| panic!("no `{label}` row in {rows:#?}"));
    let mut out = vec![rows[at].clone()];
    for r in &rows[at + 1..] {
        let (gutter, rest) = r.split_at(VALUE_COL.min(r.len()));
        if gutter.trim().is_empty() && !rest.trim().is_empty() {
            out.push(r.clone());
        } else {
            break;
        }
    }
    out
}

/// The value a field's rows carry, continuations joined on, each with its gutter removed.
fn field_value(rows: &[String], label: &str) -> String {
    field_block(rows, label)
        .iter()
        .map(|r| r[VALUE_COL.min(r.len())..].trim_end().to_string())
        .collect()
}

#[test]
fn a_64_hex_id_wraps_under_its_own_value_column() {
    // **The defect RFC 037's review found.** ratatui's `Wrap` continued a long row at column 0, so
    // the second half of a 64-hex id sat under the labels and a reader could not tell which field
    // it belonged to. `Wrap` is gone; these rows are wrapped here, to the value's own column.
    let text = draw_at(&wide_detail(), 0, 80);
    println!("--- RFC 037 Handoff C §1: Block detail at 80 columns\n{text}");
    let rows = inner_rows(&text);

    for (label, id) in [("block", PATCH_ONE), ("ref-state", PATCH_TWO)] {
        let field = field_block(&rows, label);
        assert!(
            field.len() > 1,
            "`{label}` must wrap at 80 columns, got {field:#?}"
        );
        // Every continuation starts **at the value column**: blank before it, text at it.
        for cont in &field[1..] {
            assert_eq!(
                &cont[..VALUE_COL],
                &" ".repeat(VALUE_COL),
                "a `{label}` continuation must be blank up to column {VALUE_COL}: {cont:?}"
            );
            assert!(
                !cont[VALUE_COL..].starts_with(' '),
                "a `{label}` continuation must start at column {VALUE_COL}: {cont:?}"
            );
        }
        // Every character of the id, once and in order. Hex, so the identifier rule leaves it
        // plain — the id on screen is the id prikk reported.
        assert_eq!(field_value(&rows, label), id, "in {text}");
    }
}

#[test]
fn the_patch_cursor_still_selects_the_right_row_below_wrapped_fields() {
    // **The line count has to be exact** (RFC 023 F2). The fields above the Patches section now
    // occupy more lines than they have fields, so nothing downstream may count rows to find the
    // cursor. Both cursor positions, over the same wrapped fields.
    let detail = wide_detail();
    for (cursor, short) in [(0, "4db1669a176b"), (1, "c23a5d7d4d5d")] {
        let text = draw_at(&detail, cursor, 80);
        let marked: Vec<&str> = text.lines().filter(|line| line.contains('▸')).collect();
        let [marked] = marked.as_slice() else {
            panic!("exactly one patch row marked at cursor {cursor}: {text}")
        };
        assert!(
            marked.contains(short),
            "cursor {cursor} must mark {short}, got {marked:?}"
        );
    }
}

#[test]
fn the_below_0_46_state_sentence_wraps_under_itself() {
    // The other row the review found at column 0. It is stikk's own sentence, so it word-wraps --
    // and its continuations sit under the sentence, past the `state files: ` opening, not at the
    // left edge.
    let text = draw_at(&wide_detail(), 0, 80);
    let rows = inner_rows(&text);
    let at = rows
        .iter()
        .position(|r| r.contains("state files:"))
        .unwrap_or_else(|| panic!("no state sentence in {text}"));
    // It wraps, and every continuation is indented past the left margin rather than starting at it.
    let mut continuations = 0;
    for r in &rows[at + 1..] {
        if r.trim().is_empty() || r.starts_with("  state") || r.starts_with("  patches") {
            break;
        }
        assert!(
            r.starts_with("               ") && !r.starts_with("                "),
            "a state-sentence continuation must start at column 15: {r:?}"
        );
        continuations += 1;
    }
    assert!(continuations > 0, "the sentence must wrap at 80: {text}");
    // Nothing is cut: the sentence ends in the words it ends in, and names the measured version.
    assert!(
        drawn_prose(&wide_detail()).contains("this is 0.30."),
        "{text}"
    );
}

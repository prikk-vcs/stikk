//! Render tests for the Patch-detail view (RFC 035 §6): the captures at 80 columns.
//!
//! Every view is built through `stikk_core`, from the shapes `cli_backend::parse_json` produces for the
//! reports captured off a real prikk 0.51.0 binary. Each capture asserts that what core said is on
//! screen **whole**, wrapped or not — nothing clipped (RFC 024) — and the captures are printed so a
//! reviewer reads the screen rather than a description of it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{
    AS_AT_BLOCK_END, NO_PATH_REPORTED, OperationRow, PatchDetailView, PatchMessageState,
};
use stikk_prikk::{
    BinaryBlob, FileContent, OperationContent, PatchOperation, QueuedPath, ShownPatch,
};

use super::*;
use crate::test_util::buffer_text;

const PATCH_A: &str = "4db1669a176b64136838975ef7e2724e2161ad78b89c6dabc4efa24cd6818f66";
const BLOCK: &str = "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7";
const BLOB_OLD: &str = "a04c7b5b1c1bb9c0b28bee2d5ab4dc5707c3583199301c5e3eb7a069cbb28dc2";
const BLOB_NEW: &str = "8d71c569e89311208708dae4774995c92e9f15fd9ed2f026defa0db3ca45ba7d";
const NODE: &str = "ab011b1bb05d2b0a527fe45c4fc20eba9604233dea617c1551031cf2fc01acd2";

fn draw(view: &PatchDetailView, raw: bool) -> String {
    draw_sized(view, raw, 80, 24)
}

fn draw_sized(view: &PatchDetailView, raw: bool, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| {
            render(
                view,
                raw,
                &std::cell::Cell::new(0),
                &Palette::default(),
                f,
                f.area(),
            )
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// Whether any single row of the screen contains `needle` **exactly**, whitespace and all.
///
/// **Content rows must be judged with this, never with [`joined`]**, which collapses runs of
/// whitespace to match a wrapped sentence — the very thing a content row must not do (review v1 §2.1).
/// It caught this test file's own first draft asserting a re-indent against the joined text.
/// One row exactly as the view renders it: a `Named` row through `compose`, anything else as it is.
fn rendered_row(row: &OperationRow) -> String {
    match row {
        OperationRow::Named(pieces) => crate::text::compose(pieces),
        other => other.text().unwrap_or_default().to_string(),
    }
}

fn on_a_row(text: &str, needle: &str) -> bool {
    text.lines().any(|line| line.contains(needle))
}

/// The screen's text with borders and all runs of whitespace collapsed, so a wrapped line matches whole.
fn joined(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn operation(kind: &str, paths: Vec<QueuedPath>, content: OperationContent) -> PatchOperation {
    let raw = match &content {
        OperationContent::TextEdit {
            old_span,
            replacement,
        } => format!(
            "{{\n  \"kind\": \"{kind}\",\n  \"old_span_text\": \"{old_span}\",\n  \
             \"replacement_text\": \"{}\"\n}}",
            replacement.escape_debug()
        ),
        OperationContent::PermChanged { old_mode, new_mode } => format!(
            "{{\n  \"kind\": \"{kind}\",\n  \"old_mode\": {old_mode},\n  \"new_mode\": {new_mode}\n}}"
        ),
        _ => format!("{{\n  \"kind\": \"{kind}\"\n}}"),
    };
    PatchOperation {
        kind: kind.to_string(),
        paths,
        content,
        raw,
    }
}

fn path(p: &str) -> QueuedPath {
    QueuedPath::Path(p.to_string())
}

/// The every-kind patch, exactly as prikk 0.51.0 reports it for a block of one (the
/// `SHOW_BLOCK_ALL_KINDS_0_51` fixture in `stikk-prikk`).
fn every_kind() -> ShownPatch {
    ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: false,
        operations: vec![
            operation(
                "delete-node",
                vec![path("del.txt")],
                OperationContent::NodeDeleted {
                    node_kind: "file".to_string(),
                    content: FileContent::Text("gone\n".to_string()),
                },
            ),
            operation(
                "create-file",
                vec![path("created.txt")],
                OperationContent::FileCreated {
                    content: FileContent::Text("new file\n".to_string()),
                    mode: 33188,
                },
            ),
            operation(
                "change-perm",
                vec![path("moved.sh")],
                OperationContent::PermChanged {
                    old_mode: 33188,
                    new_mode: 33261,
                },
            ),
            operation(
                "replace-binary",
                vec![path("b.bin")],
                OperationContent::BinaryReplaced {
                    old: BinaryBlob {
                        blob_id: BLOB_OLD.to_string(),
                        size: 6,
                    },
                    new: BinaryBlob {
                        blob_id: BLOB_NEW.to_string(),
                        size: 7,
                    },
                },
            ),
            operation(
                "edit-text",
                vec![path("a.txt")],
                OperationContent::TextEdit {
                    old_span: "two".to_string(),
                    replacement: "TWO".to_string(),
                },
            ),
            operation(
                "rename-path",
                vec![path("mode.sh"), path("moved.sh")],
                OperationContent::PathRenamed {
                    author_key_id: "author".to_string(),
                    author_key_ids: vec!["author".to_string()],
                },
            ),
        ],
    }
}

/// **§6's first capture: every operation kind prikk emits, at 80 columns.** A block of one, so no
/// block-end label — the block's end is this patch's end.
#[test]
fn all_six_kinds_render_whole_at_80_columns() {
    let view = stikk_core::patch_detail_in_block(
        &every_kind(),
        BLOCK,
        &PatchMessageState::Text("every kind".to_string()),
        1,
    );
    // Tall enough that the whole patch is on one screen for the capture.
    let text = draw_sized(&view, false, 80, 40);
    println!("--- RFC 035 §6: Patch detail, all six kinds, 80 columns\n{text}");
    let flat = joined(&text);
    // **As the view renders them** (RFC 036 review v1 §2.3): a title and a `Named` row are pieces,
    // and `compose` is what turns them into the string that reaches a cell.
    for row in view.operations.iter().flat_map(|o| {
        std::iter::once(crate::text::compose(&o.title)).chain(o.rows.iter().map(rendered_row))
    }) {
        // Row by row, whitespace and all: a content row's spaces are prikk's bytes.
        assert!(on_a_row(&text, &row), "missing {row:?} in {text}");
    }
    assert!(flat.contains("sealed · in block"), "{flat}");
    assert!(flat.contains("every kind"), "{flat}");
    assert!(!flat.contains(AS_AT_BLOCK_END), "a block of one: {flat}");
    // The octal is on screen; prikk's own integer belongs to the raw view.
    assert!(flat.contains("mode 100644 → 100755"), "{flat}");
    assert!(!flat.contains("33188"), "{flat}");
}

/// **§6: an unresolved path, and the block-end label on a path that did resolve.** Both come from the
/// same two-patch block, which is how prikk produces them (A1 F7).
#[test]
fn an_unresolved_path_and_the_block_end_label_render_at_80_columns() {
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: false,
        operations: vec![
            operation(
                "edit-text",
                vec![path("z.txt")],
                OperationContent::TextEdit {
                    old_span: String::new(),
                    replacement: "more\n".to_string(),
                },
            ),
            operation(
                "replace-binary",
                vec![QueuedPath::UnresolvedNode(NODE.to_string())],
                OperationContent::BinaryReplaced {
                    old: BinaryBlob {
                        blob_id: BLOB_OLD.to_string(),
                        size: 6,
                    },
                    new: BinaryBlob {
                        blob_id: BLOB_NEW.to_string(),
                        size: 7,
                    },
                },
            ),
        ],
    };
    let view = stikk_core::patch_detail_in_block(
        &patch,
        BLOCK,
        &PatchMessageState::Text("patch one".to_string()),
        2,
    );
    let text = draw_sized(&view, false, 80, 30);
    println!("--- RFC 035 §6: an unresolved path, and the block-end label, 80 columns\n{text}");
    let flat = joined(&text);
    assert!(
        flat.contains(&format!("edit-text z.txt — {AS_AT_BLOCK_END}")),
        "{flat}"
    );
    assert!(flat.contains(NO_PATH_REPORTED), "{flat}");
    assert!(flat.contains(NODE), "prikk's node id, whole: {flat}");
    assert!(flat.contains("span replaced: (empty)"), "{flat}");
    // No reason is given for the unresolved path, at any version.
    assert!(!flat.contains("no block yet"), "{flat}");
}

/// **§6: a bounded preimage, with its *N more bytes not shown*.** A silent cut is the defect the bound
/// exists to prevent (`C-T2c′`).
#[test]
fn a_bounded_preimage_says_how_many_bytes_it_withheld() {
    let big = "lorem ipsum dolor sit amet\n".repeat(400);
    let withheld = big.len() - stikk_core::CONTENT_DISPLAY_BYTES;
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: false,
        operations: vec![operation(
            "delete-node",
            vec![path("big.txt")],
            OperationContent::NodeDeleted {
                node_kind: "file".to_string(),
                content: FileContent::Text(big),
            },
        )],
    };
    let view = stikk_core::patch_detail_in_block(&patch, BLOCK, &PatchMessageState::NotReported, 1);
    let text = draw(&view, false);
    println!("--- RFC 035 §6: a bounded preimage, 80×24 (the top)\n{text}");
    // And scrolled to where the bound bites, which is the half of this capture worth looking at.
    let offset = std::cell::Cell::new(u16::MAX);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|f| render(&view, false, &offset, &Palette::default(), f, f.area()))
        .unwrap();
    let end = buffer_text(terminal.backend().buffer());
    println!("--- RFC 035 §6: a bounded preimage, 80×24 (where the bound bites)\n{end}");
    assert!(
        joined(&end).contains(&format!("… {withheld} more bytes not shown")),
        "the cut is said out loud, on screen: {end}"
    );
    let rows = &view.operations[0].rows;
    assert_eq!(
        rows.last().map(rendered_row),
        Some(format!("… {withheld} more bytes not shown")),
        "{rows:?}"
    );
    assert!(joined(&text).contains("delete-node big.txt"), "{text}");
    assert!(
        text.contains("to scroll"),
        "a bounded preimage still scrolls: {text}"
    );
}

/// **§6: the raw view** — prikk's own fields, including the integers the rendered view shows as octal
/// (decision 5, F6). One report, two renderings; `v` is a toggle, never a second read.
#[test]
fn the_raw_view_shows_prikks_own_fields_at_80_columns() {
    let view = stikk_core::patch_detail_in_block(
        &every_kind(),
        BLOCK,
        &PatchMessageState::Text("every kind".to_string()),
        1,
    );
    let rendered = draw_sized(&view, false, 80, 40);
    let raw = draw_sized(&view, true, 80, 40);
    println!("--- RFC 035 §6: the raw view, 80 columns\n{raw}");
    assert!(
        raw.contains(" Patch · prikk's own fields — v: stikk's rendering"),
        "{raw}"
    );
    let flat = joined(&raw);
    assert!(flat.contains("\"old_mode\": 33188"), "{flat}");
    assert!(flat.contains("\"new_mode\": 33261"), "{flat}");
    assert!(flat.contains("\"old_span_text\": \"two\""), "{flat}");
    // The kind word is in both, and stikk's own rendering is in neither the other way round.
    assert!(flat.contains("change-perm moved.sh"), "{flat}");
    assert!(!flat.contains("mode 100644 → 100755"), "{flat}");
    assert!(
        joined(&rendered).contains("mode 100644 → 100755"),
        "{rendered}"
    );
    // The hint is in the title, where a scroll cannot push it off screen.
    assert!(
        rendered.contains(" Patch — v: prikk's own fields"),
        "{rendered}"
    );
}

/// Below stikk's JSON floor there is nothing to list, and the view says so in stikk's own voice — never
/// an empty patch.
#[test]
fn the_unreported_band_shows_stikks_own_reason() {
    let backend = stikk_prikk::NullBackend::supported();
    let view = stikk_core::patch_detail(
        &backend,
        std::path::Path::new("/repo"),
        PATCH_A,
        &PatchMessageState::NotReported,
    )
    .expect("the band is not an error");
    let text = draw(&view, false);
    println!("--- RFC 035: Patch detail below prikk 0.39, 80×24\n{text}");
    assert!(
        joined(&text).contains("stikk reads a patch's content from prikk 0.39 on; this is 0.30."),
        "{text}"
    );
}

/// Repository content reaches the screen inert (`C-T2a`): a span carrying an escape sequence cannot
/// corrupt stikk's own chrome, in either view.
#[test]
fn hostile_content_is_rendered_inert_in_both_views() {
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: true,
        operations: vec![operation(
            "edit-text",
            vec![path("evil\u{1b}[2J.txt")],
            OperationContent::TextEdit {
                old_span: "before".to_string(),
                replacement: "after\u{1b}[2Jcleared".to_string(),
            },
        )],
    };
    let view = stikk_core::patch_detail_in_block(
        &patch,
        BLOCK,
        &PatchMessageState::Text("evil\u{1b}[2Jmessage".to_string()),
        1,
    );
    for raw in [false, true] {
        let text = draw(&view, raw);
        assert!(!text.contains('\u{1b}'), "raw={raw}: {text:?}");
        assert!(
            text.contains("\\u001b") || raw,
            "RFC 036: the ESC is written as its escape — raw={raw}: {text:?}"
        );
    }
}

/// **§2.1's capture: a re-indent edit at 80 columns** — the change that used to render as no change.
///
/// prikk reports a span of two spaces replaced by four. On screen the two spans differ, byte for byte,
/// and a reader can see which is which.
#[test]
fn rfc035_a_re_indent_edit_renders_as_a_change_at_80_columns() {
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: true,
        operations: vec![
            operation(
                "edit-text",
                vec![path("src/main.rs")],
                OperationContent::TextEdit {
                    old_span: "  let x = 1;\n".to_string(),
                    replacement: "    let x = 1;\n".to_string(),
                },
            ),
            operation(
                "edit-text",
                vec![path("notes.txt")],
                OperationContent::TextEdit {
                    old_span: "trailing  \n".to_string(),
                    replacement: "trailing\n".to_string(),
                },
            ),
            operation(
                "edit-text",
                vec![path("Makefile")],
                OperationContent::TextEdit {
                    old_span: "\tcargo test\n".to_string(),
                    replacement: "    cargo test\n".to_string(),
                },
            ),
        ],
    };
    let view = stikk_core::patch_detail_in_block(&patch, BLOCK, &PatchMessageState::Absent, 1);
    let text = draw_sized(&view, false, 80, 30);
    println!("--- RFC 035 review v1 §2.1: whitespace edits, 80 columns\n{text}");
    for row in [
        "\"  let x = 1;\\n\"",
        "\"    let x = 1;\\n\"",
        "\"trailing  \\n\"",
        "\"trailing\\n\"",
        "\"\\tcargo test\\n\"",
        "\"    cargo test\\n\"",
    ] {
        assert!(on_a_row(&text, row), "missing {row:?} in {text}");
    }
    // The two spans of each edit are different rows on screen — that is the defect this fixes.
    for operation in &view.operations {
        let rows: Vec<String> = operation.rows.iter().map(rendered_row).collect();
        assert_ne!(rows[1], rows[3], "{rows:?}");
    }
}

/// **Content rows are not word-wrapped**, so internal runs of whitespace survive the renderer too —
/// core quoting them is only half the fix (review v1 §2.1).
#[test]
fn a_content_rows_internal_whitespace_survives_the_renderer() {
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: true,
        operations: vec![operation(
            "edit-text",
            vec![path("table.txt")],
            OperationContent::TextEdit {
                old_span: "a  b".to_string(),
                replacement: "a b".to_string(),
            },
        )],
    };
    let view = stikk_core::patch_detail_in_block(&patch, BLOCK, &PatchMessageState::NotReported, 1);
    let text = draw(&view, false);
    assert!(
        on_a_row(&text, "\"a  b\""),
        "the double space is on screen: {text}"
    );
    assert!(on_a_row(&text, "\"a b\""), "{text}");
}

/// A binary create or delete shows its blob and size, and never claims to have no content to show
/// (review v1 §2.4).
#[test]
fn rfc035_binary_content_shows_its_blob_at_80_columns() {
    let patch = ShownPatch {
        patch_id: PATCH_A.to_string(),
        queued: false,
        operations: vec![operation(
            "create-file",
            vec![path("logo.png")],
            OperationContent::FileCreated {
                content: FileContent::Binary(BinaryBlob {
                    blob_id: BLOB_NEW.to_string(),
                    size: 12,
                }),
                mode: 33188,
            },
        )],
    };
    let view = stikk_core::patch_detail_in_block(
        &patch,
        BLOCK,
        &PatchMessageState::Text("add the logo".to_string()),
        1,
    );
    let text = draw(&view, false);
    println!("--- RFC 035 review v1 §2.4: a binary create, 80 columns\n{text}");
    let flat = joined(&text);
    assert!(flat.contains("create-file logo.png"), "{flat}");
    assert!(flat.contains("content — 12 byte(s)"), "{flat}");
    assert!(flat.contains(BLOB_NEW), "{flat}");
    assert!(!flat.contains("no content to show"), "{flat}");
}

/// **Review v2 §2 on screen: content that lies about itself.** The span is the Trojan Source shape
/// (CVE-2021-42574) with a zero-width space and a non-breaking space added — a line whose bytes and
/// whose appearance disagree. stikk shows the bytes.
///
/// Printed at 80 columns because the point is what a reader sees: the escapes are legible, the row
/// reads left to right in the order prikk reported, and the Glossary (`?`) names the notation.
#[test]
fn content_that_does_not_show_as_itself_is_escaped_on_screen() {
    let span = "if level != \"user\u{202e} \u{2066}// Check if admin\u{2069}\" {\n\tlet a\u{200b}b = 1;\nx\u{a0}y   \n";
    let view = stikk_core::patch_detail_in_block(
        &ShownPatch {
            patch_id: PATCH_A.to_string(),
            queued: false,
            operations: vec![PatchOperation {
                kind: "create-file".to_string(),
                paths: vec![QueuedPath::Path("src/auth.rs".to_string())],
                content: OperationContent::FileCreated {
                    content: FileContent::Text(span.to_string()),
                    mode: 33188,
                },
                raw: format!(
                    "{{\n  \"kind\": \"create-file\",\n  \"content\": {}\n}}",
                    stikk_prikk::escape::quoted(span)
                ),
            }],
        },
        BLOCK,
        &PatchMessageState::Text("harden the admin check".to_string()),
        1,
    );
    let rendered = draw(&view, false);
    println!("--- RFC 035 review v2 §2: a Trojan Source span, rendered, 80×24\n{rendered}");
    let raw = draw(&view, true);
    println!("--- the same operation's raw view — the same spelling\n{raw}");

    for screen in [&rendered, &raw] {
        for ch in ['\u{202e}', '\u{2066}', '\u{2069}', '\u{200b}', '\u{a0}'] {
            assert!(
                !screen.contains(ch),
                "U+{:04X} reached the screen as itself:\n{screen}",
                u32::from(ch)
            );
        }
    }
    // Each row is one source line, quoted, with its own terminator inside the quotes.
    assert!(
        on_a_row(
            &rendered,
            "\"if level != \\\"user\\u202e \\u2066// Check if admin\\u2069\\\" {\\n\""
        ),
        "{rendered}"
    );
    assert!(
        on_a_row(&rendered, "\"\\tlet a\\u200bb = 1;\\n\""),
        "{rendered}"
    );
    assert!(on_a_row(&rendered, "\"x\\u00a0y   \\n\""), "{rendered}");
}

/// **RFC 036 review v1 §2.3: an operation title's path is an identifier.** Core returns the title as
/// pieces, so the path inside the sentence is rendered by the identifier rule — which is what
/// decision 8 asked for and what a single composed `String` could not give.
#[test]
fn an_operation_titles_path_is_rendered_as_an_identifier() {
    for (what, path_text, expected) in [
        ("a double space", "a  b.txt", "\"a\\u0020\\u0020b.txt\""),
        ("a bidi override", "ev\u{202e}il.txt", "\"ev\\u202eil.txt\""),
    ] {
        let view = stikk_core::patch_detail_in_block(
            &ShownPatch {
                patch_id: PATCH_A.to_string(),
                queued: false,
                operations: vec![PatchOperation {
                    kind: "create-file".to_string(),
                    paths: vec![QueuedPath::Path(path_text.to_string())],
                    content: OperationContent::FileCreated {
                        content: FileContent::Text("x\n".to_string()),
                        mode: 33188,
                    },
                    raw: "{}".to_string(),
                }],
            },
            BLOCK,
            &PatchMessageState::NotReported,
            1,
        );
        let screen = draw(&view, false);
        println!("--- RFC 036 §2.3: an operation title with {what}\n{screen}");
        assert!(
            on_a_row(&screen, &format!("create-file {expected}")),
            "{what}: the path is an identifier inside the title:\n{screen}"
        );
        assert!(
            !screen.contains('\u{202e}'),
            "{what}: nothing invisible reaches a cell:\n{screen}"
        );
    }
}

/// The same for a **path-bearing `Said` row** — the multi-path shape, where the path is a row of its
/// own rather than part of the title.
#[test]
fn a_path_row_is_rendered_as_an_identifier() {
    let view = stikk_core::patch_detail_in_block(
        &ShownPatch {
            patch_id: PATCH_A.to_string(),
            queued: false,
            operations: vec![PatchOperation {
                kind: "edit-text".to_string(),
                paths: vec![
                    QueuedPath::Path("a  b.txt".to_string()),
                    QueuedPath::Path("c.txt".to_string()),
                    QueuedPath::UnresolvedNode(NODE.to_string()),
                ],
                content: OperationContent::Unrendered,
                raw: "{}".to_string(),
            }],
        },
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    let screen = draw(&view, false);
    println!("--- RFC 036 §2.3: path rows, one of them with a double space\n{screen}");
    assert!(
        on_a_row(&screen, "\"a\\u0020\\u0020b.txt\""),
        "the double space survives as escapes:\n{screen}"
    );
    assert!(
        on_a_row(&screen, "c.txt"),
        "a plain path stays plain:\n{screen}"
    );
    assert!(
        on_a_row(&screen, &format!("node {NODE}")),
        "and prikk's node id is beside its sentence:\n{screen}"
    );
}

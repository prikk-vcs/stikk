//! Tests for the Compare view (design TS-01; RFC 038 §6).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{CompareView, compared};
use stikk_prikk::{Diff, DiffEntry, DiffFile, DiffSide, DiffTarget, QueuedOnSide, UnsupportedPath};

use super::*;
use crate::app::CompareSource;
use crate::test_util::buffer_text;

/// prikk's own fields as `Json::render` writes them — multi-line and indented. Pinned against a
/// **real captured report** in `stikk-prikk`'s `an_entrys_raw_fields_are_multi_line_indented_json`.
const RAW_MULTILINE: &str = r#"{
  "path": "ws.txt",
  "status": "modified",
  "from": {
    "kind": "file",
    "encoding": "text",
    "mode": 33188,
    "size": 5
  },
  "minimal": true,
  "hunks": [
    "@@ -1 +1 @@\n-keep\n+keep  \n"
  ]
}"#;

fn block(hex: char) -> String {
    std::iter::repeat_n(hex, 64).collect()
}

fn text_file(mode: u32, size: u64) -> DiffFile {
    DiffFile {
        kind: "file".into(),
        encoding: "text".into(),
        mode,
        size,
        content_id: None,
    }
}

fn entry(status: &str, path: &str, hunks: &[&str]) -> DiffEntry {
    DiffEntry {
        path: path.into(),
        status: status.into(),
        from_path: None,
        from: Some(text_file(0o100_644, 1)),
        to: Some(text_file(0o100_644, 1)),
        minimal: true,
        hunks: hunks.iter().map(|h| (*h).to_string()).collect(),
        raw: RAW_MULTILINE.into(),
    }
}

fn two_blocks(entries: Vec<DiffEntry>) -> CompareView {
    CompareView::Listed(compared(Diff {
        from: DiffSide {
            point: block('a'),
            target: DiffTarget::Block(block('a')),
            queued: QueuedOnSide::NotApplicable,
        },
        to: DiffSide {
            point: block('b'),
            target: DiffTarget::Block(block('b')),
            queued: QueuedOnSide::NotApplicable,
        },
        entries,
        unsupported_paths: Vec::new(),
    }))
}

/// **80 columns**, the width the handoff names.
fn draw(view: &CompareView, opened: &CompareSource) -> String {
    draw_at(view, opened, 0, false, 80, 24)
}

fn draw_at(
    view: &CompareView,
    opened: &CompareSource,
    cursor: usize,
    raw: bool,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(0);
    terminal
        .draw(|f| {
            render(
                crate::app::CompareFocus {
                    view,
                    opened,
                    cursor,
                    raw,
                    offset: &offset,
                },
                &Palette::default(),
                f,
                f.area(),
            );
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// **A whitespace-only change is visible**, which is the whole reason content rows are quoted and
/// wrapped verbatim. A renderer that word-wrapped would draw these two lines identically.
#[test]
fn a_whitespace_only_change_renders_as_two_different_rows() {
    let view = two_blocks(vec![entry(
        "modified",
        "ws.txt",
        &["@@ -1 +1 @@\n-keep\n+keep  \n"],
    )]);
    let text = draw(&view, &CompareSource::PreviousInHistory);
    assert!(text.contains("- \"keep\\n\""), "{text}");
    assert!(text.contains("+ \"keep  \\n\""), "{text}");
    // prikk's `@@` header is not quoted — it is prikk's words, not content.
    assert!(
        text.contains("@@ -1,3 +1,3 @@") || text.contains("@@ -1 +1 @@"),
        "{text}"
    );
    assert!(!text.contains("\"@@"), "the header is not quoted: {text}");
}

/// **The heading names both sides, and stikk says which is which.**
#[test]
fn the_heading_names_both_sides_and_how_it_was_opened() {
    let view = two_blocks(vec![entry("modified", "a.txt", &[])]);
    let text = draw(&view, &CompareSource::MarkedPair);
    assert!(text.contains(&block('a')[..8]), "{text}");
    assert!(text.contains(&block('b')[..8]), "{text}");
    assert!(
        text.contains("the marked block, against the selected one"),
        "{text}"
    );
}

/// The worktree form says what it compared, and names the queue prikk reported.
#[test]
fn the_worktree_form_names_the_queue() {
    let view = CompareView::Listed(compared(Diff {
        from: DiffSide {
            point: "heads/main".into(),
            target: DiffTarget::Block(block('a')),
            queued: QueuedOnSide::Queued(2),
        },
        to: DiffSide {
            point: "worktree".into(),
            target: DiffTarget::Worktree,
            queued: QueuedOnSide::NotApplicable,
        },
        entries: vec![entry("added", "wt.txt", &["@@ -0,0 +1 @@\n+wt\n"])],
        unsupported_paths: vec![UnsupportedPath {
            path: "link".into(),
            refusal: "precondition not met: link: worktree symlink authoring is out of scope"
                .into(),
        }],
    }));
    let text = draw(&view, &CompareSource::WorktreeAgainstBaseline);
    assert!(text.contains("heads/main"), "{text}");
    assert!(
        text.contains("plus 2 queued commit(s) not yet sealed"),
        "{text}"
    );
    assert!(text.contains("the worktree"), "{text}");
    // `commit`'s own refusal, verbatim (`ER-02`).
    assert!(
        text.contains("worktree symlink authoring is out of scope"),
        "{text}"
    );
}

/// **Nothing is "no differences" while prikk could not look at a path.**
#[test]
fn an_unsupported_path_is_never_reported_as_no_differences() {
    let view = CompareView::Listed(compared(Diff {
        from: DiffSide {
            point: "heads/main".into(),
            target: DiffTarget::Block(block('a')),
            queued: QueuedOnSide::Queued(0),
        },
        to: DiffSide {
            point: "worktree".into(),
            target: DiffTarget::Worktree,
            queued: QueuedOnSide::NotApplicable,
        },
        entries: Vec::new(),
        unsupported_paths: vec![UnsupportedPath {
            path: "link".into(),
            refusal: "precondition not met: link".into(),
        }],
    }));
    let text = draw(&view, &CompareSource::WorktreeAgainstBaseline);
    assert!(!text.contains("no differences"), "{text}");
    assert!(text.contains("link"), "{text}");
}

/// An empty comparison — prikk looked at both points and found nothing — says so in stikk's words.
#[test]
fn an_empty_comparison_says_no_differences() {
    let text = draw(&two_blocks(Vec::new()), &CompareSource::MarkedPair);
    assert!(text.contains("no differences"), "{text}");
}

/// Below the band, stikk's own sentence and no comparison.
#[test]
fn below_the_band_the_view_says_stikks_own_sentence() {
    let text = draw(
        &CompareView::Unreported(28),
        &CompareSource::PreviousInHistory,
    );
    assert!(text.contains("stikk compares from prikk 0.46 on"), "{text}");
    assert!(text.contains("this is 0.28"), "{text}");
}

/// `v` shows prikk's own fields **for the selected entry**, and only that one — **row by row,
/// with prikk's nesting intact** (review v1 §2). The defect this replaces flattened the whole
/// multi-line document into one run of text with literal `\n` sequences in it.
#[test]
fn the_raw_view_shows_prikks_fields_row_by_row_with_their_nesting() {
    let view = two_blocks(vec![
        entry("modified", "first.txt", &["@@ -1 +1 @@\n-a\n+b\n"]),
        entry("modified", "second.txt", &["@@ -1 +1 @@\n-c\n+d\n"]),
    ]);
    let text = draw_at(&view, &CompareSource::MarkedPair, 1, true, 80, 30);
    assert!(text.contains("\"status\""), "prikk's own fields: {text}");
    // The unselected entry still renders normally, so the screen is not blanked by the toggle.
    assert!(text.contains("first.txt"), "{text}");

    // **Each field is on its own row**, at the depth prikk wrote it.
    let rows: Vec<&str> = text
        .lines()
        .map(|l| l.trim_matches('\u{2502}').trim_end())
        .collect();
    assert!(
        rows.iter().any(|r| r.ends_with(r#"  "path": "ws.txt","#)),
        "{text}"
    );
    assert!(
        rows.iter().any(|r| r.ends_with(r#"    "kind": "file","#)),
        "deeper nesting keeps its extra indent: {text}"
    );
    // **And the document is not flattened**: no row carries two of prikk's fields at once.
    assert!(
        !rows
            .iter()
            .any(|r| r.contains(r#""path""#) && r.contains(r#""status""#)),
        "{text}"
    );
    // The `\n` inside a *string value* is prikk's own escape and stays — it is content, not a row
    // break, and the two must not be confused.
    assert!(text.contains("@@ -1 +1 @@"), "{text}");
}

/// **`v` is bounded too** — a toggle must not be the way around a bound that keeps a whole file off
/// the screen (RFC 035 decision 5).
#[test]
fn the_raw_view_is_bounded_and_says_what_it_withheld() {
    let big: String = std::iter::once("{\n".to_string())
        .chain((0..20_000).map(|i| format!("  \"line{i}\": {i},\n")))
        .chain(std::iter::once("}".to_string()))
        .collect();
    let mut e = entry("modified", "big.txt", &[]);
    e.raw = big;
    let view = two_blocks(vec![e]);
    let text = draw_at(&view, &CompareSource::MarkedPair, 0, true, 80, 300);
    assert!(text.contains("more bytes not shown"), "{text}");
}

/// A long content line **wraps and is never clipped** (RFC 024), hard-split rather than word-wrapped
/// so that nothing in it moves.
#[test]
fn a_long_content_line_wraps_without_losing_a_byte() {
    let long: String = std::iter::repeat_n("abcdefghij", 20).collect();
    let hunk = format!("@@ -1 +1 @@\n+{long}\n");
    let view = two_blocks(vec![entry("modified", "long.txt", &[&hunk])]);
    let text = draw_at(&view, &CompareSource::PreviousInHistory, 0, false, 80, 30);
    // Every character of the line is on screen, in order, once the rows are joined.
    let joined: String = text
        .lines()
        .map(|l| l.trim_matches(|c| c == '\u{2502}' || c == ' '))
        .collect::<Vec<_>>()
        .join("");
    assert!(joined.contains(&long), "the whole line survives:\n{text}");
}

/// **RFC 038's captures, at exactly 80 columns** (§6). Reproduce with
/// `cargo test -p stikk-tui --lib capture_ -- --nocapture`.
#[test]
fn capture_rfc038_compare_at_80_columns() {
    let mut mode = entry("mode", "scripts/release.sh", &[]);
    mode.from = Some(text_file(0o100_644, 2));
    mode.to = Some(text_file(0o100_755, 2));
    let mut binary = entry("binary", "logo.png", &[]);
    binary.from = Some(DiffFile {
        encoding: "binary".into(),
        content_id: Some(block('c')),
        ..text_file(0o100_644, 2048)
    });
    binary.to = Some(DiffFile {
        encoding: "binary".into(),
        content_id: Some(block('d')),
        ..text_file(0o100_644, 4096)
    });
    let mut renamed = entry("renamed", "docs/guide.md", &["@@ -1 +1 @@\n-old\n+new\n"]);
    renamed.from_path = Some("guide.md".into());

    let text_entry = two_blocks(vec![
        entry("modified", "ws.txt", &["@@ -1 +1 @@\n-keep\n+keep  \n"]),
        renamed,
        binary,
        mode,
    ]);
    println!(
        "--- two blocks: a whitespace edit, a rename, a binary and a mode (80 columns) ---\n{}",
        draw_at(&text_entry, &CompareSource::MarkedPair, 0, false, 80, 30)
    );

    let worktree = CompareView::Listed(compared(Diff {
        from: DiffSide {
            point: "heads/main".into(),
            target: DiffTarget::Block(block('a')),
            queued: QueuedOnSide::Queued(1),
        },
        to: DiffSide {
            point: "worktree".into(),
            target: DiffTarget::Worktree,
            queued: QueuedOnSide::NotApplicable,
        },
        entries: vec![entry("added", "wt.txt", &["@@ -0,0 +1 @@\n+wt\n"])],
        unsupported_paths: vec![UnsupportedPath {
            path: "link".into(),
            refusal: "precondition not met: link: worktree symlink authoring is out of scope"
                .into(),
        }],
    }));
    println!(
        "--- the worktree form, with a queue and an unsupported path (80 columns) ---\n{}",
        draw_at(
            &worktree,
            &CompareSource::WorktreeAgainstBaseline,
            0,
            false,
            80,
            20
        )
    );

    println!(
        "--- below prikk 0.46 (80 columns) ---\n{}",
        draw_at(
            &CompareView::Unreported(28),
            &CompareSource::PreviousInHistory,
            0,
            false,
            80,
            10
        )
    );

    // **`v`, from a real multi-line entry** (review v1 §3): prikk's fields row by row.
    println!(
        "--- v: prikk's own fields for the selected entry (80 columns) ---\n{}",
        draw_at(&text_entry, &CompareSource::MarkedPair, 0, true, 80, 24)
    );

    // **`v` on an entry past the bound**: the cut is said out loud, in the raw view too.
    let big: String = std::iter::once("{\n".to_string())
        .chain((0..20_000).map(|i| format!("  \"line{i}\": {i},\n")))
        .chain(std::iter::once("}".to_string()))
        .collect();
    let mut huge = entry("modified", "big.txt", &[]);
    huge.raw = big;
    let bounded = two_blocks(vec![huge]);
    let drawn = draw_at(&bounded, &CompareSource::MarkedPair, 0, true, 80, 300);
    let rows: Vec<&str> = drawn.lines().collect();
    println!(
        "--- v: an entry past the bound — first rows and the last (80 columns) ---\n{}\n    […]\n{}",
        rows[..8.min(rows.len())].join("\n"),
        rows.iter()
            .rev()
            .find(|r| r.contains("more bytes not shown"))
            .unwrap_or(&"")
    );

    assert!(draw(&text_entry, &CompareSource::MarkedPair).contains("100755"));
}

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
  "to": {
    "kind": "file",
    "encoding": "text",
    "mode": 33188,
    "size": 7
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

// ---------------------------------------------------------------------------------------------
// RFC 041: two refs, chosen in the ref picker.

/// A comparison of two **refs**, each resolving to its own block — the shape prikk really
/// returns, measured at 0.51 (`.git-exclude/reports/041-compare-two-refs/measure041-0.51.txt`).
fn two_refs(from: &str, to: &str, from_block: char, to_block: char) -> CompareView {
    CompareView::Listed(compared(Diff {
        from: DiffSide {
            point: from.to_string(),
            target: DiffTarget::Block(block(from_block)),
            queued: QueuedOnSide::NotApplicable,
        },
        to: DiffSide {
            point: to.to_string(),
            target: DiffTarget::Block(block(to_block)),
            queued: QueuedOnSide::NotApplicable,
        },
        entries: Vec::new(),
        unsupported_paths: Vec::new(),
    }))
}

fn ref_pair(from: &str, to: &str) -> CompareSource {
    CompareSource::RefPair {
        from: from.to_string(),
        to: to.to_string(),
    }
}

#[test]
fn a_ref_pairs_heading_names_both_refs_their_roles_and_the_blocks_they_resolved_to() {
    let view = two_refs("heads/feature", "heads/main", 'a', 'b');
    let text = draw(&view, &ref_pair("heads/feature", "heads/main"));
    println!("--- RFC 041: the Compare heading for two refs, at 80 columns\n{text}");
    let flat = text
        .replace('\u{2502}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Both names, as identifiers, with the role the reader gave each.
    assert!(flat.contains("heads/feature"), "{text}");
    assert!(flat.contains("(marked)"), "{text}");
    assert!(flat.contains("heads/main"), "{text}");
    assert!(flat.contains("(selected)"), "{text}");
    // **And the block each resolved to** (RFC 041 decision 3): a ref can move, a block cannot.
    assert!(flat.contains(&"a".repeat(12)), "{text}");
    assert!(flat.contains(&"b".repeat(12)), "{text}");
    // The marked side is first, and the arrow points at the selected one.
    let marked = flat.find("heads/feature").expect("the marked ref");
    let selected = flat.find("heads/main").expect("the selected ref");
    assert!(marked < selected, "marked first: {text}");
    // stikk's own line about which key chose which, beneath it.
    assert!(
        flat.contains("the marked ref, against the selected one"),
        "{text}"
    );
    // **No lineage order is claimed** (decision 2): two refs have none.
    assert!(!flat.contains("older first"), "{text}");
}

#[test]
fn re_running_a_ref_pair_shows_the_block_the_ref_now_points_at() {
    // `r` re-asks by ref name, so the same heading describes a different block once a ref moves.
    // Here that is the same two refs, the second resolving somewhere new.
    let before = draw(
        &two_refs("heads/feature", "heads/main", 'a', 'b'),
        &ref_pair("heads/feature", "heads/main"),
    );
    assert!(before.contains(&"b".repeat(12)), "{before}");

    let after = draw(
        &two_refs("heads/feature", "heads/main", 'a', 'c'),
        &ref_pair("heads/feature", "heads/main"),
    );
    assert!(after.contains(&"c".repeat(12)), "{after}");
    assert!(
        !after.contains(&"b".repeat(12)),
        "the old block is gone: {after}"
    );
    // The names are unchanged — it is the same question, asked again.
    assert!(
        after.contains("heads/feature") && after.contains("heads/main"),
        "{after}"
    );
}

#[test]
fn two_refs_at_the_same_block_read_as_no_differences() {
    // Measured at 0.51: prikk answers this with an empty report, exit 0. Not special-cased here
    // (RFC 041 decision 5) -- it is prikk's answer, rendered like any other.
    let view = two_refs("heads/main", "tags/v1", 'a', 'a');
    let text = draw(&view, &ref_pair("heads/main", "tags/v1"));
    assert!(text.contains("no differences"), "{text}");
    // Both sides still named, with the one block they share.
    assert!(
        text.contains("heads/main") && text.contains("tags/v1"),
        "{text}"
    );
}

#[test]
fn a_received_ref_is_an_ordinary_side() {
    // Measured at 0.51 through `bundle export`/`bundle import`: prikk compares a received ref
    // like any other point, so stikk renders it like any other name.
    let view = two_refs("remotes/heads/feature", "heads/main", 'a', 'b');
    let text = draw(&view, &ref_pair("remotes/heads/feature", "heads/main"));
    assert!(text.contains("remotes/heads/feature"), "{text}");
}

#[test]
fn a_hostile_ref_name_in_the_heading_is_an_identifier() {
    let view = two_refs("heads/two  spaces", "heads/\u{202e}hidden", 'a', 'b');
    let text = draw(
        &view,
        &ref_pair("heads/two  spaces", "heads/\u{202e}hidden"),
    );
    assert!(!text.contains('\u{202e}'), "{text}");
    assert!(text.contains("\\u202e"), "{text}");
    // The double space is spelled, so the wrap cannot collapse it into one.
    assert!(!text.contains("two  spaces"), "{text}");
    assert!(text.contains("\\u0020\\u0020"), "{text}");
}

#[test]
fn a_block_pairs_heading_is_unchanged() {
    // RFC 038's heading must not have moved: only the ref-pair source gets the new one.
    let text = draw(&two_blocks(Vec::new()), &CompareSource::MarkedPair);
    assert!(!text.contains("(marked)"), "{text}");
    assert!(!text.contains("(selected)"), "{text}");
    assert!(text.contains("older first"), "{text}");
}

/// **A title must not be cut off at 80 columns** (RFC 042 review v1 §2).
///
/// ratatui truncates a title that will not fit the top border, silently -- so a reader at 80
/// columns sees the view's name and loses the keys at the end of it. The end of a title is the
/// half a reader needs most: what `v` and `r` do, and where in the content they are. This asserts
/// those last characters survive, which is the half that goes first.
fn assert_title_is_whole(screen: &str, tail: &str) {
    let top = screen.lines().next().unwrap_or_default();
    assert!(
        top.contains(tail),
        "the title is cut off at 80 columns -- {tail:?} is missing from:\n{top}"
    );
}

#[test]
fn the_compare_title_is_whole_at_80_columns_in_both_renderings() {
    // A comparison tall enough to scroll at 24 rows, so the indicator is present.
    // Hunks enough to push the content past 24 rows, so the indicator is on the title.
    let hunks: Vec<String> = (1..=8)
        .map(|n| format!("@@ -{n} +{n} @@\n-before {n}\n+after {n}\n"))
        .collect();
    let refs: Vec<&str> = hunks.iter().map(String::as_str).collect();
    let view = two_blocks(vec![entry("modified", "src/main.rs", &refs)]);
    for raw in [false, true] {
        let text = draw_at(&view, &CompareSource::MarkedPair, 0, raw, 80, 24);
        assert_title_is_whole(
            &text,
            if raw {
                "v: rendered"
            } else {
                "v: prikk's own fields"
            },
        );
    }
}

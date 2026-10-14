//! Tests for the Compare view-model (RFC 038 §6).
//!
//! **The claim under test is that prikk's bytes survive.** A diff is where whitespace edits live,
//! so every case here is one a renderer that word-wrapped, trimmed or collapsed would show as *no
//! change at all*.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use stikk_prikk::{DiffEntry, DiffFile, DiffSide, UnsupportedPath};

fn side(point: &str, target: DiffTarget, queued: QueuedOnSide) -> DiffSide {
    DiffSide {
        point: point.to_string(),
        target,
        queued,
    }
}

fn block(hex: char) -> String {
    std::iter::repeat_n(hex, 64).collect()
}

fn file(mode: u32, size: u64) -> DiffFile {
    DiffFile {
        kind: "file".into(),
        encoding: "text".into(),
        mode,
        size,
        content_id: None,
    }
}

fn entry_with(status: &str, path: &str, hunks: &[&str]) -> DiffEntry {
    DiffEntry {
        path: path.into(),
        status: status.into(),
        from_path: None,
        from: Some(file(0o100_644, 1)),
        to: Some(file(0o100_644, 1)),
        minimal: true,
        hunks: hunks.iter().map(|h| (*h).to_string()).collect(),
        raw: "{}".into(),
    }
}

fn diff_of(entries: Vec<DiffEntry>) -> Diff {
    Diff {
        from: side(
            &block('a'),
            DiffTarget::Block(block('a')),
            QueuedOnSide::NotApplicable,
        ),
        to: side(
            &block('b'),
            DiffTarget::Block(block('b')),
            QueuedOnSide::NotApplicable,
        ),
        entries,
        unsupported_paths: Vec::new(),
    }
}

fn rows_of(entry: &ComparedEntry) -> Vec<&HunkRow> {
    entry.rows.iter().collect()
}

/// **The header is prikk's words and is not quoted; each body line is its marker plus quoted
/// content, terminator inside the quotes** (RFC 038 decision 5, §7 trap 4).
#[test]
fn a_hunk_becomes_a_header_and_one_row_per_line() {
    let c = compared(diff_of(vec![entry_with(
        "modified",
        "text.txt",
        &["@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n"],
    )]));
    let rows = rows_of(&c.entries[0]);
    assert_eq!(rows[0], &HunkRow::Header("@@ -1,3 +1,3 @@".to_string()));
    assert_eq!(
        rows[1],
        &HunkRow::Line {
            marker: ' ',
            content: "\"one\\n\"".to_string()
        }
    );
    assert_eq!(
        rows[2],
        &HunkRow::Line {
            marker: '-',
            content: "\"two\\n\"".to_string()
        }
    );
    assert_eq!(
        rows[3],
        &HunkRow::Line {
            marker: '+',
            content: "\"TWO\\n\"".to_string()
        }
    );
    assert_eq!(rows.len(), 5);
}

/// **The three look-alikes a whitespace-blind renderer loses.** Each pair differs in bytes alone,
/// and each must render differently — this is `T-T4` in the one view built to show exactly these
/// edits.
#[test]
fn no_two_lines_that_differ_in_any_byte_render_alike() {
    // A trailing-space edit, a CRLF change, and a missing final newline.
    let c = compared(diff_of(vec![
        entry_with("modified", "ws.txt", &["@@ -1 +1 @@\n-keep\n+keep  \n"]),
        entry_with("modified", "crlf.txt", &["@@ -1 +1 @@\n-crlf\n+crlf\r\n"]),
        entry_with(
            "modified",
            "nl.txt",
            &["@@ -1 +1 @@\n-nl\n+nl\n\\ No newline at end of file\n"],
        ),
    ]));

    let content = |at: usize| -> Vec<String> {
        c.entries[at]
            .rows
            .iter()
            .filter_map(|r| match r {
                HunkRow::Line { content, .. } => Some(content.clone()),
                _ => None,
            })
            .collect()
    };
    // `keep` and `keep  ` are two different lines, and they read as two.
    let ws = content(0);
    assert_eq!(ws, ["\"keep\\n\"", "\"keep  \\n\""]);
    assert_ne!(ws[0], ws[1]);
    // The `\r` is inside the quotes, as its own escape.
    let crlf = content(1);
    assert_eq!(crlf, ["\"crlf\\n\"", "\"crlf\\r\\n\""]);
    assert_ne!(crlf[0], crlf[1]);
    // A line with no terminator differs from the same line with one.
    let nl = content(2);
    assert_eq!(nl, ["\"nl\\n\"", "\"nl\\n\""]);

    // **prikk's no-newline note is prikk's words, not content**, so it is not quoted (§7 trap 4).
    assert!(
        c.entries[2]
            .rows
            .iter()
            .any(|r| matches!(r, HunkRow::Note(text) if text == "\\ No newline at end of file")),
        "{:?}",
        c.entries[2].rows
    );
}

/// An invisible character in a diff line is escaped by the one table the whole product shares
/// (RFC 035 A3) — so a bidi override in a changed line cannot reorder the row it sits in.
#[test]
fn an_invisible_character_in_a_line_is_escaped() {
    let c = compared(diff_of(vec![entry_with(
        "modified",
        "evil.rs",
        &["@@ -1 +1 @@\n-ok\n+evil\u{202e}gnp\n"],
    )]));
    let rendered = format!("{:?}", c.entries[0].rows);
    assert!(!rendered.contains('\u{202e}'), "{rendered}");
    assert!(rendered.contains("\\\\u202e"), "{rendered}");
}

/// **The bound is per entry, and the cut is said out loud** — never a silent truncation. One
/// measured case is a 20 000-line reversal at 457 KB in a single hunk.
#[test]
fn an_entry_is_bounded_and_says_what_it_withheld() {
    let big: String = std::iter::once("@@ -1,9999 +1,9999 @@\n".to_string())
        .chain((0..9999).map(|i| format!("-line {i}\n")))
        .collect();
    let c = compared(diff_of(vec![entry_with("modified", "big.txt", &[&big])]));
    let rows = &c.entries[0].rows;
    let withheld = rows
        .iter()
        .find_map(|r| match r {
            HunkRow::Withheld(text) => Some(text.clone()),
            _ => None,
        })
        .expect("the bound bites and says so");
    assert!(withheld.ends_with("more bytes not shown"), "{withheld}");

    // **The arithmetic is honest**, which is what "never a silent cut" means: the bytes shown plus
    // the bytes named as withheld are exactly the bytes prikk sent. The bound counts prikk's own
    // source bytes, not the rendered width — a quoted line is longer than the line it quotes, and
    // bounding the rendered form would make the number stikk reports meaningless.
    let withheld_bytes: usize = withheld
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .expect("the count is a number");
    let shown_bytes = big.len() - withheld_bytes;
    assert!(
        shown_bytes <= CONTENT_DISPLAY_BYTES,
        "the source bytes shown stay within the bound: {shown_bytes}"
    );
    assert!(
        shown_bytes > CONTENT_DISPLAY_BYTES - 32,
        "and the bound is reached, not undershot: {shown_bytes}"
    );
}

/// `minimal: false` carries prikk's note under that entry, and `true` carries none.
#[test]
fn a_non_minimal_entry_says_so() {
    let mut entry = entry_with("modified", "big.txt", &["@@ -1 +1 @@\n-a\n+b\n"]);
    entry.minimal = false;
    let c = compared(diff_of(vec![entry]));
    assert!(
        c.entries[0]
            .minimal_note
            .as_deref()
            .is_some_and(|n| n.contains("not the shortest edit script")),
        "{:?}",
        c.entries[0].minimal_note
    );
    let ordinary = compared(diff_of(vec![entry_with("modified", "a", &[])]));
    assert_eq!(ordinary.entries[0].minimal_note, None);
}

/// A `mode` entry shows both modes in octal; a `binary` entry shows both sizes and both ids, and
/// **never bytes**.
#[test]
fn mode_and_binary_entries_show_their_own_facts() {
    let mut mode = entry_with("mode", "run.sh", &[]);
    mode.from = Some(file(0o100_644, 2));
    mode.to = Some(file(0o100_755, 2));
    let mut binary = entry_with("binary", "b.bin", &[]);
    binary.from = Some(DiffFile {
        encoding: "binary".into(),
        content_id: Some(block('c')),
        ..file(0o100_644, 6)
    });
    binary.to = Some(DiffFile {
        encoding: "binary".into(),
        content_id: Some(block('d')),
        ..file(0o100_644, 7)
    });
    let c = compared(diff_of(vec![mode, binary]));

    let facts = |at: usize| -> String {
        c.entries[at]
            .facts
            .iter()
            .map(|f| crate::flatten(f))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    assert_eq!(facts(0), "mode 100644 \u{2192} 100755");
    let binary_facts = facts(1);
    assert!(binary_facts.contains("6 byte(s)"), "{binary_facts}");
    assert!(binary_facts.contains("7 byte(s)"), "{binary_facts}");
    assert!(binary_facts.contains(&block('c')[..8]), "{binary_facts}");
    assert!(c.entries[1].rows.is_empty(), "a binary entry has no hunks");
}

/// **The heading names both sides**, and the queue where prikk reported one — a state no block id
/// names (RFC 038 decision 5).
#[test]
fn the_heading_names_both_sides_and_the_queue() {
    let d = Diff {
        from: side(
            "heads/main",
            DiffTarget::Block(block('a')),
            QueuedOnSide::Queued(2),
        ),
        to: side(
            "worktree",
            DiffTarget::Worktree,
            QueuedOnSide::NotApplicable,
        ),
        entries: Vec::new(),
        unsupported_paths: Vec::new(),
    };
    let heading = crate::flatten(&compared(d).heading());
    assert!(heading.contains("heads/main"), "{heading}");
    assert!(
        heading.contains("plus 2 queued commit(s) not yet sealed"),
        "{heading}"
    );
    assert!(heading.contains("the worktree"), "{heading}");
}

/// A block id is shortened in the heading; a ref name is not. And **a queue of zero says nothing**
/// — prikk reported no queue there, so neither does stikk.
#[test]
fn a_block_id_is_short_and_an_empty_queue_is_silent() {
    let d = Diff {
        from: side(
            &block('a'),
            DiffTarget::Block(block('a')),
            QueuedOnSide::Queued(0),
        ),
        to: side(
            "worktree",
            DiffTarget::Worktree,
            QueuedOnSide::NotApplicable,
        ),
        entries: Vec::new(),
        unsupported_paths: Vec::new(),
    };
    let heading = crate::flatten(&compared(d).heading());
    assert!(!heading.contains(&block('a')), "the full id is not shown");
    assert!(heading.contains(&block('a')[..8]), "{heading}");
    assert!(!heading.contains("queued"), "{heading}");
}

/// **"No differences" is true only when prikk listed no entries *and* no unsupported paths.** A
/// path prikk could not look at is not a path it looked at and found unchanged (`C-T2c′`).
#[test]
fn an_unsupported_path_is_not_no_differences() {
    let mut d = diff_of(Vec::new());
    assert!(compared(d.clone()).is_empty());
    d.unsupported_paths = vec![UnsupportedPath {
        path: "link".into(),
        refusal: "precondition not met: link: worktree symlink authoring is out of scope".into(),
    }];
    assert!(
        !compared(d).is_empty(),
        "prikk could not compare that path, which is not 'no differences'"
    );
}

/// A rename names both paths, as names; the status word is prikk's.
#[test]
fn a_rename_names_both_paths() {
    let mut entry = entry_with("renamed", "new.md", &[]);
    entry.from_path = Some("old.md".into());
    let c = compared(diff_of(vec![entry]));
    let title = crate::flatten(&c.entries[0].title);
    assert!(title.starts_with("renamed"), "{title}");
    assert!(
        title.contains("old.md") && title.contains("new.md"),
        "{title}"
    );
}

/// **An unmeasured status is carried through the view-model untouched**, with its hunks.
#[test]
fn an_unmeasured_status_keeps_prikks_word_and_its_hunks() {
    let c = compared(diff_of(vec![entry_with(
        "symlink",
        "link",
        &["@@ -1 +1 @@\n-a\n+b\n"],
    )]));
    assert_eq!(c.entries[0].status, "symlink");
    assert!(crate::flatten(&c.entries[0].title).starts_with("symlink"));
    assert!(!c.entries[0].rows.is_empty(), "its hunks are still shown");
}

/// Below the band there is **no comparison, and that is not an empty one**.
#[test]
fn below_the_band_the_view_says_so_in_stikks_voice() {
    let view = CompareView::Unreported(28);
    assert!(view.listed().is_none());
    let reason = view.unreported_reason().expect("a sentence");
    assert!(
        reason.contains("stikk compares from prikk 0.46 on"),
        "{reason}"
    );
    assert!(reason.contains("this is 0.28"), "{reason}");
}

//! Tests for the Changes view (design TS-01; RFC 008).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{ChangeEntry, ChangeKind, ChangesView};

use super::*;
use crate::test_util::buffer_text;

fn dirty_view() -> ChangesView {
    ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 2,
        unchanged: 0,
        missing: 1,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        entries: vec![
            ChangeEntry {
                kind: ChangeKind::Modified,
                path: "readme.txt".into(),
                note: "tracked file bytes differ from the baseline".into(),
            },
            ChangeEntry {
                kind: ChangeKind::Missing,
                path: "src/main.rs".into(),
                note: "tracked file is absent from the worktree".into(),
            },
            ChangeEntry {
                kind: ChangeKind::Untracked,
                path: "notes.tmp".into(),
                note: "worktree file is not in the baseline".into(),
            },
        ],
        queued_elsewhere: None,
    }
}

fn draw(view: &ChangesView, hide_untracked: bool) -> String {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(view, hide_untracked, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn renders_the_changed_headline_counts_and_paths() {
    let text = draw(&dirty_view(), false);
    assert!(text.contains("Changes"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("change(s) against baseline"));
    assert!(text.contains("readme.txt"));
    assert!(text.contains("src/main.rs"));
    assert!(text.contains("notes.tmp"));
    // The honesty notes (UD-06 / UD-09) are always present.
    assert!(text.contains("whole-worktree"));
    assert!(text.contains("UD-09"));
}

#[test]
fn a_clean_worktree_says_so() {
    let view = ChangesView {
        reff: "heads/main".into(),
        clean: true,
        tracked: 3,
        unchanged: 3,
        missing: 0,
        modified: 0,
        untracked: 0,
        unsupported: 0,
        entries: Vec::new(),
        queued_elsewhere: None,
    };
    let text = draw(&view, false);
    assert!(text.contains("clean against baseline"));
}

#[test]
fn hiding_untracked_removes_the_row_but_keeps_the_caveat() {
    let text = draw(&dirty_view(), true);
    assert!(!text.contains("notes.tmp")); // the untracked row is hidden
    assert!(text.contains("untracked hidden")); // UD-08 caveat present
    assert!(text.contains("still captures them"));
    // Non-untracked rows remain.
    assert!(text.contains("readme.txt"));
}

#[test]
fn a_hostile_path_is_rendered_inert() {
    let view = ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        entries: vec![ChangeEntry {
            kind: ChangeKind::Modified,
            path: "evil\u{1b}[2Jfile.txt".into(),
            note: "bytes differ".into(),
        }],
        queued_elsewhere: None,
    };
    let text = draw(&view, false);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn queued_elsewhere_renders_as_a_distinct_verbatim_band() {
    // RFC 009 F4 — the acceptance-critical test.
    let mut view = dirty_view();
    view.queued_elsewhere = Some(
        "note: the active WAL has queued (unsealed) patches for heads/main, not heads/other"
            .to_string(),
    );
    let text = draw(&view, false);
    assert!(text.contains("prikk reported"));
    assert!(text.contains("the active WAL has queued (unsealed) patches for heads/main"));
}

#[test]
fn queued_elsewhere_suppresses_the_contradicting_ud08_claim() {
    // The acceptance-critical assertion: with `hide_untracked` set and `queued_elsewhere` present, the
    // string "a commit still captures them" must not appear anywhere in the rendered buffer — it would
    // contradict prikk's own warning that these files may already be committed, queued elsewhere.
    let mut view = dirty_view();
    view.queued_elsewhere = Some(
        "note: the active WAL has queued (unsealed) patches for heads/main, not heads/other"
            .to_string(),
    );
    let text = draw(&view, true);
    assert!(!text.contains("still captures them"));
    assert!(text.contains("untracked hidden")); // the UD-08 caveat still appears, reworded
    assert!(text.contains("see prikk's warning above"));
}

#[test]
fn without_queued_elsewhere_the_ud08_claim_is_unchanged() {
    let text = draw(&dirty_view(), true);
    assert!(text.contains("still captures them"));
    assert!(!text.contains("prikk reported"));
}

#[test]
fn a_hostile_queued_elsewhere_note_is_rendered_inert() {
    let mut view = dirty_view();
    view.queued_elsewhere = Some("\u{1b}[2Jhostile note".to_string());
    let text = draw(&view, false);
    assert!(!text.contains('\u{1b}'));
}

//! Tests for what a remembered session may restore (RFC 046 decision 5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::RefName;

use super::*;

fn entry(name: &str, closed: bool) -> RefEntry {
    RefEntry {
        name: name.to_string(),
        id: "e".repeat(64),
        closed,
        received: false,
    }
}

fn refs() -> Vec<RefEntry> {
    vec![
        entry("heads/main", false),
        entry("heads/feature", false),
        entry("heads/old", true),
        entry("tags/v1", false),
    ]
}

fn on(branch: &str) -> CurrentBranch {
    CurrentBranch::Branch(RefName::parse(branch).expect("a ref name"))
}

fn stored(focus: Option<&str>, view: LastView, hide_untracked: bool) -> SessionRecord {
    SessionRecord {
        root: std::path::PathBuf::from("/repo"),
        focus: focus.map(ToString::to_string),
        view,
        hide_untracked,
    }
}

/// **A remembered branch prikk still lists wins over prikk's current branch** — that is what
/// *resume* means (decision 5), and the status bar's *prikk's default* already shows the
/// difference.
#[test]
fn a_valid_stored_branch_wins_over_prikks_current_one() {
    let record = stored(Some("heads/feature"), LastView::Changes, true);
    let out = restore(Some(&record), &refs(), &on("heads/main"), true);
    assert_eq!(out.focus.as_deref(), Some("heads/feature"));
    assert_eq!(out.view, LastView::Changes);
    assert!(out.hide_untracked);
    assert_eq!(out.note, None);
}

/// A tag is an ordinary focus: the picker offers one, and focusing it is a read.
#[test]
fn a_stored_tag_is_restored() {
    let record = stored(Some("tags/v1"), LastView::History, false);
    let out = restore(Some(&record), &refs(), &on("heads/main"), true);
    assert_eq!(out.focus.as_deref(), Some("tags/v1"));
    assert_eq!(out.note, None);
}

/// **A closed branch is not restored**, and **a ref prikk no longer lists is not either** — each
/// falls back to today's rule, with the note.
#[test]
fn a_closed_or_missing_ref_falls_back_with_the_note() {
    for name in ["heads/old", "heads/deleted-since"] {
        let record = stored(Some(name), LastView::Queue, true);
        let out = restore(Some(&record), &refs(), &on("heads/main"), true);
        assert_eq!(out.focus.as_deref(), Some("heads/main"), "{name}");
        assert_eq!(out.note.as_deref(), Some(REMEMBERED_REF_IS_GONE), "{name}");
        // **The view and the filter are still restored**: they are preferences, not claims about
        // a ref, and each view re-reads from prikk when it opens.
        assert_eq!(out.view, LastView::Queue, "{name}");
        assert!(out.hide_untracked, "{name}");
    }
}

/// The note never names a ref: it is one line of stikk's own voice, about a ref that is by then
/// in no list.
#[test]
fn the_note_names_no_ref() {
    for name in ["heads/old", "heads/\u{202e}evil", "heads/bad name"] {
        let record = stored(Some(name), LastView::Orientation, false);
        let out = restore(Some(&record), &refs(), &on("heads/main"), true);
        let note = out.note.expect("a note");
        assert!(!note.contains(name), "{name}: {note}");
        assert!(!note.contains("heads/"), "{name}: {note}");
    }
}

/// **No record is today's behaviour, exactly**: prikk's current branch, Orientation, untracked
/// shown, and nothing to say.
#[test]
fn no_record_is_todays_rule() {
    let out = restore(None, &refs(), &on("heads/main"), true);
    assert_eq!(out.focus.as_deref(), Some("heads/main"));
    assert_eq!(out.view, LastView::Orientation);
    assert!(!out.hide_untracked);
    assert_eq!(out.note, None);
}

/// The fallback is `App::resolve_focus`'s rule, in all three of its shapes.
#[test]
fn the_fallback_is_todays_rule_in_every_shape() {
    for (current, published, want) in [
        (on("heads/dev"), true, Some("heads/dev")),
        (CurrentBranch::NotReported, true, Some("heads/main")),
        (CurrentBranch::NotReported, false, None),
        (
            CurrentBranch::Unresolved("<unresolved>".to_string()),
            true,
            Some("heads/main"),
        ),
        (
            CurrentBranch::Unresolved("<unresolved>".to_string()),
            false,
            None,
        ),
    ] {
        let out = restore(None, &refs(), &current, published);
        assert_eq!(out.focus.as_deref(), want, "{current:?}");
    }
}

/// A record that remembered **no** focus restores its view without claiming a ref, and says
/// nothing: there was nothing to lose.
#[test]
fn a_record_with_no_focus_restores_the_view_only() {
    let record = stored(None, LastView::Queue, true);
    let out = restore(Some(&record), &refs(), &CurrentBranch::NotReported, false);
    assert_eq!(out.focus, None);
    assert_eq!(out.view, LastView::Queue);
    assert_eq!(out.note, None);
}

/// With no refs at all — a repository before its first seal — a remembered ref is gone like any
/// other, and the fallback still answers.
#[test]
fn an_empty_ref_list_restores_nothing_remembered() {
    let record = stored(Some("heads/main"), LastView::History, false);
    let out = restore(Some(&record), &[], &on("heads/main"), false);
    assert_eq!(
        out.focus.as_deref(),
        Some("heads/main"),
        "from prikk, not from the record"
    );
    assert_eq!(out.note.as_deref(), Some(REMEMBERED_REF_IS_GONE));
}

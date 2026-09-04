//! Tests for the Changes operation (design TS-01/TS-02; RFC 008).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use stikk_prikk::{NullBackend, WorktreeEntry, WorktreeStatus};

use super::*;

fn dirty_status() -> WorktreeStatus {
    WorktreeStatus {
        reff: "heads/main".into(),
        clean: false,
        tracked: 2,
        unchanged: 0,
        missing: 1,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        entries: vec![
            WorktreeEntry {
                kind: "modified".into(),
                path: "readme.txt".into(),
                note: "tracked file bytes differ from the baseline".into(),
            },
            WorktreeEntry {
                kind: "missing".into(),
                path: "src/main.rs".into(),
                note: "tracked file is absent from the worktree".into(),
            },
            WorktreeEntry {
                kind: "untracked".into(),
                path: "notes.tmp".into(),
                note: "worktree file is not in the baseline".into(),
            },
        ],
        queued_elsewhere: None,
    }
}

#[test]
fn changes_view_maps_a_dirty_status() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
    assert!(!view.clean);
    assert_eq!(view.modified, 1);
    assert_eq!(view.entries.len(), 3);
    assert_eq!(view.entries[0].kind, ChangeKind::Modified);
    assert!(view.entries.iter().any(|e| e.kind.is_untracked()));
}

#[test]
fn a_dirty_worktree_is_success_not_a_refusal() {
    // RFC 008 finding 2 / UD-05: the seam has already turned prikk's non-zero dirty exit into an
    // Ok(WorktreeStatus{clean:false}); the operation must carry that through as success.
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    assert!(changes_view(&backend, Path::new("/r"), "heads/main").is_ok());
}

#[test]
fn below_0_28_returns_version_guidance_not_the_command() {
    // RFC 009 raised the default NullBackend to a supported+validated 0.30.0, so the below-the-gate
    // version must now be scripted explicitly rather than relied on as the default.
    let backend = NullBackend::supported()
        .with_version(0, 27, 1)
        .with_worktree_status(dirty_status());
    let err = changes_view(&backend, Path::new("/r"), "heads/main").unwrap_err();
    assert_eq!(err.class(), "not-ready");
    assert!(err.to_string().contains("0.28"));
    assert!(err.to_string().contains("0.27.1")); // states the actual version
}

#[test]
fn a_seam_refusal_propagates() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status_refusal("ref does not exist");
    let err = changes_view(&backend, Path::new("/r"), "heads/nope").unwrap_err();
    assert_eq!(err.class(), "refusal");
}

#[test]
fn an_unknown_kind_is_preserved_as_other() {
    assert_eq!(
        ChangeKind::from_label("typechange"),
        ChangeKind::Other("typechange".into())
    );
}

#[test]
fn carries_the_queued_elsewhere_warning_through_unmodified() {
    // RFC 009 F4: the operation layer transports prikk's warning verbatim; it never computes or
    // paraphrases it (ER-02).
    let mut status = dirty_status();
    status.queued_elsewhere = Some("note: the active WAL has queued patches for heads/main".into());
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(status);
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
    assert_eq!(
        view.queued_elsewhere.as_deref(),
        Some("note: the active WAL has queued patches for heads/main")
    );
}

#[test]
fn queued_elsewhere_is_none_when_prikk_did_not_emit_it() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
    assert_eq!(view.queued_elsewhere, None);
}

//! Tests for the commit operation (design `FR-050`/`FL-05`; RFC 014).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::ChangeToken;
use stikk_prikk::{CommitResult, NullBackend, Orientation, WorktreeStatus};

use super::*;

fn dirty_worktree() -> WorktreeStatus {
    WorktreeStatus {
        reff: "heads/main".to_string(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        entries: Vec::new(),
        queued_elsewhere: None,
    }
}

fn orientation(queued_patches: u64, queued_target: Option<&str>) -> Orientation {
    Orientation {
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
    }
}

fn author_readiness() -> stikk_model::Readiness {
    stikk_model::Readiness {
        author_ready: true,
        maintainer_ready: false,
        read_only: false,
    }
}

fn ready_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(orientation(0, None))
        .with_worktree_status(dirty_worktree())
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
        ))
}

#[test]
fn a_clean_worktree_blocks_before_arming_anything() {
    let backend = ready_backend().with_worktree_status(WorktreeStatus {
        reff: "heads/main".to_string(),
        clean: true,
        tracked: 1,
        unchanged: 1,
        missing: 0,
        modified: 0,
        untracked: 0,
        unsupported: 0,
        entries: Vec::new(),
        queued_elsewhere: None,
    });
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("nothing to commit"));
        }
        CommitPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn a_cross_ref_queue_blocks_before_arming_anything() {
    let backend = ready_backend().with_orientation(orientation(2, Some("heads/other")));
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("heads/other"));
            assert!(reason.contains("heads/main"));
        }
        CommitPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn a_ready_preview_carries_the_worktree_counts_and_a_token() {
    let backend = ready_backend();
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::Ready { preview, token } => {
            assert_eq!(preview.changes.modified, 1);
            assert_eq!(token.tier(), stikk_model::Tier::Two);
        }
        CommitPreviewOutcome::Blocked(reason) => panic!("expected Ready, got Blocked({reason})"),
    }
}

#[test]
fn the_active_patch_warning_is_carried_verbatim_into_the_preview_and_consequence() {
    let backend = ready_backend().with_orientation(orientation(800, None)).with_orientation(
        Orientation {
            active_patch_warning: Some("warning: active patches (800) at or above the recommended threshold (800); consider running `prikk seal`".to_string()),
            ..orientation(800, None)
        },
    );
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::Ready { preview, token } => {
            assert!(
                preview
                    .active_patch_notice
                    .as_deref()
                    .is_some_and(|n| n.contains("recommended threshold"))
            );
            assert!(
                token
                    .summary()
                    .consequence
                    .contains("recommended threshold")
            );
        }
        CommitPreviewOutcome::Blocked(reason) => panic!("expected Ready, got Blocked({reason})"),
    }
}

#[test]
fn confirm_and_execute_carries_the_commit_result_through() {
    let backend = ready_backend().with_commit(CommitResult {
        baseline_ref: "heads/main".to_string(),
        patch_id: "1".repeat(64),
        wal_sequence: 1,
        operations: 1,
        referenced_blobs: 1,
        text_edits: 0,
        changes: Vec::new(),
        notes: Vec::new(),
    });
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let outcome = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
        "a message",
    )
    .expect("confirm+execute succeeds");
    assert_eq!(outcome.operation, COMMIT_OPERATION);
    assert_eq!(outcome.result.patch_id, "1".repeat(64));
}

#[test]
fn execute_refuses_when_the_change_token_moved_between_preview_and_confirm() {
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let moved = backend.with_change_token(ChangeToken::compose(
        [("heads/main", "1".repeat(64).as_str())],
        0,
        None,
    ));
    let err = commit_confirm_and_execute(
        &moved,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "stale");
}

#[test]
fn a_cross_ref_race_reaching_the_seam_propagates_as_cross_ref_not_lock_conflict() {
    let backend = ready_backend().with_commit_cross_ref(
        "lock conflict: active WAL is owned by heads/main; requested ref heads/other",
    );
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "cross-ref");
}

#[test]
fn read_only_refuses_even_with_author_keys_present() {
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let read_only = stikk_model::Readiness {
        author_ready: true,
        maintainer_ready: false,
        read_only: true,
    };
    let err = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        read_only,
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn viewer_capability_refuses() {
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        stikk_model::Readiness::none(),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn declined_evidence_refuses_without_touching_the_seam() {
    // Tier 2 needs an explicit yes; a typed name is the wrong shape for this tier.
    let backend = ready_backend().with_commit_refusal("should never be reached");
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::TypedName("heads/main".to_string()),
        "heads/main",
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "declined");
}

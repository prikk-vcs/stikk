//! Tests for the commit operation (design `FR-050`/`FL-05`; RFC 014).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::ChangeToken;
use stikk_prikk::{CommitResult, NullBackend, Orientation, WorktreeStatus};

use super::*;
use stikk_model::RoleReadiness;

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
        refused: None,
        entries: Vec::new(),
        queued_elsewhere: None,
        declarations: Vec::new(),
    }
}

fn orientation(queued_patches: u64, queued_target: Option<&str>) -> Orientation {
    Orientation {
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

fn author_readiness() -> stikk_model::Readiness {
    stikk_model::Readiness {
        author: RoleReadiness::Unknown,
        maintainer: stikk_model::RoleReadiness::NotReady,
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
            &stikk_model::CurrentBranch::NotReported,
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
        refused: None,
        entries: Vec::new(),
        queued_elsewhere: None,
        declarations: Vec::new(),
    });
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("nothing to commit"));
        }
        CommitPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
        CommitPreviewOutcome::WouldRefuse(paths) => panic!("expected Blocked, got {paths:?}"),
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
        CommitPreviewOutcome::WouldRefuse(paths) => panic!("expected Blocked, got {paths:?}"),
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
        CommitPreviewOutcome::WouldRefuse(paths) => panic!("expected Ready, got {paths:?}"),
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
        CommitPreviewOutcome::WouldRefuse(paths) => panic!("expected Ready, got {paths:?}"),
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
        &stikk_model::CurrentBranch::NotReported,
    ));
    let err = commit_confirm_and_execute(
        &moved,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
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
        author: RoleReadiness::Unknown,
        maintainer: stikk_model::RoleReadiness::NotReady,
        read_only: true,
    };
    let err = commit_confirm_and_execute(
        &backend,
        repo,
        *token,
        read_only,
        crate::confirm::Evidence::ExplicitYes,
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
        "a message",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "declined");
}

// RFC 027 decision 5 — commit's prevention, on prikk's verdict and nothing else.

fn entry(kind: &str, path: &str, authoring: stikk_prikk::Authoring) -> stikk_prikk::WorktreeEntry {
    stikk_prikk::WorktreeEntry {
        kind: kind.to_string(),
        path: path.to_string(),
        note: "a note".to_string(),
        authoring,
    }
}

/// A dirty ≥ 0.39 tree: prikk 0.41's own measured shape — an authored modification beside an untracked
/// symlink `commit` refuses, with the reason `commit` prints.
fn refused_symlink_worktree() -> WorktreeStatus {
    WorktreeStatus {
        untracked: 1,
        refused: Some(1),
        entries: vec![
            entry(
                "untracked",
                "link.txt",
                stikk_prikk::Authoring::Refused(
                    "precondition not met: link.txt: worktree symlink authoring is out of scope"
                        .to_string(),
                ),
            ),
            entry("modified", "readme.txt", stikk_prikk::Authoring::Authored),
        ],
        ..dirty_worktree()
    }
}

#[test]
fn a_refused_entry_makes_commit_unavailable_and_carries_the_entry() {
    let backend = ready_backend()
        .with_version(0, 41, 0)
        .with_worktree_status(refused_symlink_worktree());
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::WouldRefuse(paths) => {
            assert_eq!(
                paths,
                vec![RefusedPath {
                    kind: ChangeKind::Untracked,
                    path: "link.txt".to_string(),
                    reason: "precondition not met: link.txt: worktree symlink authoring is out of \
                             scope"
                        .to_string(),
                }],
                "only the refused entry is carried, with its kind and prikk's reason verbatim"
            );
        }
        other => panic!("expected WouldRefuse, got {other:?}"),
    }
}

#[test]
fn the_cross_ref_and_clean_checks_still_come_first() {
    // Decision 5: after both. A cross-ref commit is refused whatever the entries say, and says so.
    let backend = ready_backend()
        .with_version(0, 41, 0)
        .with_worktree_status(refused_symlink_worktree())
        .with_orientation(orientation(2, Some("heads/other")));
    assert!(matches!(
        commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads"),
        CommitPreviewOutcome::Blocked(_)
    ));
}

#[test]
fn an_unreported_verdict_offers_commit_exactly_as_before() {
    // Below 0.39: prikk states no verdict, so stikk infers no refusal (decision 5).
    let mut status = dirty_worktree();
    status.untracked = 1;
    status.entries = vec![entry(
        "untracked",
        "link.txt",
        stikk_prikk::Authoring::Unreported,
    )];
    let backend = ready_backend()
        .with_version(0, 28, 1)
        .with_worktree_status(status);
    assert!(matches!(
        commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads"),
        CommitPreviewOutcome::Ready { .. }
    ));
}

#[test]
fn at_0_41_an_unsupported_path_prikk_marks_authored_does_not_block() {
    // Q1 ruled (b): prevention rests on prikk's verdict alone. **A prikk 0.41 fact, kept true of 0.41**
    // (RFC 029 Handoff A §5): 0.41 marks these `authored`. 0.42 marks them refused, and the next test is
    // the same rule doing what it waited for.
    let mut status = dirty_worktree();
    status.unsupported = 1;
    status.refused = Some(0);
    status.entries = vec![entry(
        "unsupported-path",
        "/tmp/repo/back\\slash.txt",
        stikk_prikk::Authoring::Authored,
    )];
    let backend = ready_backend()
        .with_version(0, 41, 0)
        .with_worktree_status(status);
    assert!(matches!(
        commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads"),
        CommitPreviewOutcome::Ready { .. }
    ));
}

#[test]
fn at_0_42_a_refused_unsupported_path_makes_commit_unavailable() {
    // RFC 029 Handoff A §5: prikk 0.42 reports an unrepresentable name as refused, with `commit`'s own
    // reason, so RFC 027's Q1 ruling (b) now prevents that commit — **with no change in stikk**. The entry
    // is in prikk 0.42's captured shape (`WORKTREE_UNSUPPORTED_JSON_0_42`): relative path, refused.
    let reason = "invalid name: backslashes are not allowed in repository paths";
    let mut status = dirty_worktree();
    status.modified = 0;
    status.unsupported = 1;
    status.refused = Some(1);
    status.entries = vec![entry(
        "unsupported-path",
        "back\\slash.txt",
        stikk_prikk::Authoring::Refused(reason.to_string()),
    )];
    let backend = ready_backend()
        .with_version(0, 42, 0)
        .with_worktree_status(status);
    match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        CommitPreviewOutcome::WouldRefuse(paths) => assert_eq!(
            paths,
            vec![RefusedPath {
                kind: ChangeKind::Unsupported,
                path: "back\\slash.txt".to_string(),
                reason: reason.to_string(),
            }]
        ),
        other => panic!("expected WouldRefuse at 0.42, got {other:?}"),
    }
}

#[test]
fn the_next_steps_say_prikkignore_is_committed_too() {
    // RFC 027 F3: `.prikkignore` is authored into the same commit. A step without that clause is false.
    let steps = would_refuse_next_steps(&[RefusedPath {
        kind: ChangeKind::Untracked,
        path: "link.txt".to_string(),
        reason: "r".to_string(),
    }]);
    assert!(steps.iter().any(|s| s.contains("Remove or replace")));
    let ignore = steps
        .iter()
        .find(|s| s.contains(".prikkignore"))
        .expect("a .prikkignore step");
    assert!(
        ignore.contains("part of this commit"),
        "the .prikkignore step must say the file is committed: {ignore:?}"
    );
    assert!(
        !steps.iter().any(|s| s.contains('\u{FFFD}')),
        "no substituted name, so no caution about one: {steps:?}"
    );
}

#[test]
fn a_substituted_name_gets_the_caution_that_it_is_not_the_real_name() {
    let steps = would_refuse_next_steps(&[RefusedPath {
        kind: ChangeKind::Untracked,
        path: "bad\u{FFFD}name.txt".to_string(),
        reason: "r".to_string(),
    }]);
    let caution = steps
        .iter()
        .find(|s| s.contains('\u{FFFD}'))
        .expect("a caution for a name shown with U+FFFD");
    assert!(caution.contains("not the file's real name"));
    assert!(caution.contains("will not match"));
}

// RFC 030 decision 3 — commit's confirmation re-reads the worktree (handoff v2 §5d, tests 1–6).
//
// Every `Stale` and refusal path below also asserts `commit_calls() == 0`: the re-read exists so that
// `prikk commit` does not run on a worktree the preview never showed, and a test that only checked the
// error would pass on code that committed first and complained after.

/// The worktree a preview lists: one modified file, authored.
fn previewed_worktree() -> WorktreeStatus {
    WorktreeStatus {
        modified: 1,
        entries: vec![entry(
            "modified",
            "readme.txt",
            stikk_prikk::Authoring::Authored,
        )],
        ..dirty_worktree()
    }
}

/// Preview, then confirm at once with an explicit yes — the only two steps a commit has.
fn preview_then_confirm(backend: &NullBackend) -> Result<Outcome<CommitResult>> {
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    commit_confirm_and_execute(
        backend,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
        "a message",
    )
}

fn is_stale(result: &Result<Outcome<CommitResult>>, want: StaleCause) -> bool {
    matches!(
        result,
        Err(StikkError::Stale { operation, cause }) if operation == COMMIT_OPERATION && *cause == want
    )
}

#[test]
fn an_equal_re_read_commits() {
    let backend = ready_backend()
        .with_worktree_status(previewed_worktree())
        .with_worktree_status_on_reread(previewed_worktree());
    let result = preview_then_confirm(&backend);
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(backend.commit_calls(), 1);
}

#[test]
fn an_entry_added_before_confirmation_is_stale_worktree_and_never_commits() {
    let mut grown = previewed_worktree();
    grown.untracked = 1;
    grown.entries.push(entry(
        "untracked",
        "notes.tmp",
        stikk_prikk::Authoring::Authored,
    ));
    let backend = ready_backend()
        .with_worktree_status(previewed_worktree())
        .with_worktree_status_on_reread(grown);
    let result = preview_then_confirm(&backend);
    assert!(is_stale(&result, StaleCause::Worktree), "{result:?}");
    assert_eq!(backend.commit_calls(), 0, "commit must not run");
}

#[test]
fn only_a_count_differing_is_stale_worktree_and_never_commits() {
    let mut counted = previewed_worktree();
    counted.modified = 2;
    let backend = ready_backend()
        .with_worktree_status(previewed_worktree())
        .with_worktree_status_on_reread(counted);
    let result = preview_then_confirm(&backend);
    assert!(is_stale(&result, StaleCause::Worktree), "{result:?}");
    assert_eq!(backend.commit_calls(), 0, "commit must not run");
}

#[test]
fn only_the_declarations_differing_is_stale_worktree_and_never_commits() {
    // M7's shape: the shell moved a.txt to b.txt before the preview; `prikk mv` declared it after. The
    // entries are identical, and only the declaration says `commit` would now author `rename-path`.
    let moved = WorktreeStatus {
        missing: 1,
        modified: 0,
        untracked: 1,
        entries: vec![
            entry("missing", "a.txt", stikk_prikk::Authoring::Authored),
            entry("untracked", "b.txt", stikk_prikk::Authoring::Authored),
        ],
        ..dirty_worktree()
    };
    let declared = WorktreeStatus {
        declarations: vec![stikk_prikk::RenameDeclaration {
            old_path: "a.txt".to_string(),
            new_path: "b.txt".to_string(),
        }],
        ..moved.clone()
    };
    let backend = ready_backend()
        .with_worktree_status(moved)
        .with_worktree_status_on_reread(declared);
    let result = preview_then_confirm(&backend);
    assert!(is_stale(&result, StaleCause::Worktree), "{result:?}");
    assert_eq!(backend.commit_calls(), 0, "commit must not run");
}

#[test]
fn a_refused_re_read_propagates_and_never_commits() {
    let backend = ready_backend()
        .with_worktree_status(previewed_worktree())
        .with_worktree_status_refusal_on_reread("error: ref does not exist: heads/main");
    let result = preview_then_confirm(&backend);
    match &result {
        Err(StikkError::Refusal { message }) => {
            assert_eq!(message, "error: ref does not exist: heads/main");
        }
        other => panic!("expected the re-read's refusal, got {other:?}"),
    }
    assert_eq!(backend.commit_calls(), 0, "commit must not run");
}

#[test]
fn a_current_branch_moved_alone_is_stale_repository_before_any_re_read() {
    // M6: refs and queue unchanged, prikk's current branch moved, the worktree unchanged. The token
    // catches it first — and the re-read here is scripted to refuse, so reaching it would have returned
    // that refusal instead of `Stale { Repository }`.
    let token_on = |branch: &str| {
        ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &stikk_model::CurrentBranch::Branch(
                stikk_model::RefName::parse(branch).expect("a ref name"),
            ),
        )
    };
    let backend = ready_backend()
        .with_worktree_status(previewed_worktree())
        .with_worktree_status_refusal_on_reread("the re-read must not be reached")
        .with_change_token(token_on("heads/main"));
    let repo = std::path::Path::new("/repo");
    let CommitPreviewOutcome::Ready { token, .. } =
        commit_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let switched = backend.clone().with_change_token(token_on("heads/dev"));
    let result = commit_confirm_and_execute(
        &switched,
        repo,
        *token,
        author_readiness(),
        crate::confirm::Evidence::ExplicitYes,
        "a message",
    );
    assert!(is_stale(&result, StaleCause::Repository), "{result:?}");
    assert_eq!(switched.commit_calls(), 0, "commit must not run");
}

/// RFC 029 Handoff B §5, safeguard 3: commit's notice in each row of the table, byte-exact, computed
/// from the preview's own Orientation read.
#[test]
fn commits_branch_notice_follows_each_row_of_safeguard_three_exactly() {
    use stikk_model::{CurrentBranch, RefName};
    let rows = [
        (
            CurrentBranch::Branch(RefName::parse("heads/dev").unwrap()),
            Some(
                "This targets heads/main. prikk's current branch is heads/dev, the ref prikk uses \
                 when no --ref is given.",
            ),
        ),
        (
            CurrentBranch::Branch(RefName::parse("heads/main").unwrap()),
            None,
        ),
        (
            CurrentBranch::Unresolved("<unresolved; run `prikk doctor`>".to_string()),
            Some(
                "This targets heads/main. prikk reports its current branch as <unresolved; run `prikk doctor`>.",
            ),
        ),
        (CurrentBranch::NotReported, None),
    ];
    for (current, expected) in rows {
        let backend = ready_backend().with_orientation(Orientation {
            current_branch: current.clone(),
            ..orientation(0, None)
        });
        match commit_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads")
        {
            CommitPreviewOutcome::Ready { token, .. } => {
                assert_eq!(
                    token.summary().branch_notice.as_deref(),
                    expected,
                    "{current:?}"
                );
            }
            other => panic!("expected Ready for {current:?}, got {other:?}"),
        }
    }
}

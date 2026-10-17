//! Tests for switching branch (RFC 045).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::{ChangeToken, CurrentBranch, Readiness, RoleReadiness, Tier};
use stikk_prikk::{Diff, DiffEntry, DiffSide, DiffTarget, NullBackend, Orientation, QueuedOnSide};

use super::*;

fn repo() -> &'static std::path::Path {
    std::path::Path::new("/repo")
}

fn entry(path: &str, status: &str, from_path: Option<&str>) -> DiffEntry {
    DiffEntry {
        path: path.to_string(),
        status: status.to_string(),
        from_path: from_path.map(ToString::to_string),
        from: None,
        to: None,
        minimal: true,
        hunks: Vec::new(),
        raw: "{}".to_string(),
    }
}

fn side(point: &str) -> DiffSide {
    DiffSide {
        point: point.to_string(),
        target: DiffTarget::Block("b".repeat(64)),
        queued: QueuedOnSide::NotApplicable,
    }
}

fn diff(entries: Vec<DiffEntry>, unsupported: Vec<stikk_prikk::UnsupportedPath>) -> Diff {
    Diff {
        from: side("heads/main"),
        to: side("heads/feature"),
        entries,
        unsupported_paths: unsupported,
    }
}

/// A 0.52 session on `heads/main`, with `queued` patches targeting `queued_target`.
fn backend(current: CurrentBranch, queued: u64, queued_target: Option<&str>) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 52, 0)
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: queued,
            queued_target: queued_target.map(ToString::to_string),
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: current.clone(),
        })
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            queued,
            queued_target,
            &current,
        ))
}

fn on_main() -> NullBackend {
    backend(
        CurrentBranch::Branch(stikk_model::RefName::parse("heads/main").expect("a ref name")),
        0,
        None,
    )
}

/// The ordinary case: one modified, one deleted, one added — RFC 045 F4's own measurement.
fn ready() -> NullBackend {
    on_main().with_diff(diff(
        vec![
            entry("a.txt", "modified", None),
            entry("b.txt", "deleted", None),
            entry("c.txt", "added", None),
        ],
        Vec::new(),
    ))
}

fn summary(preview: SwitchPreview) -> ConfirmationSummary {
    match preview {
        SwitchPreview::Ready { token } => token.summary().clone(),
        SwitchPreview::Blocked(reason) => panic!("expected Ready, got blocked: {reason}"),
    }
}

fn blocked_reason(preview: SwitchPreview) -> String {
    match preview {
        SwitchPreview::Blocked(reason) => reason,
        SwitchPreview::Ready { .. } => panic!("expected Blocked"),
    }
}

/// **The preview is the file plan** (decision 5), classified from prikk's own entries — written
/// is `added` + `modified`, deleted is `deleted`, which is exactly what the switch then reported
/// at 0.52.0 (F4).
#[test]
fn the_plan_is_what_compare_says_the_switch_will_write_and_delete() {
    let s = summary(switch_preview(&ready(), repo(), "heads/feature").expect("reads"));
    assert_eq!(s.operation, "Switch branch");
    let plan = s.materializes.expect("a plan");
    assert_eq!(plan.written, vec!["a.txt", "c.txt"]);
    assert_eq!(plan.deleted, vec!["b.txt"]);
    assert_eq!(
        s.counts,
        vec![("written", 2), ("deleted", 1)],
        "the counts prikk itself reported for this comparison"
    );
}

/// **A rename moves bytes**: the destination is written and the source goes.
#[test]
fn a_rename_is_a_write_and_a_delete() {
    let backend = on_main().with_diff(diff(
        vec![entry("new.txt", "renamed", Some("old.txt"))],
        Vec::new(),
    ));
    let plan = summary(switch_preview(&backend, repo(), "heads/feature").expect("reads"))
        .materializes
        .expect("a plan");
    assert_eq!(plan.written, vec!["new.txt"]);
    assert_eq!(plan.deleted, vec!["old.txt"]);
}

/// **A plan that is not whole is not offered** (§6 trap 2) — an unmeasured status word, or any
/// unsupported path.
#[test]
fn an_incomplete_plan_blocks_rather_than_dropping_what_it_cannot_place() {
    let unknown = on_main().with_diff(diff(
        vec![
            entry("a.txt", "modified", None),
            entry("s.txt", "symlink", None),
        ],
        Vec::new(),
    ));
    let reason = blocked_reason(switch_preview(&unknown, repo(), "heads/feature").expect("reads"));
    assert!(reason.contains("the whole plan"), "{reason}");

    let unsupported = on_main().with_diff(diff(
        vec![entry("a.txt", "modified", None)],
        vec![stikk_prikk::UnsupportedPath {
            path: "link".to_string(),
            refusal: "worktree symlink authoring is out of scope".to_string(),
        }],
    ));
    let reason =
        blocked_reason(switch_preview(&unsupported, repo(), "heads/feature").expect("reads"));
    assert!(reason.contains("the whole plan"), "{reason}");
}

/// **No differences says so**, rather than showing an empty list and leaving the reader to guess.
#[test]
fn two_branches_holding_the_same_files_say_the_switch_writes_nothing() {
    let backend = on_main().with_diff(diff(Vec::new(), Vec::new()));
    let s = summary(switch_preview(&backend, repo(), "heads/feature").expect("reads"));
    assert!(s.materializes.as_ref().expect("a plan").is_empty());
    assert!(s.consequence.contains("writes nothing"), "{s:?}");
}

/// **The three blocked cases that need no plan do not read one** (decision 6): the diff is never
/// asked for, which a backend that would panic on it proves.
#[test]
fn the_blocked_cases_do_not_read_a_comparison() {
    // A backend whose `diff` refuses: if any of these three read it, the preview would block with
    // `NO_FILE_PLAN` instead of its own reason, and these assertions would fail.
    let refusing = |b: NullBackend| b.with_diff_refusal("diff must not be read here");

    let already =
        blocked_reason(switch_preview(&refusing(on_main()), repo(), "heads/main").expect("reads"));
    assert_eq!(already, ALREADY_THE_CURRENT_BRANCH);

    let queued = refusing(backend(
        CurrentBranch::Branch(stikk_model::RefName::parse("heads/main").expect("a ref name")),
        1,
        Some("heads/feature"),
    ));
    let reason = blocked_reason(switch_preview(&queued, repo(), "heads/feature").expect("reads"));
    assert_eq!(reason, WORK_IS_QUEUED_FOR_THIS_BRANCH);

    let unreported = refusing(backend(CurrentBranch::NotReported, 0, None));
    let reason =
        blocked_reason(switch_preview(&unreported, repo(), "heads/feature").expect("reads"));
    assert_eq!(reason, NO_FILE_PLAN);
}

/// **Work queued for another ref is prikk's to refuse, not stikk's** (§6 trap 4): prikk's own
/// message already names the way out, and widening the guard would take that decision from it.
#[test]
fn work_queued_elsewhere_is_not_blocked_here() {
    let elsewhere = backend(
        CurrentBranch::Branch(stikk_model::RefName::parse("heads/main").expect("a ref name")),
        1,
        Some("heads/main"),
    )
    .with_diff(diff(vec![entry("a.txt", "modified", None)], Vec::new()));
    let s = summary(switch_preview(&elsewhere, repo(), "heads/feature").expect("reads"));
    assert!(s.materializes.is_some(), "the preview is offered");
}

/// **No plan, no switch** (decision 6): an unresolved pointer, and a comparison prikk will not
/// make, both say so in stikk's words and name prikk's own command as the way out.
#[test]
fn without_a_comparison_there_is_no_switch_and_stikk_says_why() {
    let unresolved = backend(
        CurrentBranch::Unresolved("<unresolved; run `prikk doctor`>".to_string()),
        0,
        None,
    );
    let reason =
        blocked_reason(switch_preview(&unresolved, repo(), "heads/feature").expect("reads"));
    assert_eq!(reason, NO_FILE_PLAN);

    // prikk's own answer for an unpublished current branch is *"does not exist"*, which is false
    // and alarming about the branch the reader is standing on. stikk's reason replaces it.
    let refused = on_main().with_diff_refusal("ref heads/main does not exist in this repository");
    let reason = blocked_reason(switch_preview(&refused, repo(), "heads/feature").expect("reads"));
    assert_eq!(reason, NO_FILE_PLAN);
    assert!(!reason.contains("does not exist"), "{reason}");
    assert!(reason.contains("prikk branch switch"), "{reason}");

    // Below the band the seam answers `Unreported`, which is the same absence of a plan.
    let below = on_main();
    let reason = blocked_reason(switch_preview(&below, repo(), "heads/feature").expect("reads"));
    assert_eq!(reason, NO_FILE_PLAN);
}

/// **A comparison a lock stopped is not "no file plan"** (review v1 C1).
///
/// `NO_FILE_PLAN` sends the reader to `prikk branch switch`, which is the way out of an
/// uncomparable current branch and of nothing else. A held lock, an environment fault or an
/// internal error says nothing about whether a plan exists, and each has its own handling
/// everywhere else in stikk — so each travels as itself.
#[test]
fn a_comparison_stopped_by_a_lock_travels_as_itself() {
    let locked =
        on_main().with_diff_lock_conflict("another prikk process holds the repository lock");
    let error = switch_preview(&locked, repo(), "heads/feature")
        .expect_err("a lock is not a blocked preview");
    assert_eq!(error.class(), "lock-conflict", "{error}");
    assert!(
        error.to_string().contains("holds the repository lock"),
        "prikk's own words reach the caller: {error}"
    );
    assert!(
        !error.to_string().contains("prikk branch switch"),
        "and never the no-plan reason: {error}"
    );
}

/// **Tier Three, and no key** (decisions 2 and 3).
#[test]
fn a_switch_is_tier_three_and_consumes_no_signing_key() {
    match switch_preview(&ready(), repo(), "heads/feature").expect("reads") {
        SwitchPreview::Ready { token } => {
            assert_eq!(token.tier(), Tier::Three);
            assert_eq!(
                token.summary().capability,
                None,
                "prikk needs no key, so stikk asks for none"
            );
            assert!(token.summary().signing_key_id.is_none());
        }
        SwitchPreview::Blocked(reason) => panic!("{reason}"),
    }
}

/// **A session with no signing key at all may switch** — the whole point of keying the gate on
/// the category rather than the tier (§2b).
#[test]
fn a_session_with_no_key_may_switch() {
    let viewer = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    };
    assert!(
        crate::capability_gate(
            SWITCH_OPERATION,
            stikk_model::RequestCategory::Materialization,
            viewer
        )
        .is_ok(),
        "a switch signs nothing"
    );
    // And read-only still refuses: the requirement is read-write, not nothing.
    let read_only = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::NotReady,
        read_only: true,
    };
    assert!(
        crate::capability_gate(
            SWITCH_OPERATION,
            stikk_model::RequestCategory::Materialization,
            read_only
        )
        .is_err()
    );
}

/// **No name is inside the consequence** (RFC 044 review C1), and the names are restated in the
/// targets, with the target's block beside it.
#[test]
fn no_name_reaches_the_consequence_and_the_targets_name_the_block() {
    let tip = "a1b2c3d4e5f6".to_string() + &"0".repeat(52);
    let backend = ready().with_history(stikk_prikk::History {
        reff: "heads/feature".to_string(),
        blocks: vec![stikk_prikk::BlockRow {
            block_id: tip.clone(),
            ref_state_id: "r".repeat(64),
            update_seq: 2,
            kind: "Normal".to_string(),
            rollback_block: false,
            parents: 1,
            patches: 1,
            rollback_patches: 0,
            required_attestations: 0,
            messages: Vec::new(),
            previous_ref_state: None,
        }],
    });
    let s = summary(switch_preview(&backend, repo(), "heads/feature").expect("reads"));
    for name in ["heads/main", "heads/feature"] {
        assert!(!s.consequence.contains(name), "{name}: {s:?}");
        assert!(s.target_ids.iter().any(|id| id == name), "{s:?}");
    }
    let short_tip: String = tip.chars().take(crate::SHORT_ID_CHARS).collect();
    assert_eq!(
        s.target_ids,
        vec![
            "heads/main".to_string(),
            "heads/feature".to_string(),
            short_tip
        ]
    );
    // The half a reader most needs: what a switch will not touch, and that nothing is lost.
    assert!(
        s.consequence.contains("Untracked files are not touched"),
        "{s:?}"
    );
    assert!(s.consequence.contains("restores every byte"), "{s:?}");
}

/// The band is the one `diff` already uses, never a second literal.
#[test]
fn the_band_is_diffs_own() {
    assert_eq!(SWITCH_FROM_MINOR, stikk_prikk::DIFF_FROM_MINOR);
    assert_eq!(SWITCH_FROM_MINOR, 46);
    let reason = switch_unavailable(45);
    assert!(reason.contains("0.46"), "{reason}");
    assert!(reason.contains("0.45"), "{reason}");
}

/// **The remainder line counts exactly what it hides** and never invites `Esc`.
#[test]
fn the_unshown_line_counts_what_it_hides() {
    assert!(crate::unshown_paths_line(3, 10).contains("and 3 more"));
    assert!(crate::unshown_paths_line(10, 10).contains("none shown here"));
    for line in [
        crate::unshown_paths_line(3, 10),
        crate::unshown_paths_line(10, 10),
    ] {
        assert!(line.contains("10"), "{line}");
        assert!(!line.contains("Esc"), "{line}");
        // **The whole route, not one key** (review v1 C2): `d` alone, in the picker, answers
        // "mark a ref with m first".
        assert!(line.contains("ref picker"), "{line}");
        assert!(line.contains("m on one branch"), "{line}");
        assert!(line.contains("d on the other"), "{line}");
    }
}

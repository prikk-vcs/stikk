//! Tests for the seal operation (design `FR-052`/`FL-06`; RFC 016).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::Binding;
use stikk_model::{ChangeToken, Readiness, RoleReadiness, Tier};
use stikk_prikk::{NullBackend, Orientation, SealResult};

use super::*;

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

fn maintainer_readiness(maintainer: RoleReadiness) -> Readiness {
    Readiness {
        author: RoleReadiness::Unknown,
        maintainer,
        read_only: false,
    }
}

/// The queue seal reads at every version (RFC 028 decision 4) — unreported, as the 0.30 `NullBackend`
/// reports it.
fn queue(count: u64, target: Option<&str>) -> stikk_prikk::QueueReport {
    stikk_prikk::QueueReport::Unreported {
        count,
        target: target.map(str::to_string),
    }
}

fn ready_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(orientation(1, Some("heads/main")))
        .with_queue(queue(1, Some("heads/main")))
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &stikk_model::CurrentBranch::NotReported,
        ))
}

#[test]
fn an_empty_queue_blocks_before_arming_anything() {
    let backend = ready_backend().with_queue(queue(0, None));
    match seal_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        SealPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("nothing to seal"));
        }
        SealPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn a_cross_ref_queue_blocks_before_arming_anything() {
    let backend = ready_backend().with_queue(queue(2, Some("heads/other")));
    match seal_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        SealPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("heads/other"));
            assert!(reason.contains("heads/main"));
        }
        SealPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn a_ready_preview_carries_the_patch_count_and_tier_three() {
    let backend = ready_backend();
    match seal_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        SealPreviewOutcome::Ready { token } => {
            assert_eq!(token.tier(), Tier::Three);
            assert_eq!(token.summary().counts, vec![("patches", 1)]);
            assert_eq!(
                token.summary().capability,
                stikk_model::Capability::Maintainer
            );
        }
        SealPreviewOutcome::Blocked(reason) => panic!("expected Ready, got Blocked({reason})"),
    }
}

#[test]
fn the_trust_refusal_warning_appears_only_when_adoption_is_unknown() {
    // RFC 016 §7: "driven by the RoleReadiness state, not shown unconditionally." `env.rs` can
    // only ever produce `Unknown` for present key material (RFC 016 F3), but this test still proves
    // the *rule* — that the warning is conditional, not baked into the base consequence text.
    assert!(!consequence(RoleReadiness::NotReady).contains("trust refusal"));
    assert!(consequence(RoleReadiness::Unknown).contains("trust refusal"));
    assert!(!consequence(RoleReadiness::Known(Binding::Matches)).contains("trust refusal"));
}

#[test]
fn the_consequence_never_promises_success() {
    // RFC 016 decision 2, in every readiness state.
    for state in [
        RoleReadiness::Known(Binding::Matches),
        RoleReadiness::NotReady,
        RoleReadiness::Unknown,
    ] {
        assert!(consequence(state).contains("does not promise success"));
    }
}

/// RFC 016 §8: the consent copy must state what is true of the ceremony, never justify itself by
/// prikk's `--allow-no-audit` flag or otherwise attribute the requirement to prikk. A test, not just a
/// reviewer, per the handoff's explicit instruction.
#[test]
fn the_consent_copy_never_names_prikks_flag_as_its_reason() {
    let lowered = SEAL_CONSENT_COPY.to_ascii_lowercase();
    for forbidden in [
        "--allow-no-audit",
        "allow-no-audit",
        "prikk requires",
        "prikk demands",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "consent copy must not cite {forbidden:?} as its reason: {SEAL_CONSENT_COPY:?}"
        );
    }
    assert!(lowered.contains("cannot be undone"));
}

#[test]
fn confirm_and_execute_carries_the_seal_result_through() {
    let backend = ready_backend().with_seal(SealResult {
        patches: 1,
        block_id: "1".repeat(64),
        reff: "heads/main".to_string(),
        ref_state: "2".repeat(64),
        notes: vec!["note: audit plugins remain later PRs".to_string()],
    });
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let outcome = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        maintainer_readiness(RoleReadiness::Unknown),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect("confirm+execute succeeds");
    assert_eq!(outcome.operation, SEAL_OPERATION);
    assert_eq!(outcome.result.block_id, "1".repeat(64));
}

#[test]
fn execute_refuses_when_the_change_token_moved_between_preview_and_confirm() {
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let moved = backend.with_change_token(ChangeToken::compose(
        [("heads/main", "1".repeat(64).as_str())],
        0,
        None,
        &stikk_model::CurrentBranch::NotReported,
    ));
    let err = seal_confirm_and_execute(
        &moved,
        repo,
        *token,
        maintainer_readiness(RoleReadiness::Unknown),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "stale");
}

#[test]
fn a_cross_ref_race_reaching_the_seam_propagates_as_cross_ref_not_lock_conflict() {
    let backend = ready_backend().with_seal_cross_ref(
        "active WAL is owned by heads/main; requested seal ref is heads/other",
    );
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        maintainer_readiness(RoleReadiness::Unknown),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "cross-ref");
}

#[test]
fn a_trust_refusal_reaching_the_seam_propagates_as_not_ready() {
    // RFC 017 F5/F6, this ceremony's own consumer of it (RFC 016 §9): a trust refusal must not degrade
    // to a bare refusal by the time it reaches the ceremony.
    let backend = ready_backend().with_seal_not_ready(
        "invalid signature: maintainer signer key id x is not trusted by policy",
    );
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        maintainer_readiness(RoleReadiness::Unknown),
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn read_only_refuses_even_with_maintainer_keys_present() {
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let read_only = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: true,
    };
    let err = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        read_only,
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn author_only_capability_refuses() {
    // AUTHOR readiness alone does not satisfy tier 3 — sealing needs MAINTAINER.
    let backend = ready_backend();
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let author_only = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    };
    let err = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        author_only,
        crate::confirm::Evidence::ExplicitYes,
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn declined_evidence_refuses_without_touching_the_seam() {
    // Tier 3 needs an explicit yes; a typed name is the wrong shape for this (untyped, RFC 013 Q3)
    // tier.
    let backend = ready_backend().with_seal_refusal("should never be reached");
    let repo = std::path::Path::new("/repo");
    let SealPreviewOutcome::Ready { token } =
        seal_preview(&backend, repo, "heads/main").expect("reads")
    else {
        panic!("expected Ready");
    };
    let err = seal_confirm_and_execute(
        &backend,
        repo,
        *token,
        maintainer_readiness(RoleReadiness::Unknown),
        crate::confirm::Evidence::TypedName("heads/main".to_string()),
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "declined");
}

/// RFC 029 Handoff B §5, safeguard 3: seal's notice in each row of the table, byte-exact — the same
/// words as commit's, from the same function.
#[test]
fn seals_branch_notice_follows_each_row_of_safeguard_three_exactly() {
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
            ..orientation(1, Some("heads/main"))
        });
        match seal_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
            SealPreviewOutcome::Ready { token } => {
                assert_eq!(
                    token.summary().branch_notice.as_deref(),
                    expected,
                    "{current:?}"
                );
            }
            SealPreviewOutcome::Blocked(reason) => {
                panic!("expected Ready for {current:?}, got Blocked({reason})")
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 028 decision 4 (Handoff B §2–§3): seal names what it freezes, from one queue read.
// ---------------------------------------------------------------------------------------------

const PATCH_A: &str = "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a";
const PATCH_B: &str = "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478";

/// Two queued patches for `target`, with messages as prikk reports them at `minor`.
fn listed(minor: u32, target: &str) -> stikk_prikk::QueueReport {
    let message = |text: &str| {
        if minor >= 42 {
            stikk_prikk::QueuedMessage::Text(text.to_string())
        } else {
            stikk_prikk::QueuedMessage::NotReported
        }
    };
    stikk_prikk::QueueReport::Listed(stikk_prikk::Queue {
        count: 2,
        target: stikk_prikk::QueueTarget::Ref(target.to_string()),
        threshold: Some(stikk_prikk::QueueThreshold {
            status: stikk_prikk::ThresholdStatus::None,
            warn: 800,
            hard_limit: 1000,
        }),
        patches: vec![
            stikk_prikk::QueuedPatch {
                patch_id: PATCH_A.to_string(),
                message: message("add b"),
                operations: Vec::new(),
            },
            stikk_prikk::QueuedPatch {
                patch_id: PATCH_B.to_string(),
                message: message("rename a to c"),
                operations: Vec::new(),
            },
        ],
    })
}

fn ready_summary(backend: &NullBackend) -> ConfirmationSummary {
    match seal_preview(backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        SealPreviewOutcome::Ready { token } => token.summary().clone(),
        SealPreviewOutcome::Blocked(reason) => panic!("expected Ready, got Blocked({reason})"),
    }
}

#[test]
fn at_0_42_the_summary_names_each_patch_with_its_message() {
    let backend = ready_backend()
        .with_version(0, 42, 0)
        .with_queue(listed(42, "heads/main"));
    let summary = ready_summary(&backend);
    assert_eq!(summary.counts, vec![("patches", 2)]);
    assert_eq!(
        summary.freezes,
        Some(crate::confirm::FrozenPatches::Listed {
            rows: vec![
                "493e58168d77  add b".to_string(),
                "71c62d0a05e5  rename a to c".to_string(),
            ],
            foot: None,
        })
    );
}

#[test]
fn at_0_41_the_summary_names_short_ids_and_says_messages_are_not_reported() {
    let backend = ready_backend()
        .with_version(0, 41, 0)
        .with_queue(listed(41, "heads/main"));
    assert_eq!(
        ready_summary(&backend).freezes,
        Some(crate::confirm::FrozenPatches::Listed {
            rows: vec!["493e58168d77".to_string(), "71c62d0a05e5".to_string()],
            foot: Some("prikk 0.41 does not report a queued patch's message.".to_string()),
        })
    );
}

#[test]
fn below_0_39_the_summary_says_prikk_does_not_list_queued_patches() {
    // `NullBackend::supported()` reports prikk 0.30.
    let summary = ready_summary(&ready_backend());
    assert_eq!(summary.counts, vec![("patches", 1)]);
    assert_eq!(
        summary.freezes,
        Some(crate::confirm::FrozenPatches::Unlisted(
            "prikk 0.30 does not list queued patches.".to_string()
        ))
    );
}

#[test]
fn the_blocks_and_the_count_come_from_the_queue_read_not_orientation() {
    // Orientation disagrees with the queue in every case below; whichever the outcome follows is the read.
    let backend = ready_backend()
        .with_version(0, 42, 0)
        .with_orientation(orientation(5, Some("heads/other")))
        .with_queue(listed(42, "heads/main"));
    assert_eq!(ready_summary(&backend).counts, vec![("patches", 2)]);

    let empty_queue = ready_backend()
        .with_orientation(orientation(3, Some("heads/main")))
        .with_queue(queue(0, None));
    assert!(matches!(
        seal_preview(&empty_queue, std::path::Path::new("/repo"), "heads/main").expect("reads"),
        SealPreviewOutcome::Blocked(reason) if reason.contains("nothing to seal")
    ));

    let other_ref = ready_backend()
        .with_version(0, 42, 0)
        .with_orientation(orientation(2, Some("heads/main")))
        .with_queue(listed(42, "heads/other"));
    assert!(matches!(
        seal_preview(&other_ref, std::path::Path::new("/repo"), "heads/main").expect("reads"),
        SealPreviewOutcome::Blocked(reason) if reason.contains("heads/other")
    ));
}

#[test]
fn missing_target_metadata_does_not_block_and_prikk_decides() {
    let mut report = listed(42, "heads/main");
    if let stikk_prikk::QueueReport::Listed(queue) = &mut report {
        queue.target = stikk_prikk::QueueTarget::MissingMetadata;
    }
    let backend = ready_backend().with_version(0, 42, 0).with_queue(report);
    assert_eq!(ready_summary(&backend).counts, vec![("patches", 2)]);
}

#[test]
fn the_remainder_lines_words_are_exact() {
    assert_eq!(
        crate::confirm::unshown_patches_line(9, 12),
        "and 9 more not shown — the Queue view lists all 12"
    );
    assert_eq!(
        crate::confirm::unshown_patches_line(12, 12),
        "none shown here — the Queue view lists all 12"
    );
}

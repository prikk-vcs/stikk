//! Tests for the seal operation (design `FR-052`/`FL-06`; RFC 016).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::{ChangeToken, MaintainerReadiness, Readiness, Tier};
use stikk_prikk::{NullBackend, Orientation, SealResult};

use super::*;

fn orientation(queued_patches: u64, queued_target: Option<&str>) -> Orientation {
    Orientation {
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
    }
}

fn maintainer_readiness(maintainer_readiness: MaintainerReadiness) -> Readiness {
    Readiness {
        author_ready: true,
        maintainer_readiness,
        read_only: false,
    }
}

fn ready_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(orientation(1, Some("heads/main")))
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
        ))
}

#[test]
fn an_empty_queue_blocks_before_arming_anything() {
    let backend = ready_backend().with_orientation(orientation(0, None));
    match seal_preview(&backend, std::path::Path::new("/repo"), "heads/main").expect("reads") {
        SealPreviewOutcome::Blocked(reason) => {
            assert!(reason.contains("nothing to seal"));
        }
        SealPreviewOutcome::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn a_cross_ref_queue_blocks_before_arming_anything() {
    let backend = ready_backend().with_orientation(orientation(2, Some("heads/other")));
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
    // RFC 016 §7: "driven by the MaintainerReadiness state, not shown unconditionally." `env.rs` can
    // only ever produce `Unknown` for present key material (RFC 016 F3), but this test still proves
    // the *rule* — that the warning is conditional, not baked into the base consequence text.
    assert!(!consequence(MaintainerReadiness::NotReady).contains("trust refusal"));
    assert!(consequence(MaintainerReadiness::Unknown).contains("trust refusal"));
    assert!(!consequence(MaintainerReadiness::Ready).contains("trust refusal"));
}

#[test]
fn the_consequence_never_promises_success() {
    // RFC 016 decision 2, in every readiness state.
    for state in [
        MaintainerReadiness::Ready,
        MaintainerReadiness::NotReady,
        MaintainerReadiness::Unknown,
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
        maintainer_readiness(MaintainerReadiness::Unknown),
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
    ));
    let err = seal_confirm_and_execute(
        &moved,
        repo,
        *token,
        maintainer_readiness(MaintainerReadiness::Unknown),
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
        maintainer_readiness(MaintainerReadiness::Unknown),
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
        maintainer_readiness(MaintainerReadiness::Unknown),
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
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Unknown,
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
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::NotReady,
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
        maintainer_readiness(MaintainerReadiness::Unknown),
        crate::confirm::Evidence::TypedName("heads/main".to_string()),
        "heads/main",
    )
    .expect_err("must refuse");
    assert_eq!(err.class(), "declined");
}

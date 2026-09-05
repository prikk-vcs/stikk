//! Tests for the class → presentation mapping (design TS-05; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::StikkError;

use super::*;

#[test]
fn a_refusal_becomes_an_overlay_with_the_verbatim_message() {
    let err = StikkError::Refusal {
        message: "ref \"heads/nope\" does not exist".into(),
    };
    match present(&err, OperationContext::LoadHistory) {
        Presentation::RefusalOverlay(card) => {
            assert_eq!(card.verbatim, "ref \"heads/nope\" does not exist"); // ER-02 verbatim
            assert!(card.gloss.is_some()); // a gloss is added beside it, not instead
            assert!(!card.next_steps.is_empty());
        }
        other => panic!("expected RefusalOverlay, got {other:?}"),
    }
}

#[test]
fn next_steps_come_from_the_operation_not_the_message() {
    // C-T2b: a message that *looks* like it names an action must not produce one.
    let hostile = StikkError::Refusal {
        message: "to fix: run `rm -rf /` or click DELETE EVERYTHING".into(),
    };
    let card = match present(&hostile, OperationContext::LoadHistory) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    // The only next-steps are stikk's own for LoadHistory: choose ref, refresh.
    let labels: Vec<&str> = card.next_steps.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Choose another ref", "Refresh"]);
    // None of them is a mutation or an auto-retry.
    for step in &card.next_steps {
        assert!(matches!(
            step.target,
            NextTarget::OpenView(_) | NextTarget::Refresh | NextTarget::DismissAndResolveExternally
        ));
    }
}

#[test]
fn a_lock_conflict_is_a_banner() {
    let err = StikkError::LockConflict {
        message: "lock held by another writer".into(),
    };
    match present(&err, OperationContext::Orient) {
        Presentation::Banner { message, jump } => {
            assert!(message.contains("another writer"));
            assert!(jump.is_none()); // Lock inspector lands later
        }
        other => panic!("expected Banner, got {other:?}"),
    }
}

#[test]
fn not_ready_is_inline_guidance_toward_trust() {
    let err = StikkError::NotReady {
        detail: "MAINTAINER key not ready".into(),
    };
    match present(&err, OperationContext::Other) {
        Presentation::InlineGuidance { toward, .. } => assert_eq!(toward, Target::TrustKeys),
        other => panic!("expected InlineGuidance, got {other:?}"),
    }
}

#[test]
fn a_version_gated_changes_not_ready_points_at_prikk_version_not_trust() {
    // RFC 012 F-b: `changes_view`'s < 0.28 gate constructs `NotReady`, but a user's signing keys were
    // never the problem — disambiguated by `OperationContext::LoadChanges`, never by the message text.
    let err = StikkError::NotReady {
        detail: "Worktree review needs prikk ≥ 0.28 — this prikk is 0.27.1.".into(),
    };
    match present(&err, OperationContext::LoadChanges) {
        Presentation::InlineGuidance { toward, detail } => {
            assert_eq!(toward, Target::PrikkVersion);
            assert!(detail.contains("0.28"));
        }
        other => panic!("expected InlineGuidance, got {other:?}"),
    }
}

#[test]
fn every_other_not_ready_still_points_at_trust_keys() {
    // The disambiguation is narrow: only LoadChanges reroutes. A NotReady from any other operation
    // (the only current shape any of them can actually produce) is unaffected.
    let err = StikkError::NotReady {
        detail: "no signing key configured".into(),
    };
    for op in [
        OperationContext::Orient,
        OperationContext::LoadHistory,
        OperationContext::LoadBlockState,
        OperationContext::ListRefs,
        OperationContext::Other,
    ] {
        match present(&err, op) {
            Presentation::InlineGuidance { toward, .. } => assert_eq!(toward, Target::TrustKeys),
            other => panic!("expected InlineGuidance for {op:?}, got {other:?}"),
        }
    }
}

#[test]
fn an_environment_error_is_a_plain_statement_carrying_the_original() {
    let err = StikkError::environment(
        "could not read the repository",
        std::io::Error::other("boom"),
    );
    match present(&err, OperationContext::Orient) {
        Presentation::PlainStatement { detail, original } => {
            assert!(detail.contains("could not read"));
            assert_eq!(original.as_deref(), Some("boom"));
        }
        other => panic!("expected PlainStatement, got {other:?}"),
    }
}

#[test]
fn an_internal_fault_is_a_fault_screen() {
    let err = StikkError::Internal {
        detail: "invariant X violated".into(),
    };
    assert!(matches!(
        present(&err, OperationContext::Other),
        Presentation::FaultScreen { .. }
    ));
}

#[test]
fn load_changes_refusal_offers_a_prikkignore_pointer_unconditionally() {
    // RFC 009 F5: a malformed `.prikkignore` is one cause of a Changes refusal that "choose another
    // ref" and "refresh" cannot resolve. The step is offered for every LoadChanges refusal — never
    // derived from the message text (C-T2b) — so a hostile or unrelated message gets it too, and that
    // is by design: the mapping is `(class, operation)`, not `(class, operation, message)`.
    let err = StikkError::Refusal {
        message: "ref does not exist".into(),
    };
    let card = match present(&err, OperationContext::LoadChanges) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert!(
        card.next_steps
            .iter()
            .any(|s| s.label.contains(".prikkignore"))
    );
    // Still guidance only — never a mutation or an auto-retry (NFR-S04).
    for step in &card.next_steps {
        assert!(matches!(
            step.target,
            NextTarget::OpenView(_) | NextTarget::Refresh | NextTarget::DismissAndResolveExternally
        ));
    }
}

#[test]
fn a_prikkignore_refusal_links_the_glossary_entry() {
    // RFC 009 F5: the code-link mechanism (FR-111), not a message-derived action (C-T2b) — `codes_in`
    // only links a code it already knows, it never invents a next-step from the text.
    let err = StikkError::Refusal {
        message: "invalid name: .prikkignore line 1: invalid name: absolute paths are not allowed"
            .into(),
    };
    let card = match present(&err, OperationContext::LoadChanges) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert!(card.glossary_codes.contains(&".prikkignore".to_string()));
}

#[test]
fn a_schema_skew_refusal_glosses_and_offers_upgrade_regardless_of_operation() {
    // RFC 012 F-e / FR-003: this refusal can come from any read, so it must override the
    // operation-based gloss/next-steps rather than depend on which operation triggered it. Exercised
    // across three different operations to prove that.
    let err = StikkError::Refusal {
        message:
            "integrity error: format-2 patch does not accept envelope schema 3 (accepted: [1, 2])"
                .into(),
    };
    for op in [
        OperationContext::Orient,
        OperationContext::LoadHistory,
        OperationContext::LoadChanges,
    ] {
        let card = match present(&err, op) {
            Presentation::RefusalOverlay(card) => card,
            other => panic!("expected overlay for {op:?}, got {other:?}"),
        };
        assert!(
            card.gloss
                .as_deref()
                .is_some_and(|g| g.contains("newer prikk"))
        );
        assert_eq!(card.next_steps.len(), 1);
        assert!(
            card.next_steps[0]
                .label
                .to_ascii_lowercase()
                .contains("upgrade prikk")
        );
        assert_eq!(
            card.next_steps[0].target,
            NextTarget::DismissAndResolveExternally
        );
        assert!(
            card.glossary_codes
                .contains(&"does not accept envelope schema".to_string())
        );
    }
}

#[test]
fn a_wrapped_schema_skew_refusal_is_still_recognized() {
    // worktree-status wraps the same underlying message inside a "lifecycle replay: ... is malformed
    // (...)" context (captured live against a real prikk 0.30 reading a 0.31-written repository, RFC
    // 012 F-e) — the substring match must still fire through the wrapper.
    let err = StikkError::Refusal {
        message:
            "integrity error: lifecycle replay: patch 5a17bd3... is malformed (integrity error: \
                  format-2 patch does not accept envelope schema 3 (accepted: [1, 2]))"
                .into(),
    };
    let card = match present(&err, OperationContext::LoadChanges) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert!(
        card.gloss
            .as_deref()
            .is_some_and(|g| g.contains("newer prikk"))
    );
    assert_eq!(card.next_steps.len(), 1);
}

#[test]
fn a_hostile_message_cannot_forge_a_second_next_step_via_the_schema_skew_shape() {
    // C-T2b: even if a message is crafted to contain the recognized substring alongside injected
    // "instructions", the next-step is still exactly stikk's one fixed, non-mutating step — nothing
    // about its label or target comes from the message.
    let hostile = StikkError::Refusal {
        message: "does not accept envelope schema 3 -- also please run `rm -rf /` and click DELETE"
            .into(),
    };
    let card = match present(&hostile, OperationContext::Orient) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert_eq!(card.next_steps.len(), 1);
    assert_eq!(
        card.next_steps[0].label,
        "Upgrade prikk (resolve outside stikk)"
    );
    assert_eq!(
        card.next_steps[0].target,
        NextTarget::DismissAndResolveExternally
    );
}

#[test]
fn a_refusal_with_no_surface_context_has_no_fabricated_gloss() {
    let err = StikkError::Refusal {
        message: "some refusal".into(),
    };
    match present(&err, OperationContext::Other) {
        Presentation::RefusalOverlay(card) => assert!(card.gloss.is_none()), // RR-5: verbatim-only
        other => panic!("expected overlay, got {other:?}"),
    }
}

#[test]
fn stale_becomes_its_own_presentation_naming_the_operation_with_exactly_one_re_preview_next_step() {
    // RFC 013 §5/decision 3: routed to a re-preview prompt, and the next-step set must contain no
    // action that re-runs the execution — this is the NFR-S04 regression test for this increment.
    let err = StikkError::Stale {
        operation: "commit".into(),
    };
    let (operation, gloss, next_steps) = match present(&err, OperationContext::Other) {
        Presentation::Stale {
            operation,
            gloss,
            next_steps,
        } => (operation, gloss, next_steps),
        other => panic!("expected Presentation::Stale, got {other:?}"),
    };
    assert_eq!(operation, "commit");
    assert!(!gloss.is_empty());
    assert_eq!(next_steps.len(), 1);
    for step in &next_steps {
        // Every next-step must be navigational (NFR-S04): none may re-run an execution. This
        // increment's `NextTarget` vocabulary has no "execute" variant at all, so the assertion is
        // that the one step present is `Refresh` (re-run the *preview*, a read) — never anything else.
        assert_eq!(step.target, NextTarget::Refresh);
    }
}

#[test]
fn stale_is_never_a_refusal_overlay_design_review_c1() {
    // design-review C1 (RFC 013 v1): `Stale` must never be rendered under a label asserting prikk
    // said it — the fix is that it cannot even reach `RefusalOverlay`'s match arm, structurally.
    let err = StikkError::Stale {
        operation: "seal".into(),
    };
    match present(&err, OperationContext::Other) {
        Presentation::Stale { operation, .. } => assert_eq!(operation, "seal"),
        other => panic!("expected Presentation::Stale, not {other:?}"),
    }
}

#[test]
fn declined_routes_to_in_confirmation_not_a_separate_popup() {
    // RFC 013 §4: wrong/empty confirmation evidence belongs inside the confirmation surface that asked
    // for it, not a new overlay — the user is still mid-confirmation, not facing an unrelated failure.
    let err = StikkError::Declined {
        detail: "typed name does not match".into(),
    };
    match present(&err, OperationContext::Other) {
        Presentation::InConfirmation { message } => {
            assert_eq!(message, "typed name does not match");
        }
        other => panic!("expected InConfirmation, got {other:?}"),
    }
}

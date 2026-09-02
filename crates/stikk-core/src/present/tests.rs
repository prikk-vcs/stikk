//! Tests for the class → presentation mapping (design TS-05; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used)]

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
fn a_refusal_with_no_surface_context_has_no_fabricated_gloss() {
    let err = StikkError::Refusal {
        message: "some refusal".into(),
    };
    match present(&err, OperationContext::Other) {
        Presentation::RefusalOverlay(card) => assert!(card.gloss.is_none()), // RR-5: verbatim-only
        other => panic!("expected overlay, got {other:?}"),
    }
}

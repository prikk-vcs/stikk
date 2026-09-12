//! Tests for the class → presentation mapping (design TS-05; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::StikkError;

use super::*;

#[test]
fn a_refusal_becomes_an_overlay_with_the_verbatim_message() {
    let err = StikkError::Refusal {
        message: "ref \"heads/nope\" does not exist".into(),
    };
    match present(&err, OperationContext::LoadHistory, None) {
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
    let card = match present(&hostile, OperationContext::LoadHistory, None) {
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
    match present(&err, OperationContext::Orient, None) {
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
    match present(&err, OperationContext::Other, None) {
        Presentation::InlineGuidance { toward, .. } => assert_eq!(toward, Target::TrustKeys),
        other => panic!("expected InlineGuidance, got {other:?}"),
    }
}

/// The acceptance-critical fix (review v2, C1): v1 appended the trust-refusal gloss to
/// `InlineGuidance`'s `detail`, which `shell.rs::render_banner` shows in a single, non-wrapping row —
/// off-screen at every realistic terminal width, unnoticed because the old test asserted on
/// `app.banner()`'s `String`, never on what a `TestBackend` cell actually shows. This is now a
/// `RefusalOverlay`, whose renderer already wraps and keeps `verbatim`/`gloss` visually distinct — see
/// `crates/stikk-tui/src/overlay/tests.rs`'s `trust_refusal_gloss_is_reachable_at_80_columns` for the
/// render-level proof this fix asked for.
#[test]
fn a_trust_refusal_gets_a_refusal_overlay_not_a_cramped_banner() {
    // RFC 016 §9 / RFC 017 F5's other half: both captured wordings must get the same treatment.
    for message in [
        "invalid signature: maintainer signer key id different-maintainer is not trusted by policy",
        "invalid signature: maintainer signer public key does not match trusted key maintainer",
    ] {
        let err = StikkError::NotReady {
            detail: message.to_string(),
        };
        let card = match present(&err, OperationContext::Other, None) {
            Presentation::RefusalOverlay(card) => card,
            other => panic!("expected RefusalOverlay, got {other:?}"),
        };
        assert_eq!(card.verbatim, message); // ER-02: prikk's verbatim words, untouched
        let gloss = card
            .gloss
            .expect("a trust refusal must get adoption guidance");
        assert!(gloss.contains("adopted"));
        assert!(gloss.contains("object trust"));
        assert!(!gloss.to_ascii_lowercase().contains("may publish"));
        assert!(gloss.contains("cannot verify"));
        assert!(
            card.glossary_codes
                .contains(&"maintainer signer".to_string())
        );
        assert_eq!(card.next_steps.len(), 1);
        assert_eq!(card.next_steps[0].target, NextTarget::Refresh);
    }
}

#[test]
fn an_absent_signing_key_stays_inline_guidance_not_a_refusal_overlay() {
    // The narrow-match discipline (C-T2b): only the two captured trust-refusal clauses get the
    // overlay treatment — a plain absent-key message must still take the ordinary, short-banner path.
    let err = StikkError::NotReady {
        detail: "author signing is required: set PRIKK_AUTHOR_KEY_ID (no signing key configured)"
            .to_string(),
    };
    match present(&err, OperationContext::Other, None) {
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
    match present(&err, OperationContext::LoadChanges, None) {
        Presentation::InlineGuidance { toward, detail, .. } => {
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
        match present(&err, op, None) {
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
    match present(&err, OperationContext::Orient, None) {
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
        present(&err, OperationContext::Other, None),
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
    let card = match present(&err, OperationContext::LoadChanges, None) {
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
    let card = match present(&err, OperationContext::LoadChanges, None) {
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
        let card = match present(&err, op, None) {
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
fn a_bundle_decode_skew_refusal_glosses_and_offers_upgrade() {
    // RFC 015 F5 — captured live: a real prikk 0.31.1 verifying a bundle a real prikk 0.32.0 exported.
    let err = StikkError::Refusal {
        message:
            "malformed persisted data: invalid PatchPurpose canonical form: canonical encoding \
                  error: unknown PatchPayload field tag: 6"
                .into(),
    };
    let card = match present(&err, OperationContext::Other, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert!(
        card.gloss
            .as_deref()
            .is_some_and(|g| g.contains("newer prikk"))
    );
    assert!(card.gloss.as_deref().is_some_and(|g| g.contains("bundle")));
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
            .contains(&"canonical encoding error: unknown".to_string())
    );
}

#[test]
fn a_bundle_decode_skew_refusal_is_distinct_from_the_repository_level_shape() {
    // The two shapes must not collide: a repository-level schema-skew message must not trigger the
    // bundle-decode gloss, and vice versa.
    let repo_level = StikkError::Refusal {
        message: "integrity error: format-2 patch does not accept envelope schema 4 (accepted: [1, 2, 3])"
            .into(),
    };
    let card = match present(&repo_level, OperationContext::Other, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected overlay, got {other:?}"),
    };
    assert!(card.gloss.as_deref().is_some_and(|g| !g.contains("bundle")));
}

#[test]
fn the_full_queue_precondition_gets_an_honest_gloss_never_another_writer() {
    // RFC 017 F4, the acceptance-critical fix: this message reaches `present()` as a `Refusal` (the
    // classifier no longer routes it through `LockConflict`), and the gloss must not claim a writer
    // stikk never saw — the exact contradiction that shipped on the commit path. Captured live (see
    // `classify/tests.rs`).
    let err = StikkError::Refusal {
        message:
            "lock conflict: active WAL has 1 queued patches, at or above the configured limit \
                  (1); run `prikk seal` before committing again"
                .to_string(),
    };
    let card = match present(&err, OperationContext::Commit, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    // The contradiction is gone: the gloss no longer *claims* a writer is active — it says the opposite
    // (Nothing is locked and no other writer is involved), beside prikk's own words about the queue.
    assert!(
        !card
            .gloss
            .as_deref()
            .unwrap_or_default()
            .contains("another writer is active")
    );
    assert!(
        card.gloss
            .as_deref()
            .is_some_and(|g| g.contains("Nothing is locked"))
    );
    assert!(card.verbatim.contains("run `prikk seal`"));
    // RFC 016 §10 closes the gap RFC 017 left open: a seal-ceremony next-step now exists alongside
    // Refresh — never a retry of this refusal itself (NFR-S04 is intact; sealing is its own ceremony).
    assert_eq!(card.next_steps.len(), 2);
    assert_eq!(
        card.next_steps[0].target,
        NextTarget::OpenView(Target::Seal)
    );
    assert_eq!(card.next_steps[1].target, NextTarget::Refresh);
}

#[test]
fn the_active_rs_full_queue_wording_gets_the_same_gloss() {
    let err = StikkError::Refusal {
        message:
            "lock conflict: active WAL has 64 queued patches, at or above the configured limit \
                  (64); run doctor or seal before appending again"
                .to_string(),
    };
    let card = match present(&err, OperationContext::Other, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    assert!(
        !card
            .gloss
            .as_deref()
            .unwrap_or_default()
            .contains("another writer is active")
    );
    assert!(
        card.gloss
            .as_deref()
            .is_some_and(|g| g.contains("Nothing is locked"))
    );
    assert_eq!(card.next_steps.len(), 2);
    assert_eq!(
        card.next_steps[0].target,
        NextTarget::OpenView(Target::Seal)
    );
}

#[test]
fn a_genuine_lock_conflict_still_gets_fr_106s_ordinary_banner() {
    // Unaffected by RFC 017: a real held lock still routes through `LockConflict`, never `Refusal`.
    let err = StikkError::LockConflict {
        message: "active lock already exists: /repo/.prikk/active/default/active.lock".to_string(),
    };
    match present(&err, OperationContext::Other, None) {
        Presentation::Banner { message, .. } => assert!(message.contains("already exists")),
        other => panic!("expected Banner, got {other:?}"),
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
    let card = match present(&err, OperationContext::LoadChanges, None) {
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
    let card = match present(&hostile, OperationContext::Orient, None) {
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
    match present(&err, OperationContext::Other, None) {
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
    let (operation, gloss, next_steps) = match present(&err, OperationContext::Other, None) {
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
    match present(&err, OperationContext::Other, None) {
        Presentation::Stale { operation, .. } => assert_eq!(operation, "seal"),
        other => panic!("expected Presentation::Stale, not {other:?}"),
    }
}

#[test]
fn cross_ref_becomes_a_refusal_overlay_never_a_lock_conflict_shape() {
    // RFC 014 F2/decision 2: prikk's own words, so RefusalOverlay is correct here (unlike Stale) — but
    // this is its own class, distinct from LockConflict, so a future lock-inspector jump never attaches.
    let err = StikkError::CrossRef {
        message: "lock conflict: active WAL is owned by heads/main; requested ref heads/other"
            .into(),
    };
    let card = match present(&err, OperationContext::Commit, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    assert!(card.verbatim.contains("active WAL is owned by"));
    assert!(card.gloss.as_deref().is_some_and(|g| g.contains("Seal")));
    let labels: Vec<&str> = card.next_steps.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Choose another ref", "Refresh"]);
}

#[test]
fn a_commit_refusal_offers_back_to_changes_and_refresh() {
    let err = StikkError::Refusal {
        message: "invalid name: worktree has no node-addressed changes to commit".into(),
    };
    let card = match present(&err, OperationContext::Commit, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    let labels: Vec<&str> = card.next_steps.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Back to Changes", "Refresh"]);
}

#[test]
fn declined_routes_to_in_confirmation_not_a_separate_popup() {
    // RFC 013 §4: wrong/empty confirmation evidence belongs inside the confirmation surface that asked
    // for it, not a new overlay — the user is still mid-confirmation, not facing an unrelated failure.
    let err = StikkError::Declined {
        detail: "typed name does not match".into(),
    };
    match present(&err, OperationContext::Other, None) {
        Presentation::InConfirmation { message } => {
            assert_eq!(message, "typed name does not match");
        }
        other => panic!("expected InConfirmation, got {other:?}"),
    }
}

/// RFC 023 F1 — the captured Windows-floor refusal reaches a gloss, and prikk's words survive beside it.
///
/// **The message is captured, not composed here**: it is exactly what the Windows leg of RFC 022's
/// first widened matrix run (`34675099061`) produced from a real prikk 0.28, where `block_state`'s
/// subdirectory commit was refused by prikk's own path validator on a path prikk itself had built.
const CAPTURED_BACKSLASH_REFUSAL_0_28_WINDOWS: &str =
    "error: invalid name: backslashes are not allowed in repository paths";

#[test]
fn the_captured_backslash_refusal_gets_a_gloss_and_keeps_prikks_words() {
    let err = StikkError::Refusal {
        message: CAPTURED_BACKSLASH_REFUSAL_0_28_WINDOWS.to_string(),
    };
    let card = match present(&err, OperationContext::Commit, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    let gloss = card
        .gloss
        .as_deref()
        .expect("the captured message must reach a gloss");
    // `ER-02`: the gloss is additive. prikk's sentence is still there, unedited, whatever stikk adds.
    assert_eq!(card.verbatim, CAPTURED_BACKSLASH_REFUSAL_0_28_WINDOWS);
    // The refusal card's `glossary:` line names the code, so the Glossary overlay (F2) can explain it.
    assert!(
        card.glossary_codes
            .iter()
            .any(|c| c == "backslashes are not allowed in repository paths"),
        "the code must be linked from the card: {:?}",
        card.glossary_codes
    );
    // Exactly one next-step, and it is guidance the user resolves outside stikk (`CON-1`) — never a
    // retry of the refusal itself.
    assert_eq!(card.next_steps.len(), 1);
    assert_eq!(
        card.next_steps[0].target,
        NextTarget::DismissAndResolveExternally
    );

    // Whichever platform this build is, the card carries that platform's advice.
    let expected = backslash_path_advice(cfg!(windows), None);
    assert_eq!(gloss, expected.gloss);
    assert_eq!(card.next_steps[0].label, expected.next_step);
}

/// **Both** backslash glosses, asserted on every platform.
///
/// `ci.yml` runs `cargo test --workspace` on ubuntu only, so a `cfg!(windows)` assertion is an
/// assertion that never runs — and the Windows branch is the entire point of RFC 023 F1. Taking the
/// platform as an argument is what makes this testable at all; see `backslash_path_advice`.
#[test]
fn both_backslash_glosses_say_the_true_thing_for_their_platform() {
    // **`Some(28)` now, not `None`.** RFC 026 C narrowed this by version as well as platform, so the
    // Windows-defect gloss is the one shown to a user actually on 0.28; `None` gets the hedged
    // variant, which its own test covers.
    let windows = backslash_path_advice(true, Some(28));
    // The point of the gloss: the user may have typed no backslash, and the fix is a prikk version.
    assert!(
        windows.gloss.contains("typed no backslash"),
        "{}",
        windows.gloss
    );
    assert!(windows.gloss.contains("0.29.0"), "{}", windows.gloss);
    // It must NOT overstate the limitation — RFC 023 §2's explicit warning. A top-level file commits
    // fine at 0.28 on Windows; RFC 022's suite skips only the subdirectory half of one test for exactly
    // that reason, and a gloss claiming commits are impossible would be the same wrong-picture failure
    // pointing the other way.
    assert!(
        windows
            .gloss
            .contains("top level of the worktree commits normally"),
        "{}",
        windows.gloss
    );
    assert!(
        !windows.gloss.contains("cannot commit"),
        "the gloss must not claim commits are impossible: {}",
        windows.gloss
    );
    assert!(windows.next_step.contains("Upgrade prikk"));

    // Off Windows the same prikk message has the other cause entirely, and telling that user to upgrade
    // prikk would be a stikk-authored claim contradicting the evidence beside it — the failure RFC 017
    // F4 fixed. prikk 0.28's defect is the platform separator leaking into a repository path; it cannot
    // happen where the separator is already `/`.
    let unix = backslash_path_advice(false, Some(28));
    assert!(
        unix.gloss.contains("has a backslash in its name"),
        "{}",
        unix.gloss
    );
    assert!(
        !unix.gloss.contains("0.29"),
        "off Windows this is not a prikk version problem: {}",
        unix.gloss
    );
    assert!(unix.next_step.contains("Rename"));

    // And they are actually different — a refactor that collapsed them would otherwise pass everything
    // above on one platform.
    assert_ne!(windows.gloss, unix.gloss);
    assert_ne!(windows.next_step, unix.next_step);
}

#[test]
fn an_ordinary_invalid_name_refusal_is_untouched_by_the_backslash_gloss() {
    // The code is the backslash clause, never prikk's shared `invalid name:` prefix — which every other
    // name refusal carries too. Matching the prefix would put a Windows-and-0.28 story over refusals
    // that have nothing to do with either.
    let err = StikkError::Refusal {
        message: "error: invalid name: ref names may not end with .lock".to_string(),
    };
    let card = match present(&err, OperationContext::Commit, None) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    assert!(
        !card
            .gloss
            .as_deref()
            .unwrap_or_default()
            .contains("backslash"),
        "an unrelated invalid-name refusal must not get the backslash gloss: {card:?}"
    );
    assert!(card.glossary_codes.is_empty());
}

// --- RFC 026 Handoff C §2: the gloss narrowed by platform and version -------------------------
//
// Hermetic, the way `paths.rs` tests its platform enum rather than the real platform: the two facts
// are arguments, so all three cases run on every machine.

/// Not Windows: prikk 0.28's defect is the **platform separator** leaking into a repository path, and
/// it cannot happen where the separator is already `/`. One explanation, whatever prikk is running.
#[test]
fn off_windows_only_the_real_backslash_explanation_is_shown() {
    for minor in [None, Some(28), Some(41)] {
        let advice = backslash_path_advice(false, minor);
        assert!(
            advice.gloss.contains("has a backslash in its name"),
            "{minor:?}: {}",
            advice.gloss
        );
        assert!(
            !advice.gloss.contains("0.29.0"),
            "{minor:?}: off Windows this is not a prikk version problem: {}",
            advice.gloss
        );
        assert!(advice.next_step.contains("Rename"));
    }
}

/// Windows on prikk ≥ 0.29: prikk fixed the separator defect in 0.29.0, so only the other cause
/// remains. **This is the narrowing** — before it, this user read a paragraph about a version they
/// are not running and had to work out that it was not theirs.
#[test]
fn on_windows_above_the_fix_the_version_half_is_dropped() {
    for minor in [29, 38, 41] {
        let advice = backslash_path_advice(true, Some(minor));
        assert!(
            advice.gloss.contains("has a backslash in its name"),
            "0.{minor}: {}",
            advice.gloss
        );
        assert!(
            !advice.gloss.contains("typed no backslash"),
            "0.{minor}: prikk fixed this in 0.29.0, so the defect half does not apply: {}",
            advice.gloss
        );
        assert!(advice.next_step.contains("Rename"), "0.{minor}");
    }
}

/// Windows on prikk 0.28 exactly: **both remain genuinely possible**, so both are shown, defect
/// first. RFC 023 F1 measured that this message is prikk's generic validator — a lone "upgrade prikk"
/// would contradict the evidence for a user whose file really is named with a backslash.
#[test]
fn on_windows_at_0_28_both_halves_stay() {
    let advice = backslash_path_advice(true, Some(28));
    assert!(
        advice.gloss.contains("typed no backslash"),
        "{}",
        advice.gloss
    );
    assert!(advice.gloss.contains("0.29.0"), "{}", advice.gloss);
    // And the other cause is still named, second.
    assert!(
        advice.gloss.contains("backslash in its name"),
        "both halves, in that order: {}",
        advice.gloss
    );
    assert!(advice.next_step.contains("Upgrade prikk"));
}

/// An unknown version on Windows keeps both, because it cannot exclude 0.28 — the direction that
/// keeps an explanation rather than hiding it from the user it was written for.
#[test]
fn on_windows_with_an_unknown_version_both_halves_stay() {
    let unknown = backslash_path_advice(true, None);
    assert!(
        unknown
            .gloss
            .contains("If this repository is on prikk 0.28"),
        "{}",
        unknown.gloss
    );
    assert!(
        unknown.gloss.contains("backslash in its name"),
        "{}",
        unknown.gloss
    );
    // **The hedge belongs here and only here.** Where stikk knows the version it asserts instead —
    // hedging about a fact stikk holds is the defect this narrowing closes.
    let at_28 = backslash_path_advice(true, Some(28));
    assert!(
        !at_28.gloss.contains("If this repository is on"),
        "{}",
        at_28.gloss
    );
    assert!(
        at_28.gloss.contains("This prikk builds repository paths"),
        "{}",
        at_28.gloss
    );
}

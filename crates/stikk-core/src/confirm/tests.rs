//! Tests for the preview→confirm→execute gate (design `FR-120`/`FR-121`, `OPL-01…05`; RFC 013).
//!
//! This is a safety gate, so the negatives are tested hardest (handoff §7). The "scripted, non-mutating
//! operation exercising every tier" the handoff asks for as the test vehicle is [`scripted_op`] below —
//! a trivial `compute`/`run` pair with no seam mutation behind it, since none exists yet.
//!
//! **Structural guarantees, stated rather than tested** (handoff §7: "a test cannot prove this; the
//! type signatures can"): [`execute`] takes a [`ConfirmedToken`] by value, so a used token cannot be
//! passed again — the borrow checker refuses the second call to compile, not a runtime check. Neither
//! [`PreviewToken`] nor [`ConfirmedToken`] has a public constructor or field, and there is no
//! `From`/`Into` between them — `preview`/`confirm` are their only producers. Reading this file's
//! `use` list and the absence of any `PreviewToken { .. }`/`ConfirmedToken { .. }` literal anywhere in
//! it (including here, in the crate that defines them) is the evidence: even these tests build tokens
//! only by calling `preview`/`confirm`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use stikk_model::{Capability, ChangeToken, Readiness, RequestCategory, StikkError};
use stikk_prikk::NullBackend;

use super::*;

fn intent(category: RequestCategory) -> Intent {
    Intent {
        category,
        operation: "test-op",
    }
}

fn summary(target_name: Option<&str>) -> ConfirmationSummary {
    ConfirmationSummary {
        operation: "Test operation".to_string(),
        target_ids: vec!["abc123".to_string()],
        counts: vec![("items", 1)],
        capability: Capability::Viewer,
        consequence: "Nothing real happens — this is the scripted test vehicle".to_string(),
        target_name: target_name.map(str::to_string),
    }
}

/// A scripted, non-mutating "operation": its preview computes a fixed view + summary, and its
/// execution just echoes a marker — standing in for a real seam mutation, which does not exist this
/// increment.
fn scripted_op(
    backend: &NullBackend,
    category: RequestCategory,
    target_name: Option<&str>,
) -> Result<(&'static str, PreviewToken)> {
    preview(backend, Path::new("/r"), intent(category), || {
        Ok(("scripted preview view", summary(target_name)))
    })
}

fn backend_with_token(seed: &str) -> NullBackend {
    NullBackend::supported().with_change_token(ChangeToken::compose(
        [("heads/main", seed)],
        0,
        None,
    ))
}

fn ready(author: bool, maintainer: bool, read_only: bool) -> Readiness {
    Readiness {
        author_ready: author,
        maintainer_ready: maintainer,
        read_only,
    }
}

#[test]
fn preview_stamps_the_current_change_token_and_derives_the_tier() {
    let backend = backend_with_token("a");
    let (view, token) = scripted_op(&backend, RequestCategory::QueueMutation, None).unwrap();
    assert_eq!(view, "scripted preview view");
    assert_eq!(token.tier(), Tier::Two);
    assert_eq!(token.summary().operation, "Test operation");
}

#[test]
fn confirm_succeeds_when_nothing_moved() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::QueueMutation, None).unwrap();
    let readiness = ready(true, false, false); // AUTHOR present — enough for tier 2
    confirm(
        &backend,
        Path::new("/r"),
        token,
        readiness,
        Evidence::ExplicitYes,
    )
    .expect("confirm succeeds when the token is unchanged and evidence is correct");
}

#[test]
fn confirm_refuses_when_the_change_token_moved_between_preview_and_confirm() {
    let at_preview_time = backend_with_token("a");
    let (_, token) = scripted_op(&at_preview_time, RequestCategory::QueueMutation, None).unwrap();

    let at_confirm_time = backend_with_token("b"); // a different repository state
    let err = confirm(
        &at_confirm_time,
        Path::new("/r"),
        token,
        ready(true, false, false),
        Evidence::ExplicitYes,
    )
    .expect_err("must refuse — the world moved");
    assert_eq!(err.class(), "stale");
    assert!(matches!(err, StikkError::Stale { operation } if operation == "test-op"));
}

#[test]
fn execute_refuses_when_the_change_token_moved_between_confirm_and_execute() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::QueueMutation, None).unwrap();
    let confirmed = confirm(
        &backend,
        Path::new("/r"),
        token,
        ready(true, false, false),
        Evidence::ExplicitYes,
    )
    .expect("confirm succeeds");

    // The world moves after confirmation, before execution — a different backend stands in for it.
    let at_execute_time = backend_with_token("b");
    let err = execute(&at_execute_time, Path::new("/r"), confirmed, || Ok(()))
        .expect_err("must refuse — the world moved again");
    assert_eq!(err.class(), "stale");
}

#[test]
fn execute_succeeds_and_carries_the_operation_name_when_nothing_moved() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::QueueMutation, None).unwrap();
    let confirmed = confirm(
        &backend,
        Path::new("/r"),
        token,
        ready(true, false, false),
        Evidence::ExplicitYes,
    )
    .expect("confirm succeeds");
    let outcome = execute(&backend, Path::new("/r"), confirmed, || Ok(7))
        .expect("execute succeeds when nothing moved");
    assert_eq!(outcome.operation, "test-op");
    assert_eq!(outcome.result, 7);
}

#[test]
fn read_only_refuses_tier_two_and_tier_three_with_keys_present() {
    let backend = backend_with_token("a");
    for category in [RequestCategory::QueueMutation, RequestCategory::Publication] {
        let (_, token) = scripted_op(&backend, category, None).unwrap();
        let fully_keyed_but_read_only = ready(true, true, true);
        let err = confirm(
            &backend,
            Path::new("/r"),
            token,
            fully_keyed_but_read_only,
            Evidence::ExplicitYes,
        )
        .expect_err("read-only must refuse even with every key present");
        assert_eq!(err.class(), "not-ready");
    }
}

#[test]
fn read_only_refuses_tier_two_and_tier_three_without_keys_either() {
    let backend = backend_with_token("a");
    for category in [RequestCategory::QueueMutation, RequestCategory::Publication] {
        let (_, token) = scripted_op(&backend, category, None).unwrap();
        let err = confirm(
            &backend,
            Path::new("/r"),
            token,
            ready(false, false, true),
            Evidence::ExplicitYes,
        )
        .expect_err("read-only must refuse regardless of key presence");
        assert_eq!(err.class(), "not-ready");
    }
}

#[test]
fn tier_one_is_never_blocked_by_read_only() {
    // Tier 1 never reaches `confirm` in real use (it has no preview step) — exercised directly against
    // `capability_gate` to prove the gate itself imposes no restriction at this tier.
    assert!(capability_gate("read", Tier::One, ready(false, false, true)).is_ok());
}

#[test]
fn tier_two_refuses_at_viewer_capability() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::QueueMutation, None).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(), // Viewer: no keys, not read-only
        Evidence::ExplicitYes,
    )
    .expect_err("Viewer must not satisfy tier 2");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn tier_three_refuses_at_author_capability() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Publication, None).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        ready(true, false, false), // Author, not Maintainer
        Evidence::ExplicitYes,
    )
    .expect_err("Author must not satisfy tier 3");
    assert_eq!(err.class(), "not-ready");
}

#[test]
fn tier_three_succeeds_at_maintainer_capability() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Publication, None).unwrap();
    confirm(
        &backend,
        Path::new("/r"),
        token,
        ready(true, true, false),
        Evidence::ExplicitYes,
    )
    .expect("Maintainer satisfies tier 3");
}

#[test]
fn tier_three_typed_needs_no_signing_capability_only_read_write() {
    // AC-04: Operator is orthogonal to the signing ladder. Viewer-level readiness (no keys) still
    // satisfies tier 3-typed's capability gate, as long as the session is not read-only.
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Trust, Some("my-key")).unwrap();
    confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::TypedName("my-key".to_string()),
    )
    .expect("no signing readiness is required for tier 3-typed, only read-write");
}

#[test]
fn tier_three_typed_exact_match_passes() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Recovery, Some("heads/main")).unwrap();
    confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::TypedName("heads/main".to_string()),
    )
    .expect("an exact match must pass");
}

#[test]
fn tier_three_typed_a_wrong_name_refuses() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Recovery, Some("heads/main")).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::TypedName("heads/other".to_string()),
    )
    .expect_err("a wrong name must refuse");
    assert_eq!(err.class(), "declined");
}

#[test]
fn tier_three_typed_a_near_miss_refuses() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Recovery, Some("heads/main")).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::TypedName("heads/mai".to_string()), // one character short
    )
    .expect_err("a near-miss must refuse — exact match only");
    assert_eq!(err.class(), "declined");
}

#[test]
fn tier_three_typed_empty_input_refuses() {
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Recovery, Some("heads/main")).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::TypedName(String::new()),
    )
    .expect_err("empty input must refuse");
    assert_eq!(err.class(), "declined");
}

#[test]
fn wrong_evidence_shape_for_the_tier_also_refuses() {
    // An explicit yes offered where a typed name is required (or vice versa) must not accidentally
    // satisfy the tier — the match is exhaustive on (tier, evidence shape), not just the tier.
    let backend = backend_with_token("a");
    let (_, token) = scripted_op(&backend, RequestCategory::Recovery, Some("heads/main")).unwrap();
    let err = confirm(
        &backend,
        Path::new("/r"),
        token,
        Readiness::none(),
        Evidence::ExplicitYes,
    )
    .expect_err("an explicit yes must not satisfy a tier-3-typed requirement");
    assert_eq!(err.class(), "declined");

    let backend2 = backend_with_token("a");
    let (_, token2) = scripted_op(&backend2, RequestCategory::QueueMutation, None).unwrap();
    let err2 = confirm(
        &backend2,
        Path::new("/r"),
        token2,
        ready(true, false, false),
        Evidence::TypedName("anything".to_string()),
    )
    .expect_err("a typed name must not satisfy a tier-2 requirement");
    assert_eq!(err2.class(), "declined");
}

//! Tests for the change-token operation-layer bridge (design LC-4/OP-04; RFC 003).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use stikk_model::ChangeToken;
use stikk_prikk::NullBackend;

use super::*;

fn token(seed: &str) -> ChangeToken {
    ChangeToken::compose([("heads/main", seed)], 0, None)
}

#[test]
fn change_token_passes_through_the_seam() {
    let expected = token(&"a".repeat(64));
    let backend = NullBackend::supported().with_change_token(expected);
    let actual = change_token(&backend, Path::new("/r")).expect("succeeds");
    assert_eq!(actual, expected);
}

#[test]
fn a_change_token_refusal_propagates() {
    let backend = NullBackend::supported().with_change_token_refusal("prikk refused");
    let err = change_token(&backend, Path::new("/r")).unwrap_err();
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("prikk refused"));
}

#[test]
fn an_unchanged_token_produces_no_notice() {
    let t = token(&"a".repeat(64));
    assert!(staleness_notice(&t, &t).is_none());
}

#[test]
fn a_changed_token_produces_the_op_04_banner_verbatim() {
    let before = token(&"a".repeat(64));
    let after = token(&"b".repeat(64));
    match staleness_notice(&before, &after) {
        Some(Presentation::Banner { message, jump }) => {
            assert_eq!(message, "repository changed outside stikk — refreshed");
            assert!(jump.is_none());
        }
        other => panic!("expected a Banner, got {other:?}"),
    }
}

#[test]
fn staleness_is_order_sensitive_only_in_which_argument_not_in_the_result() {
    // The notice is the same regardless of which token is "previous" vs "current" — the comparison is
    // symmetric equality, not a directional diff.
    let a = token(&"a".repeat(64));
    let b = token(&"b".repeat(64));
    assert_eq!(staleness_notice(&a, &b), staleness_notice(&b, &a));
}

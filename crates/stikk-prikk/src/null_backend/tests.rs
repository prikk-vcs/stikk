//! Tests for the scripted backend (design TS-02).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

#[test]
fn supported_backend_reports_a_clean_repo() {
    let backend = NullBackend::supported();
    let hs = backend.handshake().unwrap();
    assert!(hs.supported);
    assert!(hs.validated);
    assert_eq!(hs.version.minor, 30); // RFC 009: the default is now the validated ceiling
    let o = backend.orientation(Path::new("/anywhere")).unwrap();
    assert_eq!(o.queued_patches, 0);
}

#[test]
fn refusal_orientation_surfaces_verbatim() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let err = backend.orientation(Path::new("/x")).unwrap_err();
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("retired format 3"));
}

#[test]
fn unsupported_version_is_reported() {
    let backend = NullBackend::supported().unsupported();
    assert!(!backend.handshake().unwrap().supported);
}

#[test]
fn with_version_recomputes_both_supported_and_validated() {
    let below_floor = NullBackend::supported().with_version(0, 27, 1);
    let hs = below_floor.handshake().unwrap();
    assert!(!hs.supported && !hs.validated);

    // RFC 012 F-e raised the validated ceiling to 0.31; the above-ceiling case moves with it.
    let above_ceiling = NullBackend::supported().with_version(0, 32, 0);
    let hs = above_ceiling.handshake().unwrap();
    assert!(hs.supported && !hs.validated);
}

#[test]
fn with_change_token_scripts_an_arbitrary_token() {
    // RFC 003: independent of this backend's own refs/orientation fields — a script may want a token
    // with no matching ref/orientation state, since only equality between two tokens is load-bearing.
    let a = stikk_model::ChangeToken::compose([("heads/main", "a".repeat(64).as_str())], 0, None);
    let b = stikk_model::ChangeToken::compose([("heads/main", "b".repeat(64).as_str())], 0, None);
    assert_ne!(a, b);

    let backend = NullBackend::supported().with_change_token(a);
    assert_eq!(backend.change_token(Path::new("/x")).unwrap(), a);
    assert_ne!(backend.change_token(Path::new("/x")).unwrap(), b);
}

#[test]
fn with_queued_elsewhere_sets_the_note_on_the_scripted_status() {
    let backend = NullBackend::supported().with_queued_elsewhere("note: queued elsewhere");
    let status = backend
        .worktree_status(Path::new("/x"), "heads/main")
        .unwrap();
    assert_eq!(
        status.queued_elsewhere.as_deref(),
        Some("note: queued elsewhere")
    );
}

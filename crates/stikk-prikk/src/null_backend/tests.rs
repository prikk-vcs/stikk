//! Tests for the scripted backend (design TS-02).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

#[test]
fn supported_backend_reports_a_clean_repo() {
    let backend = NullBackend::supported();
    let hs = backend.handshake().unwrap();
    assert!(hs.supported);
    assert_eq!(hs.version.minor, 27);
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

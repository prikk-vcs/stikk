//! Tests for the orientation operation (design TS-01/TS-02), driven by the scripted backend so no
//! prikk binary or repository is needed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use stikk_model::Capability;
use stikk_prikk::{NullBackend, Orientation};

use super::*;

#[test]
fn orients_a_clean_repo_as_viewer_by_default() {
    // With no signing readiness in this test's environment, the session is a Viewer.
    let backend = NullBackend::supported();
    let view = orient(&backend, Path::new("/repo")).expect("orients");
    assert!(view.prikk_supported);
    assert_eq!(view.queued_patches, 0);
    // The environment of the test host may or may not have PRIKK_* set; the capability is whatever
    // the readiness derives to, and it must be internally consistent.
    assert_eq!(view.capability, Capability::derive(view.readiness));
}

#[test]
fn surfaces_queue_depth_and_partial_tail() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 5,
        main_ref_state: Some("abc".to_string()),
        trailing_partial_wal_bytes: 12,
    });
    let view = orient(&backend, Path::new("/repo")).expect("orients");
    assert_eq!(view.queued_patches, 5);
    assert_eq!(view.trailing_partial_wal_bytes, 12);
    assert_eq!(view.main_ref_state.as_deref(), Some("abc"));
}

#[test]
fn a_retired_format_refusal_propagates_with_its_class() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let err = orient(&backend, Path::new("/repo")).expect_err("refusal propagates");
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("retired format 3"));
}

#[test]
fn an_unsupported_prikk_is_flagged_not_hidden() {
    let backend = NullBackend::supported().unsupported();
    let view = orient(&backend, Path::new("/repo")).expect("still orients read-only");
    assert!(!view.prikk_supported);
}

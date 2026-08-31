//! Tests for capability derivation (design `stikk-04` AC-01…04, NFR-S01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn no_readiness_is_viewer() {
    assert_eq!(Capability::derive(Readiness::none()), Capability::Viewer);
}

#[test]
fn author_readiness_grants_author() {
    let r = Readiness {
        author_ready: true,
        maintainer_ready: false,
        read_only: false,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Author);
    assert!(cap.may_author());
    assert!(!cap.may_publish());
}

#[test]
fn maintainer_readiness_grants_maintainer_and_implies_author() {
    let r = Readiness {
        author_ready: true,
        maintainer_ready: true,
        read_only: false,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Maintainer);
    assert!(cap.may_author());
    assert!(cap.may_publish());
}

#[test]
fn read_only_collapses_everything_to_viewer() {
    // NFR-S01: read-only mode wins over any key presence.
    let r = Readiness {
        author_ready: true,
        maintainer_ready: true,
        read_only: true,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Viewer);
    assert!(!cap.may_author());
    assert!(!cap.may_publish());
}

#[test]
fn operator_actions_are_available_at_every_level() {
    // AC-04: recovery is orthogonal to the mutating axis, but each action still confirms.
    assert!(Capability::Viewer.may_operate());
    assert!(Capability::Maintainer.may_operate());
}

#[test]
fn readiness_holds_no_secret_only_flags() {
    // Structural guarantee (LC-13): the type is three bools; there is nowhere for key material.
    // This test documents the invariant; the compiler enforces the shape.
    let r = Readiness::none();
    assert!(!r.author_ready && !r.maintainer_ready && !r.read_only);
}

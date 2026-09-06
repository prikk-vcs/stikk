//! Tests for capability derivation (design `stikk-04` AC-01…04, NFR-S01; RFC 016 Q1).

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
        maintainer_readiness: MaintainerReadiness::NotReady,
        read_only: false,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Author);
    assert!(cap.may_author());
    assert!(!cap.may_publish());
}

#[test]
fn maintainer_unknown_grants_maintainer_and_implies_author() {
    // RFC 016 Q1: the affordance is offered on `Unknown` too — hiding seal from someone whose key
    // genuinely is adopted would be its own confident-but-wrong picture (C-T4d). `Ready` itself is
    // unconstructible today (no supported prikk can answer the adoption question), so `Unknown` is the
    // only reachable value this test can exercise for real — see `maintainer_ready_grants_maintainer`
    // for the (currently hypothetical) `Ready` case.
    let r = Readiness {
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Unknown,
        read_only: false,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Maintainer);
    assert!(cap.may_author());
    assert!(cap.may_publish());
}

#[test]
fn maintainer_ready_grants_maintainer() {
    // `Ready` cannot be produced by `stikk-prikk::env` today (RFC 016 F3), but `derive` must still
    // treat it identically to `Unknown` once prikk can answer the adoption question — the whole point
    // of the three-valued type is that this behavior needs no change when that day comes (RFC 016 Q1).
    let r = Readiness {
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Ready,
        read_only: false,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Maintainer);
    assert!(cap.may_author());
    assert!(cap.may_publish());
}

#[test]
fn maintainer_not_ready_without_author_is_viewer() {
    let r = Readiness {
        author_ready: false,
        maintainer_readiness: MaintainerReadiness::NotReady,
        read_only: false,
    };
    assert_eq!(Capability::derive(r), Capability::Viewer);
}

#[test]
fn read_only_collapses_everything_to_viewer() {
    // NFR-S01: read-only mode wins over any key presence.
    let r = Readiness {
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Unknown,
        read_only: true,
    };
    let cap = Capability::derive(r);
    assert_eq!(cap, Capability::Viewer);
    assert!(!cap.may_author());
    assert!(!cap.may_publish());
}

#[test]
fn operator_actions_are_available_regardless_of_signing_readiness() {
    // AC-04: recovery is orthogonal to the *mutating* axis (key presence), but each action still
    // confirms. Moved to `Readiness` by RFC 012 F-a — see the read-only tests below for the axis that
    // actually governs it.
    let no_keys = Readiness::none();
    assert!(no_keys.may_operate());
    let fully_ready = Readiness {
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Unknown,
        read_only: false,
    };
    assert!(fully_ready.may_operate());
}

#[test]
fn read_only_locks_out_recovery_too() {
    // RFC 012 F-a's ruling: FR-121 governs. A read-only session may not clear another writer's lock —
    // a read-only mode that still permitted mutating recovery would itself be the "confident-but-wrong
    // picture" (T-T4) this project refuses. True with or without key presence, since `may_operate`
    // depends only on `read_only`.
    let read_only_no_keys = Readiness {
        author_ready: false,
        maintainer_readiness: MaintainerReadiness::NotReady,
        read_only: true,
    };
    assert!(!read_only_no_keys.may_operate());
    let read_only_fully_keyed = Readiness {
        author_ready: true,
        maintainer_readiness: MaintainerReadiness::Unknown,
        read_only: true,
    };
    assert!(!read_only_fully_keyed.may_operate());
}

#[test]
fn readiness_holds_no_secret_only_flags() {
    // Structural guarantee (LC-13): the type has nowhere for key material — a bool, a three-valued
    // enum with no payload, and a bool. This test documents the invariant; the compiler enforces the
    // shape.
    let r = Readiness::none();
    assert!(!r.author_ready);
    assert_eq!(r.maintainer_readiness, MaintainerReadiness::NotReady);
    assert!(!r.read_only);
}

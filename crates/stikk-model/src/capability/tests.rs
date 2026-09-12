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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::NotReady,
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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Known(Binding::Matches),
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
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    };
    assert_eq!(Capability::derive(r), Capability::Viewer);
}

#[test]
fn read_only_collapses_everything_to_viewer() {
    // NFR-S01: read-only mode wins over any key presence.
    let r = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
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
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::NotReady,
        read_only: true,
    };
    assert!(!read_only_no_keys.may_operate());
    let read_only_fully_keyed = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
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
    assert_eq!(r.author, RoleReadiness::NotReady);
    assert_eq!(r.maintainer, RoleReadiness::NotReady);
    assert!(!r.read_only);
}

// --- RFC 026 §3: the two unknowns, and what each does to capability --------------------------
//
// The handoff asserted these are different and asked where keeping them apart is expensive. The
// answer is: here, and only here. Everything downstream reads `arms()`, so the distinction costs one
// match arm in the fold and one sentence in each renderer — and buys the difference between hiding an
// action a user can perform and offering one prikk will refuse.

fn with(author: RoleReadiness, maintainer: RoleReadiness) -> Readiness {
    Readiness {
        author,
        maintainer,
        read_only: false,
    }
}

/// **`Unknown` grants and `Unverifiable` withholds.** One variant that did each would be the shape
/// `C-T2c′` forbids, and this is the assertion that stops them being merged by a later refactor.
#[test]
fn unknown_grants_where_unverifiable_withholds() {
    assert!(RoleReadiness::Unknown.arms());
    assert!(!RoleReadiness::Unverifiable.arms());

    // And the difference reaches capability, not just the predicate.
    assert_eq!(
        Capability::derive(with(RoleReadiness::Unknown, RoleReadiness::Unknown)),
        Capability::Maintainer,
        "RFC 016 Q1: offer the action and let prikk refuse — hiding seal from someone whose key is \
         adopted is its own confident-but-wrong picture"
    );
    assert_eq!(
        Capability::derive(with(
            RoleReadiness::Unverifiable,
            RoleReadiness::Unverifiable
        )),
        Capability::Viewer,
        "prikk 0.40: stikk cannot see whether there is key material at all, so it offers nothing and \
         says unknown — Q1(b)"
    );
}

/// The bindings that arm, and the ones that do not, at the one place the decision is made.
#[test]
fn each_binding_arms_or_withholds_for_its_own_reason() {
    for (binding, arms, why) in [
        (Binding::Matches, true, "prikk confirmed the key binds"),
        (
            Binding::Unrecorded,
            true,
            "prikk accepts a first signature and binds the id then",
        ),
        (
            Binding::NotAdopted,
            false,
            "prikk refuses the seal; offering it would be offering a failure",
        ),
        (
            Binding::Mismatch,
            false,
            "prikk refuses at signing time; the card says which two things disagree",
        ),
        (
            Binding::Absent,
            false,
            "no usable seed to bind, or no repository to ask",
        ),
    ] {
        assert_eq!(
            RoleReadiness::Known(binding).arms(),
            arms,
            "{binding:?}: {why}"
        );
    }
}

/// `mismatch` is a **refusal to arm**, not a caveat on an offered action (RFC 026 Decision 4).
#[test]
fn a_mismatched_maintainer_key_does_not_reach_maintainer_capability() {
    assert_eq!(
        Capability::derive(with(
            RoleReadiness::Known(Binding::Matches),
            RoleReadiness::Known(Binding::Mismatch),
        )),
        Capability::Author,
        "the author half still arms; the maintainer half must not"
    );
}

/// An unadopted maintainer key is the state RFC 025 wanted a fourth variant for. prikk names it, and
/// it withholds — the trust glossary entry is the next step, now offered *before* the refusal.
#[test]
fn an_unadopted_maintainer_key_withholds_but_leaves_author_intact() {
    assert_eq!(
        Capability::derive(with(
            RoleReadiness::Known(Binding::Unrecorded),
            RoleReadiness::Known(Binding::NotAdopted),
        )),
        Capability::Author
    );
}

/// Read-only still collapses everything, whatever prikk says (`NFR-S01`). The one fold keeps its
/// precedence.
#[test]
fn read_only_still_overrides_a_perfectly_bound_pair() {
    let readiness = Readiness {
        author: RoleReadiness::Known(Binding::Matches),
        maintainer: RoleReadiness::Known(Binding::Matches),
        read_only: true,
    };
    assert_eq!(Capability::derive(readiness), Capability::Viewer);
    assert!(!readiness.may_operate());
}

//! Tests for the request-category policy (design `stikk-04` CT-03).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn read_categories_never_mutate_and_are_cancellable() {
    for cat in [
        RequestCategory::ReadHistory,
        RequestCategory::ReadState,
        RequestCategory::WorktreeAnalysis,
        RequestCategory::Integrity,
    ] {
        assert!(!cat.mutates(), "{} must not mutate", cat.name());
        assert!(
            cat.cancellable_in_flight(),
            "{} must be cancellable in flight",
            cat.name()
        );
    }
}

#[test]
fn mutating_categories_are_not_cancellable_in_flight() {
    // A mutation is cancellable only before it executes; prikk's own call is the atomic unit.
    for cat in [
        RequestCategory::QueueMutation,
        RequestCategory::Publication,
        RequestCategory::Exchange,
        RequestCategory::Trust,
        RequestCategory::Recovery,
        RequestCategory::Materialization,
    ] {
        assert!(cat.mutates(), "{} must mutate", cat.name());
        assert!(
            !cat.cancellable_in_flight(),
            "{} must not be cancellable mid-write",
            cat.name()
        );
    }
}

#[test]
fn read_categories_are_tier_one() {
    // RFC 013 decision 6: tier 1 stays free, always — every read category maps here.
    for cat in [
        RequestCategory::ReadHistory,
        RequestCategory::ReadState,
        RequestCategory::WorktreeAnalysis,
        RequestCategory::Integrity,
    ] {
        assert_eq!(cat.tier(), Tier::One, "{} must be tier 1", cat.name());
    }
}

#[test]
fn queue_mutation_and_exchange_are_tier_two() {
    assert_eq!(RequestCategory::QueueMutation.tier(), Tier::Two);
    // RFC 050 decision 2: measured at prikk 0.53, an import creates and advances no local ref and
    // signs nothing, and an export only writes a file outside the repository. `FL-11` asks for a
    // tier-2 confirmation; it was Tier Three before, which no measurement supported.
    assert_eq!(RequestCategory::Exchange.tier(), Tier::Two);
}

#[test]
fn publication_and_materialization_are_tier_three() {
    assert_eq!(RequestCategory::Publication.tier(), Tier::Three);
    // RFC 045 decision 3: restate, then an explicit yes — not the typed name. A switch refuses
    // rather than overwrite anything not in history, and switching back restores every byte.
    assert_eq!(RequestCategory::Materialization.tier(), Tier::Three);
}

#[test]
fn trust_and_recovery_are_tier_three_typed() {
    assert_eq!(RequestCategory::Trust.tier(), Tier::ThreeTyped);
    assert_eq!(RequestCategory::Recovery.tier(), Tier::ThreeTyped);
}

#[test]
fn tier_ordering_reflects_increasing_ceremony() {
    assert!(Tier::One < Tier::Two);
    assert!(Tier::Two < Tier::Three);
    assert!(Tier::Three < Tier::ThreeTyped);
}

#[test]
fn every_mutating_category_has_a_tier_above_one() {
    // Structural cross-check with `mutates()`: a category that mutates must never be tier 1, and vice
    // versa — the two policies must never disagree about which categories are "free".
    for cat in RequestCategory::ALL {
        assert_eq!(
            cat.mutates(),
            cat.tier() > Tier::One,
            "{} disagrees between mutates() and tier()",
            cat.name()
        );
    }
}

#[test]
fn all_lists_every_category_with_a_unique_name() {
    assert_eq!(RequestCategory::ALL.len(), 10);
    let mut names: Vec<&str> = RequestCategory::ALL.iter().map(|c| c.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 10, "category names must be unique");
}

/// **The gate's key moved from the tier to the category, and nothing that existed moved with it**
/// (RFC 045 §2b).
///
/// `capability_gate` used to read the requirement off the tier: Two meant AUTHOR, Three meant
/// MAINTAINER, anything else meant read-write alone. That rule is restated here, and every
/// category is held to it — **except [`RequestCategory::Materialization`]**, the one the change
/// exists for, which is Tier Three and needs no key because prikk needs none (RFC 045 F1,
/// measured). The exception is asserted too, so it cannot quietly become the rule.
#[test]
fn the_requirement_matches_what_the_tier_used_to_imply() {
    for cat in RequestCategory::ALL {
        let implied_by_tier = match cat.tier() {
            Tier::Two => SigningRequirement::Author,
            Tier::Three => SigningRequirement::Maintainer,
            Tier::One | Tier::ThreeTyped => SigningRequirement::ReadWrite,
        };
        if cat == RequestCategory::Materialization || cat == RequestCategory::Exchange {
            assert_eq!(
                cat.signing_requirement(),
                SigningRequirement::ReadWrite,
                "{} signs nothing, so it may ask for no key",
                cat.name()
            );
            assert_ne!(
                implied_by_tier,
                cat.signing_requirement(),
                "{} is a category the change exists for; if it stops differing, the change \
                 is no longer doing anything",
                cat.name()
            );
        } else {
            assert_eq!(
                cat.signing_requirement(),
                implied_by_tier,
                "{} changed what it requires",
                cat.name()
            );
        }
    }
}

/// The table the handoff spells out, row by row — so a reader can check it against §2b without
/// re-deriving it from the tier.
#[test]
fn each_category_requires_exactly_what_the_table_says() {
    use RequestCategory as C;
    use SigningRequirement as S;
    for (cat, want) in [
        (C::ReadHistory, S::ReadWrite),
        (C::ReadState, S::ReadWrite),
        (C::WorktreeAnalysis, S::ReadWrite),
        (C::Integrity, S::ReadWrite),
        (C::QueueMutation, S::Author),
        (C::Publication, S::Maintainer),
        (C::Exchange, S::ReadWrite),
        (C::Trust, S::ReadWrite),
        (C::Recovery, S::ReadWrite),
        (C::Materialization, S::ReadWrite),
    ] {
        assert_eq!(cat.signing_requirement(), want, "{}", cat.name());
    }
}

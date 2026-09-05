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
fn queue_mutation_is_tier_two() {
    assert_eq!(RequestCategory::QueueMutation.tier(), Tier::Two);
}

#[test]
fn publication_and_exchange_are_tier_three() {
    assert_eq!(RequestCategory::Publication.tier(), Tier::Three);
    assert_eq!(RequestCategory::Exchange.tier(), Tier::Three);
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
    assert_eq!(RequestCategory::ALL.len(), 9);
    let mut names: Vec<&str> = RequestCategory::ALL.iter().map(|c| c.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 9, "category names must be unique");
}

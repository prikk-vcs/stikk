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
fn all_lists_every_category_with_a_unique_name() {
    assert_eq!(RequestCategory::ALL.len(), 9);
    let mut names: Vec<&str> = RequestCategory::ALL.iter().map(|c| c.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 9, "category names must be unique");
}

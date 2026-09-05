//! Tests for the command registry behind the palette (design TU-07, FR-125; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::{Readiness, Tier};

use super::*;

fn readiness(author: bool, maintainer: bool, read_only: bool) -> Readiness {
    Readiness {
        author_ready: author,
        maintainer_ready: maintainer,
        read_only,
    }
}

#[test]
fn the_registry_lists_the_seeded_commands_with_bindings() {
    let all = commands();
    assert!(all.iter().any(|c| c.id == "view.history"));
    assert!(all.iter().any(|c| c.id == "view.glossary"));
    assert!(all.iter().any(|c| c.id == "op.commit"));
    // Every command carries a non-empty binding and name (discoverability, FR-125).
    for command in all {
        assert!(!command.name.is_empty());
        assert!(!command.binding.is_empty());
    }
}

#[test]
fn a_viewer_may_run_every_tier_one_command() {
    let viewer = readiness(false, false, false);
    for command in commands().iter().filter(|c| c.tier == Tier::One) {
        assert!(command.available_to(viewer));
    }
}

#[test]
fn commit_is_visible_but_disabled_for_a_viewer_and_enabled_for_author() {
    // RFC 014 §6: the first mutating command in the registry. Tier::Two ⟺ AUTHOR, the same ladder
    // `capability_gate` enforces.
    let commit = commands()
        .iter()
        .find(|c| c.id == "op.commit")
        .expect("commit is registered");
    assert!(!commit.available_to(readiness(false, false, false)));
    assert!(commit.available_to(readiness(true, false, false)));
    assert!(commit.available_to(readiness(true, true, false)));
    assert_eq!(
        commit
            .unmet_reason(readiness(false, false, false))
            .as_deref(),
        Some("commit needs AUTHOR signing readiness")
    );
    assert!(commit.unmet_reason(readiness(true, false, false)).is_none());
}

#[test]
fn read_only_disables_commit_even_with_author_keys_present() {
    // RFC 014 §6's whole point: the palette must agree with `confirm`, which read-only refuses
    // regardless of capability (NFR-S01) — a bare-`Capability` check could never see this.
    let commit = commands()
        .iter()
        .find(|c| c.id == "op.commit")
        .expect("commit is registered");
    assert!(!commit.available_to(readiness(true, false, true)));
    assert!(
        commit
            .unmet_reason(readiness(true, false, true))
            .as_deref()
            .is_some_and(|r| r.contains("read-only"))
    );
}

#[test]
fn the_capability_gate_disables_below_the_minimum() {
    // A synthetic tier-three (Maintainer-equivalent) command is visible-but-disabled for a Viewer
    // (FR-104), enabled for a Maintainer — the mechanism a future publication command registers behind.
    let seal = Command {
        id: "work.seal",
        name: "Seal…",
        binding: "S",
        operation: "seal",
        tier: Tier::Three,
        opens: None,
    };
    assert!(!seal.available_to(readiness(false, false, false)));
    assert!(!seal.available_to(readiness(true, false, false)));
    assert!(seal.available_to(readiness(true, true, false)));
    // The disabled entry carries a reason (FR-104), and is None once available.
    assert_eq!(
        seal.unmet_reason(readiness(false, false, false)).as_deref(),
        Some("seal needs MAINTAINER signing readiness")
    );
    assert!(seal.unmet_reason(readiness(true, true, false)).is_none());
}

#[test]
fn matching_filters_by_name_and_keeps_order() {
    let hits = matching("history");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "view.history");
    // Empty filter returns everything.
    assert_eq!(matching("").len(), commands().len());
    // A no-match filter returns nothing (but never errors).
    assert!(matching("zzzzz").is_empty());
}

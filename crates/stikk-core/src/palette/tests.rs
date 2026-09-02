//! Tests for the command registry behind the palette (design TU-07, FR-125; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::Capability;

use super::*;

#[test]
fn the_registry_lists_the_seeded_commands_with_bindings() {
    let all = commands();
    assert!(all.iter().any(|c| c.id == "view.history"));
    assert!(all.iter().any(|c| c.id == "view.glossary"));
    // Every command carries a non-empty binding and name (discoverability, FR-125).
    for command in all {
        assert!(!command.name.is_empty());
        assert!(!command.binding.is_empty());
    }
}

#[test]
fn a_viewer_may_run_every_current_command() {
    // Increment 4's commands are all Viewer-level reads; the gate mechanism still applies.
    for command in commands() {
        assert!(command.available_to(Capability::Viewer));
    }
}

#[test]
fn the_capability_gate_disables_below_the_minimum() {
    // A synthetic Maintainer-only command is visible-but-disabled for a Viewer (FR-104), enabled for a
    // Maintainer — the mechanism mutating commands will register behind.
    let seal = Command {
        id: "work.seal",
        name: "Seal…",
        binding: "S",
        min_capability: Capability::Maintainer,
        opens: None,
    };
    assert!(!seal.available_to(Capability::Viewer));
    assert!(!seal.available_to(Capability::Author));
    assert!(seal.available_to(Capability::Maintainer));
    // The disabled entry carries a reason (FR-104), and is None once available.
    assert_eq!(
        seal.unmet_reason(Capability::Viewer).as_deref(),
        Some("needs maintainer readiness")
    );
    assert!(seal.unmet_reason(Capability::Maintainer).is_none());
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

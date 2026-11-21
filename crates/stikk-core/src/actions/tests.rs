//! The one catalog's own tests (design `TU-05`/`TU-06`/`TU-07`, `FR-125`; RFC 061 decision 1).
//!
//! **The availability tests are RFC 007's, moved, not rewritten** (RFC 061 Handoff A §1.1): the words
//! a disabled entry gives do not change because the list it lives in did.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::{Readiness, RoleReadiness, Tier};

use super::*;

fn readiness(author: bool, maintainer: bool, read_only: bool) -> Readiness {
    Readiness {
        author: if author {
            RoleReadiness::Unknown
        } else {
            RoleReadiness::NotReady
        },
        maintainer: if maintainer {
            RoleReadiness::Unknown
        } else {
            RoleReadiness::NotReady
        },
        read_only,
    }
}

#[test]
fn the_catalog_names_every_action_it_lists() {
    let all = all();
    assert!(all.iter().any(|a| a.id == "view.history"));
    assert!(all.iter().any(|a| a.id == "view.glossary"));
    assert!(all.iter().any(|a| a.id == "op.commit"));
    for action in all {
        assert!(!action.name.is_empty(), "{}", action.id);
        // A kebab-case, dotted id (RFC 002 rule 1), so a test or a key map can name it.
        assert!(
            action
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '.' || c == '-'),
            "{} is not a kebab-case id",
            action.id
        );
        assert!(!action.operation.is_empty(), "{}", action.id);
        assert!(!action.applies.is_empty(), "{} applies nowhere", action.id);
    }
}

/// **No two actions share a key, ignoring case** (RFC 061 decision 2) — which is what makes Caps
/// Lock harmless.
#[test]
fn no_two_actions_share_a_key_ignoring_case() {
    let mut seen: Vec<(String, &str)> = Vec::new();
    for action in all() {
        let Some(key) = action.key else { continue };
        let folded = match key {
            Key::Letter(c) => c.to_ascii_lowercase().to_string(),
            other => format!("{other:?}"),
        };
        if let Some((_, first)) = seen.iter().find(|(k, _)| *k == folded) {
            panic!("{folded:?} is bound by both {first} and {}", action.id);
        }
        seen.push((folded, action.id));
    }
    assert!(seen.len() > 15, "only {} keys are bound", seen.len());
}

/// **Every letter the catalog binds is lower case**, so `by_letter`'s fold always finds it.
#[test]
fn every_bound_letter_is_lower_case() {
    for action in all() {
        if let Some(Key::Letter(c)) = action.key {
            assert!(
                c.is_ascii_lowercase(),
                "{} binds {c:?}, which the fold could not match",
                action.id
            );
        }
    }
}

/// **A letter finds its action in either case** (decision 2's whole point).
#[test]
fn a_letter_finds_the_same_action_in_either_case() {
    for action in all() {
        let Some(Key::Letter(c)) = action.key else {
            continue;
        };
        let lower = by_letter(c).expect("the letter is bound");
        let upper = by_letter(c.to_ascii_uppercase()).expect("and so is its upper case");
        assert_eq!(lower.id, action.id);
        assert_eq!(upper.id, action.id, "Caps Lock changed {c:?}'s meaning");
    }
}

/// **The key line's entries are the ones with a hint and a key**, and every one of them ranks.
#[test]
fn every_key_line_entry_has_a_key_and_a_rank() {
    for action in all() {
        if action.hint.is_some() {
            assert!(action.key.is_some(), "{} hints with no key", action.id);
            assert!(action.rank > 0, "{} hints with no rank", action.id);
            assert!(
                !action.applies_to(Where::Card),
                "{} is a card's own key, which the key line never shows",
                action.id
            );
        }
        if action.rank > 0 {
            assert!(action.hint.is_some(), "{} ranks with no hint", action.id);
        }
    }
}

/// **`? help` and `q back` are never dropped** (decision 5), which is rank 255.
#[test]
fn help_and_back_never_drop() {
    for id in ["view.glossary", "app.back"] {
        assert_eq!(by_id(id).expect(id).rank, 255, "{id}");
    }
    // Nothing else comes near them: 254 is reserved for `Enter actions`, which arrives with the
    // list it opens (RFC 061 decision 3).
    for action in all() {
        if !["view.glossary", "app.back"].contains(&action.id) {
            assert!(action.rank < 254, "{} outranks the two", action.id);
        }
    }
}

/// **A row's actions, in the order a reader meets them** (decision 3).
#[test]
fn each_row_kind_offers_what_rfc_061_lists() {
    let ids = |kind: RowKind| -> Vec<&'static str> {
        applying_to(Where::Row(kind))
            .into_iter()
            .map(|action| action.id)
            .collect()
    };
    assert_eq!(
        ids(RowKind::Ref),
        vec![
            "ref.open-history",
            "ref.switch",
            "ref.merge-plan",
            "ref.branch",
            "ref.tag",
            "ref.close",
            "ref.export",
            "block.mark",
        ]
    );
    assert_eq!(
        ids(RowKind::Block),
        vec![
            "block.open",
            "block.mark",
            "op.compare",
            "block.undo",
            "block.tag"
        ]
    );
    assert_eq!(
        ids(RowKind::File),
        vec!["file.open-patch", "file.save", "file.save-all"]
    );
    assert_eq!(ids(RowKind::QueuedPatch), vec!["patch.open"]);
    assert_eq!(ids(RowKind::TrustKey), vec!["trust.remove"]);
    // **A Changes row has no action of its own** (decision 3): what `d` compares is the whole
    // worktree. Its list is what the screen offers.
    assert!(ids(RowKind::ChangesRow).is_empty());
    let changes: Vec<&str> = applying_to(Where::Screen(ScreenKind::Changes))
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert_eq!(
        changes,
        vec!["op.commit", "op.compare", "changes.untracked"]
    );
}

#[test]
fn a_viewer_may_run_every_tier_one_action() {
    let viewer = readiness(false, false, false);
    for action in all().iter().filter(|a| a.tier() == Tier::One) {
        assert!(action.available_to(viewer), "{}", action.id);
    }
}

#[test]
fn commit_is_visible_but_disabled_for_a_viewer_and_enabled_for_author() {
    // RFC 014 §6: the first mutating command in the registry. Tier::Two ⟺ AUTHOR, the same ladder
    // `capability_gate` enforces.
    let commit = by_id("op.commit").expect("commit is in the catalog");
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
    let commit = by_id("op.commit").expect("commit is in the catalog");
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
    let seal = Action {
        id: "work.seal",
        name: "Seal…",
        hint: None,
        key: None,
        applies: &[Where::Everywhere],
        operation: "seal",
        category: Some(stikk_model::RequestCategory::Publication),
        needs_focused_ref: false,
        rank: 0,
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
    let hits = matching("Open History");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "view.history");
    // **An empty filter returns every action but a card's own** (`FR-125`), which is what makes the
    // palette the catalog it is meant to be.
    let cards = all()
        .iter()
        .filter(|action| action.applies_to(Where::Card))
        .count();
    assert_eq!(matching("").len(), all().len() - cards);
    assert!(
        matching("").len() > 30,
        "the palette lists {} entries",
        matching("").len()
    );
    // A no-match filter returns nothing (but never errors).
    assert!(matching("zzzzz").is_empty());
}

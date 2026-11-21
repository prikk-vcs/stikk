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
    // And `Enter actions` drops last of the rest.
    let first = by_id("row.actions").expect("row.actions");
    assert_eq!(first.rank, 254);
    for action in all() {
        if !["view.glossary", "app.back", "row.actions"].contains(&action.id) {
            assert!(
                action.rank < first.rank,
                "{} outranks `Enter actions`",
                action.id
            );
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
    assert_eq!(ids(RowKind::File), vec!["file.save", "file.save-all"]);
    // **A block's patch row opens the patch**, which is what Enter did there before.
    assert_eq!(ids(RowKind::BlockPatch), vec!["patch.open"]);
    assert_eq!(ids(RowKind::QueuedPatch), vec!["patch.open"]);
    assert_eq!(ids(RowKind::TrustKey), vec!["trust.remove"]);
    // **Changes has no selectable row** (decision 3): what `d` compares is the whole worktree, so
    // Enter there answers with the screen's own actions.
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

/// **What a Changes row is offered, in the RFC's own order** (decision 3): the screen's own actions
/// before the one it shares with every screen, so `commit` is not what Enter, Enter would run.
#[test]
fn a_changes_row_reads_compare_untracked_commit() {
    let ids: Vec<&str> = here(None, Some(ScreenKind::Changes))
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert_eq!(
        ids.get(..3),
        Some(&["op.compare", "changes.untracked", "op.commit"][..])
    );
    // Then what can be done anywhere, since no row is selected.
    assert!(ids.contains(&"ref.pick"), "{ids:?}");
}

/// **A screen with no row and no actions of its own answers with what applies everywhere**, less the
/// ways of moving around - never with an empty list.
#[test]
fn orientation_answers_with_the_actions_that_apply_everywhere() {
    let list = here(None, Some(ScreenKind::Orientation));
    assert!(list.len() > 8, "only {} entries", list.len());
    // **Its own first**: `Enter` on the root opened History before RFC 061, so Enter, Enter does.
    assert_eq!(list.first().map(|action| action.id), Some("view.history"));
    for action in &list {
        assert!(!action.is_navigation(), "{} is a way of moving", action.id);
    }
    for id in [
        "row.actions",
        "row.open",
        "app.back",
        "app.move",
        "app.quit",
    ] {
        assert!(
            !list.iter().any(|action| action.id == id),
            "{id} is in the list"
        );
    }
}

/// **A row's list is about the row**: with a row selected, the actions that apply everywhere are
/// left out, so the list does not bury five row actions under fifteen global ones.
#[test]
fn a_rows_list_leaves_out_what_applies_everywhere() {
    let list = here(Some(RowKind::Block), Some(ScreenKind::History));
    let ids: Vec<&str> = list.iter().map(|action| action.id).collect();
    assert_eq!(
        ids,
        vec![
            "block.open",
            "block.mark",
            "op.compare",
            "block.undo",
            "block.tag"
        ]
    );
}

/// **Every row kind's first entry is the one the right arrow runs** (decision 3).
#[test]
fn the_first_entry_of_each_row_kind_is_its_open_action() {
    for (kind, first) in [
        (RowKind::Ref, "ref.open-history"),
        (RowKind::Block, "block.open"),
        (RowKind::BlockPatch, "patch.open"),
        (RowKind::QueuedPatch, "patch.open"),
        (RowKind::TrustKey, "trust.remove"),
        (RowKind::File, "file.save"),
    ] {
        let list = here(Some(kind), None);
        assert_eq!(
            list.first().map(|action| action.id),
            Some(first),
            "{kind:?}"
        );
    }
}

/// **Every entry of every list is an action the catalog holds**, and no list is ever empty.
#[test]
fn every_list_holds_catalog_entries_only() {
    let kinds = [
        RowKind::Ref,
        RowKind::Block,
        RowKind::File,
        RowKind::BlockPatch,
        RowKind::QueuedPatch,
        RowKind::TrustKey,
    ];
    let screens = [
        ScreenKind::Orientation,
        ScreenKind::History,
        ScreenKind::BlockDetail,
        ScreenKind::Changes,
        ScreenKind::Queue,
        ScreenKind::PatchDetail,
        ScreenKind::Compare,
        ScreenKind::Verify,
        ScreenKind::Trust,
        ScreenKind::MergePlan,
        ScreenKind::Bundle,
    ];
    for screen in screens {
        for row in kinds.map(Some).into_iter().chain([None]) {
            let list = here(row, Some(screen));
            assert!(!list.is_empty(), "{row:?} on {screen:?} offers nothing");
            for action in list {
                assert!(by_id(action.id).is_some(), "{}", action.id);
            }
        }
    }
}

/// **Every key a reader is taught is one the catalog binds** — the two spellings of a key agree, so a
/// document built from either names the same keys (RFC 061 decision 6).
#[test]
fn a_keys_tokens_are_the_words_it_is_named_by() {
    for action in all() {
        let Some(key) = action.key else { continue };
        let every = key.every();
        let tokens = key.tokens();
        assert!(!tokens.is_empty(), "{}", action.id);
        for token in &tokens {
            assert!(
                every.contains(token.as_str()),
                "{}: {token:?} is not in {every:?}",
                action.id
            );
        }
        // The key line's label is one of the tokens, or — where one key is two arrows — made of them.
        let label = key.label();
        assert!(
            tokens.contains(&label) || label == format!("{}{}", tokens[0], tokens[1]),
            "{}: the label {label:?} is not among {tokens:?}",
            action.id
        );
    }
}

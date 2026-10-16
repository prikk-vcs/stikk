//! Tests for the three ref writes (RFC 044).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::{ChangeToken, CurrentBranch, Readiness, RoleReadiness, Tier};
use stikk_prikk::{NullBackend, Orientation};

use super::*;

fn orientation(current: CurrentBranch) -> Orientation {
    Orientation {
        interrupted_materialization: None,
        queued_patches: 0,
        queued_target: None,
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: current,
    }
}

fn backend_on(current: CurrentBranch) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 52, 0)
        .with_orientation(orientation(current.clone()))
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &current,
        ))
}

fn ready() -> NullBackend {
    backend_on(CurrentBranch::Branch(
        stikk_model::RefName::parse("heads/main").expect("a ref name"),
    ))
}

/// The same session, with MAINTAINER readiness present -- `RoleReadiness::Unknown`, the only
/// value `stikk-prikk::env` can produce for presence (RFC 016 F3), and the one
/// `Capability::derive` grants `Maintainer` on.
fn maintainer_ready() -> NullBackend {
    ready().with_readiness(stikk_prikk::ReadinessReport {
        readiness: Readiness {
            author: RoleReadiness::Unknown,
            maintainer: RoleReadiness::Unknown,
            read_only: false,
        },
        author: stikk_prikk::RoleDetail::default(),
        maintainer: stikk_prikk::RoleDetail::default(),
    })
}

fn repo() -> &'static std::path::Path {
    std::path::Path::new("/repo")
}

fn summary(preview: RefWritePreview) -> crate::confirm::ConfirmationSummary {
    match preview {
        RefWritePreview::Ready { token } => token.summary().clone(),
        RefWritePreview::Blocked(reason) => panic!("expected Ready, got blocked: {reason}"),
    }
}

fn blocked(preview: RefWritePreview) -> String {
    match preview {
        RefWritePreview::Blocked(reason) => reason,
        RefWritePreview::Ready { .. } => panic!("expected Blocked"),
    }
}

#[test]
fn creating_a_branch_restates_the_name_the_source_and_what_does_not_change() {
    let s = summary(
        create_branch_preview(&ready(), repo(), "heads/feature", "heads/main").expect("reads"),
    );
    assert_eq!(s.operation, "Create a branch");
    assert_eq!(s.target_ids, vec!["heads/feature", "heads/main"]);
    assert!(
        s.consequence
            .contains("heads/feature will point at heads/main's tip"),
        "{s:?}"
    );
    // **What a ref write does not touch** is the half a reader most needs to be told.
    assert!(
        s.consequence.contains("Nothing in the worktree changes"),
        "{s:?}"
    );
}

#[test]
fn creating_a_tag_says_when_it_carries_no_message() {
    let with = summary(
        create_tag_preview(
            &ready(),
            repo(),
            NewTag {
                name: "tags/v1",
                target: "heads/main",
                message: "first release",
            },
        )
        .expect("reads"),
    );
    assert!(!with.consequence.contains("carries no message"), "{with:?}");
    // **The message itself is restated** (decision 6), in its own field rather than inside
    // stikk's prose: it is the one string on the card the user wrote.
    assert_eq!(with.message_note.as_deref(), Some("first release"));

    let without = summary(
        create_tag_preview(
            &ready(),
            repo(),
            NewTag {
                name: "tags/v1",
                target: "heads/main",
                message: "",
            },
        )
        .expect("reads"),
    );
    assert!(
        without.consequence.contains("It carries no message"),
        "{without:?}"
    );
    assert!(without.message_note.is_none(), "{without:?}");
}

/// The other two carry no message, ever.
#[test]
fn only_a_tag_can_carry_a_message() {
    for preview in [
        create_branch_preview(&ready(), repo(), "heads/f", "heads/main").expect("reads"),
        close_branch_preview(&ready(), repo(), "heads/old").expect("reads"),
    ] {
        assert!(summary(preview).message_note.is_none());
    }
}

#[test]
fn closing_a_branch_restates_prikks_promise_that_nothing_is_reclaimed() {
    let s = summary(close_branch_preview(&ready(), repo(), "heads/old").expect("reads"));
    assert_eq!(s.operation, "Close a branch");
    assert!(s.consequence.contains("Nothing is reclaimed"), "{s:?}");
    assert!(s.consequence.contains("can be reopened"), "{s:?}");
}

/// **The one guard stikk adds** (RFC 044 decision 7, F3): prikk would close the branch the
/// repository is on, and leave it stranded.
#[test]
fn closing_the_current_branch_is_blocked_with_stikks_own_reason() {
    let reason = blocked(close_branch_preview(&ready(), repo(), "heads/main").expect("reads"));
    assert!(reason.contains("this is the branch you are on"), "{reason}");
    // It names the way out, since stikk cannot switch branches yet.
    assert!(reason.contains("prikk branch switch"), "{reason}");
    // Never prikk's words: prikk does not refuse this at all with an empty WAL.
    assert!(!reason.contains("non-empty active WAL"), "{reason}");
}

#[test]
fn another_branch_is_not_blocked_by_that_guard() {
    // The guard is about identity, not about closing in general.
    summary(close_branch_preview(&ready(), repo(), "heads/other").expect("reads"));
}

#[test]
fn the_guard_does_not_fire_when_prikk_does_not_name_a_current_branch() {
    // Below 0.42 there is no current branch to compare against. The frontend does not offer
    // these there at all; if one arrives anyway, stikk does not invent a match.
    let backend = backend_on(CurrentBranch::NotReported);
    summary(close_branch_preview(&backend, repo(), "heads/main").expect("reads"));
}

#[test]
fn an_empty_name_is_blocked_before_anything_runs() {
    for reason in [
        blocked(create_branch_preview(&ready(), repo(), "", "heads/main").expect("reads")),
        blocked(
            create_tag_preview(
                &ready(),
                repo(),
                NewTag {
                    name: "",
                    target: "heads/main",
                    message: "",
                },
            )
            .expect("reads"),
        ),
        blocked(close_branch_preview(&ready(), repo(), "").expect("reads")),
    ] {
        assert!(reason.contains("needs a name"), "{reason}");
    }
}

/// **stikk adds no naming rules of its own** (decision 5): a name prikk accepts, stikk sends.
#[test]
fn a_name_stikk_would_not_have_chosen_is_still_sent() {
    for name in [
        "heads/bad name",
        "heads/UPPER",
        "heads/x/y/z",
        "heads/\u{1f600}",
    ] {
        let s =
            summary(create_branch_preview(&ready(), repo(), name, "heads/main").expect("reads"));
        assert_eq!(
            s.target_ids.first().map(String::as_str),
            Some(name),
            "the name reaches prikk unaltered"
        );
    }
}

/// **Tier Three and the publish capability**, exactly as seal.
#[test]
fn every_ref_write_is_tier_three_and_maintainer() {
    for preview in [
        create_branch_preview(&ready(), repo(), "heads/f", "heads/main").expect("reads"),
        create_tag_preview(
            &ready(),
            repo(),
            NewTag {
                name: "tags/v",
                target: "heads/main",
                message: "",
            },
        )
        .expect("reads"),
        close_branch_preview(&ready(), repo(), "heads/old").expect("reads"),
    ] {
        match preview {
            RefWritePreview::Ready { token } => {
                assert_eq!(token.tier(), Tier::Three);
                assert_eq!(
                    token.summary().capability,
                    stikk_model::Capability::Maintainer
                );
            }
            RefWritePreview::Blocked(reason) => panic!("{reason}"),
        }
    }
}

/// **Without a maintainer key, stikk says what is missing** before prikk's signature refusal
/// could (decision 2).
#[test]
fn a_session_without_a_maintainer_key_is_told_what_is_missing() {
    let s =
        summary(create_branch_preview(&ready(), repo(), "heads/f", "heads/main").expect("reads"));
    assert!(
        s.consequence.contains("needs a MAINTAINER signing key"),
        "the default NullBackend is a Viewer: {s:?}"
    );
    // And never prikk's wording for it.
    assert!(!s.consequence.contains("invalid signature"), "{s:?}");
}

#[test]
fn the_band_is_the_one_constant() {
    assert_eq!(
        REF_WRITES_FROM_MINOR,
        stikk_prikk::CURRENT_BRANCH_FROM_MINOR
    );
    assert_eq!(REF_WRITES_FROM_MINOR, 42);
}

/// The operation names are stikk's own, and are what `Intent` carries.
#[test]
fn the_operation_names_are_stikks_own() {
    for name in [
        CREATE_BRANCH_OPERATION,
        CREATE_TAG_OPERATION,
        CLOSE_BRANCH_OPERATION,
    ] {
        assert!(!name.is_empty());
        assert_eq!(name, name.to_lowercase(), "{name}");
    }
}

/// The counterpart to [`a_session_without_a_maintainer_key_is_told_what_is_missing`]: with the key
/// present the sentence is **absent**, so the preview never warns about something that is in place.
#[test]
fn a_maintainer_ready_session_is_not_warned_about_a_missing_key() {
    let s = summary(
        create_branch_preview(&maintainer_ready(), repo(), "heads/f", "heads/main").expect("reads"),
    );
    assert!(!s.consequence.contains("MAINTAINER signing key"), "{s:?}");
    assert!(s.consequence.contains("heads/main"), "{s:?}");
}

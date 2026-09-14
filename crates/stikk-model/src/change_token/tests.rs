//! Tests for [`ChangeToken`] composition (design `LC-4`; RFC 003).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::CurrentBranch;

fn token(refs: &[(&str, &str)], queued: u64, target: Option<&str>) -> ChangeToken {
    ChangeToken::compose(
        refs.iter().copied(),
        queued,
        target,
        &CurrentBranch::NotReported,
    )
}

fn with_branch(current: &CurrentBranch) -> ChangeToken {
    ChangeToken::compose([("heads/main", "aaaa")], 0, None, current)
}

fn branch(name: &str) -> CurrentBranch {
    CurrentBranch::Branch(crate::RefName::parse(name).expect("a valid ref name"))
}

#[test]
fn the_same_state_yields_the_same_token() {
    let refs = [("heads/main", "aaaa"), ("heads/other", "bbbb")];
    assert_eq!(token(&refs, 0, None), token(&refs, 0, None));
}

#[test]
fn a_moved_ref_changes_the_token() {
    let before = token(&[("heads/main", "aaaa")], 0, None);
    let after = token(&[("heads/main", "cccc")], 0, None);
    assert_ne!(before, after);
}

#[test]
fn a_new_ref_changes_the_token() {
    let before = token(&[("heads/main", "aaaa")], 0, None);
    let after = token(&[("heads/main", "aaaa"), ("heads/other", "bbbb")], 0, None);
    assert_ne!(before, after);
}

#[test]
fn a_removed_ref_changes_the_token() {
    let before = token(&[("heads/main", "aaaa"), ("heads/other", "bbbb")], 0, None);
    let after = token(&[("heads/main", "aaaa")], 0, None);
    assert_ne!(before, after);
}

#[test]
fn a_changed_queue_count_changes_the_token() {
    let refs = [("heads/main", "aaaa")];
    assert_ne!(token(&refs, 0, None), token(&refs, 1, None));
}

#[test]
fn a_changed_queue_target_changes_the_token() {
    let refs = [("heads/main", "aaaa")];
    assert_ne!(
        token(&refs, 1, Some("heads/main")),
        token(&refs, 1, Some("heads/other"))
    );
    // None vs Some must also differ — an empty queue is not the same state as a queue targeting a ref.
    assert_ne!(token(&refs, 1, None), token(&refs, 1, Some("heads/main")));
}

#[test]
fn ref_order_does_not_affect_the_token() {
    // RFC 003 handoff §2: never inherit prikk's ordering incidentally — sort explicitly, always.
    let a = token(&[("heads/main", "aaaa"), ("heads/other", "bbbb")], 0, None);
    let b = token(&[("heads/other", "bbbb"), ("heads/main", "aaaa")], 0, None);
    assert_eq!(a, b);
}

#[test]
fn an_empty_ref_list_is_a_valid_state() {
    // A freshly-init'ed repository with no refs and no queue is a real, composable state.
    let empty = ChangeToken::compose(std::iter::empty(), 0, None, &CurrentBranch::NotReported);
    assert_eq!(
        empty,
        ChangeToken::compose(std::iter::empty(), 0, None, &CurrentBranch::NotReported)
    );
}

#[test]
fn debug_shows_only_the_digest_no_structure() {
    let t = token(&[("heads/main", "aaaa")], 0, None);
    let shown = format!("{t:?}");
    assert!(shown.starts_with("ChangeToken("));
    assert!(!shown.contains("heads/main"));
    assert!(!shown.contains("aaaa"));
}

// RFC 030 decision 2: prikk's current branch, three states, each distinct.

#[test]
fn the_same_current_branch_state_yields_the_same_token() {
    for state in [
        CurrentBranch::NotReported,
        CurrentBranch::Unresolved("<unresolved; run `prikk doctor`>".to_string()),
        branch("heads/main"),
    ] {
        assert_eq!(
            with_branch(&state),
            with_branch(&state.clone()),
            "{state:?}"
        );
    }
}

#[test]
fn each_pair_of_current_branch_states_composes_distinct_tokens() {
    let not_reported = with_branch(&CurrentBranch::NotReported);
    let unresolved = with_branch(&CurrentBranch::Unresolved(
        "<unresolved; run `prikk doctor`>".to_string(),
    ));
    let a_branch = with_branch(&branch("heads/main"));
    assert_ne!(not_reported, unresolved, "not reported vs unresolved");
    assert_ne!(not_reported, a_branch, "not reported vs a branch");
    assert_ne!(unresolved, a_branch, "unresolved vs a branch");
}

#[test]
fn unresolved_text_never_equals_a_branch_of_the_same_spelling() {
    // The discriminant is what keeps these apart: without it both would hash the string `heads/x`.
    assert_ne!(
        with_branch(&CurrentBranch::Unresolved("heads/x".to_string())),
        with_branch(&branch("heads/x"))
    );
}

#[test]
fn a_branch_switch_changes_the_token() {
    // A terminal `prikk branch switch heads/dev` at 0.42 moves no ref, tag or queue — only this.
    assert_ne!(
        with_branch(&branch("heads/main")),
        with_branch(&branch("heads/dev"))
    );
}

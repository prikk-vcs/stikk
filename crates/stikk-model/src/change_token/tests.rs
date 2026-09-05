//! Tests for [`ChangeToken`] composition (design `LC-4`; RFC 003).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn token(refs: &[(&str, &str)], queued: u64, target: Option<&str>) -> ChangeToken {
    ChangeToken::compose(refs.iter().copied(), queued, target)
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
    let empty = ChangeToken::compose(std::iter::empty(), 0, None);
    assert_eq!(empty, ChangeToken::compose(std::iter::empty(), 0, None));
}

#[test]
fn debug_shows_only_the_digest_no_structure() {
    let t = token(&[("heads/main", "aaaa")], 0, None);
    let shown = format!("{t:?}");
    assert!(shown.starts_with("ChangeToken("));
    assert!(!shown.contains("heads/main"));
    assert!(!shown.contains("aaaa"));
}

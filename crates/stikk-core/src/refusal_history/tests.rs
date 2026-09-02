//! Tests for the session refusal history (design DM-06, FR-112; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn records_are_returned_newest_first() {
    let mut history = RefusalHistory::new();
    history.record("first", "refusal", OperationContext::LoadHistory);
    history.record("second", "lock-conflict", OperationContext::Orient);
    let recent = history.recent();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].verbatim, "second"); // newest first
    assert_eq!(recent[1].verbatim, "first");
    assert!(recent[0].seq > recent[1].seq);
}

#[test]
fn the_ring_is_capped_and_drops_the_oldest() {
    let mut history = RefusalHistory::new();
    for i in 0..60 {
        history.record(format!("refusal {i}"), "refusal", OperationContext::Other);
    }
    assert_eq!(history.len(), 50); // CAPACITY
    let recent = history.recent();
    assert_eq!(recent[0].verbatim, "refusal 59"); // newest kept
    assert_eq!(recent[49].verbatim, "refusal 10"); // oldest 0..=9 dropped
}

#[test]
fn a_fresh_history_is_empty() {
    let history = RefusalHistory::new();
    assert!(history.is_empty());
    assert!(history.recent().is_empty());
}

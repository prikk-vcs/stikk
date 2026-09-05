//! Golden-message tests for the failure classifier (design TS-03; RFC 007).
//!
//! The messages below are the shapes prikk emits on exit 1 for failures stikk can already provoke at
//! prikk 0.27.1 (a bad ref, a foreign directory, a held lock). A prikk version that rewords one of
//! these fails here rather than misclassifying at a user.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::RequestCategory;

use super::*;

fn on_stderr(msg: &str) -> (&str, &str) {
    ("", msg)
}

#[test]
fn a_missing_ref_is_a_refusal_verbatim() {
    let (out, err) = on_stderr("error: ref \"heads/nope\" does not exist");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "refusal");
    assert!(e.to_string().contains("does not exist"));
}

#[test]
fn a_foreign_directory_is_environment() {
    let (out, err) = on_stderr("error: not a prikk repository (no .prikk directory found)");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "environment");
    assert!(e.to_string().contains("not a prikk repository"));
}

#[test]
fn a_retired_format_is_environment_with_the_migration_message() {
    let (out, err) =
        on_stderr("error: retired repository format 2; run `prikk migrate` to upgrade");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "environment");
    assert!(e.to_string().contains("migrate"));
}

#[test]
fn a_held_lock_is_a_lock_conflict() {
    let (out, err) = on_stderr("error: repository lock held by another writer (pid 4213)");
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "lock-conflict");
    assert!(e.to_string().contains("another writer"));
}

#[test]
fn a_cross_ref_commit_refusal_is_not_a_lock_conflict() {
    // RFC 014 F2: prikk words this as a lock conflict, but nothing is locked. Captured live from a
    // real prikk 0.31.1 binary: `prikk commit --from-worktree --ref heads/other -m x` against a
    // repository whose active WAL owns `heads/main`.
    let (out, err) = on_stderr(
        "error: lock conflict: active WAL is owned by heads/main; requested ref heads/other",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "cross-ref");
    assert!(e.to_string().contains("active WAL is owned by"));
}

#[test]
fn a_ref_state_precondition_is_a_lock_conflict() {
    let (out, err) = on_stderr("error: ref-state precondition failed: heads/main advanced");
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "lock-conflict");
}

#[test]
fn a_missing_signing_key_is_not_ready() {
    let (out, err) = on_stderr("error: no signing key: MAINTAINER key not ready");
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "not-ready");
    assert!(e.to_string().contains("MAINTAINER"));
}

#[test]
fn an_integrity_finding_routes_only_for_the_integrity_category() {
    let msg = "error: verify finding: block 76cee1dc has an unverifiable author signature";
    // During an integrity read, it is an integrity finding.
    let as_integrity = classify("", msg, RequestCategory::Integrity);
    assert_eq!(as_integrity.class(), "integrity-finding");
    // The same text outside an integrity request stays a plain refusal (context disambiguates).
    let as_read = classify("", msg, RequestCategory::ReadHistory);
    assert_eq!(as_read.class(), "refusal");
}

#[test]
fn a_schema_skew_message_stays_a_refusal_not_an_integrity_finding() {
    // RFC 012 F-e: the handoff's explicit caution — do NOT widen `is_integrity_finding` to catch this.
    // It would route ordinary version skew into the (nonexistent) Verify view instead of an explanation,
    // and would catch genuine integrity findings in read contexts too. The gloss/next-step for this
    // shape belong in `present()` (message-shape recognition on an already-correct `Refusal`), never
    // here in the classifier. Message captured live from a real prikk 0.30.0 reading a repository a
    // real prikk 0.31.0 had written.
    let msg = "error: integrity error: format-2 patch does not accept envelope schema 3 \
               (accepted: [1, 2])";
    let as_read = classify("", msg, RequestCategory::ReadHistory);
    assert_eq!(as_read.class(), "refusal");
    // Even during an integrity read, this is not a *finding* — it is stikk failing to read at all.
    let as_integrity = classify("", msg, RequestCategory::Integrity);
    assert_eq!(as_integrity.class(), "refusal");
}

#[test]
fn an_unrecognized_message_degrades_to_a_verbatim_refusal() {
    // RR-5 / NFR-I03: never a fabricated specific class, never a dropped message.
    let (out, err) = on_stderr("error: something entirely new prikk started saying");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "refusal");
    assert!(e.to_string().contains("something entirely new"));
}

#[test]
fn stdout_is_used_when_stderr_is_empty() {
    let e = classify(
        "error: ref \"tags/gone\" does not exist\n",
        "",
        RequestCategory::ReadHistory,
    );
    assert_eq!(e.class(), "refusal");
    assert!(e.to_string().contains("tags/gone"));
}

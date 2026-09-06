//! Golden-message tests for the failure classifier (design TS-03; RFC 007; RFC 017).
//!
//! Every message below is either **captured live** against a real prikk 0.33.0 binary (the provenance
//! comment names the exact command) or **read from prikk's source** at the `0.33.0` tag with a
//! `file:line` citation, never composed from prikk's documentation or prose (RFC 017 F2's own defect:
//! five arms matched strings nobody had ever captured, some of them never emitted at any version stikk
//! has supported). Where a test is source-read rather than live-provoked, its comment says so — the
//! review request for this increment reports which is which and why.

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
fn a_foreign_directory_is_environment_via_the_no_such_file_arm_not_the_deleted_one() {
    // RFC 017 §2 row 1/2, F3: captured live — `prikk status` in an empty directory, prikk 0.33.0 —
    // `not a prikk repository` is confirmed absent from every prikk output (never emitted at any
    // version) and has been deleted. A foreign directory is still classified `environment`, but only
    // because the discovery failure surfaces as `i/o error: No such file or directory`, which the
    // `no such file` arm below catches — correct by accident until this increment named which arm
    // actually does it.
    let (out, err) = on_stderr("error: i/o error: No such file or directory (os error 2)");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "environment");
    assert!(e.to_string().contains("No such file or directory"));
}

#[test]
fn not_a_prikk_repository_is_no_longer_matched_because_prikk_never_says_it() {
    // The deleted arm's own string, proven to degrade safely (RFC 017 decision 1's own test
    // requirement) — nothing has ever emitted this, so this test exists only to show the degradation
    // is safe, not to defend a real shape.
    let (out, err) = on_stderr("error: not a prikk repository (no .prikk directory found)");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn a_permission_fault_is_environment() {
    // Captured live: `chmod 000 .prikk && prikk status`, prikk 0.33.0.
    let (out, err) = on_stderr("error: i/o error: Permission denied (os error 13)");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "environment");
}

#[test]
fn unsupported_prikk_version_and_could_not_prikk_are_no_longer_matched() {
    // RFC 017 F2: both confirmed absent from prikk's own output — `"unsupported prikk version"` is
    // stikk's own version-gate wording (never prikk's), and no message anywhere pairs `"could not"`
    // with the literal word `"prikk"`. Proven safe to have deleted: an unrelated `"could not"` message
    // now degrades to a verbatim refusal rather than a guessed environment fault.
    let (out, err) = on_stderr("error: could not do the thing prikk asked for");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn a_retired_repository_format_is_environment_with_the_real_migration_message() {
    // RFC 017 F2/§5: the old fixture asserted `run \`prikk migrate\` to upgrade` — prikk has no
    // `migrate` command and never has. This is captured **live**: a real repository was built with a
    // real prikk 0.19.0 binary (`prikk init` at the `0.19.0` tag — format 2 was current then; RFC 017
    // says it was retired "after 0.19.0"), then read with the real prikk 0.33.0 binary via `prikk
    // status`. prikk's real message says migration is **not supported**, the opposite of the deleted
    // fixture's claim.
    let (out, err) = on_stderr(
        "error: integrity error: this repository uses format 2, which prikk no longer supports \
         (this version requires format 6). format-2 support was removed after 0.19.0; migration from \
         format 2 is not supported.",
    );
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "environment");
    assert!(
        e.to_string()
            .contains("migration from format 2 is not supported")
    );
}

#[test]
fn the_invented_migrate_command_fixture_no_longer_matches_anything_it_used_to() {
    // The deleted arm's own invented string (RFC 017 F2's "worst of it"): proven to degrade safely
    // rather than silently keep asserting a migration path prikk does not offer.
    let (out, err) =
        on_stderr("error: retired repository format 2; run `prikk migrate` to upgrade");
    let e = classify(out, err, RequestCategory::ReadHistory);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn a_cross_ref_commit_refusal_at_0_33_0_is_still_not_a_lock_conflict() {
    // RFC 017 F1's regression guard: prikk 0.33.0 reworded this message's class prefix from
    // `lock conflict:` to `precondition not met:` (our own upstream report, acted on within a day).
    // Captured live: `prikk commit --from-worktree --ref heads/other -m x` against a repository whose
    // active WAL owns `heads/main`, prikk 0.33.0. The match never depended on either class word.
    let (out, err) = on_stderr(
        "error: precondition not met: active WAL is owned by heads/main; requested ref heads/other",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "cross-ref");
    assert!(e.to_string().contains("active WAL is owned by"));
}

#[test]
fn a_genuine_held_lock_is_still_a_lock_conflict() {
    // Captured live: an `active.lock` file pre-placed at the path prikk expects, then
    // `prikk commit --from-worktree --ref heads/main -m x`, prikk 0.33.0.
    let (out, err) = on_stderr(
        "error: lock conflict: active lock already exists: /tmp/repo/.prikk/active/default/active.lock",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "lock-conflict");
}

#[test]
fn a_lock_authority_mismatch_is_a_lock_conflict() {
    // Source-read, not live-provoked (`lock.rs`'s own literal): needs a second repository authority to
    // construct, which stikk has no UI path to build today. Quoted verbatim from
    // `prikk-store/src/lock.rs:52` at the `0.33.0` tag.
    let (out, err) =
        on_stderr("error: lock conflict: active lock belongs to a different repository authority");
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "lock-conflict");
}

#[test]
fn a_ref_cas_mismatch_is_a_lock_conflict() {
    // Source-read, not live-provoked (`refs.rs:450` at the `0.33.0` tag) — a real CAS race needs two
    // genuinely concurrent mutations, timing-dependent to construct reliably in a test harness.
    let (out, err) = on_stderr(
        "error: lock conflict: ref CAS mismatch for heads/main: expected \"abc\", got \"def\"",
    );
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "lock-conflict");
}

#[test]
fn a_ref_moved_during_planning_is_a_lock_conflict() {
    // Source-read, not live-provoked (`rollback_draft.rs:165` at the `0.33.0` tag) — needs an in-flight
    // rollback draft, which stikk does not build yet.
    let (out, err) = on_stderr(
        "error: lock conflict: rollback-draft target ref changed during planning; retry rollback-draft",
    );
    let e = classify(out, err, RequestCategory::Recovery);
    assert_eq!(e.class(), "lock-conflict");
}

#[test]
fn another_writer_and_ref_state_precondition_are_no_longer_matched() {
    // RFC 017 F2: both confirmed absent — they appear only in `PrikkError`'s own doc comments,
    // describing the class, never as actual output text.
    for msg in [
        "error: lock conflict: another writer holds the lock",
        "error: lock conflict: ref-state precondition failed",
    ] {
        let (out, err) = on_stderr(msg);
        // Both still carry the `lock conflict:` class prefix, which is no longer sufficient alone —
        // proving the narrowing actually narrowed, not merely renamed the same broad match.
        let e = classify(out, err, RequestCategory::Publication);
        assert_eq!(
            e.class(),
            "refusal",
            "expected {msg:?} to degrade, got {e:?}"
        );
    }
}

#[test]
fn the_full_queue_precondition_is_no_longer_a_lock_conflict_the_live_defect() {
    // RFC 017 F4, the acceptance-critical fix. Captured live: `PRIKK_ACTIVE_PATCH_WARN=1
    // PRIKK_ACTIVE_PATCH_LIMIT=1 prikk commit --from-worktree --ref heads/main -m x` on a repository
    // already at the limit, prikk 0.33.0. This is the ordinary commit path, shipped in 0.4.0's
    // candidate — nothing is locked and no other writer exists, and it must never render `FR-106`'s
    // "another writer is active" gloss again.
    let (out, err) = on_stderr(
        "error: lock conflict: active WAL has 1 queued patches, at or above the configured limit (1); \
         run `prikk seal` before committing again",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn the_active_rs_full_queue_wording_also_falls_through() {
    // `active.rs:87`'s own wording differs at the tail ("run doctor or seal" vs "run `prikk seal`") —
    // source-read, not live-provoked (this path is reached by rollback-draft-append, not commit, which
    // stikk does not build yet). Both wordings share "queued patches"/"configured limit", so neither
    // matches the narrowed lock-conflict arm.
    let (out, err) = on_stderr(
        "error: lock conflict: active WAL has 64 queued patches, at or above the configured limit \
         (64); run doctor or seal before appending again",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn the_other_five_captured_preconditions_are_refusals_not_lock_conflicts() {
    // RFC 017 F4's remaining five sites: `refs.rs:133`, `rollback_verify.rs:178`,
    // `seal_from_accepted.rs:189`, and `rollback_draft.rs:158` at the `0.33.0` tag — all source-read,
    // none live-provoked (none has a UI path in stikk today: no Queue/Seal ceremony, no rollback flow,
    // no sync). All must fall through to a verbatim refusal, exactly like the full-queue case, since
    // none is a lock and none has a view to route into yet (RFC 017 §3's rule, applied uniformly).
    for msg in [
        "error: lock conflict: repository mutation is blocked by incomplete ref publication; run \
         verify/doctor and use signer-backed seal retry",
        "error: lock conflict: rollback-draft-verify requires an active WAL containing only the \
         rollback draft",
        "error: lock conflict: sealing from an accepted claim requires an empty active WAL -- seal or \
         discard local work first",
        "error: lock conflict: rollback-draft requires an empty active WAL",
    ] {
        let (out, err) = on_stderr(msg);
        let e = classify(out, err, RequestCategory::QueueMutation);
        assert_eq!(
            e.class(),
            "refusal",
            "expected {msg:?} to degrade, got {e:?}"
        );
    }
}

#[test]
fn a_missing_signing_key_is_not_ready_for_either_role() {
    // Captured live: `prikk commit` / `prikk seal --allow-no-audit` with no `PRIKK_*_KEY_ID` set,
    // prikk 0.33.0. Both roles produce the identical shape.
    for msg in [
        "error: author signing is required: set PRIKK_AUTHOR_KEY_ID (no signing key configured)",
        "error: maintainer signing is required: set PRIKK_MAINTAINER_KEY_ID (no signing key \
         configured)",
    ] {
        let (out, err) = on_stderr(msg);
        let e = classify(out, err, RequestCategory::Trust);
        assert_eq!(
            e.class(),
            "not-ready",
            "expected {msg:?} to be not-ready, got {e:?}"
        );
    }
}

#[test]
fn not_ready_and_key_not_adopted_are_no_longer_matched() {
    // RFC 017 F2: neither string appears anywhere in prikk's source. The redundant `"maintainer"` +
    // `"required"` clause is also gone — the real message already satisfies `"no signing key"`, so it
    // added nothing.
    let (out, err) = on_stderr("error: some other maintainer condition is required and not met");
    let e = classify(out, err, RequestCategory::Trust);
    assert_eq!(e.class(), "refusal");
}

#[test]
fn a_trust_refusal_is_not_ready_for_both_captured_wordings() {
    // RFC 017 F5/§6, captured live: `prikk seal --allow-no-audit` with a different (untrusted)
    // `PRIKK_MAINTAINER_KEY_ID`/`_SEED` pair than the one `prikk setup` adopted, prikk 0.33.0 — both
    // variants reached by using a different key id, and by reusing the trusted id with a different seed.
    for msg in [
        "error: invalid signature: maintainer signer key id different-maintainer is not trusted by \
         policy",
        "error: invalid signature: maintainer signer public key does not match trusted key maintainer",
    ] {
        let (out, err) = on_stderr(msg);
        let e = classify(out, err, RequestCategory::Publication);
        assert_eq!(
            e.class(),
            "not-ready",
            "expected {msg:?} to be not-ready, got {e:?}"
        );
    }
}

#[test]
fn an_integrity_finding_no_longer_exists_every_string_degrades_in_every_category() {
    // RFC 017 F2/§3: none of `"finding"`, `"verify"`, `"doctor"` was ever founded on real `verify`
    // output — a real `prikk verify`/`prikk doctor` run against a clean repository (prikk 0.33.0)
    // produces only success text (exit 0), never reaching the classifier at all, and corrupting a real
    // repository object to force a genuine finding was judged disproportionate to this increment's
    // scope (RFC 017 §3: narrow to what can be captured, and this could not be, cheaply). The arm is
    // removed entirely rather than kept on a guess — `FR-100`'s Verify view does not exist yet, and an
    // arm routing into a view that is not built is worse than none.
    let msg = "error: verify finding: block 76cee1dc has an unverifiable author signature";
    for category in [RequestCategory::Integrity, RequestCategory::ReadHistory] {
        let (out, err) = on_stderr(msg);
        let e = classify(out, err, category);
        assert_eq!(
            e.class(),
            "refusal",
            "expected {category:?} to degrade, got {e:?}"
        );
    }
}

#[test]
fn a_schema_skew_message_stays_a_refusal_not_an_integrity_finding() {
    // RFC 012 F-e / RFC 017 F0: `integrity error:` is now known to be the prefix of *both* schema skew
    // and retired formats, neither of which is a verify finding — sharpening the caution this test
    // already enforced. The gloss/next-step for this shape belong in `present()` (message-shape
    // recognition on an already-correct `Refusal`), never here. Message captured live from a real
    // prikk 0.30.0 reading a repository a real prikk 0.31.0 had written (RFC 012).
    let msg = "error: integrity error: format-2 patch does not accept envelope schema 3 \
               (accepted: [1, 2])";
    let as_read = classify("", msg, RequestCategory::ReadHistory);
    assert_eq!(as_read.class(), "refusal");
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

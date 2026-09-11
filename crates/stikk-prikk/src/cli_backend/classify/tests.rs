//! Golden-message tests for the failure classifier (design TS-03; RFC 007; RFC 017).
//!
//! Every message below is either **captured live** against a real prikk binary (the provenance comment
//! names the exact command and version) or **read from prikk's source** at a named tag with a
//! `file:line` citation, never composed from prikk's documentation or prose (RFC 017 F2's own defect:
//! five arms matched strings nobody had ever captured, some of them never emitted at any version stikk
//! has supported). Where a test is source-read rather than live-provoked, its comment says so — the
//! review request for that increment reports which is which and why.
//!
//! Two tags are cited here. **0.33.0** is RFC 017's, when this file was written. **0.38.0** is
//! RFC 021's re-baseline, which added the second class word for the six reclassified preconditions,
//! the four-genuine-locks correspondence, and the symlink-authoring refusal — all re-read or
//! re-captured at that tag, and each marked accordingly rather than the file's provenance being
//! re-stamped wholesale.

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
fn seals_own_cross_ref_wording_also_classifies_cross_ref_rfc_016_f4() {
    // RFC 016 F4/§6: seal's cross-ref refusal is worded differently from commit's in **both** the
    // class prefix (seal carries none at 0.33.0, where commit's became `precondition not met:`) and
    // the trailing clause (`"requested seal ref is …"` vs commit's `"requested ref …"`). Captured live:
    // `prikk seal --allow-no-audit --ref heads/other` against a repository whose active WAL owns
    // `heads/main`, prikk 0.33.0 — no class prefix at all, confirmed byte-for-byte at 0.28.0 too. The
    // old two-clause match (`"active wal is owned by"` **and** `"requested ref"`) would never have
    // fired here, since seal's trailing text is "requested seal ref is", not "requested ref" — this is
    // the regression test for the widening in `is_cross_ref_conflict`.
    let (out, err) =
        on_stderr("error: active WAL is owned by heads/main; requested seal ref is heads/other");
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "cross-ref");
    assert!(e.to_string().contains("requested seal ref is"));
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

// RFC 021 §3/§4 — **both class words are live inside the supported range.** prikk 0.35 reclassified
// all six of the precondition sites RFC 017 F4 found from `PrikkError::LockConflict` to
// `PrikkError::Precondition`, which renders `precondition not met:` instead of `lock conflict:`. The
// message text is byte-identical on both sides; only the class word moved. stikk supports 0.28 through
// 0.38, so a user on 0.34 sees the first wording and a user on 0.35+ sees the second — **the fixtures
// below therefore carry both, rather than the older being replaced by the newer.** Replacing would
// silently drop the half of the range that is still reachable.
//
// Which release: bisected live across real 0.33.0/0.34.0/0.35.0/0.36.0/0.37.0/0.38.0 binaries on
// 2026-09-12 using the full-queue commit path (the one site with a stikk UI path) — `lock conflict:`
// through 0.34.0, `precondition not met:` from 0.35.0 on. prikk's own
// `tests/rfc132_part2_precondition_prefix.rs` at the 0.38.0 tag documents the change as RFC 132 part 2
// and names all six sites.
//
// That this changes no stikk behaviour is the point RFC 017 F1/F4 built for: the classifier matches
// each message's own semantic clause, never the class prefix. Each test below asserts *both* wordings
// reach the same class, which is the claim that has to hold, not that either one parses.
const FULL_QUEUE_LOCK_WORDING: &str = "error: lock conflict: active WAL has 1 queued patches, at or above the configured limit (1); \
     run `prikk seal` before committing again";
const FULL_QUEUE_PRECONDITION_WORDING: &str = "error: precondition not met: active WAL has 1 queued patches, at or above the configured limit \
     (1); run `prikk seal` before committing again";

#[test]
fn the_full_queue_precondition_is_no_longer_a_lock_conflict_the_live_defect() {
    // RFC 017 F4, the acceptance-critical fix. Captured live twice: at prikk 0.33.0 (RFC 017) and
    // again at prikk 0.38.0 (RFC 021), both with `PRIKK_ACTIVE_PATCH_WARN=1
    // PRIKK_ACTIVE_PATCH_LIMIT=1 prikk commit --from-worktree --ref heads/main -m x` on a repository
    // already at the limit. This is the ordinary commit path, shipped since 0.4.0 — nothing is locked
    // and no other writer exists, and it must never render `FR-106`'s "another writer is active"
    // gloss again, under either class word.
    for msg in [FULL_QUEUE_LOCK_WORDING, FULL_QUEUE_PRECONDITION_WORDING] {
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
fn the_active_rs_full_queue_wording_also_falls_through() {
    // `active.rs`'s own copy of the same check words its tail differently ("run doctor or seal" vs
    // "run `prikk seal`") — source-read at both the 0.33.0 and 0.38.0 tags, never live-provoked, since
    // it is reached by rollback-draft-append rather than commit and stikk builds neither. All four
    // wordings share "queued patches"/"configured limit", so none matches the narrowed lock-conflict
    // arm.
    for msg in [
        "error: lock conflict: active WAL has 64 queued patches, at or above the configured limit \
         (64); run doctor or seal before appending again",
        "error: precondition not met: active WAL has 64 queued patches, at or above the configured \
         limit (64); run doctor or seal before appending again",
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
fn the_other_four_captured_preconditions_are_refusals_under_either_class_word() {
    // RFC 017 F4's remaining four sites: `refs.rs`, `rollback_verify.rs`, `seal_from_accepted.rs`, and
    // `rollback_draft.rs` — source-read at the 0.33.0 tag and re-read at 0.38.0, where all four now
    // construct `PrikkError::Precondition`. **`rollback-draft`'s is additionally captured live at
    // 0.38.0** (`prikk rollback-draft --append-inverse --ref heads/main -m undo` over a non-empty
    // active WAL), which is the one of the four that has a CLI path at all; the other three have no
    // stikk UI path either (RFC 016 builds only the ordinary seal ceremony, not rollback-verify or
    // sync's accepted-claim seal). All must fall through to a verbatim refusal under both class
    // words, since none is a lock and none has a view to route into yet (RFC 017 §3's rule, applied
    // uniformly).
    //
    // *(This test was named "other five" and listed four sites — an off-by-one in RFC 017's own
    // bookkeeping, corrected here while re-reading the same sites at 0.38. Two full-queue sites plus
    // these four are the six prikk's RFC 132 part 2 reclassified.)*
    for tail in [
        "repository mutation is blocked by incomplete ref publication; run verify/doctor and use \
         signer-backed seal retry",
        "rollback-draft-verify requires an active WAL containing only the rollback draft",
        "sealing from an accepted claim requires an empty active WAL -- seal or discard local work \
         first",
        "rollback-draft requires an empty active WAL",
    ] {
        for prefix in ["error: lock conflict: ", "error: precondition not met: "] {
            let msg = format!("{prefix}{tail}");
            let (out, err) = on_stderr(&msg);
            let e = classify(out, err, RequestCategory::QueueMutation);
            assert_eq!(
                e.class(),
                "refusal",
                "expected {msg:?} to degrade, got {e:?}"
            );
        }
    }
}

#[test]
fn the_symlink_authoring_refusal_degrades_verbatim() {
    // RFC 021 §6, carried against RFC 014 rather than acted on. Captured live at prikk 0.38.0:
    // `ln -sf real.txt link.txt` in the worktree, then `prikk commit --from-worktree --ref
    // heads/main -m x`. Two facts worth pinning together: `worktree-status` reports the symlink as an
    // ordinary `untracked` path with `unsupported paths: 0` — it does **not** mark the path commit
    // will then refuse over — and the commit refusal is an `integrity error:` by class word while
    // being a precondition in substance.
    //
    // stikk must not read the `integrity error:` prefix as `IntegrityFinding`: RFC 017 §3 removed that
    // arm precisely because it had no captured evidence and no view to route into, and routing this
    // into a verify report would be a confident wrong picture. Degrading to prikk's own words is the
    // honest answer, and this test is what keeps it that way.
    let (out, err) = on_stderr(
        "error: integrity error: worktree authoring: unsupported symlink authoring: link.txt: \
         worktree symlink authoring is out of scope",
    );
    let e = classify(out, err, RequestCategory::QueueMutation);
    assert_eq!(e.class(), "refusal");
    assert!(
        e.to_string()
            .contains("worktree symlink authoring is out of scope")
    );
}

#[test]
fn the_four_genuine_locks_prikk_still_constructs_at_0_38_all_classify_lock_conflict() {
    // RFC 021's convergence finding. RFC 017 F4 narrowed `is_lock_conflict` by hand, from 0.33's
    // taxonomy, to the four clauses that name an actual lock — while prikk still carried ten
    // `lock conflict:` sites. prikk 0.35 then reclassified the other six upstream, and at the 0.38.0
    // tag `PrikkError::LockConflict` has **exactly four** construction sites left: `lock.rs` ×2
    // (`{kind} lock already exists: {path}`, `active lock belongs to a different repository
    // authority`), `refs.rs` (`ref CAS mismatch for …`), and `rollback_draft.rs` (`rollback-draft
    // target ref changed during planning; retry rollback-draft`).
    //
    // **Those are the same four this arm matches** — stikk's hand-narrowing and prikk's own taxonomy
    // arrived at the same set from opposite directions. This test pins that correspondence so a future
    // reclassification shows up here rather than as a silently mis-glossed refusal.
    for msg in [
        "error: lock conflict: active lock already exists: /tmp/repo/.prikk/active/default/active.lock",
        "error: lock conflict: active lock belongs to a different repository authority",
        "error: lock conflict: ref CAS mismatch for heads/main: expected \"abc\", got \"def\"",
        "error: lock conflict: rollback-draft target ref changed during planning; retry rollback-draft",
    ] {
        let (out, err) = on_stderr(msg);
        let e = classify(out, err, RequestCategory::Publication);
        assert_eq!(
            e.class(),
            "lock-conflict",
            "expected {msg:?} to stay a lock conflict, got {e:?}"
        );
    }
}

#[test]
fn seals_empty_queue_refusal_degrades_to_a_verbatim_refusal() {
    // RFC 016 §2/§6: captured live — `prikk seal --allow-no-audit` with nothing queued, byte-identical
    // at both prikk 0.28.0 and 0.33.0. Not among RFC 017 F4's six preconditions (a distinct refusal,
    // never carrying prikk's `lock conflict:`/`precondition not met:` class word at either version) —
    // stikk prevents this client-side from the queue count it already knows (RFC 016 decision 4), and
    // the classifier's job is only to make the rare race that survives prevention land honestly.
    let (out, err) = on_stderr("error: active WAL has no patch records to seal");
    let e = classify(out, err, RequestCategory::Publication);
    assert_eq!(e.class(), "refusal");
    assert!(e.to_string().contains("no patch records to seal"));
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

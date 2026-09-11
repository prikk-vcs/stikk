//! The real-binary integration suite's actual test functions (`TS-07`; RFC 019). See
//! `stikk_real_binary`'s crate-level doc for the boundary this holds and why it lives in its own crate.
//!
//! **Does not run by default.** Every test here is `#[ignore]`d except
//! [`a_deliberately_wrong_fixture_is_detected_as_a_mismatch`], which needs no real binary at all —
//! `cargo test --workspace --locked` runs that one and skips the rest, same as today.
//!
//! **To run the rest:** install a floor and a ceiling `prikk` binary —
//!
//! ```sh
//! read -r FLOOR CEILING < <(cargo run --quiet -p stikk-prikk --example print_version_matrix)
//! cargo install prikk --version "0.${FLOOR}.0"   --locked --root /tmp/prikk-floor
//! cargo install prikk --version "0.${CEILING}.0" --locked --root /tmp/prikk-ceiling
//! ```
//!
//! (**derived, not written** — the same form `.github/workflows/real-binary.yml` uses, and for the
//! reason the previous version of this comment proved: it hardcoded `0.28.0`/`0.33.0`, warned in the
//! next line that those "drift the moment RFC 019 §6 does its job", and then drifted at the very next
//! ceiling raise. RFC 021's sweep caught it. Today the numbers are 28 and 38) — then:
//!
//! ```sh
//! STIKK_TEST_PRIKK_FLOOR_BIN=/tmp/prikk-floor/bin/prikk \
//! STIKK_TEST_PRIKK_CEILING_BIN=/tmp/prikk-ceiling/bin/prikk \
//! cargo test -p stikk-real-binary -- --ignored --test-threads=1
//! ```
//!
//! **`--test-threads=1` is required, not a suggestion.** Several tests set this *process's*
//! environment variables (`PRIKK_*_SEED`/`PRIKK_*_KEY_ID`) to drive `CliBackend`'s real signing-readiness
//! gate, which reads the process environment directly by design (`stikk-prikk::env`) — that is not safe
//! to do from multiple threads at once. `ENV_LOCK` below guards it besides, in case the flag is
//! forgotten, but the flag is still the point: a lock only serializes access, it does not make a test
//! that ran under a *different* test's leftover environment correct.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Mutex;

use stikk_model::StikkError;
use stikk_prikk::{CliBackend, Prikk};
use stikk_real_binary::support::{Fixture, PrikkBin, assert_matches_fixture};

/// Serializes every test that touches process environment variables — see this file's own module doc.
///
/// Review C1: lock **recovery**, not just acquisition, matters here. Every call site takes the guard
/// with `unwrap_or_else(PoisonError::into_inner)` rather than `unwrap()` — a panicked test must not
/// poison this mutex for every test that runs after it, or one real failure (the case this suite exists
/// to surface) turns into a cascade of `PoisonError`s that hide the four other real failures sitting
/// behind it. Recovery is safe: every test that reaches this lock immediately clears, then sets, only
/// the environment it needs (`Fixture::clear_env` at both entry and exit — see each test), so a guard
/// inherited from a panicked predecessor carries no state that matters.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// `init → commit → seal → verify`, asserting repository **state** by re-reading it through
/// `CliBackend`, never by trusting the very `CommitResult`/`SealResult` under test (RFC 019 F2's last
/// row — the half nothing has ever covered) — at both ends of the supported range in one test, since
/// both need the same shape of assertion and nothing about it varies by version.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn full_lifecycle_asserts_repository_state_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Deterministic starting state regardless of how a predecessor under this same guard died.
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        let before = backend
            .orientation(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: orientation before commit: {e}", bin.minor));
        assert_eq!(
            before.queued_patches, 0,
            "0.{}: a fresh repo's queue should be empty",
            bin.minor
        );

        fixture.set_author_env();
        let commit = backend
            .commit(fixture.repo(), "heads/main", "first commit")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();
        assert_eq!(commit.baseline_ref, "heads/main", "0.{}", bin.minor);

        // The point of this test: re-read, don't trust `commit`'s own parsed result.
        let after_commit = backend
            .orientation(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: orientation after commit: {e}", bin.minor));
        assert_eq!(
            after_commit.queued_patches, 1,
            "0.{}: the queue should actually contain the patch, not just say it will",
            bin.minor
        );
        assert_eq!(
            after_commit.queued_target.as_deref(),
            Some("heads/main"),
            "0.{}",
            bin.minor
        );

        fixture.set_maintainer_env();
        let seal = backend
            .seal(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
        assert_eq!(seal.patches, 1, "0.{}", bin.minor);

        let after_seal = backend
            .orientation(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: orientation after seal: {e}", bin.minor));
        assert_eq!(
            after_seal.queued_patches, 0,
            "0.{}: the queue should be empty after seal",
            bin.minor
        );

        let history = backend
            .history(fixture.repo(), "heads/main", 10)
            .unwrap_or_else(|e| panic!("0.{}: history after seal: {e}", bin.minor));
        assert_eq!(
            history.blocks.len(),
            1,
            "0.{}: a new block should exist",
            bin.minor
        );
        assert_eq!(history.blocks[0].block_id, seal.block_id, "0.{}", bin.minor);
        assert_eq!(history.blocks[0].patches, 1, "0.{}", bin.minor);

        // `verify` itself isn't on the `Prikk` trait yet (no view consumes it — `FR-100`); confirming
        // the repository this suite just built and mutated verifies clean is a plain `Command` check.
        assert!(
            std::process::Command::new(&bin.path)
                .arg("verify")
                .current_dir(fixture.repo())
                .status()
                .unwrap_or_else(|e| panic!("0.{}: spawn prikk verify: {e}", bin.minor))
                .success(),
            "0.{}: the repository this suite just built and mutated should verify clean",
            bin.minor
        );
    }
}

/// RFC 014/016 decision: a cross-ref commit is **prevented** client-side (`stikk-core::commit::preview`)
/// rather than classified — built on reading prikk's source, never checked against prikk itself until
/// now. This bypasses the prevention entirely (calls `CliBackend::commit` directly, which implements no
/// prevention of its own — that lives one layer up, in `stikk-core`) and confirms prikk really does
/// refuse, at both ends of the range, with the wording each end actually uses (RFC 017 F1: the class
/// prefix moved between them; the clause matched on did not).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn commit_cross_ref_is_a_real_refusal_prikk_would_also_give() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Deterministic starting state regardless of how a predecessor under this same guard died.
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        fixture.set_author_env();
        // Put the active WAL's queue target on heads/main first — required before "wrong ref" means
        // anything (an empty queue has no owner to conflict with).
        backend
            .commit(fixture.repo(), "heads/main", "seed the queue")
            .unwrap_or_else(|e| panic!("0.{}: seeding commit: {e}", bin.minor));

        let result = backend.commit(fixture.repo(), "heads/other", "should refuse");
        Fixture::clear_env();

        let message = match result {
            Err(StikkError::CrossRef { message }) => message,
            Err(other) => panic!("0.{}: expected CrossRef, got {other:?}", bin.minor),
            Ok(_) => panic!("0.{}: a cross-ref commit must not succeed", bin.minor),
        };

        let fixture_text = if bin.minor >= 33 {
            "error: precondition not met: active WAL is owned by heads/main; requested ref heads/other"
        } else {
            "error: lock conflict: active WAL is owned by heads/main; requested ref heads/other"
        };
        assert_matches_fixture(
            &format!("commit cross-ref @ 0.{}", bin.minor),
            fixture_text,
            &message,
        );
    }
}

/// The clean-worktree half of commit's client-side prevention (`stikk-core::commit::preview`) — same
/// shape as the cross-ref test above.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn commit_on_a_clean_worktree_is_a_real_refusal_prikk_would_also_give() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Deterministic starting state regardless of how a predecessor under this same guard died.
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "author the only change")
            .unwrap_or_else(|e| panic!("0.{}: seeding commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seeding seal: {e}", bin.minor));
        // The worktree now matches the sealed baseline exactly — genuinely clean, not merely unsealed.
        fixture.set_author_env();
        let result = backend.commit(fixture.repo(), "heads/main", "nothing changed");
        Fixture::clear_env();

        let message = match result {
            Err(StikkError::Refusal { message }) => message,
            Err(other) => panic!("0.{}: expected Refusal, got {other:?}", bin.minor),
            Ok(_) => panic!("0.{}: a clean-worktree commit must not succeed", bin.minor),
        };

        let fixture_text = if bin.minor >= 33 {
            "error: precondition not met: worktree has no node-addressed changes to commit"
        } else {
            "error: invalid name: worktree has no node-addressed changes to commit"
        };
        assert_matches_fixture(
            &format!("commit clean-worktree @ 0.{}", bin.minor),
            fixture_text,
            &message,
        );
    }
}

/// RFC 016 decision 4: an empty-queue seal is prevented client-side (`stikk-core::seal::preview`).
/// Bypassed here the same way as the commit tests above.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn seal_on_an_empty_queue_is_a_real_refusal_prikk_would_also_give() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Deterministic starting state regardless of how a predecessor under this same guard died.
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        fixture.set_maintainer_env();
        let result = backend.seal(fixture.repo(), "heads/main");
        Fixture::clear_env();

        let message = match result {
            Err(StikkError::Refusal { message }) => message,
            Err(other) => panic!("0.{}: expected Refusal, got {other:?}", bin.minor),
            Ok(_) => panic!("0.{}: an empty-queue seal must not succeed", bin.minor),
        };

        assert_matches_fixture(
            &format!("seal empty-queue @ 0.{}", bin.minor),
            "error: active WAL has no patch records to seal",
            &message,
        );
    }
}

/// RFC 016 F4/decision 4: a cross-ref seal is prevented client-side. Seal's own wording is the one case
/// this project has found identical at both ends of the range, with no class prefix at all at either —
/// this test proves that byte-for-byte, not merely asserts it from a captured fixture written once.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn seal_cross_ref_is_a_real_refusal_prikk_would_also_give() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Deterministic starting state regardless of how a predecessor under this same guard died.
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "seed the queue")
            .unwrap_or_else(|e| panic!("0.{}: seeding commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        let result = backend.seal(fixture.repo(), "heads/other");
        Fixture::clear_env();

        let message = match result {
            Err(StikkError::CrossRef { message }) => message,
            Err(other) => panic!("0.{}: expected CrossRef, got {other:?}", bin.minor),
            Ok(_) => panic!("0.{}: a cross-ref seal must not succeed", bin.minor),
        };

        assert_matches_fixture(
            &format!("seal cross-ref @ 0.{}", bin.minor),
            "error: active WAL is owned by heads/main; requested seal ref is heads/other",
            &message,
        );
    }
}

/// RFC 019 §5/§9: "a capture-and-diff suite that has never been seen to fail is a hypothesis." Needs no
/// real binary — proves [`assert_matches_fixture`] actually panics, and prints both sides, on a genuine
/// mismatch, rather than only ever being exercised on the happy path where fixture and live agree. Not
/// `#[ignore]`d: this runs in the ordinary `cargo test --workspace --locked` gate, same as any other
/// unit test, since it needs nothing external.
#[test]
#[should_panic(expected = "no longer matches the committed fixture")]
fn a_deliberately_wrong_fixture_is_detected_as_a_mismatch() {
    assert_matches_fixture(
        "deliberate mismatch",
        "error: active WAL has no patch records to seal",
        "error: active WAL has a suspicious number of patch records to seal",
    );
}

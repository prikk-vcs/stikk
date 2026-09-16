//! The real-binary integration suite's actual test functions (`TS-07`; RFC 019). See
//! `stikk_real_binary`'s crate-level doc for the boundary this holds and why it lives in its own crate.
//!
//! **Coverage: all twelve `Prikk` seam methods, at both ends of the supported range** (RFC 022 §2).
//! `handshake` is driven by the version guard every test runs through [`PrikkBin::resolve`];
//! `orientation`, `history`, `commit` and `seal` since RFC 019; `worktree_status`, `block_state`,
//! `refs`, `tags` and `change_token` since RFC 022; `readiness` since 0.6.0's preparation; and `queue`
//! since RFC 028. (0.5.0's changelog said "four of the nine
//! surfaces" — it undercounted `handshake` and there were always ten, not nine; the correction is in
//! `## Unreleased`.) Two failure-classifier arms are provoked here as well rather than only cited: a
//! genuinely held lock, and the full-queue precondition prikk 0.35 reclassified — see
//! `classify.rs`'s `is_lock_conflict`, whose other two arms carry reachability notes instead.
//!
//! **The discipline throughout is RFC 019's**: assert what the repository *became*, re-read afterwards,
//! not that stikk's parse of a string succeeded. A test that only proves a string parsed would have
//! passed against every defect this suite exists to catch.
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
//! ceiling raise. RFC 021's sweep caught it. Today the numbers are 28 and 42) — then:
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

// ---------------------------------------------------------------------------------------------
// RFC 022 §2 — the five surfaces nothing had exercised.
//
// Same discipline as the four above, which is the reason this suite has ever found anything: assert
// what the **repository** is, re-read afterwards, not that a string parsed. A parser test proves stikk
// understood a capture; only a state assertion proves stikk understood the repository.
//
// **Each test builds its own `Fixture`** (RFC 022 Decision 5/F4). Measured on the 0.5.0 prep runs: a
// fixture repository costs **~0.3s**, ten of them 3.25s, inside jobs that take 64–116s end to end and
// are dominated by two `cargo install prikk` invocations. Sharing fixtures would couple tests — one
// test's mutation silently becoming another's precondition — to save something invisible next to the
// install. The number is here so the next person tempted to optimise this finds the reason rather than
// the opportunity.
//
// **Re-measured on the widened suite** (RFC 022, run `34675342447`): 13 tests, 25 fixture repositories,
// **0.87s on ubuntu, 4.23s on macOS, 9.84s on Windows** — the whole body, both prikk ends, per platform.
// Widening the suite by 8 tests cost under 10s on the slowest platform, inside jobs still dominated by
// the two `cargo install prikk` invocations. The expensive part was already paid, and still is.
// ---------------------------------------------------------------------------------------------

/// `refs` and `tags`, after a real `branch create`, `branch close` and `tag create`.
///
/// The names asserted are ones this test created, so the expectation comes from what it did rather
/// than from a fixture's say-so. `tag create` needs MAINTAINER readiness, which `Fixture` establishes.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn refs_and_tags_report_what_the_repository_actually_has() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        // A ref only exists once something is sealed onto it.
        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "seed")
            .unwrap_or_else(|e| panic!("0.{}: seeding commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seeding seal: {e}", bin.minor));

        for args in [
            vec!["branch", "create", "heads/feature", "--from", "heads/main"],
            vec!["branch", "create", "heads/retired", "--from", "heads/main"],
            vec!["branch", "close", "heads/retired"],
            vec![
                "tag",
                "create",
                "tags/v1",
                "--target",
                "heads/main",
                "-m",
                "one",
            ],
        ] {
            let out = std::process::Command::new(&bin.path)
                .args(&args)
                .current_dir(fixture.repo())
                .output()
                .unwrap_or_else(|e| panic!("0.{}: spawn prikk {args:?}: {e}", bin.minor));
            assert!(
                out.status.success(),
                "0.{}: prikk {args:?} failed: {}",
                bin.minor,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Fixture::clear_env();

        let refs = backend
            .refs(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor));
        let named = |n: &str| refs.iter().find(|r| r.name == n).cloned();

        assert!(
            named("heads/main").is_some(),
            "0.{}: refs {refs:?}",
            bin.minor
        );
        assert!(
            named("heads/feature").is_some(),
            "0.{}: refs {refs:?}",
            bin.minor
        );

        // The closed branch is reported the way `--all` reports it — present, and marked.
        let retired = named("heads/retired")
            .unwrap_or_else(|| panic!("0.{}: closed branch absent from refs {refs:?}", bin.minor));
        assert!(
            retired.closed,
            "0.{}: a closed branch must be reported closed, got {retired:?}",
            bin.minor
        );
        assert!(
            !named("heads/feature").expect("present").closed,
            "0.{}: an open branch must not be reported closed",
            bin.minor
        );

        let tags = backend
            .tags(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: tags: {e}", bin.minor));
        assert!(
            tags.iter().any(|t| t.name == "tags/v1"),
            "0.{}: the created tag is absent from tags {tags:?}",
            bin.minor
        );
        // `refs` and `tags` are separate reads of separate prikk commands (RFC 009 F3): a tag must not
        // be silently missing from the one that is supposed to list it.
        assert!(
            tags.iter().all(|t| t.is_tag()),
            "0.{}: tags returned a non-tag ref: {tags:?}",
            bin.minor
        );
    }
}

/// `block_state` on a block this test sealed itself, so the expected paths are known from what was
/// committed rather than from a fixture.
///
/// **This test found a real upstream defect on its first matrix run** (RFC 022, run `34675099061`):
/// **prikk 0.28 cannot commit a file in a subdirectory on Windows.** Its commit-side worktree scan
/// built the repository path with `path.to_str()`, which yields `src\main.rs` on Windows, and prikk's
/// own `RepoPath::parse` then refuses it — `error: invalid name: backslashes are not allowed in
/// repository paths`. prikk fixed it in **0.29.0**, routing that call through the separator-safe
/// `pathbuf_to_slash_string` the RFC 124 ignore-mechanism bug had already produced
/// (`crates/prikk-store/.../node_authoring/worktree_files.rs`, and their own comment says the call
/// "pre-dates RFC 124 … the same latent defect"). **Only the floor of stikk's supported range is
/// affected, and only on Windows.**
///
/// It is **not a stikk defect** — stikk passes no path here; `prikk commit` walks the worktree itself —
/// so nothing is worked around. The subdirectory half is skipped on that one combination and the skip
/// is **announced**, the same discipline F0's 0.28 skip follows: a test that quietly does less on one
/// platform is how a platform-specific hole stays invisible. The top-level file is still asserted at
/// both ends everywhere, so the test never becomes a no-op.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn block_state_lists_the_files_the_suite_itself_committed() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        // `Fixture` already wrote readme.txt; add a second file in a subdirectory so the assertion is
        // about a set of paths rather than a single one — except on the one combination where prikk
        // itself cannot do it (see this test's doc comment).
        let subdirectory_committable = !(cfg!(windows) && bin.minor == 28);
        if subdirectory_committable {
            std::fs::create_dir_all(fixture.repo().join("src"))
                .unwrap_or_else(|e| panic!("0.{}: mkdir: {e}", bin.minor));
            std::fs::write(fixture.repo().join("src/main.rs"), "fn main() {}\n")
                .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        } else {
            println!(
                "block_state: SKIPPING the subdirectory path at 0.28 on Windows — prikk 0.28's \
                 commit-side worktree scan builds `src\\main.rs` and its own RepoPath::parse refuses \
                 the backslash. Upstream, fixed in prikk 0.29.0; found by this suite's first matrix \
                 run. The top-level file is still asserted here. (RFC 022 §2.)"
            );
        }

        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "two files")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        let seal = backend
            .seal(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        let state = backend
            .block_state(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: block_state: {e}", bin.minor));

        // The replayed state is the block this test just sealed, not some other tip.
        assert_eq!(
            state.target_block, seal.block_id,
            "0.{}: block_state replayed a different block than seal reported",
            bin.minor
        );
        let mut files = state.files.clone();
        files.sort();
        let expected = if subdirectory_committable {
            vec!["readme.txt".to_string(), "src/main.rs".to_string()]
        } else {
            vec!["readme.txt".to_string()]
        };
        assert_eq!(
            files, expected,
            "0.{}: replayed state does not match what was committed",
            bin.minor
        );
        assert!(
            state.total_bytes > 0,
            "0.{}: two non-empty files replayed to zero bytes",
            bin.minor
        );
    }
}

/// `change_token` — the one surface whose contract is a **property**, not a value.
///
/// A token that never changes and a token that changes on every call both pass a one-sided test, so
/// both directions are asserted: **identical across a read, different across a mutation.** This is the
/// primitive every preview in `stikk-core` is gated on (`LC-4`, `FR-106`), so a token that lied in
/// either direction would either wave through a stale preview or refuse every fresh one.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn change_token_is_stable_across_a_read_and_moves_across_a_mutation() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        let before = backend
            .change_token(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: change_token: {e}", bin.minor));

        // Direction one: a pure read must not move it. `orientation` is a read of the same state the
        // token is composed from, so if anything could move it spuriously, this would.
        backend
            .orientation(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor));
        let after_read = backend
            .change_token(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: change_token after read: {e}", bin.minor));
        assert_eq!(
            before, after_read,
            "0.{}: a read moved the change token — every armed preview would refuse spuriously",
            bin.minor
        );

        // Direction two: a real mutation must move it. Commit alone changes the queue depth, which is
        // part of what the token composes.
        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "move the token")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();
        let after_commit = backend
            .change_token(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: change_token after commit: {e}", bin.minor));
        assert_ne!(
            before, after_commit,
            "0.{}: a commit did not move the change token — a stale preview would execute",
            bin.minor
        );

        // And a seal, which moves the ref pointer as well as emptying the queue.
        fixture.set_maintainer_env();
        backend
            .seal(fixture.repo(), "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
        let after_seal = backend
            .change_token(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: change_token after seal: {e}", bin.minor));
        assert_ne!(
            after_commit, after_seal,
            "0.{}: a seal did not move the change token",
            bin.minor
        );
    }
}

/// `worktree_status` on a genuinely dirty tree — modified, missing, and untracked at once.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn worktree_status_reports_the_dirty_tree_the_suite_created() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        std::fs::write(repo.join("gone.txt"), "delete me\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "baseline")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        // Clean first — the baseline for the dirty assertions below, and the case that would hide a
        // parser reporting everything as changed.
        let clean = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status (clean): {e}", bin.minor));
        assert!(
            clean.clean,
            "0.{}: freshly sealed tree is not clean: {clean:?}",
            bin.minor
        );
        assert!(
            clean.entries.is_empty(),
            "0.{}: {:?}",
            bin.minor,
            clean.entries
        );

        // Now make each of the three kinds happen, and assert the entry list matches the acts.
        std::fs::write(repo.join("readme.txt"), "changed\n")
            .unwrap_or_else(|e| panic!("0.{}: modify: {e}", bin.minor));
        std::fs::remove_file(repo.join("gone.txt"))
            .unwrap_or_else(|e| panic!("0.{}: delete: {e}", bin.minor));
        std::fs::write(repo.join("new.txt"), "new\n")
            .unwrap_or_else(|e| panic!("0.{}: add: {e}", bin.minor));

        let dirty = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status (dirty): {e}", bin.minor));
        assert!(!dirty.clean, "0.{}: dirty tree reported clean", bin.minor);

        let of = |kind: &str| -> Vec<String> {
            let mut v: Vec<String> = dirty
                .entries
                .iter()
                .filter(|e| e.kind == kind)
                .map(|e| e.path.clone())
                .collect();
            v.sort();
            v
        };
        assert_eq!(
            of("modified"),
            vec!["readme.txt".to_string()],
            "0.{}",
            bin.minor
        );
        assert_eq!(
            of("missing"),
            vec!["gone.txt".to_string()],
            "0.{}",
            bin.minor
        );
        assert_eq!(
            of("untracked"),
            vec!["new.txt".to_string()],
            "0.{}",
            bin.minor
        );

        // The counts prikk reports and the entries stikk parsed must agree — RFC 021 F0 was exactly a
        // disagreement between them that nothing checked.
        assert_eq!(dirty.modified, 1, "0.{}", bin.minor);
        assert_eq!(dirty.missing, 1, "0.{}", bin.minor);
        assert_eq!(dirty.untracked, 1, "0.{}", bin.minor);
        assert_eq!(
            dirty.entries.len() as u64,
            dirty.modified + dirty.missing + dirty.untracked + dirty.unsupported,
            "0.{}: entry count disagrees with prikk's own counts: {:?}",
            bin.minor,
            dirty.entries
        );
    }
}

/// **RFC 022 §3 — F0, provoked against a real binary rather than pinned by a capture.**
///
/// F0 is the one defect of its class that reached a shipped release: on prikk ≥ 0.38, `prikk mv` of a
/// path whose first whitespace-delimited token is a change kind made stikk report a **third** change
/// entry for a file that does not exist. It is pinned by a captured fixture in
/// `stikk-prikk`'s `parse/tests.rs`; this drives the whole path — real `mv`, real
/// `worktree-status`, real parser — so a future regression is caught against the binary and not only
/// against a string someone captured once.
///
/// **Version-gated, and the skip is announced.** `prikk mv` does not exist below 0.33, so the floor end
/// cannot provoke it. A test that quietly does nothing at one end is the inert-suite failure mode
/// RFC 019's own review warned about, so the skip prints.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn f0_a_renamed_kind_word_path_is_not_a_fabricated_entry() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let mut provoked_at = Vec::new();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if bin.minor < 38 {
            eprintln!(
                "F0: SKIPPED at 0.{} — `prikk mv` and the `live rename declarations:` section it \
                 populates do not exist below 0.38, so this defect is unreachable there. Announced \
                 rather than silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // The path whose first token is a change kind — the whole point of the defect.
        std::fs::write(repo.join("modified draft.txt"), "draft body\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "add draft")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        let mv = std::process::Command::new(&bin.path)
            .args(["mv", "modified draft.txt", "renamed.txt"])
            .current_dir(&repo)
            .output()
            .unwrap_or_else(|e| panic!("0.{}: spawn prikk mv: {e}", bin.minor));
        assert!(
            mv.status.success(),
            "0.{}: prikk mv failed: {}",
            bin.minor,
            String::from_utf8_lossy(&mv.stderr).trim()
        );

        let status = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status: {e}", bin.minor));

        // prikk reports two changes. Before F0's fix stikk reported three, the third
        // `modified "draft.txt -> renamed.txt"` — a file that does not exist, in a state prikk never
        // named.
        assert_eq!(
            status.entries.len(),
            2,
            "0.{}: prikk reported 2 changes; stikk parsed {:?}",
            bin.minor,
            status
                .entries
                .iter()
                .map(|e| (&e.kind, &e.path))
                .collect::<Vec<_>>()
        );
        assert!(
            !status.entries.iter().any(|e| e.path.contains("->")),
            "0.{}: a rename declaration was parsed as a change entry: {:?}",
            bin.minor,
            status.entries
        );
        assert!(
            status
                .entries
                .iter()
                .any(|e| e.kind == "missing" && e.path == "modified draft.txt")
        );
        assert!(
            status
                .entries
                .iter()
                .any(|e| e.kind == "untracked" && e.path == "renamed.txt")
        );
        provoked_at.push(bin.minor);
    }
    assert!(
        !provoked_at.is_empty(),
        "F0 was provoked at no version at all — the ceiling is below 0.38, so this test is inert and \
         is failing rather than passing silently (RFC 022 §3)."
    );
}

/// **RFC 027 F0, against real binaries at both ends: a path prikk cannot represent is listed.**
///
/// prikk names it `unsupported-path` at every tag from 0.28.0 to 0.41.0; stikk matched `unsupported` and
/// dropped the line, so the Changes header counted what its list never showed. A backslash in a file
/// name is the reachable case on Unix. The entry count is checked against prikk's own
/// `unsupported paths:` counter, which prikk computes from the list it prints.
///
/// **Skipped on Windows, and the skip is announced**: a backslash is the path separator there and cannot
/// be part of a name. A non-UTF-8 name is deliberately absent — macOS is expected to refuse creating one
/// and Windows cannot express one; the captured parser fixtures cover that shape.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn f0_an_unsupported_path_is_listed_as_prikk_counts_it() {
    if cfg!(windows) {
        eprintln!(
            "RFC 027 F0: SKIPPED on Windows — a backslash is the path separator there, so a file name \
             cannot contain one and prikk's `unsupported-path` entry cannot be provoked this way. \
             Announced rather than silent (RFC 022 §3)."
        );
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // A baseline to compare against: the fixture's `readme.txt`, committed and sealed.
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "first patch")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        std::fs::write(repo.join("back\\slash.txt"), "x\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));

        let status = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status: {e}", bin.minor));

        let unsupported: Vec<_> = status
            .entries
            .iter()
            .filter(|e| e.kind == "unsupported-path")
            .collect();
        assert_eq!(
            status.unsupported, 1,
            "0.{}: prikk counted {} unsupported paths; expected the one backslash name",
            bin.minor, status.unsupported
        );
        assert_eq!(
            u64::try_from(unsupported.len()).expect("fits"),
            status.unsupported,
            "0.{}: prikk's `unsupported paths:` is {}, stikk listed {:?}",
            bin.minor,
            status.unsupported,
            status.entries
        );
        assert!(
            unsupported[0].path.ends_with("back\\slash.txt"),
            "0.{}: the unsupported entry names {:?}",
            bin.minor,
            unsupported[0].path
        );
        assert!(
            !unsupported[0].note.is_empty(),
            "0.{}: prikk's reason should ride beside the entry",
            bin.minor
        );
    }
}

/// Create `link` as a symlink to `target`, or say why not — **measured, not assumed** (RFC 027 decision
/// 6): creating a symlink on Windows needs a privilege a CI runner may or may not hold.
fn make_symlink(target: &str, link: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
    }
}

/// **RFC 027 decisions 3, 5 and 6, against real binaries at both ends**: an untracked symlink beside an
/// ordinary modification — the tree prikk 0.41 refuses to commit.
///
/// - **≥ 0.39**: the entry is `Refused`, and **its reason equals what `prikk commit` prints for this same
///   tree** — compared against a real commit attempt made here, not a literal. `refused` is `Some(1)`,
///   commit's preview is unavailable carrying that path, and once the symlink is gone the preview is
///   ready.
/// - **Below 0.39**: every entry is `Unreported`, `refused` is `None`, the preview offers commit exactly
///   as before, and `commit` is refused with prikk's own message, verbatim.
///
/// **A platform that cannot create the symlink is announced as a skip**, with the OS error, rather than
/// assumed either way.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn a_refused_symlink_is_reported_prevented_and_matches_commits_own_refusal() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "baseline")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        std::fs::write(repo.join("readme.txt"), "hello again\n")
            .unwrap_or_else(|e| panic!("0.{}: modify: {e}", bin.minor));
        let link = repo.join("link.txt");
        if let Err(e) = make_symlink("readme.txt", &link) {
            eprintln!(
                "RFC 027: SKIPPED at 0.{} — this platform could not create a symlink ({e}), so the \
                 refused-symlink tree cannot be built here. Announced rather than silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }

        let status = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status: {e}", bin.minor));

        // prikk's own commit on this very tree — the string every verdict is compared against.
        fixture.set_author_env();
        let attempt = backend.commit(&repo, "heads/main", "would be refused");
        let preview = stikk_core::commit_preview(&backend, &repo, "heads/main");
        Fixture::clear_env();
        let message = match attempt {
            Err(StikkError::Refusal { message }) => message,
            other => panic!(
                "0.{}: expected prikk to refuse the commit, got {other:?}",
                bin.minor
            ),
        };

        if bin.minor >= 39 {
            assert_eq!(status.refused, Some(1), "0.{}: {status:?}", bin.minor);
            let entry = status
                .entries
                .iter()
                .find(|e| e.path == "link.txt")
                .unwrap_or_else(|| panic!("0.{}: link.txt not listed: {status:?}", bin.minor));
            let stikk_prikk::Authoring::Refused(reason) = &entry.authoring else {
                panic!("0.{}: link.txt is not refused: {entry:?}", bin.minor);
            };
            // prikk prints `error: <reason>`; the verdict is that reason, exactly.
            assert_eq!(
                message.strip_prefix("error: "),
                Some(reason.as_str()),
                "0.{}: worktree-status's verdict and commit's own refusal disagree",
                bin.minor
            );
            let readme = status
                .entries
                .iter()
                .find(|e| e.path == "readme.txt")
                .unwrap_or_else(|| panic!("0.{}: readme.txt not listed", bin.minor));
            assert_eq!(readme.authoring, stikk_prikk::Authoring::Authored);

            match preview {
                Ok(stikk_core::CommitPreviewOutcome::WouldRefuse(paths)) => assert_eq!(
                    paths,
                    vec![stikk_core::RefusedPath {
                        kind: stikk_core::ChangeKind::Untracked,
                        path: "link.txt".to_string(),
                        reason: reason.clone(),
                    }],
                    "0.{}",
                    bin.minor
                ),
                other => panic!("0.{}: expected WouldRefuse, got {other:?}", bin.minor),
            }

            // Remove the one refused path, and commit is available again.
            std::fs::remove_file(&link).unwrap_or_else(|e| panic!("0.{}: rm: {e}", bin.minor));
            fixture.set_author_env();
            let after = stikk_core::commit_preview(&backend, &repo, "heads/main");
            Fixture::clear_env();
            assert!(
                matches!(after, Ok(stikk_core::CommitPreviewOutcome::Ready { .. })),
                "0.{}: with the symlink removed the preview should be ready, got {after:?}",
                bin.minor
            );
        } else {
            assert_eq!(
                status.refused, None,
                "0.{}: unreported, never zero",
                bin.minor
            );
            assert!(
                status
                    .entries
                    .iter()
                    .all(|e| e.authoring == stikk_prikk::Authoring::Unreported),
                "0.{}: {:?}",
                bin.minor,
                status.entries
            );
            // Offered exactly as before: stikk infers no refusal prikk has not stated.
            assert!(
                matches!(preview, Ok(stikk_core::CommitPreviewOutcome::Ready { .. })),
                "0.{}: expected Ready, got {preview:?}",
                bin.minor
            );
            // And prikk's refusal, verbatim through the seam.
            assert_matches_fixture(
                &format!("commit refused symlink @ 0.{}", bin.minor),
                "error: integrity error: worktree authoring: unsupported symlink authoring: \
                 link.txt: worktree symlink authoring is out of scope",
                &message,
            );
        }
    }
}

/// **RFC 027 F6 against real binaries**: prikk's queued-elsewhere report arrives as prikk's own sentence
/// below 0.39, where stikk reads prose, and as the typed ref at 0.39 and above, where it reads JSON.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn queued_elsewhere_arrives_as_prikks_note_below_0_39_and_as_its_ref_above() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // One unsealed patch queued for heads/main; then ask about heads/other.
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "queued patch")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();

        let status = backend
            .worktree_status(&repo, "heads/other")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status: {e}", bin.minor));

        match (bin.minor >= 39, &status.queued_elsewhere) {
            (true, Some(stikk_prikk::QueuedElsewhere::Ref(queued))) => {
                assert_eq!(queued, "heads/main", "0.{}", bin.minor);
                assert_eq!(status.refused, Some(0), "0.{}", bin.minor);
            }
            (false, Some(stikk_prikk::QueuedElsewhere::Note(note))) => {
                assert!(
                    note.starts_with(
                        "note: the active WAL has queued (unsealed) patches for heads/main, not \
                         heads/other"
                    ),
                    "0.{}: {note:?}",
                    bin.minor
                );
                assert!(
                    note.contains("do not delete based on this report alone"),
                    "0.{}: {note:?}",
                    bin.minor
                );
            }
            (_, other) => panic!(
                "0.{}: expected the {} form, got {other:?}",
                bin.minor,
                if bin.minor >= 39 {
                    "typed-ref"
                } else {
                    "verbatim-note"
                }
            ),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 022 §4 — classifier arms, provoked rather than cited.
//
// The six precondition fixtures RFC 021 B moved by hand are the measure of success here: **the next
// re-baseline should find a class-word change without a person doing it.** These two tests are the
// reachable half of that — the arms with a real path through a `Prikk` seam method. The unreachable
// arms carry reachability notes in `classify.rs` instead (Q1 ruled (a): they stay, because `present()`
// adds no gloss to a `LockConflict` — it renders prikk's verbatim message in a banner, so the worst
// case from an unreachable arm is a banner instead of a refusal card, with nothing fabricated either
// way).
// ---------------------------------------------------------------------------------------------

/// The **full-queue precondition**, provoked through `CliBackend::commit` at both ends.
///
/// This is the one message of RFC 017 F4's six that has a live stikk path — the ordinary commit path,
/// shipped since 0.4.0 — and the one whose misclassification was the live defect that RFC caught:
/// rendering `FR-106`'s "another writer is active" over a queue that is merely full, with nothing
/// locked and no other writer.
///
/// **It also spans the class-word change.** prikk 0.35 reclassified it from `lock conflict:` to
/// `precondition not met:` with the text byte-identical; stikk's classifier matches the semantic clause
/// and not the prefix, so both ends must reach `refusal`. RFC 021 B established that by hand across six
/// binaries; this asserts it every run, which is the point of widening the suite.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn the_full_queue_precondition_classifies_refusal_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        fixture.set_author_env();
        backend
            .commit(fixture.repo(), "heads/main", "fill the queue")
            .unwrap_or_else(|e| panic!("0.{}: seeding commit: {e}", bin.minor));

        // Second commit against a queue already at the configured limit. The thresholds are prikk's
        // own environment knobs, not something stikk derives (RFC 014 F5).
        std::fs::write(fixture.repo().join("second.txt"), "second\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        // SAFETY: same discipline as `Fixture::set_*_env` — the caller holds `ENV_LOCK`, and both
        // variables are removed before it is released.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("PRIKK_ACTIVE_PATCH_WARN", "1");
            std::env::set_var("PRIKK_ACTIVE_PATCH_LIMIT", "1");
        }
        let result = backend.commit(fixture.repo(), "heads/main", "over the limit");
        // SAFETY: still under `ENV_LOCK` — this removes the two variables set above before the lock is
        // released.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("PRIKK_ACTIVE_PATCH_WARN");
            std::env::remove_var("PRIKK_ACTIVE_PATCH_LIMIT");
        }
        Fixture::clear_env();

        let err = result.expect_err("a commit over the configured limit must be refused");
        assert_eq!(
            err.class(),
            "refusal",
            "0.{}: a full queue is not a lock — nothing is held and no other writer exists. \
             Classifying it `lock-conflict` renders FR-106's \"another writer is active\" over it, \
             which is RFC 017 F4's live defect. Got: {err:?}",
            bin.minor
        );
        let message = err.to_string();
        assert!(
            message.contains("queued patches") && message.contains("configured limit"),
            "0.{}: unexpected message for the full-queue precondition: {message}",
            bin.minor
        );
        // The class word itself differs across the range (`lock conflict:` ≤ 0.34,
        // `precondition not met:` ≥ 0.35) and neither is matched on — that is the property under test.
        assert!(
            message.contains("lock conflict:") || message.contains("precondition not met:"),
            "0.{}: prikk stopped prefixing this message at all — re-read the classifier's clause \
             match before assuming that is harmless: {message}",
            bin.minor
        );
    }
}

/// A **genuinely held lock**, provoked through `CliBackend::commit` at both ends — the one
/// `is_lock_conflict` arm that names a real lock *and* has a reachable path.
///
/// This is the counterpart to the test above: together they prove the narrowing RFC 017 F4 performed
/// still separates the two cases against real binaries, rather than only against strings. A regression
/// that widened the arm back would show up here as the full-queue test failing; one that narrowed it too
/// far would show up as this one failing.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn a_genuinely_held_lock_classifies_lock_conflict_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        // Pre-place the lock file at the path prikk takes, the way RFC 017 captured this message.
        let lock_dir = fixture.repo().join(".prikk/active/default");
        std::fs::create_dir_all(&lock_dir)
            .unwrap_or_else(|e| panic!("0.{}: mkdir lock dir: {e}", bin.minor));
        std::fs::write(lock_dir.join("active.lock"), b"")
            .unwrap_or_else(|e| panic!("0.{}: place lock: {e}", bin.minor));

        fixture.set_author_env();
        let result = backend.commit(fixture.repo(), "heads/main", "blocked by a real lock");
        Fixture::clear_env();

        let err = result.expect_err("a commit against a held lock must be refused");
        assert_eq!(
            err.class(),
            "lock-conflict",
            "0.{}: an actually-held lock must stay a lock conflict — this is the case FR-106's \
             retry guidance is *for*. Got: {err:?}",
            bin.minor
        );
        assert!(
            err.to_string().contains("lock already exists"),
            "0.{}: unexpected message for a held lock: {err}",
            bin.minor
        );
    }
}

/// RFC 023 Handoff B — [`stikk_prikk::key_id`] against a **real** process environment.
///
/// **The handoff expected the suite to cover this already, and it did not.** The suite drives
/// `CliBackend`'s seam methods; it never builds a `ConfirmationSummary`, so nothing in it reached the
/// key-id module. What is true is the useful half: `Fixture` sets `PRIKK_*_KEY_ID` **for real** in this
/// process, so the module's real-lookup entry points can be exercised here and nowhere else.
///
/// That gap matters. `key_id.rs`'s unit tests drive `key_id_with`, the injected form — hermetic, and
/// deliberately so. The public `author_key_id()` / `maintainer_key_id()` wrappers, the two functions
/// that actually touch `std::env::var_os`, had **no test at all**, exactly as `env.rs`'s own tests
/// acknowledge for `read_readiness`. This closes that for the id half.
///
/// It needs no prikk binary of its own, but it runs at both ends anyway: `Fixture::build` is what sets
/// the environment, and building one is version-dependent (`prikk key generate` at ≥ 0.33, the manual
/// path below it), so "the ids a real fixture configures" is a per-version fact, not a constant.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn the_key_id_module_reads_the_ids_a_real_fixture_configured() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);

        // Nothing set: an absence, not a placeholder and not a stale value from another test.
        assert_eq!(
            stikk_prikk::key_id::author_key_id(),
            None,
            "0.{}: an unset AUTHOR id must read as absent",
            bin.minor
        );
        assert_eq!(stikk_prikk::key_id::maintainer_key_id(), None);

        fixture.set_author_env();
        assert_eq!(
            stikk_prikk::key_id::author_key_id().as_deref(),
            Some(fixture.author_key_id()),
            "0.{}: the module must report the id the fixture actually configured",
            bin.minor
        );
        // And it does not confuse the roles against a real environment, where both variables exist in
        // the same process and differ only by name.
        assert_eq!(
            stikk_prikk::key_id::maintainer_key_id(),
            None,
            "0.{}: AUTHOR readiness must not make a MAINTAINER id appear",
            bin.minor
        );

        fixture.set_maintainer_env();
        assert_eq!(
            stikk_prikk::key_id::maintainer_key_id().as_deref(),
            Some(fixture.maintainer_key_id()),
            "0.{}: the MAINTAINER id must read back as configured",
            bin.minor
        );

        Fixture::clear_env();
        assert_eq!(
            stikk_prikk::key_id::author_key_id(),
            None,
            "0.{}: clearing the environment must return both ids to absent",
            bin.minor
        );
        assert_eq!(stikk_prikk::key_id::maintainer_key_id(), None);
    }
}

/// RFC 026 §6 — the three JSON reports, read from a **real** binary at both ends.
///
/// `log`, `branch` and `tag` were the entire remainder of stikk's prose parsing, and prikk 0.39 gave
/// each a `--format json`. stikk now asks for JSON at ≥ 0.39 and prose below, so **this one test
/// exercises two different parsers** depending on which binary it is handed — which is the only way to
/// know both paths still work, since the floor is 0.28 and the prose readers are the only thing there.
///
/// **Seeded entirely through raw prikk, not through `CliBackend`.** Every other test here drives
/// stikk's own `commit`/`seal`, and at ≥ 0.40 those are refused by stikk's client-side readiness gate
/// (RFC 026 F1 — the model reads `PRIKK_*_SEED`, which prikk no longer uses). That defect is Handoff
/// B's to fix and is deliberately untouched here; seeding with raw prikk keeps this test measuring the
/// parsers rather than waiting on it. The reads themselves go through `Prikk`, which is the point.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn the_three_reports_parse_at_both_ends_json_above_prose_below() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);

        // Raw prikk, inheriting the fixture's own era-correct configuration.
        fixture.set_author_env();
        fixture.set_maintainer_env();
        let run = |args: &[&str]| {
            let out = std::process::Command::new(&bin.path)
                .args(args)
                .current_dir(fixture.repo())
                .output()
                .unwrap_or_else(|e| panic!("0.{}: spawn prikk {args:?}: {e}", bin.minor));
            assert!(
                out.status.success(),
                "0.{}: prikk {args:?} failed: {}",
                bin.minor,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        };
        run(&[
            "commit",
            "--from-worktree",
            "--ref",
            "heads/main",
            "-m",
            "seeded for the report readers",
        ]);
        run(&["seal", "--ref", "heads/main", "--allow-no-audit"]);
        run(&["branch", "create", "heads/feature", "--from", "heads/main"]);
        run(&[
            "tag",
            "create",
            "tags/v1",
            "--target",
            "heads/main",
            "-m",
            "one",
        ]);
        Fixture::clear_env();

        // `log` — the message is the field whose shape moved most across re-baselines.
        let history = backend
            .history(fixture.repo(), "heads/main", 10)
            .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor));
        assert_eq!(history.reff, "heads/main", "0.{}", bin.minor);
        assert_eq!(history.blocks.len(), 1, "0.{}: {history:?}", bin.minor);
        let block = &history.blocks[0];
        assert_eq!(block.patches, 1, "0.{}: {block:?}", bin.minor);
        assert!(!block.block_id.is_empty(), "0.{}", bin.minor);
        // Messages persist from prikk **0.32** (`UD-01` retired, RFC 015 F2); below it prikk validates
        // and discards them, so an empty list there is correct rather than a parse failure — and
        // `patch_count` stays the authoritative total either way (RFC 015 F4).
        if bin.minor >= 32 {
            assert!(
                block
                    .messages
                    .iter()
                    .any(|m| m.message == "seeded for the report readers"),
                "0.{}: the commit message must survive the report: {block:?}",
                bin.minor
            );
        } else {
            assert!(
                block.messages.is_empty(),
                "0.{}: this prikk does not persist commit messages, so none should be reported: \
                 {block:?}",
                bin.minor
            );
        }

        // `branch` — and the tag-leak difference between the two eras, asserted rather than assumed.
        let refs = backend
            .refs(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor));
        for wanted in ["heads/main", "heads/feature"] {
            assert!(
                refs.iter().any(|r| r.name == wanted),
                "0.{}: {wanted} missing from {refs:?}",
                bin.minor
            );
        }
        let leaks_tags = refs.iter().any(|r| r.name.starts_with("tags/"));
        if bin.minor >= 39 {
            assert!(
                !leaks_tags,
                "0.{}: prikk stopped listing tag refs in `branch list --all` at 0.39; it is listing \
                 them again: {refs:?}",
                bin.minor
            );
        } else {
            assert!(
                leaks_tags,
                "0.{}: this era's `branch list --all` does list tag refs, which is why \
                 `stikk_core::history::list_refs` de-duplicates by name: {refs:?}",
                bin.minor
            );
        }

        // `tag` — a tag points at a block, and that id must be the block `log` just reported.
        let tags = backend
            .tags(fixture.repo())
            .unwrap_or_else(|e| panic!("0.{}: tags: {e}", bin.minor));
        let tag = tags
            .iter()
            .find(|t| t.name == "tags/v1")
            .unwrap_or_else(|| panic!("0.{}: tags/v1 missing from {tags:?}", bin.minor));
        assert_eq!(
            tag.id, block.block_id,
            "0.{}: the tag must point at the block `log` reported",
            bin.minor
        );
        assert!(
            tags.iter().all(|t| t.name.starts_with("tags/")),
            "0.{}",
            bin.minor
        );
    }
}

/// Read one role's `binding` string out of prikk's own raw `key status --format json`, so the test can
/// assert what **prikk** says rather than a constant stikk agrees with itself about.
///
/// A deliberately small reader: this crate has no JSON dependency, and the product's reader is the
/// thing under test, so it must not also be the witness.
fn raw_binding(json: &str, role: &str) -> Option<String> {
    let start = json.find(&format!("\"role\": \"{role}\""))?;
    let tail = &json[start..];
    let end = tail.find('}').unwrap_or(tail.len());
    let object = &tail[..end];
    let at = object.find("\"binding\": ")? + "\"binding\": ".len();
    let value = object[at..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    let value = value.strip_prefix('"')?;
    Some(value[..value.find('"')?].to_string())
}

/// **`Prikk::readiness` against a real binary at both ends** — the method 0.6.0 is about, and until
/// this test the only one of eleven the suite never asserted an answer from (it ran only as the gate
/// inside `commit`/`seal`).
///
/// Every expectation below comes from prikk's letter 007 measurement and RFC 026 F4, not from a run of
/// this test. **If the binary answers differently, that is a finding to report, not an assertion to
/// adjust.**
///
/// **No assertion depends on a real key directory.** prikk 0.41 resolves one from `$XDG_CONFIG_HOME`,
/// `$HOME/.config` or `%APPDATA%`, none of which `clear_env` touches, so an "unconfigured" assertion at
/// the ceiling would pass on a CI runner and pass or fail by laptop elsewhere. The ceiling's `NotReady`
/// case instead points `PRIKK_AUTHOR_SEED_FILE` at a path that does not exist: prikk's own rule is that
/// a set override always wins even when its file is missing, so no key directory is consulted.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn readiness_reports_each_band_as_prikk_does_at_both_ends() {
    use stikk_model::{Binding, RoleReadiness};

    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();

    // --- floor: the ≤ 0.39 band, environment presence ---
    let floor = PrikkBin::floor();
    let fixture = Fixture::build(&floor);
    let backend = CliBackend::with_program(&floor.path);
    fixture.set_author_env();
    fixture.set_maintainer_env();
    let report = backend
        .readiness(fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", floor.minor));
    assert_eq!(
        report.readiness.author,
        RoleReadiness::Unknown,
        "0.{}",
        floor.minor
    );
    assert_eq!(
        report.readiness.maintainer,
        RoleReadiness::Unknown,
        "0.{}",
        floor.minor
    );
    // Deterministic on this band: it has no key directory to fall back to.
    Fixture::clear_env();
    let report = backend
        .readiness(fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", floor.minor));
    assert_eq!(
        report.readiness.author,
        RoleReadiness::NotReady,
        "0.{}",
        floor.minor
    );
    assert_eq!(
        report.readiness.maintainer,
        RoleReadiness::NotReady,
        "0.{}",
        floor.minor
    );
    drop(fixture);

    // --- ceiling: the ≥ 0.41 band, prikk's own `key status` ---
    let ceiling = PrikkBin::ceiling();
    let fixture = Fixture::build(&ceiling);
    let backend = CliBackend::with_program(&ceiling.path);
    fixture.set_author_env();
    fixture.set_maintainer_env();

    let raw_status = || {
        let out = std::process::Command::new(&ceiling.path)
            .args(["key", "status", "--format", "json"])
            .current_dir(fixture.repo())
            .output()
            .unwrap_or_else(|e| panic!("0.{}: spawn key status: {e}", ceiling.minor));
        assert!(
            out.status.success(),
            "0.{}: raw key status failed",
            ceiling.minor
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let report = backend
        .readiness(fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", ceiling.minor));
    let raw = raw_status();
    println!("0.{} key status, fresh fixture:\n{raw}", ceiling.minor);
    assert_eq!(
        report.readiness.author,
        RoleReadiness::Known(Binding::Unrecorded),
        "0.{}: a fresh repository has recorded no author signature",
        ceiling.minor
    );
    assert_eq!(
        report.readiness.maintainer,
        RoleReadiness::Known(Binding::Matches),
        "0.{}: the fixture adopted this maintainer key",
        ceiling.minor
    );
    assert_eq!(
        report.author.key_id.as_deref(),
        Some(fixture.author_key_id())
    );
    assert_eq!(report.author.key_id_source.as_deref(), Some("environment"));
    // What prikk itself says, beside what stikk read.
    assert_eq!(
        raw_binding(&raw, "author").as_deref(),
        Some("unrecorded"),
        "{raw}"
    );
    assert_eq!(
        raw_binding(&raw, "maintainer").as_deref(),
        Some("matches"),
        "{raw}"
    );

    // One raw commit as the author binds the id.
    let out = std::process::Command::new(&ceiling.path)
        .args([
            "commit",
            "--from-worktree",
            "--ref",
            "heads/main",
            "-m",
            "bind the author id",
        ])
        .current_dir(fixture.repo())
        .output()
        .unwrap_or_else(|e| panic!("0.{}: spawn commit: {e}", ceiling.minor));
    assert!(
        out.status.success(),
        "0.{}: raw commit failed: {}",
        ceiling.minor,
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let report = backend
        .readiness(fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", ceiling.minor));
    let raw = raw_status();
    println!("0.{} key status, after one commit:\n{raw}", ceiling.minor);
    assert_eq!(
        report.readiness.author,
        RoleReadiness::Known(Binding::Matches),
        "0.{}: a signature now records this id",
        ceiling.minor
    );
    assert_eq!(
        raw_binding(&raw, "author").as_deref(),
        Some("matches"),
        "{raw}"
    );

    // `NotReady` at the ceiling, deterministically: a set override that points at nothing.
    let absent = fixture.repo().with_file_name("absent-author.seed");
    // SAFETY: same discipline as `Fixture::set_*_env` — the caller holds `ENV_LOCK`, and
    // `Fixture::clear_env` below removes this variable (it clears all six) before the lock is released.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("PRIKK_AUTHOR_SEED_FILE", &absent);
    }
    let report = backend.readiness(fixture.repo());
    let raw = raw_status();
    Fixture::clear_env();
    let report = report.unwrap_or_else(|e| panic!("0.{}: readiness: {e}", ceiling.minor));
    println!("0.{} key status, missing override:\n{raw}", ceiling.minor);
    assert_eq!(
        report.readiness.author,
        RoleReadiness::NotReady,
        "0.{}",
        ceiling.minor
    );
    assert_eq!(
        report.author.reason.as_deref(),
        Some("override-missing"),
        "0.{}: prikk's reason, verbatim",
        ceiling.minor
    );
    assert!(!absent.exists(), "nothing was written at the override path");
}

// ---------------------------------------------------------------------------------------------
// RFC 030 decision 5 — a confirmed commit authors the worktree its preview showed.
//
// **Written first and run red on the unchanged code** (RFC 030 handoff §6): a safeguard whose test never
// failed has not been shown to guard. Both tests drive the real confirm path — `commit_preview`, then
// `commit_confirm_and_execute` — which the lifecycle test above deliberately bypasses.
// ---------------------------------------------------------------------------------------------

/// Run a raw prikk command in `repo` and require it to succeed, naming prikk's stderr if it does not.
fn prikk_ok(bin: &PrikkBin, repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    let out = std::process::Command::new(&bin.path)
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| panic!("0.{}: spawn prikk {args:?}: {e}", bin.minor));
    assert!(
        out.status.success(),
        "0.{}: prikk {args:?} exited {:?}: {}",
        bin.minor,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    out
}

/// The paths a `ChangesView`-shaped preview listed, sorted — what the user saw.
fn previewed_paths(preview: &stikk_core::CommitPreview) -> Vec<String> {
    let mut paths: Vec<String> = preview
        .changes
        .entries
        .iter()
        .map(|e| e.path.clone())
        .collect();
    paths.sort();
    paths
}

/// **RFC 030 at prikk 0.42: a branch switch between preview and confirmation.**
///
/// The race, measured by the architect at 0.42 and reproduced here against the real confirm path:
/// preview a commit on `heads/main` whose only change is one untracked file, run a raw
/// `prikk branch switch heads/dev` — which prikk allows, carrying the untracked file across — then
/// confirm. The worktree now holds `heads/dev`'s files against `heads/main`'s baseline. **The confirmation
/// must be `Stale`, and nothing may be queued.**
///
/// The preview holds only an untracked file on purpose: prikk refuses to switch over a modified tracked
/// file (RFC 030 handoff §2, M5), and a refused switch tests nothing.
///
/// **Below 0.42 the skip is announced**: prikk has no `branch switch` there.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc030_a_branch_switch_between_preview_and_confirmation_is_stale_and_commits_nothing() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if bin.minor < 42 {
            eprintln!(
                "RFC 030: branch-switch race SKIPPED at 0.{} — `prikk branch switch` does not exist below \
                 0.42, so the worktree cannot be replaced this way there. Announced rather than silent \
                 (RFC 022 §3).",
                bin.minor
            );
            continue;
        }
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // heads/main: readme.txt and shared.txt, sealed.
        std::fs::write(repo.join("shared.txt"), "main\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "main baseline")
            .unwrap_or_else(|e| panic!("0.{}: commit main: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal main: {e}", bin.minor));

        // heads/dev: a file main lacks, and a shared.txt that differs. Sealed. Then back to main, clean.
        prikk_ok(
            &bin,
            &repo,
            &["branch", "create", "heads/dev", "--from", "heads/main"],
        );
        prikk_ok(&bin, &repo, &["branch", "switch", "heads/dev"]);
        std::fs::write(repo.join("dev-only.txt"), "dev only\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        std::fs::write(repo.join("shared.txt"), "dev\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/dev", "dev work")
            .unwrap_or_else(|e| panic!("0.{}: commit dev: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/dev")
            .unwrap_or_else(|e| panic!("0.{}: seal dev: {e}", bin.minor));
        Fixture::clear_env();
        prikk_ok(&bin, &repo, &["branch", "switch", "heads/main"]);
        let clean = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status: {e}", bin.minor));
        assert!(
            clean.clean,
            "0.{}: back on main, clean: {clean:?}",
            bin.minor
        );

        // One untracked file, previewed: the only change the user sees.
        std::fs::write(repo.join("note.txt"), "a note\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        let outcome = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: commit_preview: {e}", bin.minor));
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness;
        Fixture::clear_env();
        let stikk_core::CommitPreviewOutcome::Ready { preview, token } = outcome else {
            panic!("0.{}: expected a Ready preview, got {outcome:?}", bin.minor);
        };
        assert_eq!(
            previewed_paths(&preview),
            ["note.txt"],
            "0.{}: the preview lists only the untracked file",
            bin.minor
        );

        // The terminal switch. prikk carries the untracked file across and replaces the rest.
        prikk_ok(&bin, &repo, &["branch", "switch", "heads/dev"]);

        // **Without this, the test could pass on an unchanged tree and prove nothing.**
        let now = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status after switch: {e}", bin.minor));
        let mut now_paths: Vec<String> = now.entries.iter().map(|e| e.path.clone()).collect();
        now_paths.sort();
        assert_ne!(
            now_paths,
            previewed_paths(&preview),
            "0.{}: the switch must have changed what heads/main's worktree-status lists",
            bin.minor
        );
        println!(
            "0.{}: previewed {:?}; after `branch switch heads/dev`, heads/main lists {now_paths:?}",
            bin.minor,
            previewed_paths(&preview)
        );

        fixture.set_author_env();
        let result = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "confirmed after a branch switch",
        );
        Fixture::clear_env();
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .queued_patches;
        println!(
            "0.{}: confirmation returned {result:?}; prikk's status reports {queued} queued patch(es)",
            bin.minor
        );
        // The token sees the switch first (prikk's current branch moved), so the cause is Repository.
        assert!(
            matches!(
                result,
                Err(StikkError::Stale {
                    cause: stikk_model::StaleCause::Repository,
                    ..
                })
            ),
            "0.{}: a commit confirmed after a branch switch must be Stale {{ Repository }}; got {result:?}",
            bin.minor
        );
        assert_eq!(
            queued, 0,
            "0.{}: nothing may be queued, by prikk's own status",
            bin.minor
        );
    }
}

/// **RFC 030 at both ends: a file added between preview and confirmation.**
///
/// Preview a commit whose only change is one untracked file, add a second file, confirm: the result must
/// be `Stale`, with nothing queued. **Then the positive control**: preview again and confirm at once, and
/// the commit is recorded with both files. A safeguard that blocked every commit would pass the first
/// half; the second half is what rules that out.
///
/// Versions are reported one by one rather than stopping at the first: a version whose confirmation is
/// not `Stale` is recorded, and the test fails at the end naming every such version and what happened.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc030_a_file_added_between_preview_and_confirmation_is_stale_then_a_fresh_preview_commits() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let mut not_stale = Vec::new();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // The fixture's own readme.txt is the one untracked change.
        fixture.set_author_env();
        let outcome = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: commit_preview: {e}", bin.minor));
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness;
        Fixture::clear_env();
        let stikk_core::CommitPreviewOutcome::Ready { preview, token } = outcome else {
            panic!("0.{}: expected a Ready preview, got {outcome:?}", bin.minor);
        };
        assert_eq!(previewed_paths(&preview), ["readme.txt"], "0.{}", bin.minor);

        std::fs::write(repo.join("second.txt"), "added after the preview\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));

        fixture.set_author_env();
        let result = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "confirmed after a file was added",
        );
        Fixture::clear_env();
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .queued_patches;
        println!(
            "0.{}: confirmation returned {result:?}; prikk's status reports {queued} queued patch(es)",
            bin.minor
        );
        if !matches!(
            result,
            Err(StikkError::Stale {
                cause: stikk_model::StaleCause::Worktree,
                ..
            })
        ) || queued != 0
        {
            not_stale.push(format!(
                "0.{}: expected Stale {{ Worktree }} with nothing queued; got {result:?} with {queued} queued",
                bin.minor
            ));
            continue;
        }

        // The positive control: a fresh preview, confirmed at once, commits — both files.
        fixture.set_author_env();
        let fresh = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: fresh commit_preview: {e}", bin.minor));
        Fixture::clear_env();
        let stikk_core::CommitPreviewOutcome::Ready { preview, token } = fresh else {
            panic!(
                "0.{}: expected a fresh Ready preview, got {fresh:?}",
                bin.minor
            );
        };
        assert_eq!(
            previewed_paths(&preview),
            ["readme.txt", "second.txt"],
            "0.{}",
            bin.minor
        );
        fixture.set_author_env();
        let committed = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "confirmed at once",
        )
        .unwrap_or_else(|e| panic!("0.{}: a fresh confirmation must commit: {e}", bin.minor));
        Fixture::clear_env();
        let mut recorded: Vec<String> = committed
            .result
            .changes
            .iter()
            .map(|c| c.path.clone())
            .collect();
        recorded.sort();
        assert_eq!(
            recorded,
            ["readme.txt", "second.txt"],
            "0.{}: prikk recorded both files",
            bin.minor
        );
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .queued_patches;
        assert_eq!(queued, 1, "0.{}: prikk's status holds the patch", bin.minor);
    }
    assert!(
        not_stale.is_empty(),
        "RFC 030: a confirmation went through after the worktree changed:\n{}",
        not_stale.join("\n")
    );
}

/// prikk's own `declarations` array from `worktree-status --format json` (prikk ≥ 0.38), as text —
/// read raw so this suite can show what the typed report must compare, independent of whether stikk's
/// reader carries it.
fn raw_declarations(bin: &PrikkBin, repo: &std::path::Path, reff: &str) -> String {
    let out = std::process::Command::new(&bin.path)
        .args(["worktree-status", "--ref", reff, "--format", "json"])
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| panic!("0.{}: spawn worktree-status: {e}", bin.minor));
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text
        .find("\"declarations\"")
        .unwrap_or_else(|| panic!("0.{}: no declarations array in: {text}", bin.minor));
    text[start..]
        .trim_end()
        .trim_end_matches('}')
        .trim()
        .to_string()
}

/// **RFC 030 amendment A2 at prikk ≥ 0.38: a rename declared between preview and confirmation.**
///
/// M7, measured by the architect at 0.42: seal `a.txt`, move it to `b.txt` in the shell, preview — prikk
/// lists `missing a.txt` and `untracked b.txt`, with no declarations. Then `prikk mv a.txt b.txt` records
/// the declaration **without changing the listed paths**, and `commit` authors `rename-path a.txt -> b.txt`,
/// an operation the preview never held. **The confirmation must be `Stale`, and nothing may be queued.**
///
/// The positive control previews again and confirms at once: the commit records the rename.
///
/// **Below 0.38 the skip is announced**: `prikk mv` does not exist there, so nothing can be declared.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc030_a_rename_declared_between_preview_and_confirmation_is_stale_then_a_fresh_preview_commits()
{
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let mut not_stale = Vec::new();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if bin.minor < 38 {
            eprintln!(
                "RFC 030: rename-declaration race SKIPPED at 0.{} — `prikk mv` does not exist below 0.38, \
                 so no rename can be declared there. Announced rather than silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // a.txt sealed on heads/main (beside the fixture's readme.txt).
        std::fs::write(repo.join("a.txt"), "a body\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "baseline")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        // The shell moves it. The preview sees a missing and an untracked file, and no declaration.
        std::fs::rename(repo.join("a.txt"), repo.join("b.txt"))
            .unwrap_or_else(|e| panic!("0.{}: shell mv: {e}", bin.minor));
        fixture.set_author_env();
        let outcome = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: commit_preview: {e}", bin.minor));
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness;
        Fixture::clear_env();
        let stikk_core::CommitPreviewOutcome::Ready { preview, token } = outcome else {
            panic!("0.{}: expected a Ready preview, got {outcome:?}", bin.minor);
        };
        assert_eq!(
            previewed_paths(&preview),
            ["a.txt", "b.txt"],
            "0.{}",
            bin.minor
        );
        let entries_of = |status: &stikk_prikk::WorktreeStatus| -> Vec<(String, String)> {
            let mut e: Vec<(String, String)> = status
                .entries
                .iter()
                .map(|x| (x.kind.clone(), x.path.clone()))
                .collect();
            e.sort();
            e
        };
        let before = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status before: {e}", bin.minor));
        let declarations_before = raw_declarations(&bin, &repo, "heads/main");

        // prikk records the rename. The file is already moved, so no bytes change.
        let mv = prikk_ok(&bin, &repo, &["mv", "a.txt", "b.txt"]);
        println!(
            "0.{}: prikk mv printed {:?}",
            bin.minor,
            String::from_utf8_lossy(&mv.stdout).trim()
        );

        let after = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: worktree_status after: {e}", bin.minor));
        let declarations_after = raw_declarations(&bin, &repo, "heads/main");
        // **The reason declarations must be compared:** the listed entries do not move.
        assert_eq!(
            entries_of(&after),
            entries_of(&before),
            "0.{}: prikk mv over an already-moved file leaves the listed entries unchanged",
            bin.minor
        );
        assert_ne!(
            declarations_after, declarations_before,
            "0.{}: prikk's declarations must differ after prikk mv",
            bin.minor
        );
        // And stikk's reader carries them (RFC 030 amendment A1), so the confirmation can compare them.
        assert!(before.declarations.is_empty(), "0.{}", bin.minor);
        assert_eq!(
            after.declarations,
            vec![stikk_prikk::RenameDeclaration {
                old_path: "a.txt".to_string(),
                new_path: "b.txt".to_string(),
            }],
            "0.{}",
            bin.minor
        );
        println!(
            "0.{}: entries unchanged {:?}; declarations before {declarations_before}, after {declarations_after}",
            bin.minor,
            entries_of(&after)
        );

        fixture.set_author_env();
        let result = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "confirmed after a rename was declared",
        );
        Fixture::clear_env();
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .queued_patches;
        println!(
            "0.{}: confirmation returned {result:?}; prikk's status reports {queued} queued patch(es)",
            bin.minor
        );
        if !matches!(
            result,
            Err(StikkError::Stale {
                cause: stikk_model::StaleCause::Worktree,
                ..
            })
        ) || queued != 0
        {
            not_stale.push(format!(
                "0.{}: expected Stale {{ Worktree }} with nothing queued; got {result:?} with {queued} queued",
                bin.minor
            ));
            continue;
        }

        // The positive control: preview again, confirm at once, and the rename is recorded.
        fixture.set_author_env();
        let fresh = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: fresh commit_preview: {e}", bin.minor));
        Fixture::clear_env();
        let stikk_core::CommitPreviewOutcome::Ready { token, .. } = fresh else {
            panic!(
                "0.{}: expected a fresh Ready preview, got {fresh:?}",
                bin.minor
            );
        };
        fixture.set_author_env();
        let committed = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "rename confirmed at once",
        )
        .unwrap_or_else(|e| panic!("0.{}: a fresh confirmation must commit: {e}", bin.minor));
        Fixture::clear_env();
        assert!(
            committed
                .result
                .changes
                .iter()
                .any(|c| c.operation == "rename-path"),
            "0.{}: prikk records the declared rename: {:?}",
            bin.minor,
            committed.result.changes
        );
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .queued_patches;
        assert_eq!(queued, 1, "0.{}: prikk's status holds the patch", bin.minor);
    }
    assert!(
        not_stale.is_empty(),
        "RFC 030: a confirmation went through after a rename was declared:\n{}",
        not_stale.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 029 Handoff B: stikk follows prikk's current branch, and names it on both confirmations.
// ---------------------------------------------------------------------------------------------

/// **RFC 029 Handoff B at both ends: prikk's current branch, as the confirmations see it.**
///
/// **At 0.42.** Seal `heads/main`, create `heads/dev` from it, and run a raw `prikk branch switch
/// heads/dev`. `orient()` reports the branch. A commit previewed for `heads/main` is `Ready` and carries
/// §5's first-row notice byte for byte; confirming it is legitimate and succeeds. A seal previewed for
/// `heads/main` then carries the same notice. **And the notice's own claim is re-measured**: after that
/// seal, a raw `prikk commit` with no `--ref` queues for `heads/dev` (`C-T2b`).
///
/// **At 0.28.** prikk reports no current branch, and neither preview carries a notice.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc029b_prikks_current_branch_is_reported_and_named_on_both_confirmations() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let at_42 = bin.minor >= 42;

        // heads/main: readme.txt, sealed.
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "main baseline")
            .unwrap_or_else(|e| panic!("0.{}: commit main: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal main: {e}", bin.minor));
        if at_42 {
            // `branch create` signs as MAINTAINER, so the environment stays set through it.
            prikk_ok(
                &bin,
                &repo,
                &["branch", "create", "heads/dev", "--from", "heads/main"],
            );
            prikk_ok(&bin, &repo, &["branch", "switch", "heads/dev"]);
        }
        Fixture::clear_env();

        let view = stikk_core::orient(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: orient: {e}", bin.minor));
        let expected_branch = if at_42 {
            stikk_model::CurrentBranch::Branch(
                stikk_model::RefName::parse("heads/dev").expect("a valid ref name"),
            )
        } else {
            stikk_model::CurrentBranch::NotReported
        };
        assert_eq!(
            view.current_branch, expected_branch,
            "0.{}: orient()'s current branch",
            bin.minor
        );
        let expected_notice = at_42.then_some(
            "This targets heads/main. prikk's current branch is heads/dev, the ref prikk uses when \
             no --ref is given.",
        );

        // Commit, previewed for heads/main.
        std::fs::write(repo.join("note.txt"), "a note\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        let outcome = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: commit_preview: {e}", bin.minor));
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness;
        let stikk_core::CommitPreviewOutcome::Ready { token, .. } = outcome else {
            panic!(
                "0.{}: expected a Ready commit preview, got {outcome:?}",
                bin.minor
            );
        };
        println!(
            "0.{}: commit preview for heads/main, notice: {:?}",
            bin.minor,
            token.summary().branch_notice
        );
        assert_eq!(
            token.summary().branch_notice.as_deref(),
            expected_notice,
            "0.{}: commit's notice",
            bin.minor
        );
        // Legitimate: committing to a ref other than prikk's current branch is allowed.
        stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "committed to heads/main while prikk's current branch is elsewhere",
        )
        .unwrap_or_else(|e| panic!("0.{}: the confirmed commit: {e}", bin.minor));
        Fixture::clear_env();

        // Seal, previewed for heads/main.
        fixture.set_maintainer_env();
        let outcome = stikk_core::seal_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal_preview: {e}", bin.minor));
        let stikk_core::SealPreviewOutcome::Ready { token } = outcome else {
            panic!("0.{}: expected a Ready seal preview", bin.minor);
        };
        println!(
            "0.{}: seal preview for heads/main, notice: {:?}",
            bin.minor,
            token.summary().branch_notice
        );
        assert_eq!(
            token.summary().branch_notice.as_deref(),
            expected_notice,
            "0.{}: seal's notice",
            bin.minor
        );
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        if !at_42 {
            eprintln!(
                "RFC 029 B: the `--ref` default re-measurement SKIPPED at 0.{} — there is no current \
                 branch below 0.42, so a raw commit without `--ref` has nothing to default to. \
                 Announced rather than silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }

        // The notice says prikk uses its current branch when no --ref is given. Measure it.
        std::fs::write(repo.join("second.txt"), "second\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        let raw = prikk_ok(&bin, &repo, &["commit", "-m", "no --ref given"]);
        Fixture::clear_env();
        let queued = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor));
        println!(
            "0.{}: raw `prikk commit -m` with no --ref printed {:?}; status reports {} queued for {:?}",
            bin.minor,
            String::from_utf8_lossy(&raw.stdout)
                .lines()
                .find(|line| line.starts_with("baseline ref:")),
            queued.queued_patches,
            queued.queued_target
        );
        assert_eq!(
            queued.queued_target.as_deref(),
            Some("heads/dev"),
            "0.{}: a raw commit with no --ref must queue for prikk's current branch",
            bin.minor
        );
    }
}

/// **RFC 029 Handoff B review v1 §2.3 at both ends: an unpublished `heads/main` reads as an empty history.**
///
/// Picking the empty picker's `heads/main (not published)` opens History, so both readers must take a freshly
/// initialised repository's history rather than refuse it: prose `history: <empty>` below 0.39, and JSON with
/// an empty `blocks` array from 0.39.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc029b_an_unpublished_heads_main_reads_as_an_empty_history_at_both_ends() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let history = backend
            .history(fixture.repo(), "heads/main", 20)
            .unwrap_or_else(|e| {
                panic!(
                    "0.{}: history of an unpublished heads/main must not refuse: {e}",
                    bin.minor
                )
            });
        println!(
            "0.{}: history of an unpublished heads/main: ref {:?}, {} block(s)",
            bin.minor,
            history.reff,
            history.blocks.len()
        );
        assert!(
            history.blocks.is_empty(),
            "0.{}: an unpublished ref has no blocks: {history:?}",
            bin.minor
        );
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 028 Handoff A §7: the queue, read from real prikk at both ends.
// ---------------------------------------------------------------------------------------------

/// **RFC 028 at both ends: a queued patch is the patch that seals.**
///
/// Two patches are queued on a sealed `heads/main`: one ordinary (`add b`), and — at prikk ≥ 0.38 — one
/// `prikk mv` rename, each committed with a message.
///
/// - **At ≥ 0.39** the queue is listed. Its count equals Orientation's; the rename carries both paths and
///   the fixture's own author key id; at ≥ 0.42 each message equals what was committed, and below 0.42 it
///   is not reported. **Then the queue is sealed, and the queued ids must equal the ids `log` reports for
///   that block** — asserted, not assumed.
/// - **Below 0.39** the list is unreported, and its count and target equal Orientation's.
/// - **Below 0.38** there is no `prikk mv`, and the rename is an announced skip.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc028_queued_patch_ids_are_the_ids_that_seal_and_the_rename_carries_both_paths() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // heads/main: readme.txt, sealed.
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "base")
            .unwrap_or_else(|e| panic!("0.{}: commit base: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal base: {e}", bin.minor));

        // Queued: `add b`, then (≥ 0.38) a rename.
        fixture.set_author_env();
        std::fs::write(repo.join("b.txt"), "b\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        backend
            .commit(&repo, "heads/main", "add b")
            .unwrap_or_else(|e| panic!("0.{}: commit add b: {e}", bin.minor));
        let renamed = bin.minor >= 38;
        if renamed {
            prikk_ok(&bin, &repo, &["mv", "readme.txt", "renamed.txt"]);
            backend
                .commit(&repo, "heads/main", "rename readme")
                .unwrap_or_else(|e| panic!("0.{}: commit rename: {e}", bin.minor));
        } else {
            eprintln!(
                "RFC 028: the rename case SKIPPED at 0.{} — `prikk mv` does not exist below 0.38. \
                 Announced rather than silent (RFC 022 §3).",
                bin.minor
            );
        }
        let expected_count: u64 = if renamed { 2 } else { 1 };

        let orientation = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor));
        let report = backend
            .queue(&repo)
            .unwrap_or_else(|e| panic!("0.{}: queue: {e}", bin.minor));
        assert_eq!(
            orientation.queued_patches, expected_count,
            "0.{}",
            bin.minor
        );

        let queue = match report {
            stikk_prikk::QueueReport::Unreported { count, target } => {
                assert!(bin.minor < 39, "0.{}: unreported at ≥ 0.39", bin.minor);
                assert_eq!(count, orientation.queued_patches, "0.{}: count", bin.minor);
                assert_eq!(target, orientation.queued_target, "0.{}: target", bin.minor);
                println!(
                    "0.{}: the queue is unreported: {count} patch(es) for {target:?}, as Orientation says",
                    bin.minor
                );
                Fixture::clear_env();
                continue;
            }
            stikk_prikk::QueueReport::Listed(queue) => queue,
        };
        assert!(bin.minor >= 39, "0.{}: listed below 0.39", bin.minor);
        assert_eq!(
            queue.count, orientation.queued_patches,
            "0.{}: count",
            bin.minor
        );
        assert_eq!(
            queue.target,
            stikk_prikk::QueueTarget::Ref("heads/main".to_string()),
            "0.{}",
            bin.minor
        );

        let expected_messages = ["add b", "rename readme"];
        for (patch, expected) in queue.patches.iter().zip(expected_messages) {
            let want = if bin.minor >= 42 {
                stikk_prikk::QueuedMessage::Text(expected.to_string())
            } else {
                stikk_prikk::QueuedMessage::NotReported
            };
            assert_eq!(patch.message, want, "0.{}: {}", bin.minor, patch.patch_id);
        }
        let rename = &queue.patches[1].operations;
        assert_eq!(
            rename,
            &vec![stikk_prikk::QueuedOperation {
                kind: "rename-path".to_string(),
                paths: vec![
                    stikk_prikk::QueuedPath::Path("readme.txt".to_string()),
                    stikk_prikk::QueuedPath::Path("renamed.txt".to_string()),
                ],
                author_key_id: Some(fixture.author_key_id().to_string()),
            }],
            "0.{}: the rename carries both paths and its key id",
            bin.minor
        );

        // Seal, and compare the ids `log` reports for that block.
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal the queue: {e}", bin.minor));
        Fixture::clear_env();
        let history = backend
            .history(&repo, "heads/main", 5)
            .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor));
        let mut queued_ids: Vec<&str> = queue.patches.iter().map(|p| p.patch_id.as_str()).collect();
        let mut sealed_ids: Vec<&str> = history.blocks[0]
            .messages
            .iter()
            .map(|m| m.patch_id.as_str())
            .collect();
        println!(
            "0.{}: queued ids {queued_ids:?}; after sealing, `log` reports block {} with patch ids {sealed_ids:?} \
             and messages {:?}",
            bin.minor,
            history.blocks[0].block_id,
            history.blocks[0]
                .messages
                .iter()
                .map(|m| m.message.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(history.blocks[0].patches, queue.count, "0.{}", bin.minor);
        queued_ids.sort_unstable();
        sealed_ids.sort_unstable();
        assert_eq!(
            queued_ids, sealed_ids,
            "0.{}: a queued patch is the patch that seals",
            bin.minor
        );
    }
}

/// **RFC 028 F5 at both ends: History for another ref does not claim the queue.**
///
/// One patch is queued for `heads/main`; `heads/other` is published from it. History for `heads/other`
/// must say the active WAL's patch is `heads/main`'s, not put it on `heads/other`'s lineage — and History for
/// `heads/main` still claims it.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc028_history_for_another_ref_does_not_claim_the_queue() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "base")
            .unwrap_or_else(|e| panic!("0.{}: commit base: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal base: {e}", bin.minor));
        // `branch create` signs as MAINTAINER, so the environment stays set through it.
        prikk_ok(
            &bin,
            &repo,
            &["branch", "create", "heads/other", "--from", "heads/main"],
        );
        fixture.set_author_env();
        std::fs::write(repo.join("queued.txt"), "queued\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        backend
            .commit(&repo, "heads/main", "queued for main")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();

        let other = stikk_core::history_view(&backend, &repo, "heads/other", 20)
            .unwrap_or_else(|e| panic!("0.{}: history heads/other: {e}", bin.minor));
        let main = stikk_core::history_view(&backend, &repo, "heads/main", 20)
            .unwrap_or_else(|e| panic!("0.{}: history heads/main: {e}", bin.minor));
        println!(
            "0.{}: heads/other's tier: {:?}; heads/main's tier: {:?}",
            bin.minor,
            other.queued_tier(),
            main.queued_tier()
        );
        assert_eq!(
            other.queued_tier().as_deref(),
            Some(
                "the active WAL holds 1 patch(es) for heads/main — not this ref's history · Q: Queue"
            ),
            "0.{}: another ref's tier",
            bin.minor
        );
        assert_eq!(
            main.queued_tier().as_deref(),
            Some("1 patch(es) in the active WAL — not yet sealed · Q: Queue"),
            "0.{}: this ref's tier",
            bin.minor
        );
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 028 Handoff B §5: the seal confirmation names the patches that freeze.
// ---------------------------------------------------------------------------------------------

/// **RFC 028 Handoff B at both ends: the card named what froze.**
///
/// Two patches are queued on a sealed `heads/main`.
/// - **At ≥ 0.39**, `seal_preview`'s summary names both: each row begins with the queue's own patch id, cut
///   to the short form, and at ≥ 0.42 carries the message as committed. **Then the queue is sealed**, and the
///   block's patch ids begin with those short ids — the card named the patches that froze.
/// - **Below 0.39** the summary says prikk does not list queued patches, and its count equals Orientation's.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc028b_the_seal_confirmation_names_the_patches_that_freeze() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "base")
            .unwrap_or_else(|e| panic!("0.{}: commit base: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal base: {e}", bin.minor));
        fixture.set_author_env();
        for (file, message) in [("b.txt", "add b"), ("c.txt", "add c")] {
            std::fs::write(repo.join(file), format!("{file}\n"))
                .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
            backend
                .commit(&repo, "heads/main", message)
                .unwrap_or_else(|e| panic!("0.{}: commit {message}: {e}", bin.minor));
        }

        fixture.set_maintainer_env();
        let outcome = stikk_core::seal_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal_preview: {e}", bin.minor));
        let stikk_core::SealPreviewOutcome::Ready { token } = outcome else {
            panic!("0.{}: expected a Ready seal preview", bin.minor);
        };
        let summary = token.summary().clone();
        let orientation = backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor));
        assert_eq!(
            summary.counts,
            vec![("patches", orientation.queued_patches)],
            "0.{}: counts",
            bin.minor
        );

        if bin.minor < 39 {
            println!("0.{}: the card says {:?}", bin.minor, summary.freezes);
            assert_eq!(
                summary.freezes,
                Some(stikk_core::FrozenPatches::Unlisted(format!(
                    "prikk 0.{} does not list queued patches.",
                    bin.minor
                ))),
                "0.{}",
                bin.minor
            );
            Fixture::clear_env();
            continue;
        }

        let stikk_prikk::QueueReport::Listed(queue) = backend
            .queue(&repo)
            .unwrap_or_else(|e| panic!("0.{}: queue: {e}", bin.minor))
        else {
            panic!("0.{}: expected a listed queue", bin.minor);
        };
        let Some(stikk_core::FrozenPatches::Listed { rows, .. }) = &summary.freezes else {
            panic!(
                "0.{}: expected listed rows, got {:?}",
                bin.minor, summary.freezes
            );
        };
        assert_eq!(rows.len(), 2, "0.{}: {rows:?}", bin.minor);
        let shorts: Vec<String> = rows
            .iter()
            .map(|row| row.split("  ").next().unwrap_or_default().to_string())
            .collect();
        for ((row, short), (patch, message)) in rows
            .iter()
            .zip(&shorts)
            .zip(queue.patches.iter().zip(["add b", "add c"]))
        {
            assert_eq!(
                short.len(),
                stikk_core::SHORT_ID_CHARS,
                "0.{}: {row:?}",
                bin.minor
            );
            assert!(
                patch.patch_id.starts_with(short.as_str()),
                "0.{}: {short} is a prefix of the queue's {}",
                bin.minor,
                patch.patch_id
            );
            if bin.minor >= 42 {
                assert_eq!(row, &format!("{short}  {message}"), "0.{}", bin.minor);
            }
        }

        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
        let history = backend
            .history(&repo, "heads/main", 5)
            .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor));
        let sealed: Vec<&str> = history.blocks[0]
            .messages
            .iter()
            .map(|m| m.patch_id.as_str())
            .collect();
        println!(
            "0.{}: the card named {rows:?}; the sealed block {} holds patch ids {sealed:?}",
            bin.minor, history.blocks[0].block_id
        );
        assert_eq!(sealed.len(), shorts.len(), "0.{}", bin.minor);
        for short in &shorts {
            assert!(
                sealed.iter().any(|id| id.starts_with(short.as_str())),
                "0.{}: no sealed patch id begins with {short}: {sealed:?}",
                bin.minor
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 032: what a commit will author — declared renames, and a ref with no published history.
// ---------------------------------------------------------------------------------------------

/// A sealed `heads/main` holding `a.txt` and `keep.txt`, beside the fixture's `readme.txt`: the base every RFC
/// 032 rename row starts from (handoff §2).
fn rfc032_sealed_base(bin: &PrikkBin) -> (Fixture, CliBackend, std::path::PathBuf) {
    let fixture = Fixture::build(bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();
    for (name, body) in [("a.txt", "alpha\n"), ("keep.txt", "keep\n")] {
        std::fs::write(repo.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
    }
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: base commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: base seal: {e}", bin.minor));
    Fixture::clear_env();
    (fixture, backend, repo)
}

fn rfc032_prikk(bin: &PrikkBin, repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(&bin.path)
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| panic!("0.{}: spawn prikk {args:?}: {e}", bin.minor))
}

fn rfc032_mv(bin: &PrikkBin, repo: &std::path::Path, from: &str, to: &str) {
    let out = rfc032_prikk(bin, repo, &["mv", from, to]);
    assert!(
        out.status.success(),
        "0.{}: prikk mv {from} {to}: {}",
        bin.minor,
        String::from_utf8_lossy(&out.stderr).trim()
    );
}

fn rfc032_write(repo: &std::path::Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).unwrap_or_else(|e| panic!("write {name}: {e}"));
}

/// A raw `prikk commit` signed as AUTHOR: whether it succeeded, and everything it printed (stdout, then
/// stderr). **The surface is `prikk commit`'s printed output**, whose deletion word is `delete-file` (RFC 032 A4).
fn rfc032_raw_commit(bin: &PrikkBin, fixture: &Fixture, repo: &std::path::Path) -> (bool, String) {
    fixture.set_author_env();
    let out = rfc032_prikk(
        bin,
        repo,
        &[
            "commit",
            "--from-worktree",
            "--ref",
            "heads/main",
            "-m",
            "rfc032 probe",
        ],
    );
    Fixture::clear_env();
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn rfc032_preview(
    bin: &PrikkBin,
    fixture: &Fixture,
    backend: &CliBackend,
    repo: &std::path::Path,
) -> stikk_core::CommitPreviewOutcome {
    fixture.set_author_env();
    let outcome = stikk_core::commit_preview(backend, repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: commit_preview: {e}", bin.minor));
    Fixture::clear_env();
    outcome
}

fn rfc032_printed_line(printed: &str, line: &str) -> bool {
    printed.lines().any(|l| l.trim() == line)
}

fn rfc032_skip_below_0_38(bin: &PrikkBin, what: &str) -> bool {
    if bin.minor < 38 {
        eprintln!(
            "RFC 032: {what} SKIPPED at 0.{} — `prikk mv` does not exist below 0.38, so no rename can be declared \
             there. Announced rather than silent (RFC 022 §3).",
            bin.minor
        );
        return true;
    }
    false
}

type Rfc032Setup = fn(&PrikkBin, &std::path::Path);

/// **RFC 032 at prikk ≥ 0.38: a declared rename prikk lists both halves of is marked, counted, and authored as
/// a rename.** Rows A, B and C (handoff §2), and the four states the dev team measured beside them: an
/// unrelated deletion, a file moved over the destination, a declaration rewritten by a second `prikk mv`, and
/// two renames. Each asserts the Changes view's marks, the confirmation's count and line, and the operations
/// `prikk commit`'s printed output names.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_a_paired_declared_rename_is_marked_counted_and_authored_as_a_rename() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let cases: [(&str, Rfc032Setup, u64, &[&str]); 7] = [
        (
            "row A: mv",
            |bin, repo| rfc032_mv(bin, repo, "a.txt", "b.txt"),
            1,
            &["rename-path a.txt -> b.txt"],
        ),
        (
            "row B: mv, keep.txt edited",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_write(repo, "keep.txt", "keep\nmore\n");
            },
            1,
            &["edit-text keep.txt", "rename-path a.txt -> b.txt"],
        ),
        (
            "row C: mv, b.txt edited",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_write(repo, "b.txt", "alpha\nmore\n");
            },
            1,
            &["rename-path a.txt -> b.txt", "edit-text b.txt"],
        ),
        (
            "mv, an unrelated tracked file deleted",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::remove_file(repo.join("keep.txt")).unwrap();
            },
            1,
            &["delete-file keep.txt", "rename-path a.txt -> b.txt"],
        ),
        (
            "mv, the shell moves keep.txt over b.txt",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::rename(repo.join("keep.txt"), repo.join("b.txt")).unwrap();
            },
            1,
            &[
                "delete-file keep.txt",
                "edit-text b.txt",
                "rename-path a.txt -> b.txt",
            ],
        ),
        (
            "prikk mv a.txt b.txt, then prikk mv b.txt c.txt",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_mv(bin, repo, "b.txt", "c.txt");
            },
            1,
            &["rename-path a.txt -> c.txt"],
        ),
        (
            "two renames",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_mv(bin, repo, "keep.txt", "k2.txt");
            },
            2,
            &[
                "rename-path a.txt -> b.txt",
                "rename-path keep.txt -> k2.txt",
            ],
        ),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc032_skip_below_0_38(&bin, "paired declared renames") {
            continue;
        }
        for (name, setup, renames, operations) in cases {
            let ctx = format!("0.{} {name}", bin.minor);
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            setup(&bin, &repo);

            let read = stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
            println!("{ctx}: {:?}", read.view.declared_renames);
            assert_eq!(read.history, stikk_core::RefHistory::Published, "{ctx}");
            assert_eq!(read.view.renames, renames, "{ctx}: {:?}", read.view);
            assert!(
                read.view
                    .declared_renames
                    .iter()
                    .all(|d| d.state == stikk_core::DeclarationState::Paired),
                "{ctx}: {:?}",
                read.view.declared_renames
            );
            let marked = read
                .view
                .entries
                .iter()
                .filter(|e| e.rename.is_some())
                .count() as u64;
            assert_eq!(
                marked,
                2 * renames,
                "{ctx}: both halves of each pair, nothing else"
            );

            let stikk_core::CommitPreviewOutcome::Ready { token, .. } =
                rfc032_preview(&bin, &fixture, &backend, &repo)
            else {
                panic!("{ctx}: expected a Ready preview");
            };
            let summary = token.summary();
            assert!(
                summary.counts.contains(&("renames", renames)),
                "{ctx}: {:?}",
                summary.counts
            );
            assert_eq!(
                summary.rename_note.as_deref(),
                Some(stikk_core::RENAMES_ALSO_COUNTED),
                "{ctx}"
            );
            assert!(summary.declaration_notices.is_empty(), "{ctx}");

            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(committed, "{ctx}: prikk commit:\n{printed}");
            for operation in operations {
                assert!(
                    rfc032_printed_line(&printed, operation),
                    "{ctx}: `prikk commit`'s printed output names `{operation}`:\n{printed}"
                );
            }
            let authored = printed
                .lines()
                .filter(|l| l.trim().starts_with("rename-path "))
                .count() as u64;
            assert_eq!(
                authored, renames,
                "{ctx}: renames authored, per `prikk commit`'s output:\n{printed}"
            );
        }
    }
}

/// **RFC 032 at prikk ≥ 0.38: a declaration whose destination is not listed is named, never marked or counted,
/// and `prikk commit` authors a deletion.** Rows 1 and 3, and the destination replaced by a directory. prikk's own
/// line for rows 1 and 3 is asserted too: it is what makes the destination-absent sentence true.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_a_declaration_without_its_destination_is_named_and_commit_authors_a_deletion() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    const SENTENCE: &str = "declared rename a.txt → b.txt: b.txt is not a file in the worktree, so prikk will not author it as a rename";
    let cases: [(&str, Rfc032Setup, &[&str]); 3] = [
        (
            "row 1: mv, b.txt deleted",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::remove_file(repo.join("b.txt")).unwrap();
            },
            &[
                "delete-file a.txt",
                "declaration a.txt -> b.txt: destination is gone; recorded as a deletion, not a rename",
            ],
        ),
        (
            "row 3: mv, the shell moves b.txt to c.txt",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::rename(repo.join("b.txt"), repo.join("c.txt")).unwrap();
            },
            &[
                "delete-file a.txt",
                "create-file c.txt",
                "declaration a.txt -> b.txt: destination is gone; recorded as a deletion, not a rename",
            ],
        ),
        (
            "mv, b.txt replaced by a directory",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::remove_file(repo.join("b.txt")).unwrap();
                std::fs::create_dir(repo.join("b.txt")).unwrap();
                rfc032_write(repo, "b.txt/q.txt", "q\n");
            },
            &[
                "delete-file a.txt",
                "create-file b.txt/q.txt",
                "declaration a.txt -> b.txt: destination is ignored; recorded as a deletion, not a rename",
            ],
        ),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc032_skip_below_0_38(&bin, "declarations without their destination") {
            continue;
        }
        for (name, setup, printed_lines) in cases {
            let ctx = format!("0.{} {name}", bin.minor);
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            setup(&bin, &repo);

            let read = stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
            assert_eq!(read.view.renames, 0, "{ctx}");
            assert!(
                read.view.entries.iter().all(|e| e.rename.is_none()),
                "{ctx}: no mark"
            );
            let notices: Vec<String> = read
                .view
                .declared_renames
                .iter()
                .filter_map(stikk_core::DeclaredRename::notice)
                .collect();
            assert_eq!(
                notices,
                [SENTENCE],
                "{ctx}: {:?}",
                read.view.declared_renames
            );

            let stikk_core::CommitPreviewOutcome::Ready { token, .. } =
                rfc032_preview(&bin, &fixture, &backend, &repo)
            else {
                panic!("{ctx}: expected a Ready preview");
            };
            let summary = token.summary();
            assert!(
                summary.counts.iter().all(|(label, _)| *label != "renames"),
                "{ctx}"
            );
            assert_eq!(summary.rename_note, None, "{ctx}");
            assert_eq!(summary.declaration_notices, [SENTENCE], "{ctx}");

            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(committed, "{ctx}: prikk commit:\n{printed}");
            for line in printed_lines {
                assert!(
                    rfc032_printed_line(&printed, line),
                    "{ctx}: `prikk commit`'s printed output says `{line}`:\n{printed}"
                );
            }
            assert!(
                !printed
                    .lines()
                    .any(|l| l.trim().starts_with("rename-path ")),
                "{ctx}: no rename authored:\n{printed}"
            );
        }
    }
}

/// **RFC 032 at prikk ≥ 0.38: a declaration whose source is present again is named, and `prikk commit` refuses.**
/// Row 2, and its variant with the source recreated with different content, which prikk lists as `modified`. A
/// notice, never a prevention (RFC 027's ruling). **No way out is offered here**, and prikk's own refusal of the
/// move with both copies present is recorded.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_a_declaration_whose_source_is_back_is_named_and_prikk_refuses_the_commit() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    const SENTENCE: &str = "declared rename a.txt → b.txt: a.txt is present again, and prikk refuses to commit until the declaration is resolved";
    const REFUSAL: &str =
        "precondition not met: a.txt -> b.txt: the source is present in the worktree again";
    let cases: [(&str, Rfc032Setup); 2] = [
        ("row 2: mv, a.txt recreated", |bin, repo| {
            rfc032_mv(bin, repo, "a.txt", "b.txt");
            rfc032_write(repo, "a.txt", "alpha\n");
        }),
        ("mv, a.txt recreated with other content", |bin, repo| {
            rfc032_mv(bin, repo, "a.txt", "b.txt");
            rfc032_write(repo, "a.txt", "other\n");
        }),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc032_skip_below_0_38(&bin, "declarations whose source is back") {
            continue;
        }
        for (name, setup) in cases {
            let ctx = format!("0.{} {name}", bin.minor);
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            setup(&bin, &repo);

            let read = stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
            assert_eq!(
                read.view
                    .declared_renames
                    .iter()
                    .map(|d| d.state.clone())
                    .collect::<Vec<_>>(),
                [stikk_core::DeclarationState::SourcePresent {
                    destination_listed: true
                }],
                "{ctx}: {:?}",
                read.view
            );
            let stikk_core::CommitPreviewOutcome::Ready { token, .. } =
                rfc032_preview(&bin, &fixture, &backend, &repo)
            else {
                panic!("{ctx}: a notice, not a prevention: commit is still offered");
            };
            assert_eq!(token.summary().declaration_notices, [SENTENCE], "{ctx}");

            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(!committed, "{ctx}: prikk commit must refuse:\n{printed}");
            assert!(
                printed.contains(REFUSAL),
                "{ctx}: prikk's refusal text:\n{printed}"
            );

            let moved_back = rfc032_prikk(&bin, &repo, &["mv", "b.txt", "a.txt"]);
            assert!(
                !moved_back.status.success(),
                "{ctx}: with both copies present prikk refuses the move, which is why stikk offers no way out"
            );
        }
    }
}

/// **RFC 032 at prikk ≥ 0.38: row 5, recorded as 0.42 does it, and the measured way out.** `prikk mv a.txt b.txt`,
/// then the shell moves `b.txt` back. prikk reports the worktree clean with the declaration still listed, and
/// `prikk commit` refuses (F8). commit's preview is blocked as clean, and its reason carries the notice with the
/// way out (A3). Then `prikk mv b.txt a.txt` drops the declaration, and the preview is blocked as clean with no
/// notice.
///
/// **If a later prikk clears the declaration when the destination is moved back, this test fails and says so**;
/// it does not assert that the loop is right.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_row_5_is_recorded_as_prikk_reports_it_and_prikk_mv_back_drops_the_declaration() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    const CLEAN: &str =
        "the worktree matches this ref's replay baseline — there is nothing to commit";
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc032_skip_below_0_38(&bin, "row 5 and its way out") {
            continue;
        }
        let ctx = format!("0.{} row 5", bin.minor);
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        rfc032_mv(&bin, &repo, "a.txt", "b.txt");
        std::fs::rename(repo.join("b.txt"), repo.join("a.txt")).unwrap();

        let status = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: worktree_status: {e}"));
        assert!(
            !status.declarations.is_empty(),
            "{ctx}: RFC 032 F8 CHANGED — prikk no longer lists the declaration once the destination is moved back \
             to the source. Revisit decision 2, amendment A3 and this test: {status:?}"
        );
        assert!(status.clean, "{ctx}: prikk reports row 5 clean: {status:?}");

        match rfc032_preview(&bin, &fixture, &backend, &repo) {
            stikk_core::CommitPreviewOutcome::Blocked(reason) => assert_eq!(
                reason,
                format!(
                    "{CLEAN}; declared rename a.txt → b.txt: a.txt is present again, and prikk refuses to commit until \
                     the declaration is resolved; in a terminal, prikk mv b.txt a.txt drops it"
                ),
                "{ctx}"
            ),
            other => panic!("{ctx}: expected Blocked, got {other:?}"),
        }
        let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
        assert!(!committed, "{ctx}: prikk commit refuses:\n{printed}");
        assert!(
            printed.contains("the source is present in the worktree again"),
            "{ctx}: prikk's refusal:\n{printed}"
        );

        // The way out, as measured (A3).
        let way_out = rfc032_prikk(&bin, &repo, &["mv", "b.txt", "a.txt"]);
        let said = String::from_utf8_lossy(&way_out.stdout);
        assert!(
            way_out.status.success(),
            "{ctx}: prikk mv b.txt a.txt: {}",
            String::from_utf8_lossy(&way_out.stderr)
        );
        println!("{ctx}: prikk mv b.txt a.txt printed:\n{said}");
        let after = backend
            .worktree_status(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: worktree_status after the way out: {e}"));
        assert!(
            after.declarations.is_empty(),
            "{ctx}: the declaration is dropped: {after:?}"
        );
        match rfc032_preview(&bin, &fixture, &backend, &repo) {
            stikk_core::CommitPreviewOutcome::Blocked(reason) => {
                assert_eq!(reason, CLEAN, "{ctx}: blocked as clean, with no notice");
            }
            other => panic!("{ctx}: expected Blocked after the way out, got {other:?}"),
        }
    }
}

/// **RFC 032 at both ends: a ref with no published history is named from `refs()` and the queue, and its first
/// commit is not stale** (trap 1; amendment A2). A fresh repository's `heads/main`, then its first commit left
/// unsealed, then a second file, then the seal. And, separately, `heads/main` while the queue belongs to a ref
/// never created.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_a_ref_with_no_published_history_is_named_and_its_first_commit_is_not_stale() {
    use stikk_core::{RefHistory, UnpublishedQueue};
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let ctx = format!("0.{}", bin.minor);
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let read = || {
            stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"))
        };

        // Row D: nothing committed.
        let first = read();
        assert_eq!(
            first.history,
            RefHistory::Unpublished(UnpublishedQueue::Empty),
            "{ctx}"
        );
        assert_eq!(
            first
                .history
                .changes_headline("heads/main", first.view.clean)
                .as_deref(),
            Some(
                "heads/main has no published history — every file is listed as untracked, and a commit would be its first"
            ),
            "{ctx}: {:?}",
            first.view
        );
        let stikk_core::CommitPreviewOutcome::Ready { token, .. } =
            rfc032_preview(&bin, &fixture, &backend, &repo)
        else {
            panic!("{ctx}: expected a Ready preview on an unpublished heads/main");
        };
        assert_eq!(
            token.summary().history_notice.as_deref(),
            Some("heads/main has no published history: this would be its first commit"),
            "{ctx}"
        );
        fixture.set_author_env();
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("{ctx}: readiness: {e}"))
            .readiness;
        let committed = stikk_core::commit_confirm_and_execute(
            &backend,
            &repo,
            *token,
            readiness,
            stikk_core::Evidence::ExplicitYes,
            "first",
        );
        Fixture::clear_env();
        assert!(
            committed.is_ok(),
            "{ctx}: the first commit commits, not stale: {committed:?}"
        );

        // After the first commit, unsealed: still unpublished, and the queue is the baseline (A2).
        let queued = read();
        assert_eq!(
            queued.history,
            RefHistory::Unpublished(UnpublishedQueue::ForThisRef(1)),
            "{ctx}"
        );
        assert!(queued.view.clean, "{ctx}: {:?}", queued.view);
        assert_eq!(
            queued
                .history
                .changes_headline("heads/main", true)
                .as_deref(),
            Some(
                "heads/main has no published history yet — nothing in the worktree beyond its 1 queued patch(es), and nothing is sealed"
            ),
            "{ctx}"
        );
        rfc032_write(&repo, "z.txt", "z\n");
        let more = read();
        let paths: Vec<&str> = more.view.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["z.txt"], "{ctx}: only the new file is untracked");
        assert_eq!(
            more.history
                .changes_headline("heads/main", more.view.clean)
                .as_deref(),
            Some(
                "heads/main has no published history yet — its 1 queued patch(es) are the baseline here, and nothing is sealed"
            ),
            "{ctx}"
        );
        let stikk_core::CommitPreviewOutcome::Ready { token, .. } =
            rfc032_preview(&bin, &fixture, &backend, &repo)
        else {
            panic!("{ctx}: expected a Ready preview for a second commit");
        };
        assert_eq!(
            token.summary().history_notice.as_deref(),
            Some(
                "heads/main has no published history: this adds to its 1 queued patch(es), and nothing is sealed until the queue is sealed"
            ),
            "{ctx}"
        );

        // Sealed: published.
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: seal: {e}"));
        Fixture::clear_env();
        let sealed = read();
        assert_eq!(sealed.history, RefHistory::Published, "{ctx}");
        assert_eq!(
            sealed
                .history
                .changes_headline("heads/main", sealed.view.clean),
            None,
            "{ctx}"
        );

        // A queue for a ref never created: heads/main is named unpublished, and nothing is claimed about a queue.
        let other = Fixture::build(&bin);
        let other_repo = other.repo().to_path_buf();
        other.set_author_env();
        let out = rfc032_prikk(
            &bin,
            &other_repo,
            &[
                "commit",
                "--from-worktree",
                "--ref",
                "heads/other",
                "-m",
                "elsewhere",
            ],
        );
        Fixture::clear_env();
        assert!(
            out.status.success(),
            "{ctx}: prikk commit --ref heads/other: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        rfc032_write(&other_repo, "w.txt", "w\n");
        let elsewhere = stikk_core::changes_view(&backend, &other_repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: changes_view with a queue for heads/other: {e}"));
        assert_eq!(
            elsewhere.history,
            RefHistory::Unpublished(UnpublishedQueue::NotThisRef),
            "{ctx}"
        );
        assert_eq!(
            elsewhere
                .history
                .changes_headline("heads/main", elsewhere.view.clean)
                .as_deref(),
            Some("heads/main has no published history"),
            "{ctx}"
        );
    }
}

/// **RFC 032 amendment A7 at prikk ≥ 0.39: a paired declared rename whose destination prikk refuses.**
///
/// `prikk mv a.txt b.txt`, then `b.txt` replaced by a symlink — measured at 0.42.0, and neither the RFC nor the
/// handoff anticipated it. prikk lists **both halves**, so the declaration is paired and the two rows stay
/// annotated; prikk's verdict on the destination is **`refused`**, and `commit` refuses the whole commit. **So
/// nothing may promise that the rename will be authored**: `content_note` is `None`.
///
/// **Below 0.39** prikk states no per-entry verdict (`refused` is unreported, never zero), and below 0.38 there is
/// no `prikk mv`; both are announced skips. A platform that cannot create a symlink is announced too.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc032_a_paired_rename_whose_destination_prikk_refuses_promises_nothing() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if bin.minor < 39 {
            eprintln!(
                "RFC 032 A7: a refused rename destination SKIPPED at 0.{} — prikk states no per-entry commit verdict \
                 below 0.39 (unreported, never zero), and `prikk mv` does not exist below 0.38. Announced rather than \
                 silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }
        let ctx = format!("0.{}", bin.minor);
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        rfc032_mv(&bin, &repo, "a.txt", "b.txt");
        std::fs::remove_file(repo.join("b.txt"))
            .unwrap_or_else(|e| panic!("{ctx}: remove b.txt: {e}"));
        if let Err(e) = make_symlink("keep.txt", &repo.join("b.txt")) {
            eprintln!(
                "RFC 032 A7: SKIPPED at 0.{} — this platform could not create a symlink ({e}), so a refused rename \
                 destination cannot be built here. Announced rather than silent (RFC 022 §3).",
                bin.minor
            );
            continue;
        }

        let read = stikk_core::changes_view(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
        assert_eq!(
            read.view
                .declared_renames
                .iter()
                .map(|d| d.state.clone())
                .collect::<Vec<_>>(),
            [stikk_core::DeclarationState::Paired],
            "{ctx}: prikk lists both halves: {:?}",
            read.view
        );
        assert_eq!(read.view.renames, 1, "{ctx}");
        assert_eq!(read.view.refused, Some(1), "{ctx}: {:?}", read.view);
        assert_eq!(
            read.view.content_note(),
            None,
            "{ctx}: nothing may promise a rename while prikk would refuse the commit"
        );
        let destination = read
            .view
            .entries
            .iter()
            .find(|e| e.path == "b.txt")
            .unwrap_or_else(|| panic!("{ctx}: b.txt not listed: {:?}", read.view));
        assert!(destination.rename.is_some(), "{ctx}: the annotation stays");
        let stikk_core::Authoring::Refused(reason) = &destination.authoring else {
            panic!("{ctx}: b.txt is not refused: {destination:?}");
        };

        // prikk's own commit on this very tree, and stikk's prevention, agree with that verdict.
        fixture.set_author_env();
        let attempt = backend.commit(&repo, "heads/main", "would be refused");
        let preview = stikk_core::commit_preview(&backend, &repo, "heads/main");
        Fixture::clear_env();
        let message = match attempt {
            Err(StikkError::Refusal { message }) => message,
            other => panic!("{ctx}: expected prikk to refuse, got {other:?}"),
        };
        assert_eq!(
            message.strip_prefix("error: "),
            Some(reason.as_str()),
            "{ctx}: the verdict and commit's own refusal disagree"
        );
        match preview {
            Ok(stikk_core::CommitPreviewOutcome::WouldRefuse(paths)) => assert!(
                paths.iter().any(|p| p.path == "b.txt"),
                "{ctx}: commit is unavailable, carrying b.txt: {paths:?}"
            ),
            other => panic!("{ctx}: expected WouldRefuse, got {other:?}"),
        }
    }
}

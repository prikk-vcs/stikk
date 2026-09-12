//! The real-binary integration suite's actual test functions (`TS-07`; RFC 019). See
//! `stikk_real_binary`'s crate-level doc for the boundary this holds and why it lives in its own crate.
//!
//! **Coverage: all ten `Prikk` seam methods, at both ends of the supported range** (RFC 022 §2).
//! `handshake` is driven by the version guard every test runs through [`PrikkBin::resolve`];
//! `orientation`, `history`, `commit` and `seal` since RFC 019; `worktree_status`, `block_state`,
//! `refs`, `tags` and `change_token` since RFC 022. (0.5.0's changelog said "four of the nine
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

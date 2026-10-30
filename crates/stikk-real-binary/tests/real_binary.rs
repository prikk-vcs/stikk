//! The real-binary integration suite's actual test functions (`TS-07`; RFC 019). See
//! `stikk_real_binary`'s crate-level doc for the boundary this holds and why it lives in its own crate.
//!
//! **Coverage: all eighteen `Prikk` seam methods, at both ends of the supported range** (RFC 022 §2).
//! `handshake` is driven by the version guard every test runs through [`PrikkBin::resolve`];
//! `orientation`, `history`, `commit` and `seal` since RFC 019; `worktree_status`, `block_state`,
//! `refs`, `tags` and `change_token` since RFC 022; `readiness` since 0.6.0's preparation; `queue`
//! since RFC 028; `show` since RFC 035; `diff` since RFC 038; `verify` since RFC 040; and
//! `create_branch`, `create_tag` and `close_branch` since RFC 044. (The count read "twelve" from
//! RFC 022 until RFC 044's sweep: `show`, `diff` and `verify` each arrived with legs of their own
//! and none of them came back to this line.) (0.5.0's changelog said "four of the nine
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
            .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor))
            .refs;
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

        let stikk_prikk::StateReport::Listed(state) = backend
            .block_state(fixture.repo(), "heads/main", &seal.block_id, true)
            .unwrap_or_else(|e| panic!("0.{}: block_state: {e}", bin.minor))
        else {
            panic!(
                "0.{}: the tip's files are listed at every supported version",
                bin.minor
            )
        };

        // The replayed state is the block this test just sealed, not some other tip.
        assert_eq!(
            state.target_block, seal.block_id,
            "0.{}: block_state replayed a different block than seal reported",
            bin.minor
        );
        let mut files: Vec<String> = state.files.iter().map(|f| f.path.clone()).collect();
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
///
/// **`heads/other` is published here, and that is the RFC 034 change** (2026-09-22). This test used to ask
/// about a ref that was never created, which every prikk through 0.44 answered with an ordinary report.
/// **prikk ≥ 0.45 refuses a ref it considers absent**, and the RFC 034 repair deliberately does not paper
/// over that: its retry reads the current branch, sees the report name `heads/main`, and returns prikk's
/// original refusal rather than another ref's report (`T-T4`, Handoff A §3 trap 2). **So the arrangement
/// was wrong, not the code** — the queued-elsewhere fact is about a real ref either way, and publishing it
/// measures the same thing at both ends.
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

        // heads/main published, then heads/other published from it.
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "baseline")
            .unwrap_or_else(|e| panic!("0.{}: baseline commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        // `branch create` signs as MAINTAINER from prikk 0.42, so its env stays set across it.
        let created = std::process::Command::new(&bin.path)
            .args(["branch", "create", "heads/other", "--from", "heads/main"])
            .current_dir(&repo)
            .output()
            .unwrap_or_else(|e| panic!("0.{}: spawn prikk branch create: {e}", bin.minor));
        Fixture::clear_env();
        assert!(
            created.status.success(),
            "0.{}: prikk branch create: {}",
            bin.minor,
            String::from_utf8_lossy(&created.stderr).trim()
        );

        // One unsealed patch queued for heads/main; then ask about heads/other.
        std::fs::write(repo.join("queued.txt"), "queued\n")
            .unwrap_or_else(|e| panic!("0.{}: write queued.txt: {e}", bin.minor));
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
            .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor))
            .refs;
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
        // RFC 034: inside the band prikk fills its own verdict on the same declaration, so the
        // expectation follows the band rather than pinning one era's shape.
        let inside = bin.minor >= RFC034_VERDICT_FROM;
        assert_eq!(
            after.declarations,
            vec![stikk_prikk::RenameDeclaration {
                resolution: inside.then_some(stikk_prikk::DeclarationResolution::Rename),
                refusal: None,
                content_changed: inside.then_some(false),
                mode_changed: inside.then_some(false),
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
            other
                .queued_tier()
                .as_deref()
                .map(stikk_core::flatten)
                .as_deref(),
            Some(
                "the active WAL holds 1 patch(es) for heads/main — not this ref's history · Q: Queue"
            ),
            "0.{}: another ref's tier",
            bin.minor
        );
        assert_eq!(
            main.queued_tier()
                .as_deref()
                .map(stikk_core::flatten)
                .as_deref(),
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

/// prikk's own reason for a declaration whose destination is a **directory**, by version (RFC 034 §5).
/// Through 0.43 prikk said the destination is *ignored*; **0.44 renamed it** to name what it actually
/// found — the fix stikk asked for in letter 014 (RFC 034 F5). Measured at 0.43.0 and 0.44.0, and
/// unchanged through 0.46.0.
fn rfc032_directory_destination_reason(minor: u32) -> &'static str {
    const RENAMED_FROM: u32 = 44;
    if minor >= RENAMED_FROM {
        "destination is a directory"
    } else {
        "destination is ignored"
    }
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
            // RFC 034 §4: inside the band the card carries what prikk says the rename **also** does, so
            // the expectation comes from prikk's report rather than from this file's idea of the row.
            let content_words: Vec<String> = read
                .view
                .declared_renames
                .iter()
                .filter_map(stikk_core::DeclaredRename::content_sentence)
                .collect();
            assert_eq!(summary.declaration_notices, content_words, "{ctx}");
            // And where the destination's bytes really did change — edited after the move, or another
            // file moved over it — prikk must be reporting exactly that.
            if bin.minor >= RFC034_VERDICT_FROM
                && (name.contains("b.txt edited") || name.contains("over b.txt"))
            {
                assert_eq!(
                    content_words,
                    ["declared rename a.txt → b.txt: its content changed too"],
                    "{ctx}: prikk reports the content change"
                );
            }

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
    // RFC 034 §3 and §6: inside the band prikk says **why** it will record a deletion; below it, stikk
    // can say only that the report does not list the destination — an ignored destination is in the
    // worktree, and the report cannot tell the two apart.
    const SENTENCE_FROM_0_44: &str = "declared rename a.txt → b.txt: b.txt is not a file in the worktree, so prikk will record a deletion, not a rename";
    const SENTENCE_BELOW_0_44: &str = "declared rename a.txt → b.txt: prikk's report does not list b.txt, so prikk will not author it as a rename";
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
                // `{destination}` is filled per binary: prikk 0.44 renamed this reason (see
                // `rfc032_directory_destination_reason`).
                "declaration a.txt -> b.txt: {destination}; recorded as a deletion, not a rename",
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
            let sentence = if bin.minor >= RFC034_VERDICT_FROM {
                SENTENCE_FROM_0_44
            } else {
                SENTENCE_BELOW_0_44
            };
            assert_eq!(
                notices,
                [sentence],
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
            assert_eq!(summary.declaration_notices, [sentence], "{ctx}");

            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(committed, "{ctx}: prikk commit:\n{printed}");
            for line in printed_lines {
                let line = line.replace(
                    "{destination}",
                    rfc032_directory_destination_reason(bin.minor),
                );
                assert!(
                    rfc032_printed_line(&printed, &line),
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
    // prikk reworded this refusal at 0.43 (RFC 034 §5). Measured: 0.42.0 says the declared move "was
    // not completed on disk"; 0.43.0, 0.44.0, 0.45.0 and 0.46.0 all name the two ways out prikk will
    // accept instead. The clause below is the stable half of each wording.
    const REWORDED_FROM: u32 = 43;
    const REFUSAL_THROUGH_0_42: &str = "the source is present in the worktree again";
    const REFUSAL_FROM_0_43: &str = "both paths exist in the worktree, so which one is the tracked node is not prikk's to guess";
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
            // **RFC 034 §5 moved this line.** Below the band prikk states no verdict stikk may act on,
            // so RFC 032 could only annotate an offered commit; inside the band prikk refuses the
            // declaration itself, and stikk prevents the commit with prikk's own words.
            match rfc032_preview(&bin, &fixture, &backend, &repo) {
                stikk_core::CommitPreviewOutcome::WouldRefuseDeclarations(declarations)
                    if bin.minor >= RFC034_VERDICT_FROM =>
                {
                    let [refused] = declarations.as_slice() else {
                        panic!("{ctx}: one refused declaration: {declarations:?}");
                    };
                    assert!(
                        refused.reason.contains("a.txt -> b.txt"),
                        "{ctx}: prikk's own refusal: {}",
                        refused.reason
                    );
                }
                stikk_core::CommitPreviewOutcome::Ready { token, .. }
                    if bin.minor < RFC034_VERDICT_FROM =>
                {
                    assert_eq!(token.summary().declaration_notices, [SENTENCE], "{ctx}");
                }
                other => panic!("{ctx}: unexpected outcome for this band: {other:?}"),
            }

            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(!committed, "{ctx}: prikk commit must refuse:\n{printed}");
            assert!(
                printed.contains(if bin.minor >= REWORDED_FROM {
                    REFUSAL_FROM_0_43
                } else {
                    REFUSAL_THROUGH_0_42
                }),
                "{ctx}: `prikk commit`'s printed output carries prikk's own refusal:\n{printed}"
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

        // **RFC 034 §5 moved this line too**, and this is the row that decides the check's order:
        // prikk calls the worktree clean and exits 0, so inside the band the refused-declaration check
        // must run **before** the clean check or prikk's verdict disappears behind "nothing to commit".
        match rfc032_preview(&bin, &fixture, &backend, &repo) {
            stikk_core::CommitPreviewOutcome::WouldRefuseDeclarations(declarations)
                if bin.minor >= RFC034_VERDICT_FROM =>
            {
                let [refused] = declarations.as_slice() else {
                    panic!("{ctx}: one refused declaration: {declarations:?}");
                };
                assert!(
                    refused
                        .reason
                        .contains("the source is present in the worktree again"),
                    "{ctx}: prikk's own refusal: {}",
                    refused.reason
                );
            }
            stikk_core::CommitPreviewOutcome::Blocked(reason)
                if bin.minor < RFC034_VERDICT_FROM =>
            {
                assert_eq!(
                    reason,
                    format!(
                        "{CLEAN}; declared rename a.txt → b.txt: a.txt is present again, and prikk refuses to commit until \
                         the declaration is resolved; in a terminal, prikk mv b.txt a.txt drops it"
                    ),
                    "{ctx}"
                );
            }
            other => panic!("{ctx}: unexpected outcome for this band: {other:?}"),
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

// ---------------------------------------------------------------------------------------------
// RFC 034 Handoff A: prikk ≥ 0.45 refuses `--ref R` for a ref with no published history, and stikk
// retries without `--ref` — accepting the report **only** when it names the ref that was asked for.
// ---------------------------------------------------------------------------------------------

/// **An absent ref must still refuse** (Handoff A §3 trap 2). A mistyped ref is the case that would turn
/// the repair into a lie: on prikk ≥ 0.45 the retry returns *the current branch's* report, and stikk must
/// reject it and show prikk's own refusal instead of another ref's worktree.
///
/// Below 0.45 prikk answers a never-created ref with an ordinary report naming it, and stikk shows that,
/// unchanged — the repair never fires, because prikk never refuses.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc034_an_absent_ref_is_never_answered_with_another_refs_report() {
    const REFUSES_ABSENT_REF_FROM: u32 = 45;
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let ctx = format!("0.{}", bin.minor);

        let status = backend.worktree_status(&repo, "heads/typo");
        let history = backend.history(&repo, "heads/typo", 20);

        if bin.minor >= REFUSES_ABSENT_REF_FROM {
            for (what, message) in [
                (
                    "worktree_status",
                    status.as_ref().err().map(ToString::to_string),
                ),
                ("history", history.as_ref().err().map(ToString::to_string)),
            ] {
                let message = message.unwrap_or_else(|| {
                    panic!("{ctx}: {what} must refuse a ref prikk calls absent, got a report")
                });
                assert!(
                    message.contains("heads/typo")
                        && message.contains("does not exist in this repository"),
                    "{ctx}: {what} shows prikk's own refusal: {message}"
                );
                assert!(
                    !message.contains("heads/main"),
                    "{ctx}: {what} must not mention the ref stikk read instead: {message}"
                );
            }
            // The decisive one: nothing about heads/main reached the caller as heads/typo's answer.
            assert!(status.is_err() && history.is_err(), "{ctx}");
        } else {
            let status =
                status.unwrap_or_else(|e| panic!("{ctx}: prikk reports an absent ref here: {e}"));
            let history =
                history.unwrap_or_else(|e| panic!("{ctx}: prikk reports an absent ref here: {e}"));
            assert_eq!(
                status.reff, "heads/typo",
                "{ctx}: prikk's own answer, unchanged"
            );
            assert_eq!(history.reff, "heads/typo", "{ctx}");
            assert!(history.blocks.is_empty(), "{ctx}: {history:?}");
        }
    }
}

/// **A repository's first commit, end to end** — the path prikk 0.45 broke and RFC 034 repairs (F0).
///
/// On a fresh repository nothing is published, so at prikk ≥ 0.45 every `--ref` read of `heads/main`
/// refuses and the repair's retry carries each one. This drives what a first-run user actually does:
/// Changes lists the untracked files under RFC 032's no-published-history words, History is empty rather
/// than refused, commit previews and confirms, and after `seal` the same reads work natively.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc034_a_first_commit_on_a_fresh_repository_reads_previews_and_commits_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let ctx = format!("0.{}", bin.minor);

        // Changes, on a ref with no published history.
        let read = stikk_core::changes_view(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: changes_view on a fresh repository: {e}"));
        assert_eq!(
            read.history,
            stikk_core::RefHistory::Unpublished(stikk_core::UnpublishedQueue::Empty),
            "{ctx}"
        );
        assert_eq!(
            read.history
                .changes_headline("heads/main", read.view.clean)
                .as_deref(),
            Some(
                "heads/main has no published history — every file is listed as untracked, and a commit would be its first"
            ),
            "{ctx}"
        );
        assert!(
            read.view.entries.iter().any(|e| e.path == "readme.txt"),
            "{ctx}: the untracked file is listed: {:?}",
            read.view.entries
        );

        // History, likewise: empty, not refused (RFC 029 B).
        let history = backend
            .history(&repo, "heads/main", 20)
            .unwrap_or_else(|e| panic!("{ctx}: history of an unpublished heads/main: {e}"));
        assert_eq!(history.reff, "heads/main", "{ctx}");
        assert!(history.blocks.is_empty(), "{ctx}: {history:?}");

        // Commit previews, confirms and queues.
        fixture.set_author_env();
        let outcome = stikk_core::commit_preview(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: commit_preview on a fresh repository: {e}"));
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("{ctx}: readiness: {e}"))
            .readiness;
        let stikk_core::CommitPreviewOutcome::Ready { token, .. } = outcome else {
            panic!("{ctx}: expected a Ready preview, got {outcome:?}");
        };
        assert_eq!(
            token.summary().history_notice.as_deref(),
            Some("heads/main has no published history: this would be its first commit"),
            "{ctx}"
        );
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
            "{ctx}: the first commit must commit, not go stale: {committed:?}"
        );

        // Sealed: published, and every read works without the retry.
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: seal: {e}"));
        Fixture::clear_env();
        let sealed = stikk_core::changes_view(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("{ctx}: changes_view after seal: {e}"));
        assert_eq!(sealed.history, stikk_core::RefHistory::Published, "{ctx}");
        let history = backend
            .history(&repo, "heads/main", 20)
            .unwrap_or_else(|e| panic!("{ctx}: history after seal: {e}"));
        assert_eq!(history.blocks.len(), 1, "{ctx}: {history:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 034 decision 4: at prikk ≥ 0.44 stikk says what prikk's own classifier resolves, and a
// declaration prikk would refuse prevents the commit.
// ---------------------------------------------------------------------------------------------

/// The first prikk whose declaration verdict stikk reads (RFC 034 §1; 0.43's classifier is wrong).
const RFC034_VERDICT_FROM: u32 = 44;

fn rfc034_skip_below_the_band(bin: &PrikkBin, what: &str) -> bool {
    if bin.minor < RFC034_VERDICT_FROM {
        eprintln!(
            "RFC 034: {what} SKIPPED at 0.{} — prikk states no declaration verdict stikk will read below \
             0.{RFC034_VERDICT_FROM} (0.43 reports one whose classifier is wrong, measured), and `prikk mv` \
             does not exist below 0.38. Announced rather than silent (RFC 022 §3).",
            bin.minor
        );
        return true;
    }
    false
}

/// **RFC 034 §3–§4 against real binaries: prikk's own resolution, and what stikk says about it.**
///
/// Each row is a state measured at 0.44.0 and 0.46.0. The assertion pairs **stikk's sentence** with
/// **`prikk commit`'s own printed output** for the same worktree, so a wording that drifts apart from
/// what prikk does fails here rather than in front of a user (RFC 032 A4 — the surface is named).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc034_stikk_says_what_prikks_resolution_says_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    type Row = (
        &'static str,
        Rfc032Setup,
        stikk_core::DeclarationResolution,
        Option<&'static str>,
        &'static str,
    );
    let rows: [Row; 7] = [
        (
            "rename",
            |bin, repo| rfc032_mv(bin, repo, "a.txt", "b.txt"),
            stikk_core::DeclarationResolution::Rename,
            None,
            "rename-path a.txt -> b.txt",
        ),
        (
            "rename, content changed",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_write(repo, "b.txt", "alpha\nmore\n");
            },
            stikk_core::DeclarationResolution::Rename,
            Some("declared rename a.txt → b.txt: its content changed too"),
            "edit-text b.txt",
        ),
        (
            "rename, mode changed",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let path = repo.join("b.txt");
                    let mut perms = std::fs::metadata(&path).unwrap().permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&path, perms).unwrap();
                }
            },
            stikk_core::DeclarationResolution::Rename,
            Some("declared rename a.txt → b.txt: its mode changed too"),
            "change-perm b.txt",
        ),
        (
            "deletion, destination gone",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::remove_file(repo.join("b.txt")).unwrap();
            },
            stikk_core::DeclarationResolution::Deletion,
            Some(
                "declared rename a.txt → b.txt: b.txt is not a file in the worktree, so prikk will record a deletion, not a rename",
            ),
            "delete-file a.txt",
        ),
        (
            "deletion, destination is a directory",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::remove_file(repo.join("b.txt")).unwrap();
                std::fs::create_dir(repo.join("b.txt")).unwrap();
                rfc032_write(repo, "b.txt/q.txt", "q\n");
            },
            stikk_core::DeclarationResolution::Deletion,
            Some(
                "declared rename a.txt → b.txt: b.txt is not a file in the worktree, so prikk will record a deletion, not a rename",
            ),
            "declaration a.txt -> b.txt: destination is a directory; recorded as a deletion, not a rename",
        ),
        (
            "deletion-ignored",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_write(repo, ".prikkignore", "b.txt\n");
            },
            stikk_core::DeclarationResolution::DeletionIgnored,
            Some(
                "declared rename a.txt → b.txt: b.txt is ignored, so prikk will record a deletion, not a rename",
            ),
            "declaration a.txt -> b.txt: destination is ignored; recorded as a deletion, not a rename",
        ),
        (
            "never-tracked",
            |bin, repo| {
                rfc032_write(repo, "new.txt", "new\n");
                rfc032_mv(bin, repo, "new.txt", "new2.txt");
            },
            stikk_core::DeclarationResolution::NeverTracked,
            Some(
                "declared rename new.txt → new2.txt: new.txt was never tracked, so prikk will create new2.txt and drop the declaration",
            ),
            "declaration new.txt -> new2.txt: source was never a tracked node; there was no node to rename",
        ),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc034_skip_below_the_band(&bin, "prikk's declaration resolutions") {
            continue;
        }
        for (name, setup, resolution, sentence, printed) in &rows {
            let ctx = format!("0.{} {name}", bin.minor);
            // Windows has no executable bit, so the mode row's arrangement cannot arrange
            // anything: prikk correctly reports `mode_changed: false` and stikk correctly says
            // nothing. Announced rather than silent (RFC 022 §3).
            if *name == "rename, mode changed" && !cfg!(unix) {
                eprintln!("{ctx}: SKIPPED — this platform has no executable bit to change");
                continue;
            }
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            setup(&bin, &repo);

            let read = stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
            let declaration = read
                .view
                .declared_renames
                .first()
                .unwrap_or_else(|| panic!("{ctx}: no declaration: {:?}", read.view));
            assert_eq!(
                declaration.resolution.as_ref(),
                Some(resolution),
                "{ctx}: prikk's own word"
            );
            let said = declaration
                .notice()
                .or_else(|| declaration.content_sentence());
            assert_eq!(said.as_deref(), *sentence, "{ctx}: what stikk says");
            assert_eq!(
                read.view.renames,
                u64::from(*resolution == stikk_core::DeclarationResolution::Rename),
                "{ctx}: only a resolved rename is counted"
            );

            let (committed, output) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(committed, "{ctx}: prikk commit:\n{output}");
            assert!(
                rfc032_printed_line(&output, printed),
                "{ctx}: `prikk commit`'s printed output says `{printed}`:\n{output}"
            );
        }
    }
}

/// **RFC 034 §5 against real binaries: a declaration prikk would refuse makes commit unavailable**, and
/// the reason stikk shows is **prikk's own refusal, compared against a real `prikk commit` attempt on
/// the same worktree** — not a literal copied into this file.
///
/// The second row is the one that decides the check's order: prikk reports that worktree **clean** and
/// exits 0, so a clean check running first would hide prikk's verdict behind "nothing to commit".
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc034_a_refused_declaration_prevents_the_commit_at_both_ends() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let rows: [(&str, Rfc032Setup, bool); 2] = [
        (
            "both paths present",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                rfc032_write(repo, "a.txt", "alpha\n");
            },
            false,
        ),
        (
            "the source is back, and prikk calls the worktree clean",
            |bin, repo| {
                rfc032_mv(bin, repo, "a.txt", "b.txt");
                std::fs::rename(repo.join("b.txt"), repo.join("a.txt")).unwrap();
            },
            true,
        ),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc034_skip_below_the_band(&bin, "refused declarations") {
            continue;
        }
        for (name, setup, expect_clean) in rows {
            let ctx = format!("0.{} {name}", bin.minor);
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            setup(&bin, &repo);

            let read = stikk_core::changes_view(&backend, &repo, "heads/main")
                .unwrap_or_else(|e| panic!("{ctx}: changes_view: {e}"));
            assert_eq!(
                read.view.clean, expect_clean,
                "{ctx}: prikk's own `clean`: {:?}",
                read.view
            );
            assert_eq!(read.view.refused_declarations, Some(1), "{ctx}");
            assert_eq!(
                read.view.refused,
                Some(0),
                "{ctx}: no *path* is refused here"
            );

            // prikk's own refusal for this very worktree, from a real commit attempt.
            let (committed, printed) = rfc032_raw_commit(&bin, &fixture, &repo);
            assert!(!committed, "{ctx}: prikk commit must refuse:\n{printed}");
            let prikks_refusal = printed
                .lines()
                .find_map(|line| line.trim().strip_prefix("error: precondition not met: "))
                .unwrap_or_else(|| panic!("{ctx}: prikk's refusal line:\n{printed}"));

            match rfc032_preview(&bin, &fixture, &backend, &repo) {
                stikk_core::CommitPreviewOutcome::WouldRefuseDeclarations(declarations) => {
                    let [refused] = declarations.as_slice() else {
                        panic!("{ctx}: one refused declaration: {declarations:?}");
                    };
                    assert_eq!(refused.old_path, "a.txt", "{ctx}");
                    assert_eq!(refused.new_path, "b.txt", "{ctx}");
                    assert_eq!(
                        refused.reason, prikks_refusal,
                        "{ctx}: stikk shows prikk's own refusal, and `prikk commit` prints the same"
                    );
                }
                other => panic!("{ctx}: expected the declaration prevention, got {other:?}"),
            }
        }
    }
}

/// **RFC 034 §7 against real binaries: a checkout that stopped part-way.**
///
/// The marker is set by a `checkout --patch-materialize` that refuses **after** writing something, and
/// then read by another prikk. **Only an older prikk can set it**: measured at 0.46.0, the checkout
/// refuses with *"nothing was written"* — prikk 0.43 "plans every write before the first", which is the
/// fix RFC 034 F7 records. So this leg has the floor try, and the ceiling read.
///
/// **If the floor cannot set it either, the leg announces a skip rather than passing quietly** (RFC 022
/// §3): the state is then unreachable from the suite's two ends, and the hand measurement in the review
/// request carries it instead (0.42 sets, 0.43 and 0.46 read).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc034_an_interrupted_materialization_is_shown_and_blocks_only_commit() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let floor = PrikkBin::floor();
    let ceiling = PrikkBin::ceiling();
    let fixture = Fixture::build(&floor);
    let repo = fixture.repo().to_path_buf();
    let floor_backend = CliBackend::with_program(&floor.path);
    let ceiling_backend = CliBackend::with_program(&ceiling.path);

    // A published ref, so the floor's checkout has something to materialize.
    fixture.set_author_env();
    floor_backend
        .commit(&repo, "heads/main", "baseline")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", floor.minor));
    fixture.set_maintainer_env();
    floor_backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", floor.minor));
    Fixture::clear_env();

    // A file in the way, so materialization refuses part-way through.
    std::fs::write(repo.join("readme.txt"), "in the way\n")
        .unwrap_or_else(|e| panic!("write: {e}"));
    let attempt = rfc032_prikk(
        &floor,
        &repo,
        &["checkout", "--patch-materialize", "--ref", "heads/main"],
    );
    println!(
        "0.{}: checkout --patch-materialize said: {}",
        floor.minor,
        String::from_utf8_lossy(&attempt.stderr).trim()
    );

    let orientation = stikk_core::orient(&ceiling_backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: orient: {e}", ceiling.minor));
    let Some(marker) = orientation.interrupted_materialization.as_deref() else {
        eprintln!(
            "RFC 034 §7: the interrupted-materialization marker SKIPPED — prikk 0.{} left none, and \
             prikk 0.{} plans every write before the first, so it refuses without writing (measured). \
             The state is unreachable from this suite's two ends; the review request carries the hand \
             measurement (0.42 sets it, 0.43 and 0.46 report it). Announced rather than silent (RFC 022 §3).",
            floor.minor, ceiling.minor
        );
        return;
    };

    // **prikk's routes, each whole** (RFC 057 decision 2): the JSON names the ways out as
    // commands, and stikk carries them exactly.
    assert!(
        marker.iter().any(|route| route.contains("prikk ")),
        "0.{}: {marker:?}",
        ceiling.minor
    );
    // Commit is unavailable, in stikk's sentence with prikk's routes inside it.
    //
    // **The card's wording is not the Orientation view's** (RFC 057 review v1): the view gives
    // each route a row and leads in with *"run one of:"*, while the card folds them into one
    // sentence with each command in backticks, because a card's text wraps on whitespace and
    // two bare commands would run together.
    fixture.set_author_env();
    let preview = stikk_core::commit_preview(&ceiling_backend, &repo, "heads/main");
    Fixture::clear_env();
    match preview {
        Ok(stikk_core::CommitPreviewOutcome::Blocked(reason)) => {
            assert!(
                reason.starts_with("A checkout or branch switch stopped part-way."),
                "0.{}: stikk's sentence: {reason}",
                ceiling.minor
            );
            assert!(
                !reason.contains("prikk reports"),
                "0.{}: the words are stikk's: {reason}",
                ceiling.minor
            );
            for route in marker {
                assert!(
                    reason.contains(&format!("`{route}`")),
                    "0.{}: prikk's route, whole and delimited: {reason}",
                    ceiling.minor
                );
            }
        }
        other => panic!("0.{}: expected Blocked, got {other:?}", ceiling.minor),
    }
    // **Reads are not blocked**, and stikk must not block them.
    stikk_core::changes_view(&ceiling_backend, &repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: Changes must keep working: {e}", ceiling.minor));
    ceiling_backend
        .history(&repo, "heads/main", 20)
        .unwrap_or_else(|e| panic!("0.{}: History must keep working: {e}", ceiling.minor));
}

// =============================================================================================
// RFC 035 — Patch detail, against real binaries (§7).
//
// What these legs exist to catch is the gap between what prikk's report says and what stikk's view
// says about it. Every expectation below is read **from the binary's own report in the same run**,
// not from a literal in this file, except where the literal is stikk's own sentence.
// =============================================================================================

/// stikk's JSON floor, which is also Patch detail's band floor (RFC 035 A1, decision 1 as amended).
/// **Read from the seam, never written here** (review v1 §2.2).
const RFC035_CONTENT_FROM: u32 = stikk_prikk::JSON_REPORTS_FROM_MINOR;

/// One operation's rendered rows as one string. **Content rows arrive quoted and escaped** (review v1
/// §2.1), so an assertion looking for prikk's own text looks for it inside the quotes.
fn rfc035_rows(operation: &stikk_core::OperationView) -> String {
    operation
        .rows
        .iter()
        .map(|row| match row {
            // A row that mixes stikk's words with a repository name is flattened here, which a
            // renderer must never do — this is a test asserting the words (RFC 036 review v1 §2.3).
            stikk_core::OperationRow::Named(pieces) => stikk_core::flatten(pieces),
            other => other.text().unwrap_or_default().to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What stikk shows for a file's content must be what prikk reported it to be (review v1 §2.4): a
/// text file's own text, or a binary file's blob id and byte count — **never its bytes**, and never
/// "no content to show" for a kind prikk named.
fn rfc035_assert_file_content(bin: &PrikkBin, content: &stikk_prikk::FileContent, rendered: &str) {
    match content {
        stikk_prikk::FileContent::Text(text) => {
            for line in text.lines() {
                assert!(
                    rendered.contains(line),
                    "0.{}: prikk's text {line:?} is on screen: {rendered}",
                    bin.minor
                );
            }
        }
        stikk_prikk::FileContent::Binary(blob) => {
            assert!(
                rendered.contains(&blob.blob_id)
                    && rendered.contains(&format!("{} byte(s)", blob.size)),
                "0.{}: prikk's blob id and size: {rendered}",
                bin.minor
            );
            assert!(
                !rendered.contains("no content to show"),
                "0.{}: {rendered}",
                bin.minor
            );
        }
        stikk_prikk::FileContent::Unavailable(blob_id) => assert!(
            rendered.contains(blob_id.as_str()),
            "0.{}: the id prikk could not read is still shown: {rendered}",
            bin.minor
        ),
        stikk_prikk::FileContent::Unrendered(kind) => assert!(
            rendered.contains(kind.as_str()),
            "0.{}: prikk's own kind word: {rendered}",
            bin.minor
        ),
    }
}

fn rfc035_skip_below_the_band(bin: &PrikkBin, what: &str) -> bool {
    if bin.minor < RFC035_CONTENT_FROM {
        eprintln!(
            "RFC 035: {what} SKIPPED at 0.{} — stikk reads a patch's content from prikk \
             0.{RFC035_CONTENT_FROM} on, its JSON floor, and 0.39–0.45 is unmeasured by the project \
             owner's ruling. Announced rather than silent (RFC 022 §3).",
            bin.minor
        );
        return true;
    }
    false
}

/// The tip block of `heads/main`, as `log` reports it.
fn rfc035_tip(
    backend: &CliBackend,
    repo: &std::path::Path,
    bin: &PrikkBin,
) -> stikk_prikk::BlockRow {
    let history = backend
        .history(repo, "heads/main", 5)
        .unwrap_or_else(|e| panic!("0.{}: log: {e}", bin.minor));
    history
        .blocks
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("0.{}: no sealed block", bin.minor))
}

/// **RFC 035 §7 at the ceiling: one sealed patch holding every operation kind prikk emits**, where
/// every path resolves and the words stikk renders are prikk's own fields.
///
/// The repository is the one RFC 035's own measurement script built, and the expectations are read out
/// of `show`'s report in this run: for each operation, stikk's rendered rows must carry prikk's path,
/// its spans, its mode, its blob ids and sizes, and its asserting key id.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc035_every_operation_kind_renders_from_prikks_fields_at_both_ends() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc035_skip_below_the_band(&bin, "every operation kind") {
            continue;
        }
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        // The base seal holds a.txt and keep.txt; add the rest of the kinds' starting state.
        rfc032_write(&repo, "del.txt", "gone\n");
        rfc032_write(&repo, "mode.sh", "x\n");
        std::fs::write(repo.join("b.bin"), [0xff, 0xfe, 0x00, b'b', b'i', b'n'])
            .unwrap_or_else(|e| panic!("write b.bin: {e}"));
        // Review v1 §2.4: a binary file to delete, so the patch below carries a binary preimage as
        // well as a binary create — the content kind §2's table lacked.
        std::fs::write(repo.join("old.bin"), [0xff, 0xfe, 0x00, 1, 2, 3, 4, 5])
            .unwrap_or_else(|e| panic!("write old.bin: {e}"));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "more kinds")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        // One patch, every kind: an edit, a delete, a create, a binary replacement, a mode change and
        // a rename.
        rfc032_write(&repo, "a.txt", "ALPHA\n");
        std::fs::remove_file(repo.join("del.txt")).unwrap_or_else(|e| panic!("remove: {e}"));
        rfc032_write(&repo, "created.txt", "new file\n");
        std::fs::write(
            repo.join("new.bin"),
            [0x89, b'P', b'N', b'G', 13, 10, 26, 10],
        )
        .unwrap_or_else(|e| panic!("write new.bin: {e}"));
        std::fs::remove_file(repo.join("old.bin")).unwrap_or_else(|e| panic!("remove: {e}"));
        std::fs::write(
            repo.join("b.bin"),
            [0xff, 0xfe, 0x00, b'B', b'I', b'N', b'2'],
        )
        .unwrap_or_else(|e| panic!("write b.bin: {e}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = repo.join("mode.sh");
            let mut perms = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("stat: {e}"))
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap_or_else(|e| panic!("chmod: {e}"));
        }
        rfc032_mv(&bin, &repo, "keep.txt", "moved.txt");
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "every kind")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        let row = rfc035_tip(&backend, &repo, &bin);
        assert_eq!(row.patches, 1, "0.{}: a block of one patch", bin.minor);
        let detail = stikk_core::block_detail(&backend, &repo, "heads/main", row.clone(), true)
            .unwrap_or_else(|e| panic!("0.{}: block detail: {e}", bin.minor));
        let rows = detail.patches.rows();
        let [patch_row] = rows else {
            panic!("0.{}: one patch listed, got {rows:?}", bin.minor)
        };
        assert_eq!(
            patch_row.message,
            stikk_core::PatchMessageState::Text("every kind".to_string()),
            "0.{}: joined by id from the block's log row",
            bin.minor
        );

        let view = stikk_core::patch_detail_in_block(
            &patch_row.patch,
            &row.block_id,
            &patch_row.message,
            rows.len(),
            Some(bin.minor),
        );
        assert!(
            view.heading[0].starts_with("sealed · in block"),
            "0.{}: {:?}",
            bin.minor,
            view.heading
        );

        // Every kind prikk reported is rendered, and every rendered value is prikk's own.
        let mut seen: Vec<&str> = Vec::new();
        for (operation, rendered) in patch_row.patch.operations.iter().zip(&view.operations) {
            seen.push(&operation.kind);
            assert!(
                stikk_core::flatten(&rendered.title).starts_with(&operation.kind),
                "0.{}: prikk's kind word leads the row: {rendered:?}",
                bin.minor
            );
            for path in &operation.paths {
                let stikk_prikk::QueuedPath::Path(path) = path else {
                    panic!(
                        "0.{}: a block of one resolves every path, got {:?}",
                        bin.minor, operation.paths
                    )
                };
                let whole = format!(
                    "{} {}",
                    stikk_core::flatten(&rendered.title),
                    rfc035_rows(rendered)
                );
                assert!(
                    whole.contains(path.as_str()),
                    "0.{}: prikk's path {path} is on screen: {whole}",
                    bin.minor
                );
            }
            let rendered_rows = rfc035_rows(rendered);
            match &operation.content {
                stikk_prikk::OperationContent::TextEdit {
                    old_span,
                    replacement,
                } => {
                    for span in [old_span, replacement] {
                        for line in span.lines() {
                            assert!(
                                rendered_rows.contains(line),
                                "0.{}: prikk's span {line:?} is on screen: {rendered_rows}",
                                bin.minor
                            );
                        }
                    }
                }
                stikk_prikk::OperationContent::FileCreated { content, mode } => {
                    assert!(
                        rendered_rows.contains(&format!("mode {mode:o}")),
                        "0.{}: prikk's mode as octal: {rendered_rows}",
                        bin.minor
                    );
                    rfc035_assert_file_content(&bin, content, &rendered_rows);
                }
                stikk_prikk::OperationContent::NodeDeleted { content, .. } => {
                    assert!(
                        rendered_rows.contains("removed content"),
                        "0.{}: the preimage is labelled as what was removed: {rendered_rows}",
                        bin.minor
                    );
                    rfc035_assert_file_content(&bin, content, &rendered_rows);
                }
                stikk_prikk::OperationContent::BinaryReplaced { old, new } => {
                    for blob in [old, new] {
                        assert!(
                            rendered_rows.contains(&blob.blob_id)
                                && rendered_rows.contains(&format!("{} byte(s)", blob.size)),
                            "0.{}: prikk's blob id and size: {rendered_rows}",
                            bin.minor
                        );
                    }
                }
                stikk_prikk::OperationContent::PermChanged { old_mode, new_mode } => assert!(
                    rendered_rows.contains(&format!("mode {old_mode:o} → {new_mode:o}")),
                    "0.{}: both modes, as octal: {rendered_rows}",
                    bin.minor
                ),
                stikk_prikk::OperationContent::PathRenamed { author_key_id, .. } => assert!(
                    rendered_rows.contains(author_key_id.as_str()),
                    "0.{}: the asserting AUTHOR key id: {rendered_rows}",
                    bin.minor
                ),
                stikk_prikk::OperationContent::Unrendered => panic!(
                    "0.{}: prikk emitted a kind stikk does not read: {}",
                    bin.minor, operation.kind
                ),
            }
            // The raw view carries prikk's own fields for the same operation, out of the same report.
            assert!(
                !rendered.raw.is_empty() && rendered.raw.join("\n").contains(&operation.kind),
                "0.{}: {rendered:?}",
                bin.minor
            );
        }
        seen.sort_unstable();
        // `create-file` and `delete-node` each appear twice now — once for a text file and once for a
        // binary one (review v1 §2.4) — so the comparison is over the set of kinds.
        seen.dedup();
        let mut want = vec![
            "edit-text",
            "create-file",
            "delete-node",
            "replace-binary",
            "rename-path",
        ];
        // Windows has no executable bit, so there is no mode change to make there.
        if cfg!(unix) {
            want.push("change-perm");
        }
        want.sort_unstable();
        assert_eq!(seen, want, "0.{}: every kind in one patch", bin.minor);
        // A block of one is never labelled: the block's end is this patch's end (decision 3a).
        assert!(
            view.operations
                .iter()
                .all(|o| !stikk_core::flatten(&o.title).contains(stikk_core::AS_AT_BLOCK_END)),
            "0.{}: {:?}",
            bin.minor,
            view.operations
        );
    }
}

/// **RFC 035 A1 F7, both directions, against a real binary** — the measurement this whole increment's
/// band exists for.
///
/// Two patches in **one** block. In the first repository patch 2 renames `a.txt` to `z.txt`, and patch
/// 1's `edit-text` is resolved **at the block's end**, so stikk shows `z.txt` *and* says where that
/// name comes from. In the second, patch 2 deletes the file patch 1 edited, and prikk resolves nothing
/// at all — so stikk says exactly that, with no reason attached.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc035_a_sealed_blocks_patch_is_resolved_at_the_blocks_end_at_both_ends() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc035_skip_below_the_band(&bin, "a block of two patches") {
            continue;
        }
        if bin.minor < 38 {
            eprintln!(
                "RFC 035: the rename direction SKIPPED at 0.{} — `prikk mv` does not exist below \
                 0.38. The delete direction below still runs. Announced rather than silent.",
                bin.minor
            );
        } else {
            // ---- direction one: a later rename gives patch 1's edit a name it never saw.
            let (fixture, backend, repo) = rfc032_sealed_base(&bin);
            rfc032_write(&repo, "a.txt", "alpha\nmore\n");
            fixture.set_author_env();
            backend
                .commit(&repo, "heads/main", "patch one")
                .unwrap_or_else(|e| panic!("0.{}: commit 1: {e}", bin.minor));
            Fixture::clear_env();
            rfc032_mv(&bin, &repo, "a.txt", "z.txt");
            rfc032_write(&repo, "z.txt", "alpha\nmore\nagain\n");
            fixture.set_author_env();
            backend
                .commit(&repo, "heads/main", "patch two")
                .unwrap_or_else(|e| panic!("0.{}: commit 2: {e}", bin.minor));
            fixture.set_maintainer_env();
            backend
                .seal(&repo, "heads/main")
                .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
            Fixture::clear_env();

            let row = rfc035_tip(&backend, &repo, &bin);
            assert_eq!(row.patches, 2, "0.{}: one block, two patches", bin.minor);
            let detail = stikk_core::block_detail(&backend, &repo, "heads/main", row.clone(), true)
                .unwrap_or_else(|e| panic!("0.{}: block detail: {e}", bin.minor));
            let rows = detail.patches.rows();
            assert_eq!(rows.len(), 2, "0.{}: both patches listed", bin.minor);
            let first = &rows[0];
            let view = stikk_core::patch_detail_in_block(
                &first.patch,
                &row.block_id,
                &first.message,
                rows.len(),
                Some(bin.minor),
            );
            let edit = first
                .patch
                .operations
                .iter()
                .find(|o| o.kind == "edit-text")
                .unwrap_or_else(|| panic!("0.{}: patch 1 edits: {:?}", bin.minor, first.patch));
            // **Whatever prikk resolved it to, stikk renders that** — and labels it only
            // where prikk resolved at the block's end (RFC 043 decision 3).
            match &edit.paths[..] {
                [stikk_prikk::QueuedPath::Path(path)] => {
                    let title = view
                        .operations
                        .iter()
                        .find(|o| stikk_core::flatten(&o.title).starts_with("edit-text"))
                        .map(|o| stikk_core::flatten(&o.title))
                        .unwrap_or_default();
                    if bin.minor >= stikk_prikk::PATCH_PATH_AT_PATCH_FROM_MINOR {
                        // **prikk 0.52 fixed this** (letter 020; RFC 043 F2): patch 1's path is
                        // the name patch 1 saw, so there is nothing left to caution about.
                        assert_eq!(
                            path, "a.txt",
                            "0.{}: the name this patch saw, not the later one's",
                            bin.minor
                        );
                        assert_eq!(
                            title, "edit-text a.txt",
                            "0.{}: resolved at the patch, so no label",
                            bin.minor
                        );
                    } else {
                        assert_eq!(
                            path, "z.txt",
                            "0.{}: measured at 0.46.0 and 0.51.0 — the later patch's name",
                            bin.minor
                        );
                        assert_eq!(
                            title,
                            format!("edit-text {path} — {}", stikk_core::AS_AT_BLOCK_END),
                            "0.{}: prikk's own name, labelled as the block's end",
                            bin.minor
                        );
                    }
                }
                other => panic!("0.{}: patch 1's edit names {other:?}", bin.minor),
            }
        }

        // ---- direction two: a later delete leaves nothing to resolve.
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        rfc032_write(&repo, "a.txt", "beta\n");
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "edit a")
            .unwrap_or_else(|e| panic!("0.{}: commit 1: {e}", bin.minor));
        Fixture::clear_env();
        std::fs::remove_file(repo.join("a.txt")).unwrap_or_else(|e| panic!("remove: {e}"));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "delete a")
            .unwrap_or_else(|e| panic!("0.{}: commit 2: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();

        let row = rfc035_tip(&backend, &repo, &bin);
        let detail = stikk_core::block_detail(&backend, &repo, "heads/main", row.clone(), true)
            .unwrap_or_else(|e| panic!("0.{}: block detail: {e}", bin.minor));
        let rows = detail.patches.rows();
        assert_eq!(rows.len(), 2, "0.{}: both patches listed", bin.minor);
        let first = &rows[0];
        let edit = first
            .patch
            .operations
            .iter()
            .find(|o| o.kind == "edit-text")
            .unwrap_or_else(|| panic!("0.{}: patch 1 edits: {:?}", bin.minor, first.patch));
        let view = stikk_core::patch_detail_in_block(
            &first.patch,
            &row.block_id,
            &first.message,
            rows.len(),
            Some(bin.minor),
        );
        if bin.minor >= stikk_prikk::PATCH_PATH_AT_PATCH_FROM_MINOR {
            // **0.52 names it** (RFC 043 F2): patch 1 edited `a.txt`, and a later delete no longer
            // takes that name away. Where there was one sentence and no path, there is a path.
            match &edit.paths[..] {
                [stikk_prikk::QueuedPath::Path(path)] => {
                    assert_eq!(path, "a.txt", "0.{}: the name this patch saw", bin.minor);
                }
                other => panic!("0.{}: patch 1's edit names {other:?}", bin.minor),
            }
            let shown = view
                .operations
                .iter()
                .find(|o| stikk_core::flatten(&o.title).starts_with("edit-text"))
                .unwrap_or_else(|| panic!("0.{}: {:?}", bin.minor, view.operations));
            assert_eq!(
                stikk_core::flatten(&shown.title),
                "edit-text a.txt",
                "0.{}: resolved at the patch, so no label",
                bin.minor
            );
        } else {
            assert!(
                matches!(edit.paths[..], [stikk_prikk::QueuedPath::UnresolvedNode(_)]),
                "0.{}: sealing removed the path prikk had already given: {:?}",
                bin.minor,
                edit.paths
            );
            let shown = view
                .operations
                .iter()
                .find(|o| stikk_core::flatten(&o.title) == "edit-text")
                .unwrap_or_else(|| panic!("0.{}: {:?}", bin.minor, view.operations));
            assert_eq!(
                shown.rows[0].text(),
                Some(stikk_core::NO_PATH_REPORTED),
                "0.{}: one sentence, and no reason — stikk cannot know which applies",
                bin.minor
            );
            assert!(
                !stikk_core::flatten(&shown.title).contains(stikk_core::AS_AT_BLOCK_END),
                "0.{}: there is no name to label: {shown:?}",
                bin.minor
            );
        }
    }
}

/// **RFC 035 §4.2 against a real binary: a queued patch, opened the way the Queue opens it** — one
/// `show <patch-id>` read, with the message the Queue already holds.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc035_a_queued_patch_opens_from_the_queue_at_both_ends() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc035_skip_below_the_band(&bin, "a queued patch's detail") {
            continue;
        }
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        rfc032_write(&repo, "a.txt", "alpha\nqueued\n");
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "a queued patch")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();

        // What the Queue view itself reads, and the patch it would open.
        let queue = backend
            .queue(&repo)
            .unwrap_or_else(|e| panic!("0.{}: queue: {e}", bin.minor));
        let stikk_prikk::QueueReport::Listed(queue) = queue else {
            panic!("0.{}: the queue is enumerated at this version", bin.minor)
        };
        let [queued] = queue.patches.as_slice() else {
            panic!("0.{}: one queued patch, got {:?}", bin.minor, queue.patches)
        };
        let message = stikk_core::PatchMessageState::from_queued(&queued.message);

        let view = stikk_core::patch_detail(&backend, &repo, &queued.patch_id, &message)
            .unwrap_or_else(|e| panic!("0.{}: patch detail: {e}", bin.minor));
        assert_eq!(view.patch_id, queued.patch_id);
        assert!(!view.unreported);
        assert_eq!(
            view.heading[0], "queued",
            "0.{}: prikk's own word, and no block to name",
            bin.minor
        );
        assert!(
            !view.operations.is_empty(),
            "0.{}: prikk reports this patch's operations",
            bin.minor
        );
        // Never labelled: a queued patch has no block to resolve against (decision 3a, F2).
        assert!(
            view.operations
                .iter()
                .all(|o| !stikk_core::flatten(&o.title).contains(stikk_core::AS_AT_BLOCK_END)),
            "0.{}: {:?}",
            bin.minor,
            view.operations
        );
        // And what stikk shows is prikk's own span text for the edit it made.
        let rendered = view
            .operations
            .iter()
            .map(rfc035_rows)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            rendered.contains("queued"),
            "0.{}: prikk's replacement text: {rendered}",
            bin.minor
        );
    }
}

/// **RFC 036 §8: the names prikk really reports, rendered by the rules, at both ends.**
///
/// §1 measured prikk 0.51: `worktree-status --format json` reports an on-disk filename carrying a bidi
/// override, a zero-width space, a non-breaking space or a tab **with its exact characters**, under
/// `unsupported-path`; and a double space, a leading space and a `"` are ordinary `untracked` entries
/// that a commit would author. This drives those names through the real binary and the real view, and
/// requires that **nothing that does not show as itself reaches a cell** and that **no two of them
/// render alike**.
///
/// **A filesystem that refuses a name skips that name, announced** (RFC 022 §3) — Windows refuses most
/// of them, and a skipped name is not a failure.
///
/// **What this leg proves that a unit test cannot**: that prikk really hands stikk these bytes. The
/// rendering at 80 columns is covered by the view's own tests, which can reach the private renderer.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc036_names_prikk_reports_render_by_the_rules_at_both_ends() {
    // Written as escapes so this source file holds no invisible character of its own.
    const NAMES: [(&str, &str); 6] = [
        ("bidi override", "evil\u{202e}gnp.txt"),
        ("zero-width space", "a\u{200b}b.txt"),
        ("non-breaking space", "a\u{a0}b.txt"),
        ("double space", "a  b.txt"),
        ("leading space", " lead.txt"),
        ("double quote", "say\"hi.txt"),
    ];
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let (_fixture, backend, repo) = rfc032_sealed_base(&bin);
        let mut created: Vec<&str> = Vec::new();
        for (what, name) in NAMES {
            match std::fs::write(repo.join(name), "x\n") {
                Ok(()) => created.push(name),
                Err(e) => println!(
                    "RFC 036: 0.{} SKIPPED {what} — this filesystem refuses the name ({e}). \
                     Announced rather than silent (RFC 022 §3).",
                    bin.minor
                ),
            }
        }
        if created.is_empty() {
            continue;
        }
        let view = stikk_core::changes_view(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: changes: {e}", bin.minor))
            .view;
        // **What prikk reports is prikk's, and it differs by version** — measured here rather than
        // assumed. At 0.51 every name comes back relative and byte-for-byte. At 0.28 the three
        // non-ASCII ones come back as **absolute** paths (the shape RFC 029 A recorded for 0.39–0.41
        // and prikk made relative at 0.42), and ` lead.txt` comes back as `lead.txt` — prikk 0.28
        // drops the leading space. stikk renders what it is given either way, so the leg asserts
        // **stikk's** claim, not prikk's shape, and prints prikk's shape so a reader can see it.
        let listed: Vec<&stikk_core::ChangeEntry> = view
            .entries
            .iter()
            .filter(|e| e.path != "a.txt" && e.path != "keep.txt")
            .collect();
        println!(
            "RFC 036: 0.{} prikk reported {:?}",
            bin.minor,
            listed.iter().map(|e| &e.path).collect::<Vec<_>>()
        );
        assert_eq!(
            listed.len(),
            created.len(),
            "0.{}: one entry per created name, whatever shape prikk gives it",
            bin.minor
        );
        let mut rendered: Vec<String> = listed
            .iter()
            .map(|e| stikk_tui::text::identifier(&e.path))
            .collect();
        rendered.sort();
        let distinct = {
            let mut copy = rendered.clone();
            copy.dedup();
            copy.len()
        };
        assert_eq!(
            distinct,
            rendered.len(),
            "0.{}: two names render alike: {rendered:?}",
            bin.minor
        );
        for value in &rendered {
            for ch in ['\u{202e}', '\u{200b}', '\u{a0}', '\t'] {
                assert!(
                    !value.contains(ch),
                    "0.{}: U+{:04X} survives in {value}",
                    bin.minor,
                    u32::from(ch)
                );
            }
        }
        println!("RFC 036: 0.{} rendered {rendered:?}", bin.minor);
        // And prikk's own verdict on each is carried, so what the view says about them is prikk's.
        for entry in &listed {
            assert!(
                matches!(
                    entry.authoring,
                    stikk_prikk::Authoring::Authored
                        | stikk_prikk::Authoring::Refused(_)
                        | stikk_prikk::Authoring::Unreported
                ),
                "0.{}: {:?}",
                bin.minor,
                entry.authoring
            );
        }
    }
}

/// **RFC 035 review v2 §2: a real file whose content does not show as itself.**
///
/// prikk stores the bytes; stikk must show them and must not let a terminal act on them. The file
/// committed here carries a bidi override ("Trojan Source", CVE-2021-42574), a zero-width space, a
/// non-breaking space, a tab and a line of trailing whitespace — the four families the escape table
/// was widened for. **Every one of them is checked twice**: once in the rendered rows and once in the
/// raw view of prikk's own fields, because they used to share nothing but a coincidence.
///
/// Done against real binaries rather than only in a unit test because the claim spans prikk's writer
/// and stikk's reader: a prikk that normalised or refused any of these would make stikk's escaping
/// moot, and the only way to know is to ask one.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc035_content_that_does_not_show_as_itself_is_escaped_at_both_ends() {
    // Written as escapes so this source file holds no override of its own — `rustc` denies one in a
    // literal, which is the same lesson one layer down.
    const OVERRIDE: char = '\u{202e}';
    const ZERO_WIDTH: char = '\u{200b}';
    const NO_BREAK: char = '\u{a0}';
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        if rfc035_skip_below_the_band(&bin, "content that does not show as itself") {
            continue;
        }
        let body = format!(
            "if level != \"user{OVERRIDE} // admin\" {{\n\tlet a{ZERO_WIDTH}b = 1;\nx{NO_BREAK}y   \n}}\n"
        );
        let (fixture, backend, repo) = rfc032_sealed_base(&bin);
        rfc032_write(&repo, "auth.rs", &body);
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "content that lies about itself")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();

        let queue = backend
            .queue(&repo)
            .unwrap_or_else(|e| panic!("0.{}: queue: {e}", bin.minor));
        let stikk_prikk::QueueReport::Listed(queue) = queue else {
            panic!("0.{}: the queue is enumerated at this version", bin.minor)
        };
        let [queued] = queue.patches.as_slice() else {
            panic!("0.{}: one queued patch, got {:?}", bin.minor, queue.patches)
        };
        let view = stikk_core::patch_detail(
            &backend,
            &repo,
            &queued.patch_id,
            &stikk_core::PatchMessageState::from_queued(&queued.message),
        )
        .unwrap_or_else(|e| panic!("0.{}: patch detail: {e}", bin.minor));

        let rendered: String = view.operations.iter().map(rfc035_rows).collect();
        let raw: String = view
            .operations
            .iter()
            .flat_map(|operation| operation.raw.iter().cloned())
            .collect();
        assert!(
            !rendered.is_empty(),
            "0.{}: prikk reported this patch's content",
            bin.minor
        );
        for (what, ch) in [
            ("a bidi override", OVERRIDE),
            ("a zero-width space", ZERO_WIDTH),
            ("a non-breaking space", NO_BREAK),
            ("a tab", '\t'),
        ] {
            for (where_, text) in [("the rendered rows", &rendered), ("the raw view", &raw)] {
                assert!(
                    !text.contains(ch),
                    "0.{}: {what} (U+{:04X}) reached {where_} as itself",
                    bin.minor,
                    u32::from(ch)
                );
            }
        }
        for escape in ["\\u202e", "\\u200b", "\\u00a0", "\\t"] {
            assert!(
                rendered.contains(escape),
                "0.{}: {escape} is how the rendered rows report it: {rendered}",
                bin.minor
            );
            assert!(
                raw.contains(escape),
                "0.{}: and the raw view spells it the same way: {raw}",
                bin.minor
            );
        }
        // The trailing whitespace prikk stored is still there, inside the quotes.
        assert!(
            rendered.contains("y   "),
            "0.{}: trailing whitespace survives: {rendered}",
            bin.minor
        );
    }
}

/// **RFC 035 §7 at the floor: Block detail's below-0.39 sentence, and the Queue unchanged.**
///
/// At stikk's JSON floor there is no patch content to read, and stikk says so **in its own voice** —
/// never *"prikk does not report it"*, which is false at prikk 0.36–0.38 (`C-T4d`). Above the floor the
/// same block lists its patches instead, which is what makes this leg about the band rather than about
/// one binary.
#[test]
#[ignore = "requires two real prikk binaries; see the module docs"]
fn rfc035_below_the_json_floor_the_patches_section_names_stikks_own_limit() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let (_fixture, backend, repo) = rfc032_sealed_base(&bin);
        let row = rfc035_tip(&backend, &repo, &bin);
        let detail = stikk_core::block_detail(&backend, &repo, "heads/main", row, true)
            .unwrap_or_else(|e| panic!("0.{}: block detail: {e}", bin.minor));
        if bin.minor < RFC035_CONTENT_FROM {
            assert_eq!(
                detail.patches,
                stikk_core::BlockPatches::Unreported(bin.minor),
                "0.{}: unreported, never an empty list",
                bin.minor
            );
            let reason = detail
                .patches
                .unreported_reason()
                .unwrap_or_else(|| panic!("0.{}: a reason", bin.minor));
            assert_eq!(
                reason,
                format!(
                    "stikk reads a patch's content from prikk 0.{RFC035_CONTENT_FROM} on; this is \
                     0.{}.",
                    bin.minor
                )
            );
            assert!(
                !reason.contains("prikk does not report"),
                "0.{}: {reason}",
                bin.minor
            );
        } else {
            assert_eq!(detail.patches.unreported_reason(), None);
            assert_eq!(
                detail.patches.rows().len(),
                1,
                "0.{}: the base block's one patch",
                bin.minor
            );
        }
        // The Queue is unchanged at either end: still the count and target it always reported.
        let queue = stikk_core::queue_view(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: queue view: {e}", bin.minor));
        assert_eq!(
            queue.heading, "Nothing is queued.",
            "0.{}: {queue:?}",
            bin.minor
        );
    }
}

/// **RFC 037 Handoff A: Block detail lists the files at the block it was opened on, not the tip's.**
///
/// The claim spans prikk's `tree` and stikk's reader, and it is the whole increment: a two-block
/// lineage whose first block holds a file its successor deletes. Block detail for the **older**
/// block must list that file and not the tip's, and the ids must match the block asked about.
///
/// At the floor — below `TREE_FROM_MINOR`, where prikk has no `tree` at all — the same older block
/// must say so in stikk's own voice rather than show an empty list (`C-T2c′`), and the tip must
/// still list its files from the prose replay.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc037_block_detail_lists_the_files_at_the_block_it_was_opened_on() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();

        // Block one: `only-in-the-first.txt`.
        std::fs::write(repo.join("only-in-the-first.txt"), "first\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "one")
            .unwrap_or_else(|e| panic!("0.{}: commit one: {e}", bin.minor));
        fixture.set_maintainer_env();
        let first = backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal one: {e}", bin.minor));
        Fixture::clear_env();

        // Block two: delete it, add another.
        std::fs::remove_file(repo.join("only-in-the-first.txt"))
            .unwrap_or_else(|e| panic!("0.{}: remove: {e}", bin.minor));
        std::fs::write(repo.join("only-in-the-tip.txt"), "second\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "two")
            .unwrap_or_else(|e| panic!("0.{}: commit two: {e}", bin.minor));
        fixture.set_maintainer_env();
        let tip = backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal two: {e}", bin.minor));
        Fixture::clear_env();

        let history = stikk_core::history_view(&backend, &repo, "heads/main", 10)
            .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor));
        assert_eq!(history.blocks.len(), 2, "0.{}: two blocks", bin.minor);
        let tip_row = history.blocks[0].clone();
        let older_row = history.blocks[1].clone();
        assert_eq!(tip_row.block_id, tip.block_id, "0.{}", bin.minor);
        assert_eq!(older_row.block_id, first.block_id, "0.{}", bin.minor);

        let older = stikk_core::block_detail(&backend, &repo, "heads/main", older_row, false)
            .unwrap_or_else(|e| panic!("0.{}: older block detail: {e}", bin.minor));
        let at_tip = stikk_core::block_detail(&backend, &repo, "heads/main", tip_row, true)
            .unwrap_or_else(|e| panic!("0.{}: tip block detail: {e}", bin.minor));

        // The tip answers at every supported version — from `tree` above the band, from the prose
        // replay below it.
        let tip_state = at_tip
            .state
            .listed()
            .unwrap_or_else(|| panic!("0.{}: the tip's files are always listed", bin.minor));
        // Membership, not an exact set: the fixture ships its own files, and this leg's claim is
        // about *which block* answers, not about what else the repository holds.
        let tip_paths: Vec<&str> = tip_state.files.iter().map(|f| f.path.as_str()).collect();
        assert!(
            tip_paths.contains(&"only-in-the-tip.txt")
                && !tip_paths.contains(&"only-in-the-first.txt"),
            "0.{}: the tip lists the tip's files, got {tip_paths:?}",
            bin.minor
        );

        if bin.minor >= stikk_prikk::TREE_FROM_MINOR {
            let state = older.state.listed().unwrap_or_else(|| {
                panic!(
                    "0.{}: at or above 0.{} every block lists its files",
                    bin.minor,
                    stikk_prikk::TREE_FROM_MINOR
                )
            });
            assert_eq!(
                state.target_block, first.block_id,
                "0.{}: the listing is about the block stikk asked for",
                bin.minor
            );
            let paths: Vec<&str> = state.files.iter().map(|f| f.path.as_str()).collect();
            assert!(
                paths.contains(&"only-in-the-first.txt") && !paths.contains(&"only-in-the-tip.txt"),
                "0.{}: the older block holds the file the tip deleted and not the tip's, \
                 got {paths:?}",
                bin.minor
            );
            // **Each file carries its mode**, and it is prikk's own integer. The value itself is
            // not asserted: prikk records no executable bit on Windows, so a mode pinned here would
            // fail on one platform for a reason that is prikk's design, not a defect (prikk's 0.46
            // changelog).
            for file in &state.files {
                assert!(
                    file.mode.is_some(),
                    "0.{}: {} carries no mode",
                    bin.minor,
                    file.path
                );
                assert_eq!(file.kind.as_deref(), Some("file"), "0.{}", bin.minor);
            }
            assert!(
                state.total_bytes > 0,
                "0.{}: files replayed to zero bytes",
                bin.minor
            );
            assert!(
                older.state.unreported_reason().is_none(),
                "0.{}: nothing is said instead of a list that exists",
                bin.minor
            );
            println!(
                "RFC 037: 0.{} older block {} lists {:?}, total {} byte(s), modes {:?}",
                bin.minor,
                &state.target_block[..12],
                paths,
                state.total_bytes,
                state
                    .files
                    .iter()
                    .map(|f| f.mode.map(|m| format!("{m:06o}")))
                    .collect::<Vec<_>>()
            );
        } else {
            assert!(
                older.state.listed().is_none(),
                "0.{}: below 0.{} prikk replays only to a tip",
                bin.minor,
                stikk_prikk::TREE_FROM_MINOR
            );
            let reason = older
                .state
                .unreported_reason()
                .unwrap_or_else(|| panic!("0.{}: a sentence instead of an empty list", bin.minor));
            assert!(
                reason.contains("replays only to the ref tip"),
                "0.{}: {reason}",
                bin.minor
            );
            println!("RFC 037: 0.{} older block says — {reason}", bin.minor);
        }

        // **The prose band's files carry no mode, and that is said, not shown as zero.**
        if bin.minor < stikk_prikk::TREE_FROM_MINOR {
            for file in &tip_state.files {
                assert_eq!(
                    file.mode, None,
                    "0.{}: the prose replay reports no mode for {}",
                    bin.minor, file.path
                );
            }
        }
    }
}

/// **RFC 037 Handoff B: a fresh repository's picker offers the branch you are on.**
///
/// The claim spans prikk's `branch list` and stikk's reader, and the state it is about is the one
/// every repository starts in: nothing sealed, so nothing published, so — until prikk 0.47 — no ref
/// the picker could offer at all.
///
/// At the ceiling the branch must be named, must **not** be in the ref list (RFC 032's *published*
/// test, F4), and must keep being named after a commit is queued and stop being named after the
/// first seal. At the floor there is no such row, and that is *not reported*, not "none".
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc037_the_picker_offers_the_unpublished_current_branch_on_a_fresh_repository() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let reports = bin.minor >= stikk_prikk::UNPUBLISHED_BRANCH_FROM_MINOR;

        let fresh = stikk_core::list_refs(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: list_refs on a fresh repository: {e}", bin.minor));
        assert!(
            fresh.refs.is_empty(),
            "0.{}: nothing is published yet, got {:?}",
            bin.minor,
            fresh.refs
        );

        if !reports {
            assert_eq!(
                fresh.unpublished_current,
                stikk_prikk::UnpublishedBranch::NotReported,
                "0.{}: below 0.{} prikk cannot say, which is not 'none'",
                bin.minor,
                stikk_prikk::UNPUBLISHED_BRANCH_FROM_MINOR
            );
            println!(
                "RFC 037: 0.{} reports no unpublished current branch at all — the picker is as it was",
                bin.minor
            );
            continue;
        }

        assert_eq!(
            fresh.unpublished_current.name(),
            Some("heads/main"),
            "0.{}: prikk names the branch you are on",
            bin.minor
        );

        // **Changes still says the ref has no published history**, with the field present — RFC 032
        // decision 5 reads membership in the list, and the branch is beside it, never in it (F4).
        let read = stikk_core::changes_view(&backend, &repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: changes: {e}", bin.minor));
        let headline = read.history.changes_headline("heads/main", read.view.clean);
        assert!(
            headline
                .as_deref()
                .is_some_and(|line| line.contains("no published history")),
            "0.{}: RFC 032's sentence is unchanged, got {headline:?}",
            bin.minor
        );

        // History on that ref opens, with its queued tier and no sealed blocks.
        let history = stikk_core::history_view(&backend, &repo, "heads/main", 10)
            .unwrap_or_else(|e| panic!("0.{}: history on an unpublished ref: {e}", bin.minor));
        assert!(
            history.blocks.is_empty(),
            "0.{}: nothing is sealed, got {} block(s)",
            bin.minor,
            history.blocks.len()
        );

        // Still named with a commit queued and nothing sealed.
        std::fs::write(repo.join("first.txt"), "first\n")
            .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "one")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        Fixture::clear_env();
        let queued = stikk_core::list_refs(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: list_refs with a queued commit: {e}", bin.minor));
        assert_eq!(
            queued.unpublished_current.name(),
            Some("heads/main"),
            "0.{}: a queued commit publishes nothing",
            bin.minor
        );
        assert!(queued.refs.is_empty(), "0.{}", bin.minor);

        // And the first seal publishes it: into the list, out of the field.
        fixture.set_maintainer_env();
        backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
        let sealed = stikk_core::list_refs(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: list_refs after the seal: {e}", bin.minor));
        assert_eq!(
            sealed.unpublished_current,
            stikk_prikk::UnpublishedBranch::None,
            "0.{}: prikk answers 'none' once it is published",
            bin.minor
        );
        assert!(
            sealed.refs.iter().any(|r| r.name == "heads/main"),
            "0.{}: and it is in the list instead, got {:?}",
            bin.minor,
            sealed.refs
        );
        println!(
            "RFC 037: 0.{} fresh={:?} queued={:?} sealed={:?} (list after the seal: {:?})",
            bin.minor,
            fresh.unpublished_current.name(),
            queued.unpublished_current.name(),
            sealed.unpublished_current.name(),
            sealed.refs.iter().map(|r| &r.name).collect::<Vec<_>>()
        );
    }
}

/// **RFC 038 commit 1: `prikk diff` reads at the ceiling and is unreported at the floor.**
///
/// The seam's own leg: every status F1 measured, from a real binary, plus the bare `diff`'s
/// worktree side and its queue. The view-model's legs are separate; this one is about the reader.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc038_diff_reads_every_measured_status_at_the_ceiling() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let write = |name: &str, body: &str| {
            std::fs::write(repo.join(name), body)
                .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
        };
        let seal = |message: &str| {
            fixture.set_author_env();
            backend
                .commit(&repo, "heads/main", message)
                .unwrap_or_else(|e| panic!("0.{}: commit {message}: {e}", bin.minor));
            fixture.set_maintainer_env();
            let sealed = backend
                .seal(&repo, "heads/main")
                .unwrap_or_else(|e| panic!("0.{}: seal {message}: {e}", bin.minor));
            Fixture::clear_env();
            sealed
        };

        write("text.txt", "one\ntwo\nthree\n");
        write("ws.txt", "keep\n");
        write("del.txt", "gone\n");
        let base = seal("base");
        write("text.txt", "one\nTWO\nthree\n");
        write("ws.txt", "keep  \n");
        std::fs::remove_file(repo.join("del.txt"))
            .unwrap_or_else(|e| panic!("0.{}: remove: {e}", bin.minor));
        write("added.txt", "new\n");
        let tip = seal("second");

        let comparison = stikk_prikk::Comparison::Points {
            from: base.block_id.clone(),
            to: tip.block_id.clone(),
        };
        let report = backend
            .diff(&repo, &comparison)
            .unwrap_or_else(|e| panic!("0.{}: diff: {e}", bin.minor));

        if bin.minor < stikk_prikk::DIFF_FROM_MINOR {
            assert_eq!(
                report,
                stikk_prikk::DiffReport::Unreported,
                "0.{}: below 0.{} stikk spawns nothing and reports nothing",
                bin.minor,
                stikk_prikk::DIFF_FROM_MINOR
            );
            println!(
                "RFC 038: 0.{} does not compare — unreported, never an empty comparison",
                bin.minor
            );
            continue;
        }

        let stikk_prikk::DiffReport::Listed(d) = report else {
            panic!("0.{}: at the ceiling the comparison is listed", bin.minor)
        };
        assert_eq!(
            d.from.target,
            stikk_prikk::DiffTarget::Block(base.block_id.clone()),
            "0.{}: the left side is the block stikk asked for",
            bin.minor
        );
        assert_eq!(
            d.to.target,
            stikk_prikk::DiffTarget::Block(tip.block_id.clone()),
            "0.{}",
            bin.minor
        );
        let mut got: Vec<(&str, &str)> = d
            .entries
            .iter()
            .map(|e| (e.status.as_str(), e.path.as_str()))
            .collect();
        got.sort_unstable();
        assert_eq!(
            got,
            [
                ("added", "added.txt"),
                ("deleted", "del.txt"),
                ("modified", "text.txt"),
                ("modified", "ws.txt"),
            ],
            "0.{}",
            bin.minor
        );
        // **The whitespace-only edit keeps its trailing spaces** all the way through the seam —
        // the case a renderer that collapsed whitespace would show as no change at all.
        let ws = d
            .entries
            .iter()
            .find(|e| e.path == "ws.txt")
            .unwrap_or_else(|| panic!("0.{}: no ws.txt entry", bin.minor));
        assert_eq!(
            ws.hunks,
            ["@@ -1 +1 @@\n-keep\n+keep  \n"],
            "0.{}: prikk's hunk, byte for byte",
            bin.minor
        );

        // The bare `diff`: the worktree against the tip plus the queue.
        write("queued.txt", "q\n");
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "queued")
            .unwrap_or_else(|e| panic!("0.{}: queued commit: {e}", bin.minor));
        Fixture::clear_env();
        write("wt.txt", "wt\n");
        let stikk_prikk::DiffReport::Listed(bare) = backend
            .diff(&repo, &stikk_prikk::Comparison::WorktreeAgainstBaseline)
            .unwrap_or_else(|e| panic!("0.{}: bare diff: {e}", bin.minor))
        else {
            panic!("0.{}: listed", bin.minor)
        };
        assert_eq!(
            bare.to.target,
            stikk_prikk::DiffTarget::Worktree,
            "0.{}",
            bin.minor
        );
        assert_eq!(
            bare.from.queued,
            stikk_prikk::QueuedOnSide::Queued(1),
            "0.{}: the left side folds the queue in",
            bin.minor
        );
        assert_eq!(
            bare.to.queued,
            stikk_prikk::QueuedOnSide::NotApplicable,
            "0.{}",
            bin.minor
        );
        println!(
            "RFC 038: 0.{} two blocks -> {got:?}; bare diff left={:?} queued={:?} right={:?}",
            bin.minor, bare.from.point, bare.from.queued, bare.to.target
        );
    }
}

/// **RFC 038 commit 2: the three comparisons the entry points make, against real binaries.**
///
/// History's marked pair, Block detail's previous-in-lineage, and Changes' worktree form — each
/// built here exactly as the key that opens it builds it, so the view-model is driven by a real
/// report rather than a stand-in. At the floor, all three are unreported.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc038_the_three_comparisons_read_against_real_binaries() {
    for bin in [PrikkBin::floor(), PrikkBin::ceiling()] {
        let fixture = Fixture::build(&bin);
        let backend = CliBackend::with_program(&bin.path);
        let repo = fixture.repo().to_path_buf();
        let write = |name: &str, body: &str| {
            std::fs::write(repo.join(name), body)
                .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
        };
        let seal = |message: &str| {
            fixture.set_author_env();
            backend
                .commit(&repo, "heads/main", message)
                .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
            fixture.set_maintainer_env();
            let sealed = backend
                .seal(&repo, "heads/main")
                .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
            Fixture::clear_env();
            sealed
        };

        // A whitespace-only edit across three blocks, so every comparison has something to show
        // and the one it shows is the case a whitespace-blind renderer would lose.
        write("ws.txt", "keep\n");
        let first = seal("one");
        write("ws.txt", "keep  \n");
        let second = seal("two");
        write("ws.txt", "keep   \n");
        let third = seal("three");

        let marked_pair = stikk_core::compare(
            &backend,
            &repo,
            &stikk_prikk::Comparison::Points {
                from: first.block_id.clone(),
                to: third.block_id.clone(),
            },
        )
        .unwrap_or_else(|e| panic!("0.{}: marked pair: {e}", bin.minor));
        let previous = stikk_core::compare(
            &backend,
            &repo,
            &stikk_prikk::Comparison::Points {
                from: second.block_id.clone(),
                to: third.block_id.clone(),
            },
        )
        .unwrap_or_else(|e| panic!("0.{}: previous in lineage: {e}", bin.minor));

        // Changes' form, with a commit queued so the left side folds one in.
        write("queued.txt", "q\n");
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "queued")
            .unwrap_or_else(|e| panic!("0.{}: queued commit: {e}", bin.minor));
        Fixture::clear_env();
        write("wt.txt", "wt\n");
        let worktree = stikk_core::compare(
            &backend,
            &repo,
            &stikk_prikk::Comparison::WorktreeAgainstBaseline,
        )
        .unwrap_or_else(|e| panic!("0.{}: worktree: {e}", bin.minor));

        if bin.minor < stikk_core::COMPARE_FROM_MINOR {
            for (what, view) in [
                ("marked pair", &marked_pair),
                ("previous", &previous),
                ("worktree", &worktree),
            ] {
                assert!(
                    view.listed().is_none(),
                    "0.{}: {what} is unreported below the band",
                    bin.minor
                );
                assert!(
                    view.unreported_reason()
                        .is_some_and(|r| r.contains("stikk compares from prikk 0.46 on")),
                    "0.{}: {what}",
                    bin.minor
                );
            }
            println!(
                "RFC 038: 0.{} — all three comparisons unreported, in stikk's own words",
                bin.minor
            );
            continue;
        }

        let pair = marked_pair
            .listed()
            .unwrap_or_else(|| panic!("0.{}: the pair is listed", bin.minor));
        let ws = pair
            .entries
            .iter()
            .find(|e| e.status == "modified")
            .unwrap_or_else(|| panic!("0.{}: no modified entry: {:?}", bin.minor, pair.entries));
        // **The whitespace-only edit survives all the way to the rows** — quoted, with the
        // terminator inside the quotes, so `keep` and `keep   ` cannot render alike.
        let content: Vec<&str> = ws
            .rows
            .iter()
            .filter_map(|r| match r {
                stikk_core::HunkRow::Line { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            content,
            ["\"keep\\n\"", "\"keep   \\n\""],
            "0.{}: prikk's bytes, quoted",
            bin.minor
        );

        let prev = previous
            .listed()
            .unwrap_or_else(|| panic!("0.{}: previous is listed", bin.minor));
        assert!(!prev.is_empty(), "0.{}", bin.minor);

        let wt = worktree
            .listed()
            .unwrap_or_else(|| panic!("0.{}: worktree is listed", bin.minor));
        assert_eq!(
            wt.to.target,
            stikk_prikk::DiffTarget::Worktree,
            "0.{}",
            bin.minor
        );
        assert_eq!(
            wt.from.queued,
            stikk_prikk::QueuedOnSide::Queued(1),
            "0.{}: the left side folds the queue in",
            bin.minor
        );
        let heading = stikk_core::flatten(&wt.heading());
        assert!(
            heading.contains("plus 1 queued commit(s) not yet sealed")
                && heading.contains("the worktree"),
            "0.{}: {heading}",
            bin.minor
        );

        // **Two identical points are "no differences", and that is prikk's answer, not a
        // special case** — the same code path as every other comparison.
        let same = stikk_core::compare(
            &backend,
            &repo,
            &stikk_prikk::Comparison::Points {
                from: third.block_id.clone(),
                to: third.block_id.clone(),
            },
        )
        .unwrap_or_else(|e| panic!("0.{}: same block twice: {e}", bin.minor));
        assert!(
            same.listed().is_some_and(stikk_core::Compared::is_empty),
            "0.{}: a block compared with itself",
            bin.minor
        );

        println!(
            "RFC 038: 0.{} pair={:?} previous={} entries worktree heading={heading:?}",
            bin.minor,
            content,
            prev.entries.len()
        );
    }
}

/// **RFC 040: a real repository, sound, then with a byte flipped — Verify reports the problem,
/// never a refusal.**
///
/// This is the one behaviour the whole increment turns on (F2). prikk exits **1** on a damaged
/// repository and still prints a complete report; stikk's seam is the only place that reads a
/// non-zero exit as success, and if it ever stopped doing so, Verify would say *prikk refused* at
/// exactly the moment prikk is reporting damage. No fake binary can prove that — only a real
/// prikk and real damage.
///
/// The damage is the one `measure040b.sh` makes: a byte flipped inside a blob container, which
/// prikk reports as `item-failure`.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc040_verify_reports_real_damage_as_a_verdict_never_as_a_refusal() {
    let bin = PrikkBin::ceiling();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    std::fs::write(repo.join("a.txt"), "one\n")
        .unwrap_or_else(|e| panic!("0.{}: write a.txt: {e}", bin.minor));
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/main", "one")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    // **Sound first**, so the damaged run below is a change and not just an assertion.
    let sound = stikk_core::verify(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: verify on a sound repository: {e}", bin.minor));
    let sound = sound
        .checked()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert!(
        sound.ok,
        "0.{}: a freshly sealed repository must verify sound: {:?}",
        bin.minor, sound.conditions
    );
    assert_eq!(sound.verdict(), "prikk reports this repository sound");
    // prikk's own stage list, whatever its length -- never one stikk wrote down.
    assert!(
        !sound.stages.is_empty(),
        "0.{}: prikk reported no stages at all",
        bin.minor
    );
    assert!(
        sound.stages.iter().all(|s| s.is_evaluated()),
        "0.{}: {:?}",
        bin.minor,
        sound.stages
    );

    // **Now flip one byte inside a blob container**, exactly as `measure040b.sh` does.
    let container = repo.join(".prikk/containers/blob/a.container");
    let mut bytes = std::fs::read(&container)
        .unwrap_or_else(|e| panic!("0.{}: read {}: {e}", bin.minor, container.display()));
    assert!(
        !bytes.is_empty(),
        "0.{}: the blob container is empty; the layout has moved and this test is measuring \
         nothing",
        bin.minor
    );
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xff;
    std::fs::write(&container, &bytes)
        .unwrap_or_else(|e| panic!("0.{}: write {}: {e}", bin.minor, container.display()));

    // **The whole point.** `verify` now exits 1. It must come back as a verdict.
    let damaged = match stikk_core::verify(&backend, &repo) {
        Ok(view) => view,
        Err(error) => panic!(
            "0.{}: a damaged repository came back as a REFUSAL, which is the one thing Verify \
             must never say (RFC 040 F2): {error}",
            bin.minor
        ),
    };
    let damaged = damaged
        .checked()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert!(
        !damaged.ok,
        "0.{}: prikk must not call a damaged repository sound",
        bin.minor
    );
    assert!(
        !damaged.conditions.is_empty(),
        "0.{}: a false verdict with no condition beside it",
        bin.minor
    );
    // prikk's own id and words, verbatim -- not stikk's summary of them.
    let ids: Vec<&str> = damaged.conditions.iter().map(|c| c.id.as_str()).collect();
    assert!(
        ids.contains(&"item-failure"),
        "0.{}: prikk's measured condition for this damage is `item-failure`, got {ids:?}",
        bin.minor
    );
    assert_eq!(damaged.verdict(), "prikk reports 1 problem(s)");
    // And every stage still reported, as measured (F2): the verdict carries the signal.
    assert!(
        damaged.stages.iter().all(|s| s.is_evaluated()),
        "0.{}: {:?}",
        bin.minor,
        damaged.stages
    );
}

/// **RFC 040: below the band, stikk says it did not check — never that the repository is sound.**
///
/// The mirror of the test above. At 0.28 nothing is spawned, and the view's sentence is stikk's
/// own (`C-T4d`).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc040_below_the_band_verify_says_stikk_did_not_check() {
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);

    let view = stikk_core::verify(&backend, fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: below the band must not be an error: {e}", bin.minor));
    assert!(
        view.checked().is_none(),
        "0.{}: nothing should have been read",
        bin.minor
    );
    let reason = view
        .unreported_reason()
        .unwrap_or_else(|| panic!("0.{}: a reason in stikk's own voice", bin.minor));
    assert!(
        reason.contains("from prikk 0.39 on"),
        "0.{}: {reason}",
        bin.minor
    );
    assert!(
        reason.contains(&format!("this is 0.{}", bin.minor)),
        "0.{}: {reason}",
        bin.minor
    );
    // **Never a verdict.** A repository nobody checked is not a repository found sound.
    assert!(!reason.contains("sound"), "0.{}: {reason}", bin.minor);
}

/// **RFC 041: two diverged refs compared, against real binaries.**
///
/// The comparison a branch user asks first, driven the way the picker drives it:
/// `Comparison::Points` with two **ref names**, not block ids. Measured first at 0.51
/// (`.git-exclude/reports/041-compare-two-refs/measure041-0.51.txt`); this holds the product to
/// what was measured.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc041_two_diverged_refs_compare_through_their_names_at_the_ceiling() {
    let bin = PrikkBin::ceiling();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    let run = |args: &[&str]| {
        let out = std::process::Command::new(&bin.path)
            .args(args)
            .current_dir(&repo)
            .output()
            .unwrap_or_else(|e| panic!("0.{}: spawn prikk {args:?}: {e}", bin.minor));
        assert!(
            out.status.success(),
            "0.{}: prikk {args:?} failed: {}",
            bin.minor,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    };
    let seal = |body: &str, path: &str, reff: &str| {
        std::fs::write(repo.join(path), body)
            .unwrap_or_else(|e| panic!("0.{}: write {path}: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, reff, "x")
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
    };

    // A common base, then one block on each branch: a real divergence.
    seal("base\n", "common.txt", "heads/main");
    fixture.set_maintainer_env();
    run(&["branch", "create", "heads/feature", "--from", "heads/main"]);
    run(&["branch", "switch", "heads/feature"]);
    Fixture::clear_env();
    seal("only on feature\n", "feature.txt", "heads/feature");
    fixture.set_maintainer_env();
    run(&["branch", "switch", "heads/main"]);
    Fixture::clear_env();
    seal("only on main\n", "main.txt", "heads/main");
    fixture.set_maintainer_env();
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

    let compare = |from: &str, to: &str| {
        stikk_core::compare(
            &backend,
            &repo,
            &stikk_prikk::Comparison::Points {
                from: from.to_string(),
                to: to.to_string(),
            },
        )
        .unwrap_or_else(|e| panic!("0.{}: compare {from} -> {to}: {e}", bin.minor))
    };

    // **Two diverged branches.** Both sides keep the name they were asked by, and each resolves
    // to its own block -- which is what the heading names beside the ref (decision 3).
    let view = compare("heads/main", "heads/feature");
    let forward = view
        .listed()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert_eq!(forward.from.point, "heads/main");
    assert_eq!(forward.to.point, "heads/feature");
    let from_block = match &forward.from.target {
        stikk_prikk::DiffTarget::Block(id) => id.clone(),
        stikk_prikk::DiffTarget::Worktree => {
            panic!("0.{}: a ref must resolve to a block", bin.minor)
        }
    };
    let to_block = match &forward.to.target {
        stikk_prikk::DiffTarget::Block(id) => id.clone(),
        stikk_prikk::DiffTarget::Worktree => {
            panic!("0.{}: a ref must resolve to a block", bin.minor)
        }
    };
    assert_ne!(
        from_block, to_block,
        "0.{}: the two branches must have diverged",
        bin.minor
    );
    assert!(
        !forward.entries.is_empty(),
        "0.{}: a divergence must show entries",
        bin.minor
    );
    let statuses = |c: &stikk_core::Compared| {
        let mut v: Vec<String> = c.entries.iter().map(|e| e.status.clone()).collect();
        v.sort();
        v
    };

    // **The same pair reversed swaps the sides**, as measured: `added` becomes `deleted`.
    let view = compare("heads/feature", "heads/main");
    let back = view
        .listed()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert_eq!(back.from.point, "heads/feature");
    assert_eq!(back.to.point, "heads/main");
    assert_eq!(
        statuses(forward),
        statuses(back),
        "0.{}: the same two statuses, on opposite sides",
        bin.minor
    );
    assert!(
        statuses(forward).iter().any(|s| s == "added")
            && statuses(forward).iter().any(|s| s == "deleted"),
        "0.{}: measured as one added and one deleted, got {:?}",
        bin.minor,
        statuses(forward)
    );

    // **A tag is an ordinary side.** It resolves to the block it tags, which is `heads/main`'s.
    let view = compare("tags/v1", "heads/feature");
    let tagged = view
        .listed()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert_eq!(tagged.from.point, "tags/v1");
    assert_eq!(
        tagged.from.target,
        stikk_prikk::DiffTarget::Block(from_block.clone()),
        "0.{}: the tag resolves to the block heads/main is on",
        bin.minor
    );
    assert_eq!(statuses(tagged), statuses(forward));

    // **Two refs at the same block are no differences**, computed by prikk (decision 5).
    let view = compare("heads/main", "tags/v1");
    let same = view
        .listed()
        .unwrap_or_else(|| panic!("0.{}: 0.51 is above the band", bin.minor));
    assert!(
        same.is_empty(),
        "0.{}: a ref and a tag on one block differ in nothing, got {:?}",
        bin.minor,
        same.entries
    );
}

/// **RFC 041 below the band:** at 0.28 a ref pair is *not reported*, in stikk's own voice --
/// never an empty comparison, which would say the two refs are the same.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc041_below_the_band_a_ref_pair_is_unreported_not_empty() {
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);

    let view = stikk_core::compare(
        &backend,
        fixture.repo(),
        &stikk_prikk::Comparison::Points {
            from: "heads/main".to_string(),
            to: "heads/feature".to_string(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: below the band must not be an error: {e}", bin.minor));
    assert!(
        view.listed().is_none(),
        "0.{}: nothing should have been read",
        bin.minor
    );
    let reason = view
        .unreported_reason()
        .unwrap_or_else(|| panic!("0.{}: a reason in stikk's own voice", bin.minor));
    assert!(
        reason.contains(&format!("0.{}", stikk_core::COMPARE_FROM_MINOR)),
        "0.{}: {reason}",
        bin.minor
    );
    assert!(
        reason.contains(&format!("this is 0.{}", bin.minor)),
        "0.{}: {reason}",
        bin.minor
    );
}

/// **RFC 044 at the ceiling: the three ref writes against a real repository.**
///
/// Each one goes through the shipped path — preview, confirm, execute — and is then asserted by
/// **re-reading the lists**, never by reading prikk's own success prose (trap 2). The duplicate
/// name is here for the other half: prikk's refusal has to arrive verbatim, not be pre-empted by a
/// naming rule stikk invented (decision 5).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc044_the_three_ref_writes_change_the_lists_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::REF_WRITES_FROM_MINOR {
        eprintln!(
            "RFC 044 SKIPPED at 0.{} — the three writes are banded at 0.{}. Announced rather than \
             silent (RFC 022 §3).",
            bin.minor,
            stikk_core::REF_WRITES_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    // Two sealed blocks, so there is a tip and an older block id to tag.
    let mut blocks: Vec<String> = Vec::new();
    for (name, body) in [("one.txt", "one\n"), ("two.txt", "two\n")] {
        std::fs::write(repo.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
        fixture.set_author_env();
        backend
            .commit(&repo, "heads/main", "x")
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        let sealed = backend
            .seal(&repo, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
        blocks.push(sealed.block_id);
    }
    let older = blocks
        .first()
        .unwrap_or_else(|| panic!("0.{}: two blocks were sealed", bin.minor))
        .clone();

    // Every write runs with MAINTAINER readiness in the environment, as the UI's own gate requires.
    let write = |run: &dyn Fn(stikk_core::PreviewToken, stikk_model::Readiness) -> _,
                 preview: stikk_core::RefWritePreview,
                 what: &str| {
        let stikk_core::RefWritePreview::Ready { token } = preview else {
            panic!(
                "0.{}: {what}: expected a Ready preview, got {preview:?}",
                bin.minor
            );
        };
        fixture.set_maintainer_env();
        let readiness = backend
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness;
        let out = run(*token, readiness);
        Fixture::clear_env();
        out
    };

    // ── 1. create a branch from heads/main, and see it listed ───────────────────────────────
    fixture.set_maintainer_env();
    let preview = stikk_core::create_branch_preview(&backend, &repo, "heads/feature", "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: create_branch_preview: {e}", bin.minor));
    Fixture::clear_env();
    let created = write(
        &|token, readiness| {
            stikk_core::create_branch_confirm_and_execute(
                &backend,
                &repo,
                token,
                readiness,
                stikk_core::Evidence::ExplicitYes,
                "heads/feature",
                "heads/main",
            )
        },
        preview,
        "create a branch",
    )
    .unwrap_or_else(|e| panic!("0.{}: create a branch: {e}", bin.minor));
    let stikk_prikk::RefWrite::Done(lines) = created.result else {
        panic!("0.{}: the ceiling is above the band", bin.minor);
    };
    assert!(
        !lines.is_empty(),
        "0.{}: prikk's own lines are what the result screen shows",
        bin.minor
    );
    let listed = |backend: &CliBackend| {
        stikk_core::list_refs(backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: list_refs: {e}", bin.minor))
    };
    assert!(
        listed(&backend)
            .refs
            .iter()
            .any(|entry| entry.name == "heads/feature" && !entry.closed),
        "0.{}: the new branch must be in the list prikk reports, open",
        bin.minor
    );

    // ── 2. tag an older block id, and see the tag at that block ─────────────────────────────
    fixture.set_maintainer_env();
    let tag = stikk_core::NewTag {
        name: "tags/v1",
        target: older.as_str(),
        message: "first release",
    };
    let preview = stikk_core::create_tag_preview(&backend, &repo, tag)
        .unwrap_or_else(|e| panic!("0.{}: create_tag_preview: {e}", bin.minor));
    Fixture::clear_env();
    write(
        &|token, readiness| {
            stikk_core::create_tag_confirm_and_execute(
                &backend,
                &repo,
                token,
                readiness,
                stikk_core::Evidence::ExplicitYes,
                tag,
            )
        },
        preview,
        "create a tag",
    )
    .unwrap_or_else(|e| panic!("0.{}: create a tag: {e}", bin.minor));
    let tags = backend
        .tags(&repo)
        .unwrap_or_else(|e| panic!("0.{}: tags: {e}", bin.minor));
    assert!(
        tags.iter().any(|entry| entry.name == "tags/v1"),
        "0.{}: the new tag must be in prikk's own tag list, got {:?}",
        bin.minor,
        tags.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    // **At that block, not at the tip**: a tag of a block id is the one thing `--target` takes
    // that a branch's `--from` does not (F2), so the pointer is checked, not just the name.
    let at = stikk_core::compare(
        &backend,
        &repo,
        &stikk_prikk::Comparison::Points {
            from: "tags/v1".to_string(),
            to: older.clone(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: compare the tag with its block: {e}", bin.minor));
    assert!(
        at.listed().is_some_and(stikk_core::Compared::is_empty),
        "0.{}: the tag and the block it names differ in nothing; got {at:?}",
        bin.minor
    );

    // ── 3. a duplicate name is prikk's refusal, verbatim ────────────────────────────────────
    fixture.set_maintainer_env();
    let preview = stikk_core::create_branch_preview(&backend, &repo, "heads/feature", "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: duplicate preview: {e}", bin.minor));
    Fixture::clear_env();
    let refused = write(
        &|token, readiness| {
            stikk_core::create_branch_confirm_and_execute(
                &backend,
                &repo,
                token,
                readiness,
                stikk_core::Evidence::ExplicitYes,
                "heads/feature",
                "heads/main",
            )
        },
        preview,
        "a duplicate name",
    );
    match refused {
        Err(StikkError::Refusal { message }) => {
            assert!(
                message.contains("heads/feature") && message.contains("exists"),
                "0.{}: prikk's own words, unaltered: {message}",
                bin.minor
            );
            println!("0.{}: duplicate branch refusal — {message}", bin.minor);
        }
        other => panic!(
            "0.{}: a duplicate name must arrive as prikk's refusal; got {other:?}",
            bin.minor
        ),
    }

    // ── 4. close a non-current branch, and see it closed ────────────────────────────────────
    fixture.set_maintainer_env();
    let preview = stikk_core::close_branch_preview(&backend, &repo, "heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: close_branch_preview: {e}", bin.minor));
    Fixture::clear_env();
    write(
        &|token, readiness| {
            stikk_core::close_branch_confirm_and_execute(
                &backend,
                &repo,
                token,
                readiness,
                stikk_core::Evidence::ExplicitYes,
                "heads/feature",
            )
        },
        preview,
        "close a branch",
    )
    .unwrap_or_else(|e| panic!("0.{}: close a branch: {e}", bin.minor));
    assert!(
        listed(&backend)
            .refs
            .iter()
            .any(|entry| entry.name == "heads/feature" && entry.closed),
        "0.{}: prikk's own list must now mark it closed",
        bin.minor
    );

    // ── 5. the branch the repository is on is refused, and nothing runs ─────────────────────
    //
    // **F3 is why**: with an empty WAL prikk closes it, and a default-source `branch create`
    // then refuses because `.prikk/current-branch` names a closed branch. The proof that stikk
    // spawned nothing is that `heads/main` is still open afterwards, by prikk's own list.
    fixture.set_maintainer_env();
    let blocked = stikk_core::close_branch_preview(&backend, &repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: close_branch_preview on main: {e}", bin.minor));
    Fixture::clear_env();
    match blocked {
        stikk_core::RefWritePreview::Blocked(reason) => {
            assert_eq!(
                reason,
                stikk_core::CLOSING_THE_CURRENT_BRANCH,
                "0.{}: stikk's own words, not prikk's",
                bin.minor
            );
        }
        stikk_core::RefWritePreview::Ready { .. } => panic!(
            "0.{}: closing the current branch must never reach a confirmation",
            bin.minor
        ),
    }
    assert!(
        listed(&backend)
            .refs
            .iter()
            .any(|entry| entry.name == "heads/main" && !entry.closed),
        "0.{}: heads/main must still be open — nothing was spawned",
        bin.minor
    );
}

/// **RFC 044 below the band:** at 0.28 the three writes are *unavailable*, in stikk's own voice,
/// and **nothing is spawned** — the seam answers from the handshake alone.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc044_below_the_band_the_three_ref_writes_are_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    for (what, answer) in [
        (
            "create a branch",
            backend.create_branch(&repo, "heads/feature", "heads/main"),
        ),
        (
            "create a tag",
            backend.create_tag(&repo, "tags/v1", "heads/main", ""),
        ),
        ("close a branch", backend.close_branch(&repo, "heads/main")),
    ] {
        let answer = answer.unwrap_or_else(|e| {
            panic!("0.{}: {what} below the band must not error: {e}", bin.minor)
        });
        assert!(
            matches!(answer, stikk_prikk::RefWrite::Unavailable),
            "0.{}: {what} must answer Unavailable, got {answer:?}",
            bin.minor
        );
    }
    // Nothing was created, by prikk's own list — the seam never spawned it.
    let refs = stikk_core::list_refs(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: list_refs: {e}", bin.minor));
    assert!(
        !refs.refs.iter().any(|entry| entry.name == "heads/feature"),
        "0.{}: nothing may have been created, got {:?}",
        bin.minor,
        refs.refs.iter().map(|r| &r.name).collect::<Vec<_>>()
    );

    // And stikk's own words for it, with the band's number and this prikk's.
    let reason = stikk_core::ref_writes_unavailable(bin.minor);
    assert!(
        reason.contains(&format!("0.{}", stikk_core::REF_WRITES_FROM_MINOR)),
        "{reason}"
    );
    assert!(reason.contains(&format!("0.{}", bin.minor)), "{reason}");
    println!("0.{}: {reason}", bin.minor);
}

/// **RFC 045 at the ceiling: a switch against a real repository.**
///
/// The load-bearing assertion is F4's: **the preview's own written and deleted paths are what
/// the switch then did**, checked against the worktree's file list afterwards — not against
/// prikk's counts, which this test never reads for state. The rest are the refusals that must
/// write nothing, and the case stikk does not offer.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc045_the_preview_is_what_the_switch_does_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::SWITCH_FROM_MINOR {
        eprintln!(
            "RFC 045 SKIPPED at 0.{} — a switch is banded at 0.{}. Announced rather than silent \
             (RFC 022 §3).",
            bin.minor,
            stikk_core::SWITCH_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
    };
    let seal = |reff: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, "x")
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let worktree_files = || {
        let mut out: Vec<String> = std::fs::read_dir(&repo)
            .unwrap_or_else(|e| panic!("0.{}: read the worktree: {e}", bin.minor))
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().into_owned();
                (name != ".prikk").then_some(name)
            })
            .collect();
        out.sort();
        out
    };
    let current = || match backend
        .orientation(&repo)
        .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
        .current_branch
    {
        stikk_model::CurrentBranch::Branch(name) => name.as_str().to_string(),
        other => panic!("0.{}: expected a current branch, got {other:?}", bin.minor),
    };
    // Each switch runs through the shipped path: preview, confirm, execute.
    let switch =
        |target: &str| -> stikk_model::Result<stikk_core::Outcome<stikk_prikk::SwitchResult>> {
            let preview = stikk_core::switch_preview(&backend, &repo, target)
                .unwrap_or_else(|e| panic!("0.{}: switch_preview {target}: {e}", bin.minor));
            let stikk_core::SwitchPreview::Ready { token } = preview else {
                panic!("0.{}: expected a Ready preview, got {preview:?}", bin.minor);
            };
            let plan = token
                .summary()
                .materializes
                .clone()
                .unwrap_or_else(|| panic!("0.{}: a Ready preview carries a plan", bin.minor));
            let readiness = backend
                .readiness(&repo)
                .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
                .readiness;
            PLAN.with(|slot| *slot.borrow_mut() = Some(plan));
            stikk_core::switch_confirm_and_execute(
                &backend,
                &repo,
                *token,
                readiness,
                stikk_core::Evidence::ExplicitYes,
                target,
            )
        };
    let previewed = || PLAN.with(|slot| slot.borrow().clone().expect("a plan was previewed"));

    // heads/main: a.txt, b.txt. heads/feature: a.txt changed, b.txt gone, c.txt new.
    write("a.txt", "one\n");
    write("b.txt", "two\n");
    seal("heads/main");
    fixture.set_maintainer_env();
    prikk_ok(
        &bin,
        &repo,
        &["branch", "create", "heads/feature", "--from", "heads/main"],
    );
    Fixture::clear_env();
    let main_a = std::fs::read_to_string(repo.join("a.txt")).expect("a.txt");
    let main_files = worktree_files();

    switch("heads/feature").unwrap_or_else(|e| panic!("0.{}: switch: {e}", bin.minor));
    assert_eq!(current(), "heads/feature", "0.{}", bin.minor);
    write("a.txt", "one, changed\n");
    std::fs::remove_file(repo.join("b.txt")).expect("remove b.txt");
    write("c.txt", "three\n");
    seal("heads/feature");

    // ── 1. the plan is the switch ───────────────────────────────────────────────────────────
    switch("heads/main").unwrap_or_else(|e| panic!("0.{}: switch back: {e}", bin.minor));
    let plan = previewed();
    assert_eq!(
        plan.written,
        vec!["a.txt".to_string(), "b.txt".to_string()],
        "0.{}: the preview's written paths",
        bin.minor
    );
    assert_eq!(
        plan.deleted,
        vec!["c.txt".to_string()],
        "0.{}: the preview's deleted paths",
        bin.minor
    );
    assert_eq!(
        worktree_files(),
        main_files,
        "0.{}: the worktree is exactly what the plan said it would be",
        bin.minor
    );
    // **Switching back restores every byte** (F3), checked by content and not by a count.
    assert_eq!(
        std::fs::read_to_string(repo.join("a.txt")).expect("a.txt"),
        main_a,
        "0.{}: the bytes are back",
        bin.minor
    );
    assert_eq!(current(), "heads/main", "0.{}", bin.minor);

    // ── 2. a dirty worktree is refused verbatim, and nothing is written ─────────────────────
    write("a.txt", "dirty\n");
    let err = switch("heads/feature").expect_err("a dirty worktree is refused");
    assert_eq!(err.class(), "refusal", "0.{}: {err}", bin.minor);
    assert!(
        err.to_string().contains("the worktree is not clean"),
        "0.{}: prikk's own words: {err}",
        bin.minor
    );
    assert_eq!(current(), "heads/main", "0.{}: nothing moved", bin.minor);
    assert_eq!(
        std::fs::read_to_string(repo.join("a.txt")).expect("a.txt"),
        "dirty\n",
        "0.{}: nothing was written over",
        bin.minor
    );
    write("a.txt", &main_a);

    // ── 3. an untracked file the target does not need is left alone ─────────────────────────
    write("notes.md", "mine\n");
    switch("heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: switch with an untracked file: {e}", bin.minor));
    assert_eq!(
        std::fs::read_to_string(repo.join("notes.md")).expect("notes.md"),
        "mine\n",
        "0.{}: an untracked file is never touched",
        bin.minor
    );
    std::fs::remove_file(repo.join("notes.md")).expect("remove notes.md");
    switch("heads/main").unwrap_or_else(|e| panic!("0.{}: switch back: {e}", bin.minor));

    // ── 4. work queued for the target is not offered, and nothing is spawned ────────────────
    //
    // **F5**: prikk allows this and writes the target's sealed files only, leaving the queued
    // work absent from the worktree. The proof that stikk spawned nothing is that the current
    // branch is unchanged afterwards, by prikk's own report.
    write("queued.txt", "for feature\n");
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/feature", "queued for feature")
        .unwrap_or_else(|e| panic!("0.{}: commit onto heads/feature: {e}", bin.minor));
    Fixture::clear_env();
    std::fs::remove_file(repo.join("queued.txt")).expect("remove queued.txt");
    let blocked = stikk_core::switch_preview(&backend, &repo, "heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: switch_preview: {e}", bin.minor));
    match blocked {
        stikk_core::SwitchPreview::Blocked(reason) => assert_eq!(
            reason,
            stikk_core::WORK_IS_QUEUED_FOR_THIS_BRANCH,
            "0.{}: stikk's own words",
            bin.minor
        ),
        stikk_core::SwitchPreview::Ready { .. } => panic!(
            "0.{}: a switch to a branch the queue holds work for must not reach a confirmation",
            bin.minor
        ),
    }
    assert_eq!(
        current(),
        "heads/main",
        "0.{}: nothing was spawned — the current branch is unchanged",
        bin.minor
    );
}

thread_local! {
    /// The plan the last previewed switch carried, so the leg above can compare it with what the
    /// switch then did without threading it back through a closure's return type.
    static PLAN: std::cell::RefCell<Option<stikk_core::FilePlan>> =
        const { std::cell::RefCell::new(None) };
}

/// **RFC 045 below the band:** at 0.28 a switch is *unavailable*, in stikk's own voice, and
/// nothing is spawned — the seam answers from the handshake alone.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc045_below_the_band_a_switch_is_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    let answer = backend
        .switch_branch(&repo, "heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: below the band must not error: {e}", bin.minor));
    assert!(
        matches!(answer, stikk_prikk::SwitchResult::Unavailable),
        "0.{}: got {answer:?}",
        bin.minor
    );
    // And the preview says the same thing without a plan to show.
    let preview = stikk_core::switch_preview(&backend, &repo, "heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: switch_preview: {e}", bin.minor));
    assert!(
        matches!(preview, stikk_core::SwitchPreview::Blocked(_)),
        "0.{}: got {preview:?}",
        bin.minor
    );
    let reason = stikk_core::switch_unavailable(bin.minor);
    assert!(
        reason.contains(&format!("0.{}", stikk_core::SWITCH_FROM_MINOR)),
        "{reason}"
    );
    assert!(reason.contains(&format!("0.{}", bin.minor)), "{reason}");
    println!("0.{}: {reason}", bin.minor);
}

/// **RFC 046 at the ceiling: a session restored against prikk's own lists.**
///
/// The decision `stikk-core::restore` makes is a pure function, so this drives it with lists a
/// real prikk produced rather than with fixtures — which is the only way to know that a record
/// written in one launch still means what it meant when the next one reads it.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc046_a_session_restores_against_real_lists_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();
    // `STIKK_STATE_DIR`'s own scratch, outside the repository — which is where every record
    // belongs, and what the next test proves stikk refuses to do otherwise.
    let state = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture has a parent", bin.minor))
        .join("state");

    std::fs::write(repo.join("a.txt"), "one\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    prikk_ok(
        &bin,
        &repo,
        &["branch", "create", "heads/second", "--from", "heads/main"],
    );
    Fixture::clear_env();

    let record = stikk_state::SessionRecord {
        root: repo.clone(),
        focus: Some("heads/second".to_string()),
        view: stikk_state::LastView::Changes,
        hide_untracked: true,
    };
    stikk_state::session::save(&state, &repo, &record)
        .unwrap_or_else(|e| panic!("0.{}: save: {e}", bin.minor));

    let reread = || {
        stikk_state::session::load(&state, &repo)
            .unwrap_or_else(|| panic!("0.{}: the record reads back", bin.minor))
    };
    let lists = || {
        stikk_core::list_refs(&backend, &repo)
            .unwrap_or_else(|e| panic!("0.{}: list_refs: {e}", bin.minor))
            .refs
    };
    let current = || {
        backend
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor))
            .current_branch
    };

    // ── 1. all three come back, from prikk's real lists ─────────────────────────────────────
    assert_eq!(reread(), record, "0.{}: the record round-trips", bin.minor);
    let restored = stikk_core::restore(Some(&reread()), &lists(), &current(), true);
    assert_eq!(
        restored.focus.as_deref(),
        Some("heads/second"),
        "0.{}: the remembered branch wins over prikk's current one",
        bin.minor
    );
    assert_eq!(
        restored.view,
        stikk_state::LastView::Changes,
        "0.{}",
        bin.minor
    );
    assert!(restored.hide_untracked, "0.{}", bin.minor);
    assert_eq!(restored.note, None, "0.{}: nothing to say", bin.minor);

    // ── 2. close that branch with prikk, and the fallback takes over, with the note ─────────
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "close", "heads/second"]);
    Fixture::clear_env();
    let restored = stikk_core::restore(Some(&reread()), &lists(), &current(), true);
    assert_eq!(
        restored.focus.as_deref(),
        Some("heads/main"),
        "0.{}: a closed branch is not restored",
        bin.minor
    );
    assert_eq!(
        restored.note.as_deref(),
        Some(stikk_core::REMEMBERED_REF_IS_GONE),
        "0.{}",
        bin.minor
    );
    // The view and the filter are preferences and survive: each view re-reads from prikk.
    assert_eq!(
        restored.view,
        stikk_state::LastView::Changes,
        "0.{}",
        bin.minor
    );
    assert!(restored.hide_untracked, "0.{}", bin.minor);
}

/// **RFC 046's `C-E2` leg**: a state directory inside the repository is refused, and **the
/// repository is byte-identical afterwards**. This is the control the refusal exists for — its
/// first real caller, measured against a real repository rather than a scratch directory.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc046_a_state_dir_inside_the_repository_is_refused_and_changes_nothing() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    std::fs::write(repo.join("a.txt"), "one\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    let before = tree_fingerprint(&repo);
    assert!(
        before.len() > 5,
        "0.{}: a sealed repository has files",
        bin.minor
    );

    let record = stikk_state::SessionRecord {
        root: repo.clone(),
        focus: Some("heads/main".to_string()),
        view: stikk_state::LastView::History,
        hide_untracked: false,
    };
    let outside_dir = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture has a parent", bin.minor));
    // **Three of these walked past the check until RFC 046 review v1 C1**: a relative state
    // directory, a `..` that leads back in, and a symlink into the repository. Each is measured
    // here against a real repository, where the consequence would be a stikk file the next
    // `commit --from-worktree` captures.
    let mut inside = vec![
        repo.join(".stikk"),
        repo.join(".prikk").join("stikk"),
        repo.join("docs").join("state"),
        // Down and back up: `<repo>/..` canonicalizes to the fixture, and the remainder names
        // the repository again.
        repo.join("..").join(
            repo.file_name()
                .unwrap_or_else(|| panic!("0.{}: the repo has a name", bin.minor)),
        ),
    ];
    #[cfg(unix)]
    {
        let link = outside_dir.join("link-into-repo");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&repo, &link)
            .unwrap_or_else(|e| panic!("0.{}: symlink: {e}", bin.minor));
        inside.push(link.join("state"));
    }
    for inside in inside {
        let Err(error) = stikk_state::session::save(&inside, &repo, &record) else {
            panic!("0.{}: C-E2 must refuse {}", bin.minor, inside.display());
        };
        assert_eq!(error.class(), "stikk-internal", "0.{}: {error}", bin.minor);
        assert!(
            error.to_string().contains("refusing to write"),
            "0.{}: {error}",
            bin.minor
        );
    }
    // The relative shape, which needs the process to be inside the worktree to mean anything.
    // `set_current_dir` is process-wide, so it is restored immediately — `ENV_LOCK` is held for
    // the whole test, which is what keeps that safe here.
    let restore = std::env::current_dir().ok();
    if std::env::set_current_dir(&repo).is_ok() {
        let error = stikk_state::session::save(std::path::Path::new("state"), &repo, &record)
            .expect_err("a relative state directory must not escape the check");
        assert_eq!(error.class(), "stikk-internal", "0.{}: {error}", bin.minor);
        if let Some(restore) = restore {
            let _ = std::env::set_current_dir(restore);
        }
    }

    assert_eq!(
        tree_fingerprint(&repo),
        before,
        "0.{}: every file in the repository must be byte-identical",
        bin.minor
    );

    // And a state directory outside it writes, so the refusals above are about where, not what.
    let outside = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture has a parent", bin.minor))
        .join("state-outside");
    stikk_state::session::save(&outside, &repo, &record).unwrap_or_else(|e| {
        panic!(
            "0.{}: a state dir outside the repository writes: {e}",
            bin.minor
        )
    });
    assert_eq!(
        tree_fingerprint(&repo),
        before,
        "0.{}: and that write did not touch the repository either",
        bin.minor
    );
}

/// Every file under `root`, by relative path and content, so a comparison catches a new file, a
/// removed one and an edited one alike. Not a hash: the difference should be readable when it
/// fails.
fn tree_fingerprint(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = std::fs::read(&path) {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.push((relative, bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// **RFC 046 decision 6**: a private launch leaves the state directory untouched. The launcher
/// decides this (`STIKK_PRIVATE=1` → no [`stikk_tui::Session`]), so what is asserted here is the
/// store's half: nothing exists until something saves.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc046_a_private_session_leaves_the_state_directory_empty() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let state = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture has a parent", bin.minor))
        .join("private-state");
    std::fs::create_dir_all(&state)
        .unwrap_or_else(|e| panic!("0.{}: create the state dir: {e}", bin.minor));

    // What a private launch does: it never calls `load` and never calls `save`.
    assert_eq!(
        stikk_state::session::load(&state, &repo),
        None,
        "0.{}: nothing to read",
        bin.minor
    );
    let left: Vec<String> = std::fs::read_dir(&state)
        .unwrap_or_else(|e| panic!("0.{}: read the state dir: {e}", bin.minor))
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        left.is_empty(),
        "0.{}: the state directory is untouched, got {left:?}",
        bin.minor
    );
}

/// **RFC 047 at the ceiling: the baseline stikk finds is the one prikk accepts.**
///
/// The load-bearing assertion is the first one: the block [`stikk_core::merge_baseline`] names
/// from two `log` reads is the block a real `prikk merge` then merges from. A baseline that is
/// merely plausible is not a weaker answer here but a wrong one — measured at 0.52, a baseline
/// older than the true merge base turned a confluent pair into a `Conflict` — so nothing short
/// of prikk accepting it proves the finder right.
///
/// The rest is scenario 1 and scenario 3 of `measure047.sh`, driven through the shipped path.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc047_the_baseline_stikk_finds_is_the_one_prikk_merges_from() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::MERGE_FROM_MINOR {
        eprintln!(
            "RFC 047 SKIPPED at 0.{} — the merge plan is banded at 0.{}. Announced rather than \
             silent (RFC 022 §3).",
            bin.minor,
            stikk_core::MERGE_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();
    let limit = stikk_core::MERGE_LINEAGE_LIMIT;

    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
    };
    let seal = |reff: &str, message: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, message)
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let switch = |target: &str| {
        fixture.set_maintainer_env();
        prikk_ok(&bin, &repo, &["branch", "switch", target]);
        Fixture::clear_env();
    };
    let tip = |reff: &str| -> String {
        backend
            .history(&repo, reff, 1)
            .unwrap_or_else(|e| panic!("0.{}: log {reff}: {e}", bin.minor))
            .blocks
            .first()
            .unwrap_or_else(|| panic!("0.{}: {reff} has a tip", bin.minor))
            .block_id
            .clone()
    };

    // ── scenario 1's shape: feature adds c.txt, main changes b.txt ───────────────────────────
    write("a.txt", "a\n");
    write("b.txt", "b\n");
    seal("heads/main", "base");
    let base = tip("heads/main");
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "create", "heads/feature"]);
    Fixture::clear_env();
    switch("heads/feature");
    write("c.txt", "c\n");
    seal("heads/feature", "feature adds c");
    switch("heads/main");
    write("b.txt", "b2\n");
    seal("heads/main", "main changes b");

    // **The merge is main into feature** — feature is not the current branch, which is the only
    // shape stikk offers (F3).
    let plan = stikk_core::merge_plan_view(&backend, &repo, "heads/feature", "heads/main", limit)
        .unwrap_or_else(|e| panic!("0.{}: merge_plan_view: {e}", bin.minor));
    let stikk_core::MergePlanRead::Read(view) = plan else {
        panic!("0.{}: expected a plan, got {plan:?}", bin.minor);
    };
    assert!(
        base.starts_with(&view.baseline),
        "0.{}: stikk named {} where the base block is {base}",
        bin.minor,
        view.baseline
    );
    assert_eq!(
        view.outcome,
        stikk_prikk::MergeOutcome::Confluent,
        "0.{}: scenario 1 is confluent; prikk printed {:?}",
        bin.minor,
        view.lines
    );
    // prikk's own lines are what stikk shows, and the verdict is one of them.
    assert!(
        view.lines
            .iter()
            .any(|line| line.trim() == "evidence outcome: Confluent"),
        "0.{}: {:?}",
        bin.minor,
        view.lines
    );

    // **The proof: prikk merges from the block stikk named.** A baseline prikk will not accept
    // refuses here with *"is not an ancestor of target"*, which is exactly the failure a plausible
    // guess would produce.
    fixture.set_maintainer_env();
    prikk_ok(
        &bin,
        &repo,
        &[
            "merge",
            "--allow-no-audit",
            "--baseline-block",
            &base,
            "--into",
            "heads/feature",
            "--from",
            "heads/main",
        ],
    );
    Fixture::clear_env();

    // ── after the merge, stikk names no baseline at all (decision 3) ─────────────────────────
    let after = stikk_core::merge_plan_view(&backend, &repo, "heads/main", "heads/feature", limit)
        .unwrap_or_else(|e| panic!("0.{}: merge_plan_view after the merge: {e}", bin.minor));
    assert_eq!(
        after,
        stikk_core::MergePlanRead::Declined(stikk_core::MERGED_ALREADY.to_string()),
        "0.{}: a Merge block above the shared one is declined",
        bin.minor
    );
    // And the reason is a real one: feature's lineage really does hold a `Merge` block now, with
    // both signals agreeing — the fixture the core tests pin.
    let lineage = backend
        .history(&repo, "heads/feature", limit)
        .unwrap_or_else(|e| panic!("0.{}: log heads/feature: {e}", bin.minor));
    let merge_block = lineage
        .blocks
        .iter()
        .find(|block| block.kind == "Merge")
        .unwrap_or_else(|| panic!("0.{}: {:?}", bin.minor, lineage.blocks));
    assert!(
        merge_block.parents > 1,
        "0.{}: kind and parent_count agree: {merge_block:?}",
        bin.minor
    );

    // ── scenario 3: both sides change a.txt ──────────────────────────────────────────────────
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
    };
    let seal = |reff: &str, message: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, message)
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let switch = |target: &str| {
        fixture.set_maintainer_env();
        prikk_ok(&bin, &repo, &["branch", "switch", target]);
        Fixture::clear_env();
    };
    write("a.txt", "a\n");
    seal("heads/main", "base");
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "create", "heads/feature"]);
    Fixture::clear_env();
    switch("heads/feature");
    write("a.txt", "a from feature\n");
    seal("heads/feature", "feature a");
    switch("heads/main");
    write("a.txt", "a from main\n");
    seal("heads/main", "main a");

    let plan = stikk_core::merge_plan_view(&backend, &repo, "heads/feature", "heads/main", limit)
        .unwrap_or_else(|e| panic!("0.{}: merge_plan_view, conflicting: {e}", bin.minor));
    let stikk_core::MergePlanRead::Read(view) = plan else {
        panic!(
            "0.{}: a conflicting pair still has a plan, got {plan:?}",
            bin.minor
        );
    };
    assert_eq!(
        view.outcome,
        stikk_prikk::MergeOutcome::Conflict,
        "0.{}: prikk printed {:?}",
        bin.minor,
        view.lines
    );
}

/// **RFC 047 at the ceiling: the fact the into-current guard exists for** (F3).
///
/// stikk's own guard is in the TUI, where it declines without spawning (`App`'s tests). What a
/// real binary can show is **why it has to**: prikk allows this merge, writes no file, and the
/// way out is not even a switch. If prikk ever changes that, this leg fails and the guard — and
/// the sentence stikk shows for it — stop being true.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc047_a_merge_into_the_current_branch_leaves_the_worktree_behind() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::MERGE_FROM_MINOR {
        eprintln!(
            "RFC 047 SKIPPED at 0.{} — banded at 0.{}.",
            bin.minor,
            stikk_core::MERGE_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body).expect("write");
    };
    let seal = |reff: &str, message: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, message)
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let switch = |target: &str| {
        fixture.set_maintainer_env();
        let out = std::process::Command::new(&bin.path)
            .args(["branch", "switch", target])
            .current_dir(&repo)
            .output()
            .expect("spawn prikk");
        Fixture::clear_env();
        out
    };

    write("a.txt", "a\n");
    seal("heads/main", "base");
    let base = backend
        .history(&repo, "heads/main", 1)
        .expect("log")
        .blocks
        .first()
        .expect("a tip")
        .block_id
        .clone();
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "create", "heads/feature"]);
    Fixture::clear_env();
    assert!(switch("heads/feature").status.success());
    write("c.txt", "c\n");
    seal("heads/feature", "feature adds c");
    assert!(switch("heads/main").status.success());

    // **prikk allows it** — into the branch the repository is on, which stikk never offers.
    fixture.set_maintainer_env();
    prikk_ok(
        &bin,
        &repo,
        &[
            "merge",
            "--allow-no-audit",
            "--baseline-block",
            &base,
            "--into",
            "heads/main",
            "--from",
            "heads/feature",
        ],
    );
    Fixture::clear_env();

    // … and the adopted file is simply not there.
    assert!(
        !repo.join("c.txt").exists(),
        "0.{}: F3 — the merge wrote no file",
        bin.minor
    );
    // … and the way out is not a switch: prikk refuses, because the worktree is not clean.
    let refused = switch("heads/feature");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        !refused.status.success() && said.contains("c.txt"),
        "0.{}: F3 — a switch out is refused: {said}",
        bin.minor
    );
}

/// **RFC 047 below the band:** at 0.28 the merge plan is *unavailable*, in stikk's own voice,
/// and **nothing is spawned** to find that out.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc047_below_the_band_the_merge_plan_is_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    assert!(
        bin.minor < stikk_core::MERGE_FROM_MINOR,
        "the floor is below the band"
    );
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();

    let plan = backend
        .merge_plan(&repo, &"0".repeat(64), "heads/main", "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: merge_plan: {e}", bin.minor));
    assert_eq!(
        plan,
        stikk_prikk::MergePlan::Unavailable,
        "0.{}: nothing is spawned below the band",
        bin.minor
    );
    // The merge itself answers the same way, and spawns nothing either.
    let merged = backend
        .merge(&repo, &"0".repeat(64), "heads/main", "heads/feature")
        .unwrap_or_else(|e| panic!("0.{}: merge: {e}", bin.minor));
    assert_eq!(
        merged,
        stikk_prikk::MergeResult::Unavailable,
        "0.{}: nothing is spawned below the band",
        bin.minor
    );
    // stikk's own reason names both numbers (`C-T4d`).
    let reason = stikk_core::merge_unavailable(bin.minor);
    assert!(reason.contains(&format!("0.{}", bin.minor)), "{reason}");
    assert!(
        reason.contains(&format!("0.{}", stikk_core::MERGE_FROM_MINOR)),
        "{reason}"
    );
}

/// **RFC 047 Handoff B at the ceiling: scenario 1, end to end.**
///
/// Preview, confirm, merge — and then the two facts that make the merge worth anything: prikk's
/// own `log` shows a `Merge` block on the branch merged into, and **a switch to that branch
/// writes the adopted file**. Along the way, the two cases stikk never arms: a conflicting pair,
/// and a merge into the branch the repository is on, after which that branch's lineage is
/// **unchanged** — the proof that nothing was spawned.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc047_a_merge_lands_and_a_switch_to_it_writes_the_adopted_file() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::MERGE_FROM_MINOR {
        eprintln!(
            "RFC 047 SKIPPED at 0.{} — banded at 0.{}.",
            bin.minor,
            stikk_core::MERGE_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();
    let limit = stikk_core::MERGE_LINEAGE_LIMIT;

    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body).expect("write");
    };
    let seal = |reff: &str, message: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, message)
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let switch = |target: &str| {
        fixture.set_maintainer_env();
        prikk_ok(&bin, &repo, &["branch", "switch", target]);
        Fixture::clear_env();
    };
    let lineage = |reff: &str| -> Vec<String> {
        backend
            .history(&repo, reff, limit)
            .unwrap_or_else(|e| panic!("0.{}: log {reff}: {e}", bin.minor))
            .blocks
            .iter()
            .map(|block| format!("{}:{}", block.block_id, block.kind))
            .collect()
    };

    // heads/main: a.txt, b.txt. feature adds c.txt; main changes b.txt. Current is heads/main,
    // so the merge stikk offers is **main into feature**.
    write("a.txt", "a\n");
    write("b.txt", "b\n");
    seal("heads/main", "base");
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "create", "heads/feature"]);
    Fixture::clear_env();
    switch("heads/feature");
    write("c.txt", "c\n");
    seal("heads/feature", "feature adds c");
    switch("heads/main");
    write("b.txt", "b2\n");
    seal("heads/main", "main changes b");

    // ── the case stikk never arms: into the branch the repository is on (F3) ─────────────────
    let before = lineage("heads/main");
    let blocked = stikk_core::merge_preview(
        &backend,
        &repo,
        "heads/main",
        "heads/feature",
        "whatever",
        limit,
    )
    .unwrap_or_else(|e| panic!("0.{}: merge_preview into current: {e}", bin.minor));
    match blocked {
        stikk_core::MergePreview::Blocked(reason) => {
            assert_eq!(
                reason,
                stikk_core::INTO_THE_CURRENT_BRANCH,
                "0.{}",
                bin.minor
            );
        }
        other => panic!(
            "0.{}: into the current branch must block, got {other:?}",
            bin.minor
        ),
    }
    assert_eq!(
        lineage("heads/main"),
        before,
        "0.{}: nothing was spawned to decide that",
        bin.minor
    );

    // ── the merge stikk does offer: main into feature ────────────────────────────────────────
    let stikk_core::MergePlanRead::Read(plan) =
        stikk_core::merge_plan_view(&backend, &repo, "heads/feature", "heads/main", limit)
            .unwrap_or_else(|e| panic!("0.{}: merge_plan_view: {e}", bin.minor))
    else {
        panic!("0.{}: expected a plan", bin.minor);
    };
    assert_eq!(plan.outcome, stikk_prikk::MergeOutcome::Confluent);

    let preview = stikk_core::merge_preview(
        &backend,
        &repo,
        "heads/feature",
        "heads/main",
        &plan.baseline_short(),
        limit,
    )
    .unwrap_or_else(|e| panic!("0.{}: merge_preview: {e}", bin.minor));
    let stikk_core::MergePreview::Ready { token } = preview else {
        panic!("0.{}: expected a ready preview, got {preview:?}", bin.minor);
    };
    // The card restates both names, the baseline and the adopted tip — all four as ids.
    assert_eq!(token.summary().target_ids.len(), 4, "0.{}", bin.minor);
    assert!(
        token.summary().merge_plan.is_some(),
        "0.{}: prikk's plan rides to the card",
        bin.minor
    );

    fixture.set_maintainer_env();
    let readiness = backend
        .readiness(&repo)
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
        .readiness;
    let outcome = stikk_core::merge_confirm_and_execute(
        &backend,
        &repo,
        *token,
        readiness,
        stikk_core::Evidence::ExplicitYes,
        &stikk_core::MergeTarget {
            baseline: plan.baseline.clone(),
            into: "heads/feature".to_string(),
            from: "heads/main".to_string(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: merge: {e}", bin.minor));
    Fixture::clear_env();
    let stikk_prikk::MergeResult::Done(printed) = outcome.result else {
        panic!("0.{}: expected a merge that ran", bin.minor);
    };
    assert!(
        printed
            .first()
            .is_some_and(|line| line.starts_with("merged heads/main into heads/feature")),
        "0.{}: {printed:?}",
        bin.minor
    );

    // **prikk's own log shows the merge block**, on the branch merged into and nowhere else.
    let feature = lineage("heads/feature");
    assert!(
        feature.first().is_some_and(|row| row.ends_with(":Merge")),
        "0.{}: {feature:?}",
        bin.minor
    );
    assert!(
        !lineage("heads/main")
            .iter()
            .any(|row| row.ends_with(":Merge")),
        "0.{}: the branch merged *from* is unchanged",
        bin.minor
    );

    // **The worktree is untouched until the reader switches** — which is exactly what stikk's
    // one line on the result card says, and this is the measurement behind it.
    assert_eq!(
        std::fs::read_to_string(repo.join("b.txt")).expect("b.txt"),
        "b2\n",
        "0.{}: no file was written by the merge",
        bin.minor
    );
    switch("heads/feature");
    assert!(
        repo.join("c.txt").exists(),
        "0.{}: the switch writes the adopted side's files",
        bin.minor
    );
}

/// **RFC 047 Handoff B at the ceiling: a conflicting pair is never armed.**
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc047_a_conflicting_pair_is_never_armed() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::MERGE_FROM_MINOR {
        eprintln!("RFC 047 SKIPPED at 0.{}.", bin.minor);
        return;
    }
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = fixture.repo().to_path_buf();
    let limit = stikk_core::MERGE_LINEAGE_LIMIT;

    let write = |name: &str, body: &str| {
        std::fs::write(repo.join(name), body).expect("write");
    };
    let seal = |reff: &str, message: &str| {
        fixture.set_author_env();
        backend
            .commit(&repo, reff, message)
            .unwrap_or_else(|e| panic!("0.{}: commit on {reff}: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(&repo, reff)
            .unwrap_or_else(|e| panic!("0.{}: seal {reff}: {e}", bin.minor));
        Fixture::clear_env();
    };
    let switch = |target: &str| {
        fixture.set_maintainer_env();
        prikk_ok(&bin, &repo, &["branch", "switch", target]);
        Fixture::clear_env();
    };

    write("a.txt", "a\n");
    seal("heads/main", "base");
    fixture.set_maintainer_env();
    prikk_ok(&bin, &repo, &["branch", "create", "heads/feature"]);
    Fixture::clear_env();
    switch("heads/feature");
    write("a.txt", "a from feature\n");
    seal("heads/feature", "feature a");
    switch("heads/main");
    write("a.txt", "a from main\n");
    seal("heads/main", "main a");

    let stikk_core::MergePlanRead::Read(plan) =
        stikk_core::merge_plan_view(&backend, &repo, "heads/feature", "heads/main", limit)
            .unwrap_or_else(|e| panic!("0.{}: merge_plan_view: {e}", bin.minor))
    else {
        panic!("0.{}: a conflicting pair still has a plan", bin.minor);
    };
    assert_eq!(plan.outcome, stikk_prikk::MergeOutcome::Conflict);

    // **No token is minted**, so there is nothing to confirm — and the reason names prikk's own
    // word rather than a sentence of stikk's about conflicts in general.
    let preview = stikk_core::merge_preview(
        &backend,
        &repo,
        "heads/feature",
        "heads/main",
        &plan.baseline_short(),
        limit,
    )
    .unwrap_or_else(|e| panic!("0.{}: merge_preview: {e}", bin.minor));
    match preview {
        stikk_core::MergePreview::Blocked(reason) => {
            assert!(reason.contains("Conflict"), "0.{}: {reason}", bin.minor);
            assert!(
                reason.contains("Nothing was written"),
                "0.{}: {reason}",
                bin.minor
            );
        }
        other => panic!(
            "0.{}: a conflicting pair must block, got {other:?}",
            bin.minor
        ),
    }
}

// =============================================================================================
// RFC 050 Handoff A — opening a bundle, against real prikk binaries.
//
// `measure050b.sh`'s shape: the receiver is a **copy** of the sender's repository, so the two
// share history and prikk can answer `fast-forward`, `already-included` and `diverged`. A
// receiver built from scratch cannot connect to the sender's blocks at all, which is why the
// first measurement script only ever saw `no-local-history` and `does-not-connect`.
//
// Nothing here imports or exports through stikk: Handoff A has no writes. The bundles these
// legs read are made with raw `prikk bundle export`, which is the file a user would be handed.
// =============================================================================================

/// Copy a directory tree. The receiver is a copy of the sender's repository (`measure050b.sh`),
/// and std has no recursive copy.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|e| panic!("create {}: {e}", to.display()));
    let entries =
        std::fs::read_dir(from).unwrap_or_else(|e| panic!("read {}: {e}", from.display()));
    for entry in entries.flatten() {
        let kind = entry
            .file_type()
            .unwrap_or_else(|e| panic!("stat {}: {e}", entry.path().display()));
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)
                .unwrap_or_else(|e| panic!("copy {}: {e}", entry.path().display()));
        }
    }
}

/// A [`Prikk`] that counts the three bundle calls stikk must sometimes **not** make, and
/// delegates everything else to the real backend — so a leg can prove, against the real binary
/// rather than against a fake, that a refused bundle was never previewed and that a guard
/// declined without importing.
struct CountingWrites {
    inner: CliBackend,
    /// Atomic rather than a `Cell`: [`Prikk`] is `Sync`, as every backend the worker thread
    /// holds must be, and counters are the whole of this wrapper's state.
    previews: std::sync::atomic::AtomicUsize,
    /// How many times `bundle_import` was spawned. **The number the guard legs assert is
    /// zero** (RFC 050 F2): a guard that declines after the write would be no guard.
    imports: std::sync::atomic::AtomicUsize,
    /// How many times `bundle_export` was spawned, for the two destination rules.
    exports: std::sync::atomic::AtomicUsize,
    /// How many times `save_file` was spawned. **The number RFC 053's guard legs assert is
    /// zero**: stikk's two destination rules decline before prikk is asked, and a rule that
    /// declined after the write would be no rule.
    saves: std::sync::atomic::AtomicUsize,
    /// How many times `extract` was spawned, for the same two rules.
    extracts: std::sync::atomic::AtomicUsize,
    /// How many times a **trust write** was spawned -- `trust_add` and `trust_remove` together,
    /// since RFC 056's guard legs assert that neither ran.
    trust_writes: std::sync::atomic::AtomicUsize,
}

impl CountingWrites {
    fn new(bin: &PrikkBin) -> Self {
        Self {
            inner: CliBackend::with_program(&bin.path),
            previews: std::sync::atomic::AtomicUsize::new(0),
            imports: std::sync::atomic::AtomicUsize::new(0),
            exports: std::sync::atomic::AtomicUsize::new(0),
            saves: std::sync::atomic::AtomicUsize::new(0),
            trust_writes: std::sync::atomic::AtomicUsize::new(0),
            extracts: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Prikk for CountingWrites {
    fn handshake(&self) -> stikk_model::Result<stikk_prikk::Handshake> {
        self.inner.handshake()
    }
    fn readiness(
        &self,
        repo: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::ReadinessReport> {
        self.inner.readiness(repo)
    }
    fn orientation(&self, repo: &std::path::Path) -> stikk_model::Result<stikk_prikk::Orientation> {
        self.inner.orientation(repo)
    }
    fn history(
        &self,
        repo: &std::path::Path,
        reff: &str,
        limit: usize,
    ) -> stikk_model::Result<stikk_prikk::History> {
        self.inner.history(repo, reff, limit)
    }
    fn block_state(
        &self,
        repo: &std::path::Path,
        reff: &str,
        block_id: &str,
        is_tip: bool,
    ) -> stikk_model::Result<stikk_prikk::StateReport> {
        self.inner.block_state(repo, reff, block_id, is_tip)
    }
    fn refs(&self, repo: &std::path::Path) -> stikk_model::Result<stikk_prikk::BranchList> {
        self.inner.refs(repo)
    }
    fn tags(&self, repo: &std::path::Path) -> stikk_model::Result<Vec<stikk_prikk::RefEntry>> {
        self.inner.tags(repo)
    }
    fn worktree_status(
        &self,
        repo: &std::path::Path,
        reff: &str,
    ) -> stikk_model::Result<stikk_prikk::WorktreeStatus> {
        self.inner.worktree_status(repo, reff)
    }
    fn queue(&self, repo: &std::path::Path) -> stikk_model::Result<stikk_prikk::QueueReport> {
        self.inner.queue(repo)
    }
    fn show(
        &self,
        repo: &std::path::Path,
        id: &str,
    ) -> stikk_model::Result<stikk_prikk::ShowReport> {
        self.inner.show(repo, id)
    }
    fn diff(
        &self,
        repo: &std::path::Path,
        comparison: &stikk_prikk::Comparison,
    ) -> stikk_model::Result<stikk_prikk::DiffReport> {
        self.inner.diff(repo, comparison)
    }
    fn verify(&self, repo: &std::path::Path) -> stikk_model::Result<stikk_prikk::VerifyReport> {
        self.inner.verify(repo)
    }
    fn change_token(
        &self,
        repo: &std::path::Path,
    ) -> stikk_model::Result<stikk_model::ChangeToken> {
        self.inner.change_token(repo)
    }
    fn commit(
        &self,
        repo: &std::path::Path,
        reff: &str,
        message: &str,
    ) -> stikk_model::Result<stikk_prikk::CommitResult> {
        self.inner.commit(repo, reff, message)
    }
    fn seal(
        &self,
        repo: &std::path::Path,
        reff: &str,
    ) -> stikk_model::Result<stikk_prikk::SealResult> {
        self.inner.seal(repo, reff)
    }
    fn create_branch(
        &self,
        repo: &std::path::Path,
        name: &str,
        from: &str,
    ) -> stikk_model::Result<stikk_prikk::RefWrite> {
        self.inner.create_branch(repo, name, from)
    }
    fn create_tag(
        &self,
        repo: &std::path::Path,
        name: &str,
        target: &str,
        message: &str,
    ) -> stikk_model::Result<stikk_prikk::RefWrite> {
        self.inner.create_tag(repo, name, target, message)
    }
    fn close_branch(
        &self,
        repo: &std::path::Path,
        name: &str,
    ) -> stikk_model::Result<stikk_prikk::RefWrite> {
        self.inner.close_branch(repo, name)
    }
    fn switch_branch(
        &self,
        repo: &std::path::Path,
        name: &str,
    ) -> stikk_model::Result<stikk_prikk::SwitchResult> {
        self.inner.switch_branch(repo, name)
    }
    fn merge_plan(
        &self,
        repo: &std::path::Path,
        baseline: &str,
        left: &str,
        right: &str,
    ) -> stikk_model::Result<stikk_prikk::MergePlan> {
        self.inner.merge_plan(repo, baseline, left, right)
    }
    fn merge(
        &self,
        repo: &std::path::Path,
        baseline: &str,
        into: &str,
        from: &str,
    ) -> stikk_model::Result<stikk_prikk::MergeResult> {
        self.inner.merge(repo, baseline, into, from)
    }
    fn bundle_verify(
        &self,
        path: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::BundleVerify> {
        self.inner.bundle_verify(path)
    }
    fn bundle_preview(
        &self,
        repo: &std::path::Path,
        path: &std::path::Path,
        local_ref: Option<&str>,
    ) -> stikk_model::Result<stikk_prikk::BundlePreview> {
        self.previews
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.bundle_preview(repo, path, local_ref)
    }
    fn bundle_import(
        &self,
        repo: &std::path::Path,
        path: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::BundleImport> {
        self.imports
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.bundle_import(repo, path)
    }
    fn trusted_keys(
        &self,
        repo: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::TrustedKeys> {
        self.inner.trusted_keys(repo)
    }
    fn trust_add(
        &self,
        repo: &std::path::Path,
        key_id: &str,
        public_key: &str,
    ) -> stikk_model::Result<stikk_prikk::TrustWritten> {
        self.trust_writes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.trust_add(repo, key_id, public_key)
    }
    fn trust_remove(
        &self,
        repo: &std::path::Path,
        key_id: &str,
    ) -> stikk_model::Result<stikk_prikk::TrustWritten> {
        self.trust_writes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.trust_remove(repo, key_id)
    }
    fn file_meta(
        &self,
        repo: &std::path::Path,
        block: &str,
        path: &str,
    ) -> stikk_model::Result<stikk_prikk::FileMetaRead> {
        self.inner.file_meta(repo, block, path)
    }
    fn save_file(
        &self,
        repo: &std::path::Path,
        block: &str,
        path: &str,
        output: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::FileSaved> {
        self.saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.save_file(repo, block, path, output)
    }
    fn extract_plan(
        &self,
        repo: &std::path::Path,
        block: &str,
        dir: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::Extracted> {
        self.inner.extract_plan(repo, block, dir)
    }
    fn extract(
        &self,
        repo: &std::path::Path,
        block: &str,
        dir: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::Extracted> {
        self.extracts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.extract(repo, block, dir)
    }
    fn bundle_export(
        &self,
        repo: &std::path::Path,
        ref_name: &str,
        output: &std::path::Path,
    ) -> stikk_model::Result<stikk_prikk::BundleExport> {
        self.exports
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.bundle_export(repo, ref_name, output)
    }
}

/// **RFC 050 at the ceiling: every connectivity stikk draws a sentence for, from prikk's own
/// report.**
///
/// Four of the six need a receiver that shares the sender's history, which is why this builds
/// one by copying the repository — `measure050b.sh`'s shape, and the only way `fast-forward`,
/// `already-included` and the two `diverged` rows can be reached at all.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_every_connectivity_reads_from_prikks_own_report() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 050 SKIPPED at 0.{} — bundles are banded at 0.{}. Announced rather than silent \
             (RFC 022 §3).",
            bin.minor,
            stikk_core::BUNDLE_FROM_MINOR
        );
        return;
    }
    let sender = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);
    let repo = sender.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    let write = |at: &std::path::Path, name: &str, body: &str| {
        std::fs::write(at.join(name), body)
            .unwrap_or_else(|e| panic!("0.{}: write {name}: {e}", bin.minor));
    };
    let seal = |fixture: &Fixture, at: &std::path::Path, message: &str| {
        fixture.set_author_env();
        backend
            .commit(at, "heads/main", message)
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        fixture.set_maintainer_env();
        backend
            .seal(at, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
    };
    let export = |at: &std::path::Path, to: &std::path::Path| {
        prikk_ok(
            &bin,
            at,
            &[
                "bundle",
                "export",
                "--ref",
                "heads/main",
                "--output",
                &to.display().to_string(),
            ],
        );
    };
    let read = |at: &std::path::Path, file: &std::path::Path| -> stikk_core::BundleView {
        match stikk_core::open_bundle(&backend, at, file) {
            Ok(stikk_core::BundleOpened::Read(view)) => *view,
            other => panic!("0.{}: expected a read bundle, got {other:?}", bin.minor),
        }
    };

    // ── the sender at v1, and a receiver that is a copy of it ────────────────────────────────
    write(&repo, "a.txt", "a\n");
    write(&repo, "b.txt", "b\n");
    seal(&sender, &repo, "base");
    let v1 = root.join("v1.bundle");
    export(&repo, &v1);

    let receiver = root.join("receiver");
    copy_tree(&repo, &receiver);

    write(&repo, "a.txt", "a2\n");
    seal(&sender, &repo, "sender edits a");
    let v2 = root.join("v2.bundle");
    export(&repo, &v2);

    // **fast-forward**: the receiver is at the bundle's parent block.
    let view = read(&receiver, &v2);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::FastForward,
        "0.{}: {view:?}",
        bin.minor
    );
    assert_eq!(view.sentence, stikk_core::WOULD_BRING_FORWARD);
    assert!(
        view.verified
            .iter()
            .any(|line| line.contains("not yet a trusted one")),
        "0.{}: prikk's caveat travels with the verification: {:?}",
        bin.minor,
        view.verified
    );
    assert!(
        view.preview.sealed_by_note.contains("not a trust decision"),
        "0.{}: {}",
        bin.minor,
        view.preview.sealed_by_note
    );

    // **already-included**: the same pair the other way round. prikk reports `conflict: null`
    // here, which is why the field is read as nullable rather than as a fourth word.
    let view = read(&receiver, &v1);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::AlreadyIncluded
    );
    assert_eq!(view.preview.conflict, None, "0.{}: {view:?}", bin.minor);
    assert_eq!(view.sentence, stikk_core::ALREADY_HELD);

    // **diverged, applying cleanly**: each side edited a different file.
    write(&receiver, "b.txt", "b-receiver\n");
    seal(&sender, &receiver, "receiver edits b");
    let view = read(&receiver, &v2);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::Diverged
    );
    assert_eq!(
        view.preview.conflict,
        Some(stikk_prikk::BundleConflict::AppliesCleanly)
    );
    assert_eq!(view.sentence, stikk_core::DIVERGED_CLEAN);
    assert_eq!(view.conflict_detail(), None);

    // **diverged, in conflict**: both sides edited the same file. prikk's replay error is
    // carried verbatim, and stikk's sentence says only that there is one.
    write(&receiver, "a.txt", "a-receiver\n");
    seal(&sender, &receiver, "receiver edits a");
    let view = read(&receiver, &v2);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::Diverged
    );
    assert_eq!(
        view.preview.conflict,
        Some(stikk_prikk::BundleConflict::Conflict)
    );
    assert_eq!(view.sentence, stikk_core::DIVERGED_CONFLICT);
    let detail = view
        .conflict_detail()
        .unwrap_or_else(|| panic!("0.{}: a conflict carries prikk's reason", bin.minor));
    assert!(!detail.is_empty(), "0.{}: {detail}", bin.minor);

    // **no-local-history**: a repository with nothing sealed on that ref.
    let fresh = Fixture::build(&bin);
    let view = read(fresh.repo(), &v1);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::NoLocalHistory
    );
    assert_eq!(view.sentence, stikk_core::NEW_TO_THIS_REPOSITORY);
    assert!(
        !view.preview.effects.is_empty(),
        "0.{}: everything the bundle holds is new here",
        bin.minor
    );

    // **does-not-connect**: an independent repository, with its own history of the same name.
    let other = Fixture::build(&bin);
    write(other.repo(), "a.txt", "a\n");
    write(other.repo(), "b.txt", "b\n");
    seal(&other, other.repo(), "an independent base");
    let view = read(other.repo(), &v1);
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::DoesNotConnect
    );
    assert_eq!(view.sentence, stikk_core::SHARES_NOTHING);
}

/// **A bundle prikk will not read is prikk's refusal, and no preview is spawned** — proved
/// against the real binary with a counting wrapper, not with a fake.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_a_truncated_bundle_is_refused_and_never_previewed() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 050 SKIPPED at 0.{} — bundles are banded at 0.{}.",
            bin.minor,
            stikk_core::BUNDLE_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write a.txt: {e}", bin.minor));
    fixture.set_author_env();
    counting
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    counting
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    let whole = root.join("whole.bundle");
    prikk_ok(
        &bin,
        &repo,
        &[
            "bundle",
            "export",
            "--ref",
            "heads/main",
            "--output",
            &whole.display().to_string(),
        ],
    );
    // A real truncation, not an invented file: the first 200 bytes of a real bundle, which is
    // what `measure050.sh` cut and what prikk answered *"unexpected end of record"* to.
    let bytes =
        std::fs::read(&whole).unwrap_or_else(|e| panic!("0.{}: read the bundle: {e}", bin.minor));
    let cut = root.join("truncated.bundle");
    std::fs::write(&cut, &bytes[..200.min(bytes.len())])
        .unwrap_or_else(|e| panic!("0.{}: write the truncated bundle: {e}", bin.minor));

    let opened = stikk_core::open_bundle(&counting, &repo, &cut)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Refused { path, message } = opened else {
        panic!(
            "0.{}: a truncated bundle is refused, got {opened:?}",
            bin.minor
        );
    };
    assert!(
        message.contains("malformed persisted data"),
        "0.{}: prikk's words, kept: {message}",
        bin.minor
    );
    // **prikk names no file in this refusal**, which is why stikk carries the path itself
    // (review v1 C1) -- measured here rather than assumed, against the real binary.
    assert!(
        !message.contains("truncated.bundle"),
        "0.{}: prikk named the file after all; the screen's path would be a duplicate: {message}",
        bin.minor
    );
    assert_eq!(path, cut.display().to_string(), "0.{}", bin.minor);
    assert_eq!(
        counting.previews.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "0.{}: a file prikk will not read has nothing to preview",
        bin.minor
    );

    // The whole bundle, through the same wrapper, **is** previewed — so the assertion above is
    // about the refusal and not about a path that never previews anything.
    let opened = stikk_core::open_bundle(&counting, &repo, &whole)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    assert!(
        matches!(opened, stikk_core::BundleOpened::Read(_)),
        "0.{}: {opened:?}",
        bin.minor
    );
    assert_eq!(
        counting.previews.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "0.{}",
        bin.minor
    );
}

/// **Below the band nothing is spawned**, and *unavailable* is not *failed*.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_below_the_band_a_bundle_is_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    assert!(
        bin.minor < stikk_core::BUNDLE_FROM_MINOR,
        "the floor is below the band"
    );
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = fixture.repo().to_path_buf();
    let nowhere = repo.join("not-a-bundle.bundle");

    let verified = counting
        .bundle_verify(&nowhere)
        .unwrap_or_else(|e| panic!("0.{}: bundle_verify: {e}", bin.minor));
    assert_eq!(
        verified,
        stikk_prikk::BundleVerify::Unavailable,
        "0.{}: nothing is spawned below the band",
        bin.minor
    );
    let previewed = counting
        .bundle_preview(&repo, &nowhere, None)
        .unwrap_or_else(|e| panic!("0.{}: bundle_preview: {e}", bin.minor));
    assert_eq!(previewed, stikk_prikk::BundlePreview::Unavailable);

    // **And the operation says so in stikk's own voice**, naming the two numbers — never a
    // refusal, and never a file prikk was asked about.
    let opened = stikk_core::open_bundle(&counting, &repo, &nowhere)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Unavailable(message) = opened else {
        panic!(
            "0.{}: below the band is unavailable, got {opened:?}",
            bin.minor
        );
    };
    assert!(
        message.contains(&format!("0.{}", stikk_core::BUNDLE_FROM_MINOR)),
        "0.{}: {message}",
        bin.minor
    );
    assert!(
        message.contains(&format!("0.{}", bin.minor)),
        "0.{}: {message}",
        bin.minor
    );
}

/// **RFC 050 Handoff B at the ceiling: an import lands, and the two guards decline without
/// importing.**
///
/// The load-bearing assertion is the older-bundle one: F2 measured prikk moving
/// `remotes/heads/main` **back** on an ordinary import, so the leg imports `v2`, reads the
/// received tip with prikk's own `log`, offers `v1`, and asserts both that stikk declined and
/// that the tip prikk reports is unchanged.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_an_import_lands_and_an_older_bundle_is_declined() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 050 SKIPPED at 0.{} — bundles are banded at 0.{}.",
            bin.minor,
            stikk_core::BUNDLE_FROM_MINOR
        );
        return;
    }
    let sender = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = sender.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();
    let limit = stikk_core::BUNDLE_LINEAGE_LIMIT;

    let seal = |at: &std::path::Path, message: &str| {
        sender.set_author_env();
        counting
            .commit(at, "heads/main", message)
            .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
        sender.set_maintainer_env();
        counting
            .seal(at, "heads/main")
            .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
        Fixture::clear_env();
    };
    let export = |to: &std::path::Path| {
        prikk_ok(
            &bin,
            &repo,
            &[
                "bundle",
                "export",
                "--ref",
                "heads/main",
                "--output",
                &to.display().to_string(),
            ],
        );
    };
    // prikk's own answer for where the received branch points.
    let received_tip = |at: &std::path::Path| -> String {
        counting
            .history(at, "remotes/heads/main", 1)
            .unwrap_or_else(|e| panic!("0.{}: log remotes/heads/main: {e}", bin.minor))
            .blocks
            .first()
            .unwrap_or_else(|| panic!("0.{}: the received branch has a tip", bin.minor))
            .block_id
            .clone()
    };

    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    seal(&repo, "base");
    let v1 = root.join("v1.bundle");
    export(&v1);
    std::fs::write(repo.join("a.txt"), "a2\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    seal(&repo, "sender edits a");
    let v2 = root.join("v2.bundle");
    export(&v2);

    // ── the receiver: a repository of its own, with no history of that name ─────────────────
    let receiver = Fixture::build(&bin);
    let there = receiver.repo().to_path_buf();
    let run = |file: &std::path::Path| -> stikk_core::ImportPreview {
        stikk_core::import_preview(&counting, &there, file, limit)
            .unwrap_or_else(|e| panic!("0.{}: import_preview: {e}", bin.minor))
    };

    // **v2 imports**: nothing received yet, so there is nothing to move back.
    let stikk_core::ImportPreview::Ready { token } = run(&v2) else {
        panic!("0.{}: a first import arms", bin.minor);
    };
    let target = stikk_core::ImportTarget {
        path: v2.clone(),
        received: "remotes/heads/main".to_string(),
    };
    let outcome = stikk_core::import_confirm_and_execute(
        &counting,
        &there,
        *token,
        counting
            .readiness(&there)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &target,
    )
    .unwrap_or_else(|e| panic!("0.{}: import: {e}", bin.minor));
    let stikk_prikk::BundleImport::Done(printed) = outcome.result else {
        panic!("0.{}: the import ran", bin.minor);
    };
    assert!(
        printed
            .iter()
            .any(|line| line.contains("no local ref was created or advanced")),
        "0.{}: prikk's note travels: {printed:?}",
        bin.minor
    );
    // `branch list` shows it, and its history reads.
    let refs = counting
        .refs(&there)
        .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor));
    assert!(
        refs.refs
            .iter()
            .any(|entry| entry.name == "remotes/heads/main" && entry.received),
        "0.{}: the received ref is listed: {:?}",
        bin.minor,
        refs.refs.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
    let after_v2 = received_tip(&there);

    // **The guards, from here on.** Every one of them must leave `bundle import` unspawned.
    let imports_before = counting.imports.load(std::sync::atomic::Ordering::SeqCst);

    // Guard 2: v1 is older than what was received.
    match run(&v1) {
        stikk_core::ImportPreview::Blocked(reason) => assert_eq!(
            reason,
            stikk_core::WOULD_MOVE_THE_RECEIVED_BRANCH_BACK,
            "0.{}",
            bin.minor
        ),
        other => panic!(
            "0.{}: an older bundle is declined, got {other:?}",
            bin.minor
        ),
    }
    // Guard 1: v2 again is already received.
    match run(&v2) {
        stikk_core::ImportPreview::Blocked(reason) => {
            assert_eq!(reason, stikk_core::ALREADY_RECEIVED, "0.{}", bin.minor);
        }
        other => panic!(
            "0.{}: the same bundle twice is declined, got {other:?}",
            bin.minor
        ),
    }
    assert_eq!(
        counting.imports.load(std::sync::atomic::Ordering::SeqCst),
        imports_before,
        "0.{}: neither guard spawned `bundle import`",
        bin.minor
    );
    // **And prikk's own `log` agrees the received branch did not move.**
    assert_eq!(
        received_tip(&there),
        after_v2,
        "0.{}: the received tip is where the import left it",
        bin.minor
    );
}

/// **RFC 050 Handoff B at the ceiling: the two export rules, and a round trip.**
///
/// A declined export must leave the filesystem exactly as it was — so the worktree is hashed
/// before and after, and the file in the way is read back byte for byte.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_an_export_refuses_the_two_destinations_and_round_trips_otherwise() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 050 SKIPPED at 0.{} — bundles are banded at 0.{}.",
            bin.minor,
            stikk_core::BUNDLE_FROM_MINOR
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    fixture.set_author_env();
    counting
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    counting
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    /// Every file in the worktree with its bytes, so a refusal can be proved to have written
    /// nothing at all -- not merely no bundle.
    fn worktree(at: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![at.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().is_some_and(|n| n == ".prikk") {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    out.push((path.display().to_string(), bytes));
                }
            }
        }
        out.sort();
        out
    }

    let before = worktree(&repo);
    let exports_before = counting.exports.load(std::sync::atomic::Ordering::SeqCst);

    // **Inside the repository: declined, and the worktree is byte-identical.**
    let inside = repo.join("out.bundle");
    match stikk_core::export_preview(&counting, &repo, "heads/main", &inside)
        .unwrap_or_else(|e| panic!("0.{}: export_preview: {e}", bin.minor))
    {
        stikk_core::ExportPreview::Blocked(reason) => {
            assert_eq!(reason, stikk_core::INSIDE_THE_REPOSITORY, "0.{}", bin.minor);
        }
        other => panic!("0.{}: inside is declined, got {other:?}", bin.minor),
    }
    assert!(!inside.exists(), "0.{}: nothing was written", bin.minor);
    assert_eq!(
        worktree(&repo),
        before,
        "0.{}: the worktree is untouched",
        bin.minor
    );

    // **An existing destination: declined, and the file in the way is untouched.**
    let taken = root.join("taken.bundle");
    std::fs::write(&taken, b"not a bundle")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    match stikk_core::export_preview(&counting, &repo, "heads/main", &taken)
        .unwrap_or_else(|e| panic!("0.{}: export_preview: {e}", bin.minor))
    {
        stikk_core::ExportPreview::Blocked(reason) => {
            assert_eq!(reason, stikk_core::DESTINATION_EXISTS, "0.{}", bin.minor);
        }
        other => panic!(
            "0.{}: an existing file is declined, got {other:?}",
            bin.minor
        ),
    }
    assert_eq!(
        std::fs::read(&taken).unwrap_or_default(),
        b"not a bundle",
        "0.{}: the file in the way is byte-identical",
        bin.minor
    );
    assert_eq!(
        counting.exports.load(std::sync::atomic::Ordering::SeqCst),
        exports_before,
        "0.{}: neither rule spawned `bundle export`",
        bin.minor
    );

    // **The round trip**: stikk exports, prikk verifies, and a second repository imports it.
    let out = root.join("main.bundle");
    let target = stikk_core::ExportTarget {
        reff: "heads/main".to_string(),
        output: out.clone(),
    };
    let stikk_core::ExportPreview::Ready { token } =
        stikk_core::export_preview(&counting, &repo, "heads/main", &out)
            .unwrap_or_else(|e| panic!("0.{}: export_preview: {e}", bin.minor))
    else {
        panic!("0.{}: a clean destination exports", bin.minor);
    };
    stikk_core::export_confirm_and_execute(
        &counting,
        &repo,
        *token,
        counting
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &target,
    )
    .unwrap_or_else(|e| panic!("0.{}: export: {e}", bin.minor));
    assert!(out.exists(), "0.{}: the bundle is written", bin.minor);
    assert_eq!(
        worktree(&repo),
        before,
        "0.{}: an export changes nothing in the repository",
        bin.minor
    );

    let second = Fixture::build(&bin);
    let opened = stikk_core::open_bundle(&counting, second.repo(), &out)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Read(view) = opened else {
        panic!(
            "0.{}: the exported file verifies, got {opened:?}",
            bin.minor
        );
    };
    assert_eq!(
        view.preview.connectivity,
        stikk_prikk::Connectivity::NoLocalHistory
    );
    let stikk_core::ImportPreview::Ready { token } = stikk_core::import_preview(
        &counting,
        second.repo(),
        &out,
        stikk_core::BUNDLE_LINEAGE_LIMIT,
    )
    .unwrap_or_else(|e| panic!("0.{}: import_preview: {e}", bin.minor)) else {
        panic!("0.{}: the exported file imports", bin.minor);
    };
    stikk_core::import_confirm_and_execute(
        &counting,
        second.repo(),
        *token,
        counting
            .readiness(second.repo())
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &stikk_core::ImportTarget {
            path: out,
            received: "remotes/heads/main".to_string(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: import: {e}", bin.minor));
    let refs = counting
        .refs(second.repo())
        .unwrap_or_else(|e| panic!("0.{}: refs: {e}", bin.minor));
    assert!(
        refs.refs
            .iter()
            .any(|entry| entry.name == "remotes/heads/main"),
        "0.{}: what stikk exported, stikk imported",
        bin.minor
    );
}

/// **Below the band both writes are unavailable**, and nothing is spawned.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc050_below_the_band_the_bundle_writes_are_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    assert!(
        bin.minor < stikk_core::BUNDLE_FROM_MINOR,
        "the floor is below the band"
    );
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = fixture.repo().to_path_buf();
    let nowhere = repo.join("nothing.bundle");

    assert_eq!(
        counting
            .bundle_import(&repo, &nowhere)
            .unwrap_or_else(|e| panic!("0.{}: bundle_import: {e}", bin.minor)),
        stikk_prikk::BundleImport::Unavailable,
        "0.{}",
        bin.minor
    );
    assert_eq!(
        counting
            .bundle_export(&repo, "heads/main", &nowhere)
            .unwrap_or_else(|e| panic!("0.{}: bundle_export: {e}", bin.minor)),
        stikk_prikk::BundleExport::Unavailable,
        "0.{}",
        bin.minor
    );
    assert!(
        !nowhere.exists(),
        "0.{}: nothing was written below the band",
        bin.minor
    );
}

// =============================================================================================
// RFC 053 — getting files back, against a real prikk.
// =============================================================================================

/// A fixture with four files at its first block, and that block's id.
///
/// `a.txt`, `bin.dat`, `dir/sub/d.txt` and an executable `run.sh` — the shape
/// `measure053.sh` built, so the counts these legs assert are the measured ones.
fn rfc053_fixture(bin: &PrikkBin, fixture: &Fixture, counting: &CountingWrites) -> String {
    let repo = fixture.repo().to_path_buf();
    std::fs::create_dir_all(repo.join("dir/sub"))
        .unwrap_or_else(|e| panic!("0.{}: mkdir: {e}", bin.minor));
    std::fs::write(repo.join("a.txt"), "one\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    std::fs::write(repo.join("dir/sub/d.txt"), "deep\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    std::fs::write(repo.join("bin.dat"), [0u8, 1, 2])
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    std::fs::write(repo.join("run.sh"), "#!/bin/sh\necho hi\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(repo.join("run.sh"), std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("0.{}: chmod: {e}", bin.minor));
    }
    fixture.set_author_env();
    counting
        .commit(&repo, "heads/main", "first")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    counting
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();
    counting
        .history(&repo, "heads/main", 1)
        .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor))
        .blocks
        .first()
        .unwrap_or_else(|| panic!("0.{}: the sealed block", bin.minor))
        .block_id
        .clone()
}

/// Whether this ceiling offers the saves at all.
fn rfc053_in_band(bin: &PrikkBin) -> bool {
    if bin.minor < stikk_core::FILES_FROM_MINOR {
        eprintln!(
            "RFC 053 SKIPPED at 0.{} — saving files is banded at 0.{}.",
            bin.minor,
            stikk_core::FILES_FROM_MINOR
        );
        return false;
    }
    true
}

/// **Leg 1: one file saved, and its bytes are `cat`'s own.**
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_one_file_is_saved_with_prikks_own_bytes_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) {
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    let out = root.join("a-saved.txt");
    let stikk_core::SaveFilePreview::Ready { token } =
        stikk_core::save_file_preview(&counting, &repo, &block, "a.txt", &out)
            .unwrap_or_else(|e| panic!("0.{}: save_file_preview: {e}", bin.minor))
    else {
        panic!("0.{}: a file outside the repository is offered", bin.minor);
    };
    stikk_core::save_file_confirm_and_execute(
        &counting,
        &repo,
        *token,
        counting
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &stikk_core::SaveFileTarget {
            block: block.clone(),
            path: "a.txt".to_string(),
            output: out.clone(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: save_file: {e}", bin.minor));

    // **prikk's own answer for the same point**, written beside it and compared byte for byte.
    let theirs = root.join("a-prikk.txt");
    prikk_ok(
        &bin,
        &repo,
        &[
            "cat",
            "--path",
            "a.txt",
            "--ref",
            &block,
            "--output",
            &theirs.display().to_string(),
        ],
    );
    assert_eq!(
        std::fs::read(&out).unwrap_or_else(|e| panic!("0.{}: read ours: {e}", bin.minor)),
        std::fs::read(&theirs).unwrap_or_else(|e| panic!("0.{}: read prikk's: {e}", bin.minor)),
        "0.{}: stikk saved exactly what prikk saves",
        bin.minor
    );
}

/// **Leg 2: every file extracted, and the set matches `tree`'s.**
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_every_file_is_saved_and_matches_prikks_tree_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) {
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    let dir = root.join("all-files");
    let stikk_core::SaveAllPreview::Ready { token } =
        stikk_core::save_all_preview(&counting, &repo, &block, &dir)
            .unwrap_or_else(|e| panic!("0.{}: save_all_preview: {e}", bin.minor))
    else {
        panic!(
            "0.{}: a folder outside the repository is offered",
            bin.minor
        );
    };
    stikk_core::save_all_confirm_and_execute(
        &counting,
        &repo,
        *token,
        counting
            .readiness(&repo)
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &stikk_core::SaveAllTarget {
            block: block.clone(),
            dir: dir.clone(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: extract: {e}", bin.minor));

    // **The paths and sizes stikk wrote, against prikk's own `tree` at that block.** Read from
    // prikk directly rather than through the seam: `StateFile` carries no size, and the point of
    // this leg is to compare what landed on disk with what prikk says is there.
    let tree = prikk_ok(&bin, &repo, &["tree", "--ref", &block, "--format", "json"]);
    let tree = String::from_utf8_lossy(&tree.stdout).into_owned();
    let mut wanted: Vec<(String, u64)> = Vec::new();
    for entry in tree.split("\"path\"").skip(1) {
        let path = entry
            .split('"')
            .nth(1)
            .unwrap_or_else(|| panic!("0.{}: a path in prikk's tree: {entry}", bin.minor))
            .to_string();
        let size = entry
            .split("\"size\"")
            .nth(1)
            .and_then(|rest| {
                rest.trim_start_matches([':', ' '])
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
            })
            .and_then(|digits| digits.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("0.{}: a size in prikk's tree: {entry}", bin.minor));
        wanted.push((path, size));
    }
    // **However many prikk lists** -- the fixture builder adds files of its own, and pinning a
    // count here would be asserting the harness rather than the comparison this leg is for.
    assert!(
        wanted.len() >= 4,
        "0.{}: prikk lists at least this leg's four files: {wanted:?}",
        bin.minor
    );
    wanted.sort();

    let mut written: Vec<(String, u64)> = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at)
            .unwrap_or_else(|e| panic!("0.{}: read_dir: {e}", bin.minor))
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                // **`/`, whatever the platform writes** — prikk reports repository paths with
                // forward slashes everywhere, and `Path::display` uses the platform's own
                // separator. Without this the comparison passes on Linux and macOS and fails on
                // Windows with `dir\sub\d.txt` against `dir/sub/d.txt`, which is what the
                // first full-matrix run found: the files were in the right places with the right
                // sizes, and only this test's spelling of them differed.
                let relative = path
                    .strip_prefix(&dir)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                let size = std::fs::metadata(&path)
                    .unwrap_or_else(|e| panic!("0.{}: stat: {e}", bin.minor))
                    .len();
                written.push((relative, size));
            }
        }
    }
    written.sort();
    assert_eq!(
        written, wanted,
        "0.{}: what stikk wrote is what prikk lists at that block",
        bin.minor
    );
}

/// **Leg 3: a destination inside the repository is declined, nothing is spawned, and the
/// worktree is unchanged** — for `F` and for `A` (RFC 053 §7 trap 5).
///
/// prikk 0.54 would have written both there; 0.55 refuses only the `extract`. stikk's own rule
/// covers both, and this proves it never reached prikk at all.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_a_destination_inside_the_repository_is_declined_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) {
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();

    let before = counting
        .worktree_status(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: worktree-status: {e}", bin.minor));
    let saves_before = counting.saves.load(std::sync::atomic::Ordering::SeqCst);
    let extracts_before = counting.extracts.load(std::sync::atomic::Ordering::SeqCst);

    for inside in [repo.join("a-copy.txt"), repo.join("dir/a-copy.txt")] {
        match stikk_core::save_file_preview(&counting, &repo, &block, "a.txt", &inside)
            .unwrap_or_else(|e| panic!("0.{}: save_file_preview: {e}", bin.minor))
        {
            stikk_core::SaveFilePreview::Blocked(reason) => assert_eq!(
                reason,
                stikk_core::FILES_INSIDE_THE_REPOSITORY,
                "0.{}",
                bin.minor
            ),
            other => panic!("0.{}: inside is declined, got {other:?}", bin.minor),
        }
        match stikk_core::save_all_preview(&counting, &repo, &block, &inside)
            .unwrap_or_else(|e| panic!("0.{}: save_all_preview: {e}", bin.minor))
        {
            stikk_core::SaveAllPreview::Blocked(reason) => assert_eq!(
                reason,
                stikk_core::FILES_INSIDE_THE_REPOSITORY,
                "0.{}",
                bin.minor
            ),
            other => panic!("0.{}: inside is declined, got {other:?}", bin.minor),
        }
        assert!(
            !inside.exists(),
            "0.{}: nothing was written at {}",
            bin.minor,
            inside.display()
        );
    }

    assert_eq!(
        counting.saves.load(std::sync::atomic::Ordering::SeqCst),
        saves_before,
        "0.{}: `cat --output` was never spawned",
        bin.minor
    );
    assert_eq!(
        counting.extracts.load(std::sync::atomic::Ordering::SeqCst),
        extracts_before,
        "0.{}: `extract` was never spawned",
        bin.minor
    );
    // **The worktree is what it was** — not merely "no copy": an untracked file anywhere would
    // show here, and the next commit would have captured it.
    let after = counting
        .worktree_status(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: worktree-status: {e}", bin.minor));
    assert_eq!(
        after.entries.len(),
        before.entries.len(),
        "0.{}: the worktree is unchanged",
        bin.minor
    );
    assert_eq!(after.untracked, before.untracked, "0.{}", bin.minor);
}

/// **Leg 4: an existing destination is declined, and its bytes are untouched.**
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_an_existing_destination_is_declined_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) {
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    let taken = root.join("already-here.txt");
    std::fs::write(&taken, b"not mine to overwrite")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    let taken_dir = root.join("already-a-folder");
    std::fs::create_dir_all(&taken_dir).unwrap_or_else(|e| panic!("0.{}: mkdir: {e}", bin.minor));

    let saves_before = counting.saves.load(std::sync::atomic::Ordering::SeqCst);
    let extracts_before = counting.extracts.load(std::sync::atomic::Ordering::SeqCst);

    match stikk_core::save_file_preview(&counting, &repo, &block, "a.txt", &taken)
        .unwrap_or_else(|e| panic!("0.{}: save_file_preview: {e}", bin.minor))
    {
        stikk_core::SaveFilePreview::Blocked(reason) => {
            assert_eq!(
                reason,
                stikk_core::FILES_DESTINATION_EXISTS,
                "0.{}",
                bin.minor
            );
        }
        other => panic!(
            "0.{}: an existing file is declined, got {other:?}",
            bin.minor
        ),
    }
    match stikk_core::save_all_preview(&counting, &repo, &block, &taken_dir)
        .unwrap_or_else(|e| panic!("0.{}: save_all_preview: {e}", bin.minor))
    {
        stikk_core::SaveAllPreview::Blocked(reason) => {
            assert_eq!(
                reason,
                stikk_core::FILES_DESTINATION_EXISTS,
                "0.{}",
                bin.minor
            );
        }
        other => panic!(
            "0.{}: an existing directory is declined, got {other:?}",
            bin.minor
        ),
    }
    assert_eq!(
        std::fs::read(&taken).unwrap_or_else(|e| panic!("0.{}: read: {e}", bin.minor)),
        b"not mine to overwrite",
        "0.{}: the file in the way is untouched",
        bin.minor
    );
    assert_eq!(
        counting.saves.load(std::sync::atomic::Ordering::SeqCst),
        saves_before,
        "0.{}: nothing spawned",
        bin.minor
    );
    assert_eq!(
        counting.extracts.load(std::sync::atomic::Ordering::SeqCst),
        extracts_before,
        "0.{}: nothing spawned",
        bin.minor
    );
}

/// **Leg 5: an executable file's card carries the notice** — from prikk's own `mode`, which
/// `cat --output` does not carry to the copy (RFC 053 F3).
#[cfg(unix)]
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_an_executable_files_card_warns_about_the_mode_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) {
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    for (path, wanted) in [("run.sh", true), ("a.txt", false)] {
        let out = root.join(format!("{path}-saved"));
        let stikk_core::SaveFilePreview::Ready { token } =
            stikk_core::save_file_preview(&counting, &repo, &block, path, &out)
                .unwrap_or_else(|e| panic!("0.{}: {path}: {e}", bin.minor))
        else {
            panic!("0.{}: {path} is offered", bin.minor);
        };
        let notices = &token.summary().notices;
        assert_eq!(
            notices.contains(&stikk_core::LOSES_THE_EXECUTE_BIT.to_string()),
            wanted,
            "0.{}: {path}'s card: {notices:?}",
            bin.minor
        );
    }
}

/// **Leg 6: a received branch's plan carries the provisional notice** (RFC 053 F4).
///
/// Built the way RFC 050's legs build one: a second repository imports a bundle this one
/// exported, so its received history has a sealer the receiver has not adopted.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_a_received_branchs_plan_is_provisional_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc053_in_band(&bin) || bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 053 leg 6 SKIPPED at 0.{} — needs bundles too.",
            bin.minor
        );
        return;
    }
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let block = rfc053_fixture(&bin, &fixture, &counting);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();
    let _ = &block;

    let bundle = root.join("shared.bundle");
    prikk_ok(
        &bin,
        &repo,
        &[
            "bundle",
            "export",
            "--ref",
            "heads/main",
            "--output",
            &bundle.display().to_string(),
        ],
    );
    let receiver = Fixture::build(&bin);
    prikk_ok(
        &bin,
        receiver.repo(),
        &["bundle", "import", "--input", &bundle.display().to_string()],
    );

    // The received branch's own tip, which is the point this plan is for.
    let received = counting
        .history(receiver.repo(), "remotes/heads/main", 1)
        .unwrap_or_else(|e| panic!("0.{}: history: {e}", bin.minor))
        .blocks
        .first()
        .unwrap_or_else(|| panic!("0.{}: the received tip", bin.minor))
        .block_id
        .clone();

    let dir = receiver
        .repo()
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the receiver's root", bin.minor))
        .join("received-files");
    let stikk_core::SaveAllPreview::Ready { token } =
        stikk_core::save_all_preview(&counting, receiver.repo(), &received, &dir)
            .unwrap_or_else(|e| panic!("0.{}: save_all_preview: {e}", bin.minor))
    else {
        panic!("0.{}: a received branch's files are offered", bin.minor);
    };
    assert!(
        token
            .summary()
            .notices
            .contains(&stikk_core::PROVISIONAL_REPLAY.to_string()),
        "0.{}: prikk replayed from the start and the card says so: {:?}",
        bin.minor,
        token.summary().notices
    );
}

/// **At the floor both keys are unavailable, and nothing is spawned.**
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc053_below_the_band_both_saves_are_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let counting = CountingWrites::new(&bin);
    let repo = fixture.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();
    let block = "f".repeat(64);

    match stikk_core::save_file_preview(&counting, &repo, &block, "a.txt", &root.join("a-out.txt"))
        .unwrap_or_else(|e| panic!("0.{}: save_file_preview: {e}", bin.minor))
    {
        stikk_core::SaveFilePreview::Blocked(reason) => assert!(
            reason.contains(&format!("0.{}", stikk_core::FILES_FROM_MINOR)),
            "0.{}: the band is named: {reason}",
            bin.minor
        ),
        other => panic!("0.{}: below the band is declined, got {other:?}", bin.minor),
    }
    match stikk_core::save_all_preview(&counting, &repo, &block, &root.join("all-out"))
        .unwrap_or_else(|e| panic!("0.{}: save_all_preview: {e}", bin.minor))
    {
        stikk_core::SaveAllPreview::Blocked(reason) => assert!(
            reason.contains(&format!("0.{}", stikk_core::FILES_FROM_MINOR)),
            "0.{}: {reason}",
            bin.minor
        ),
        other => panic!("0.{}: below the band is declined, got {other:?}", bin.minor),
    }
    assert_eq!(
        counting.saves.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "0.{}: nothing spawned below the band",
        bin.minor
    );
    assert_eq!(
        counting.extracts.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "0.{}: nothing spawned below the band",
        bin.minor
    );
}

// =============================================================================================
// RFC 054 — trust, read-only, against a real prikk.
// =============================================================================================

/// Whether this ceiling lists adopted keys at all.
fn rfc054_in_band(bin: &PrikkBin) -> bool {
    if bin.minor < stikk_core::TRUST_FROM_MINOR {
        eprintln!(
            "RFC 054 SKIPPED at 0.{} — the trust list is banded at 0.{}.",
            bin.minor,
            stikk_core::TRUST_FROM_MINOR
        );
        return false;
    }
    true
}

/// A well-formed public key and the `ed25519-<16 hex>` id prikk derives from it, from one hex
/// digit — enough to adopt, and distinguishable from any other.
fn rfc054_key(seed_digit: char) -> (String, String) {
    let public: String = std::iter::repeat_n(seed_digit, 64).collect();
    let id = format!("ed25519-{}", &public[..16]);
    (id, public)
}

/// **Legs 1–3: a bare repository lists nothing; after `setup` one row is yours; a second
/// adopted key is a second row, unmarked.**
///
/// One test for three states of one repository: splitting them would build three fixtures to
/// assert what one progression shows.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc054_the_trust_list_grows_and_only_your_own_key_is_marked_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc054_in_band(&bin) {
        return;
    }
    let backend = CliBackend::with_program(&bin.path);

    // **Leg 1: `init` alone.** `Fixture::build` runs `setup`, so a bare repository is made here.
    let bare = std::env::temp_dir().join(format!(
        "stikk-rfc054-bare-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).unwrap_or_else(|e| panic!("0.{}: mkdir: {e}", bin.minor));
    prikk_ok(&bin, &bare, &["init", "."]);
    match stikk_core::trust::read(&backend, &bare)
        .unwrap_or_else(|e| panic!("0.{}: trust read: {e}", bin.minor))
    {
        stikk_core::TrustView::Listed { rows, .. } => assert!(
            rows.is_empty(),
            "0.{}: a repository made with `init` alone has adopted nothing: {rows:?}",
            bin.minor
        ),
        other => panic!("0.{}: expected a list, got {other:?}", bin.minor),
    }
    let _ = std::fs::remove_dir_all(&bare);

    // **Leg 2: after `setup`.** One row, and it is yours — `setup` adopts the maintainer key it
    // creates (RFC 054 F3), and `key status` names the same pair.
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    fixture.set_maintainer_env();
    let view = stikk_core::trust::read(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: trust read: {e}", bin.minor));
    let stikk_core::TrustView::Listed { rows, .. } = view else {
        panic!("0.{}: expected a list", bin.minor);
    };
    assert_eq!(rows.len(), 1, "0.{}: setup adopted one key", bin.minor);
    assert!(
        rows[0].yours,
        "0.{}: and it is the key `key status` names: {rows:?}",
        bin.minor
    );
    let mine = rows[0].key_id.clone();

    // **Leg 3: a second key, adopted in the harness.** Two rows, and the second is not yours.
    let (theirs_id, theirs_key) = rfc054_key('7');
    prikk_ok(
        &bin,
        &repo,
        &[
            "trust",
            "maintainer",
            "add",
            "--key-id",
            &theirs_id,
            "--public-key",
            &theirs_key,
        ],
    );
    let view = stikk_core::trust::read(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: trust read: {e}", bin.minor));
    let stikk_core::TrustView::Listed { rows, .. } = view else {
        panic!("0.{}: expected a list", bin.minor);
    };
    assert_eq!(rows.len(), 2, "0.{}: {rows:?}", bin.minor);
    assert_eq!(
        rows.iter().filter(|r| r.yours).count(),
        1,
        "0.{}: exactly one row is yours: {rows:?}",
        bin.minor
    );
    assert!(
        rows.iter().any(|r| r.key_id == mine && r.yours),
        "0.{}: and it is still the one `setup` made",
        bin.minor
    );
    assert!(
        rows.iter().any(|r| r.key_id == theirs_id && !r.yours),
        "0.{}: the collaborator's is not: {rows:?}",
        bin.minor
    );
    Fixture::clear_env();
}

/// **Leg 4: a bundle's sealer is *not trusted* in a receiver that has not adopted it, and
/// *trusted* once the harness does.**
///
/// The receiver has adopted nothing at all, so there is no id to collide with — the plain case
/// decision 3 describes. The case where a sealer's id **is** one the receiver has adopted, for
/// another key, is
/// [`rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import`].
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc054_a_bundles_sealer_becomes_trusted_when_the_key_is_adopted_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc054_in_band(&bin) || bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!(
            "RFC 054 leg 4 SKIPPED at 0.{} — needs bundles too.",
            bin.minor
        );
        return;
    }
    let backend = CliBackend::with_program(&bin.path);
    let sender = Fixture::build(&bin);
    let repo = sender.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    sender.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    sender.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    let bundle = root.join("sealed.bundle");
    prikk_ok(
        &bin,
        &repo,
        &[
            "bundle",
            "export",
            "--ref",
            "heads/main",
            "--output",
            &bundle.display().to_string(),
        ],
    );

    // **A receiver made with `init` alone**, so it has adopted nothing at all — the honest
    // shape of meeting a stranger's bundle. A `setup` receiver would hold its own key under
    // the harness's `maintainer` id (`support.rs`, `MAINTAINER_KEY_ID`) and collide with the
    // sender's, which is
    // `rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import`'s subject
    // rather than this one's.
    let receiver_dir = root.join("receiver");
    let _ = std::fs::remove_dir_all(&receiver_dir);
    std::fs::create_dir_all(&receiver_dir)
        .unwrap_or_else(|e| panic!("0.{}: mkdir: {e}", bin.minor));
    prikk_ok(&bin, &receiver_dir, &["init", "."]);
    let receiver = receiver_dir.as_path();
    let opened = stikk_core::open_bundle(&backend, receiver, &bundle)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Read(view) = opened else {
        panic!("0.{}: the bundle verifies, got {opened:?}", bin.minor);
    };
    let judged = view.sealers.judged().to_vec();
    assert!(
        !judged.is_empty(),
        "0.{}: the bundle records a sealer",
        bin.minor
    );
    assert!(
        judged
            .iter()
            .all(|s| s.trust == stikk_core::SealerTrust::NotTrusted),
        "0.{}: the receiver adopted nothing of the sender's: {judged:?}",
        bin.minor
    );
    let sealer = judged[0].key_id.clone();

    // **The sender's public key, from the sender's own list** — not from the bundle, which is
    // the caution stikk's own empty state gives.
    let stikk_core::TrustView::Listed { rows, .. } = stikk_core::trust::read(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: the sender's list: {e}", bin.minor))
    else {
        panic!("0.{}: the sender lists its own key", bin.minor);
    };
    let sender_key = rows
        .iter()
        .find(|row| row.key_id == sealer)
        .map(|row| row.public_key.clone())
        .unwrap_or_else(|| {
            panic!(
                "0.{}: the sealer {sealer} is in the sender's own list: {rows:?}",
                bin.minor
            )
        });

    prikk_ok(
        &bin,
        receiver,
        &[
            "trust",
            "maintainer",
            "add",
            "--key-id",
            &sealer,
            "--public-key",
            &sender_key,
        ],
    );

    let opened = stikk_core::open_bundle(&backend, receiver, &bundle)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Read(view) = opened else {
        panic!("0.{}: the bundle still verifies", bin.minor);
    };
    assert!(
        view.sealers
            .judged()
            .iter()
            .all(|s| s.trust == stikk_core::SealerTrust::NamesATrustedKey),
        "0.{}: adopted, and not yours: {:?}",
        bin.minor,
        view.sealers
    );
}

/// **The measured case behind decision 3's wording: a sealer id is a name the sealer chose,
/// and only `verify` after import checks the signature behind it.**
///
/// `bundle-preview-v1`'s `sealed_by` is the `key_id` string out of the signature envelope
/// (RFC 054 F4) — ids, no public keys, checked against nothing at preview time. So a bundle
/// sealed under an id the receiver has adopted previews cleanly and imports cleanly, whoever
/// holds the key. The card therefore says the id **names** an adopted key, and carries
/// [`stikk_core::NAMES_BUT_DOES_NOT_PROVE`] beneath the rows.
///
/// **Where the shared id comes from here: the harness, not prikk.** `support.rs`'s
/// `MAINTAINER_KEY_ID` puts every fixture's key under `maintainer`, so two fixtures hold
/// *different* keys under the *same* id. (Since prikk 0.45.0 a key made by `setup` or
/// `key generate --out` gets `ed25519-<16 hex of its public key>`, so two strangers do not
/// normally collide by accident — a sender who wants to can still choose the id.) The shape
/// is the same either way, and it is the architect's measured spoof at 0.55.0:
/// `.git-exclude/reports/054-trust-read/spoof054-0.55.txt`.
///
/// **This leg is the evidence for the sentence stikk shows.** If prikk ever stops catching
/// this at `verify`, it goes red.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc054_in_band(&bin) || bin.minor < stikk_core::BUNDLE_FROM_MINOR {
        eprintln!("RFC 054's naming leg SKIPPED at 0.{}.", bin.minor);
        return;
    }
    let backend = CliBackend::with_program(&bin.path);
    let sender = Fixture::build(&bin);
    let repo = sender.repo().to_path_buf();
    let root = repo
        .parent()
        .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
        .to_path_buf();

    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    sender.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    // **Both fixtures' keys sit under the harness's `maintainer`**, which is the whole point
    // of this leg: the sender seals under an id the receiver has adopted for another key.
    sender.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();

    let bundle = root.join("named-id.bundle");
    prikk_ok(
        &bin,
        &repo,
        &[
            "bundle",
            "export",
            "--ref",
            "heads/main",
            "--output",
            &bundle.display().to_string(),
        ],
    );

    // A receiver with its own `setup` key — a *different* key under the *same* id.
    let receiver = Fixture::build(&bin);
    let opened = stikk_core::open_bundle(&backend, receiver.repo(), &bundle)
        .unwrap_or_else(|e| panic!("0.{}: open_bundle: {e}", bin.minor));
    let stikk_core::BundleOpened::Read(view) = opened else {
        panic!("0.{}: the bundle verifies", bin.minor);
    };
    let judged = view.sealers.judged().to_vec();
    assert_eq!(
        judged.len(),
        1,
        "0.{}: one recorded sealer: {judged:?}",
        bin.minor
    );
    assert_eq!(
        judged[0].key_id, "maintainer",
        "0.{}: the harness's id, shared by both fixtures",
        bin.minor
    );
    // **`NamesATrustedKey`, not `NamesYourKey`**, and the environment is the reason: the keys
    // are cleared above, so `key status` resolves no key of the reader's own and no row in the
    // Trust view is marked *yours*. The id is adopted here all the same — by the receiver's own
    // key, which is not the one that sealed this bundle, and that is exactly the case the
    // sentence beneath the rows is for.
    assert_eq!(
        judged[0].trust,
        stikk_core::SealerTrust::NamesATrustedKey,
        "0.{}: the id names an adopted key, and the card says no more than that",
        bin.minor
    );

    // **The sentence is on the card**, beneath the rows and above prikk's own note.
    assert!(
        view.sealers.names_an_adopted_key(),
        "0.{}: the sentence is owed here",
        bin.minor
    );
    let stikk_core::ImportPreview::Ready { token } = stikk_core::import_preview(
        &backend,
        receiver.repo(),
        &bundle,
        stikk_core::BUNDLE_LINEAGE_LIMIT,
    )
    .unwrap_or_else(|e| panic!("0.{}: import_preview: {e}", bin.minor)) else {
        panic!("0.{}: the bundle imports", bin.minor);
    };
    let summary = token.summary();
    let aside = summary
        .asides
        .iter()
        .find(|aside| aside.body.iter().any(|line| line.contains("maintainer")))
        .unwrap_or_else(|| panic!("0.{}: the sealer aside: {summary:?}", bin.minor));
    assert_eq!(
        aside.body.first().map(String::as_str),
        Some("maintainer — names a key this repository trusts"),
        "0.{}: the row: {:?}",
        bin.minor,
        aside.body
    );
    assert_eq!(
        aside.body.get(1).map(String::as_str),
        Some(stikk_core::NAMES_BUT_DOES_NOT_PROVE),
        "0.{}: what a name is worth, above prikk's note: {:?}",
        bin.minor,
        aside.body
    );

    // **Import, and then ask prikk.** This is the step the sentence sends the reader to.
    stikk_core::import_confirm_and_execute(
        &backend,
        receiver.repo(),
        *token,
        backend
            .readiness(receiver.repo())
            .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor))
            .readiness,
        stikk_core::Evidence::ExplicitYes,
        &stikk_core::ImportTarget {
            path: bundle.clone(),
            received: "remotes/heads/main".to_string(),
        },
    )
    .unwrap_or_else(|e| panic!("0.{}: import: {e}", bin.minor));

    let stikk_core::VerifyView::Checked(checked) = stikk_core::verify(&backend, receiver.repo())
        .unwrap_or_else(|e| panic!("0.{}: verify: {e}", bin.minor))
    else {
        panic!("0.{}: verify reports in band", bin.minor);
    };
    assert!(
        !checked.ok,
        "0.{}: **the point of the leg** — the seal does not verify against the adopted key",
        bin.minor
    );
    assert!(
        checked
            .conditions
            .iter()
            .any(|row| row.id == "publication-trust"),
        "0.{}: prikk's own condition: {:?}",
        bin.minor,
        checked.conditions
    );
    Fixture::clear_env();
}

/// **At the floor the list is unavailable, and the sealer row is 0.16.0's** — nothing extra,
/// not *could not be read* (RFC 054 decision 4).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc054_below_the_band_trust_is_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    let fixture = Fixture::build(&bin);
    let backend = CliBackend::with_program(&bin.path);

    match stikk_core::trust::read(&backend, fixture.repo())
        .unwrap_or_else(|e| panic!("0.{}: trust read: {e}", bin.minor))
    {
        stikk_core::TrustView::Unavailable(reason) => assert!(
            reason.contains(&format!("0.{}", stikk_core::TRUST_FROM_MINOR)),
            "0.{}: the band is named: {reason}",
            bin.minor
        ),
        other => panic!("0.{}: expected unavailable, got {other:?}", bin.minor),
    }
    assert_eq!(
        stikk_core::sealers(&backend, fixture.repo(), &["ed25519-0".to_string()]),
        stikk_core::SealerTrustRead::Unavailable,
        "0.{}: a card below the band says nothing extra",
        bin.minor
    );
}

// ---------------------------------------------------------------- RFC 056: adopt and remove

/// Whether this binary lists and writes trust (RFC 056 decision 1, the Trust view's own band).
fn rfc056_in_band(bin: &PrikkBin) -> bool {
    if bin.minor >= stikk_core::TRUST_FROM_MINOR {
        return true;
    }
    eprintln!(
        "RFC 056 SKIPPED at 0.{} — adopt and remove need prikk ≥ 0.{}.",
        bin.minor,
        stikk_core::TRUST_FROM_MINOR
    );
    false
}

/// An operator who may write, which is all either trust write asks for (F1).
fn rfc056_operator() -> stikk_model::Readiness {
    stikk_model::Readiness {
        read_only: false,
        ..stikk_model::Readiness::none()
    }
}

/// The adopted keys, read through the seam.
fn rfc056_list(
    backend: &impl stikk_prikk::Prikk,
    repo: &std::path::Path,
    minor: u32,
) -> Vec<stikk_core::TrustRow> {
    match stikk_core::trust::read(backend, repo)
        .unwrap_or_else(|e| panic!("0.{minor}: the trust read: {e}"))
    {
        stikk_core::TrustView::Listed { rows, .. } => rows,
        other => panic!("0.{minor}: expected a list, got {other:?}"),
    }
}

/// A second repository's own maintainer key, as **prikk's own list** reports it — a real key
/// pair, with no seed read here or anywhere in this file.
fn rfc056_another_key(bin: &PrikkBin, backend: &impl stikk_prikk::Prikk) -> (String, Fixture) {
    let other = Fixture::build(bin);
    let rows = rfc056_list(backend, other.repo(), bin.minor);
    let public = rows
        .first()
        .map(|row| row.public_key.clone())
        .unwrap_or_else(|| panic!("0.{}: `setup` adopts its own key", bin.minor));
    (public, other)
}

/// Adopt `public` under `key_id`, through stikk's own preview and confirmation.
fn rfc056_adopt(
    backend: &impl stikk_prikk::Prikk,
    repo: &std::path::Path,
    minor: u32,
    key_id: &str,
    public: &str,
) {
    let preview = stikk_core::adopt_preview(backend, repo, rfc056_operator(), key_id, public)
        .unwrap_or_else(|e| panic!("0.{minor}: adopt_preview: {e}"));
    let stikk_core::AdoptPreview::Ready { token } = preview else {
        panic!("0.{minor}: expected an armed adopt, got {preview:?}");
    };
    stikk_core::adopt_confirm_and_execute(
        backend,
        repo,
        *token,
        rfc056_operator(),
        stikk_core::Evidence::TypedName(key_id.to_string()),
        key_id,
        public,
    )
    .unwrap_or_else(|e| panic!("0.{minor}: the adopt: {e}"));
}

/// Remove `key_id`, through stikk's own preview and confirmation.
fn rfc056_remove(
    backend: &impl stikk_prikk::Prikk,
    repo: &std::path::Path,
    minor: u32,
    key_id: &str,
) -> stikk_core::TrustWriteResult {
    let preview = stikk_core::remove_preview(backend, repo, rfc056_operator(), key_id)
        .unwrap_or_else(|e| panic!("0.{minor}: remove_preview: {e}"));
    let stikk_core::RemovePreview::Ready { token } = preview else {
        panic!("0.{minor}: expected an armed removal, got {preview:?}");
    };
    stikk_core::remove_confirm_and_execute(
        backend,
        repo,
        *token,
        rfc056_operator(),
        stikk_core::Evidence::TypedName(key_id.to_string()),
        key_id,
    )
    .unwrap_or_else(|e| panic!("0.{minor}: the removal: {e}"))
    .result
}

/// **Legs 1, 2 and 3 at the ceiling** (RFC 056's Delivery): an adopt grows the list; adopting
/// the same key again is declined; a *different* key under that id is the security event. The
/// last two spawn **no write at all**.
///
/// Leg 1 adopts under the key's **own default id**, so the no-notice path runs against real
/// prikk too — the harness's fixtures all sit under `maintainer` (`support.rs`), which is not a
/// default id and would take the notice path every time.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc056_adopting_grows_the_list_and_the_two_declines_write_nothing_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc056_in_band(&bin) {
        return;
    }
    let counting = CountingWrites::new(&bin);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let (theirs, _other) = rfc056_another_key(&bin, &counting);
    let their_id = stikk_core::default_key_id(&theirs);
    let before = rfc056_list(&counting, &repo, bin.minor).len();

    // **Leg 1.** The card carries no default-id notice, because the id is the key's own.
    let preview =
        stikk_core::adopt_preview(&counting, &repo, rfc056_operator(), &their_id, &theirs)
            .unwrap_or_else(|e| panic!("0.{}: adopt_preview: {e}", bin.minor));
    let stikk_core::AdoptPreview::Ready { token } = preview else {
        panic!("0.{}: expected an armed adopt, got {preview:?}", bin.minor);
    };
    assert!(
        token.summary().notices.is_empty(),
        "0.{}: the id is this key's default: {:?}",
        bin.minor,
        token.summary().notices
    );
    assert_eq!(token.summary().target_name.as_deref(), Some(&their_id[..]));
    stikk_core::adopt_confirm_and_execute(
        &counting,
        &repo,
        *token,
        rfc056_operator(),
        stikk_core::Evidence::TypedName(their_id.clone()),
        &their_id,
        &theirs,
    )
    .unwrap_or_else(|e| panic!("0.{}: the adopt: {e}", bin.minor));
    let rows = rfc056_list(&counting, &repo, bin.minor);
    assert_eq!(rows.len(), before + 1, "0.{}: the list grew", bin.minor);
    assert!(
        rows.iter()
            .any(|row| row.key_id == their_id && row.public_key == theirs),
        "0.{}: and it is the key that was adopted: {rows:?}",
        bin.minor
    );

    let writes = counting
        .trust_writes
        .load(std::sync::atomic::Ordering::SeqCst);

    // **Leg 2.** The same key again: stikk answers from the list it just read.
    match stikk_core::adopt_preview(&counting, &repo, rfc056_operator(), &their_id, &theirs)
        .unwrap_or_else(|e| panic!("0.{}: adopt_preview: {e}", bin.minor))
    {
        stikk_core::AdoptPreview::Blocked(said) => {
            assert_eq!(said, stikk_core::ALREADY_TRUSTED, "0.{}", bin.minor);
        }
        other => panic!("0.{}: expected a decline, got {other:?}", bin.minor),
    }

    // **Leg 3.** A different key under that id: the security event, with both public keys.
    // **The fixture's own key**, found by its id rather than by `yours`: the environment is
    // cleared here, so `key status` resolves nothing and no row is marked.
    let mine = rfc056_list(&counting, &repo, bin.minor)
        .into_iter()
        .find(|row| row.key_id != their_id)
        .unwrap_or_else(|| panic!("0.{}: `setup` adopted this repository's own key", bin.minor));
    match stikk_core::adopt_preview(
        &counting,
        &repo,
        rfc056_operator(),
        &their_id,
        &mine.public_key,
    )
    .unwrap_or_else(|e| panic!("0.{}: adopt_preview: {e}", bin.minor))
    {
        stikk_core::AdoptPreview::TwoKeys(conflict) => {
            assert_eq!(conflict.key_id, their_id);
            assert_eq!(conflict.adopted, theirs, "0.{}", bin.minor);
            assert_eq!(conflict.given, mine.public_key, "0.{}", bin.minor);
        }
        other => panic!(
            "0.{}: expected the security event, got {other:?}",
            bin.minor
        ),
    }

    assert_eq!(
        counting
            .trust_writes
            .load(std::sync::atomic::Ordering::SeqCst),
        writes,
        "0.{}: **neither decline spawned a write**",
        bin.minor
    );
    Fixture::clear_env();
}

/// **Legs 4, 5 and 6 at the ceiling**: a removal shrinks the list; re-adopting that id with a
/// *different* key is prikk's own refusal, verbatim (F2 — the case stikk cannot see, because
/// `trust-list-v1` lists only current keys); and a list of one declines with no write.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc056_removing_shrinks_the_list_and_a_removed_id_is_prikks_refusal_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc056_in_band(&bin) {
        return;
    }
    let counting = CountingWrites::new(&bin);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let (theirs, _other) = rfc056_another_key(&bin, &counting);
    let their_id = stikk_core::default_key_id(&theirs);
    rfc056_adopt(&counting, &repo, bin.minor, &their_id, &theirs);
    let before = rfc056_list(&counting, &repo, bin.minor).len();

    // **Leg 5.** The removal, and the list it leaves.
    let result = rfc056_remove(&counting, &repo, bin.minor, &their_id);
    assert!(
        result
            .printed
            .iter()
            .any(|line| line.contains("revoked maintainer key")),
        "0.{}: prikk's own line: {:?}",
        bin.minor,
        result.printed
    );
    assert_eq!(
        result.see_verify,
        Some(stikk_core::REMOVED_SEE_VERIFY),
        "0.{}: a removal points at Verify",
        bin.minor
    );
    let stikk_core::TrustView::Listed { rows, .. } = &result.view else {
        panic!("0.{}: the list is re-read", bin.minor);
    };
    assert_eq!(rows.len(), before - 1, "0.{}: it shrank", bin.minor);
    assert!(
        !rows.iter().any(|row| row.key_id == their_id),
        "0.{}: and the removed id is gone: {rows:?}",
        bin.minor
    );

    // **Leg 4.** Re-adopting that id with another key: stikk's guards see nothing wrong, because
    // the id is no longer in the list — and prikk refuses it anyway, in words that say exactly
    // what happened (decision 5).
    // As above: by id, because the environment is cleared and nothing is marked *yours*.
    let mine = rfc056_list(&counting, &repo, bin.minor)
        .into_iter()
        .find(|row| row.key_id != their_id)
        .unwrap_or_else(|| panic!("0.{}: `setup` adopted this repository's own key", bin.minor));
    let preview = stikk_core::adopt_preview(
        &counting,
        &repo,
        rfc056_operator(),
        &their_id,
        &mine.public_key,
    )
    .unwrap_or_else(|e| panic!("0.{}: adopt_preview: {e}", bin.minor));
    let stikk_core::AdoptPreview::Ready { token } = preview else {
        panic!(
            "0.{}: stikk cannot see this one coming (F2), got {preview:?}",
            bin.minor
        );
    };
    let err = stikk_core::adopt_confirm_and_execute(
        &counting,
        &repo,
        *token,
        rfc056_operator(),
        stikk_core::Evidence::TypedName(their_id.clone()),
        &their_id,
        &mine.public_key,
    )
    .expect_err("prikk refuses a removed id under a different key");
    assert!(
        err.to_string()
            .contains("already adopted with a different public key"),
        "0.{}: prikk's words, verbatim: {err}",
        bin.minor
    );

    // **Leg 6.** A list of one: stikk declines before prikk is asked, and says what to do.
    let rows = rfc056_list(&counting, &repo, bin.minor);
    assert_eq!(rows.len(), 1, "0.{}: one key left: {rows:?}", bin.minor);
    let writes = counting
        .trust_writes
        .load(std::sync::atomic::Ordering::SeqCst);
    match stikk_core::remove_preview(&counting, &repo, rfc056_operator(), &rows[0].key_id)
        .unwrap_or_else(|e| panic!("0.{}: remove_preview: {e}", bin.minor))
    {
        stikk_core::RemovePreview::Blocked(said) => {
            assert_eq!(said, stikk_core::LAST_KEY, "0.{}", bin.minor);
        }
        other => panic!("0.{}: expected a decline, got {other:?}", bin.minor),
    }
    assert_eq!(
        counting
            .trust_writes
            .load(std::sync::atomic::Ordering::SeqCst),
        writes,
        "0.{}: the last-key decline spawned nothing",
        bin.minor
    );
    Fixture::clear_env();
}

/// **Leg 7, the one that matters**: removing the key that sealed this history makes stikk's own
/// Verify fail it — and adopting the same key under the same id again repairs it.
///
/// **This leg is the evidence for decision 3's sentence**, the one the remove card always
/// carries: *"If this key sealed blocks in this repository, Verify will fail them until the same
/// key is adopted again under the same id."* If prikk ever stops behaving this way, this goes
/// red and the sentence is reconsidered rather than quietly left wrong.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc056_removing_the_sealing_key_fails_verify_until_it_is_adopted_again_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc056_in_band(&bin) || bin.minor < stikk_core::VERIFY_FROM_MINOR {
        return;
    }
    let backend = CliBackend::with_program(&bin.path);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();

    // History this repository's own key sealed.
    std::fs::write(repo.join("a.txt"), "a\n")
        .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
    fixture.set_author_env();
    backend
        .commit(&repo, "heads/main", "base")
        .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
    fixture.set_maintainer_env();
    backend
        .seal(&repo, "heads/main")
        .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
    Fixture::clear_env();
    assert!(
        rfc056_verify_ok(&backend, &repo, bin.minor),
        "0.{}: the premise — a sealed repository verifies",
        bin.minor
    );

    // **A second key first**, because prikk refuses to remove the last one (F4) — which is
    // exactly what leg 6 asserts stikk says before asking.
    let (theirs, _other) = rfc056_another_key(&bin, &backend);
    let their_id = stikk_core::default_key_id(&theirs);
    rfc056_adopt(&backend, &repo, bin.minor, &their_id, &theirs);

    let sealing = rfc056_list(&backend, &repo, bin.minor)
        .into_iter()
        .find(|row| row.key_id != their_id)
        .unwrap_or_else(|| panic!("0.{}: the key that sealed this history", bin.minor));
    rfc056_remove(&backend, &repo, bin.minor, &sealing.key_id);

    let stikk_core::VerifyView::Checked(checked) = stikk_core::verify(&backend, &repo)
        .unwrap_or_else(|e| panic!("0.{}: verify: {e}", bin.minor))
    else {
        panic!("0.{}: verify reports in band", bin.minor);
    };
    assert!(
        !checked.ok,
        "0.{}: **the point of the leg** — the sealed history no longer verifies",
        bin.minor
    );
    assert!(
        checked
            .conditions
            .iter()
            .any(|row| row.id == "publication-trust"),
        "0.{}: prikk's own condition: {:?}",
        bin.minor,
        checked.conditions
    );

    // **And adopting the same key under the same id repairs it**, which is what makes the
    // card's sentence say *until*, not *forever*.
    rfc056_adopt(
        &backend,
        &repo,
        bin.minor,
        &sealing.key_id,
        &sealing.public_key,
    );
    assert!(
        rfc056_verify_ok(&backend, &repo, bin.minor),
        "0.{}: re-adopting the same key under the same id repairs it",
        bin.minor
    );
    Fixture::clear_env();
}

/// Whether prikk's verdict on this repository is sound, through stikk's own Verify.
fn rfc056_verify_ok(backend: &impl stikk_prikk::Prikk, repo: &std::path::Path, minor: u32) -> bool {
    match stikk_core::verify(backend, repo).unwrap_or_else(|e| panic!("0.{minor}: verify: {e}")) {
        stikk_core::VerifyView::Checked(checked) => checked.ok,
        other => panic!("0.{minor}: verify reports in band, got {other:?}"),
    }
}

/// **Leg 8**: removing your own key is allowed, and prikk then reports the maintainer role as
/// `not-adopted` — the state the card's second notice warns about, measured rather than
/// asserted from the prose.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc056_removing_your_own_key_leaves_the_maintainer_role_not_adopted_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if !rfc056_in_band(&bin) {
        return;
    }
    let backend = CliBackend::with_program(&bin.path);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let (theirs, _other) = rfc056_another_key(&bin, &backend);
    rfc056_adopt(
        &backend,
        &repo,
        bin.minor,
        &stikk_core::default_key_id(&theirs),
        &theirs,
    );

    // **The card marks it as yours**, which needs the id *and* the public key to match -- so
    // the environment carries the fixture's own key while the preview is built.
    fixture.set_maintainer_env();
    let mine = rfc056_list(&backend, &repo, bin.minor)
        .into_iter()
        .find(|row| row.yours)
        .unwrap_or_else(|| panic!("0.{}: this repository knows its own key", bin.minor));
    let preview = stikk_core::remove_preview(&backend, &repo, rfc056_operator(), &mine.key_id)
        .unwrap_or_else(|e| panic!("0.{}: remove_preview: {e}", bin.minor));
    let stikk_core::RemovePreview::Ready { token } = preview else {
        panic!(
            "0.{}: expected an armed removal, got {preview:?}",
            bin.minor
        );
    };
    assert!(
        token
            .summary()
            .notices
            .iter()
            .any(|notice| notice == stikk_core::REMOVE_YOUR_OWN),
        "0.{}: the card says it is yours: {:?}",
        bin.minor,
        token.summary().notices
    );
    stikk_core::remove_confirm_and_execute(
        &backend,
        &repo,
        *token,
        rfc056_operator(),
        stikk_core::Evidence::TypedName(mine.key_id.clone()),
        &mine.key_id,
    )
    .unwrap_or_else(|e| panic!("0.{}: the removal: {e}", bin.minor));

    let readiness = backend
        .readiness(&repo)
        .unwrap_or_else(|e| panic!("0.{}: readiness: {e}", bin.minor));
    assert_eq!(
        readiness.readiness.maintainer,
        stikk_model::RoleReadiness::Known(stikk_model::Binding::NotAdopted),
        "0.{}: prikk reports the role as not-adopted after the removal",
        bin.minor
    );
    Fixture::clear_env();
}

/// **At the floor both writes are unavailable, and nothing is spawned** (decision 1).
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc056_below_the_band_both_writes_are_unavailable() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    assert!(
        bin.minor < stikk_core::TRUST_FROM_MINOR,
        "the floor is below the band"
    );
    let counting = CountingWrites::new(&bin);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();
    let said = stikk_core::trust_unavailable(bin.minor);

    match stikk_core::adopt_preview(
        &counting,
        &repo,
        rfc056_operator(),
        "ed25519-0000000000000000",
        &"0".repeat(64),
    )
    .unwrap_or_else(|e| panic!("0.{}: adopt_preview: {e}", bin.minor))
    {
        stikk_core::AdoptPreview::Blocked(reason) => assert_eq!(reason, said, "0.{}", bin.minor),
        other => panic!("0.{}: expected a block, got {other:?}", bin.minor),
    }
    match stikk_core::remove_preview(
        &counting,
        &repo,
        rfc056_operator(),
        "ed25519-0000000000000000",
    )
    .unwrap_or_else(|e| panic!("0.{}: remove_preview: {e}", bin.minor))
    {
        stikk_core::RemovePreview::Blocked(reason) => assert_eq!(reason, said, "0.{}", bin.minor),
        other => panic!("0.{}: expected a block, got {other:?}", bin.minor),
    }
    assert_eq!(
        counting
            .trust_writes
            .load(std::sync::atomic::Ordering::SeqCst),
        0,
        "0.{}: nothing was spawned below the band",
        bin.minor
    );
    Fixture::clear_env();
}

// ---------------------------------------------------------------- RFC 057: Orientation on JSON

/// Set `PRIKK_ACTIVE_PATCH_WARN`/`_LIMIT` for one measurement, and clear them after.
///
/// **Under `ENV_LOCK`**, like every other variable this harness sets: the caller holds it.
fn rfc057_with_thresholds<T>(
    warn: Option<&str>,
    limit: Option<&str>,
    run: impl FnOnce() -> T,
) -> T {
    // SAFETY: the caller holds `ENV_LOCK`, the same discipline `Fixture::set_author_env` keeps.
    #[allow(unsafe_code)]
    unsafe {
        match warn {
            Some(value) => std::env::set_var("PRIKK_ACTIVE_PATCH_WARN", value),
            None => std::env::remove_var("PRIKK_ACTIVE_PATCH_WARN"),
        }
        match limit {
            Some(value) => std::env::set_var("PRIKK_ACTIVE_PATCH_LIMIT", value),
            None => std::env::remove_var("PRIKK_ACTIVE_PATCH_LIMIT"),
        }
    }
    let out = run();
    #[allow(unsafe_code)]
    unsafe {
        std::env::remove_var("PRIKK_ACTIVE_PATCH_WARN");
        std::env::remove_var("PRIKK_ACTIVE_PATCH_LIMIT");
    }
    out
}

/// **RFC 057's proof of "no visible change"**: at the ceiling, the JSON path and the prose
/// parser agree on every field, for each of the five measured states.
///
/// **(a)** is `Prikk::orientation`, which is the JSON path at the ceiling; **(b)** is `prikk
/// status`'s own text, read by the prose parser through
/// [`stikk_prikk::cli_backend::orientation_from_prose`] — a `#[doc(hidden)]` door that exists
/// for this leg, because the product picks one reader by version and nothing in it can run the
/// other against the same repository.
///
/// **The two typed fields are compared by their content**, which is the whole of decision 2:
/// (a)'s `Threshold` numbers must be the ones inside (b)'s `Said` sentence.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc057_json_and_prose_orientation_agree_at_the_ceiling() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::ceiling();
    if bin.minor < stikk_prikk::ORIENTATION_JSON_FROM_MINOR {
        eprintln!(
            "RFC 057 SKIPPED at 0.{} — the JSON path starts at 0.{}.",
            bin.minor,
            stikk_prikk::ORIENTATION_JSON_FROM_MINOR
        );
        return;
    }
    let backend = CliBackend::with_program(&bin.path);
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();

    // The five states `measure057.sh` measures, built in its order. **A: nothing published** is
    // this fixture before anything is committed. `compared` counts the two states where prikk
    // warns, so a leg that silently stopped reaching them cannot pass.
    let mut compared = 0_u32;

    for (at, (state, warn, limit)) in [
        ("A: nothing published", None, None),
        ("B: one sealed block", None, None),
        ("C: one queued patch", None, None),
        ("D: the warn threshold", Some("1"), None),
        ("E: the hard limit", Some("1"), Some("1")),
    ]
    .into_iter()
    .enumerate()
    {
        // Build the state this step needs, on top of the last.
        match at {
            1 => {
                std::fs::write(repo.join("a.txt"), "a\n")
                    .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
                fixture.set_author_env();
                backend
                    .commit(&repo, "heads/main", "one")
                    .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
                fixture.set_maintainer_env();
                backend
                    .seal(&repo, "heads/main")
                    .unwrap_or_else(|e| panic!("0.{}: seal: {e}", bin.minor));
                Fixture::clear_env();
            }
            2 => {
                std::fs::write(repo.join("b.txt"), "b\n")
                    .unwrap_or_else(|e| panic!("0.{}: write: {e}", bin.minor));
                fixture.set_author_env();
                backend
                    .commit(&repo, "heads/main", "two")
                    .unwrap_or_else(|e| panic!("0.{}: commit: {e}", bin.minor));
                Fixture::clear_env();
            }
            _ => {}
        }

        compared += rfc057_with_thresholds(warn, limit, || {
            // (a) the product's own path, which is JSON here.
            let json = backend
                .orientation(&repo)
                .unwrap_or_else(|e| panic!("0.{}: {state}: orientation: {e}", bin.minor));
            // (b) prikk's prose, read by the parser the floor still uses.
            let text = prikk_ok(&bin, &repo, &["status"]);
            let prose = stikk_prikk::cli_backend::orientation_from_prose(
                &String::from_utf8_lossy(&text.stdout),
                bin.minor,
            )
            .unwrap_or_else(|e| panic!("0.{}: {state}: the prose parser: {e}", bin.minor));

            assert_eq!(
                json.queued_patches, prose.queued_patches,
                "0.{}: {state}: queued",
                bin.minor
            );
            assert_eq!(
                json.queued_target, prose.queued_target,
                "0.{}: {state}: the queue's target",
                bin.minor
            );
            assert_eq!(
                json.main_ref_state, prose.main_ref_state,
                "0.{}: {state}: heads/main",
                bin.minor
            );
            assert_eq!(
                json.trailing_partial_wal_bytes, prose.trailing_partial_wal_bytes,
                "0.{}: {state}: the torn tail",
                bin.minor
            );
            assert_eq!(
                json.current_branch, prose.current_branch,
                "0.{}: {state}: the current branch",
                bin.minor
            );
            // Neither path reports an interrupted materialization in these five states, and
            // the prose path never reports one at all (RFC 057 decision 1).
            assert_eq!(
                json.interrupted_materialization, None,
                "0.{}: {state}",
                bin.minor
            );
            assert_eq!(
                prose.interrupted_materialization, None,
                "0.{}: {state}",
                bin.minor
            );

            // **The one field that is not equal, and the one that proves the point.** prikk
            // says it in prose and states it in JSON; the numbers must be the same numbers.
            match (&json.active_patch_warning, &prose.active_patch_warning) {
                (None, None) => 0,
                (
                    Some(stikk_prikk::ActivePatchWarning::Threshold {
                        blocking,
                        count,
                        threshold,
                    }),
                    Some(stikk_prikk::ActivePatchWarning::Said(said)),
                ) => {
                    assert!(
                        said.contains(&format!("({count})")),
                        "0.{}: {state}: prikk's sentence carries the same count: {said}",
                        bin.minor
                    );
                    assert!(
                        said.contains(&format!("({threshold})")),
                        "0.{}: {state}: and the same threshold: {said}",
                        bin.minor
                    );
                    assert_eq!(
                        *blocking,
                        said.contains("hard limit"),
                        "0.{}: {state}: and the same severity: {said}",
                        bin.minor
                    );
                    1
                }
                (json, prose) => panic!(
                    "0.{}: {state}: the two paths disagree about the warning: {json:?} vs {prose:?}",
                    bin.minor
                ),
            }
        });
    }
    assert_eq!(
        compared, 2,
        "0.{}: both warning states were reached -- the warn threshold and the hard limit",
        bin.minor
    );
    Fixture::clear_env();
}

/// **At the floor the prose path is the one that runs**, and `--format` is never passed.
///
/// Proved twice, because neither proof alone is complete: the **shim** records the argv stikk
/// actually spawned, and the **binary itself** refuses `--format` at this version, so an
/// `orientation` that succeeds here cannot have used it. The shim is Unix-only; the refusal is
/// measured on every platform.
#[test]
#[ignore = "needs two real prikk binaries; see this file's module doc"]
fn rfc057_at_the_floor_orientation_asks_for_prose() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Fixture::clear_env();
    let bin = PrikkBin::floor();
    assert!(
        bin.minor < stikk_prikk::ORIENTATION_JSON_FROM_MINOR,
        "the floor is below the JSON band"
    );
    let fixture = Fixture::build(&bin);
    let repo = fixture.repo().to_path_buf();

    // **prikk itself refuses the flag at this version**, exit 2, so a successful read proves
    // the flag was not passed.
    let refused = std::process::Command::new(&bin.path)
        .args(["status", "--format", "json"])
        .current_dir(&repo)
        .output()
        .unwrap_or_else(|e| panic!("0.{}: spawn: {e}", bin.minor));
    assert!(
        !refused.status.success(),
        "0.{}: prikk refuses `--format` here, which is why the band exists",
        bin.minor
    );
    let read = CliBackend::with_program(&bin.path)
        .orientation(&repo)
        .unwrap_or_else(|e| panic!("0.{}: orientation: {e}", bin.minor));
    assert_eq!(read.queued_patches, 0, "0.{}", bin.minor);
    assert_eq!(
        read.current_branch,
        stikk_model::CurrentBranch::NotReported,
        "0.{}: 0.28 keeps no current-branch pointer",
        bin.minor
    );

    #[cfg(unix)]
    {
        // **The argv stikk spawned**, recorded by a shim that execs the real binary.
        use std::os::unix::fs::PermissionsExt;
        let dir = repo
            .parent()
            .unwrap_or_else(|| panic!("0.{}: the fixture root", bin.minor))
            .join("shim");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir: {e}"));
        let argv = dir.join("argv");
        let shim = dir.join("prikk");
        std::fs::write(
            &shim,
            format!(
                "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{}\"; done\nexec \"{}\" \"$@\"\n",
                argv.display(),
                bin.path.display()
            ),
        )
        .unwrap_or_else(|e| panic!("write the shim: {e}"));
        let mut perms = std::fs::metadata(&shim)
            .unwrap_or_else(|e| panic!("stat the shim: {e}"))
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim, perms).unwrap_or_else(|e| panic!("chmod the shim: {e}"));

        CliBackend::with_program(&shim)
            .orientation(&repo)
            .unwrap_or_else(|e| panic!("0.{}: orientation through the shim: {e}", bin.minor));
        let recorded = std::fs::read_to_string(&argv).unwrap_or_else(|e| panic!("read argv: {e}"));
        assert!(
            recorded.lines().any(|line| line == "status"),
            "0.{}: it asked for status: {recorded:?}",
            bin.minor
        );
        assert!(
            !recorded.lines().any(|line| line == "--format"),
            "0.{}: and never for a format: {recorded:?}",
            bin.minor
        );
    }
    Fixture::clear_env();
}

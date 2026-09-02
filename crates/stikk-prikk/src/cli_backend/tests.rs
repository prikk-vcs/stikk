//! Tests for the CLI backend's process mechanics (design TS-03, TS-07).
//!
//! These exercise the drain-and-classify path with real but trivial programs (`true`, `false`,
//! `printf`) so they run without a prikk binary. Parsing is tested against golden fixtures in
//! `parse/tests.rs`; version parsing in `version/tests.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use stikk_model::RequestCategory;

use super::*;

#[test]
fn missing_program_is_an_environment_error_not_a_panic() {
    // A binary that does not exist must classify as environment, never crash the front-end.
    let backend = CliBackend::with_program("definitely-not-a-real-program-xyz");
    let err = backend.handshake().expect_err("should fail to launch");
    assert_eq!(err.class(), "environment");
}

#[cfg(unix)]
#[test]
fn non_zero_exit_is_classified_and_message_preserved() {
    // `sh -c 'echo "error: something refused" >&2; exit 1'` stands in for a prikk refusal.
    let backend = CliBackend::with_program("sh");
    let result = backend.run(
        None,
        RequestCategory::ReadHistory,
        [
            "-c",
            "echo 'error: merge refused: not confluent' 1>&2; exit 1",
        ],
    );
    let err = result.expect_err("non-zero exit is an error");
    // Default classification is a refusal, and the verbatim message survives.
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("merge refused: not confluent"));
}

#[cfg(unix)]
#[test]
fn lock_message_classifies_as_lock_conflict() {
    let backend = CliBackend::with_program("sh");
    let err = backend
        .run(
            None,
            RequestCategory::Publication,
            ["-c", "echo 'ref lock already exists' 1>&2; exit 1"],
        )
        .expect_err("non-zero exit");
    assert_eq!(err.class(), "lock-conflict");
}

#[cfg(unix)]
#[test]
fn drains_large_output_without_deadlock() {
    // A program that writes a lot to stdout must be fully drained (the EPIPE guard, UD-04): if stikk
    // closed the pipe early this would hang or error. `yes | head` produces bounded large output.
    let backend = CliBackend::with_program("sh");
    let out = backend
        .run(None, RequestCategory::ReadHistory, ["-c", "seq 1 100000"])
        .expect("large output drains");
    assert!(out.lines().count() >= 100_000);
}

#[cfg(unix)]
#[test]
fn run_capturing_keeps_stdout_on_a_nonzero_exit() {
    // `worktree-status` exits 1 for a dirty tree while writing the report to stdout (RFC 008): the
    // capturing runner must return that stdout with success=false, never discard it or classify it.
    let backend = CliBackend::with_program("sh");
    let (stdout, stderr, success) = backend
        .run_capturing(
            None,
            [
                "-c",
                "printf 'the report\\n'; printf 'oops\\n' 1>&2; exit 1",
            ],
        )
        .expect("capture succeeds even though the process exits 1");
    assert!(!success);
    assert!(stdout.contains("the report"));
    assert!(stderr.contains("oops"));
}

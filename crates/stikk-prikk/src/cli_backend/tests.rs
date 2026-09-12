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
fn exit_2_is_a_stikk_fault_never_prikks_refusal() {
    // RFC 009 F6: prikk 0.28+ uses exit 2 for a usage error — a bad argument list stikk assembled,
    // detected before any repository work. It must never be classified as prikk's semantic refusal.
    let backend = CliBackend::with_program("sh");
    let err = backend
        .run(
            None,
            RequestCategory::ReadHistory,
            [
                "-c",
                "echo 'error: unknown log argument: --nonexistent-flag' 1>&2; exit 2",
            ],
        )
        .expect_err("exit 2 is an error");
    assert_eq!(err.class(), "stikk-internal");
    // prikk's own message is kept — it names the bad argument — but the class is stikk's.
    assert!(err.to_string().contains("unknown log argument"));
}

#[cfg(unix)]
#[test]
fn run_capturing_also_treats_exit_2_as_a_stikk_fault() {
    // The dirty-exit caller (`worktree_status`) must never see a usage-error exit as a normal outcome
    // to interpret itself — `run_capturing` intercepts it before returning.
    let backend = CliBackend::with_program("sh");
    let err = backend
        .run_capturing(
            None,
            [
                "-c",
                "echo 'error: worktree-status requires --ref' 1>&2; exit 2",
            ],
        )
        .expect_err("exit 2 is an error");
    assert_eq!(err.class(), "stikk-internal");
}

#[cfg(unix)]
#[test]
fn handshake_probes_the_program_at_most_once_per_backend() {
    // RFC 010 / SEAM-05: the version probe is recorded at open and reused, never re-run per operation
    // (`orient` and `changes_view` used to each call it separately — RFC 010 finding 4). A real prikk
    // can't report its own invocation count, so this is a tiny script that counts its own runs via a
    // side effect (appending to a counter file) and then prints a real version line regardless of its
    // arguments — standing in for `prikk --version`.
    use std::os::unix::fs::PermissionsExt;

    let dir =
        std::env::temp_dir().join(format!("stikk-handshake-cache-test-{}", std::process::id()));
    // Defensive, not incidental: the directory name is PID-based, and a prior run of this same test
    // killed before reaching its own cleanup below (e.g. a timed-out `cargo test`) can leave a stale
    // `count` file that a later process reusing that PID would inherit, corrupting the "exactly one
    // run" assertion below through no fault of the code under test.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let counter = dir.join("count");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho x >> \"{}\"\necho 'prikk 0.30.0'\n",
            counter.display()
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    // Executing a script immediately after writing it is exactly the shape that can transiently race
    // the kernel on a heavily loaded host (ETXTBSY / "text file busy": something else briefly held the
    // inode open for reading right after creation) — observed under `cargo test --workspace`'s process
    // churn, never in isolation, and never on a *second* attempt. This is a test-environment artifact
    // of dynamically writing-then-immediately-executing a file, not a `CliBackend` behavior worth
    // retrying in production (a real `prikk` binary has been on disk for a while by the time anyone
    // runs stikk against it), so the retry lives here, not in the code under test.
    let first =
        retrying_transient_exec_busy(|| backend.handshake()).expect("first handshake succeeds");
    let second = backend
        .handshake()
        .expect("second handshake succeeds (cached)");
    assert_eq!(first, second);

    let runs = std::fs::read_to_string(&counter)
        .unwrap_or_default()
        .lines()
        .count();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        runs, 1,
        "the script must run exactly once across two handshake() calls"
    );
}

#[cfg(unix)]
#[test]
fn change_token_drives_exactly_branch_status_and_tag_no_more_no_repeats() {
    // Review C1: `change_token` composes from `refs()` + `tags()` (deduplicated) + `orientation` — not
    // `refs()` alone, since RFC 012 F3 established `branch list --all`'s tag coverage is unspecified.
    // "Zero additional spawns" was never the requirement; "no hidden or duplicated calls" is. A fake
    // `prikk` that logs which subcommand it was called with, once per call, proves exactly one each of
    // `branch`, `status`, `tag` — nothing else, none repeated — after one `change_token()` call.
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-change-token-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); // see the handshake test above for why
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$1\" >> \"{log}\"\n\
             case \"$1\" in\n\
             --version)\n\
             echo 'prikk 0.38.0'\n\
             ;;\n\
             branch)\n\
             echo 'heads/main {id}'\n\
             ;;\n\
             tag)\n\
             echo 'no tags'\n\
             ;;\n\
             status)\n\
             printf 'prikk repository: /tmp/x/.prikk\\n\
             active WAL records: 0\\n\
             trailing partial WAL bytes: 0\\n\
             heads/main RefState: <not published>\\n\
             queued patches: 0\\n\
             status: multi-operation text diff minimization and plugins not yet implemented\\n'\n\
             ;;\n\
             esac\n",
            log = log.display(),
            id = "0".repeat(64),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    let token =
        retrying_transient_exec_busy(|| backend.change_token(&dir)).expect("change_token succeeds");
    // Composed from real (scripted) data — no tags, one branch — so it must equal a token built the
    // same way by hand.
    //
    // **The `--version` spawn is new in RFC 026 §6** and is the cost of the JSON gate: `refs` and
    // `tags` each ask which era they are in. It appears **once**, not twice, because the handshake is
    // cached per backend — which is the property this test's "none repeated" clause now also covers.
    // Once per backend, not once per token: `change_token` is what every mutation preview is gated on,
    // so a per-call probe would have been a real cost rather than a one-off.
    assert_eq!(
        token,
        stikk_model::ChangeToken::compose([("heads/main", "0".repeat(64).as_str())], 0, None)
    );

    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let mut lines: Vec<&str> = calls.lines().collect();
    lines.sort_unstable();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        lines,
        vec!["--version", "branch", "status", "tag"],
        "change_token must call exactly one branch-list, one status, and one tag-list — nothing else, \
         none repeated"
    );
}

/// Build a fake `prikk` answering `branch list --all` with `branch_output` and `tag list` with
/// `tag_output` verbatim (each a complete, newline-terminated prikk-shaped report), and `status` with a
/// fixed clean/empty report — for the `change_token` determinism tests below, where only the ref/tag
/// listings vary. Returns the backend and its temp dir, which the caller must clean up.
#[cfg(unix)]
/// A scripted stand-in for prikk, answering `branch`, `tag`, `status` — and, since RFC 026 §6,
/// `--version`.
///
/// **It reports 0.38.0 deliberately.** The outputs it returns are the *prose* forms, so the version it
/// claims has to be one where stikk reads prose: at ≥ 0.39 `refs`/`tags` ask for `--format json` and
/// would be handed line-oriented text, which is a different test than the one these are. It is also
/// the era the tag-leak property below is *about* — `branch list --all` stopped including tag refs at
/// 0.39, so the unspecified behaviour these tests pin is specific to this side of that line.
fn fake_prikk_backend(
    name_suffix: &str,
    branch_output: &str,
    tag_output: &str,
) -> (CliBackend, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!(
        "stikk-change-token-{name_suffix}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir); // see the handshake test above for why
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             case \"$1\" in\n\
             --version)\n\
             printf 'prikk 0.38.0\\n'\n\
             ;;\n\
             branch)\n\
             printf '{branch_output}'\n\
             ;;\n\
             tag)\n\
             printf '{tag_output}'\n\
             ;;\n\
             status)\n\
             printf 'prikk repository: /tmp/x/.prikk\\n\
             active WAL records: 0\\n\
             trailing partial WAL bytes: 0\\n\
             heads/main RefState: <not published>\\n\
             queued patches: 0\\n\
             status: multi-operation text diff minimization and plugins not yet implemented\\n'\n\
             ;;\n\
             esac\n",
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");
    (CliBackend::with_program(&script), dir)
}

#[cfg(unix)]
#[test]
fn a_moved_tag_changes_the_change_token() {
    // Review C1's determinism requirement, positive case: a tag's id moving is a real repository
    // change, and the merged token must catch it exactly as a moved branch would.
    let tag_a = "a".repeat(64);
    let tag_b = "b".repeat(64);
    let branch = format!("heads/main {}\n", "0".repeat(64));
    let (backend_a, dir_a) =
        fake_prikk_backend("moved-tag-a", &branch, &format!("tags/v1 {tag_a}\n"));
    let (backend_b, dir_b) =
        fake_prikk_backend("moved-tag-b", &branch, &format!("tags/v1 {tag_b}\n"));

    let token_a =
        retrying_transient_exec_busy(|| backend_a.change_token(&dir_a)).expect("succeeds");
    let token_b =
        retrying_transient_exec_busy(|| backend_b.change_token(&dir_b)).expect("succeeds");
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
    assert_ne!(token_a, token_b);
}

#[cfg(unix)]
#[test]
fn a_tag_leaking_into_branch_list_yields_the_same_token_as_appearing_in_tag_list_only() {
    // Review C1's determinism requirement, the property the fix exists for: RFC 012 F3 established
    // that `branch list --all` may or may not also print a tag. Whichever way that unspecified
    // behaviour goes, the merged, deduplicated token must be identical.
    let tag_id = "a".repeat(64);
    let branch_only = format!("heads/main {}\n", "0".repeat(64));
    let branch_with_leak = format!("heads/main {}\ntags/v1 {tag_id}\n", "0".repeat(64));
    let tag_output = format!("tags/v1 {tag_id}\n");

    let (no_leak, dir_no_leak) = fake_prikk_backend("no-leak", &branch_only, &tag_output);
    let (with_leak, dir_with_leak) =
        fake_prikk_backend("with-leak", &branch_with_leak, &tag_output);

    let token_no_leak =
        retrying_transient_exec_busy(|| no_leak.change_token(&dir_no_leak)).expect("succeeds");
    let token_with_leak =
        retrying_transient_exec_busy(|| with_leak.change_token(&dir_with_leak)).expect("succeeds");
    let _ = std::fs::remove_dir_all(&dir_no_leak);
    let _ = std::fs::remove_dir_all(&dir_with_leak);
    assert_eq!(
        token_no_leak, token_with_leak,
        "the token must be identical regardless of whether branch list also happens to leak the tag"
    );
}

/// Retry an operation against a just-written, just-made-executable script past a transient ETXTBSY
/// ("text file busy") — see the call site's comment. Any other error returns immediately; this only
/// absorbs the one specific, known-transient kernel race, never masks a real failure.
#[cfg(unix)]
fn retrying_transient_exec_busy<T>(mut op: impl FnMut() -> Result<T>) -> Result<T> {
    for attempt in 0..20 {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if attempt < 19 && is_transient_exec_busy(&err) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            other => return other,
        }
    }
    unreachable!("loop always returns on its last iteration")
}

#[cfg(unix)]
fn is_transient_exec_busy(err: &stikk_model::StikkError) -> bool {
    std::error::Error::source(err)
        .and_then(|source| source.downcast_ref::<std::io::Error>())
        .and_then(std::io::Error::raw_os_error)
        == Some(26) // ETXTBSY on Linux
}

/// The `prikk key` / `prikk setup` boundary (threat model C-I1e, RFC 017 §7): stikk never invokes
/// `prikk key generate` or `prikk key public --seed-env`, and never wraps `prikk setup`. `run` and
/// `run_capturing` are private methods on `CliBackend` defined in `cli_backend.rs`, but Rust's privacy
/// rules let any **descendant** module reach a private ancestor item — so a future file added anywhere
/// under `cli_backend/` (a `seal.rs` for RFC 016's seam method, say) could add a new spawn call this
/// test would not see if it scanned `cli_backend.rs` alone. Scanned at the source level (the way
/// `env.rs`'s TS-04 test scans for a materialized seed value), but walking the whole `cli_backend`
/// module tree at test time — `cli_backend.rs` plus every `.rs` file under `cli_backend/`, current and
/// future — rather than a fixed list of `include_str!` paths, so a new file is covered without anyone
/// remembering to add it here.
#[test]
fn the_command_surface_never_names_key_or_setup() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![manifest_dir.join("src/cli_backend.rs")];
    collect_rs_files(&manifest_dir.join("src/cli_backend"), &mut files);
    assert!(
        files.len() >= 6,
        "sanity: expected to find cli_backend.rs plus at least classify.rs, parse.rs, tests.rs and \
         their own tests.rs submodules; found {files:?} — did the module layout change?"
    );
    for path in &files {
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
        // Scan code only: this doc comment and others legitimately *name* the forbidden subcommands to
        // explain the rule, so the invariant is enforced against non-comment source lines.
        let code: String = src
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["\"key\"", "\"setup\""] {
            assert!(
                !code.contains(forbidden),
                "{path:?} must never invoke `prikk {forbidden}` — key management is prikk's job, not \
                 a history browser's (C-I1e)"
            );
        }
    }
}

/// Recursively collect every `.rs` file under `dir` (helper for the boundary test above).
fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading dir {dir:?}: {e}")) {
        let path = entry
            .unwrap_or_else(|e| panic!("reading entry in {dir:?}: {e}"))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
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

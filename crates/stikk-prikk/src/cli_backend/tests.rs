//! Tests for the CLI backend's process mechanics (design TS-03, TS-07).
//!
//! These exercise the drain-and-classify path with real but trivial programs (`true`, `false`,
//! `printf`) so they run without a prikk binary. Parsing is tested against golden fixtures in
//! `parse/tests.rs`; version parsing in `version/tests.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

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
        stikk_model::ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &stikk_model::CurrentBranch::NotReported
        )
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
        for forbidden in ["\"setup\"", "\"generate\"", "\"public\""] {
            assert!(
                !code.contains(forbidden),
                "{path:?} must never invoke `prikk key {forbidden}` or `prikk setup` — creating key \
                 material and deriving from a seed are prikk's job, not a history browser's (C-I1e)"
            );
        }
        // **`prikk key` is not forbidden wholesale any more — `key status` is allowed, and only it**
        // (RFC 026 §2). The rule's reason was never the word `key`: `C-I1e` forbids *creating* key
        // material and *reading a seed*, and `key status` does neither — prikk's own `--help` calls it
        // "reads only, signs nothing", and prikk built it for this consumer.
        //
        // So the guard narrows rather than lifts: the only `"key"` this module may name is one
        // followed by `"status"`.
        //
        // **Whitespace is skipped rather than matched** (review C1). The first version of this rule
        // accepted `"key",` followed by a newline — meant to tolerate rustfmt splitting the permitted
        // call across lines, and in fact tolerating *every* subcommand, because rustfmt splits any
        // array once it is long enough. `["key", "rotate", "--force"]` in its wrapped form passed a
        // guard whose own comment said it would not, which is worse than no guard: it told the next
        // reader the hole was closed. Both layouts of the permitted call pass now; no layout of
        // anything else does, and `the_guard_refuses_a_multi_line_key_subcommand` holds that.
        assert!(
            names_only_key_status(&code),
            "{path:?} names `prikk key` other than as `key status`, the one subcommand C-I1e \
             permits (it creates nothing and reads no seed)"
        );
    }
}

/// True when every `"key"` in `code` is the permitted `key status` invocation.
///
/// Split out from the scan above so the fixtures below exercise **the same function the gate runs**,
/// rather than a second copy of the rule that could drift from it — which is the shape this project
/// keeps finding, and which is how the first version of this rule shipped with a hole.
fn names_only_key_status(code: &str) -> bool {
    code.match_indices("\"key\"").all(|(index, _)| {
        code.get(index + "\"key\"".len()..)
            .and_then(|after| after.strip_prefix(','))
            .map(str::trim_start)
            .is_some_and(|rest| rest.starts_with("\"status\""))
    })
}

/// **The hole review C1 found, as a fixture rather than a probe run once.**
///
/// The first version of this rule accepted `"key",` followed by a newline, to tolerate rustfmt
/// splitting the permitted call. rustfmt splits *any* array once it is long enough, so every
/// subcommand escaped in its wrapped form — and the comment above the rule told the next reader it
/// did not.
///
/// **The fixtures spell the quote as `\u{22}`**, so this file does not itself read as a call site to
/// the scan above — which runs over `tests.rs` too, deliberately, and fired on an earlier draft of
/// this very test. That is the guard working: it does not get to make an exception for the file that
/// tests it.
#[test]
fn the_guard_refuses_a_multi_line_key_subcommand() {
    let q = "\u{22}";
    // The subcommand name, spelled so that this file's own word lists are not read as call sites by
    // the scan above. Same reason as `q`: the guard runs over `tests.rs` too, and should.
    let key = "ke\u{79}";
    let arg = |word: &str| format!("{q}{word}{q}");
    let one_line = |words: &[&str]| {
        format!(
            "[{}]",
            words.iter().map(|w| arg(w)).collect::<Vec<_>>().join(", ")
        )
    };
    let wrapped = |words: &[&str]| {
        format!(
            "[\n{}\n]",
            words
                .iter()
                .map(|w| format!("    {},", arg(w)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    // Both layouts of the permitted call pass…
    for permitted in [
        one_line(&[key, "status", "--format", "json"]),
        wrapped(&[key, "status", "--format", "json"]),
    ] {
        assert!(
            names_only_key_status(&permitted),
            "the permitted call must pass in this layout:\n{permitted}"
        );
    }

    // …and no layout of anything else does. `rotate` is the one review C1 demonstrated escaping.
    for words in [
        vec![key, "rotate", "--force"],
        vec![key, "import"],
        vec![key, "export"],
        vec![key],
        vec![key, "statuses"],
    ] {
        for shape in [one_line(&words), wrapped(&words)] {
            assert!(
                !names_only_key_status(&shape),
                "this must be refused, in every layout:\n{shape}"
            );
        }
    }

    // And a tab-indented wrap, since the rule skips whitespace rather than matching a newline.
    let tabbed = format!("[\n\t{},\n\t{},\n]", arg(key), arg("rotate"));
    assert!(!names_only_key_status(&tabbed), "{tabbed}");
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

/// A stand-in prikk **0.41.0** — so `worktree_status` asks for `--format json` — whose `worktree-status`
/// writes `stdout` to stdout and `stderr` to stderr and exits `code`: prikk's own dirty-exit shape (RFC
/// 008 finding 2). Both payloads are files the script `cat`s, so a JSON document never meets shell
/// quoting. Returns the backend and its temp dir, which the caller must clean up.
#[cfg(unix)]
fn fake_worktree_status_backend(
    name_suffix: &str,
    stdout: &str,
    stderr: &str,
    code: u8,
) -> (CliBackend, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!(
        "stikk-worktree-status-{name_suffix}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let out = dir.join("stdout.txt");
    let err = dir.join("stderr.txt");
    std::fs::write(&out, stdout).expect("write stdout payload");
    std::fs::write(&err, stderr).expect("write stderr payload");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             case \"$1\" in\n\
             --version)\n\
             printf 'prikk 0.41.0\\n'\n\
             ;;\n\
             worktree-status)\n\
             cat '{}'\n\
             cat '{}' >&2\n\
             exit {code}\n\
             ;;\n\
             esac\n",
            out.display(),
            err.display()
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

/// **RFC 027 B, review C1: a report stikk rejects is stikk's error, not prikk refusing.**
///
/// The report here is prikk 0.41.0's captured refused-symlink JSON with **one** rule broken —
/// `refused_count` disagreeing with the entries — derived by `variant()`, so every other byte is prikk's.
/// Before the fix, the JSON branch sent the reader's error to `classify`, which fell to its default
/// `Refusal` and preferred stderr: on a dirty tree the user saw prikk's `worktree has changes against the
/// baseline` as a refusal, and on a clean tree the raw JSON. **A document that parses is a report**, so
/// its rejection must surface as what it is — an environment error naming the rule (`UD-02`).
#[cfg(unix)]
#[test]
fn a_json_report_that_breaks_a_rule_is_an_environment_error_not_prikks_refusal() {
    let report = super::parse_json::tests::variant(
        super::parse_json::tests::WORKTREE_SYMLINK_JSON_0_41,
        r#""refused_count": 1"#,
        r#""refused_count": 0"#,
    );
    for (case, stderr, code) in [
        (
            "dirty",
            "error: worktree has changes against the baseline\n",
            1,
        ),
        ("clean", "", 0),
    ] {
        let (backend, dir) = fake_worktree_status_backend(case, &report, stderr, code);
        let result = retrying_transient_exec_busy(|| backend.worktree_status(&dir, "heads/main"));
        let _ = std::fs::remove_dir_all(&dir);
        let err = result.expect_err("a report that breaks a rule must not parse");
        assert_eq!(
            err.class(),
            "environment",
            "{case}: stikk rejecting prikk's report is not prikk refusing: {err:?}"
        );
        let text = err.to_string();
        assert!(
            text.contains("refused_count"),
            "{case}: the error names the rule: {text}"
        );
        assert!(
            !text.contains("worktree has changes"),
            "{case}: prikk's stderr is not the message: {text}"
        );
        assert!(
            !text.contains("schema_version"),
            "{case}: the raw report is not the message: {text}"
        );
    }
}

/// The other half of C1: **when stdout carries no JSON document, classification is exactly as before.**
/// A ref prikk cannot find leaves stdout empty and says why on stderr; that is prikk's refusal, and it
/// must still read as one, word for word.
#[cfg(unix)]
#[test]
fn a_worktree_status_with_no_json_on_stdout_still_classifies_exactly_as_before() {
    let stderr = "error: ref does not exist: heads/nope\n";
    let (backend, dir) = fake_worktree_status_backend("no-json", "", stderr, 1);
    let result = retrying_transient_exec_busy(|| backend.worktree_status(&dir, "heads/nope"));
    let _ = std::fs::remove_dir_all(&dir);
    let err = result.expect_err("no report, so no status");
    let expected = classify::classify("", stderr, RequestCategory::WorktreeAnalysis);
    assert_eq!(err.class(), expected.class());
    assert_eq!(err.to_string(), expected.to_string());
}

#[cfg(unix)]
#[test]
fn below_0_39_the_queue_is_unreported_and_asks_prikk_for_nothing_new() {
    // RFC 028 Handoff A §3: below 0.39 `queue` answers from the prose `status` read Orientation makes,
    // as an unreported list — never an empty one — and never asks for `status --format json`. A fake
    // `prikk` logs each call's first two arguments, so a `--format` flag cannot slip in unseen.
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-queue-unreported-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); // see the handshake test above for why
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$1 $2\" >> \"{log}\"\n\
             case \"$1\" in\n\
             --version)\n\
             echo 'prikk 0.38.0'\n\
             ;;\n\
             status)\n\
             printf 'prikk repository: /tmp/x/.prikk\\n\
             active WAL records: 2\\n\
             trailing partial WAL bytes: 0\\n\
             heads/main RefState: <not published>\\n\
             queued patches: 2 targeting heads/main\\n\
             status: multi-operation text diff minimization and plugins not yet implemented\\n'\n\
             ;;\n\
             esac\n",
            log = log.display(),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    let queue = retrying_transient_exec_busy(|| backend.queue(&dir)).expect("queue succeeds");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        queue,
        crate::QueueReport::Unreported {
            count: 2,
            target: Some("heads/main".to_string()),
        }
    );
    let mut lines: Vec<&str> = calls.lines().map(str::trim).collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        vec!["--version", "status"],
        "below 0.39 the queue must be read from prose `status` alone, never `--format json`"
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 034 decision 1: the retry for a ref prikk ≥ 0.45 calls absent, and — the part the real-binary
// suite cannot produce — every path that must return prikk's **original** refusal instead.
// ---------------------------------------------------------------------------------------------

/// prikk 0.46.0's own refusal for a ref with no published history, captured from the binary.
#[cfg(unix)]
const ABSENT_REF_REFUSAL: &str =
    "error: precondition not met: ref heads/main does not exist in this repository";

/// `prikk worktree-status --format json` at 0.46.0, in a fresh repository with one untracked file and
/// `heads/main` never published — the ref-less report the retry reads. Captured verbatim.
#[cfg(unix)]
const REFLESS_WORKTREE_0_46: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/probe/repo/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 0,
  "unchanged_files": 0,
  "clean": false,
  "refused_count": 0,
  "refused_declaration_count": 0,
  "queued_elsewhere": null,
  "changes": [
    {"path": "readme.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": []
}"#;

/// `prikk log --limit 200 --format json` at 0.46.0 in the same repository. Captured verbatim.
#[cfg(unix)]
const REFLESS_LOG_0_46: &str = r#"{
  "schema_version": "log-report-v1",
  "repository": "/tmp/probe/repo/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "blocks": []
}"#;

/// A stand-in prikk for the retry: it answers `--version` as 0.46.0, **refuses whatever carries
/// `--ref`** with `refusal`, and answers the ref-less retry with `worktree_retry` or `log_retry`
/// (shell snippets). Every invocation's argument list is appended to the returned calls file, so a test
/// can assert that **no retry ran at all**.
#[cfg(unix)]
struct StandIn {
    /// A real directory, used as the repository path: `Command::current_dir` on a path that does not
    /// exist fails the spawn itself, which would test nothing about the retry.
    dir: std::path::PathBuf,
    program: std::path::PathBuf,
    calls: std::path::PathBuf,
}

#[cfg(unix)]
impl StandIn {
    /// A backend whose handshake has already run. **Two reasons, both load-bearing:** it absorbs the
    /// transient ETXTBSY race against a just-written script (`retrying_transient_exec_busy` above —
    /// observed only under `cargo test --workspace`'s process churn, and seen once here), and it empties
    /// the calls log, so a test counting reads counts the call it is measuring and nothing else.
    fn warmed_backend(&self) -> CliBackend {
        let backend = CliBackend::with_program(&self.program);
        retrying_transient_exec_busy(|| backend.handshake())
            .expect("the stand-in reports a version");
        std::fs::write(&self.calls, "").expect("clear the calls log");
        backend
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(&self.calls)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

#[cfg(unix)]
fn stand_in(label: &str, refusal: &str, worktree_retry: &str, log_retry: &str) -> StandIn {
    use std::os::unix::fs::PermissionsExt;

    let dir =
        std::env::temp_dir().join(format!("stikk-prikk-rfc034-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the stand-in's directory");
    std::fs::write(dir.join("worktree.json"), REFLESS_WORKTREE_0_46).expect("write the report");
    std::fs::write(dir.join("log.json"), REFLESS_LOG_0_46).expect("write the report");
    // The same report with only its `ref` changed: what prikk would return if the retry read a
    // different ref than the one stikk asked for.
    std::fs::write(
        dir.join("worktree-other.json"),
        REFLESS_WORKTREE_0_46.replace(r#""ref": "heads/main""#, r#""ref": "heads/other""#),
    )
    .expect("write the report");
    std::fs::write(
        dir.join("log-other.json"),
        REFLESS_LOG_0_46.replace(r#""ref": "heads/main""#, r#""ref": "heads/other""#),
    )
    .expect("write the report");

    let calls = dir.join("calls.txt");
    let program = dir.join("fake-prikk.sh");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" >> '{calls}'\n\
             case \"$1\" in\n\
             --version) printf 'prikk 0.46.0\\n'; exit 0 ;;\n\
             esac\n\
             case \" $* \" in\n\
             *\" --ref \"*) printf '%s\\n' '{refusal}' 1>&2; exit 1 ;;\n\
             esac\n\
             case \"$1\" in\n\
             worktree-status) {worktree_retry} ;;\n\
             log) {log_retry} ;;\n\
             esac\n\
             exit 1\n",
            calls = calls.display(),
        ),
    )
    .expect("write the stand-in");
    let mut perms = std::fs::metadata(&program)
        .expect("stat the stand-in")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&program, perms).expect("make the stand-in executable");

    StandIn {
        dir: dir.clone(),
        program,
        calls,
    }
}

/// `worktree-status` exits 1 on a dirty tree, so the retry does too — the report is still on stdout.
#[cfg(unix)]
const SERVE_WORKTREE: &str = "cat \"$(dirname \"$0\")/worktree.json\"; exit 1";
#[cfg(unix)]
const SERVE_WORKTREE_OTHER: &str = "cat \"$(dirname \"$0\")/worktree-other.json\"; exit 1";
#[cfg(unix)]
const SERVE_LOG: &str = "cat \"$(dirname \"$0\")/log.json\"; exit 0";
#[cfg(unix)]
const SERVE_LOG_OTHER: &str = "cat \"$(dirname \"$0\")/log-other.json\"; exit 0";

#[cfg(unix)]
#[test]
fn worktree_status_retries_without_ref_and_accepts_a_report_naming_the_ref() {
    let prikk = stand_in("wt-ok", ABSENT_REF_REFUSAL, SERVE_WORKTREE, SERVE_LOG);
    let backend = prikk.warmed_backend();
    let status = backend
        .worktree_status(&prikk.dir, "heads/main")
        .expect("the ref-less retry's report names heads/main, so it is accepted");
    assert_eq!(status.reff, "heads/main");
    assert_eq!(status.entries.len(), 1, "{status:?}");
    assert!(!status.clean);
    // Exactly two reads: the one that refused, then the one without `--ref`.
    let reads: Vec<String> = prikk
        .calls()
        .into_iter()
        .filter(|call| call.starts_with("worktree-status"))
        .collect();
    let [refused, retry] = reads.as_slice() else {
        panic!("expected exactly two worktree-status reads: {reads:?}");
    };
    assert!(refused.contains("--ref heads/main"), "{reads:?}");
    assert!(
        !retry.contains("--ref"),
        "the retry drops the pair: {reads:?}"
    );
}

#[cfg(unix)]
#[test]
fn worktree_status_keeps_prikks_refusal_when_the_retry_names_another_ref() {
    // **The failure this repair must never have** (`T-T4`): a report for a ref the user did not ask
    // about. It is also every mistyped ref on prikk ≥ 0.45, where the retry returns the current
    // branch's report.
    let prikk = stand_in(
        "wt-other",
        ABSENT_REF_REFUSAL,
        SERVE_WORKTREE_OTHER,
        SERVE_LOG,
    );
    let backend = prikk.warmed_backend();
    let error = backend
        .worktree_status(&prikk.dir, "heads/main")
        .expect_err("a report naming another ref must not be shown");
    assert_eq!(error.class(), "refusal");
    assert!(
        error
            .to_string()
            .contains("does not exist in this repository"),
        "prikk's own refusal, verbatim: {error}"
    );
}

#[cfg(unix)]
#[test]
fn worktree_status_keeps_prikks_refusal_when_the_retry_refuses_or_does_not_parse() {
    for (label, retry) in [
        (
            "wt-refuses",
            "printf 'error: precondition not met: ref heads/main does not exist in this repository\\n' 1>&2; exit 1",
        ),
        ("wt-garbage", "printf 'not a report at all\\n'; exit 1"),
    ] {
        let prikk = stand_in(label, ABSENT_REF_REFUSAL, retry, SERVE_LOG);
        let backend = prikk.warmed_backend();
        let error = backend
            .worktree_status(&prikk.dir, "heads/main")
            .expect_err("a retry that gives no usable report leaves prikk's refusal standing");
        assert_eq!(error.class(), "refusal", "{label}: {error}");
        assert!(
            error
                .to_string()
                .contains("does not exist in this repository"),
            "{label}: prikk's original refusal, verbatim: {error}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_refusal_that_is_not_prikks_absent_ref_clause_is_never_retried() {
    // Matched on prikk's own clause, never on the `precondition not met:` prefix (`classify.rs`'s
    // standing rule): another precondition with the same prefix must reach the caller untouched, and
    // must not spend a second read.
    const OTHER: &str =
        "error: precondition not met: worktree has no node-addressed changes to commit";
    let prikk = stand_in("wt-other-refusal", OTHER, SERVE_WORKTREE, SERVE_LOG);
    let backend = prikk.warmed_backend();
    let error = backend
        .worktree_status(&prikk.dir, "heads/main")
        .expect_err("an unrelated refusal stays a refusal");
    assert!(
        error.to_string().contains("no node-addressed changes"),
        "{error}"
    );
    let reads = prikk
        .calls()
        .iter()
        .filter(|call| call.starts_with("worktree-status"))
        .count();
    assert_eq!(reads, 1, "no retry was spent: {:?}", prikk.calls());
}

#[cfg(unix)]
#[test]
fn history_retries_without_ref_and_accepts_a_report_naming_the_ref() {
    let prikk = stand_in("log-ok", ABSENT_REF_REFUSAL, SERVE_WORKTREE, SERVE_LOG);
    let backend = prikk.warmed_backend();
    let history = backend
        .history(&prikk.dir, "heads/main", 200)
        .expect("the ref-less retry's report names heads/main");
    assert_eq!(history.reff, "heads/main");
    assert!(
        history.blocks.is_empty(),
        "an unpublished ref has no blocks"
    );
    let reads: Vec<String> = prikk
        .calls()
        .into_iter()
        .filter(|call| call.starts_with("log"))
        .collect();
    let [_refused, retry] = reads.as_slice() else {
        panic!("expected exactly two log reads: {reads:?}");
    };
    assert!(
        !retry.contains("--ref"),
        "the retry drops the pair: {reads:?}"
    );
    assert!(
        retry.contains("--limit 200"),
        "everything else is identical: {reads:?}"
    );
}

#[cfg(unix)]
#[test]
fn history_keeps_prikks_refusal_when_the_retry_names_another_ref() {
    let prikk = stand_in(
        "log-other",
        ABSENT_REF_REFUSAL,
        SERVE_WORKTREE,
        SERVE_LOG_OTHER,
    );
    let backend = prikk.warmed_backend();
    let error = backend
        .history(&prikk.dir, "heads/main", 200)
        .expect_err("a history for another ref must not be shown");
    assert_eq!(error.class(), "refusal");
    assert!(
        error
            .to_string()
            .contains("does not exist in this repository"),
        "{error}"
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 035 — `show`'s band. The reader itself is tested in `parse_json/tests.rs`, against fixtures
// captured from a real 0.51.0 binary; what is tested here is what `show` *spawns*.
// ---------------------------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn below_0_39_show_is_unreported_and_spawns_nothing() {
    // RFC 035 §3: below stikk's JSON floor, `show` answers `Unreported` — never an empty patch list —
    // and asks prikk for nothing at all. A fake `prikk` logs every call, so a `show` cannot slip in
    // unseen; it answers `--version` and nothing else, which is also proof the band is read from the
    // cached handshake rather than from a `show` attempt that failed.
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-show-unreported-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$1 $2\" >> \"{log}\"\n\
             case \"$1\" in\n\
             --version)\n\
             echo 'prikk 0.38.0'\n\
             ;;\n\
             esac\n",
            log = log.display(),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    let id = "a".repeat(64);
    let report = retrying_transient_exec_busy(|| backend.show(&dir, &id)).expect("show succeeds");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(report, crate::ShowReport::Unreported);
    let lines: Vec<&str> = calls.lines().map(str::trim).collect();
    assert_eq!(
        lines,
        vec!["--version"],
        "below 0.39 nothing but the version probe may run"
    );
}

#[cfg(unix)]
#[test]
fn at_0_39_and_above_show_asks_for_json_with_the_id_it_was_given() {
    // The other half: the band's upper side spawns exactly one `show <id> --format json`, with the id
    // the caller chose — a block id for a sealed patch, a patch id for a queued one (RFC 035 F2). The
    // id is never rewritten here, because only the caller knows which it has.
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-show-json-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$*\" >> \"{log}\"\n\
             case \"$1\" in\n\
             --version)\n\
             echo 'prikk 0.51.0'\n\
             ;;\n\
             show)\n\
             printf '{{\"schema_version\":\"show-report-v1\",\"patches\":[{{\"patch_id\":\"%s\",\
             \"queued\":true,\"operations\":[]}}]}}\\n' \"$2\"\n\
             ;;\n\
             esac\n",
            log = log.display(),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    let id = "b".repeat(64);
    let report = retrying_transient_exec_busy(|| backend.show(&dir, &id)).expect("show succeeds");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        report,
        crate::ShowReport::Listed(vec![crate::ShownPatch {
            patch_id: id.clone(),
            queued: true,
            operations: Vec::new(),
        }]),
        "prikk's own `queued`, and a patch with no operations is a patch, not a broken report"
    );
    let mut lines: Vec<&str> = calls.lines().map(str::trim).collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        vec!["--version", &format!("show {id} --format json")]
    );
}

// -------------------------------------------------------------------------------------------
// RFC 040 decision 1: the exit-code rule, one test per row.
//
// This is the one place in the seam where a non-zero exit can be success, so each row of the rule
// gets its own test. A fake prikk stands in because a real one cannot be made to exit 1 *without*
// a report on demand -- and the damage cases that do exit 1 with one are covered for real by the
// suite leg at 0.51.

/// prikk sound, at 0.51.0 (captured; the reader's tests hold the same text).
#[cfg(unix)]
const FAKE_SOUND: &str = r##"{
  "schema_version": "verify-report-v1",
  "verdict": {
    "ok": true,
    "failed_conditions": []
  },
  "active_sessions": {"count": 1, "verified_count": 1},
  "stages": [
    {"stage": "objects", "status": "evaluated"},
    {"stage": "refs", "status": "evaluated"},
    {"stage": "ref-update-schema-trust", "status": "evaluated"},
    {"stage": "wal-replay", "status": "evaluated"},
    {"stage": "wal-persistence", "status": "evaluated"},
    {"stage": "rollback-drafts", "status": "evaluated"},
    {"stage": "wal-record-schema", "status": "evaluated"},
    {"stage": "active-wal-metadata", "status": "evaluated"},
    {"stage": "commit-witness", "status": "evaluated"},
    {"stage": "publication-reclassification", "status": "evaluated"},
    {"stage": "commit-index", "status": "evaluated"},
    {"stage": "lifecycle-cache", "status": "evaluated"},
    {"stage": "wal-ordering", "status": "evaluated"},
    {"stage": "received-refs", "status": "evaluated"},
    {"stage": "local-tag-trust", "status": "evaluated"},
    {"stage": "object-connectivity", "status": "evaluated"},
    {"stage": "appended-file-tails", "status": "evaluated"}
  ]
}"##;

/// prikk's `item-failure` report, at 0.51.0 (captured) -- the exit-1-with-a-report case.
#[cfg(unix)]
const FAKE_ITEM_FAILURE: &str = r##"{
  "schema_version": "verify-report-v1",
  "verdict": {
    "ok": false,
    "failed_conditions": [
      {"id": "item-failure", "message": "repository verification found at least one failed object, block, or ref; see item outcomes above"}
    ]
  },
  "active_sessions": {"count": 1, "verified_count": 1},
  "stages": [
    {"stage": "objects", "status": "evaluated"},
    {"stage": "refs", "status": "evaluated"},
    {"stage": "ref-update-schema-trust", "status": "evaluated"},
    {"stage": "wal-replay", "status": "evaluated"},
    {"stage": "wal-persistence", "status": "evaluated"},
    {"stage": "rollback-drafts", "status": "evaluated"},
    {"stage": "wal-record-schema", "status": "evaluated"},
    {"stage": "active-wal-metadata", "status": "evaluated"},
    {"stage": "commit-witness", "status": "evaluated"},
    {"stage": "publication-reclassification", "status": "evaluated"},
    {"stage": "commit-index", "status": "evaluated"},
    {"stage": "lifecycle-cache", "status": "evaluated"},
    {"stage": "wal-ordering", "status": "evaluated"},
    {"stage": "received-refs", "status": "evaluated"},
    {"stage": "local-tag-trust", "status": "evaluated"},
    {"stage": "object-connectivity", "status": "evaluated"},
    {"stage": "appended-file-tails", "status": "evaluated"}
  ]
}"##;

/// A fake `prikk` that answers `--version` with 0.51.0 and `verify` with `stdout`/`stderr`/`code`.
#[cfg(unix)]
fn fake_verify_prikk(tag: &str, stdout: &str, stderr: &str, code: i32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-verify-{tag}-{}", std::process::id()));
    // See the handshake test above for why this is defensive rather than incidental.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.51.0'; exit 0; fi\n\
             cat <<'STIKK_FIXTURE_EOF'\n\
             {stdout}\n\
             STIKK_FIXTURE_EOF\n\
             printf '%s' {stderr} 1>&2\n\
             exit {code}\n",
            stdout = stdout,
            stderr = shell_single_quote(stderr),
            code = code,
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");
    script
}

#[cfg(unix)]
fn shell_single_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// **Row 1: exit 0 with a report is a report.**
#[cfg(unix)]
#[test]
fn verify_exit_0_with_a_report_is_a_report() {
    let script = fake_verify_prikk("exit0", FAKE_SOUND, "", 0);
    let backend = CliBackend::with_program(&script);
    let report = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect("exit 0 with a report reads");
    match report {
        crate::VerifyReport::Listed(v) => {
            assert!(v.verdict.ok);
            assert_eq!(v.stages.len(), 17);
        }
        crate::VerifyReport::Unreported => panic!("0.51 is above the band"),
    }
}

/// **Row 2: exit 1 with a report is a report, not a refusal** — the whole point of RFC 040 F2.
/// stderr repeats the condition, and is deliberately ignored.
#[cfg(unix)]
#[test]
fn verify_exit_1_with_a_report_is_a_report_not_a_refusal() {
    let script = fake_verify_prikk(
        "exit1ok",
        FAKE_ITEM_FAILURE,
        "error: repository verification found at least one failed object, block, or ref; see item outcomes above",
        1,
    );
    let backend = CliBackend::with_program(&script);
    let report = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect("exit 1 with a report is NOT an error");
    match report {
        crate::VerifyReport::Listed(v) => {
            assert!(!v.verdict.ok);
            assert_eq!(v.verdict.failed_conditions.len(), 1);
            assert_eq!(v.verdict.failed_conditions[0].id, "item-failure");
            // Every stage still reported, as measured.
            assert_eq!(v.stages.len(), 17);
        }
        crate::VerifyReport::Unreported => panic!("0.51 is above the band"),
    }
}

/// **Row 3: exit 1 whose stdout is not a report is prikk's refusal, verbatim.**
#[cfg(unix)]
#[test]
fn verify_exit_1_without_a_report_is_prikks_refusal() {
    let script = fake_verify_prikk("exit1bad", "", "error: no prikk repository here", 1);
    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect_err("exit 1 without a report is an error");
    assert_eq!(err.class(), "refusal");
    // **prikk's words, not stikk's parse failure.** Saying "stikk could not read the report" here
    // would bury the reason the reader actually needs.
    assert!(
        err.to_string().contains("no prikk repository here"),
        "{err}"
    );
}

/// **Row 3b: at exit 1, a report stikk cannot read is stikk's error, not prikk's refusal**
/// (RFC 040 review v1 §2).
///
/// prikk exits 1 both when it refuses to verify and when it verified and found damage. Only the
/// second prints a report — so an unreadable stdout that *is* a `verify-report-v1` means prikk
/// verified a repository and stikk could not understand the answer. Calling that *prikk refused*
/// would say the one thing Verify must never say about a damaged repository, and would do it
/// while prikk's own condition sat unread in stderr.
///
/// The case used here is the one that can really happen: `ok` disagreeing with its own
/// conditions, which the reader refuses rather than choosing between.
#[cfg(unix)]
#[test]
fn verify_exit_1_with_an_unreadable_report_is_stikks_error_not_a_refusal() {
    let unreadable = FAKE_ITEM_FAILURE.replace(r#""ok": false"#, r#""ok": true"#);
    let script = fake_verify_prikk(
        "exit1unreadable",
        &unreadable,
        "error: repository verification found at least one failed object, block, or ref; see item outcomes above",
        1,
    );
    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect_err("a report stikk cannot read is an error");
    assert_eq!(
        err.class(),
        "environment",
        "a verify-report-v1 stikk cannot read must never be classified as prikk's refusal: {err}"
    );
    // It names the rule that broke, rather than echoing prikk's stderr as a refusal.
    assert!(
        err.to_string().contains("failing condition"),
        "the error should name the broken rule: {err}"
    );
}

/// **Row 4: exit 2 is stikk's bug**, as everywhere else in the seam.
#[cfg(unix)]
#[test]
fn verify_exit_2_is_stikks_bug() {
    let script = fake_verify_prikk("exit2", "", "error: unknown flag --format", 2);
    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect_err("exit 2 is an error");
    assert_eq!(err.class(), "stikk-internal");
}

/// **Any other code classifies as it does today** — not as a report, and not as stikk's bug.
#[cfg(unix)]
#[test]
fn verify_any_other_exit_code_classifies_as_today() {
    // Even carrying a perfectly good report on stdout: the rule is exit 0 or exit 1, and widening
    // it by accident is exactly the drift this test exists to catch.
    let script = fake_verify_prikk("exit3", FAKE_SOUND, "error: something else went wrong", 3);
    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| backend.verify(std::path::Path::new(".")))
        .expect_err("exit 3 is an error");
    assert_eq!(err.class(), "refusal");
    assert!(
        err.to_string().contains("something else went wrong"),
        "{err}"
    );
}

/// **Below the band nothing is spawned**, and *unreported* is not *sound*.
#[cfg(unix)]
#[test]
fn verify_below_the_band_spawns_nothing_and_is_not_sound() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-verify-band-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$1\" >> \"{log}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.38.0'; exit 0; fi\n\
             exit 0\n",
            log = log.display(),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");

    let backend = CliBackend::with_program(&script);
    let report = retrying_transient_exec_busy(|| backend.verify(&dir)).expect("below the band");
    assert_eq!(report, crate::VerifyReport::Unreported);
    // The handshake ran; `verify` never did.
    let calls = std::fs::read_to_string(&log).expect("read the call log");
    assert!(
        !calls.contains("verify"),
        "verify must not be spawned: {calls:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// RFC 044: the three ref-writing commands.
//
// **The argument vector is the thing to pin.** These commands have no `--format json`, so the
// only machine-checkable facts about a call are the arguments stikk builds -- and three of RFC
// 044's traps live there: `--from` must always be present, `-m` must be absent for an empty
// message, and a name must arrive as one argument even when it holds a space. A fake prikk that
// records its own argv is how those become test failures rather than review findings.

/// A fake `prikk` answering `--version` with 0.52.0 and `key status` as a ready maintainer, which
/// logs every call's argv one-per-line to `argv`, and answers anything else with `lines`.
#[cfg(unix)]
fn fake_ref_write_prikk(
    tag: &str,
    lines: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-refwrite-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.52.0'; exit 0; fi\n\
             if [ \"$2\" = \"status\" ]; then\n\
             printf '%s\\n' '{{\"schema_version\":\"key-status-v1\",\"roles\":[{{\"role\":\"author\",\"usable\":true,\"key_id\":\"a\",\"binding\":\"matches\"}},{{\"role\":\"maintainer\",\"usable\":true,\"key_id\":\"m\",\"binding\":\"matches\"}}]}}'\n\
             exit 0\n\
             fi\n\
             : > \"{argv}\"\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             printf '%s' '{lines}'\n\
             exit {code}\n",
            argv = argv.display(),
            lines = lines,
            code = code,
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");
    (script, argv)
}

#[cfg(unix)]
fn recorded_argv(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("the fake prikk recorded its argv")
        .lines()
        .map(ToString::to_string)
        .collect()
}

/// **`--from` is always passed** (RFC 044 §6 trap 1), and the name is **one** argument.
#[cfg(unix)]
#[test]
fn create_branch_always_passes_from_and_keeps_a_spaced_name_whole() {
    let (script, argv) = fake_ref_write_prikk("branch", "created branch heads/bad name\n", 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.create_branch(std::path::Path::new("."), "heads/bad name", "heads/main")
    })
    .expect("a create reads");
    assert_eq!(
        out,
        crate::RefWrite::Done(vec!["created branch heads/bad name".to_string()])
    );
    assert_eq!(
        recorded_argv(&argv),
        vec!["branch", "create", "heads/bad name", "--from", "heads/main"],
        "`--from` must be present and the name must stay one argument"
    );
}

/// **An empty *or blank* message sends no `-m`**, and a non-empty one sends exactly one.
///
/// The blank case is prikk's rule, not stikk's: `tag create -m "   "` is `error: tag message must
/// not be empty`, exit 2 — a *usage* error, which would reach a reader as an internal fault
/// (measured at 0.52.0; `prikk-cli/src/tag.rs:280`). There is no second kind of tag to choose
/// between here, so a blank message is a tag with none.
#[cfg(unix)]
#[test]
fn create_tag_sends_m_only_for_a_non_empty_message() {
    for (label, message) in [("tagempty", ""), ("tagblank", "   "), ("tagtab", "\t\n")] {
        let (script, argv) = fake_ref_write_prikk(label, "created tag tags/v1\n", 0);
        let backend = CliBackend::with_program(&script);
        retrying_transient_exec_busy(|| {
            backend.create_tag(std::path::Path::new("."), "tags/v1", "heads/main", message)
        })
        .expect("a tag with no message");
        assert_eq!(
            recorded_argv(&argv),
            vec!["tag", "create", "tags/v1", "--target", "heads/main"],
            "{message:?} must send no -m at all"
        );
    }

    let (script, argv) = fake_ref_write_prikk("tagmsg", "created tag tags/v2\n", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.create_tag(
            std::path::Path::new("."),
            "tags/v2",
            "heads/main",
            "first release",
        )
    })
    .expect("a tag with a message");
    assert_eq!(
        recorded_argv(&argv),
        vec![
            "tag",
            "create",
            "tags/v2",
            "--target",
            "heads/main",
            "-m",
            "first release"
        ],
        "the message must be one argument, with its space"
    );
}

/// A block id is a legal `--target`, unlike `--from` (F2).
#[cfg(unix)]
#[test]
fn create_tag_takes_a_block_id_as_its_target() {
    let (script, argv) = fake_ref_write_prikk("tagblock", "created tag tags/old\n", 0);
    let backend = CliBackend::with_program(&script);
    let block = "e".repeat(64);
    retrying_transient_exec_busy(|| {
        backend.create_tag(std::path::Path::new("."), "tags/old", &block, "")
    })
    .expect("a tag at a block");
    assert_eq!(recorded_argv(&argv)[3], "--target");
    assert_eq!(recorded_argv(&argv)[4], block);
}

#[cfg(unix)]
#[test]
fn close_branch_passes_the_name_as_one_argument() {
    let (script, argv) = fake_ref_write_prikk("close", "closed branch heads/old\n", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| backend.close_branch(std::path::Path::new("."), "heads/old"))
        .expect("a close");
    assert_eq!(recorded_argv(&argv), vec!["branch", "close", "heads/old"]);
}

/// **Every printed line is kept, in order, and none is parsed** (§6 trap 2).
#[cfg(unix)]
#[test]
fn a_ref_writes_printed_lines_are_kept_verbatim_and_in_order() {
    let printed = "closed branch heads/old\nRefState: abc123\nnothing was reclaimed; the pointer, its history, and every object remain, and the branch is recoverable\n";
    let (script, _argv) = fake_ref_write_prikk("verbatim", printed, 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.close_branch(std::path::Path::new("."), "heads/old")
    })
    .expect("a close");
    assert_eq!(
        out,
        crate::RefWrite::Done(vec![
            "closed branch heads/old".to_string(),
            "RefState: abc123".to_string(),
            "nothing was reclaimed; the pointer, its history, and every object remain, and the branch is recoverable".to_string(),
        ])
    );
}

/// **A refusal is prikk's, verbatim** (`ER-02`), through the usual classification.
#[cfg(unix)]
#[test]
fn a_ref_writes_refusal_carries_prikks_words() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-refwrite-refuse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.52.0'; exit 0; fi\n\
         if [ \"$2\" = \"status\" ]; then\n\
         printf '%s\\n' '{\"schema_version\":\"key-status-v1\",\"roles\":[{\"role\":\"author\",\"usable\":true,\"key_id\":\"a\",\"binding\":\"matches\"},{\"role\":\"maintainer\",\"usable\":true,\"key_id\":\"m\",\"binding\":\"matches\"}]}'\n\
         exit 0\n\
         fi\n\
         echo 'error: branch heads/feature already exists' 1>&2\n\
         exit 1\n",
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make it executable");

    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| {
        backend.create_branch(std::path::Path::new("."), "heads/feature", "heads/main")
    })
    .expect_err("a duplicate name is a refusal");
    assert_eq!(err.class(), "refusal");
    assert!(
        err.to_string()
            .contains("branch heads/feature already exists"),
        "{err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **Below the band nothing is spawned**, and *unavailable* is not *failed*.
#[cfg(unix)]
#[test]
fn below_the_band_a_ref_write_spawns_nothing() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-refwrite-band-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let log = dir.join("calls");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             echo \"$1\" >> \"{log}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.41.0'; exit 0; fi\n\
             exit 0\n",
            log = log.display(),
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make it executable");

    let backend = CliBackend::with_program(&script);
    for out in [
        retrying_transient_exec_busy(|| backend.create_branch(&dir, "heads/x", "heads/main")),
        retrying_transient_exec_busy(|| backend.create_tag(&dir, "tags/x", "heads/main", "")),
        retrying_transient_exec_busy(|| backend.close_branch(&dir, "heads/x")),
    ] {
        assert_eq!(
            out.expect("below the band is not an error"),
            crate::RefWrite::Unavailable
        );
    }
    let calls = std::fs::read_to_string(&log).expect("read the call log");
    assert!(!calls.contains("branch"), "no branch command: {calls:?}");
    assert!(!calls.contains("tag"), "no tag command: {calls:?}");
    // Not even a readiness read: the band is checked first.
    assert!(
        !calls.contains("status"),
        "no readiness read either: {calls:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// =============================================================================================
// RFC 045 — `branch switch`.
//
// The same shape as RFC 044's tests above, with two differences the command itself forces: there
// are **no flags at all** to get wrong, and **stderr at exit 0 is content**, not an error.
// =============================================================================================

/// A fake `prikk` answering `--version` with `version` and `key status` as a ready maintainer,
/// logging every call's argv, and answering anything else with `out` on stdout and `err` on
/// stderr at `code`.
#[cfg(unix)]
fn fake_switch_prikk(
    tag: &str,
    version: &str,
    out: &str,
    err: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-switch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             : >> \"{argv}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk {version}'; exit 0; fi\n\
             if [ \"$2\" = \"status\" ]; then\n\
             printf '%s\\n' '{{\"schema_version\":\"key-status-v1\",\"roles\":[{{\"role\":\"author\",\"usable\":true,\"key_id\":\"a\",\"binding\":\"matches\"}},{{\"role\":\"maintainer\",\"usable\":true,\"key_id\":\"m\",\"binding\":\"matches\"}}]}}'\n\
             exit 0\n\
             fi\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             printf '%s' '{out}'\n\
             printf '%s' '{err}' 1>&2\n\
             exit {code}\n",
            argv = argv.display(),
            version = version,
            out = out,
            err = err,
            code = code,
        ),
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make the script executable");
    (script, argv)
}

/// **One argument, no flags, and a space survives** (RFC 045 §6 trap 5; F1: any argument starting
/// with `-` is a usage error, so there is nothing else to send).
#[cfg(unix)]
#[test]
fn switch_branch_sends_one_argument_and_no_flags() {
    let (script, argv) = fake_switch_prikk(
        "args",
        "0.52.0",
        "switched from heads/main to heads/bad name\n",
        "",
        0,
    );
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.switch_branch(std::path::Path::new("."), "heads/bad name")
    })
    .expect("a switch");
    assert_eq!(
        recorded_argv(&argv),
        vec!["branch", "switch", "heads/bad name"],
        "no flags, and the name stays one argument"
    );
}

/// **Both streams are kept, and kept apart** (decision 1). prikk can name a snapshot fallback on
/// stderr at exit 0; merging it into the counts would put a caveat inside a result.
#[cfg(unix)]
#[test]
fn a_switch_keeps_stdout_and_stderr_apart_at_exit_zero() {
    let (script, _argv) = fake_switch_prikk(
        "streams",
        "0.52.0",
        "switched from heads/main to heads/feature\nfiles written: 2\nfiles unchanged: 0\nfiles deleted: 1\n",
        "note: replayed from the snapshot fallback\n",
        0,
    );
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.switch_branch(std::path::Path::new("."), "heads/feature")
    })
    .expect("a switch");
    assert_eq!(
        out,
        crate::SwitchResult::Done {
            printed: vec![
                "switched from heads/main to heads/feature".to_string(),
                "files written: 2".to_string(),
                "files unchanged: 0".to_string(),
                "files deleted: 1".to_string(),
            ],
            noted: vec!["note: replayed from the snapshot fallback".to_string()],
        }
    );
}

/// **An ordinary switch notes nothing**, so the result card shows no empty stderr section.
#[cfg(unix)]
#[test]
fn an_ordinary_switch_has_no_notes() {
    let (script, _argv) = fake_switch_prikk(
        "quiet",
        "0.52.0",
        "switched from heads/main to heads/feature\nfiles written: 0\n",
        "",
        0,
    );
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.switch_branch(std::path::Path::new("."), "heads/feature")
    })
    .expect("a switch");
    match out {
        crate::SwitchResult::Done { noted, .. } => assert!(noted.is_empty(), "{noted:?}"),
        other => panic!("expected Done, got {other:?}"),
    }
}

/// **Each F2 refusal is prikk's, verbatim**, through the usual classification — and prikk writes
/// nothing for any of them.
#[cfg(unix)]
#[test]
fn every_switch_refusal_carries_prikks_words() {
    for (tag, message) in [
        (
            "missing",
            "error: precondition not met: heads/nope does not exist; run `prikk branch create heads/nope` first",
        ),
        (
            "closed",
            "error: precondition not met: heads/old is closed; switch to an open branch (`prikk branch list` shows them)",
        ),
        (
            "dirty",
            "error: precondition not met: the worktree is not clean against heads/main: a.txt (modified); commit it, or restore those files, before switching (`prikk worktree-status` lists them)",
        ),
        (
            "intheway",
            "error: precondition not met: refusing to switch to heads/feature: 1 in the way of its files: c.txt (a file that is not part of the branch being left); move them aside and run the switch again (nothing was written)",
        ),
        (
            "queuedhere",
            "error: precondition not met: the active WAL holds unsealed work for heads/main; seal it (`prikk seal --allow-no-audit --ref heads/main`) before switching to heads/feature",
        ),
    ] {
        let (script, _argv) = fake_switch_prikk(tag, "0.52.0", "", &format!("{message}\n"), 1);
        let backend = CliBackend::with_program(&script);
        let err = retrying_transient_exec_busy(|| {
            backend.switch_branch(std::path::Path::new("."), "heads/feature")
        })
        .expect_err("a refusal");
        assert_eq!(err.class(), "refusal", "{tag}: {err}");
        assert!(err.to_string().contains(message), "{tag}: {err}");
    }
}

/// **Below the band nothing is spawned** — not even the readiness read — and *unavailable* is not
/// *failed*. The command exists from 0.42; the band is `DIFF_FROM_MINOR`, because a switch
/// without its file plan is not offered (decision 4).
#[cfg(unix)]
#[test]
fn below_the_band_a_switch_spawns_nothing() {
    let (script, argv) = fake_switch_prikk("band", "0.45.0", "switched\n", "", 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.switch_branch(std::path::Path::new("."), "heads/feature")
    })
    .expect("below the band is not an error");
    assert_eq!(out, crate::SwitchResult::Unavailable);
    assert!(
        recorded_argv(&argv).is_empty(),
        "no branch command and no readiness read: {:?}",
        recorded_argv(&argv)
    );
    assert_eq!(
        crate::DIFF_FROM_MINOR,
        46,
        "the band is the one `diff` already uses, never a second literal"
    );
}

/// **No key is required** (decision 2; F1 measured with every key variable unset). The fake here
/// reports *no* usable role, and the switch still runs: a maintainer-key refusal would be a
/// requirement stikk invented.
#[cfg(unix)]
#[test]
fn a_switch_needs_no_signing_key() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-switch-nokey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'prikk 0.52.0'; exit 0; fi\n\
         if [ \"$2\" = \"status\" ]; then\n\
         printf '%s\\n' '{\"schema_version\":\"key-status-v1\",\"roles\":[{\"role\":\"author\",\"usable\":false,\"key_id\":null,\"binding\":\"unknown\"},{\"role\":\"maintainer\",\"usable\":false,\"key_id\":null,\"binding\":\"unknown\"}]}'\n\
         exit 0\n\
         fi\n\
         echo 'switched from heads/main to heads/feature'\n\
         exit 0\n",
    )
    .expect("write the fake prikk script");
    let mut perms = std::fs::metadata(&script)
        .expect("stat the script")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("make it executable");

    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| backend.switch_branch(&dir, "heads/feature"))
        .expect("a session with no key may still switch");
    assert_eq!(
        out,
        crate::SwitchResult::Done {
            printed: vec!["switched from heads/main to heads/feature".to_string()],
            noted: Vec::new(),
        }
    );

    let _ = std::fs::remove_dir_all(&dir);
}

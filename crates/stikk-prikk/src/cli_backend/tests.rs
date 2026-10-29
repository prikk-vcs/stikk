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

// =============================================================================================
// RFC 047 — `merge-plan`.
//
// The plan is prose with no JSON form (asking for one is exit 2), so what the seam owes is the
// argument vector, the lines kept whole, and **one line read** — `evidence outcome:` — with every
// way of reading it wrong turned into a refusal rather than a guess.
// =============================================================================================

/// The confluent plan prikk 0.53.0 printed for scenario 1, from the §1 re-run
/// (`.git-exclude/reports/047-merge/measure047-0.53.txt`), ids shortened for the fixture.
const CONFLUENT_PLAN: &str = "merge plan\n\
     baseline block: 88f34150e099\n\
     left selector: ref heads/main\n\
     left target block: 6d17f5f225b7\n\
     left operations: 1\n\
     right selector: ref heads/feature\n\
     right target block: d967f47871e7\n\
     right operations: 1\n\
     status: ConfluentSubset\n\
     evidence outcome: Confluent\n\
     reason: proven_confluent\n\
     action: review the evidence, then run 'prikk merge' to execute\n\
     items: 1 displayed of 1\n\
     \n\
     report:\n\
     \x20 outcome: Confluent\n\
     \x20 reason: proven_confluent\n\
     \x20 phase: final-state-comparison\n\
     note: read-only plan; no merge commit, ref update, WAL write, object write, or worktree \
     change was performed\n";

/// Scenario 3's conflicting plan, from the same run.
///
/// **The `cross:` block's `left[N]` index is deliberately not asserted anywhere.** It is not
/// stable: five fresh runs at 0.52 gave `left[0] left[0] left[1] left[1] left[1]` and five at
/// 0.53 gave `left[1] left[0] left[0] left[1] left[1]`
/// (`measure047-0.53-note.txt`). stikk shows that block verbatim and reads nothing from it.
const CONFLICT_PLAN: &str = "merge plan\n\
     baseline block: 2237a4316d32\n\
     left selector: ref heads/main\n\
     left target block: 5b59d8a49c98\n\
     right selector: ref heads/feature\n\
     right target block: 3af8f309a4be\n\
     status: BlockedConflict\n\
     evidence outcome: Conflict\n\
     reason: pair_conflict\n\
     action: inspect evidence; conflict resolution is not implemented\n\
     items: 1 displayed of 1\n\
     \n\
     cross:\n\
     \x20 left[0] op_seq=1 EditText\n\
     \x20 right[0] op_seq=1 EditText\n\
     \x20 outcome: Conflict\n\
     \x20 reason: pair_conflict\n\
     \x20 phase: classification\n\
     note: read-only plan; no merge commit, ref update, WAL write, object write, or worktree \
     change was performed\n";

/// A fake `prikk` at `version` that records its argv and prints `out`, exiting `code`.
#[cfg(unix)]
fn fake_plan_prikk(
    tag: &str,
    version: &str,
    out: &str,
    err: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-mergeplan-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let payload = dir.join("payload");
    std::fs::write(&payload, out).expect("write the payload");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             : >> \"{argv}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk {version}'; exit 0; fi\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             cat \"{payload}\"\n\
             printf '%s' '{err}' 1>&2\n\
             exit {code}\n",
            argv = argv.display(),
            payload = payload.display(),
            version = version,
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

/// **The argument vector**, and no `--format json`: asking for one is a usage error (F1).
#[cfg(unix)]
#[test]
fn merge_plan_sends_the_three_arguments_and_no_format_flag() {
    let (script, argv) = fake_plan_prikk("args", "0.53.0", CONFLUENT_PLAN, "", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.merge_plan(
            std::path::Path::new("."),
            "88f34150e099",
            "heads/main",
            "heads/feature",
        )
    })
    .expect("a plan reads");
    assert_eq!(
        recorded_argv(&argv),
        vec![
            "merge-plan",
            "--baseline-block",
            "88f34150e099",
            "--left-ref",
            "heads/main",
            "--right-ref",
            "heads/feature"
        ]
    );
}

/// **prikk's lines are kept whole**, and the one line is read apart from them.
#[cfg(unix)]
#[test]
fn a_plans_lines_are_kept_verbatim_and_the_verdict_read_apart() {
    for (tag, plan, want) in [
        ("confluent", CONFLUENT_PLAN, crate::MergeOutcome::Confluent),
        ("conflict", CONFLICT_PLAN, crate::MergeOutcome::Conflict),
    ] {
        let (script, _argv) = fake_plan_prikk(tag, "0.53.0", plan, "", 0);
        let backend = CliBackend::with_program(&script);
        let out = retrying_transient_exec_busy(|| {
            backend.merge_plan(
                std::path::Path::new("."),
                "b",
                "heads/main",
                "heads/feature",
            )
        })
        .expect("a plan reads");
        let crate::MergePlan::Read { printed, outcome } = out else {
            panic!("{tag}: 0.53 is above the band");
        };
        assert_eq!(outcome, want, "{tag}");
        assert_eq!(
            printed,
            plan.lines()
                .map(str::trim_end)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>(),
            "{tag}: every line prikk printed, in order"
        );
        // **A refusing plan is still a successful read** (F1): exit 0, and prikk says so itself.
        assert!(
            printed.last().is_some_and(|l| l.contains("read-only plan")),
            "{tag}: {printed:?}"
        );
    }
}

/// **The one line read, and its three failure shapes** — each a refusal, never a guess (`UD-02`).
#[cfg(unix)]
#[test]
fn an_unreadable_verdict_is_a_shape_error_not_a_guess() {
    for (tag, plan, fragment) in [
        (
            "missing",
            "merge plan\nstatus: ConfluentSubset\nreason: proven_confluent\n",
            "carries no `evidence outcome:` line",
        ),
        (
            "repeated",
            "merge plan\nevidence outcome: Confluent\nevidence outcome: Conflict\n",
            "more than one `evidence outcome:` line",
        ),
        (
            "a word stikk does not know",
            "merge plan\nevidence outcome: Marvellous\n",
            "an outcome stikk does not know",
        ),
    ] {
        let (script, _argv) = fake_plan_prikk(&tag.replace(' ', "-"), "0.53.0", plan, "", 0);
        let backend = CliBackend::with_program(&script);
        let error = retrying_transient_exec_busy(|| {
            backend.merge_plan(
                std::path::Path::new("."),
                "b",
                "heads/main",
                "heads/feature",
            )
        })
        .expect_err("{tag}");
        assert_eq!(error.class(), "environment", "{tag}: {error}");
        assert!(error.to_string().contains(fragment), "{tag}: {error}");
    }
}

/// **An unknown word is shown inert**, because it is prikk's text reaching a message.
#[cfg(unix)]
#[test]
fn an_unknown_verdict_is_escaped_in_the_message() {
    let (script, _argv) = fake_plan_prikk(
        "inert",
        "0.53.0",
        "merge plan\nevidence outcome: Conf\u{202e}luent\n",
        "",
        0,
    );
    let backend = CliBackend::with_program(&script);
    let error = retrying_transient_exec_busy(|| {
        backend.merge_plan(
            std::path::Path::new("."),
            "b",
            "heads/main",
            "heads/feature",
        )
    })
    .expect_err("an unknown word");
    assert!(error.to_string().contains("\\u202e"), "{error}");
}

/// **Each refusal is prikk's, verbatim** — a baseline that is not an ancestor, and one no block
/// names (F2).
#[cfg(unix)]
#[test]
fn every_merge_plan_refusal_carries_prikks_words() {
    for (tag, message) in [
        (
            "not-ancestor",
            "error: precondition not met: baseline Block 5b59d8a4 is not an ancestor of target Block 3af8f309",
        ),
        (
            "absent",
            "error: precondition not met: block 0000000000000000 is not in this repository",
        ),
    ] {
        let (script, _argv) = fake_plan_prikk(tag, "0.53.0", "", &format!("{message}\n"), 1);
        let backend = CliBackend::with_program(&script);
        let error = retrying_transient_exec_busy(|| {
            backend.merge_plan(
                std::path::Path::new("."),
                "b",
                "heads/main",
                "heads/feature",
            )
        })
        .expect_err("a refusal");
        assert_eq!(error.class(), "refusal", "{tag}: {error}");
        assert!(error.to_string().contains(message), "{tag}: {error}");
    }
}

/// **Below the band nothing is spawned**, and *unavailable* is not *failed*.
#[cfg(unix)]
#[test]
fn below_the_band_a_merge_plan_spawns_nothing() {
    let (script, argv) = fake_plan_prikk("band", "0.41.0", CONFLUENT_PLAN, "", 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.merge_plan(
            std::path::Path::new("."),
            "b",
            "heads/main",
            "heads/feature",
        )
    })
    .expect("below the band is not an error");
    assert_eq!(out, crate::MergePlan::Unavailable);
    assert!(
        recorded_argv(&argv).is_empty(),
        "no merge-plan was run: {:?}",
        recorded_argv(&argv)
    );
}

/// prikk's eight outcome words, from `MergeEvidenceOutcome` and the merge guide, round-trip.
#[test]
fn prikks_eight_outcomes_round_trip_and_only_one_merges() {
    let all = [
        "Confluent",
        "Conflict",
        "OrderedDependency",
        "Unsupported",
        "Deferred",
        "NotConfluent",
        "EvidenceFailure",
        "InvalidCandidate",
    ];
    for word in all {
        let outcome = crate::MergeOutcome::parse(word).unwrap_or_else(|| panic!("{word}"));
        assert_eq!(outcome.name(), word);
        assert_eq!(outcome.is_confluent(), word == "Confluent", "{word}");
    }
    assert_eq!(
        crate::MergeOutcome::parse("confluent"),
        None,
        "case matters"
    );
    assert_eq!(crate::MergeOutcome::parse("Marvellous"), None);
}

// =============================================================================================
// RFC 047 Handoff B — `merge`: the one write, at seal's tier
//
// The measured lines are scenario 1's, from `.git-exclude/reports/047-merge/measure047-0.53.txt`.
// Nothing here parses them; what is checked is the argument vector, the gate that runs before
// prikk does, and the fact that every class of exit arrives as the class it is.
// =============================================================================================

/// prikk 0.53.0's own output for scenario 1's merge, exactly as it printed it.
const MERGED: &str = "merged heads/main into heads/feature\n\
     baseline block: 88f34150e09929a618d75865609d7ba5d88a6f24937dbbb6f53d2214a1312d52\n\
     parent block: d967f47871e7ed45546c193f1166bdcd419f7841c5df7427f57f7082d7d5923c\n\
     adopted target block: 6d17f5f225b7bf7eded37aea8dab8680b0bb4080b8147c5e527b866b8afb7b93\n\
     adopted patches: 1\n\
     \x20 0dd35e76fe90c46782945e3ab34afc917e8210264176b1130c2a11eaac4fdb13\n\
     block id: 01163541294c3c5258cb92ea2e21f106c628fd9acf250a97c04831bb85678ad4\n\
     heads/feature RefState: 1e8e6fabc365a6861c1df9d6c20bdc9a6b4f2976820ab72712bdd3e0c672bd2c\n";

/// A fake `prikk` at `version` answering `key status` with a maintainer that is `usable` or not,
/// logging every call's argv, and answering anything else with `out`/`err` at `code`.
#[cfg(unix)]
fn fake_merge_prikk(
    tag: &str,
    version: &str,
    maintainer_usable: bool,
    out: &str,
    err: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-merge-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let payload = dir.join("payload");
    std::fs::write(&payload, out).expect("write the payload");
    let script = dir.join("fake-prikk.sh");
    let usable = if maintainer_usable { "true" } else { "false" };
    let binding = if maintainer_usable {
        "matches"
    } else {
        "absent"
    };
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             : >> \"{argv}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk {version}'; exit 0; fi\n\
             if [ \"$2\" = \"status\" ]; then\n\
             printf '%s\\n' '{{\"schema_version\":\"key-status-v1\",\"roles\":[{{\"role\":\"author\",\"usable\":true,\"key_id\":\"a\",\"binding\":\"matches\"}},{{\"role\":\"maintainer\",\"usable\":{usable},\"key_id\":\"m\",\"binding\":\"{binding}\"}}]}}'\n\
             exit 0\n\
             fi\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             cat \"{payload}\"\n\
             printf '%s' '{err}' 1>&2\n\
             exit {code}\n",
            argv = argv.display(),
            payload = payload.display(),
            version = version,
            usable = usable,
            binding = binding,
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

/// Run `merge` against a fake, and hand back both the outcome and the argv it recorded.
#[cfg(unix)]
fn merged(script: &std::path::Path) -> Result<crate::MergeResult> {
    let backend = CliBackend::with_program(script);
    retrying_transient_exec_busy(|| {
        backend.merge(
            std::path::Path::new("."),
            "88f34150e099",
            "heads/feature",
            "heads/main",
        )
    })
}

/// **The argument vector**: `--allow-no-audit` exactly as seal passes it, and one argument each.
#[cfg(unix)]
#[test]
fn merge_sends_allow_no_audit_and_one_argument_each() {
    let (script, argv) = fake_merge_prikk("args", "0.53.0", true, MERGED, "", 0);
    merged(&script).expect("a merge runs");
    let recorded = recorded_argv(&argv);
    assert_eq!(
        recorded,
        vec![
            "merge",
            "--allow-no-audit",
            "--baseline-block",
            "88f34150e099",
            "--into",
            "heads/feature",
            "--from",
            "heads/main"
        ]
    );
    // **No `--format json`**, as for the plan: there is none, and asking is a usage error.
    assert!(!recorded.iter().any(|a| a == "--format"), "{recorded:?}");
}

/// **prikk's success lines are kept whole and read by nothing.**
#[cfg(unix)]
#[test]
fn a_merges_lines_are_kept_verbatim() {
    let (script, _argv) = fake_merge_prikk("done", "0.53.0", true, MERGED, "", 0);
    let crate::MergeResult::Done(lines) = merged(&script).expect("a merge runs") else {
        panic!("expected a merge that ran");
    };
    assert_eq!(lines.len(), 8, "{lines:?}");
    assert_eq!(lines[0], "merged heads/main into heads/feature");
    assert_eq!(
        lines[4], "adopted patches: 1",
        "the count is shown, not summed from the id below it"
    );
    // The patch id keeps prikk's own indent: these are prikk's lines, not stikk's layout.
    assert_eq!(
        lines[5],
        "  0dd35e76fe90c46782945e3ab34afc917e8210264176b1130c2a11eaac4fdb13"
    );
    assert!(
        lines[7].starts_with("heads/feature RefState: "),
        "{lines:?}"
    );
}

/// **A pair prikk will not merge refuses, verbatim** (`ER-02`) — and writes nothing (F1).
#[cfg(unix)]
#[test]
fn a_non_confluent_refusal_arrives_in_prikks_own_words() {
    let said = "error: precondition not met: merge refused: heads/main is not confluent with \
                heads/feature from baseline 2237a4316d32 (outcome: Conflict, reason: pair_conflict)";
    let (script, _argv) = fake_merge_prikk("conflict", "0.53.0", true, "", said, 1);
    let error = merged(&script).expect_err("a non-confluent pair is refused");
    assert!(
        matches!(&error, stikk_model::StikkError::Refusal { message } if message.contains("is not confluent with")
            && message.contains("outcome: Conflict")),
        "{error:?}"
    );
}

/// **prikk's key refusal is prikk's**, when stikk's own gate let the call through.
///
/// It arrives as `NotReady`, not `Refusal` — the classifier reads *invalid signature* as a
/// readiness problem, which is what it is and what `seal` already produces — and it carries
/// prikk's own sentence unaltered (`ER-02`). What matters here is that stikk adds no sentence of
/// its own on top: the session looked ready, prikk disagreed, and prikk says why.
#[cfg(unix)]
#[test]
fn prikks_signature_refusal_arrives_verbatim() {
    let said = "error: invalid signature: maintainer signer public key does not match trusted key \
                maintainer";
    let (script, _argv) = fake_merge_prikk("nokey-prikk", "0.53.0", true, "", said, 1);
    let error = merged(&script).expect_err("prikk refuses the signature");
    let stikk_model::StikkError::NotReady { detail } = &error else {
        panic!("expected a readiness error, got {error:?}");
    };
    assert!(detail.contains("maintainer signer public key"), "{detail}");
    // **Not stikk's own words**: those are the ones the gate above gives, and this did not come
    // from the gate.
    assert!(!detail.contains("merging needs"), "{detail}");
}

/// **stikk's own stop comes first**, in stikk's words, and `merge` is never spawned for it.
#[cfg(unix)]
#[test]
fn without_a_maintainer_key_stikk_stops_before_prikk_runs() {
    let (script, argv) = fake_merge_prikk("nokey", "0.53.0", false, MERGED, "", 0);
    let error = merged(&script).expect_err("a merge needs MAINTAINER readiness");
    assert!(
        matches!(&error, stikk_model::StikkError::NotReady { detail } if detail.contains("merging")
            && detail.contains("MAINTAINER")),
        "{error:?}"
    );
    assert!(
        !recorded_argv(&argv).iter().any(|a| a == "merge"),
        "nothing was spawned to merge: {:?}",
        recorded_argv(&argv)
    );
}

/// **A usage exit is stikk's own bug class, never prikk's refusal voice** — the shape a missing
/// `--allow-no-audit` would produce (exit 2, measured). It must never happen; if it does, the
/// fault screen says the repository was not touched, which is true.
#[cfg(unix)]
#[test]
fn a_usage_exit_from_merge_is_an_internal_fault_not_a_refusal() {
    let said = "error: the following required arguments were not provided: --allow-no-audit";
    let (script, _argv) = fake_merge_prikk("usage", "0.53.0", true, "", said, 2);
    let error = merged(&script).expect_err("a usage error");
    assert!(
        matches!(&error, stikk_model::StikkError::Internal { detail } if detail.contains("usage error")),
        "{error:?}"
    );
}

/// **Below the band nothing is spawned**, not even the readiness read.
#[cfg(unix)]
#[test]
fn below_the_band_merge_spawns_nothing() {
    let (script, argv) = fake_merge_prikk("band", "0.41.0", true, MERGED, "", 0);
    assert_eq!(
        merged(&script).expect("the band is not an error"),
        crate::MergeResult::Unavailable
    );
    let recorded = recorded_argv(&argv);
    assert!(
        recorded.is_empty(),
        "only `--version` ran, which is not recorded: {recorded:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 050 — `bundle verify` and `bundle preview`.
//
// Every fixture below is prikk 0.53.0's own output, pasted from
// `.git-exclude/reports/050-exchange/measure050-0.53.txt` and `measure050c-0.53.txt` as those
// files hold it.

/// `bundle verify` on a bundle prikk accepts — including prikk's closing caveat, which is the
/// part a reader most needs and the part a summariser would drop.
const VERIFY_OUT: &str = "bundle verifies: heads/main\n\
     RefState: d3172482cabc746238f75f9842b30f0d0dff24ec580f4206c532e76c3991a29c\n\
     tip block: 33b2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9\n\
     objects: 6\n\
     author key material: 1 present (continuity only, not a trust decision)\n\
     repository format: 7\n\
     tool version: 0.53.0\n\
     note: this bundle contains one ref's closure only -- other refs in the source repository, if \
     any, are not included, and this bundle makes no claim about them\n\
     note: this checks structural and internal consistency only -- no signature is \
     cryptographically verified (a standalone bundle carries no trust material to check one \
     against), and this bundle's own author-key section is recorded here, never independently \
     verified, the same as at import. A verified bundle is not yet a trusted one -- import it and \
     run `prikk verify` for that.\n";

/// `bundle preview --format json` where the receiver is at the bundle's parent block.
const PREVIEW_FAST_FORWARD: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "fast-forward",
  "conflict": "applies-cleanly",
  "conflict_detail": null,
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [
    {"path": "a.txt", "kind": "edited", "current_bytes": 2, "after_bytes": 3}
  ],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// The same pair the other way round: the receiver already holds everything. **`conflict` is
/// `null` here**, which is why the field is read as nullable.
const PREVIEW_ALREADY_INCLUDED: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "already-included",
  "conflict": null,
  "conflict_detail": null,
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// Both sides moved on, each editing a different file.
const PREVIEW_DIVERGED_CLEAN: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "diverged",
  "conflict": "applies-cleanly",
  "conflict_detail": null,
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [
    {"path": "a.txt", "kind": "edited", "current_bytes": 2, "after_bytes": 3}
  ],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// Both sides edited the same file. **`conflict_detail` is prikk's replay error**, carried word
/// for word.
const PREVIEW_DIVERGED_CONFLICT: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "diverged",
  "conflict": "conflict",
  "conflict_detail": "integrity error: EditText failed during localization for node 3a19e411d69484d0fb161ade5a1314958cffed2879110e40a834a72fdbf9e1ce span e82f0ff279eebc030990061542c6b215c0417b442c38e2c3a6a4d669df987a1d: no occurrence's anchors matched the record",
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// A fresh repository. **`conflict` is `applies-cleanly` here, not `null`** — the one row where
/// an empty local side still gets an answer.
const PREVIEW_NO_LOCAL_HISTORY: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "no-local-history",
  "conflict": "applies-cleanly",
  "conflict_detail": null,
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [
    {"path": "a.txt", "kind": "created", "current_bytes": null, "after_bytes": 2},
    {"path": "b.txt", "kind": "created", "current_bytes": null, "after_bytes": 2}
  ],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// An independent repository holding the same file contents by coincidence.
const PREVIEW_DOES_NOT_CONNECT: &str = r#"{
  "schema_version": "bundle-preview-v1",
  "bundle_ref": "heads/main",
  "local_ref": "heads/main",
  "connectivity": "does-not-connect",
  "conflict": null,
  "conflict_detail": null,
  "sealed_by": ["z-maint"],
  "sealed_by_note": "recorded signer identity, continuity only -- not a trust decision",
  "effects": [],
  "rename_note": "a moved path is not yet distinguishable from a delete plus a create in this preview -- until rename authoring lands, do not infer that any delete+create pair shown here was a move",
  "manifest": {"repository_format": 7, "tool_version": "0.53.0"}
}
"#;

/// **The argument vector**, and no `--format json`: asking for one is a usage error (F1,
/// measured — `error: unknown bundle verify argument: --format`). **A path with a space stays one
/// argument**: it is passed as a single `OsString`, never through a shell.
#[cfg(unix)]
#[test]
fn bundle_verify_sends_one_input_argument_and_no_format_flag() {
    let (script, argv) = fake_plan_prikk("bverify-args", "0.53.0", VERIFY_OUT, "", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.bundle_verify(std::path::Path::new("/tmp/my bundles/v1.bundle"))
    })
    .expect("a bundle verifies");
    assert_eq!(
        recorded_argv(&argv),
        vec!["bundle", "verify", "--input", "/tmp/my bundles/v1.bundle"]
    );
}

/// prikk's lines are kept whole, in order, and **nothing of them is parsed** in this operation.
#[cfg(unix)]
#[test]
fn a_verifications_lines_are_kept_verbatim_including_the_caveat() {
    let (script, _argv) = fake_plan_prikk("bverify-lines", "0.53.0", VERIFY_OUT, "", 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.bundle_verify(std::path::Path::new("/tmp/v1.bundle"))
    })
    .expect("a bundle verifies");
    let crate::BundleVerify::Verified(lines) = out else {
        panic!("a verified bundle reads: {out:?}");
    };
    assert_eq!(lines.len(), 9, "{lines:#?}");
    assert_eq!(lines[0], "bundle verifies: heads/main");
    assert!(
        lines[8].contains("A verified bundle is not yet a trusted one"),
        "prikk's caveat is part of what the reader must see: {}",
        lines[8]
    );
}

/// A file prikk will not read is **prikk's refusal, verbatim** (`ER-02`), not stikk's paraphrase.
#[cfg(unix)]
#[test]
fn a_malformed_bundle_is_prikks_own_refusal() {
    for (tag, message) in [
        (
            "trunc",
            "error: malformed persisted data: unexpected end of record",
        ),
        (
            "junk",
            "error: malformed persisted data: invalid bundle magic",
        ),
        (
            "flip",
            "error: malformed persisted data: unknown CreateFile field tag: 253",
        ),
    ] {
        let (script, _argv) = fake_plan_prikk(
            &format!("bverify-{tag}"),
            "0.53.0",
            "",
            &format!("{message}\n"),
            1,
        );
        let backend = CliBackend::with_program(&script);
        let error = retrying_transient_exec_busy(|| {
            backend.bundle_verify(std::path::Path::new("/tmp/x.bundle"))
        })
        .expect_err("a malformed bundle is refused");
        assert!(
            error.to_string().contains("malformed persisted data"),
            "{tag}: prikk's words, kept: {error}"
        );
    }
}

/// **Below the band nothing is spawned**, for either call, and *unavailable* is not *failed*.
#[cfg(unix)]
#[test]
fn below_the_band_neither_bundle_call_spawns_anything() {
    let (script, argv) = fake_plan_prikk("bundle-band", "0.52.0", VERIFY_OUT, "", 0);
    let backend = CliBackend::with_program(&script);
    let verified = retrying_transient_exec_busy(|| {
        backend.bundle_verify(std::path::Path::new("/tmp/v1.bundle"))
    })
    .expect("below the band is not an error");
    assert_eq!(verified, crate::BundleVerify::Unavailable);
    let previewed = retrying_transient_exec_busy(|| {
        backend.bundle_preview(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/v1.bundle"),
            None,
        )
    })
    .expect("below the band is not an error");
    assert_eq!(previewed, crate::BundlePreview::Unavailable);
    assert!(
        recorded_argv(&argv).is_empty(),
        "no bundle command was run: {:?}",
        recorded_argv(&argv)
    );
}

/// **`--ref` is omitted when the caller has no opinion**, so prikk applies its own default (the
/// local ref of the bundle's own name) — and passed as one argument when the caller has one.
#[cfg(unix)]
#[test]
fn bundle_preview_omits_the_ref_flag_unless_a_local_ref_is_named() {
    let (script, argv) = fake_plan_prikk("bpreview-args", "0.53.0", PREVIEW_FAST_FORWARD, "", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.bundle_preview(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/my bundles/v2.bundle"),
            None,
        )
    })
    .expect("a preview reads");
    assert_eq!(
        recorded_argv(&argv),
        vec![
            "bundle",
            "preview",
            "--input",
            "/tmp/my bundles/v2.bundle",
            "--format",
            "json"
        ]
    );

    let (script, argv) = fake_plan_prikk("bpreview-ref", "0.53.0", PREVIEW_FAST_FORWARD, "", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.bundle_preview(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/v2.bundle"),
            Some("heads/work"),
        )
    })
    .expect("a preview reads");
    assert_eq!(
        recorded_argv(&argv),
        vec![
            "bundle",
            "preview",
            "--input",
            "/tmp/v2.bundle",
            "--ref",
            "heads/work",
            "--format",
            "json"
        ]
    );
}

/// **All five connectivities**, from prikk's own documents, parsed into stikk's values — and the
/// two shapes `conflict` takes: a word, and `null` where there is nothing to apply.
#[cfg(unix)]
#[test]
fn every_connectivity_prikk_reports_parses() {
    use crate::{BundleConflict as K, Connectivity as C};
    for (tag, text, want_connectivity, want_conflict) in [
        (
            "ff",
            PREVIEW_FAST_FORWARD,
            C::FastForward,
            Some(K::AppliesCleanly),
        ),
        (
            "included",
            PREVIEW_ALREADY_INCLUDED,
            C::AlreadyIncluded,
            None,
        ),
        (
            "diverged-clean",
            PREVIEW_DIVERGED_CLEAN,
            C::Diverged,
            Some(K::AppliesCleanly),
        ),
        (
            "diverged-conflict",
            PREVIEW_DIVERGED_CONFLICT,
            C::Diverged,
            Some(K::Conflict),
        ),
        (
            "fresh",
            PREVIEW_NO_LOCAL_HISTORY,
            C::NoLocalHistory,
            Some(K::AppliesCleanly),
        ),
        (
            "unrelated",
            PREVIEW_DOES_NOT_CONNECT,
            C::DoesNotConnect,
            None,
        ),
    ] {
        let (script, _argv) = fake_plan_prikk(&format!("bp-{tag}"), "0.53.0", text, "", 0);
        let backend = CliBackend::with_program(&script);
        let out = retrying_transient_exec_busy(|| {
            backend.bundle_preview(
                std::path::Path::new("."),
                std::path::Path::new("/tmp/x.bundle"),
                None,
            )
        })
        .unwrap_or_else(|error| panic!("{tag}: {error}"));
        let crate::BundlePreview::Read(read) = out else {
            panic!("{tag}: a preview reads");
        };
        assert_eq!(read.connectivity, want_connectivity, "{tag}");
        assert_eq!(read.conflict, want_conflict, "{tag}");
        assert_eq!(read.bundle_ref, "heads/main", "{tag}");
        assert_eq!(read.local_ref, "heads/main", "{tag}");
        assert_eq!(read.sealed_by, vec!["z-maint".to_string()], "{tag}");
        assert!(
            read.sealed_by_note.contains("not a trust decision"),
            "{tag}: prikk's continuity note travels with the ids: {}",
            read.sealed_by_note
        );
        assert!(
            read.rename_note.contains("delete plus a create"),
            "{tag}: {}",
            read.rename_note
        );
    }
}

/// The conflicting row's detail is **prikk's replay error, word for word**, and the clean rows
/// carry none. The effects come through with both sizes, `null` included.
#[cfg(unix)]
#[test]
fn a_conflicts_detail_and_the_effects_come_through_as_prikk_wrote_them() {
    let (script, _argv) = fake_plan_prikk("bp-detail", "0.53.0", PREVIEW_DIVERGED_CONFLICT, "", 0);
    let backend = CliBackend::with_program(&script);
    let crate::BundlePreview::Read(read) = retrying_transient_exec_busy(|| {
        backend.bundle_preview(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/x.bundle"),
            None,
        )
    })
    .expect("a preview reads") else {
        panic!("a preview reads");
    };
    assert_eq!(
        read.conflict_detail.as_deref(),
        Some(
            "integrity error: EditText failed during localization for node \
             3a19e411d69484d0fb161ade5a1314958cffed2879110e40a834a72fdbf9e1ce span \
             e82f0ff279eebc030990061542c6b215c0417b442c38e2c3a6a4d669df987a1d: no occurrence's \
             anchors matched the record"
        )
    );
    assert!(read.effects.is_empty(), "{:?}", read.effects);

    let (script, _argv) = fake_plan_prikk("bp-effects", "0.53.0", PREVIEW_NO_LOCAL_HISTORY, "", 0);
    let backend = CliBackend::with_program(&script);
    let crate::BundlePreview::Read(read) = retrying_transient_exec_busy(|| {
        backend.bundle_preview(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/x.bundle"),
            None,
        )
    })
    .expect("a preview reads") else {
        panic!("a preview reads");
    };
    assert_eq!(read.conflict_detail, None);
    assert_eq!(
        read.effects,
        vec![
            crate::BundleEffect {
                path: "a.txt".to_string(),
                kind: crate::EffectKind::Created,
                current_bytes: None,
                after_bytes: Some(2),
            },
            crate::BundleEffect {
                path: "b.txt".to_string(),
                kind: crate::EffectKind::Created,
                current_bytes: None,
                after_bytes: Some(2),
            },
        ]
    );
}

/// **A value outside one of prikk's three vocabularies refuses** (`UD-02`), and so does another
/// schema. stikk does not guess at a word it does not know: the sentence it would draw about the
/// bundle would be about the wrong thing.
#[cfg(unix)]
#[test]
fn an_unknown_value_refuses_rather_than_guesses() {
    for (tag, from, to, wanted) in [
        (
            "schema",
            "\"bundle-preview-v1\"",
            "\"bundle-preview-v2\"",
            "does not guess at an unknown schema version",
        ),
        (
            "connectivity",
            "\"fast-forward\"",
            "\"rebase-forward\"",
            "not one of prikk's five",
        ),
        (
            "conflict",
            "\"applies-cleanly\"",
            "\"probably-fine\"",
            "not one of prikk's three",
        ),
        (
            "kind",
            "\"edited\"",
            "\"renamed\"",
            "not one of prikk's four kinds",
        ),
    ] {
        let text = PREVIEW_FAST_FORWARD.replacen(from, to, 1);
        assert_ne!(text, PREVIEW_FAST_FORWARD, "{tag}: the fixture changed");
        let (script, _argv) = fake_plan_prikk(&format!("bp-bad-{tag}"), "0.53.0", &text, "", 0);
        let backend = CliBackend::with_program(&script);
        let error = retrying_transient_exec_busy(|| {
            backend.bundle_preview(
                std::path::Path::new("."),
                std::path::Path::new("/tmp/x.bundle"),
                None,
            )
        })
        .map_or_else(
            |error| error.to_string(),
            |read| panic!("{tag}: an unknown value must refuse, got {read:?}"),
        );
        assert!(
            error.contains(wanted),
            "{tag}: the refusal must name what stikk could not read: {error}"
        );
    }
}

/// prikk's five connectivity words, three conflict words and four effect kinds round-trip, and
/// nothing outside them parses.
#[test]
fn prikks_bundle_vocabularies_round_trip_and_are_closed() {
    for word in [
        "no-local-history",
        "does-not-connect",
        "already-included",
        "fast-forward",
        "diverged",
    ] {
        let value = crate::Connectivity::parse(word).unwrap_or_else(|| panic!("{word}"));
        assert_eq!(value.name(), word);
    }
    assert_eq!(crate::Connectivity::parse("Diverged"), None);
    assert_eq!(crate::Connectivity::parse(""), None);

    for word in ["applies-cleanly", "conflict", "undetermined"] {
        let value = crate::BundleConflict::parse(word).unwrap_or_else(|| panic!("{word}"));
        assert_eq!(value.name(), word);
    }
    assert_eq!(crate::BundleConflict::parse("null"), None);

    for word in ["created", "deleted", "edited", "permission-changed"] {
        let value = crate::EffectKind::parse(word).unwrap_or_else(|| panic!("{word}"));
        assert_eq!(value.name(), word);
    }
    assert_eq!(crate::EffectKind::parse("renamed"), None);
}

// ---------------------------------------------------------------------------------------------
// RFC 050 Handoff B — `bundle import`, `bundle export`, and the one pinned line.

/// `bundle import`'s report at 0.53, from `measure050b-0.53.txt` as that file holds it.
const IMPORT_OUT: &str = "received remotes/heads/main\n\
     RefState: 89ab4b25e0c3826a679db6d1754d01351665cfb8b3c8f7b2c7a2b4d880b96b0b\n\
     objects: 9\n\
     new objects: 3\n\
     author key material: 1 recorded (continuity only, not a trust decision)\n\
     note: no local ref was created or advanced, and no MAINTAINER key was trusted; this \
     repository now holds the received history at remotes/heads/main, which `prikk log --ref \
     remotes/heads/main` and `prikk show` read, and `prikk verify` checks once `trust maintainer \
     add` trusts the sealing key\n";

/// **The argument vector, and `--force` is not in it** (decision 5, §7 trap 1). prikk refuses an
/// existing destination without it, and that refusal is the behaviour stikk wants.
#[cfg(unix)]
#[test]
fn bundle_export_never_passes_force_and_keeps_a_spaced_path_whole() {
    let (script, argv) = fake_plan_prikk("bexport-args", "0.53.0", "wrote the bundle\n", "", 0);
    let backend = CliBackend::with_program(&script);
    retrying_transient_exec_busy(|| {
        backend.bundle_export(
            std::path::Path::new("."),
            "heads/main",
            std::path::Path::new("/tmp/my bundles/out.bundle"),
        )
    })
    .expect("an export runs");
    let argv = recorded_argv(&argv);
    assert_eq!(
        argv,
        vec![
            "bundle",
            "export",
            "--ref",
            "heads/main",
            "--output",
            "/tmp/my bundles/out.bundle"
        ]
    );
    assert!(
        !argv.iter().any(|a| a == "--force"),
        "stikk never overwrites: {argv:?}"
    );
}

/// `bundle import`'s argument vector, and its lines kept whole — prikk's *"no local ref was
/// created or advanced"* note among them.
#[cfg(unix)]
#[test]
fn bundle_import_sends_one_input_argument_and_keeps_prikks_note() {
    let (script, argv) = fake_plan_prikk("bimport-args", "0.53.0", IMPORT_OUT, "", 0);
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.bundle_import(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/my bundles/v1.bundle"),
        )
    })
    .expect("an import runs");
    assert_eq!(
        recorded_argv(&argv),
        vec!["bundle", "import", "--input", "/tmp/my bundles/v1.bundle"]
    );
    let crate::BundleImport::Done(lines) = out else {
        panic!("an import that ran: {out:?}");
    };
    assert_eq!(lines.len(), 6, "{lines:#?}");
    assert_eq!(lines[0], "received remotes/heads/main");
    assert!(
        lines[5].contains("no local ref was created or advanced"),
        "{}",
        lines[5]
    );
}

/// **Below the band neither write spawns anything**, and *unavailable* is not *failed*.
#[cfg(unix)]
#[test]
fn below_the_band_neither_bundle_write_spawns_anything() {
    let (script, argv) = fake_plan_prikk("bwrite-band", "0.52.0", IMPORT_OUT, "", 0);
    let backend = CliBackend::with_program(&script);
    let imported = retrying_transient_exec_busy(|| {
        backend.bundle_import(
            std::path::Path::new("."),
            std::path::Path::new("/tmp/v1.bundle"),
        )
    })
    .expect("below the band is not an error");
    assert_eq!(imported, crate::BundleImport::Unavailable);
    let exported = retrying_transient_exec_busy(|| {
        backend.bundle_export(
            std::path::Path::new("."),
            "heads/main",
            std::path::Path::new("/tmp/out.bundle"),
        )
    })
    .expect("below the band is not an error");
    assert_eq!(exported, crate::BundleExport::Unavailable);
    assert!(
        recorded_argv(&argv).is_empty(),
        "no bundle command was run: {:?}",
        recorded_argv(&argv)
    );
}

/// **The one pinned line**, found in prikk's real verification — and nothing else of that prose
/// read.
#[test]
fn the_tip_line_is_read_from_prikks_verification() {
    let printed: Vec<String> = VERIFY_OUT.lines().map(ToString::to_string).collect();
    assert_eq!(
        crate::bundle_tip(&printed).expect("the line is there"),
        "33b2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9"
    );
}

/// **Missing, repeated or malformed is a shape error** (`UD-02`), never a guess: this value
/// decides whether an import is offered, and a wrong one would offer an import that moves a
/// received branch backwards.
#[test]
fn a_missing_repeated_or_malformed_tip_line_refuses() {
    let good = "tip block: 33b2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9";

    let absent: Vec<String> = VERIFY_OUT
        .lines()
        .filter(|line| !line.trim().starts_with("tip block:"))
        .map(ToString::to_string)
        .collect();
    let error = crate::bundle_tip(&absent).expect_err("no line, no tip");
    assert!(
        error.to_string().contains("no `tip block:` line"),
        "{error}"
    );

    let twice: Vec<String> = VERIFY_OUT
        .lines()
        .flat_map(|line| {
            if line.trim().starts_with("tip block:") {
                vec![line.to_string(), line.to_string()]
            } else {
                vec![line.to_string()]
            }
        })
        .collect();
    let error = crate::bundle_tip(&twice).expect_err("two lines, no tip");
    assert!(
        error.to_string().contains("more than one"),
        "stikk does not choose which tip to believe: {error}"
    );

    // Short, upper-case and not-hex each refuse: a value that is not a block id compares
    // unequal against every id in the lineage, which would turn the older-bundle guard into no
    // guard at all rather than into a visible failure.
    for bad in [
        "tip block: 33b2b1bb",
        "tip block: 33B2B1BB2183DEADE7783DD25BDD37B101206E12280B16096731849BE62901A9",
        "tip block: zzb2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9",
        "tip block:",
    ] {
        let printed = vec![bad.to_string()];
        let error = crate::bundle_tip(&printed).map_or_else(
            |error| error.to_string(),
            |tip| panic!("`{bad}` is not a block id, but parsed as `{tip}`"),
        );
        assert!(
            error.contains("64 lowercase hex"),
            "`{bad}`: the refusal must say what stikk could not read: {error}"
        );
    }
    // The good one still parses, so the loop above is not asserting against a function that
    // refuses everything.
    assert!(crate::bundle_tip(&[good.to_string()]).is_ok());
}

// ---------------------------------------------------------------- RFC 053: the files seam
//
// **The argument vector is the thing to pin**, for the reason the ref writes pinned theirs: three
// of RFC 053's traps live in the argv — the destination must arrive as one absolute argument, the
// point must be the block id stikk was given, and **none of `--force`, `--allow-full-replay`,
// `--max-files` or `--max-bytes` may ever be built**. A fake prikk that records its own argv is
// how those become test failures rather than review findings.

/// A fake `prikk` answering `--version` with `version`, logging every call's argv one per line to
/// `argv`, and answering anything else with `lines` and exit `code`.
#[cfg(unix)]
fn fake_files_prikk(
    tag: &str,
    version: &str,
    lines: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-files-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk {version}'; exit 0; fi\n\
             : > \"{argv}\"\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             printf '%s' '{lines}'\n\
             exit {code}\n",
            argv = argv.display(),
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

/// A minimal `extract-report-v1`, as one line, for the fake to print.
#[cfg(unix)]
const FAKE_EXTRACT_REPORT: &str = concat!(
    r#"{"schema_version": "extract-report-v1", "target_block_id": "#,
    r#""f687c8ff7394b57414d88b191700eb67b4a15b5598cb11fb1bd848f0c1746243", "#,
    r#""provisional": false, "written": false, "file_count": 4, "total_bytes": 30, "#,
    r#""largest_file_bytes": 18}"#,
);

#[cfg(unix)]
const FAKE_BLOCK: &str = "f687c8ff7394b57414d88b191700eb67b4a15b5598cb11fb1bd848f0c1746243";

/// **The point is the block id, the path is one argument, and `--output` is absolute** — and
/// `--force` is nowhere (RFC 053 §7 traps 1–3).
#[cfg(unix)]
#[test]
fn save_file_passes_the_block_an_absolute_output_and_never_force() {
    let (script, argv) = fake_files_prikk("save", "0.55.0", "", 0);
    let backend = CliBackend::with_program(&script);
    let out = std::env::temp_dir().join("stikk-files-save-dest/a file.txt");
    let saved = retrying_transient_exec_busy(|| {
        backend.save_file(
            std::path::Path::new("."),
            FAKE_BLOCK,
            "dir/a file.txt",
            &out,
        )
    })
    .expect("a save reads");
    assert_eq!(saved, crate::FileSaved::Done);
    let recorded = recorded_argv(&argv);
    assert_eq!(
        recorded,
        vec![
            "cat".to_string(),
            "--path".to_string(),
            "dir/a file.txt".to_string(),
            "--ref".to_string(),
            FAKE_BLOCK.to_string(),
            "--output".to_string(),
            out.display().to_string(),
        ],
        "the path and the destination must each stay one argument"
    );
    assert!(
        !recorded.iter().any(|a| a == "--force"),
        "`--force` must never be built: prikk's refusal of an existing file is the behaviour \
         stikk wants"
    );
}

/// **None of the four flags, ever** — on the plan or on the write (RFC 053 decision 1).
#[cfg(unix)]
#[test]
fn extract_passes_the_block_an_absolute_dir_and_none_of_the_four_flags() {
    let dir = std::env::temp_dir().join("stikk-files-extract-dest");
    for (tag, plan_only) in [("plan", true), ("write", false)] {
        let (script, argv) = fake_files_prikk(tag, "0.55.0", FAKE_EXTRACT_REPORT, 0);
        let backend = CliBackend::with_program(&script);
        let read = retrying_transient_exec_busy(|| {
            if plan_only {
                backend.extract_plan(std::path::Path::new("."), FAKE_BLOCK, &dir)
            } else {
                backend.extract(std::path::Path::new("."), FAKE_BLOCK, &dir)
            }
        })
        .unwrap_or_else(|e| panic!("{tag}: {e}"));
        let crate::Extracted::Read(report) = read else {
            panic!("{tag}: in band, prikk answered");
        };
        assert_eq!(report.file_count, 4, "{tag}");

        let recorded = recorded_argv(&argv);
        let mut wanted = vec![
            "extract".to_string(),
            "--ref".to_string(),
            FAKE_BLOCK.to_string(),
            dir.display().to_string(),
        ];
        if plan_only {
            wanted.push("--plan-only".to_string());
        }
        wanted.push("--format".to_string());
        wanted.push("json".to_string());
        assert_eq!(recorded, wanted, "{tag}");
        for forbidden in [
            "--force",
            "--allow-full-replay",
            "--max-files",
            "--max-bytes",
        ] {
            assert!(
                !recorded.iter().any(|a| a == forbidden),
                "{tag}: `{forbidden}` must never be built — it removes a refusal stikk wants kept"
            );
        }
    }
}

/// `cat --format json` is a **read**: no `--output`, and the four fields come back.
#[cfg(unix)]
#[test]
fn file_meta_reads_without_writing_anything() {
    let report = concat!(
        r#"{"schema_version": "path-content-v1", "kind": "file", "#,
        r#""encoding": "text", "mode": 33261, "size": 18}"#,
    );
    let (script, argv) = fake_files_prikk("meta", "0.55.0", report, 0);
    let backend = CliBackend::with_program(&script);
    let read = retrying_transient_exec_busy(|| {
        backend.file_meta(std::path::Path::new("."), FAKE_BLOCK, "run.sh")
    })
    .expect("a read reads");
    let crate::FileMetaRead::Read(meta) = read else {
        panic!("in band, prikk answered");
    };
    assert_eq!(meta.mode, 33261);
    assert_eq!(meta.size, 18);
    let recorded = recorded_argv(&argv);
    assert_eq!(
        recorded,
        vec![
            "cat", "--path", "run.sh", "--ref", FAKE_BLOCK, "--format", "json"
        ]
    );
    assert!(!recorded.iter().any(|a| a == "--output"));
}

/// **A `refusal-v1` on stdout at exit 1 becomes prikk's refusal, verbatim** — not a shape error,
/// and not the classifier's reading of the stderr prose beside it (RFC 053 decision 1).
#[cfg(unix)]
#[test]
fn an_extract_refusal_document_becomes_prikks_refusal_verbatim() {
    let refusal = concat!(
        r#"{"schema_version": "refusal-v1", "command": "extract", "#,
        r#""refusal": "destination-exists", "reason": null, "message": "#,
        r#""precondition not met: refusing to extract into an existing path: /tmp/out1 "#,
        r#"(nothing was written)"}"#,
    );
    let (script, _argv) = fake_files_prikk("refusal", "0.55.0", refusal, 1);
    let backend = CliBackend::with_program(&script);
    let dir = std::env::temp_dir().join("stikk-files-refusal-dest");
    let err = retrying_transient_exec_busy(|| {
        backend.extract(std::path::Path::new("."), FAKE_BLOCK, &dir)
    })
    .expect_err("prikk exited 1 with a refusal document");
    assert_eq!(
        err.class(),
        "refusal",
        "prikk refused; stikk did not misread"
    );
    // **The message itself, not its rendering**: `Display` prefixes the class word, and what must
    // be verbatim is what the variant carries into the view.
    let stikk_model::StikkError::Refusal { message } = err else {
        panic!("a refusal document must become a refusal");
    };
    assert_eq!(
        message,
        "precondition not met: refusing to extract into an existing path: /tmp/out1 (nothing was \
         written)",
        "prikk's message, unrewritten"
    );
}

/// **Below the band nothing is spawned.** The fake would record an argv if it ran; the file must
/// not exist at all.
#[cfg(unix)]
#[test]
fn below_the_band_the_four_methods_answer_unavailable_and_spawn_nothing() {
    let dir = std::env::temp_dir().join("stikk-files-band-dest");
    let out = dir.join("a.txt");
    let (script, argv) = fake_files_prikk("band", "0.53.0", FAKE_EXTRACT_REPORT, 0);
    let backend = CliBackend::with_program(&script);
    let repo = std::path::Path::new(".");

    assert_eq!(
        retrying_transient_exec_busy(|| backend.file_meta(repo, FAKE_BLOCK, "a.txt"))
            .expect("below the band is an answer, not an error"),
        crate::FileMetaRead::Unavailable
    );
    assert_eq!(
        retrying_transient_exec_busy(|| backend.save_file(repo, FAKE_BLOCK, "a.txt", &out))
            .expect("below the band is an answer"),
        crate::FileSaved::Unavailable
    );
    assert_eq!(
        retrying_transient_exec_busy(|| backend.extract_plan(repo, FAKE_BLOCK, &dir))
            .expect("below the band is an answer"),
        crate::Extracted::Unavailable
    );
    assert_eq!(
        retrying_transient_exec_busy(|| backend.extract(repo, FAKE_BLOCK, &dir))
            .expect("below the band is an answer"),
        crate::Extracted::Unavailable
    );
    assert!(
        !argv.exists(),
        "below the band prikk must be asked its version and nothing else"
    );
}

/// **A relative destination never reaches prikk** (RFC 053 §7 trap 1): `cat` and `extract` take no
/// repository argument, so prikk would resolve it *inside the repository* — the one place this
/// must never write.
#[cfg(unix)]
#[test]
fn a_relative_destination_is_refused_before_prikk_is_asked_to_write() {
    let (script, argv) = fake_files_prikk("relative", "0.55.0", FAKE_EXTRACT_REPORT, 0);
    let backend = CliBackend::with_program(&script);
    let repo = std::path::Path::new(".");
    let relative = std::path::Path::new("out/a.txt");

    for (label, result) in [
        (
            "save_file",
            backend.save_file(repo, FAKE_BLOCK, "a.txt", relative).err(),
        ),
        (
            "extract_plan",
            backend.extract_plan(repo, FAKE_BLOCK, relative).err(),
        ),
        ("extract", backend.extract(repo, FAKE_BLOCK, relative).err()),
    ] {
        let err = result.unwrap_or_else(|| panic!("{label} must refuse a relative destination"));
        assert_eq!(err.class(), "environment", "{label}");
        assert!(err.to_string().contains("relative"), "{label}: {err}");
    }
    assert!(
        !argv.exists(),
        "the refusal must come before prikk is spawned to write"
    );
}

// ---------------------------------------------------------------- RFC 054: the trust read

/// **`list`, never `check`** (RFC 054 F2, §7 trap 2), and the argv is exactly five arguments.
#[cfg(unix)]
#[test]
fn trusted_keys_asks_list_and_never_check() {
    let report = concat!(
        r#"{"schema_version": "trust-list-v1", "keys": [{"key_id": "ed25519-31f369d3abb7f64c", "#,
        r#""public_key": "31f369d3abb7f64cd2cf70be31158181e361f25444d8de122cdff59389e7e946"}]}"#,
    );
    let (script, argv) = fake_files_prikk("trust", "0.55.0", report, 0);
    let backend = CliBackend::with_program(&script);
    let read = retrying_transient_exec_busy(|| backend.trusted_keys(std::path::Path::new(".")))
        .expect("a read reads");
    let crate::TrustedKeys::Listed(keys) = read else {
        panic!("in band, prikk answered");
    };
    assert_eq!(keys.len(), 1);
    let recorded = recorded_argv(&argv);
    assert_eq!(
        recorded,
        vec!["trust", "maintainer", "list", "--format", "json"]
    );
    assert!(
        !recorded.iter().any(|a| a == "check"),
        "`check` exits 1 for an id that is simply not trusted — one `list` answers every id"
    );
}

/// **An empty list is an answer**, not a refusal: a repository made with `init` alone.
#[cfg(unix)]
#[test]
fn an_empty_trust_list_reaches_the_caller_as_an_empty_list() {
    let (script, _argv) = fake_files_prikk(
        "trustempty",
        "0.55.0",
        r#"{"schema_version": "trust-list-v1", "keys": []}"#,
        0,
    );
    let backend = CliBackend::with_program(&script);
    let read = retrying_transient_exec_busy(|| backend.trusted_keys(std::path::Path::new(".")))
        .expect("a read reads");
    assert_eq!(read, crate::TrustedKeys::Listed(Vec::new()));
}

/// **Below the band nothing is spawned**: the fake would record an argv if it ran.
#[cfg(unix)]
#[test]
fn below_the_band_the_trust_list_is_unavailable_and_spawns_nothing() {
    let (script, argv) = fake_files_prikk(
        "trustband",
        "0.53.0",
        r#"{"schema_version": "trust-list-v1", "keys": []}"#,
        0,
    );
    let backend = CliBackend::with_program(&script);
    assert_eq!(
        retrying_transient_exec_busy(|| backend.trusted_keys(std::path::Path::new(".")))
            .expect("below the band is an answer, not an error"),
        crate::TrustedKeys::Unavailable
    );
    assert!(
        !argv.exists(),
        "below the band prikk must be asked its version and nothing else"
    );
}

// ---------------------------------------------------------------- RFC 056: the trust writes

/// A fake prikk that answers `--version` and `key status`, then records the argv of whatever
/// else it is asked and prints `lines` with exit `code`.
///
/// Separate from [`fake_ref_write_prikk`] only in the version it reports: the trust writes are
/// banded at `FILES_FROM_MINOR`, which 0.52 is below.
#[cfg(unix)]
fn fake_trust_write_prikk(
    tag: &str,
    version: &str,
    lines: &str,
    code: i32,
) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-trustwrite-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let argv = dir.join("argv");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'prikk {version}'; exit 0; fi\n\
             if [ \"$2\" = \"status\" ]; then\n\
             printf '%s\\n' '{{\"schema_version\":\"key-status-v1\",\"roles\":[{{\"role\":\"author\",\"usable\":true,\"key_id\":\"a\",\"binding\":\"matches\"}},{{\"role\":\"maintainer\",\"usable\":true,\"key_id\":\"m\",\"binding\":\"matches\"}}]}}'\n\
             exit 0\n\
             fi\n\
             : > \"{argv}\"\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{argv}\"; done\n\
             printf '%s' '{lines}'\n\
             exit {code}\n",
            argv = argv.display(),
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

/// A 64-hex public key, as prikk prints one.
#[cfg(unix)]
const FAKE_PUBLIC_KEY: &str = "31f369d3abb7f64cd2cf70be31158181e361f25444d8de122cdff59389e7e946";

/// **`add`'s argv, exactly** (RFC 056 decision 1) — and **never `--format`**, which prikk
/// refuses as an unknown argument, exit 2 (F1).
#[cfg(unix)]
#[test]
fn trust_add_passes_the_id_and_the_public_key_and_never_a_format_flag() {
    let (script, argv) = fake_trust_write_prikk(
        "add",
        "0.55.0",
        "trusted maintainer key: ed25519-31f369d3abb7f64c\nadopted maintainer keys: 2\n",
        0,
    );
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.trust_add(
            std::path::Path::new("."),
            "ed25519-31f369d3abb7f64c",
            FAKE_PUBLIC_KEY,
        )
    })
    .expect("an adopt");
    assert_eq!(
        out,
        crate::TrustWritten::Done(vec![
            "trusted maintainer key: ed25519-31f369d3abb7f64c".to_string(),
            "adopted maintainer keys: 2".to_string(),
        ])
    );
    let recorded = recorded_argv(&argv);
    assert_eq!(
        recorded,
        vec![
            "trust",
            "maintainer",
            "add",
            "--key-id",
            "ed25519-31f369d3abb7f64c",
            "--public-key",
            FAKE_PUBLIC_KEY,
        ]
    );
    assert!(!recorded.iter().any(|a| a == "--format"), "{recorded:?}");
}

/// **`remove`'s argv, exactly**, and the id is one argument.
#[cfg(unix)]
#[test]
fn trust_remove_passes_the_id_as_one_argument() {
    let (script, argv) = fake_trust_write_prikk(
        "remove",
        "0.55.0",
        "revoked maintainer key: ed25519-31f369d3abb7f64c\n",
        0,
    );
    let backend = CliBackend::with_program(&script);
    let out = retrying_transient_exec_busy(|| {
        backend.trust_remove(std::path::Path::new("."), "ed25519-31f369d3abb7f64c")
    })
    .expect("a removal");
    assert_eq!(
        out,
        crate::TrustWritten::Done(vec![
            "revoked maintainer key: ed25519-31f369d3abb7f64c".to_string()
        ])
    );
    assert_eq!(
        recorded_argv(&argv),
        vec![
            "trust",
            "maintainer",
            "remove",
            "--key-id",
            "ed25519-31f369d3abb7f64c",
        ]
    );
    assert!(
        !recorded_argv(&argv).iter().any(|a| a == "--format"),
        "neither write takes one"
    );
}

/// **prikk's two "nothing changed" lines are carried through as lines** (F1): both exit 0, and
/// the seam parses neither. stikk's guards answer these before prikk is asked; what reaches
/// here is still prikk's own sentence.
#[cfg(unix)]
#[test]
fn the_already_trusted_and_was_not_trusted_lines_are_carried_through() {
    for (tag, printed) in [
        (
            "already",
            "maintainer key already trusted: ed25519-31f369d3abb7f64c\nadopted maintainer keys: 2\n",
        ),
        (
            "notrusted",
            "maintainer key was not trusted: ed25519-0000000000000000\n",
        ),
    ] {
        let (script, _argv) = fake_trust_write_prikk(tag, "0.55.0", printed, 0);
        let backend = CliBackend::with_program(&script);
        let out = retrying_transient_exec_busy(|| {
            backend.trust_remove(std::path::Path::new("."), "ed25519-0000000000000000")
        })
        .expect("exit 0 is not a refusal");
        let crate::TrustWritten::Done(lines) = out else {
            panic!("{tag}: in band, prikk answered");
        };
        assert_eq!(
            lines,
            printed
                .trim_end()
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            "{tag}: prikk's lines, verbatim and in order"
        );
    }
}

/// **A refusal is prikk's, verbatim** (`ER-02`) — the TOFU case decision 5 shows under stikk's
/// own heading, with no prose matched on.
#[cfg(unix)]
#[test]
fn a_refused_adopt_carries_prikks_words_verbatim() {
    let said = "error: precondition not met: maintainer key id ed25519-31f369d3abb7f64c is \
                already adopted with a different public key. A key id travels in every \
                signature, so adopting this key under another local id would match none of its \
                blocks";
    let (script, _argv) = fake_trust_write_prikk("refused", "0.55.0", said, 1);
    let backend = CliBackend::with_program(&script);
    let err = retrying_transient_exec_busy(|| {
        backend.trust_add(
            std::path::Path::new("."),
            "ed25519-31f369d3abb7f64c",
            FAKE_PUBLIC_KEY,
        )
    })
    .expect_err("prikk refused");
    assert_eq!(err.class(), "refusal");
    assert!(
        err.to_string()
            .contains("already adopted with a different public key"),
        "{err}"
    );
}

/// **Below the band nothing is spawned**, for either write: the fake would record an argv.
#[cfg(unix)]
#[test]
fn below_the_band_both_trust_writes_are_unavailable_and_spawn_nothing() {
    let (script, argv) = fake_trust_write_prikk("band", "0.53.0", "unreachable\n", 0);
    let backend = CliBackend::with_program(&script);
    assert_eq!(
        retrying_transient_exec_busy(|| backend.trust_add(
            std::path::Path::new("."),
            "ed25519-31f369d3abb7f64c",
            FAKE_PUBLIC_KEY
        ))
        .expect("below the band is an answer, not an error"),
        crate::TrustWritten::Unavailable
    );
    assert_eq!(
        retrying_transient_exec_busy(
            || backend.trust_remove(std::path::Path::new("."), "ed25519-31f369d3abb7f64c")
        )
        .expect("below the band is an answer, not an error"),
        crate::TrustWritten::Unavailable
    );
    assert!(
        !argv.exists(),
        "below the band prikk must be asked its version and nothing else"
    );
}

//! **No shipped crate reads a signing seed's value** — the workspace-wide guard behind the README's
//! *"No key material ever enters stikk … This is enforced by test"* (RFC 023 Handoff C).
//!
//! That sentence was held by two tests, each over **one file**: `stikk-prikk/src/env/tests.rs`
//! proves `env.rs` materializes no value, and `stikk-prikk/src/key_id/tests.rs` proves `key_id.rs`
//! never names a seed at all. A seed read added anywhere else in the six published crates would
//! have passed every test in the workspace. This closes that, before stikk's writes grow further.
//!
//! **What it enforces.** Across every `.rs` file under `src/` of the six published crates, with
//! comments stripped, the substring `_SEED` appears only in [`ALLOWED`] — and every entry there
//! carries its reason. A new site is a **test failure naming the file**, not a silent pass.
//!
//! **Why `_SEED` and not `seed`.** The variables are `PRIKK_AUTHOR_SEED` and
//! `PRIKK_MAINTAINER_SEED`, and reading one means naming one: `std::env::var` takes the name.
//! The word *seed* on its own appears in prose and in identifiers that hold no value (`seeded`,
//! `AUTHOR_SEED_FILE` is prikk's, not stikk's), so keying on the variable suffix is what makes a
//! hit mean something. The two single-file tests above still prove the stronger property for the
//! one file that does touch the variables.
//!
//! **Why comments are stripped.** Four files explain the rule in prose — `key_id.rs`'s table,
//! `example_keys.rs`'s note on prikk 0.41, and two field docs in `stikk-prikk/src/lib.rs` and
//! `stikk-core/src/orient.rs`. Allowlisting them would mean a future *code* site in the same file
//! passed unnoticed, so the scan reads code only and those four files are not on the list at all.
//!
//! It lives here for `rfc_index`'s own reason: `stikk-real-binary` is `publish = false`, so a test
//! that reads the repository tree can never travel inside a published `.crate`. It is **not**
//! `#[ignore]`d and needs no prikk binary.

use std::path::{Path, PathBuf};

/// The six published crates. `stikk-real-binary` is deliberately absent: it is `publish = false`,
/// and it sets fake seeds on purpose to drive the real binary.
const PUBLISHED: &[&str] = &[
    "stikk",
    "stikk-model",
    "stikk-prikk",
    "stikk-state",
    "stikk-core",
    "stikk-tui",
];

/// Every file whose **code** may name a seed variable, with the reason it may.
///
/// **Presence, never a value**, in all three. Adding a row here is a decision a reviewer must
/// make deliberately, which is the point of the list being in the test rather than in a pattern.
const ALLOWED: &[(&str, &str)] = &[
    (
        "crates/stikk-prikk/src/env.rs",
        "reads presence only — `env/tests.rs` proves no value is materialized, and this is the \
         only file in the workspace that names the variables to `std::env`",
    ),
    (
        "crates/stikk/src/main.rs",
        "the `--help` text that states the rule: \"stikk reads PRIKK_*_SEED presence only, never \
         the value\"",
    ),
    (
        "crates/stikk-tui/src/view/orientation.rs",
        "Orientation's sentence that a PRIKK_*_SEED variable is set and this prikk ignores it \
         (RFC 026 §4) — the fact that one is set, reported",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Whether this path is test-only, and so free to name seeds.
///
/// **By path, as the handoff asks**, not by reading the file: a `#[cfg(test)]` module inside a
/// shipped file would be invisible to a path rule, so this project's convention is that every
/// test module lives in its own `tests.rs` or `tests/` directory — and
/// `the_scan_reaches_every_crate` below would notice if a crate's whole `src/` vanished behind
/// this rule.
fn is_test_path(relative: &str) -> bool {
    relative.ends_with("/tests.rs") || relative.contains("/tests/")
}

/// Rust source with `//` line comments and `/* */` block comments removed, and string literals
/// kept — a `_SEED` inside a string **is** code here, because that is exactly how the variable's
/// name is written.
///
/// Deliberately simple, and deliberately conservative in the one direction that matters: a `//`
/// inside a string literal makes this drop the rest of that line, which can only ever *hide* a
/// later `_SEED` on the same line. [`every_allowlisted_file_still_matches`] is the control — if
/// the stripper ever stopped seeing code, `env.rs` would stop matching and that test would fail.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// Every `.rs` file under the six crates' `src/`, as `(relative path, code with comments
/// stripped)`, test paths excluded.
fn scanned() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for crate_name in PUBLISHED {
        let src = root.join("crates").join(crate_name).join("src");
        assert!(src.is_dir(), "{} has no src/", src.display());
        walk(&root, &src, &mut out);
    }
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(root, &path, out);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("under the root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_test_path(&relative) {
            continue;
        }
        let source =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {relative}: {e}"));
        out.push((relative, strip_comments(&source)));
    }
}

/// **The guard.** A seed variable is named in code only where the allowlist says it may be.
#[test]
fn no_shipped_crate_names_a_seed_variable_outside_the_allowlist() {
    let allowed: Vec<&str> = ALLOWED.iter().map(|(path, _)| *path).collect();
    let files = scanned();
    let mut found: Vec<&str> = Vec::new();
    for (relative, code) in &files {
        if !code.contains("_SEED") {
            continue;
        }
        assert!(
            allowed.contains(&relative.as_str()),
            "{relative} names a seed variable in code, and is not on the allowlist.\n\n\
             stikk never reads a seed's value (`C-I1`; the README says so and says it is enforced \
             by test — this is that test). If this really is a presence-only read, add the file to \
             `ALLOWED` **with its reason**, in the review request, so a reviewer decides it. If it \
             reads a value, it must not ship."
        );
        found.push(relative);
    }
    // Every allowlist entry is live: an entry for a file that stopped naming a seed is a licence
    // nobody needs, and the next person to add a seed read there would inherit it silently.
    for (path, reason) in ALLOWED {
        assert!(
            found.contains(path),
            "{path} is allowlisted ({reason}) but no longer names a seed variable in code — \
             remove the entry"
        );
    }
}

/// **The scan is looking at something**, the first of the two sanity checks: every published crate
/// contributes at least one scanned file. A guard that silently scanned nothing would pass.
#[test]
fn the_scan_reaches_every_crate() {
    let files = scanned();
    assert!(
        files.len() >= 50,
        "only {} files scanned across six crates — the walk is not reaching them",
        files.len()
    );
    for crate_name in PUBLISHED {
        let prefix = format!("crates/{crate_name}/src/");
        let count = files
            .iter()
            .filter(|(relative, _)| relative.starts_with(&prefix))
            .count();
        assert!(count > 0, "{crate_name} contributed no scanned file");
    }
}

/// **The scan can still see a seed**, the second sanity check: `env.rs` is scanned, and its code
/// — after the comment strip — does contain `_SEED`. If the stripper or the walk ever stopped
/// working, the guard above would pass vacuously and this fails instead.
#[test]
fn every_allowlisted_file_still_matches() {
    let files = scanned();
    let env = files
        .iter()
        .find(|(relative, _)| relative == "crates/stikk-prikk/src/env.rs")
        .expect("env.rs is scanned");
    assert!(
        env.1.contains("PRIKK_AUTHOR_SEED") && env.1.contains("PRIKK_MAINTAINER_SEED"),
        "env.rs's code no longer names both seed variables — the comment strip has eaten code"
    );
}

/// The comment strip keeps code and drops prose, including the four files that only explain the
/// rule. **A doc comment naming a seed must not make a file look like a reader.**
#[test]
fn comments_are_stripped_and_code_is_not() {
    let stripped = strip_comments(
        "//! PRIKK_AUTHOR_SEED in a module doc\n\
         /// PRIKK_AUTHOR_SEED in an item doc\n\
         /* PRIKK_AUTHOR_SEED in a block */\n\
         const A: &str = \"PRIKK_AUTHOR_SEED\"; // and a trailing PRIKK_MAINTAINER_SEED\n",
    );
    assert_eq!(stripped.matches("_SEED").count(), 1, "{stripped}");
    assert!(
        stripped.contains("const A: &str = \"PRIKK_AUTHOR_SEED\";"),
        "{stripped}"
    );

    // And the four prose-only files are, in fact, prose-only: none of them is allowlisted, so if
    // one ever gained a code site the guard above would catch it by name.
    for prose_only in [
        "crates/stikk-prikk/src/key_id.rs",
        "crates/stikk-core/src/orient.rs",
        "crates/stikk-model/src/example_keys.rs",
        "crates/stikk-prikk/src/lib.rs",
    ] {
        let file = scanned()
            .into_iter()
            .find(|(relative, _)| relative == prose_only)
            .unwrap_or_else(|| panic!("{prose_only} is scanned"));
        assert!(
            !file.1.contains("_SEED"),
            "{prose_only} now names a seed variable in code, not only in prose"
        );
    }
}

//! **No user-facing string literal carries a run of spaces a reader never typed** — the
//! workspace-wide guard behind a defect that had shipped three times before anyone looked.
//!
//! # What goes wrong
//!
//! stikk's sentences are long, so they are written across source lines with Rust's `\` escape,
//! which eats the newline **and the following indentation**:
//!
//! ```text
//! "prikk declined to plan this merge. Nothing was written; \
//!  stikk shows prikk's reason above."
//! ```
//!
//! Drop the `\` -- a shell heredoc that interprets it, a paste, an edit -- and the newline and
//! the indent stay in the value: *"Nothing was written; stikk              shows prikk's reason
//! above."* The source still compiles, and the sentence still **reads** correctly everywhere it
//! is drawn, because `wrap_indented` splits on whitespace and rejoins with single spaces. So
//! nothing fails, nothing looks wrong, and the literal is quietly not what it says.
//!
//! Three had accumulated by RFC 050 -- in RFC 026's, RFC 040's and RFC 047's text -- and were
//! found only by grepping for one introduced the same way. This is that grep, as a test.
//!
//! # What it enforces
//!
//! Across every **shipped** `.rs` file -- test paths excluded, by `seed_guard`'s convention --
//! no run of four or more spaces between two non-space characters on one line. Four, not two,
//! because a wrapped literal's continuation is indented and a genuine two-space gap inside a
//! sentence is a typo this test is not for.
//!
//! **Test files are excluded because they are full of deliberate alignment**: JSON fixtures,
//! indented source samples, expected renderings. Every one of the three defects was in shipped
//! code, which is where a user-facing sentence lives -- and a fixture whose spacing is wrong
//! fails its own assertion, loudly, which is the thing shipped prose does not do.
//!
//! It lives here for `rfc_index`'s and `seed_guard`'s reason: `stikk-real-binary` is
//! `publish = false`, so a test that reads the repository tree can never travel inside a
//! published `.crate`. It is not `#[ignore]`d and needs no prikk binary.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Every shipped `.rs` file, as `(relative path, source)`, test paths excluded.
fn sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    walk(&root, &root.join("crates"), &mut out);
    out.sort();
    assert!(out.len() > 30, "the scan found only {} files", out.len());
    out
}

/// Whether this path is test-only, and so free to align whatever it likes.
///
/// **By path, as `seed_guard` does it**: this project's convention is that every test module
/// lives in its own `tests.rs` or `tests/` directory.
fn is_test_path(relative: &str) -> bool {
    relative.ends_with("/tests.rs")
        || relative.contains("/tests/")
        || relative.contains("/examples/")
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
        out.push((relative, source));
    }
}

/// The least number of spaces that means a lost continuation rather than a typo.
const RUN: usize = 4;

/// Shipped files whose literals **align on purpose**, with the reason each does.
///
/// Two, and both are the same kind of thing: text whose *layout* is the point, so a run of
/// spaces inside it is deliberate. A whole-file entry rather than a line number, because the
/// lines move; adding one is a decision a reviewer makes, which is why the list is in the test
/// and not in the pattern (`seed_guard`'s `ALLOWED`, for `seed_guard`'s reason).
const ALIGNED: &[(&str, &str)] = &[
    (
        "crates/stikk/src/main.rs",
        "the one-shot printout's label column (`prikk:`, `signing:`, `queued:` …) and \
         `--help`'s two columns, both aligned so a terminal reader can scan them",
    ),
    (
        "crates/stikk-tui/src/readme_visuals.rs",
        "invented fixtures for the README images -- a diff hunk with indented source, and the \
         regenerate command shown indented under a sentence",
    ),
];

/// Whether `line` holds a run of `RUN`+ spaces **inside a sentence**: a non-space on each side,
/// and neither of them a quote.
///
/// Two exclusions, and each one is a shape this workspace has on purpose:
///
/// - **The leading indent**, which is why the scan starts at the first non-space character.
///   Every line here is indented, and that is not the defect.
/// - **A run bounded by a quote**, which is an *indent literal* -- `wrap_indented(text, w, "    ")`
///   passes one on nearly every rendering line in `overlay.rs`, and `format!("    {x}")` begins
///   with one. A lost continuation always has words on both sides of the gap, because it is the
///   join of two sentence fragments.
fn has_interior_run(line: &str) -> bool {
    let body = line.trim_start();
    let mut spaces = 0usize;
    let mut previous: Option<char> = None;
    for c in body.chars() {
        if c == ' ' {
            if previous.is_some() {
                spaces += 1;
            }
            continue;
        }
        if spaces >= RUN && previous.is_some_and(|p| p != '"') && c != '"' {
            return true;
        }
        spaces = 0;
        previous = Some(c);
    }
    false
}

/// **The guard.** A line that holds a quote and a run of four or more interior spaces is a
/// literal whose `\` continuation was lost.
///
/// A line with no `"` on it cannot be a literal at all, and a `//` comment is prose a human
/// aligned on purpose — a table in a doc comment is the common case — so both are skipped. What
/// is left is narrow enough that a hit is almost always the defect, and a genuine alignment
/// inside a literal (an ASCII table in a test fixture, say) is rare enough to be worth a look.
#[test]
fn no_string_literal_carries_a_lost_line_continuation() {
    let aligned: Vec<&str> = ALIGNED.iter().map(|(path, _)| *path).collect();
    let mut found: Vec<String> = Vec::new();
    for (relative, source) in sources() {
        if aligned.contains(&relative.as_str()) {
            continue;
        }
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || !line.contains('"') {
                continue;
            }
            if has_interior_run(line) {
                found.push(format!("{relative}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a string literal holds a run of {RUN}+ spaces, which is what a lost `\\` \
         line-continuation leaves behind. The sentence still reads correctly wherever \
         `wrap_indented` draws it, which is why this needs a test rather than an eye — put the \
         `\\` back. If the alignment is deliberate, add the file to `ALIGNED` **with its \
         reason**, in the review request, so a reviewer decides it.\n\n{}",
        found.join("\n")
    );
}

/// Every allowlisted file exists and still aligns something — so a file that stops needing the
/// exemption does not keep it, and a renamed one is noticed here rather than silently widening
/// the hole.
#[test]
fn every_allowlisted_file_still_needs_its_exemption() {
    let root = repo_root();
    for (relative, reason) in ALIGNED {
        let path = root.join(relative);
        assert!(path.is_file(), "{relative} is on the allowlist and is gone");
        assert!(!reason.is_empty(), "{relative} has no reason");
        let source = std::fs::read_to_string(&path).expect("read it");
        assert!(
            source
                .lines()
                .any(|line| !line.trim_start().starts_with("//")
                    && line.contains('\"')
                    && has_interior_run(line)),
            "{relative} no longer aligns anything: take it off the allowlist"
        );
    }
}

/// The detector itself, on the shape it exists for and on the shapes it must not flag.
#[test]
fn the_run_detector_finds_the_defect_and_not_ordinary_code() {
    assert!(has_interior_run(
        "    \"Nothing was written; stikk              shows prikk's reason above.\""
    ));
    assert!(!has_interior_run(
        "            \"prikk declined to plan this merge. Nothing was written; \\\\\""
    ));
    assert!(
        !has_interior_run("    let x = 1;"),
        "an indent is not an interior run"
    );
    assert!(
        !has_interior_run("        "),
        "a blank line is not a run either"
    );
    assert!(
        !has_interior_run("  a   b"),
        "three spaces is a typo's business, not this test's"
    );
    assert!(has_interior_run("  a    b"));
    // **Indent literals are the common shape this must not flag**: a run bounded by quotes.
    assert!(!has_interior_run(
        "        for row in wrap_indented(text, width, \"    \") {"
    ));
    assert!(!has_interior_run("            format!(\"    {role}  \"),"));
}

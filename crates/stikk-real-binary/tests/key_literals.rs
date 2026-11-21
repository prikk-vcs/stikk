//! **No shipped sentence names a key the catalog does not bind** (RFC 061 decision 2; the guard the
//! sweep in its Handoff A §8.1 should have been).
//!
//! # Why a test
//!
//! That sweep was done by hand, and a hand sweep is a claim about a grep I happened to write. It
//! missed one — `view/block.rs`'s *"F saves the selected file, A saves all."*, found while RFC 060
//! Handoff B was looking at something else, after `F` and `A` had been withdrawn. A reader who
//! followed it pressed two letters that do nothing, which is the exact confusion the catalog exists
//! to prevent.
//!
//! # What it enforces
//!
//! Across every **shipped** `.rs` file of `stikk-core` and `stikk-tui`, in **string literals only**
//! (a doc comment may discuss a key's history), a sentence must not name a key that
//! `stikk_core::actions` does not bind. Two shapes, which are the two stikk has actually used:
//!
//! 1. a single letter fenced on its own — ``"Verify (`V`) checks it."``, `"Seal (S) finishes it."`;
//! 2. a bare letter in front of what it does — `"F saves …"`, `"Q: Queue"`.
//!
//! **What it catches, and what it cannot.** It flags a letter the catalog **does not bind**, and any
//! upper-case letter (every bound letter is lower case) — so a withdrawn key named in a sentence
//! fails the build. It cannot flag a sentence that names a letter which still exists but now means
//! something else (*"`s` in the ref picker"*, when `s` seals), nor one that names its key correctly
//! (*"Press b to pick one"*): both read as true to a grep, and only a reader knows that the key line
//! is where keys are named. Those two were found by hand and are recorded in the test below.
//!
//! **It is a heuristic, deliberately.** Naming a key in prose has no syntax to key off, so this
//! matches the shapes that have gone wrong and accepts that a third shape could slip past.
//!
//! It lives here for `literal_guard`'s reason: `stikk-real-binary` is `publish = false`, so a test
//! that reads the repository tree can never travel inside a published `.crate`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Every letter the catalog binds, folded — the ones a sentence may name.
fn bound_letters() -> BTreeSet<char> {
    stikk_core::actions::all()
        .iter()
        .filter_map(|action| action.key)
        .filter_map(stikk_core::Key::letter)
        .collect()
}

/// The verbs a sentence of stikk's own puts after a key, and the shapes `X:` takes.
const VERBS: &[&str] = &[
    "saves",
    "save",
    "opens",
    "open",
    "shows",
    "show",
    "toggles",
    "toggle",
    "compares",
    "compare",
    "seals",
    "seal",
    "commits",
    "commit",
    "removes",
    "remove",
    "adopts",
    "adopt",
    "scrolls",
    "scroll",
    "undoes",
    "undo",
    "switches",
    "switch",
    "imports",
    "import",
    "exports",
    "export",
    "refreshes",
    "refresh",
    "marks",
    "mark",
    "checks",
    "check",
    "lists",
    "list",
    "puts",
    "picks",
];

/// Whether this path is test-only, and so free to name whatever it asserts.
fn is_test_path(relative: &str) -> bool {
    relative.ends_with("/tests.rs")
        || relative.contains("/tests/")
        || relative.contains("/examples/")
}

/// Every shipped `.rs` file of the two crates that hold user-facing strings.
fn sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for crate_dir in ["crates/stikk-core/src", "crates/stikk-tui/src"] {
        walk(&root, &root.join(crate_dir), &mut out);
    }
    out.sort();
    assert!(out.len() > 20, "the scan found only {} files", out.len());
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
        out.push((relative, source));
    }
}

/// The parts of a line that are inside double quotes — a crude split, which is enough: a `"` inside a
/// string literal is escaped, so the halves still land on the right side of the fence.
fn quoted_parts(line: &str) -> Vec<String> {
    line.split('"')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, part)| part.to_string())
        .collect()
}

/// Every key-shaped mention in `text` whose letter the catalog does not bind.
fn unbound_mentions(text: &str, bound: &BTreeSet<char>) -> Vec<String> {
    let mut found = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    for (at, window) in chars.windows(3).enumerate() {
        // 1. `X`, and 3. (X) — a letter on its own, fenced either way.
        let fenced =
            (window[0] == '`' && window[2] == '`') || (window[0] == '(' && window[2] == ')');
        if fenced && window[1].is_ascii_alphabetic() {
            let letter = window[1];
            if !bound.contains(&letter.to_ascii_lowercase()) || letter.is_ascii_uppercase() {
                found.push(format!("{}{letter}{}", window[0], window[2]));
            }
        }
        // 2. `X ` or `X: ` in front of what it does
        let is_start = at == 0 || !chars[at - 1].is_alphanumeric();
        // **`a`, `A` and `I` are words**, so *"a commit captures the whole worktree"* is English and
        // not a key. They are caught by shape 1 where they are meant as keys, which is how they were
        // written when they were (*"`a` adopts a key"*).
        if is_start && window[0].is_ascii_alphabetic() && !"aAI".contains(window[0]) {
            let letter = window[0];
            let rest: String = chars[at + 1..].iter().collect();
            let rest = rest.strip_prefix(':').unwrap_or(&rest);
            let Some(rest) = rest.strip_prefix(' ') else {
                continue;
            };
            let next_word: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '\'')
                .collect();
            let says_what_it_does = VERBS.contains(&next_word.to_ascii_lowercase().as_str())
                // `Q: Queue` — a letter, a colon and a capitalised view name.
                || (chars[at + 1] == ':'
                    && next_word.chars().next().is_some_and(char::is_uppercase));
            if says_what_it_does
                && (!bound.contains(&letter.to_ascii_lowercase()) || letter.is_ascii_uppercase())
            {
                found.push(format!("{letter} {next_word}"));
            }
        }
    }
    found
}

#[test]
fn no_shipped_sentence_names_a_key_the_catalog_does_not_bind() {
    let bound = bound_letters();
    assert!(bound.len() > 10, "only {} letters are bound", bound.len());
    let mut found: Vec<String> = Vec::new();
    for (path, source) in sources() {
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                // A comment may discuss a key that no longer exists; that is its job.
                continue;
            }
            for part in quoted_parts(line) {
                for mention in unbound_mentions(&part, &bound) {
                    found.push(format!("{path}:{}: {mention:?} in {part:?}", number + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "a shipped sentence names a key the catalog does not bind (RFC 061 decision 2); the key \
         line and Enter's list are where keys are named:\n{}",
        found.join("\n")
    );
}

/// **And the guard would have caught the ones that went wrong** — the shapes it keys off are the
/// shapes stikk used, not shapes invented for the test.
#[test]
fn the_guard_catches_every_sentence_that_went_wrong() {
    let bound = bound_letters();
    for went_wrong in [
        "F saves one of this block's files",
        "F saves the selected file, A saves all.",
        "Seal as usual (`S`) to make the undo part of the history.",
        "Press `V` to see what Verify says now.",
        "Verify (`V`) checks it.",
        "A seal was interrupted after it published. Seal (S) finishes it.",
        "If it keeps happening, open Verify (V).",
        "2 patch(es) in the active WAL \u{2014} not yet sealed \u{b7} Q: Queue",
    ] {
        assert!(
            !unbound_mentions(went_wrong, &bound).is_empty(),
            "the guard does not catch {went_wrong:?}"
        );
    }
    // **Beyond it**, and found by hand: the letter is bound and lower case, so a grep cannot know
    // the sentence means another action, or means nothing it should say at all.
    for beyond in [
        "Switch to another branch first (`s` in the ref picker).",
        "Nothing was committed. Changes (`w`) shows them.",
        "commits are whole-worktree \u{2014} there is no staging (u: toggle untracked)",
        "d: compare the worktree with what `commit` would author against, line by line",
        "No ref focused. Press b to pick one.",
    ] {
        assert!(
            unbound_mentions(beyond, &bound_letters()).is_empty(),
            "{beyond:?} is listed as beyond the guard, but the guard caught it — move it up"
        );
    }

    // And it does not fire on the sentences that replaced them.
    for correct in [
        "see the Queue",
        "Seal as usual to make the undo part of the history.",
        "Open Verify to see what it says now.",
        "Switch to another branch first, from its own actions in the ref picker.",
        "Verify checks it.",
        "Nothing was committed. The Changes view shows them.",
        "Enter on a file saves it, or saves every file at this block.",
        "No ref focused. Pick one in Branches and tags.",
        "Compare shows the worktree against what `commit` would author against, line by line",
    ] {
        assert!(
            unbound_mentions(correct, &bound).is_empty(),
            "the guard fires on {correct:?}: {:?}",
            unbound_mentions(correct, &bound)
        );
    }
}

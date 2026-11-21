//! **Bold comes from two named styles, and nothing is italic** — the source check RFC 060 decision 8
//! asks for, in `literal_guard`'s shape and for its reasons.
//!
//! # What goes wrong
//!
//! Emphasis spreads. stikk had **fifty-one** bold uses before RFC 060 (F5) — on headings, labels,
//! keys, values, markers and badges alike — each one reasonable where it was written, and together
//! they left bold marking nothing at all. Italic was worse: three uses, and many terminal fonts draw
//! italic thin, or synthesise it, or ignore it, so the one thing a reader could not rely on was the
//! one thing it was asked to carry.
//!
//! # What it enforces
//!
//! Across every **shipped** `.rs` file of `stikk-tui` (test paths excluded, by `literal_guard`'s
//! convention):
//!
//! - **`Modifier::BOLD` appears only in `theme.rs`**, which is where `theme::heading` and
//!   `theme::selected` live (RFC 060 decision 8). A renderer asks for one of those two, or for no
//!   emphasis at all.
//! - **`Modifier::ITALIC` appears nowhere** (decision 7), with one exemption below.
//!
//! **Tests are exempt** where they assert a modifier: a test that checks the selection is bold has to
//! name `Modifier::BOLD` to do it, and that is the test doing its job.
//!
//! It lives here for `literal_guard`'s reason: `stikk-real-binary` is `publish = false`, so a test
//! that reads the repository tree can never travel inside a published `.crate`. It needs no prikk
//! binary and is not `#[ignore]`d.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Where emphasis is allowed to be named, and why.
///
/// A whole-file list rather than line numbers, because the lines move; adding one is a decision a
/// reviewer makes, which is why the list is in the test (`literal_guard`'s `ALIGNED`, for its reason).
const ALLOWED: &[(&str, &str)] = &[
    (
        "crates/stikk-tui/src/theme.rs",
        "`heading` and `selected` are the two styles RFC 060 decision 8 allows bold from, and this \
         is where they live",
    ),
    (
        "crates/stikk-tui/src/readme_visuals.rs",
        "reads the modifier back off a rendered cell to write the README's SVG -- it asks what was \
         drawn, it does not emphasise anything",
    ),
];

/// Whether this path is test-only, and so free to assert whatever it likes.
fn is_test_path(relative: &str) -> bool {
    relative.ends_with("/tests.rs")
        || relative.contains("/tests/")
        || relative.contains("/examples/")
}

/// Every shipped `.rs` file of `stikk-tui`, as `(relative path, source)`.
fn sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    walk(&root, &root.join("crates/stikk-tui/src"), &mut out);
    out.sort();
    assert!(out.len() > 10, "the scan found only {} files", out.len());
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

/// **Bold comes from `theme.rs` and nowhere else** (RFC 060 decision 8).
#[test]
fn bold_is_named_only_where_the_two_styles_live() {
    let allowed: Vec<&str> = ALLOWED.iter().map(|(path, _)| *path).collect();
    let mut found: Vec<String> = Vec::new();
    for (path, source) in sources() {
        if allowed.contains(&path.as_str()) {
            continue;
        }
        for (number, line) in source.lines().enumerate() {
            if line.contains("Modifier::BOLD") {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "bold must come from `theme::heading` or `theme::selected` (RFC 060 decision 8), not from \
         {} place(s):\n{}",
        found.len(),
        found.join("\n")
    );
}

/// **Nothing is italic** (RFC 060 decision 7).
#[test]
fn nothing_is_italic() {
    let allowed: Vec<&str> = ALLOWED.iter().map(|(path, _)| *path).collect();
    let mut found: Vec<String> = Vec::new();
    for (path, source) in sources() {
        if allowed.contains(&path.as_str()) {
            continue;
        }
        for (number, line) in source.lines().enumerate() {
            if line.contains("Modifier::ITALIC") {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "many terminal fonts draw italic thin, or not at all (RFC 060 decision 7); {} place(s) still \
         ask for it:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// **The two styles are the only ones that add bold**, and `theme.rs` adds it nowhere else — so the
/// exemption above cannot grow quietly into a third source of emphasis.
#[test]
fn theme_adds_bold_in_exactly_two_places() {
    let root = repo_root();
    let theme =
        std::fs::read_to_string(root.join("crates/stikk-tui/src/theme.rs")).expect("read theme.rs");
    let adds = theme.matches("add_modifier(Modifier::BOLD)").count();
    assert_eq!(
        adds, 2,
        "`heading` and `selected` are the two, and `theme.rs` adds bold {adds} time(s)"
    );
    assert!(
        !theme.contains("Modifier::ITALIC"),
        "not even the palette asks for italic"
    );
}

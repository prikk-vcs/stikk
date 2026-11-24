//! **What the guards scan, derived from the workspace** (RFC 067 Handoff B §4).
//!
//! Three guards used to find their files by a hand-written list of crate directories, and the move
//! of the program into `stikk-app` would have shrunk two of them silently: `app.rs` and the overlay
//! state simply left the scan, taking their user-facing strings with them.
//!
//! So the list is not written down any more. It is **every published crate**, read from the
//! workspace itself: a `crates/*/Cargo.toml` that does not say `publish = false`. A crate added
//! later is guarded the day it exists, without an edit here.

// **Compiled into each guard separately**, as any shared `tests/` module is, so every binary sees
// the helpers it does not happen to use as dead. The guards share the derivation, not the uses.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The workspace root, from this test binary's own manifest directory.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the workspace root is two levels above this crate")
        .to_path_buf()
}

/// **Every published crate's name**, in the order the workspace lists their directories.
///
/// `publish = false` is the one thing that takes a crate out: `stikk-real-binary` is excluded that
/// way, and so would be anything else added for the harness alone.
pub fn published_crates() -> Vec<String> {
    let crates = repo_root().join("crates");
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(&crates).unwrap_or_else(|e| panic!("read {}: {e}", crates.display()));
    for entry in entries {
        let dir = entry.expect("a directory entry").path();
        let manifest = dir.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
        if is_unpublished(&text) {
            continue;
        }
        out.push(package_name(&text, &manifest));
    }
    out.sort();
    assert!(
        out.len() >= 6,
        "the workspace should hold at least six published crates, found {}: {out:?}",
        out.len()
    );
    out
}

/// Every published crate's `src` directory.
pub fn published_src_dirs() -> Vec<PathBuf> {
    let root = repo_root();
    published_crates()
        .into_iter()
        .map(|name| root.join("crates").join(name).join("src"))
        .collect()
}

/// Whether this manifest says `publish = false`.
fn is_unpublished(manifest: &str) -> bool {
    manifest.lines().any(|line| {
        let line = line.trim();
        line.starts_with("publish") && line.contains("false")
    })
}

/// The `name` of the `[package]` this manifest declares.
fn package_name(manifest: &str, path: &Path) -> String {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            let value = rest.trim_start().trim_start_matches('=').trim();
            return value.trim_matches('"').to_string();
        }
    }
    panic!("no package name in {}", path.display())
}

/// **Each crate's `stikk-*` dependencies**, for the release lists' order (§4.4).
pub fn stikk_dependencies(name: &str) -> Vec<String> {
    let manifest = repo_root().join("crates").join(name).join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("stikk-") {
            let dep = format!(
                "stikk-{}",
                rest.split_whitespace().next().unwrap_or_default()
            );
            let dep = dep.trim_end_matches('=').trim().to_string();
            if dep != name && !out.contains(&dep) {
                out.push(dep);
            }
        }
    }
    out
}

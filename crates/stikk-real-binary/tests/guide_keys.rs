//! **The guide's key reference names exactly the keys the catalog binds** (RFC 061 decision 6).
//!
//! # Why a test
//!
//! The guide is prose, so it cannot be generated from the catalog — a reader needs sentences, not a
//! dump. That is exactly how it drifted: `TU-05` and the guide between them taught `Q`, `C`, `S`,
//! `i`, `I`, `V` and `v`-for-two-things long after the code had moved on (RFC 061 F3). This test
//! reads the guide's own table and holds it to the catalog, so the prose can stay prose and still be
//! true.
//!
//! It lives here for `literal_guard`'s reason: `stikk-real-binary` is `publish = false`, so a test
//! that reads the repository tree can never travel inside a published `.crate`. It needs no prikk
//! binary and is not `#[ignore]`d.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// The guide's *Key reference* section, from its heading to the next one.
fn key_reference() -> String {
    let guide = std::fs::read_to_string(repo_root().join("docs/src/guide/getting-started.md"))
        .expect("read the guide");
    let from = guide
        .find("## Key reference")
        .expect("the guide has a Key reference section");
    let rest = &guide[from + "## Key reference".len()..];
    let to = rest.find("\n## ").unwrap_or(rest.len());
    rest[..to].to_string()
}

/// Every backticked token in the table's **first column** — the keys the section names.
fn keys_named() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for row in key_reference().lines() {
        let row = row.trim();
        if !row.starts_with('|') || row.starts_with("|---") || row.starts_with("| Key") {
            continue;
        }
        let first = row
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or_default();
        let mut rest = first;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            out.insert(after[..close].to_string());
            rest = &after[close + 1..];
        }
    }
    out
}

/// Every key the catalog binds, one token each.
fn keys_bound() -> BTreeSet<String> {
    stikk_core::actions::all()
        .iter()
        .filter_map(|action| action.key)
        .flat_map(stikk_core::Key::tokens)
        .collect()
}

#[test]
fn the_guides_key_reference_names_exactly_the_catalogs_keys() {
    let named = keys_named();
    let bound = keys_bound();
    assert!(
        named.len() > 15,
        "the section names only {} keys",
        named.len()
    );
    let missing: Vec<&String> = bound.difference(&named).collect();
    assert!(
        missing.is_empty(),
        "the guide's key reference does not name {missing:?}, which the catalog binds"
    );
    let invented: Vec<&String> = named.difference(&bound).collect();
    assert!(
        invented.is_empty(),
        "the guide's key reference names {invented:?}, which the catalog binds to nothing"
    );
}

/// **And the keys that were withdrawn are named nowhere in the guide** (RFC 061 decision 2): a
/// reader who follows the guide must not press a letter that does nothing, or — worse — one that now
/// does something else.
#[test]
fn the_guide_never_names_a_withdrawn_key() {
    let guide = std::fs::read_to_string(repo_root().join("docs/src/guide/getting-started.md"))
        .expect("read the guide");
    let bound = keys_bound();
    let mut found: Vec<String> = Vec::new();
    for (number, line) in guide.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let token = &after[..close];
            // **A letter, or one of the named keys.** Not every one-character span is a key: the
            // terminal font section names `+`, `-`, `0` and `=`, and History's mark is a disc.
            let is_key_shaped = (token.chars().count() == 1
                && token.chars().all(|c| c.is_ascii_alphabetic()))
                || ["Enter", "Esc", "Space", "Tab"].contains(&token);
            if is_key_shaped && !bound.contains(token) {
                found.push(format!("{}: {token:?}", number + 1));
            }
            rest = &after[close + 1..];
        }
    }
    assert!(
        found.is_empty(),
        "the guide names {} key(s) the catalog does not bind:\n{}",
        found.len(),
        found.join("\n")
    );
}

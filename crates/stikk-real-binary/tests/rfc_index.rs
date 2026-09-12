//! The RFC index, asserted against the folders (RFC 022 §7b).
//!
//! `rfcs/README.md` states the rule this file enforces: *"The folder is the source of truth for an
//! RFC's state, and the `Status` field inside each file mirrors it."* That claim went untrue for two
//! releases — four shipped RFCs still listed under Accepted, one still sitting in `accepted/` a release
//! after it shipped, one RFC in no table at all, and a link to a path that had moved — and it was found
//! by a person reading the index while writing a handoff, not by anything mechanical.
//!
//! **This is the same defect class as every sweep finding this project has made**: a claim that was true
//! when written, in a file nothing reads mechanically. The difference is that this one is cheap to gate,
//! because both sides are on disk.
//!
//! It lives in `stikk-real-binary` because that crate is `publish = false`, so a test that reads the
//! repository root can never travel inside a published `.crate` and fail there. It is **not** `#[ignore]`d
//! and needs no prikk binary: it runs in the ordinary `cargo test --workspace` gate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The four state folders, each with the `## ` heading that indexes it and the words a `Status` line in
/// it may use. `done/` accepts two because both are in live use ("Done — shipped in 0.5.0" and
/// "Implemented"); the policy RFC is itself one of the latter.
const FOLDERS: &[(&str, &str, &[&str])] = &[
    ("proposed", "Proposed", &["proposed"]),
    ("accepted", "Accepted", &["accepted"]),
    ("done", "Done", &["done", "implemented"]),
    (
        "archive",
        "Archive",
        &["withdrawn", "superseded", "archived"],
    ),
];

fn repo_root() -> PathBuf {
    // `crates/stikk-real-binary` → the workspace root, the same way `cli_backend/tests.rs` reaches its
    // own module tree from `CARGO_MANIFEST_DIR`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Every `NNN-*.md` in `rfcs/<folder>/`, by folder. A missing folder is an empty set, not a failure:
/// `archive/` legitimately holds nothing today.
fn rfcs_on_disk(root: &Path) -> BTreeMap<&'static str, BTreeSet<String>> {
    FOLDERS
        .iter()
        .map(|(folder, _, _)| {
            let dir = root.join("rfcs").join(folder);
            let mut found = BTreeSet::new();
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries {
                    let name = entry
                        .unwrap_or_else(|e| panic!("reading {dir:?}: {e}"))
                        .file_name()
                        .to_string_lossy()
                        .into_owned();
                    if is_rfc_file(&name) {
                        found.insert(name);
                    }
                }
            }
            (*folder, found)
        })
        .collect()
}

/// `NNN-slug.md` — three leading digits, a hyphen, and a `.md` suffix. Anything else in a state folder
/// (a stray note, a `README`) is not an RFC and is not indexed.
fn is_rfc_file(name: &str) -> bool {
    name.ends_with(".md")
        && name.len() > 4
        && name.as_bytes()[..3].iter().all(u8::is_ascii_digit)
        && name.as_bytes()[3] == b'-'
}

/// Every `](./...)` link in the index, paired with the `## ` heading it appears under.
fn links_by_section(index: &str) -> Vec<(String, String)> {
    let mut section = String::from("(preamble)");
    let mut out = Vec::new();
    for line in index.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.trim().to_string();
        }
        let mut rest = line;
        while let Some(start) = rest.find("](./") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else { break };
            out.push((section.clone(), after[..end].to_string()));
            rest = &after[end..];
        }
    }
    out
}

/// Which folder a `## ` heading indexes, by its first word — `## Done (implemented)` and
/// `## Archive (withdrawn or superseded)` both carry a parenthetical.
fn folder_for_section(section: &str) -> Option<&'static str> {
    let first = section.split_whitespace().next()?;
    FOLDERS
        .iter()
        .find(|(_, heading, _)| *heading == first)
        .map(|(folder, _, _)| *folder)
}

/// Every link in the index resolves to a file that exists.
///
/// This covers handoff links too, not only RFC ones: the drift that prompted this test included a link
/// to a path that had moved, and a handoff path is exactly as likely to move as an RFC's.
#[test]
fn every_link_in_the_rfc_index_resolves() {
    let root = repo_root();
    let index =
        std::fs::read_to_string(root.join("rfcs/README.md")).expect("reading rfcs/README.md");
    let links = links_by_section(&index);
    assert!(
        links.len() > 30,
        "sanity: found only {} links in rfcs/README.md — did the index's shape change, or did the \
         link scanner stop matching it?",
        links.len()
    );
    let mut missing = Vec::new();
    for (_, link) in &links {
        let target = root.join("rfcs").join(link.trim_start_matches("./"));
        if !target.is_file() {
            missing.push(link.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "rfcs/README.md links to paths that do not exist: {missing:#?}\n\
         The index is documentation about files on disk; when one moves, the link moves with it."
    );
}

/// Every RFC appears in exactly one table, and it is the table matching its folder.
///
/// Both halves matter and they fail differently: an RFC in no table is invisible to a reader deciding
/// what to work on; an RFC in the *wrong* table is worse, because it states a lifecycle state the
/// project has already left — which is precisely what "four shipped RFCs still listed under Accepted"
/// was.
#[test]
fn every_rfc_is_indexed_in_the_table_matching_its_folder() {
    let root = repo_root();
    let index =
        std::fs::read_to_string(root.join("rfcs/README.md")).expect("reading rfcs/README.md");
    let on_disk = rfcs_on_disk(&root);

    // Collect indexed RFC links as (section-folder, link-folder, file name).
    let mut indexed: BTreeMap<&'static str, BTreeSet<String>> = FOLDERS
        .iter()
        .map(|(f, _, _)| (*f, BTreeSet::new()))
        .collect();
    let mut misfiled = Vec::new();
    for (section, link) in links_by_section(&index) {
        let path = link.trim_start_matches("./");
        let Some((link_folder, name)) = path.split_once('/') else {
            continue;
        };
        if !is_rfc_file(name) || !FOLDERS.iter().any(|(f, _, _)| *f == link_folder) {
            continue; // a handoff, or the policy link in the preamble's prose
        }
        let Some(section_folder) = folder_for_section(&section) else {
            continue; // the preamble, which links the policy RFC as prose rather than indexing it
        };
        if section_folder != link_folder {
            misfiled.push(format!(
                "{path} is listed under `## {section}`, but lives in `rfcs/{link_folder}/`"
            ));
        }
        indexed
            .get_mut(link_folder)
            .expect("link folder is one of FOLDERS")
            .insert(name.to_string());
    }

    assert!(
        misfiled.is_empty(),
        "rfcs/README.md lists RFCs under the wrong table: {misfiled:#?}\n\
         The folder is the source of truth; move the row, not the file — unless the file is what moved."
    );

    for (folder, _, _) in FOLDERS {
        let disk = &on_disk[folder];
        let listed = &indexed[folder];
        let unlisted: Vec<_> = disk.difference(listed).collect();
        let phantom: Vec<_> = listed.difference(disk).collect();
        assert!(
            unlisted.is_empty(),
            "rfcs/{folder}/ holds RFCs the index never lists: {unlisted:#?}\n\
             Add a row to `## {}`.",
            FOLDERS.iter().find(|(f, _, _)| f == folder).unwrap().1
        );
        assert!(
            phantom.is_empty(),
            "`## {}` lists RFCs that are not in rfcs/{folder}/: {phantom:#?}",
            FOLDERS.iter().find(|(f, _, _)| f == folder).unwrap().1
        );
    }
}

/// Each RFC's own `**Status.**` line names the state its folder implies.
///
/// The index's rule has two sides — the folder is the truth and the `Status` field mirrors it — and a
/// file moved between folders without its `Status` updated satisfies the two tests above while still
/// telling a reader the wrong thing. Only the **first** `**Status.**` line is read: RFC 000 quotes the
/// lifecycle's other state names further down, in prose about the policy itself.
#[test]
fn each_rfcs_status_line_mirrors_its_folder() {
    let root = repo_root();
    let on_disk = rfcs_on_disk(&root);
    let mut wrong = Vec::new();
    let mut checked = 0usize;
    for (folder, _, words) in FOLDERS {
        for name in &on_disk[folder] {
            let path = root.join("rfcs").join(folder).join(name);
            let text =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
            let status = text
                .lines()
                .find(|line| line.trim_start().starts_with("**Status."))
                .unwrap_or_else(|| {
                    panic!(
                        "rfcs/{folder}/{name} has no `**Status.**` line; the index's rule needs one"
                    )
                })
                .to_ascii_lowercase();
            checked += 1;
            if !words.iter().any(|word| status.contains(word)) {
                wrong.push(format!(
                    "rfcs/{folder}/{name}: Status says {status:?}, which names none of {words:?}"
                ));
            }
        }
    }
    assert!(
        checked >= 20,
        "sanity: only {checked} RFCs found on disk — did rfcs/ move?"
    );
    assert!(
        wrong.is_empty(),
        "RFC `Status` lines disagree with the folder they are in: {wrong:#?}\n\
         The folder is the source of truth; the Status line mirrors it."
    );
}

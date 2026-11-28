//! **The release publishes every published crate, in an order that can work** (RFC 067 Handoff B
//! §4.4).
//!
//! `release.yml` names the crates twice, in a shell `for` loop, and a list written by hand is a
//! place to forget. Forgetting it fails at a *tag*, which is the worst moment to find out: the
//! crates job publishes what it was told and the rest of the release carries on without the crate
//! nobody listed.
//!
//! So this holds both lines to the workspace itself, on every push:
//!
//! - **the set** is every crate that does not say `publish = false`;
//! - **the order** is a dependency order — each crate's `stikk-*` dependencies come before it, so
//!   `cargo publish` can succeed one crate at a time.

mod support;

use std::path::PathBuf;

/// The workflow that publishes.
fn release_yml() -> String {
    let path: PathBuf = support::repo_root().join(".github/workflows/release.yml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every `for crate in … ; do` list in the workflow, as its crate names.
fn crate_lists() -> Vec<Vec<String>> {
    let text = release_yml();
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("for crate in ") {
            let list = rest.trim_end_matches("; do").trim();
            out.push(list.split_whitespace().map(str::to_string).collect());
        }
    }
    assert_eq!(
        out.len(),
        2,
        "release.yml should hold exactly two crate lists, found {}",
        out.len()
    );
    out
}

/// **Every published crate is in both lists, and nothing else is.**
#[test]
fn both_release_lists_name_every_published_crate() {
    let published = support::published_crates();
    for (at, list) in crate_lists().into_iter().enumerate() {
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            published,
            "release.yml's list {} is {list:?}, but the workspace publishes {published:?}",
            at + 1
        );
    }
}

/// **Each crate comes after the `stikk-*` crates it depends on**, so publishing in this order can
/// succeed: a crate whose dependency is not on crates.io yet cannot be published.
#[test]
fn both_release_lists_are_in_dependency_order() {
    for (at, list) in crate_lists().into_iter().enumerate() {
        for (position, name) in list.iter().enumerate() {
            for dependency in support::stikk_dependencies(name) {
                let Some(dependency_at) = list.iter().position(|c| *c == dependency) else {
                    panic!(
                        "list {}: {name} depends on {dependency}, which it never names",
                        at + 1
                    );
                };
                assert!(
                    dependency_at < position,
                    "list {}: {name} is published at {position} but its dependency {dependency} \
                     comes later, at {dependency_at}: {list:?}",
                    at + 1
                );
            }
        }
    }
}

/// **The GUI crates are not published, and the release's lists do not name them** (RFC 070
/// decision 1).
///
/// `stikk-gui` and `stikk-gui-design` are `publish = false`, so nothing about the release changes
/// when they arrive. The check above is what holds that, and this is what shows it is load-bearing:
/// the same comparison, against a list with a GUI crate added, must refuse.
#[test]
fn the_gui_crates_are_not_in_the_release_lists() {
    let published = support::published_crates();
    for gui in ["stikk-gui", "stikk-gui-design"] {
        assert!(
            support::repo_root()
                .join("crates")
                .join(gui)
                .join("Cargo.toml")
                .is_file(),
            "{gui} is a crate in this workspace, so this test is about something real"
        );
        assert!(
            !published.contains(&gui.to_string()),
            "{gui} says `publish = false`, so the release must not name it: {published:?}"
        );
    }
    for list in crate_lists() {
        for gui in ["stikk-gui", "stikk-gui-design"] {
            assert!(
                !list.contains(&gui.to_string()),
                "release.yml's list names {gui}: {list:?}"
            );
        }
    }
    // **The run that fails it:** a list with a GUI crate in it is not the published set.
    let mut wrong = published.clone();
    wrong.push("stikk-gui".to_string());
    wrong.sort();
    assert_ne!(
        wrong, published,
        "a list that names a GUI crate differs from the published set, which is what the \
         comparison above would refuse"
    );
}

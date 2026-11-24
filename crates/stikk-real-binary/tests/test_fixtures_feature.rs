//! **`test-fixtures` stays a test feature** (RFC 067 Handoff B review v1 F2).
//!
//! `stikk-app` keeps a handful of constructors behind a `test-fixtures` feature, because a front
//! end's own tests need them across a crate boundary where `#[cfg(test)]` cannot reach:
//! `App::from_state`, `App::push_overlay_for_test`, `OrientRead::stamped`, `Surface::for_test`,
//! `App::focus_ref_for_tests` and `App::surface_error`.
//!
//! **They must never reach a shipped build.** `push_overlay_for_test` is the sharp one: it puts a
//! card on the overlay stack directly, and `Overlay`'s variants are public and constructible, so a
//! front end able to call it could show a confirmation the program never decided to show — built
//! from a plan the front end made up — and a key would act on it.
//!
//! Cargo's resolver keeps a dev-dependency's features out of a normal build, so the rule is simply
//! *where the feature may be named*: a `[dev-dependencies]` entry, and `stikk-app`'s own
//! declaration of it. Never a `[dependencies]` entry, and never a `default` feature list.

mod support;

use std::path::PathBuf;

/// The feature this test exists for.
const FEATURE: &str = "test-fixtures";

/// Every `Cargo.toml` of the workspace: the root, and each crate's.
fn manifests() -> Vec<(String, String)> {
    let root = support::repo_root();
    let mut out = vec![("Cargo.toml".to_string(), read(&root.join("Cargo.toml")))];
    let crates = root.join("crates");
    let entries =
        std::fs::read_dir(&crates).unwrap_or_else(|e| panic!("read {}: {e}", crates.display()));
    for entry in entries {
        let dir = entry.expect("a directory entry").path();
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let name = dir
                .file_name()
                .expect("a crate directory")
                .to_string_lossy()
                .to_string();
            out.push((format!("crates/{name}/Cargo.toml"), read(&manifest)));
        }
    }
    out.sort();
    out
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `[section]` a line sits in, tracked as the file is read.
fn section_of(line: &str, current: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return trimmed.trim_matches(['[', ']']).to_string();
    }
    current.to_string()
}

/// Whether this section is a dev-dependency table, where the feature is allowed.
fn is_dev_dependencies(section: &str) -> bool {
    section == "dev-dependencies"
        || section.starts_with("dev-dependencies.")
        || section.ends_with(".dev-dependencies")
}

/// **The feature is named only where it may be.**
#[test]
fn test_fixtures_is_named_only_in_dev_dependencies() {
    let mut wrong: Vec<String> = Vec::new();
    for (path, text) in manifests() {
        let mut section = String::new();
        for (at, line) in text.lines().enumerate() {
            section = section_of(line, &section);
            let trimmed = line.trim();
            if trimmed.starts_with('#') || !trimmed.contains(FEATURE) {
                continue;
            }
            // `stikk-app` declares the feature itself; that is where it is defined.
            if section == "features" && trimmed.starts_with(FEATURE) {
                continue;
            }
            if is_dev_dependencies(&section) {
                continue;
            }
            wrong.push(format!("{path}:{}: [{section}] {trimmed}", at + 1));
        }
    }
    assert!(
        wrong.is_empty(),
        "`{FEATURE}` may be enabled only by a [dev-dependencies] entry, and declared only by \
         `stikk-app`'s own [features]. Found it elsewhere:\n{}",
        wrong.join("\n")
    );
}

/// **And no default feature list turns it on**, which would reach every build that depends on the
/// crate at all.
#[test]
fn no_default_feature_list_enables_test_fixtures() {
    for (path, text) in manifests() {
        let mut section = String::new();
        for (at, line) in text.lines().enumerate() {
            section = section_of(line, &section);
            let trimmed = line.trim();
            if trimmed.starts_with('#') || section != "features" {
                continue;
            }
            if trimmed.starts_with("default") && trimmed.contains(FEATURE) {
                panic!(
                    "{path}:{}: a default feature list enables `{FEATURE}`",
                    at + 1
                );
            }
        }
    }
}

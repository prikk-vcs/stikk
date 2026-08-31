//! Tests for repository discovery (design TS-06). These build a temporary directory tree under the
//! process temp dir and clean it up.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use super::*;

/// Create a unique temp directory for one test and remove it on drop.
struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(tag: &str) -> Self {
        let mut root = std::env::temp_dir();
        let unique = format!(
            "stikk-handle-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        root.push(unique);
        fs::create_dir_all(&root).expect("create temp root");
        Self { root }
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn discovers_a_repo_from_a_nested_directory() {
    let tree = TempTree::new("discover");
    fs::create_dir_all(tree.root.join(".prikk")).unwrap();
    let nested = tree.root.join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    let handle = RepositoryHandle::discover(&nested).expect("discovers upward");
    assert_eq!(handle.root(), tree.root.as_path());
}

#[test]
fn discovery_fails_cleanly_when_not_in_a_repo() {
    let tree = TempTree::new("norepo");
    let err = RepositoryHandle::discover(&tree.root).expect_err("no .prikk here");
    assert_eq!(err.class(), "environment");
}

#[test]
fn open_requires_a_prikk_directory() {
    let tree = TempTree::new("open");
    assert!(RepositoryHandle::open(&tree.root).is_err());
    fs::create_dir_all(tree.root.join(".prikk")).unwrap();
    assert!(RepositoryHandle::open(&tree.root).is_ok());
}

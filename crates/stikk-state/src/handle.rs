//! Repository discovery and the handle that names a repository (design `stikk-04` MOD-03, data model
//! DM-02).
//!
//! A [`RepositoryHandle`] *names* a repository; it does not describe it (no repository content lives
//! here — data model DM-02). Discovery walks upward from a starting directory looking for a `.prikk`
//! directory, the same way prikk itself finds a repository, so `stikk` launched anywhere inside a
//! worktree finds its root (external design FR-001).

use std::path::{Path, PathBuf};

use stikk_model::{Result, StikkError};

/// prikk's metadata directory name — a directory containing one marks a repository root.
const PRIKK_METADATA_DIR: &str = ".prikk";

/// A named repository: its canonical worktree root (the directory that contains `.prikk`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHandle {
    root: PathBuf,
}

impl RepositoryHandle {
    /// Discover the repository containing `start` by walking upward for a `.prikk` directory.
    ///
    /// # Errors
    /// [`StikkError::Environment`] when no repository is found from `start` up to the filesystem
    /// root — the honest outcome for "this is not inside a prikk repository", not a crash.
    pub fn discover(start: &Path) -> Result<Self> {
        let mut dir: Option<&Path> = Some(start);
        while let Some(current) = dir {
            if current.join(PRIKK_METADATA_DIR).is_dir() {
                return Ok(Self {
                    root: current.to_path_buf(),
                });
            }
            dir = current.parent();
        }
        Err(StikkError::environment_msg(format!(
            "no prikk repository found at or above {}",
            start.display()
        )))
    }

    /// Treat `root` as a repository root without discovery, confirming it holds a `.prikk` directory.
    ///
    /// # Errors
    /// [`StikkError::Environment`] when `root` does not contain a `.prikk` directory.
    pub fn open(root: &Path) -> Result<Self> {
        if root.join(PRIKK_METADATA_DIR).is_dir() {
            Ok(Self {
                root: root.to_path_buf(),
            })
        } else {
            Err(StikkError::environment_msg(format!(
                "{} is not a prikk repository (no .prikk directory)",
                root.display()
            )))
        }
    }

    /// The repository's worktree root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests;

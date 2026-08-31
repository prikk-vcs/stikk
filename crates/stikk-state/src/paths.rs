//! Where stikk's own files live, and the refusal that keeps them out of any repository.
//!
//! Design `stikk-04` MOD-03 `paths.rs`, data model INV-2, threat model C-E2. This module is the one
//! place that knows stikk's on-disk locations, and it enforces the boundary that a repository stays
//! byte-identical whether or not stikk ever opened it (CON-4). [`ensure_outside_repository`] is the
//! **primary** control against writing into a repository — prikk provides no backstop, since it has
//! no general foreign-file scan of `.prikk/` — so every state, cache, and export write goes through
//! it.

use std::path::{Path, PathBuf};

use stikk_model::{Result, StikkError};

/// The name of prikk's metadata directory. A stikk write target may never be inside one.
const PRIKK_METADATA_DIR: &str = ".prikk";

/// Resolve the path of stikk's config file (design CF-01).
///
/// `STIKK_CONFIG` overrides it outright; otherwise it is `<config-base>/stikk/config`, where the base
/// follows the XDG convention (`XDG_CONFIG_HOME`, else `$HOME/.config`). Returns an environment error
/// only when no home can be determined at all.
///
/// # Errors
/// [`StikkError::Environment`] when neither `STIKK_CONFIG` nor a home directory can be resolved.
pub fn config_file() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("STIKK_CONFIG") {
        return Ok(PathBuf::from(explicit));
    }
    Ok(config_base()?.join("stikk").join("config"))
}

/// Resolve the directory holding stikk's session state, caches, and recents (design CF-01).
///
/// `STIKK_STATE_DIR` overrides it; otherwise `<state-base>/stikk/`, where the base follows the XDG
/// convention (`XDG_STATE_HOME`, else `$HOME/.local/state`). Deleting this directory is always safe:
/// stikk stores no secrets and no repository authority here (data model INV-6, DM-N1).
///
/// # Errors
/// [`StikkError::Environment`] when neither `STIKK_STATE_DIR` nor a home directory can be resolved.
pub fn state_dir() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("STIKK_STATE_DIR") {
        return Ok(PathBuf::from(explicit));
    }
    Ok(state_base()?.join("stikk"))
}

fn config_base() -> Result<PathBuf> {
    if let Some(xdg) = non_empty_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg));
    }
    Ok(home()?.join(".config"))
}

fn state_base() -> Result<PathBuf> {
    if let Some(xdg) = non_empty_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(xdg));
    }
    Ok(home()?.join(".local").join("state"))
}

fn home() -> Result<PathBuf> {
    // A full per-platform resolver (Windows `%APPDATA%`, macOS Application Support) is a later
    // increment; the XDG/HOME path is correct on the mutation platforms stikk primarily targets and
    // is honest about its scope rather than guessing. Documented as such (NFR-T rationale).
    non_empty_os("HOME").map(PathBuf::from).ok_or_else(|| {
        StikkError::environment_msg(
            "no home directory found (set HOME, or STIKK_CONFIG/STIKK_STATE_DIR)",
        )
    })
}

fn non_empty_os(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|v| !v.is_empty())
}

/// Refuse a write target that lies inside any repository (design C-E2, the primary boundary control).
///
/// Rejects two shapes: a path with a `.prikk` component anywhere (it would be inside prikk's metadata
/// directory), and — when `repo_root` is provided — a path inside the currently open repository's
/// root (its worktree, where a stray file would additionally be swept into the next `commit`, since
/// prikk has no ignore mechanism). stikk calls this before every state/cache/export write.
///
/// # Errors
/// [`StikkError::Internal`] when `target` is repository-internal — an internal fault, because stikk's
/// own path resolution should never produce such a target; surfacing it as a bug is correct.
pub fn ensure_outside_repository(target: &Path, repo_root: Option<&Path>) -> Result<()> {
    if target
        .components()
        .any(|c| c.as_os_str() == PRIKK_METADATA_DIR)
    {
        return Err(StikkError::Internal {
            detail: format!(
                "refusing to write a stikk file inside a repository metadata directory: {}",
                target.display()
            ),
        });
    }
    if let Some(root) = repo_root {
        if target.starts_with(root) {
            return Err(StikkError::Internal {
                detail: format!(
                    "refusing to write a stikk file inside the open repository worktree: {}",
                    target.display()
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

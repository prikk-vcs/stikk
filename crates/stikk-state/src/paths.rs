//! Where stikk's own files live, and the refusal that keeps them out of any repository.
//!
//! Design `stikk-04` MOD-03 `paths.rs`, data model INV-2, threat model C-E2. This module is the one
//! place that knows stikk's on-disk locations, and it enforces the boundary that a repository stays
//! byte-identical whether or not stikk ever opened it (CON-4). [`ensure_outside_repository`] is the
//! **primary** control against writing into a repository — prikk provides no backstop, since it has
//! no general foreign-file scan of `.prikk/` — so every state, cache, and export write goes through
//! it.
//!
//! **Per-platform resolution is hand-rolled, not a dependency** (`NFR-T01`/`CF-01`; RFC 012 F-c,
//! reversing the first pass of that RFC, which ruled "take the dependency" before measuring — see the
//! RFC for why that was wrong twice, not quietly edited). `stikk-state`'s only edge is `stikk-model`,
//! and this is the crate holding [`ensure_outside_repository`], the threat model's **primary** control
//! (`C-E2`) — worth real effort to keep dependency-free. The resolution logic
//! ([`config_base_with`]/[`state_base_with`]) is written against an injected lookup and an explicit
//! [`Platform`], mirroring `stikk_prikk::env`'s `read_readiness_with` pattern: all three platform
//! branches are exercised hermetically on any host, without touching process-global state or `#[cfg]`
//! in the test logic itself. `STIKK_CONFIG`/`STIKK_STATE_DIR` are still checked before any of this
//! (`CF-04` precedence), and the Linux branch is unchanged from before this RFC — an upgrading user's
//! paths do not move.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use stikk_model::{Result, StikkError};

/// The name of prikk's metadata directory. A stikk write target may never be inside one.
const PRIKK_METADATA_DIR: &str = ".prikk";

/// Resolve the path of stikk's config file (design CF-01; RFC 012 F-c).
///
/// `STIKK_CONFIG` overrides it outright; otherwise it is `<config-base>/stikk/config`, where the base
/// follows this host's platform convention (see the module doc's table). Returns an environment error
/// only when no home/platform base can be determined at all.
///
/// # Errors
/// [`StikkError::Environment`] when neither `STIKK_CONFIG` nor a platform base can be resolved.
pub fn config_file() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("STIKK_CONFIG") {
        return Ok(PathBuf::from(explicit));
    }
    Ok(config_base_with(&real_lookup, Platform::CURRENT)?
        .join("stikk")
        .join("config"))
}

/// Resolve the directory holding stikk's session state, caches, and recents (design CF-01; RFC 012
/// F-c).
///
/// `STIKK_STATE_DIR` overrides it; otherwise `<state-base>/stikk/`, where the base follows this host's
/// platform convention (see the module doc's table). Deleting this directory is always safe: stikk
/// stores no secrets and no repository authority here (data model INV-6, DM-N1).
///
/// # Errors
/// [`StikkError::Environment`] when neither `STIKK_STATE_DIR` nor a platform base can be resolved.
pub fn state_dir() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("STIKK_STATE_DIR") {
        return Ok(PathBuf::from(explicit));
    }
    Ok(state_base_with(&real_lookup, Platform::CURRENT)?.join("stikk"))
}

/// The per-platform path convention to apply (RFC 012 F-c). Real callers get [`Platform::CURRENT`]
/// from `#[cfg]`; tests pass every variant explicitly so all three branches run on any host. Whichever
/// two variants are not this build's `target_os` are legitimately unconstructed by non-test code on
/// this host (each is live in production on its own platform) — `dead_code` is silenced for exactly
/// that reason, not because the variant is actually unused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Platform {
    /// XDG on Linux (and other Unix-likes not covered by the macOS branch) — unchanged from before
    /// RFC 012, so an upgrading user's paths do not move.
    Linux,
    /// `~/Library/Application Support` on macOS — the same base for both config and state (the table
    /// in the module doc has one macOS row spanning both columns).
    MacOs,
    /// `%APPDATA%`/`%LOCALAPPDATA%` on Windows.
    Windows,
}

impl Platform {
    #[cfg(target_os = "macos")]
    const CURRENT: Self = Self::MacOs;
    #[cfg(target_os = "windows")]
    const CURRENT: Self = Self::Windows;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const CURRENT: Self = Self::Linux;
}

/// The config base for `platform`, from an injected presence lookup — the whole per-platform logic;
/// [`config_file`] supplies the real lookup and [`Platform::CURRENT`]. Kept injectable so all three
/// branches are tested hermetically (mirrors `stikk_prikk::env::read_readiness_with`).
fn config_base_with(
    lookup: &impl Fn(&str) -> Option<OsString>,
    platform: Platform,
) -> Result<PathBuf> {
    match platform {
        Platform::Linux => {
            if let Some(xdg) = non_empty(lookup, "XDG_CONFIG_HOME") {
                return Ok(PathBuf::from(xdg));
            }
            Ok(home_with(lookup)?.join(".config"))
        }
        Platform::MacOs => Ok(home_with(lookup)?
            .join("Library")
            .join("Application Support")),
        Platform::Windows => non_empty(lookup, "APPDATA")
            .map(PathBuf::from)
            .ok_or_else(no_home_error),
    }
}

/// The state base for `platform`, from an injected presence lookup. See [`config_base_with`].
fn state_base_with(
    lookup: &impl Fn(&str) -> Option<OsString>,
    platform: Platform,
) -> Result<PathBuf> {
    match platform {
        Platform::Linux => {
            if let Some(xdg) = non_empty(lookup, "XDG_STATE_HOME") {
                return Ok(PathBuf::from(xdg));
            }
            Ok(home_with(lookup)?.join(".local").join("state"))
        }
        Platform::MacOs => Ok(home_with(lookup)?
            .join("Library")
            .join("Application Support")),
        Platform::Windows => non_empty(lookup, "LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(no_home_error),
    }
}

fn home_with(lookup: &impl Fn(&str) -> Option<OsString>) -> Result<PathBuf> {
    non_empty(lookup, "HOME")
        .map(PathBuf::from)
        .ok_or_else(no_home_error)
}

/// The final fallback error, unchanged in wording from before RFC 012 F-c: no platform-specific base
/// resolved (Windows: `APPDATA`/`LOCALAPPDATA` unset) and no `HOME` either (Linux/macOS).
fn no_home_error() -> StikkError {
    StikkError::environment_msg(
        "no home directory found (set HOME, or STIKK_CONFIG/STIKK_STATE_DIR)",
    )
}

fn non_empty(lookup: &impl Fn(&str) -> Option<OsString>, name: &str) -> Option<OsString> {
    lookup(name).filter(|v| !v.is_empty())
}

/// The real environment lookup — presence *and* value both matter here (unlike `stikk_prikk::env`,
/// these are ordinary path fragments, never secret), so this simply wraps `var_os`.
fn real_lookup(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// Refuse a write target that lies inside any repository (design C-E2, the primary boundary control).
///
/// Rejects two shapes: a path with a `.prikk` component anywhere (it would be inside prikk's metadata
/// directory), and — when `repo_root` is provided — a path inside the currently open repository's
/// root (its worktree, where a stray file would additionally risk being captured by the next `commit`:
/// `.prikkignore` (prikk 0.29+, RFC 009 F5) can exclude a matching path, but only if a rule already
/// covers it, so stikk cannot rely on user configuration as a backstop for its own state). stikk calls
/// this before every state/cache/export write.
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

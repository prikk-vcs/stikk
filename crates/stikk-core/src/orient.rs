//! The orientation operation (design VW-01, FR-002; use case UC-01).
//!
//! When a session opens a repository, orientation is what it shows first: the prikk version and
//! whether it is supported, the repository's queue depth and worktree marker, and — derived from
//! signing readiness — what this session is capable of. It is read-only and needs no signing.
//!
//! This is the shape every operation follows: gather from the seam, derive a view-model, hand it up.
//! The frontends render the view-model; they do not compute it.

use std::path::Path;

use stikk_model::{Capability, Readiness, Result};
use stikk_prikk::{Prikk, env};

/// Everything the Orientation view shows (design VW-01). A plain value handed to a frontend to
/// render; it is never authority and is re-derived on refresh (data model INV-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationView {
    /// prikk's raw version line, verbatim.
    pub prikk_version: String,
    /// Whether this prikk version is within stikk's validated range; when false the UI degrades
    /// mutation to read-only and says why (NFR-R03).
    pub prikk_supported: bool,
    /// Patches queued in the active WAL, not yet sealed.
    pub queued_patches: u64,
    /// Trailing partial WAL bytes, if any — a torn tail worth surfacing.
    pub trailing_partial_wal_bytes: u64,
    /// The current `heads/main` RefState id, if published.
    pub main_ref_state: Option<String>,
    /// The capability this session has, derived from signing readiness (design AC-01…04).
    pub capability: Capability,
    /// The readiness the capability was derived from, for the signing-readiness badges (FR-104).
    pub readiness: Readiness,
}

/// Produce the orientation view for the repository rooted at `repo`, driving `prikk` through the seam.
///
/// Signing readiness is read from the environment as **presence only** (never seed values) via the
/// seam's [`env`] module (threat model C-I1); the read-only override is folded in.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises (an environment fault reaching prikk,
/// a refusal such as a retired repository format, a lock conflict).
pub fn orient(prikk: &impl Prikk, repo: &Path) -> Result<OrientationView> {
    let handshake = prikk.handshake()?;
    let orientation = prikk.orientation(repo)?;
    let readiness = env::read_readiness(env::read_only_override());
    let capability = Capability::derive(readiness);
    Ok(OrientationView {
        prikk_version: handshake.raw_version,
        prikk_supported: handshake.supported,
        queued_patches: orientation.queued_patches,
        trailing_partial_wal_bytes: orientation.trailing_partial_wal_bytes,
        main_ref_state: orientation.main_ref_state,
        capability,
        readiness,
    })
}

#[cfg(test)]
mod tests;

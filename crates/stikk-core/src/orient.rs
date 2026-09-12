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
use stikk_prikk::Prikk;

/// The lowest prikk version where a commit message actually persists (schema 4, upstream RFC 123;
/// RFC 015 F2) rather than being validated and discarded (`UD-01`, retired at this version). Named
/// here rather than on `stikk_prikk::Version` itself: that type's own floor/ceiling
/// (`SUPPORTED_MIN_MINOR`/`VALIDATED_MAX_MINOR`) are about what stikk has validated *its own parsing*
/// against, a different question from what a specific prikk release does with a message.
const MESSAGES_PERSIST_MIN: (u32, u32, u32) = (0, 32, 0);

/// Everything the Orientation view shows (design VW-01). A plain value handed to a frontend to
/// render; it is never authority and is re-derived on refresh (data model INV-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationView {
    /// prikk's raw version line, verbatim.
    pub prikk_version: String,
    /// Whether this prikk version is at or above the floor stikk requires; when false the UI degrades
    /// mutation to read-only and says why (NFR-R03).
    pub prikk_supported: bool,
    /// Whether this prikk version is within the range stikk has actually validated its output shapes
    /// against (RFC 009 decision 7). A `prikk_supported` version that is not `prikk_validated` still
    /// runs; the UI states that its shapes have not been checked, rather than asserting a validation
    /// stikk has not done.
    pub prikk_validated: bool,
    /// The validated ceiling as a display string (e.g. `"0.41"`), for the `!prikk_validated` UI copy
    /// (`NFR-R03`) — read from [`stikk_prikk::validated_ceiling_display`] rather than hardcoded, so a
    /// renderer can never carry a stale number the way `stikk-tui`'s Orientation view once did (RFC
    /// 015: still said "0.30" after RFC 012 F-e had already raised the ceiling to 31).
    pub validated_through: String,
    /// Whether this prikk version persists a commit message (schema 4, upstream RFC 123) rather than
    /// validating and discarding it (`UD-01`, retired at prikk 0.32 — RFC 015 F2). stikk supports both
    /// sides of that boundary, so the commit message prompt's copy reads this rather than asserting
    /// either blanket claim.
    pub prikk_persists_messages: bool,
    /// Patches queued in the active WAL, not yet sealed.
    pub queued_patches: u64,
    /// The ref the queue targets, when prikk reports one — absent only when the queue is empty
    /// (RFC 009 F1). The same fact behind the Changes view's `queued_elsewhere` warning.
    pub queued_target: Option<String>,
    /// Trailing partial WAL bytes, if any — a torn tail worth surfacing.
    pub trailing_partial_wal_bytes: u64,
    /// The current `heads/main` RefState id, if published.
    pub main_ref_state: Option<String>,
    /// The capability this session has, derived from signing readiness (design AC-01…04).
    pub capability: Capability,
    /// The readiness the capability was derived from, for the signing-readiness badges (FR-104).
    pub readiness: Readiness,
    /// Which `PRIKK_*_SEED` variables are set on a prikk that no longer reads them (RFC 026 §4).
    ///
    /// **Only this, not the seam's whole `RoleDetail`.** The Orientation view renders one sentence
    /// about stale variables and nothing else from that struct; the key id and prikk's `reason` are
    /// read where they are used — on the confirmation cards, straight from
    /// [`stikk_prikk::Prikk::readiness`]. Carrying the rest here would be a view holding data it does
    /// not render, and it pushed `OrientationState` past the size at which its variants diverge.
    pub stale_seed_variables: stikk_prikk::env::StaleSeedVariables,
}

/// Produce the orientation view for the repository rooted at `repo`, driving `prikk` through the seam.
///
/// Signing readiness comes from the seam's own [`stikk_prikk::Prikk::readiness`] method (RFC 026),
/// which picks its band by prikk version: environment presence at ≤ 0.39, unverifiable at 0.40, and
/// prikk's `key status` at ≥ 0.41. **No seed value is read on any band** (threat model C-I1); the
/// read-only override is folded in by the seam.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises (an environment fault reaching prikk,
/// a refusal such as a retired repository format, a lock conflict).
pub fn orient(prikk: &impl Prikk, repo: &Path) -> Result<OrientationView> {
    let handshake = prikk.handshake()?;
    let orientation = prikk.orientation(repo)?;
    let report = prikk.readiness(repo)?;
    let readiness = report.readiness;
    let capability = Capability::derive(readiness);
    let version = handshake.version;
    let prikk_persists_messages =
        (version.major, version.minor, version.patch) >= MESSAGES_PERSIST_MIN;
    Ok(OrientationView {
        prikk_version: handshake.raw_version,
        prikk_supported: handshake.supported,
        prikk_validated: handshake.validated,
        validated_through: stikk_prikk::validated_ceiling_display(),
        prikk_persists_messages,
        queued_patches: orientation.queued_patches,
        queued_target: orientation.queued_target,
        trailing_partial_wal_bytes: orientation.trailing_partial_wal_bytes,
        main_ref_state: orientation.main_ref_state,
        capability,
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables {
            author: report.author.stale_seed_variable,
            maintainer: report.maintainer.stale_seed_variable,
        },
    })
}

#[cfg(test)]
mod tests;

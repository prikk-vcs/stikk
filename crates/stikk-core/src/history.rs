//! History and block-detail operations (design FR-010/011/031/032; RFC 006).
//!
//! `history_view` composes a ref's block lineage with the unsealed queue count into one view-model;
//! `block_detail` assembles a block's metadata with — for the ref tip only — its replayed state file
//! set (prikk replays to the tip, not to an arbitrary historical block, RFC 006). `list_refs` is the
//! ref picker's source. Everything here is read-only and computes nothing prikk did not report.

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{BlockRow, Prikk, RefEntry, StateFiles};

/// A ref's history for the History view (design FR-010/011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryView {
    /// The ref this lineage belongs to.
    pub reff: String,
    /// Patches queued in the active WAL, not yet sealed — the "not yet history" tier (FR-010).
    pub queued: u64,
    /// Sealed blocks, newest-first.
    pub blocks: Vec<BlockRow>,
}

/// A single block's detail (design FR-031/032 at block granularity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDetailView {
    /// The block's metadata, from the history listing.
    pub row: BlockRow,
    /// Whether this block is the ref's tip (the only block prikk can show state for).
    pub is_tip: bool,
    /// The replayed state file set — present only for the tip; `None` for older blocks, where prikk
    /// exposes no per-block state (RFC 006, UD-09).
    pub state: Option<StateFiles>,
}

/// Produce the History view for `reff`, driving prikk through the seam (design FR-010/011).
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn history_view(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
    limit: usize,
) -> Result<HistoryView> {
    let history = prikk.history(repo, reff, limit)?;
    let queued = prikk.orientation(repo)?.queued_patches;
    Ok(HistoryView {
        reff: history.reff,
        queued,
        blocks: history.blocks,
    })
}

/// Assemble a block's detail. `is_tip` says whether `row` is `reff`'s tip; only then is the replayed
/// state file set fetched (prikk cannot replay to an older block — RFC 006).
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises while reading tip state.
pub fn block_detail(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
    row: BlockRow,
    is_tip: bool,
) -> Result<BlockDetailView> {
    let state = if is_tip {
        Some(prikk.block_state(repo, reff)?)
    } else {
        None
    };
    Ok(BlockDetailView { row, is_tip, state })
}

/// List every ref pointer — branches and tags — for the ref picker (design FR-014, completed RFC 012).
///
/// Merges [`Prikk::refs`] and [`Prikk::tags`], **de-duplicated by name**: `Prikk::refs`'s own
/// documentation notes it does not reliably exclude tags (`prikk branch list --all`'s real
/// implementation lists every ref pointer regardless of namespace — discovered empirically while
/// building this very completion, not documented prikk behavior stikk can rely on), so naively
/// concatenating the two lists would show every tag twice the day that leak is present, and merging by
/// name is what stays correct whether or not it is. An entry from `tags` wins on a name collision (the
/// documented, stable source for tags); order is `refs` first, then any `tags` entry not already named.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises, from either read.
pub fn list_refs(prikk: &impl Prikk, repo: &Path) -> Result<Vec<RefEntry>> {
    let branches = prikk.refs(repo)?;
    let tags = prikk.tags(repo)?;
    let mut merged: Vec<RefEntry> = branches
        .into_iter()
        .filter(|entry| !tags.iter().any(|tag| tag.name == entry.name))
        .collect();
    merged.extend(tags);
    Ok(merged)
}

#[cfg(test)]
mod tests;

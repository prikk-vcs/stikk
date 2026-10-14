//! History and block-detail operations (design FR-010/011/031/032; RFC 006).
//!
//! `history_view` composes a ref's block lineage with the unsealed queue count into one view-model;
//! `block_detail` assembles a block's metadata with — for the ref tip only — its replayed state file
//! set (prikk replays to the tip, not to an arbitrary historical block, RFC 006). `list_refs` is the
//! ref picker's source. Everything here is read-only and computes nothing prikk did not report.

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{BlockRow, Prikk, RefEntry, ShowReport, ShownPatch, StateFiles};

use crate::patch::{CONTENT_FROM_MINOR, PatchMessageState};
use crate::piece::Piece;

/// A ref's history for the History view (design FR-010/011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryView {
    /// The ref this lineage belongs to.
    pub reff: String,
    /// Patches queued in the active WAL, not yet sealed — the "not yet history" tier (FR-010).
    pub queued: u64,
    /// The ref the active queue targets, from the same Orientation read as `queued` (RFC 028 decision 5).
    /// `None` for an empty queue, and when prikk reports the target's metadata as missing or malformed.
    pub queued_target: Option<String>,
    /// Sealed blocks, newest-first.
    pub blocks: Vec<BlockRow>,
}

impl HistoryView {
    /// The queued tier's line, exactly (RFC 028 decision 5; Handoff A §5), or `None` when nothing is
    /// queued.
    ///
    /// **The queue is repository-wide, and the tier says whose it is.** It used to put the whole queue's
    /// count on any ref's lineage (RFC 028 F5), which is true of the WAL and wrong about the ref it sat on.
    /// The target is repository text, so the frontend renders the line through `inert` (`C-T2a`).
    #[must_use]
    pub fn queued_tier(&self) -> Option<Vec<Piece>> {
        let n = self.queued;
        match self.queued_target.as_deref() {
            _ if n == 0 => None,
            Some(target) if target == self.reff => Some(vec![Piece::said(format!(
                "{n} patch(es) in the active WAL \u{2014} not yet sealed \u{b7} Q: Queue"
            ))]),
            // **The target is a name the repository chose** (RFC 036 review v1 §2.3), so it comes
            // back as its own piece and the frontend renders it under the identifier rule.
            Some(target) => Some(vec![
                Piece::said(format!("the active WAL holds {n} patch(es) for ")),
                Piece::name(target.to_string()),
                Piece::said(" \u{2014} not this ref's history \u{b7} Q: Queue"),
            ]),
            None => Some(vec![Piece::said(format!(
                "the active WAL holds {n} patch(es); prikk reports no target ref \u{b7} Q: Queue"
            ))]),
        }
    }
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
    /// The block's patches, from its own `show <block-id>` report (`FR-030`; RFC 035 §4.1) — **for
    /// every block, not only the tip**, since `show` takes a block id and needs no replay.
    pub patches: BlockPatches,
}

/// A block's patches, as Block detail lists them (RFC 035 §4.1).
///
/// **Three variants, because "not listed" has three different reasons and they are not the same fact**
/// (`C-T2c′`): prikk reported them, this prikk reports none at all, or the read failed. The first two
/// are the shape [`stikk_prikk::QueueReport`] takes, for the same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockPatches {
    /// prikk ≥ 0.39: one row per patch, in prikk's order.
    Listed(Vec<BlockPatchRow>),
    /// Below 0.39, carrying that prikk's minor for the sentence the view shows. **stikk's own voice**
    /// (`C-T4d`): it reads a patch's content from 0.39 on. *"prikk does not report it"* would be false
    /// at 0.36–0.38, where `show-report-v1` exists and stikk reads no JSON at all.
    Unreported(u32),
    /// **The read failed, and the rest of Block detail is unaffected** (review v1 §2.3). The block's
    /// own metadata and the tip's replayed state file set were readable before this section existed
    /// and must not start depending on it: a strict reader's error about the *patches* is not a reason
    /// to take away the block.
    ///
    /// The string is the error as [`stikk_model::StikkError`] spells it — its class, then prikk's
    /// verbatim message for a refusal or stikk's own detail for an environment fault (`ER-02`), the
    /// same one-line spelling `OrientationState::Failed` carries.
    Failed(String),
}

impl BlockPatches {
    /// The sentence to show **instead of** a list, or `None` when the patches are listed: either this
    /// prikk reports none, or the read failed.
    #[must_use]
    pub fn unreported_reason(&self) -> Option<String> {
        match self {
            Self::Listed(_) => None,
            Self::Unreported(minor) => Some(format!(
                "stikk reads a patch's content from prikk 0.{CONTENT_FROM_MINOR} on; this is \
                 0.{minor}."
            )),
            Self::Failed(error) => Some(format!(
                "stikk could not read this block's patches — {error}"
            )),
        }
    }

    /// The listed rows, or an empty slice. **Never a stand-in for the other two** — the caller asks
    /// [`Self::unreported_reason`] first, and the view shows the sentence rather than a list of
    /// nothing.
    #[must_use]
    pub fn rows(&self) -> &[BlockPatchRow] {
        match self {
            Self::Listed(rows) => rows,
            Self::Unreported(_) | Self::Failed(_) => &[],
        }
    }
}

/// One patch of a block, for the Patches section and the detail `Enter` opens from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPatchRow {
    /// The patch exactly as prikk reported it. **Carried, not re-read**: `Enter` renders Patch detail
    /// from this, so opening a patch spawns nothing (RFC 035 decision 6, §5 trap 7).
    pub patch: ShownPatch,
    /// The patch's message, **joined by `patch_id`** from the block's `log` row, in its own three
    /// states (`C-T2c′`). Never joined by position: `patch_messages` omits a patch that carries no
    /// message, so positions drift (§5 trap 4).
    pub message: PatchMessageState,
}

impl BlockPatchRow {
    /// The row's one line, **as pieces** (RFC 036 review v1 §2.3): the short patch id, the message,
    /// and the operation count. The id is a name the repository chose, so the frontend renders it
    /// under its identifier rule; the message is prose, and the rest is stikk's.
    #[must_use]
    pub fn summary(&self) -> Vec<Piece> {
        let short: String = self
            .patch
            .patch_id
            .chars()
            .take(crate::SHORT_ID_CHARS)
            .collect();
        let count = self.patch.operations.len();
        let tail = format!("  \u{b7}  {count} operation(s)");
        match &self.message {
            PatchMessageState::Text(text) => {
                let mut lines = text.lines();
                let first = lines.next().unwrap_or_default().to_string();
                let ellipsis = if lines.next().is_some() {
                    "\u{2026}"
                } else {
                    ""
                };
                vec![
                    Piece::name(short),
                    Piece::said("  "),
                    Piece::said(format!("{first}{ellipsis}")),
                    Piece::said(tail),
                ]
            }
            PatchMessageState::Absent => vec![
                Piece::name(short),
                Piece::said(format!("  (no message){tail}")),
            ],
            PatchMessageState::NotReported => vec![
                Piece::name(short),
                Piece::said(format!("  (no message reported){tail}")),
            ],
        }
    }
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
    // RFC 028 decision 5: the count and the target from one Orientation read, so the tier cannot pair
    // one moment's count with another's target.
    let orientation = prikk.orientation(repo)?;
    Ok(HistoryView {
        reff: history.reff,
        queued: orientation.queued_patches,
        queued_target: orientation.queued_target,
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
    // RFC 035 §4.1: the block's own patches, for **every** block. `show` takes a block id and resolves
    // against that block, so nothing here is tip-only — unlike the state read above, which is, because
    // prikk replays only to the tip.
    //
    // **A failure here is carried, not propagated** (review v1 §2.3). Everything above — the block's
    // metadata, and the tip's replayed state — worked before this read existed, and a strict reader
    // refusing a report about the *patches* is not a reason to take the block away from the reader.
    let patches = match prikk.show(repo, &row.block_id) {
        Err(error) => BlockPatches::Failed(error.to_string()),
        Ok(ShowReport::Unreported) => BlockPatches::Unreported(prikk.handshake()?.version.minor),
        Ok(ShowReport::Listed(patches)) => BlockPatches::Listed(
            patches
                .into_iter()
                .map(|patch| BlockPatchRow {
                    // Joined by id, never by position (§5 trap 4).
                    message: PatchMessageState::from_block_join(
                        row.messages
                            .iter()
                            .find(|message| message.patch_id == patch.patch_id)
                            .map(|message| message.message.as_str()),
                    ),
                    patch,
                })
                .collect(),
        ),
    };
    Ok(BlockDetailView {
        row,
        is_tip,
        state,
        patches,
    })
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

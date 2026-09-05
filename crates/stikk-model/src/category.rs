//! The request-category vocabulary the seam and operation layer share.
//!
//! Design `stikk-04` CT-03, AR-05. Rather than each operation inventing its own idempotency,
//! cancellability, and locking story, every request the seam can make belongs to one of these nine
//! categories, and the category carries that policy as data. The operation layer reads the policy off
//! the type: a mutating category never auto-retries (NFR-S04), and stikk holds a single per-repository
//! mutation gate for all of them (design CC-02).

/// The confirmation tier a request's category requires before it may execute (design `FR-121`; RFC 013
/// decision 4). Derived from [`RequestCategory`] alone, **never** declared per operation, so a new
/// operation cannot forget to be gated — the same mechanical guarantee `FR-123` gives frontend parity.
///
/// Ordering matters: `Tier::One < Tier::Two < Tier::Three < Tier::ThreeTyped`, so a `>=` comparison
/// reads as "requires at least this much ceremony."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Free — no confirmation, ever (RFC 013 decision 6: a standing constraint, not a default). Every
    /// read category is this tier.
    One,
    /// Queue-affecting: an explicit yes.
    Two,
    /// History-publishing: restate the operation, then an explicit yes.
    Three,
    /// Recovery/trust: restate, then the typed target name, matched exactly (`FR-102`/`FR-103`).
    ThreeTyped,
}

/// One of the nine categories every prikk request belongs to (design CT-03 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum RequestCategory {
    /// Reading history: ref lists, lineage, patch/block reads, ref chains.
    ReadHistory,
    /// Reading state: state trees, content at a block, comparisons.
    ReadState,
    /// Worktree analysis: changes vs a baseline.
    WorktreeAnalysis,
    /// Queue mutation: commit, rollback draft.
    QueueMutation,
    /// Publication: seal, merge execution, branch/tag publication.
    Publication,
    /// Exchange: bundle export/verify/import, sync build/accept/seal.
    Exchange,
    /// Integrity: verify, doctor (read).
    Integrity,
    /// Trust: key add/remove, tag adoption, readiness probe.
    Trust,
    /// Recovery: doctor repair, lock clearing, compaction.
    Recovery,
}

impl RequestCategory {
    /// True when a request in this category can change repository state. A mutating request is
    /// single-shot and never auto-retried (design SEAM-04, NFR-S04); a read never mutates.
    #[must_use]
    pub const fn mutates(self) -> bool {
        match self {
            Self::ReadHistory | Self::ReadState | Self::WorktreeAnalysis | Self::Integrity => false,
            Self::QueueMutation
            | Self::Publication
            | Self::Exchange
            | Self::Trust
            | Self::Recovery => true,
        }
    }

    /// The confirmation tier this category requires (design `FR-121`; RFC 013 decision 4). Every read
    /// category is [`Tier::One`] (free — RFC 013 decision 6); every mutating category maps to
    /// [`Tier::Two`], [`Tier::Three`], or [`Tier::ThreeTyped`], beside [`Self::mutates`] for the same
    /// reason: a new category cannot be added here without a reviewer also deciding its tier.
    #[must_use]
    pub const fn tier(self) -> Tier {
        match self {
            Self::ReadHistory | Self::ReadState | Self::WorktreeAnalysis | Self::Integrity => {
                Tier::One
            }
            Self::QueueMutation => Tier::Two,
            Self::Publication | Self::Exchange => Tier::Three,
            Self::Trust | Self::Recovery => Tier::ThreeTyped,
        }
    }

    /// True when a request in this category may be cancelled while in flight. Reads are freely
    /// cancellable; a mutation is cancellable only *before* it executes (the operation layer's
    /// preview stage), never mid-write, since prikk's own call is the atomic unit.
    #[must_use]
    pub const fn cancellable_in_flight(self) -> bool {
        !self.mutates()
    }

    /// The stable machine-readable name, for the report format (design CL-07) and tests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ReadHistory => "read-history",
            Self::ReadState => "read-state",
            Self::WorktreeAnalysis => "worktree-analysis",
            Self::QueueMutation => "queue-mutation",
            Self::Publication => "publication",
            Self::Exchange => "exchange",
            Self::Integrity => "integrity",
            Self::Trust => "trust",
            Self::Recovery => "recovery",
        }
    }

    /// Every category, in declaration order — used by the parity check that no category is dropped.
    pub const ALL: [Self; 9] = [
        Self::ReadHistory,
        Self::ReadState,
        Self::WorktreeAnalysis,
        Self::QueueMutation,
        Self::Publication,
        Self::Exchange,
        Self::Integrity,
        Self::Trust,
        Self::Recovery,
    ];
}

#[cfg(test)]
mod tests;

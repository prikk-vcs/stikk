//! The request-category vocabulary the seam and operation layer share.
//!
//! Design `stikk-04` CT-03, AR-05. Rather than each operation inventing its own idempotency,
//! cancellability, and locking story, every request the seam can make belongs to one of these ten
//! categories, and the category carries that policy as data. The operation layer reads the policy off
//! the type: a mutating category never auto-retries (NFR-S04), and stikk holds a single per-repository
//! mutation gate for all of them (design CC-02).
//!
//! **The signing requirement is the category's too** ([`RequestCategory::signing_requirement`],
//! RFC 045 §2b). It used to be read off the tier, which worked only while the two happened to
//! agree: [`RequestCategory::Materialization`] is Tier Three and needs **no key at all** (RFC 045
//! F1, measured), so a tier-keyed gate would have demanded a maintainer key for an operation that
//! never signs anything.

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

/// What signing readiness a category's requests need before they may run (design `AC-01`…`AC-04`;
/// RFC 045 §2b).
///
/// **Beside the tier, not derived from it.** The tier says how much ceremony the *user* performs;
/// this says what the *session* must already have. They coincide for every category that existed
/// before RFC 045, which is exactly why reading one off the other went unnoticed until a Tier
/// Three operation needed no key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SigningRequirement {
    /// Nothing beyond being able to operate at all — a human at the machine, not in read-only
    /// mode ([`crate::Readiness::may_operate`]).
    ReadWrite,
    /// An AUTHOR signing key.
    Author,
    /// A MAINTAINER signing key.
    Maintainer,
}

/// One of the ten categories every prikk request belongs to (design CT-03 table).
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
    /// Materialization: **writes the worktree from history** — branch switch now, checkout if it
    /// ever comes (RFC 045 decision 2).
    ///
    /// **Appended rather than placed beside [`Self::Publication`]**, which is where it belongs by
    /// meaning: these variants derive `Ord`, and inserting one in the middle would silently
    /// renumber every category after it.
    Materialization,
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
            | Self::Recovery
            | Self::Materialization => true,
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
            // **Exchange is Tier Two** (RFC 050 decision 2): measured at prikk 0.53, neither
            // `bundle export` nor `bundle import` signs anything, and an import creates and
            // advances no local ref -- it stores received history under `remotes/`. `FL-11`
            // asks for a tier-2 confirmation and no more.
            Self::QueueMutation | Self::Exchange => Tier::Two,
            // **Materialization is Tier Three, not ThreeTyped** (RFC 045 decision 3): a switch
            // refuses rather than overwrite anything not in history, every refusal writes
            // nothing, and switching back restores every byte (F2, F3). What the user needs is to
            // see which files change, which the preview gives.
            Self::Publication | Self::Materialization => Tier::Three,
            Self::Trust | Self::Recovery => Tier::ThreeTyped,
        }
    }

    /// What signing readiness this category's requests need (RFC 045 §2b).
    ///
    /// **This is the gate's key, and the tier is not.** Every category keeps exactly the
    /// requirement its tier implies — `the_requirement_matches_what_the_tier_used_to_imply`
    /// asserts that over [`Self::ALL`], with [`Self::Materialization`] as the one exception it
    /// names: Tier Three, and no key, because prikk needs none (RFC 045 F1).
    /// [`Self::Exchange`] moved the other way in RFC 050 — down to Tier Two, where the implied
    /// requirement is AUTHOR — so it is the second exception: Tier Two, and **no key at all**,
    /// because neither an export nor an import signs anything (measured at prikk 0.53).
    #[must_use]
    pub const fn signing_requirement(self) -> SigningRequirement {
        match self {
            Self::QueueMutation => SigningRequirement::Author,
            Self::Publication => SigningRequirement::Maintainer,
            // The reads never reach a gate at all (`Tier::One` short-circuits it), and
            // `Trust`/`Recovery` are `ThreeTyped`, whose ceremony is the typed name rather than a
            // key: Operator is orthogonal to the signing ladder (`AC-04`).
            //
            // **`Exchange` needs no key** (RFC 050 decision 2): measured at prikk 0.53, an export
            // and an import both succeed with no key configured at all. An import trusts nobody --
            // it records the bundle's author-key material for continuity and says so -- so asking
            // for a MAINTAINER key would have been ceremony over an operation that signs nothing.
            Self::ReadHistory
            | Self::ReadState
            | Self::WorktreeAnalysis
            | Self::Integrity
            | Self::Exchange
            | Self::Trust
            | Self::Recovery
            | Self::Materialization => SigningRequirement::ReadWrite,
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
            Self::Materialization => "materialization",
        }
    }

    /// Every category, in declaration order — used by the parity check that no category is dropped.
    pub const ALL: [Self; 10] = [
        Self::ReadHistory,
        Self::ReadState,
        Self::WorktreeAnalysis,
        Self::QueueMutation,
        Self::Publication,
        Self::Exchange,
        Self::Integrity,
        Self::Trust,
        Self::Recovery,
        Self::Materialization,
    ];
}

#[cfg(test)]
mod tests;

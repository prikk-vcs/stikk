//! Derived capability levels and the signing readiness they come from.
//!
//! Design `stikk-04` AC-01…04, OPL-04. stikk has no accounts. What a session may do is *derived*
//! per session from prikk-side facts — which signing roles are ready, and whether read-only mode is
//! on — and is displayed, never stored. The capability is checked twice (design OPL-04): once to
//! decide UI affordance, once at the seam before a mutating call, because readiness can lapse between
//! render and click.
//!
//! Critically, [`Readiness`] records only **whether** a role's key material is present, never the
//! material itself (threat model C-I1, data model LC-13). This type cannot hold a secret.

/// Whether prikk's repository-side trust policy has adopted a MAINTAINER key (design `FR-104`; RFC
/// 016 F2/F3).
///
/// Presence of `PRIKK_MAINTAINER_KEY_ID`/`_SEED` in the environment is necessary but not sufficient:
/// prikk's `verify_signer_trusted` also requires the key to be **adopted** in the repository's trust
/// policy before a MAINTAINER-gated operation succeeds. **Adoption is object trust, not ref
/// authority** — prikk accepts that key's signatures on objects; adopting a key never lets it move a
/// ref (`RefStore::publish` still requires this operator's own signature). Say so wherever this state
/// is named; wording that lets a reader conclude "may publish here" is a defect (prikk RFC 138 §7.3).
///
/// Through prikk 0.33, nothing exposed a way to check adoption ahead of attempting the gated
/// operation itself (RFC 016 F3): `prikk trust maintainer` offered only `add`/`remove`, and `verify`'s
/// `sealed-block <id>: <key_id>` line is historical signer attribution — a since-revoked key still
/// prints — not current policy, and does not exist before a repository's first seal, which is exactly
/// when this question is asked. **No increment may ever resolve `Unknown` from `verify` output**
/// (RFC 016's own named trap); that remains true regardless of what follows.
///
/// **prikk 0.34 shipped the surface** (upstream RFC 138): `prikk trust maintainer list` and
/// `check --key-id`, both with `--format json`, `check` exiting `0` whichever way the answer comes out.
/// Two things follow, and they are easy to conflate:
///
/// - **`Ready` became *constructible* at ≥ 0.34 — it is not yet *constructed*.** stikk reads neither
///   command: RFC 021 raised the validated ceiling to 0.38 and deliberately built no seam method for
///   this (its Decision 5). So `stikk-prikk::env` still returns only `NotReady`/`Unknown`, and the
///   `[MNT]` badge still reads `?` on every version. Anything claiming otherwise is describing prikk's
///   capability, not stikk's behaviour.
/// - **`Unknown` is permanent, not transitional.** stikk's floor is prikk 0.28 (`ASM-2`), so every
///   session on 0.28–0.33 has no way to answer the question no matter what stikk builds. This type
///   stays three-valued for good; the increment that reads 0.34's surface makes the answer
///   *version-conditional*, never unconditional.
///
/// This is the gate for all **eight** of prikk's `GatedOperation` variants — `Seal`, `Merge`,
/// `SyncBuild`, `SyncSeal`, `SyncAdoptTag`, `TagCreate`, `BranchCreate`, `BranchClose` — not seal's
/// alone; seal is only stikk's first consumer of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintainerReadiness {
    /// Key material present **and** adopted in the repository's trust policy. **Still unconstructed
    /// today**, though no longer unconstructible in principle — see this type's own doc.
    /// `prikk trust maintainer check` shipped in 0.34 (upstream RFC 138), so the answer now exists on
    /// ≥ 0.34; stikk does not read it yet, so `stikk-prikk::env` continues to produce only
    /// `NotReady`/`Unknown`. It remains documented the way RFC 017 documented
    /// `StikkError::IntegrityFinding`: present because it is the shape of the answer, and now also
    /// because the answer itself exists upstream and an increment will come to fetch it.
    Ready,
    /// Key material absent — no `PRIKK_MAINTAINER_KEY_ID`/`_SEED` pair in the environment.
    NotReady,
    /// Key material present; adoption is **unverifiable by stikk today**, and permanently
    /// unverifiable on prikk 0.28–0.33 whatever stikk builds (RFC 016 F3; RFC 021 §5). This
    /// is what `stikk-prikk::env` returns whenever both variables are set — never a caveat layered on
    /// a boolean, because `Unknown` must never render as a pass (`C-T2c′`, the same rule this
    /// project's design already applies to `FR-035`'s three-valued author-signature outcome).
    Unknown,
}

/// Whether each signing role's key material is available to the current session, plus whether the
/// session is in read-only mode.
///
/// This carries no key material — only presence flags (and, for MAINTAINER, the further-unverifiable
/// adoption question — see [`MaintainerReadiness`]). It is the input to [`Capability::derive`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Readiness {
    /// True when an AUTHOR key id and seed are both present in the environment (presence only — the
    /// seed value is never read; see `stikk-prikk::env`).
    pub author_ready: bool,
    /// Whether a MAINTAINER key is ready to sign — see [`MaintainerReadiness`] for why this is
    /// three-valued rather than a presence-only bool the way `author_ready` is.
    pub maintainer_readiness: MaintainerReadiness,
    /// True when the session is in read-only mode (a global override, or the default when no signing
    /// readiness is present). When set, no capability above [`Capability::Viewer`] is granted.
    pub read_only: bool,
}

impl Readiness {
    /// A session with no signing readiness and no read-only override — the Viewer default.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            author_ready: false,
            maintainer_readiness: MaintainerReadiness::NotReady,
            read_only: false,
        }
    }

    /// True when a human at the machine may run recovery actions (doctor repair, lock clearing,
    /// compaction) under explicit confirmation (design AC-04; RFC 012 F-a).
    ///
    /// Lives here, not on [`Capability`], because the one fact that decides it — `read_only` — is
    /// exactly what [`Capability::derive`] discards on the way to a [`Capability`]: by the time a
    /// capability exists, a read-only session and a no-keys session are indistinguishable, so
    /// `may_operate` could not be implemented correctly as a `Capability` method. `AC-04`'s
    /// "orthogonal to the [mutating axis]" describes how Operator is *derived* — any human at the
    /// machine, not a signing role — never an exemption from the global read-only switch: `FR-121`
    /// governs, and a read-only mode that still permits clearing another writer's lock would itself be
    /// the "confident-but-wrong picture" (`T-T4`) this project refuses. Each recovery action still
    /// carries its own typed confirmation (`FR-102`) regardless of this check.
    #[must_use]
    pub const fn may_operate(self) -> bool {
        !self.read_only
    }
}

/// What a session may do, derived from [`Readiness`] (design AC-01…04).
///
/// The levels are cumulative for the mutating axis (`Maintainer` implies `Author` implies `Viewer`).
/// `Operator` is orthogonal — recovery actions available to any human at the machine under explicit
/// confirmation — and is represented as a separate query on [`Readiness`] itself
/// ([`Readiness::may_operate`]) rather than a point on this ladder or a method here: `derive` below
/// discards `read_only` on the way to a `Capability`, so the one fact `may_operate` needs does not
/// survive to exist as a method on this type (RFC 012 F-a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    /// Every read surface. The default when signing readiness is absent or read-only mode is on.
    Viewer,
    /// Viewer plus queue-affecting operations: commit, rollback draft.
    Author,
    /// Author plus history-publishing: seal, merge execution, ref/tag publication, trust changes.
    Maintainer,
}

impl Capability {
    /// Derive the mutating-axis capability from readiness. Read-only mode collapses everything to
    /// [`Capability::Viewer`] regardless of key presence (design NFR-S01).
    ///
    /// Grants `Maintainer` on [`MaintainerReadiness::Ready`] **or** [`MaintainerReadiness::Unknown`]
    /// (RFC 016 Q1): the affordance is still offered, since hiding seal from someone whose key *is*
    /// adopted would be its own confident-but-wrong picture (`C-T4d`). What `Unknown` changes is what
    /// stikk *claims* about the outcome, not what it *offers* — the badge and the ceremony's own copy
    /// carry that distinction; this method does not.
    #[must_use]
    pub const fn derive(readiness: Readiness) -> Self {
        if readiness.read_only {
            return Self::Viewer;
        }
        match readiness.maintainer_readiness {
            MaintainerReadiness::Ready | MaintainerReadiness::Unknown => Self::Maintainer,
            MaintainerReadiness::NotReady => {
                if readiness.author_ready {
                    Self::Author
                } else {
                    Self::Viewer
                }
            }
        }
    }

    /// True when this capability permits queue-affecting operations (commit, rollback draft).
    #[must_use]
    pub const fn may_author(self) -> bool {
        matches!(self, Self::Author | Self::Maintainer)
    }

    /// True when this capability permits history-publishing operations (seal, merge, publication,
    /// trust changes).
    #[must_use]
    pub const fn may_publish(self) -> bool {
        matches!(self, Self::Maintainer)
    }

    /// The stable machine-readable name (design CL-07).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Author => "author",
            Self::Maintainer => "maintainer",
        }
    }
}

#[cfg(test)]
mod tests;

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

/// How prikk's own key-status report describes the relationship between the key that would sign and
/// what the repository already knows (prikk ≥ 0.41's `key-status-v1` `binding` field).
///
/// **stikk mirrors prikk's vocabulary rather than inventing one** (RFC 026 Decision 3). A collapsed
/// vocabulary plus a paragraph explaining the collapse is how stikk ended up, until 0.6.0 removed it,
/// with a `MaintainerReadiness` type that could not express `not-adopted` — the state RFC 025 wanted a
/// fourth variant for, which prikk had a name for all along.
///
/// Measured against a real prikk 0.41.0, one repository per state (RFC 026 Handoff B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// The key that would sign is the key this repository already records for that id. The only state
    /// that earns a plain statement on a confirmation card.
    Matches,
    /// No signature by this id exists yet, so nothing is recorded to disagree with. **Arms**: prikk
    /// accepts a first signature and binds the id then. This is the default state of every freshly
    /// created repository, so it is the first commit a new user makes.
    Unrecorded,
    /// MAINTAINER only: the key is usable but the repository's trust policy has not adopted it, and
    /// prikk will refuse the seal. **Withholds.**
    NotAdopted,
    /// The key that would sign is *not* the key recorded for this id. prikk refuses at signing time,
    /// so offering the action would be offering a failure. **Withholds**, and the card says which two
    /// things disagree.
    Mismatch,
    /// prikk reported `null`: there was no usable seed to bind, or no repository to ask. A real state,
    /// not a parse failure.
    Absent,
}

/// Whether one signing role can sign, as far as this session can tell.
///
/// **Two unknowns live here and they must never share a variant** (RFC 026 §3). [`Self::Unknown`]
/// grants the capability with a caveat rendered; [`Self::Unverifiable`] withholds it and says so. A
/// single variant that sometimes did each is the shape `C-T2c′` exists to forbid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleReadiness {
    /// No key material this session can use. At ≤ 0.39 the variables are absent; at ≥ 0.41 prikk said
    /// `usable: false`, and its own `reason` travels separately for display (`ER-02`).
    NotReady,
    /// Key material is present, and whether it binds is **unanswerable on this prikk** — the ≤ 0.39
    /// band, where `binding` does not exist. **Grants**, with the caveat rendered: RFC 016's rule is
    /// to offer the action and let prikk refuse, never to render the caveat as a pass.
    Unknown,
    /// **prikk 0.40 exactly**: stikk cannot see whether there is key material at all. 0.40 moved seeds
    /// to a key directory and shipped no way to ask about them, so presence-in-the-environment stopped
    /// being an answer and nothing replaced it until 0.41. **Withholds**, and the UI says *unknown*
    /// rather than *not ready* — Q1(b), ruled: honest and actionable beats confident and wrong.
    Unverifiable,
    /// prikk answered (≥ 0.41).
    Known(Binding),
}

impl RoleReadiness {
    /// Whether this state arms the role's operations.
    ///
    /// The one place the grant/withhold split is decided, so a caller cannot re-derive it differently.
    #[must_use]
    pub const fn arms(self) -> bool {
        match self {
            Self::NotReady | Self::Unverifiable => false,
            Self::Unknown => true,
            Self::Known(binding) => matches!(binding, Binding::Matches | Binding::Unrecorded),
        }
    }
}

/// Whether each signing role's key material is available to the current session, plus whether the
/// session is in read-only mode.
///
/// This carries no key material — only presence flags (and, for MAINTAINER, the further-unverifiable
/// adoption question — see [`RoleReadiness`]). It is the input to [`Capability::derive`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Readiness {
    /// Whether the AUTHOR role can sign, as far as this session can tell.
    ///
    /// **Was `author_ready: bool`** until RFC 026. That asymmetry with MAINTAINER was right when AUTHOR
    /// had nothing to be unsure about; prikk ≥ 0.41 answers `binding` for **both** roles, and
    /// `mismatch` applies to both, so both get the same vocabulary.
    pub author: RoleReadiness,
    /// Whether the MAINTAINER role can sign, as far as this session can tell.
    pub maintainer: RoleReadiness,
    /// True when the session is in read-only mode (a global override, or the default when no signing
    /// readiness is present). When set, no capability above [`Capability::Viewer`] is granted.
    pub read_only: bool,
}

impl Readiness {
    /// A session with no signing readiness and no read-only override — the Viewer default.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            author: RoleReadiness::NotReady,
            maintainer: RoleReadiness::NotReady,
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
    /// **The one fold, and the only place the grant/withhold decision is made** — through
    /// [`RoleReadiness::arms`], so no gate can grow its own rule (RFC 026 §3).
    ///
    /// `Unknown` grants (RFC 016 Q1): the affordance is still offered, since hiding seal from someone
    /// whose key *is* adopted would be its own confident-but-wrong picture (`C-T4d`). What `Unknown`
    /// changes is what stikk *claims* about the outcome, not what it *offers*. `Unverifiable`,
    /// `Mismatch` and `NotAdopted` withhold, for the opposite reason: prikk will refuse, so offering
    /// the action would be offering a failure.
    #[must_use]
    pub const fn derive(readiness: Readiness) -> Self {
        if readiness.read_only {
            return Self::Viewer;
        }
        if readiness.maintainer.arms() {
            Self::Maintainer
        } else if readiness.author.arms() {
            Self::Author
        } else {
            Self::Viewer
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

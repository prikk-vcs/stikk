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

/// Whether each signing role's key material is available to the current session, plus whether the
/// session is in read-only mode.
///
/// This carries no key material — only presence flags. It is the input to [`Capability::derive`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Readiness {
    /// True when an AUTHOR key id and seed are both present in the environment (presence only — the
    /// seed value is never read; see `stikk-prikk::env`).
    pub author_ready: bool,
    /// True when a MAINTAINER key id and seed are both present in the environment (presence only).
    pub maintainer_ready: bool,
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
            maintainer_ready: false,
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
    #[must_use]
    pub const fn derive(readiness: Readiness) -> Self {
        if readiness.read_only {
            return Self::Viewer;
        }
        if readiness.maintainer_ready {
            Self::Maintainer
        } else if readiness.author_ready {
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

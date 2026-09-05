//! The single stikk error type and its seven presentation classes.
//!
//! Design `stikk-04` ER-01/ER-02, threat model `stikk-03` OP-03/CT-04. Two lessons from the 2026
//! prikk audit are applied here to stikk's own error type: it implements [`std::error::Error::source`]
//! so causes are inspectable, and it is `#[non_exhaustive]` so adding a class is not a breaking
//! change. The load-bearing rule is [`StikkError::Refusal`] and friends **preserve prikk's message
//! verbatim** — no layer above rewrites it (NFR-I03); a localized gloss is added beside it, never in
//! place of it.

use std::fmt;

/// Convenience alias for a fallible stikk operation.
pub type Result<T> = std::result::Result<T, StikkError>;

/// Every error stikk surfaces, tagged with the class that decides how it is presented (design
/// `stikk-04` CT-04). The class is the whole point: a refusal opens the explanation overlay, a lock
/// conflict is a banner, a not-ready is inline guidance, and so on — one mapping, both frontends.
///
/// `#[non_exhaustive]`: adding a class later is not a breaking change to downstream matchers.
#[derive(Debug)]
#[non_exhaustive]
pub enum StikkError {
    /// prikk semantically refused an operation (a non-confluent merge, a TOFU key conflict, a
    /// path/ref validation refusal). `message` is prikk's own wording, preserved verbatim.
    Refusal {
        /// prikk's verbatim refusal message. Never rewritten by a layer above.
        message: String,
    },
    /// Another writer holds a lock, or a CAS precondition failed. Presented as "another writer is
    /// active", never as corruption (design FR-106). prikk distinguishes a genuine lock from a
    /// ref-CAS mismatch; both arrive here and the message carries which.
    LockConflict {
        /// prikk's verbatim conflict message.
        message: String,
    },
    /// A signing or trust prerequisite is absent — no AUTHOR/MAINTAINER readiness, or the maintainer
    /// key is not adopted. Presented as inline guidance toward the trust/keys surface (FR-104).
    NotReady {
        /// What is missing, in stikk's own words (this class is stikk-originated, not prikk's).
        detail: String,
    },
    /// A verify/doctor finding — repository-content diagnostics. Routed into the verify/doctor views,
    /// never a popup (design OP-03).
    IntegrityFinding {
        /// prikk's verbatim finding text.
        message: String,
    },
    /// An input ceiling was (or would be) exceeded — a bundle/exchange/summary limit. Surfaced in the
    /// pre-execution confirmation, not after failure (threat model C-D2a).
    Limits {
        /// prikk's verbatim limit message.
        message: String,
    },
    /// An environment problem: I/O, permissions, a missing or wrong `prikk` binary, version skew, or
    /// an unparseable prikk output shape. Carries a stikk-authored description plus any underlying
    /// cause via [`std::error::Error::source`].
    Environment {
        /// What went wrong, in stikk's own words.
        detail: String,
        /// The underlying cause, if any (e.g. a `std::io::Error`), preserved for `source()`.
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    /// A stikk-internal fault (a bug). The repository was not touched — only the seam writes, and only
    /// on an explicit confirmed mutation — so a fault here can never be a repository write (design
    /// ER-04). Session state is already flushed; the user may continue read-only.
    Internal {
        /// A description of the internal invariant that was violated.
        detail: String,
    },
    /// The repository changed between a preview and its confirmation, or between confirmation and
    /// execution (design `OPL-02`/`CT-05`; RFC 013 decision 3). **`prikk` did not refuse this — stikk
    /// did**, on comparing the change token stamped at preview time against a freshly-read one. Never
    /// `Refusal` (that would put stikk's own words in prikk's voice) and never `LockConflict` (nothing
    /// is locked). The user's next action is to look again: `present()` routes this to a re-preview
    /// prompt, and no next-step it offers may re-run the execution (`NFR-S04`).
    Stale {
        /// The operation whose preview or confirmation no longer matches the repository's current
        /// state — stikk's own short name for it (e.g. `"commit"`), never prikk's words.
        operation: String,
    },
    /// The confirmation evidence supplied did not satisfy the tier's requirement: a missing explicit
    /// yes, or a typed name that does not exactly match the confirmation summary's target (design
    /// `FR-121`; RFC 013 §4). Distinct from [`StikkError::Stale`] (nothing about the repository moved)
    /// and from [`StikkError::NotReady`] (capability and read-only mode were both sufficient — the
    /// *evidence itself* was not). Belongs inside the confirmation surface that asked for the evidence,
    /// never a separate popup (`Presentation::InConfirmation`) — the user is still mid-confirmation, not
    /// facing a new failure.
    Declined {
        /// What was required and what was supplied, in stikk's own words.
        detail: String,
    },
    /// A commit refused because the active WAL's queue targets a different ref than the one requested
    /// (design RFC 014 F2/decision 2). **Never [`StikkError::LockConflict`]**, even though prikk words
    /// it as one (`"lock conflict: active WAL is owned by …; requested ref …"`) — nothing is locked, no
    /// other writer is active, and `LockConflict` will grow a jump to a lock inspector (`FR-102`) that
    /// would show this condition no lock to find (RFC 012 F-b's overloaded-class shape, again). stikk
    /// prevents this client-side before arming a commit (`Orientation::queued_target`); this class
    /// exists for the race where the queue moves between preview and execute. `message` is prikk's own
    /// wording, preserved verbatim (`ER-02`) — the same discipline as [`StikkError::Refusal`].
    CrossRef {
        /// prikk's verbatim cross-ref refusal message.
        message: String,
    },
}

impl StikkError {
    /// Construct an [`StikkError::Environment`] from a description and an underlying cause.
    #[must_use]
    pub fn environment(
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Environment {
            detail: detail.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Construct an [`StikkError::Environment`] with no underlying cause.
    #[must_use]
    pub fn environment_msg(detail: impl Into<String>) -> Self {
        Self::Environment {
            detail: detail.into(),
            source: None,
        }
    }

    /// The stable machine-readable class name, for the report format (design CL-07) and for tests.
    #[must_use]
    pub const fn class(&self) -> &'static str {
        match self {
            Self::Refusal { .. } => "refusal",
            Self::LockConflict { .. } => "lock-conflict",
            Self::NotReady { .. } => "not-ready",
            Self::IntegrityFinding { .. } => "integrity-finding",
            Self::Limits { .. } => "limits",
            Self::Environment { .. } => "environment",
            Self::Internal { .. } => "stikk-internal",
            Self::Stale { .. } => "stale",
            Self::Declined { .. } => "declined",
            Self::CrossRef { .. } => "cross-ref",
        }
    }

    /// True when this class must never be auto-retried (design NFR-S04): a refusal, a lock conflict, a
    /// not-ready condition, a stale precondition, declined evidence, or a cross-ref conflict is the
    /// user's to resolve, never stikk's to retry silently. `Stale` in particular must never be retried
    /// as-is — its whole point is that retrying the same execution is exactly the prohibited thing (RFC
    /// 013 decision 3); the user's only correct next step is a fresh preview.
    #[must_use]
    pub const fn is_user_resolved(&self) -> bool {
        matches!(
            self,
            Self::Refusal { .. }
                | Self::LockConflict { .. }
                | Self::NotReady { .. }
                | Self::Stale { .. }
                | Self::Declined { .. }
                | Self::CrossRef { .. }
        )
    }
}

impl fmt::Display for StikkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refusal { message }
            | Self::LockConflict { message }
            | Self::IntegrityFinding { message }
            | Self::Limits { message }
            | Self::CrossRef { message } => write!(f, "{}: {message}", self.class()),
            Self::NotReady { detail } | Self::Internal { detail } | Self::Declined { detail } => {
                write!(f, "{}: {detail}", self.class())
            }
            Self::Environment { detail, .. } => write!(f, "environment: {detail}"),
            Self::Stale { operation } => {
                write!(
                    f,
                    "stale: {operation}'s preview no longer matches the repository"
                )
            }
        }
    }
}

impl std::error::Error for StikkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Environment {
                source: Some(cause),
                ..
            } => Some(cause.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;

//! Signing-key readiness, read from the environment **as presence only, never as values**.
//!
//! This module is the single most security-sensitive code in stikk, and it is deliberately tiny. The
//! catastrophic front-end failure would be a signing seed reaching a config file, a log, an export,
//! or the screen (threat model T-I1). stikk forecloses that structurally: it never materializes a
//! `PRIKK_*_SEED` **value** into an inspectable string — only whether the variable is set — and
//! prikk, not stikk, reads seeds when it signs (design SEAM-06, data model LC-13).
//!
//! The readiness computation is written against an injected presence lookup
//! (`read_readiness_with`) so it is tested hermetically, without mutating the process environment.
//! The public
//! [`read_readiness`] supplies the real lookup, which reads presence via `var_os(...).is_some()` and
//! drops the value immediately. The invariant is guarded by a source-level test in `env/tests.rs`:
//! this module contains no value-materializing call (`env::var(`, `into_string`, `to_string_lossy`,
//! …). The one intentional value comparison is on stikk's own non-secret `STIKK_READ_ONLY` flag.

use stikk_model::{Readiness, RoleReadiness};

/// The environment variable names prikk reads for signing. stikk reads only their **presence**.
const AUTHOR_KEY_ID: &str = "PRIKK_AUTHOR_KEY_ID";
const AUTHOR_SEED: &str = "PRIKK_AUTHOR_SEED";
const MAINTAINER_KEY_ID: &str = "PRIKK_MAINTAINER_KEY_ID";
const MAINTAINER_SEED: &str = "PRIKK_MAINTAINER_SEED";

/// stikk's own read-only override (design CF-04). When set to `1`, the session is read-only and no
/// capability above Viewer is granted, regardless of key presence.
const READ_ONLY: &str = "STIKK_READ_ONLY";

/// Compute readiness from an injected presence lookup. This is the whole logic; the public entry
/// point supplies the real environment lookup. Keeping it injectable means the presence rules are
/// tested without touching process-global state, and the security invariant (no value read) is a
/// property of the *real* lookup, checked separately.
///
/// **This is the ≤ 0.39 band only** (RFC 026 §4). On prikk ≥ 0.40 the seed variables are no longer what
/// prikk reads, so presence here answers nothing about signing; `stikk_prikk::Prikk::readiness` picks
/// the band and this function is one of its three answers, not the answer.
///
/// Neither role can be better than `Unknown` on this band: `binding` does not exist below 0.41, and
/// trust-policy adoption was unverifiable below 0.34 and unread by stikk below 0.41 (RFC 016 F3). That
/// is correct rather than a shortfall — see [`stikk_model::RoleReadiness`].
fn read_readiness_with(is_set: impl Fn(&str) -> bool, read_only: bool) -> Readiness {
    // Both roles now use the same vocabulary (RFC 026 §3). On this band `binding` does not exist, so
    // "present" can only ever mean `Unknown` — the grant-with-a-caveat state, which is exactly what
    // this band has always meant for MAINTAINER and now says out loud for AUTHOR too.
    let present = |key_id: &str, seed: &str| {
        if is_set(key_id) && is_set(seed) {
            RoleReadiness::Unknown
        } else {
            RoleReadiness::NotReady
        }
    };
    Readiness {
        author: present(AUTHOR_KEY_ID, AUTHOR_SEED),
        maintainer: present(MAINTAINER_KEY_ID, MAINTAINER_SEED),
        read_only,
    }
}

/// Whether a `PRIKK_*_SEED` variable is set on a prikk that has stopped reading it (RFC 026 §4).
///
/// **This is what `legacy_variable_set` was going to tell us, and that field does not exist** — prikk
/// 0.41's `key-status-v1` has nine fields and none of them is it, measured rather than read from
/// letter 007. stikk can establish the fact itself: the variable being *set* is presence, which is the
/// only thing this module has ever read, so the detection needs no new capability and materializes no
/// value.
///
/// At ≥ 0.40 a set `PRIKK_<ROLE>_SEED` is stale: 0.40 refuses it outright and 0.41 ignores it in
/// favour of the key directory. Either way it is **not what will sign**, and a user who exported it
/// years ago has every reason to think it is.
#[must_use]
pub fn stale_seed_variables() -> StaleSeedVariables {
    stale_seed_variables_with(is_set)
}

/// The injectable half of [`stale_seed_variables`], tested without touching process-global state.
fn stale_seed_variables_with(is_set: impl Fn(&str) -> bool) -> StaleSeedVariables {
    StaleSeedVariables {
        author: is_set(AUTHOR_SEED),
        maintainer: is_set(MAINTAINER_SEED),
    }
}

/// Which `PRIKK_*_SEED` variables are set. Presence only — no value is read, here or anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StaleSeedVariables {
    /// `PRIKK_AUTHOR_SEED` is set.
    pub author: bool,
    /// `PRIKK_MAINTAINER_SEED` is set.
    pub maintainer: bool,
}

impl StaleSeedVariables {
    /// True when either is set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.author || self.maintainer
    }
}

/// Presence-only environment probe: is this variable set? The returned `bool` is all that escapes;
/// the value is never bound, inspected, converted, or logged. This is the only way this module reads
/// a signing-key variable.
fn is_set(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

/// Compute the current session's signing readiness from environment presence.
///
/// A role is "ready" when both its key-id and its seed variables are present. stikk does not verify
/// that the seed is well-formed — that is prikk's job, which fails closed on a malformed seed; stikk
/// only needs to know whether attempting a signing operation is worth offering (design FR-104).
#[must_use]
pub fn read_readiness(read_only: bool) -> Readiness {
    read_readiness_with(is_set, read_only)
}

/// Whether the `STIKK_READ_ONLY=1` override is in effect (design CF-04). Any value other than `1`
/// (including unset) leaves it off; the override can only *force* read-only, never lift it. This is
/// the one intentional value comparison in this module, on stikk's own non-secret flag — it compares
/// the `OsString` directly and never materializes it into an inspectable string.
#[must_use]
pub fn read_only_override() -> bool {
    matches!(std::env::var_os(READ_ONLY), Some(value) if value == "1")
}

#[cfg(test)]
mod tests;

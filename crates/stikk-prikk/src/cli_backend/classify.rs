//! Classify a non-zero prikk exit into the error taxonomy (design UD-05, RFC 007).
//!
//! prikk collapses distinct failures onto exit 1 (audit UD-05), so stikk classifies by the message
//! text **plus the request category** — the same fragility class as the human-output parsers (UD-02),
//! kept in the same place, version-gated and fixture-pinned. Two rules are load-bearing:
//!
//! - **The verbatim message is always preserved** (NFR-I03): every class carries prikk's own wording;
//!   the operation layer adds a localized gloss beside it, never in place of it.
//! - **An unrecognized message degrades to [`StikkError::Refusal`]** — the safe default (RR-5): it
//!   shows the message verbatim and never triggers a retry (`refusal` is user-resolved, NFR-S04). A
//!   guess at a *specific* wrong class would be worse than a generic-but-honest refusal.

use stikk_model::{RequestCategory, StikkError};

/// Map a failed prikk invocation's output to a [`StikkError`]. `category` disambiguates where the
/// message text alone is ambiguous (design UD-05: "classify by message + context").
#[must_use]
pub fn classify(stdout: &str, stderr: &str, category: RequestCategory) -> StikkError {
    let message = pick_message(stdout, stderr);
    let lowered = message.to_ascii_lowercase();

    // Environment first: a wrong/missing repository or a launch/IO problem is not a semantic refusal.
    if is_environment(&lowered) {
        return StikkError::environment_msg(message);
    }
    // A lock or CAS conflict — "another writer is active" (FR-106), never corruption.
    if is_lock_conflict(&lowered) {
        return StikkError::LockConflict { message };
    }
    // A signing/trust prerequisite absent — inline guidance toward Trust & Keys (FR-104). This is
    // most meaningful for the mutating categories, but the message wording is what decides.
    if is_not_ready(&lowered) {
        return StikkError::NotReady { detail: message };
    }
    // An integrity/verify finding, when the request was an integrity read (routed into Verify/Doctor).
    if category == RequestCategory::Integrity && is_integrity_finding(&lowered) {
        return StikkError::IntegrityFinding { message };
    }
    // Default: a semantic refusal, verbatim preserved (NFR-I03) — the safe degradation (RR-5).
    StikkError::Refusal { message }
}

/// prikk writes errors to stderr; prefer it, else fall back to stdout. Shared with the exit-2
/// usage-error path in `cli_backend.rs` (RFC 009 F6), which must not route through [`classify`].
pub(super) fn pick_message(stdout: &str, stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        stdout.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_environment(lowered: &str) -> bool {
    lowered.contains("not a prikk repository")
        || lowered.contains("no such file")
        || lowered.contains("permission denied")
        || (lowered.contains("could not") && lowered.contains("prikk"))
        || lowered.contains("unsupported prikk version")
        || lowered.contains("retired repository format")
}

fn is_lock_conflict(lowered: &str) -> bool {
    (lowered.contains("lock")
        && (lowered.contains("held")
            || lowered.contains("conflict")
            || lowered.contains("already")))
        || lowered.contains("another writer")
        || lowered.contains("ref-state precondition")
        || lowered.contains("cas mismatch")
}

fn is_not_ready(lowered: &str) -> bool {
    lowered.contains("not ready")
        || lowered.contains("no signing key")
        || lowered.contains("key not adopted")
        || (lowered.contains("maintainer") && lowered.contains("required"))
}

fn is_integrity_finding(lowered: &str) -> bool {
    lowered.contains("finding") || lowered.contains("verify") || lowered.contains("doctor")
}

#[cfg(test)]
mod tests;

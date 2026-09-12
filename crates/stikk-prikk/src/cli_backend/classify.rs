//! Classify a non-zero prikk exit into the error taxonomy (design UD-05, RFC 007; RFC 017; RFC 016).
//!
//! prikk collapses distinct failures onto exit 1 (audit UD-05), so stikk classifies by the message
//! text — the same fragility class as the human-output parsers (UD-02), kept in the same place,
//! version-gated and fixture-pinned. Three rules are load-bearing:
//!
//! - **No arm survives without a captured message behind it** (RFC 017 decision 1). Every string
//!   below either matches something a real prikk 0.33.0 binary actually printed (provoked and quoted
//!   in `classify/tests.rs`'s provenance comments) or was read directly from prikk's source at that
//!   tag with a `file:line` citation — never from prikk's *prose* or documentation, which is exactly
//!   how five arms went five releases without ever matching anything (RFC 017 F2).
//! - **Match the semantic clause, never the class prefix.** prikk changed `lock conflict:` →
//!   `precondition not met:` on the very message RFC 014 classifies (0.33.0), and nothing broke,
//!   because the match was on `active wal is owned by`, not on the word "lock". Seal's own cross-ref
//!   refusal is the sharper case (RFC 016 F4/§6): at 0.33.0 it carries **no class prefix at all**, and
//!   its trailing clause (`"requested seal ref is …"`) differs from commit's (`"requested ref …"`) —
//!   two different prefixes and two different trailing phrasings for the same repository fact, one
//!   release apart. `is_cross_ref_conflict` matches only the clause both share. Every arm here follows
//!   this rule now; `PrikkError` is `#[non_exhaustive]` and its class words are prikk's to change.
//! - **An unrecognized message degrades to [`StikkError::Refusal`]** — the safe default (RR-5): it
//!   shows the message verbatim and never triggers a retry (`refusal` is user-resolved, NFR-S04). A
//!   guess at a *specific* wrong class would be worse than a generic-but-honest refusal — this is the
//!   rule that absorbed RFC 017's five dead arms for four years with no user ever seeing anything
//!   untrue because of them.

use stikk_model::{RequestCategory, StikkError};

/// Map a failed prikk invocation's output to a [`StikkError`]. `category` is currently unused: the one
/// arm that read it (`is_integrity_finding`, gated on [`RequestCategory::Integrity`]) had no captured
/// evidence behind any of its strings (RFC 017 F2) and was removed rather than kept on a guess — see
/// `classify/tests.rs` for the degradation test. The parameter is kept, not deleted: `UD-05`'s own
/// design is "classify by message *and* context", every other seam call still supplies a category for
/// exactly that reason, and a future arm grounded on real `verify` output (once `FR-100` builds a view
/// for it to route into) will need it again.
#[must_use]
pub fn classify(stdout: &str, stderr: &str, _category: RequestCategory) -> StikkError {
    let message = pick_message(stdout, stderr);
    let lowered = message.to_ascii_lowercase();

    // Environment first: a wrong/missing repository, a launch/IO problem, or a repository format
    // prikk no longer reads at all is not a semantic refusal.
    if is_environment(&lowered) {
        return StikkError::environment_msg(message);
    }
    // A commit or seal whose focused ref is not the active WAL's queue target (RFC 014 F2; RFC 016
    // F4/decision 5). Checked **before** the lock-conflict arm below: at prikk ≤ 0.32 commit's own
    // class word was "lock conflict:", and at 0.33.0 it changed to "precondition not met:" — the match
    // has never depended on the class word, only on "active wal is owned by", which is exactly why RFC
    // 017 F1 found nothing broke when commit's class word moved. Seal's own wording drifts further
    // still: at 0.33.0 it carries **no class prefix at all**, and its trailing clause is "requested
    // seal ref is {ref}" where commit says "requested ref {ref}" — two different trailing phrasings
    // under two different prefixes for the same repository fact, one release apart. Matching on the
    // clause both commands share, and nothing else, is what survives that (RFC 016 §6).
    if is_cross_ref_conflict(&lowered) {
        return StikkError::CrossRef { message };
    }
    // A genuine lock/CAS conflict — "another writer is active" (FR-106). Narrowed to the four captured
    // semantic clauses that are actually transient and actually helped by retrying (RFC 017 F4):
    // matching prikk's `lock conflict:` class prefix itself, as the arm did through 0.32, would also
    // catch six precondition messages that share the prefix but name no writer at all — the live defect
    // this increment removes (a false "another writer is active" gloss shown beside prikk's own "run
    // `prikk seal`" on the ordinary commit path).
    if is_lock_conflict(&lowered) {
        return StikkError::LockConflict { message };
    }
    // A signing/trust prerequisite absent, or a maintainer signer not trusted by policy — inline
    // guidance toward Trust & Keys (FR-104). Both are captured at 0.33.0; the trust-refusal clauses are
    // classifier-only for this increment (RFC 017 §6) — their Trust & Keys presentation is RFC 016's.
    if is_not_ready(&lowered) || is_trust_not_ready(&lowered) {
        return StikkError::NotReady { detail: message };
    }
    // Default: a semantic refusal, verbatim preserved (NFR-I03) — the safe degradation (RR-5). This is
    // where the six real preconditions RFC 017 F4 found land today: none has a view to route into yet
    // (no Queue, no Seal ceremony, no rollback flow), and an arm pointing nowhere is worse than none
    // (RFC 017 §3's rule for `is_integrity_finding`, applied the same way here). `present()` still gives
    // the full-queue case — the one case already live on the commit path — its own honest gloss, keyed
    // on the message shape rather than a new class (RFC 017 §4).
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

/// Captured at 0.33.0: a missing/foreign directory (`i/o error: No such file or directory`, RFC 017 §2
/// row 1/2 — `not a prikk repository` is confirmed absent from every prikk output, never emitted, and
/// this is the arm that actually catches a foreign directory, by accident until now, RFC 017 F3), a
/// permission fault (`i/o error: Permission denied`), and a retired repository format (read from
/// `layout.rs`, provoked live against a real format-2 repository built with a real prikk 0.19.0 binary:
/// `this repository uses format 2, which prikk no longer supports …`). Matched on `uses format` +
/// `no longer supports` — the stable clause — never the format number or the removal version, both of
/// which change (RFC 017 §5).
fn is_environment(lowered: &str) -> bool {
    lowered.contains("no such file")
        || lowered.contains("permission denied")
        || (lowered.contains("uses format") && lowered.contains("no longer supports"))
}

/// Matched on the one clause commit's and seal's cross-ref refusals share (RFC 016 §6) — never either
/// command's own trailing phrase (commit: `"requested ref …"`; seal: `"requested seal ref is …"`), and
/// never a class prefix (commit carries one at 0.33.0; seal carries none). Both wordings are captured
/// fixtures in `classify/tests.rs`, with a test proving each classifies `CrossRef`.
fn is_cross_ref_conflict(lowered: &str) -> bool {
    lowered.contains("active wal is owned by")
}

/// The four genuine lock/CAS conflicts among `PrikkError::LockConflict`'s ten 0.33.0 construction
/// sites (RFC 017 F4) — matched on each one's own semantic clause, never the shared `lock conflict:`
/// class prefix every one of the ten carries (six of which are preconditions, not locks). That
/// narrowing is what let the six preconditions survive prikk 0.35's reclassification of their prefix
/// to `precondition not met:` without stikk noticing, and it is now asserted against real binaries at
/// both ends of the range rather than only against fixtures (RFC 022 §4):
/// `the_full_queue_precondition_classifies_refusal_at_both_ends` and
/// `a_genuinely_held_lock_classifies_lock_conflict_at_both_ends` in `stikk-real-binary`.
///
/// **Each clause carries either provenance or a reachability note.** A citation without one is what
/// RFC 017 was about, one step removed: a reachability reason is a claim about prikk that can stop
/// being true, and when it does the arm becomes provokable and should be provoked. Re-check these at
/// each re-baseline the way a fixture's provenance is re-checked.
///
/// - **`lock already exists`** — captured, and now provoked live at 0.28 and 0.38 through
///   `CliBackend::commit` with a pre-placed `.prikk/active/<queue>/active.lock`
///   (`"{kind} lock already exists: {path}"`). This is the only one of the four with a stikk UI path,
///   and the case `FR-106`'s retry guidance is actually for.
/// - **`belongs to a different repository authority`** — source-read, prikk 0.38.0
///   `crates/prikk-store/src/lock.rs:54` (`ActiveLock::require_layout`). Unreachable through the CLI:
///   it compares a held `ActiveLock`'s bound mutation-root handle against the layout it is being used
///   with, and one prikk invocation binds both from one repository, so they always agree. It would
///   become reachable if prikk grew a path carrying a lock across repositories inside a single process
///   — an embedding host, or a multi-repository command. **stikk drives one repository per invocation
///   and has no way to construct that.**
/// - **`cas mismatch`** — source-read, prikk 0.38.0 `crates/prikk-store/src/refs.rs:467`
///   (`Refs::ensure_current_matches`). **prikk documents their own guard as unable to fire through the
///   publish path**, verbatim: *"this refusal is defence against a lock-discipline regression, not a
///   live CAS gate on the only path that reaches it today … By the time this function re-reads and
///   compares, the equality is already established; it cannot fail through that path. It stays because
///   it is exactly what would catch a future change that broke that locking discipline."* **stikk keeps
///   this arm for the same reason they keep the guard**: if that discipline ever regresses the message
///   reaches stikk, and an arm deleted for being unreachable would classify a real CAS conflict as a
///   bare refusal at exactly the moment it mattered.
/// - **`changed during planning`** — source-read, prikk 0.38.0
///   `crates/prikk-store/src/rollback_draft.rs:168`. Needs the target ref to move between
///   `rollback-draft` planning its inverse and applying it — that is, an in-flight rollback draft.
///   **stikk builds no rollback draft**: there is no Rollback flow, and `Prikk` exposes no method that
///   runs `rollback-draft`. It becomes provokable the day one lands, and this note should be revisited
///   with that increment rather than after it.
///
/// RFC 022's Q1 asked whether the three unprovoked arms should be deleted; the architect ruled they
/// stay, on the evidence that `present()` adds **no gloss** to a `LockConflict` — it renders prikk's
/// verbatim message in a banner — so an unreachable arm's worst case is a banner instead of a refusal
/// card, with nothing fabricated either way. That ruling is why the cost of keeping them is a note.
fn is_lock_conflict(lowered: &str) -> bool {
    lowered.contains("lock already exists")
        || lowered.contains("belongs to a different repository authority")
        || lowered.contains("cas mismatch")
        || lowered.contains("changed during planning")
}

/// Captured at 0.33.0: absent signing readiness (`"… is required: set PRIKK_…_KEY_ID (no signing key
/// configured)"`, identical in shape for AUTHOR and MAINTAINER — provoked both ways). The three other
/// strings this arm used to also match (`"not ready"`, `"key not adopted"`, and `"maintainer"` +
/// `"required"`, the last of which is redundant with this one — the real message already contains
/// "required" too) were never found anywhere in prikk's source (RFC 017 F2) and are deleted, not
/// narrowed.
fn is_not_ready(lowered: &str) -> bool {
    lowered.contains("no signing key")
}

/// A maintainer signer not trusted by policy (RFC 017 F5/§6) — both captured live at 0.33.0:
/// `"maintainer signer key id … is not trusted by policy"` and `"maintainer signer public key does not
/// match trusted key …"`. Both previously degraded to a bare `Refusal`, losing `FR-104`'s Trust & Keys
/// routing (`is_not_ready` matched neither: they contain `"maintainer"` but not `"required"`). This is
/// the classifier half only — the ceremony's presentation of a trust refusal is RFC 016's.
fn is_trust_not_ready(lowered: &str) -> bool {
    lowered.contains("is not trusted by policy") || lowered.contains("does not match trusted key")
}

#[cfg(test)]
mod tests;

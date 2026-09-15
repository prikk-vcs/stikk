//! The seal operation (design `FR-052`/`FL-06`; RFC 016) — the second operation that writes, and the
//! first that publishes history. There is no undo, in stikk or in prikk.
//!
//! Built on RFC 013's `preview() → PreviewToken → confirm() → ConfirmedToken → execute()` machinery,
//! unchanged — the same shape [`crate::commit`] follows. Two refusals prikk would otherwise give are
//! **prevented**, not merely classified (RFC 016 F4, decision 4): an empty-queue seal and a cross-ref
//! seal (the focused ref is not the active WAL's queue target) both make sealing **unavailable with a
//! reason** ([`SealPreviewOutcome::Blocked`], `C-T4d`) before any [`PreviewToken`] exists.
//!
//! **The no-audit consent step (RFC 016 §8, decision 3) is not this module's machinery to gate.** It
//! is a distinct, client-side act the frontend collects *after* this operation's own `confirm()`
//! evidence and *before* dispatching `execute()` — "not a line in the confirmation summary," per the
//! handoff. [`SEAL_CONSENT_COPY`] is this module's contribution to that step: the copy itself, so every
//! frontend shows identical words (frontend parity), tied to the ceremony's own irreversibility, never
//! to prikk's `--allow-no-audit` flag (RFC 016 F1, amended on prikk's reply: that flag is scaffolding
//! prikk may retire, and a claim whose truth depends on someone else's unscheduled decision is a claim
//! stikk should not make, `C-T4`).

use std::path::Path;

use stikk_model::{Capability, RequestCategory, Result};
use stikk_prikk::{Prikk, QueueReport, QueueTarget, QueuedMessage, SealResult};

use crate::confirm::{
    self, ConfirmationSummary, Evidence, FrozenPatches, Intent, Outcome, PreviewToken,
};

/// stikk's own short name for this operation — used verbatim in [`stikk_model::StikkError::Stale`]/
/// [`stikk_model::StikkError::Declined`] messages (never prikk's words) and as the palette/`Intent`
/// operation id.
pub const SEAL_OPERATION: &str = "seal";

/// The no-audit consent step's copy (RFC 016 §8, decision 3). States what is true of **the ceremony**
/// — no audit ran, sealing is permanent — and nothing about prikk's `--allow-no-audit` flag: prikk
/// answered that the flag is scaffolding, the milestone that replaces it is unscheduled, and they could
/// not say what replaces it because nobody has decided. Tying this copy to the flag would make it false
/// the day the flag changes; tying it to irreversibility survives that unchanged. `seal/tests.rs`
/// enforces this mechanically: a test asserts the string never names `--allow-no-audit` or otherwise
/// attributes the requirement to prikk.
pub const SEAL_CONSENT_COPY: &str = "No independent audit has run on the patches being sealed here. \
     This freezes them into permanent, MAINTAINER-signed history — once sealed, it cannot be undone, \
     in stikk or in prikk.";

/// What [`seal_preview`] found (RFC 016 decision 4). `Blocked` carries a reason in stikk's own words —
/// never routed through [`stikk_model::StikkError`] and `present()`, since neither an empty queue nor a
/// cross-ref target is a fault or a refusal: they are ordinary, expected preview outcomes, exactly the
/// way a disabled palette entry is not an error (`C-T4d`) — the same posture
/// [`crate::commit::CommitPreviewOutcome`] takes for commit's own two preventable refusals.
#[derive(Debug)]
pub enum SealPreviewOutcome {
    /// Sealing is unavailable, with the reason to show (disabled-with-reason, `C-T4d`) — no
    /// [`PreviewToken`] exists for this outcome; nothing was armed.
    Blocked(String),
    /// Sealing is available: the token to carry into confirmation. `token` is boxed only to keep this
    /// enum's variants close in size (`clippy::large_enum_variant`) — no meaning attaches to the
    /// indirection.
    Ready {
        /// Feed this into [`seal_confirm_and_execute`] once the user supplies evidence.
        token: Box<PreviewToken>,
    },
}

/// Build the seal preview for `reff` (design `FL-06`; RFC 016 §5/§6).
///
/// Reads the queue fresh, inside the change-token-gated `compute` step (RFC 013's ruling, the same one
/// [`crate::commit::commit_preview`] follows) — never a count already on screen — and names the patches it
/// would freeze at prikk ≥ 0.39 (RFC 028 decision 4).
/// Two conditions are checked before anything is armed (RFC 016 decision 4): the active WAL must hold
/// at least one queued patch, and its queue must target `reff`. Neither reaches
/// [`stikk_prikk::Prikk::seal`] itself — prevention, not classification.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn seal_preview(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<SealPreviewOutcome> {
    let intent = Intent {
        category: RequestCategory::Publication,
        operation: SEAL_OPERATION,
    };
    let reff_owned = reff.to_string();
    let (view, token) =
        confirm::preview(prikk, repo, intent, || compute(prikk, repo, &reff_owned))?;
    Ok(match view {
        SealReadView::Blocked(reason) => SealPreviewOutcome::Blocked(reason),
        SealReadView::Ready => SealPreviewOutcome::Ready {
            token: Box::new(token),
        },
    })
}

/// The two shapes [`compute`] can produce — never exposed outside this module; [`seal_preview`]
/// unwraps this into [`SealPreviewOutcome`], attaching the token only to the `Ready` case.
enum SealReadView {
    Blocked(String),
    Ready,
}

/// The `compute` closure `preview()` runs after stamping the change token (RFC 013 `OPL-02`).
fn compute(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
) -> Result<(SealReadView, ConfirmationSummary)> {
    // RFC 028 decision 4: the empty-queue block, the cross-ref block, the count and the list all come from
    // **one** queue read, so the count and the list describe the same moment. Below 0.39 the same read
    // answers from prose `status`, so the blocks are what they always were.
    let prikk_minor = prikk.handshake()?.version.minor;
    let queue = prikk.queue(repo)?;
    let (queued_patches, queued_target) = match &queue {
        QueueReport::Listed(listed) => (
            listed.count,
            match &listed.target {
                QueueTarget::Ref(target) => Some(target.as_str()),
                // Missing or malformed metadata, or no target: not blocked here — prikk decides, as it
                // does when prose gives a sentinel.
                QueueTarget::MissingMetadata
                | QueueTarget::MalformedMetadata
                | QueueTarget::NotReported => None,
            },
        ),
        QueueReport::Unreported { count, target } => (*count, target.as_deref()),
    };
    // Orientation is read for prikk's current branch alone (RFC 029's notice); a queue or branch that
    // moves between these two reads changes the change token `preview()` stamped, so the confirmation is
    // stale (RFC 030) rather than wrong.
    let orientation = prikk.orientation(repo)?;

    // RFC 016 decision 4/F4: prevent an empty-queue seal before arming anything.
    if queued_patches == 0 {
        return Ok((
            SealReadView::Blocked(
                "nothing is queued for this ref — there is nothing to seal".to_string(),
            ),
            placeholder_summary(),
        ));
    }
    // Prevent a cross-ref seal the same way commit prevents its own (RFC 014 decision 1 / RFC 016
    // decision 4) — both ref names are stikk's own authoritative sources, never parsed from a prikk
    // refusal (`C-T2b`).
    if let Some(target) = queued_target
        && target != reff
    {
        let reason = format!(
            "the active queue belongs to {target}, but you are focused on {reff} — choose {target} \
             to seal its queue"
        );
        return Ok((SealReadView::Blocked(reason), placeholder_summary()));
    }

    // RFC 016 §7: the confirmation's trust-refusal warning is driven by `RoleReadiness`, not
    // shown unconditionally — this is the one place in the seal flow that needs it, the same way
    // `orient()` reads it for the badge (RFC 016 §3).
    let report = prikk.readiness(repo)?;
    let (signing_key_id, claim) = crate::confirm::signing_key_claim(
        report.readiness.maintainer,
        &report.maintainer,
        stikk_prikk::key_id::maintainer_key_id(),
    );
    let summary = ConfirmationSummary {
        operation: "Seal the active WAL".to_string(),
        target_ids: vec![reff.to_string()],
        counts: vec![("patches", queued_patches)],
        capability: Capability::Maintainer,
        consequence: consequence(report.readiness.maintainer),
        target_name: None,
        // RFC 026 §5: the id comes from prikk where prikk has it. `key_id.rs` still answers on the
        // bands below 0.41, where `key status` does not exist.
        signing_key_id: signing_key_id.clone(),
        signing_key_claim: claim,
        // `C-S2`: computed from the key actually in effect, every time a confirmation is built.
        signing_key_is_published_example: report
            .maintainer
            .public_key
            .as_deref()
            .and_then(stikk_model::published_example)
            .is_some(),
        // Safeguard 3, from this preview's own Orientation read above.
        branch_notice: crate::confirm::branch_notice(reff, &orientation.current_branch),
        freezes: Some(frozen_patches(&queue, prikk_minor)),
    };
    Ok((SealReadView::Ready, summary))
}

/// What the confirmation names as frozen, from the same queue read as the count (Handoff B §3).
fn frozen_patches(queue: &QueueReport, prikk_minor: u32) -> FrozenPatches {
    match queue {
        QueueReport::Unreported { .. } => FrozenPatches::Unlisted(format!(
            "prikk 0.{prikk_minor} does not list queued patches."
        )),
        QueueReport::Listed(listed) => FrozenPatches::Listed {
            rows: listed
                .patches
                .iter()
                .map(|patch| {
                    let short: String = patch
                        .patch_id
                        .chars()
                        .take(crate::confirm::SHORT_ID_CHARS)
                        .collect();
                    // The Queue view's own message line — one function, so the two never disagree.
                    match crate::queue::message_line(&patch.message) {
                        Some(message) => format!("{short}  {message}"),
                        None => short,
                    }
                })
                .collect(),
            foot: listed
                .patches
                .iter()
                .any(|patch| patch.message == QueuedMessage::NotReported)
                .then(|| {
                    format!("prikk 0.{prikk_minor} does not report a queued patch's message.")
                }),
        },
    }
}

/// A [`ConfirmationSummary`] never shown — [`SealPreviewOutcome::Blocked`] discards it along with the
/// [`PreviewToken`] `preview()` mints for it, the same harmless-unused-token posture
/// [`crate::commit::placeholder_summary`] documents.
fn placeholder_summary() -> ConfirmationSummary {
    ConfirmationSummary {
        operation: String::new(),
        target_ids: Vec::new(),
        counts: Vec::new(),
        capability: Capability::Maintainer,
        consequence: String::new(),
        target_name: None,
        signing_key_id: None,
        signing_key_claim: crate::confirm::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
    }
}

/// What becomes permanent, in stikk's own words (`TU-09`) — including RFC 016 decision 2's "never
/// promises success," and, **only when adoption is genuinely unverifiable**, that a trust refusal is
/// possible and would be prikk's own decision (RFC 016 §7/§9).
///
/// **RFC 026 narrowed when that caveat is honest.** It belongs to `Unknown` — the ≤ 0.39 band, where
/// no supported prikk could answer. At ≥ 0.41 prikk *has* answered: `Matches` and `Unrecorded` are the
/// only states that reach this card at all (the others withhold the capability), and on those stikk
/// knows adoption is in place. Printing "a trust refusal is possible — stikk cannot verify" over an
/// answer stikk just received would be the confident-but-wrong picture pointing the other way.
fn consequence(maintainer: stikk_model::RoleReadiness) -> String {
    const BASE: &str = "Freezes the active WAL's queued patches into a new, MAINTAINER-signed block. \
         This does not promise success.";
    match maintainer {
        stikk_model::RoleReadiness::Unknown => format!(
            "{BASE} A trust refusal is possible here — stikk cannot verify key adoption on this \
             prikk, before or after this attempt."
        ),
        // Unreachable in practice — `capability_gate` refuses before a preview is built — but named
        // rather than caught by a wildcard, so a new state cannot acquire a silent default here.
        stikk_model::RoleReadiness::NotReady | stikk_model::RoleReadiness::Unverifiable => {
            BASE.to_string()
        }
        stikk_model::RoleReadiness::Known(_) => BASE.to_string(),
    }
}

/// Confirm and execute a seal in one step (design `OPL-01…05`; RFC 016 §5/§7) — the same one-round-trip
/// shape [`crate::commit::commit_confirm_and_execute`] follows. `readiness` is an explicit parameter for
/// the same reason it is there: the caller (the worker thread) reads it immediately before this call.
///
/// The no-audit consent step is **not** an argument here — it is collected by the frontend as its own
/// act, before this function is ever called (RFC 016 decision 3): by the time this runs, both of the
/// ceremony's two deliberate acts have already happened.
///
/// # Errors
/// [`stikk_model::StikkError::NotReady`] if read-only mode or MAINTAINER capability is insufficient, or
/// if prikk refuses the signer as untrusted (RFC 017 F5/F6); [`stikk_model::StikkError::Stale`] if the
/// repository changed since the preview or since confirmation; [`stikk_model::StikkError::Declined`] if
/// `evidence` does not satisfy tier 3; otherwise whatever [`stikk_prikk::Prikk::seal`] or a change-token
/// read raises — including [`stikk_model::StikkError::CrossRef`] for the RFC 016 F4 race.
pub fn seal_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    reff: &str,
) -> Result<Outcome<SealResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || prikk.seal(repo, reff))
}

#[cfg(test)]
mod tests;

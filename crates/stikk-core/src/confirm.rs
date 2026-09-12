//! Preview-first, structurally: the gate every mutation in 0.4.0 sits behind (design `FR-120`/`FR-121`,
//! `OPL-01…05`, `CT-05`, `TU-09`; RFC 013).
//!
//! Three functions, three types, one guarantee: **an execution that skipped the preview, or skipped
//! confirmation, is unrepresentable — not merely forbidden.** [`preview`] is the only producer of
//! [`PreviewToken`]; [`confirm`] is the only producer of [`ConfirmedToken`]; [`execute`] is the only
//! consumer of the latter, and it takes it **by value**, so it cannot be replayed. Neither token has a
//! public constructor and there is no `From`/`Into` between them.
//!
//! The [`stikk_model::ChangeToken`] (RFC 003) is checked **twice**: once in [`confirm`] (catches the
//! world moving while the user read the preview) and once at the top of [`execute`] (the real guard,
//! immediately before whatever the seam call will be). Either mismatch is
//! [`stikk_model::StikkError::Stale`] — never a retry (`NFR-S04`): the correct next step is a fresh
//! [`preview`], not trying the same execution again.
//!
//! **No mutation ships in this increment.** [`preview`]/[`execute`] are generic over what an operation
//! actually computes/runs, so the machinery is exercised end-to-end here by a scripted, non-mutating
//! test vehicle rather than a real seam call — the seam gains its first mutating method only when
//! commit does.

use std::path::Path;
use stikk_model::{Binding, RoleReadiness};

use stikk_model::{Capability, ChangeToken, Readiness, RequestCategory, Result, StikkError, Tier};
use stikk_prikk::Prikk;

/// What a preview intends to do (design `OPL-01`; RFC 013 §2). Carries the request category, which
/// determines the tier ([`RequestCategory::tier`]) — never declared separately, so a new intent cannot
/// forget to be gated. `operation` is stikk's own short name for it (`"commit"`, `"seal"`, …), used
/// verbatim in [`StikkError::Stale`]/[`StikkError::Declined`] messages; never prikk's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intent {
    /// The request category this intent belongs to — determines the confirmation tier.
    pub category: RequestCategory,
    /// A short, stikk-authored name for this operation (e.g. `"commit"`).
    pub operation: &'static str,
}

/// The `TU-09` fact set a confirmation restates — fixed and uniform, not a per-operation blob (RFC 013
/// Q1). Composed once, at preview time, and carried unchanged through to [`confirm`]; never re-derived
/// there, which would reintroduce the drift this token exists to prevent.
///
/// Every string is stikk-authored or a prikk-authoritative id (`C-T4e`) — never a display string a
/// repository could influence. Render `target_ids` through `inert` at the frontend, as everywhere else
/// repository-sourced text reaches a cell (`C-T2a`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationSummary {
    /// The operation's human name (e.g. `"Commit"`).
    pub operation: String,
    /// The target ids and names involved, from prikk-authoritative values.
    pub target_ids: Vec<String>,
    /// Labelled counts (e.g. `("patches", 3)`) — a label stikk authored, a number prikk reported.
    pub counts: Vec<(&'static str, u64)>,
    /// The capability this operation consumes (`AUTHOR`/`MAINTAINER`) — informative only for
    /// tier-3-typed, which is gated on Operator readiness (`AC-04`), not the signing ladder.
    pub capability: Capability,
    /// What becomes permanent, and what does not — in stikk's own words.
    pub consequence: String,
    /// The exact name the user must type back for a tier-3-typed confirmation (`FR-102`/`FR-103`).
    /// `None` for every other tier.
    pub target_name: Option<String>,
    /// The key id that will sign this operation, if one is configured (`FL-05` step 5; `FL-06` as
    /// amended by RFC 023 Handoff B) — read from the environment by
    /// [`stikk_prikk::key_id`], which is a separate module from the presence-only
    /// [`stikk_prikk::env`] for a reason worth reading there.
    ///
    /// **One field, not two.** Whichever role the operation consumes is the one whose id belongs here;
    /// `capability` already says which role that is, so a pair of fields would let them disagree.
    ///
    /// **`None` is an absence and must render as nothing** — never `"(unknown)"` or any other
    /// placeholder. An operation reaching a confirmation without the id it needs is already impossible
    /// (`capability_gate` refuses on `NotReady` first), so a placeholder here could only ever be stikk
    /// claiming to know something it does not.
    ///
    /// A key id is a **label**, not key material: prikk prints it itself, and `C-I1b` already has the
    /// UI showing key ids and nothing else. It is still repository-adjacent text and renders through
    /// `inert` (`C-T2a`) like every other string here.
    pub signing_key_id: Option<String>,
    /// What the card may honestly claim about [`Self::signing_key_id`] (RFC 026 §5, following prikk's
    /// own guidance in letter 007 §2).
    ///
    /// **The id alone is not a statement.** On a repository where nothing has signed under this id
    /// yet, naming it plainly claims a binding that does not exist — and that is the *default* state
    /// of every freshly created repository, so it is the first commit a new user makes.
    pub signing_key_claim: KeyClaim,
    /// Set when the key that would sign is one prikk **publishes as an example** (`C-S2`).
    ///
    /// **Persistent, not a one-time notice.** It is recomputed for every confirmation from the key
    /// actually in effect, so a user who dismissed it once and kept working meets it again at the next
    /// one — which is what `FR-104`'s "persistently" asks for, and what makes it a control rather than
    /// a toast.
    pub signing_key_is_published_example: bool,
}

/// How firmly a confirmation may state the signing key id (RFC 026 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyClaim {
    /// No id claim at all — there is nothing prikk could tell stikk, or no id to name.
    #[default]
    None,
    /// prikk reports the key binds to what this repository already records. **The only state that
    /// earns a plain statement.**
    Bound,
    /// prikk reports no signature under this id yet: it will bind on this first signature. Stated as
    /// such — an honest confirmation, not a hedge.
    Unbound,
    /// stikk could not ask (prikk ≤ 0.40). The id is the one stikk would pass, and stikk does not know
    /// whether prikk will use it — which is exactly what RFC 026 F4 found stikk claiming without
    /// warrant, so it is said rather than implied.
    Unchecked,
}

/// The id to show and how firmly to claim it, for one role (RFC 026 §5).
///
/// **One function, both ceremonies**, so commit's card and seal's card cannot drift into two answers
/// to one question — which is the drift RFC 015 found between `stikk-tui`'s copy and the ceiling it
/// was describing.
///
/// The id comes from prikk where prikk has it (≥ 0.41, via `key status`), and from the environment
/// below that. **That is what fixes the silent case**: `PRIKK_<ROLE>_KEY_ID` unset is not "no id" —
/// prikk defaults to the role's own name and signs as it — so reading only the variable rendered
/// nothing on the default setup while prikk had an answer all along (RFC 026 Handoff B §0).
#[must_use]
pub fn signing_key_claim(
    role: RoleReadiness,
    detail: &stikk_prikk::RoleDetail,
    env_fallback: Option<String>,
) -> (Option<String>, KeyClaim) {
    let id = detail.key_id.clone().or(env_fallback);
    let Some(id) = id else {
        return (None, KeyClaim::None);
    };
    match role {
        // prikk answered.
        RoleReadiness::Known(Binding::Matches) => (Some(id), KeyClaim::Bound),
        RoleReadiness::Known(Binding::Unrecorded) => (Some(id), KeyClaim::Unbound),
        // These withhold the capability (`Capability::derive`), so a confirmation is unreachable —
        // named rather than defaulted so a future state cannot acquire a silent claim here.
        RoleReadiness::Known(Binding::NotAdopted | Binding::Mismatch) => (Some(id), KeyClaim::None),
        RoleReadiness::Known(Binding::Absent) => (None, KeyClaim::None),
        // stikk could not ask. The id is real — it is what stikk would pass — but the binding is not
        // a claim stikk has warrant for.
        RoleReadiness::Unknown | RoleReadiness::Unverifiable => (Some(id), KeyClaim::Unchecked),
        RoleReadiness::NotReady => (None, KeyClaim::None),
    }
}

/// What the user supplied to satisfy a tier's confirmation requirement (design `TU-09`; RFC 013 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// Tier 2/3: an explicit yes.
    ExplicitYes,
    /// Tier 3-typed: the user's typed input, checked against the summary's `target_name`.
    TypedName(String),
}

/// Produced only by [`preview`]. Single-use and step-scoped (RFC 013 Q2): one preview→confirm→execute
/// step, never held open across a multi-step ceremony. No public constructor and no `From`/`Into`
/// toward [`ConfirmedToken`] — the only way to get one is to preview.
#[derive(Debug)]
pub struct PreviewToken {
    intent: Intent,
    summary: ConfirmationSummary,
    change_token: ChangeToken,
    tier: Tier,
}

impl PreviewToken {
    /// The tier this preview's confirmation must satisfy — read to decide which evidence shape (an
    /// explicit yes, or a typed name) the confirmation surface should collect.
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// The fact set the confirmation surface renders. Never re-derive from a fresh read at confirm
    /// time (RFC 013 §3) — this is the one true copy.
    #[must_use]
    pub fn summary(&self) -> &ConfirmationSummary {
        &self.summary
    }
}

/// Produced only by [`confirm`]. The only thing [`execute`] accepts, and it takes this **by value**, so
/// it cannot be replayed — an execution that skipped confirmation is as unrepresentable as one that
/// skipped the preview (RFC 013, "a decision that followed from Q1/Q2").
#[derive(Debug)]
pub struct ConfirmedToken {
    intent: Intent,
    change_token: ChangeToken,
}

/// The result of a completed [`execute`] — the operation's name and whatever its own execution
/// produced (a patch id, a block id, …). Generic because no real mutation ships this increment; a real
/// operation supplies its own `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<T> {
    /// The operation that ran, stikk's own name for it.
    pub operation: String,
    /// Whatever the operation itself produced.
    pub result: T,
}

/// Compute a preview and stamp it with the repository's current change token (design `LC-4`/`OPL-02`).
///
/// Generic over what the operation itself computes: `compute` builds the view the user will actually
/// look at (a `ChangesView`, prikk's own plan text, …) plus the [`ConfirmationSummary`] restating it.
/// The change token is read **before** `compute` runs, so a token stamped here reflects the state the
/// preview was genuinely computed under.
///
/// # Errors
/// Propagates any [`StikkError`] the seam or `compute` raises.
pub fn preview<V>(
    prikk: &impl Prikk,
    repo: &Path,
    intent: Intent,
    compute: impl FnOnce() -> Result<(V, ConfirmationSummary)>,
) -> Result<(V, PreviewToken)> {
    let change_token = prikk.change_token(repo)?;
    let (view, summary) = compute()?;
    let tier = intent.category.tier();
    Ok((
        view,
        PreviewToken {
            intent,
            summary,
            change_token,
            tier,
        },
    ))
}

/// Check whether `readiness` satisfies what `tier` requires, independent of staleness or evidence
/// (design `FR-121`, `OPL-04`). Public and standalone — per `internal-design.md`'s `OPL-04`, this same
/// check serves **two** roles that must never drift apart: a frontend calls it directly for UI
/// affordance (disabling a command with the reason shown, before any preview exists — RFC 007's
/// `Command::unmet_reason` is today's Viewer/Author/Maintainer-only version of this idea; a tier-aware
/// operation should call this instead), and [`confirm`] calls it as its own first, structural check
/// (`OPL-04`'s "first check"). `confirm` uses it, but does not own it.
///
/// Tier 1 always passes (RFC 013 decision 6: free, no exception). Read-only refuses every tier above it
/// outright (`NFR-S01`), via [`Readiness::may_operate`] — the method RFC 012 F-a moved to `Readiness`
/// precisely because [`Capability`] had already discarded the fact it needs. Above that, capability
/// must satisfy the mutating axis for tiers 2/3; tier 3-typed needs no capability beyond read-write,
/// since Operator is orthogonal to the signing ladder (`AC-04`).
///
/// # Errors
/// [`StikkError::NotReady`] when read-only mode or capability is insufficient.
pub fn capability_gate(operation: &str, tier: Tier, readiness: Readiness) -> Result<()> {
    if tier == Tier::One {
        return Ok(());
    }
    if !readiness.may_operate() {
        return Err(StikkError::NotReady {
            detail: format!("{operation} is not available in read-only mode"),
        });
    }
    let capability = Capability::derive(readiness);
    match tier {
        Tier::Two if !capability.may_author() => Err(StikkError::NotReady {
            detail: format!("{operation} needs AUTHOR signing readiness"),
        }),
        Tier::Three if !capability.may_publish() => Err(StikkError::NotReady {
            detail: format!("{operation} needs MAINTAINER signing readiness"),
        }),
        _ => Ok(()),
    }
}

/// Consume a preview and produce a [`ConfirmedToken`] — the only way to get one (RFC 013, "a decision
/// that followed from Q1/Q2"). This is where every gate lives (`OPL-04`'s first check): read-only and
/// capability ([`capability_gate`]), then a fresh change-token comparison
/// ([`StikkError::Stale`] on any difference — the world moved while the
/// user was looking at the preview), then the evidence itself
/// ([`StikkError::Declined`] if it does not satisfy the tier).
///
/// # Errors
/// [`StikkError::NotReady`] if read-only mode or capability is insufficient; [`StikkError::Stale`] if
/// the repository changed since the preview; [`StikkError::Declined`] if `evidence` does not satisfy
/// `token`'s tier; any [`StikkError`] the seam's change-token read raises.
pub fn confirm(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: Readiness,
    evidence: Evidence,
) -> Result<ConfirmedToken> {
    capability_gate(token.intent.operation, token.tier, readiness)?;

    let current = prikk.change_token(repo)?;
    if current != token.change_token {
        return Err(StikkError::Stale {
            operation: token.intent.operation.to_string(),
        });
    }

    let satisfied = match (token.tier, &evidence) {
        (Tier::One, _) => true, // never reached in practice: tier 1 never calls confirm
        (Tier::Two | Tier::Three, Evidence::ExplicitYes) => true,
        (Tier::ThreeTyped, Evidence::TypedName(typed)) => {
            !typed.is_empty() && token.summary.target_name.as_deref() == Some(typed.as_str())
        }
        _ => false,
    };
    if !satisfied {
        return Err(StikkError::Declined {
            detail: format!(
                "{} was not confirmed as this tier requires",
                token.intent.operation
            ),
        });
    }

    Ok(ConfirmedToken {
        intent: token.intent,
        change_token: token.change_token,
    })
}

/// Consume a [`ConfirmedToken`] and run the operation, re-checking the change token immediately first
/// (`OPL-02`'s real guard). Takes `confirmed` **by value**: once used, it is gone, so an execution
/// cannot be replayed by holding onto the token.
///
/// # Errors
/// [`StikkError::Stale`] if the repository changed since confirmation; otherwise whatever `run` or the
/// seam's change-token read raises.
pub fn execute<T>(
    prikk: &impl Prikk,
    repo: &Path,
    confirmed: ConfirmedToken,
    run: impl FnOnce() -> Result<T>,
) -> Result<Outcome<T>> {
    let current = prikk.change_token(repo)?;
    if current != confirmed.change_token {
        return Err(StikkError::Stale {
            operation: confirmed.intent.operation.to_string(),
        });
    }
    let result = run()?;
    Ok(Outcome {
        operation: confirmed.intent.operation.to_string(),
        result,
    })
}

#[cfg(test)]
mod tests;

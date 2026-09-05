//! The one class → presentation mapping (design ER-03/OP-03; RFC 007).
//!
//! Every fallible path in stikk returns a [`stikk_model::StikkError`] (ER-01). This module is the
//! **single** place that turns one into a [`Presentation`] — where and how it is shown — so the TUI and
//! the future GUI render from one decision and cannot diverge (ER-03, tested in isolation TS-05).
//!
//! Two invariants are load-bearing:
//! - **Verbatim truth** (ER-02/NFR-I03/C-T4c): a [`RefusalCard`]'s `verbatim` is prikk's message
//!   unchanged; the `gloss` is stikk's own voice, additive and separate — never a rewrite.
//! - **Next-steps are stikk-authored** (C-T2b): they come from `(class, operation)`, never parsed from
//!   the refusal text, so a hostile message cannot forge an action the user could take.

use stikk_model::StikkError;

use crate::glossary;

/// What was attempted, so the same error class yields next-steps fit for the surface it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationContext {
    /// Opening a repository / the initial orientation.
    Orient,
    /// Loading a ref's history.
    LoadHistory,
    /// Loading a block's replayed state.
    LoadBlockState,
    /// Loading worktree-vs-baseline changes.
    LoadChanges,
    /// Listing refs (the ref picker).
    ListRefs,
    /// Committing worktree changes (RFC 014). Prevention handles the common cases (F2/F6)
    /// client-side; a refusal reaching here is the race, or something neither prevention check saw.
    Commit,
    /// Anything not otherwise distinguished.
    Other,
}

/// A navigation target both frontends understand. Read/help targets have renderers now; the mutation
/// and diagnostic targets are defined so the mapping is complete, and gain renderers with their views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Target {
    /// The Orientation view.
    Orientation,
    /// The History view.
    History,
    /// The ref picker overlay.
    RefPicker,
    /// The Changes (worktree-vs-baseline) view.
    Changes,
    /// The glossary / help browser.
    Glossary,
    /// The lock inspector (FR-102) — renderer lands with recovery.
    LockInspector,
    /// Trust &amp; Keys (FR-104) — renderer lands with trust.
    TrustKeys,
    /// prikk itself is too old for the operation attempted (RFC 012 F-b) — distinct from
    /// [`Target::TrustKeys`], which is about signing readiness, not the prikk binary's version. A user
    /// on prikk 0.27 opening Changes was previously told to check their signing keys; their keys were
    /// never the problem.
    PrikkVersion,
    /// The Verify report (FR-100) — renderer lands with integrity.
    Verify,
    /// The Doctor view (FR-101) — renderer lands with recovery.
    Doctor,
}

/// What activating a next-step does. Increment 4's set is **navigational only** (NFR-S04): open a
/// view, re-run a read, or dismiss with a "resolve it yourself, then retry" — never a mutation, never
/// an auto-retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NextTarget {
    /// Move to a view/overlay.
    OpenView(Target),
    /// Re-run the read that failed (a read, at the user's explicit request — not an auto-retry).
    Refresh,
    /// Close the explanation; the condition is the user's to resolve outside stikk.
    DismissAndResolveExternally,
}

/// One actionable next-step in a refusal card (FR-110 ③). `label` is stikk's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextStep {
    /// The action's label, in stikk's voice.
    pub label: String,
    /// What it does.
    pub target: NextTarget,
}

/// The content of a refusal overlay (FR-110/TU-08). Everything is stikk-owned or verbatim prikk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalCard {
    /// prikk's own stated reason, verbatim and unedited — never rewritten by a layer above (`ER-02`).
    /// Always prikk's words: a condition stikk itself detects (design-review C1, RFC 013) gets its own
    /// [`Presentation`] variant instead of borrowing this one, so this field's invariant never has to
    /// carry an exception.
    pub verbatim: String,
    /// A plain-language explanation in stikk's voice; `None` degrades to verbatim-only (RR-5).
    pub gloss: Option<String>,
    /// Next-steps that exist, from `(class, operation)` — never parsed from `verbatim` (C-T2b).
    pub next_steps: Vec<NextStep>,
    /// Glossary codes named in the message that resolve to an entry (FR-111 links).
    pub glossary_codes: Vec<String>,
}

/// Where and how an error is presented (design OP-03). The frontend switches on this.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Presentation {
    /// A full refusal overlay (TU-08).
    RefusalOverlay(RefusalCard),
    /// A non-modal banner with an optional jump target (lock-conflict → Lock inspector).
    Banner {
        /// The verbatim conflict message.
        message: String,
        /// Where to jump to act on it (renderer lands with the target's view).
        jump: Option<Target>,
    },
    /// Inline guidance toward a surface (not-ready → Trust &amp; Keys).
    InlineGuidance {
        /// What is missing.
        detail: String,
        /// The surface that resolves it.
        toward: Target,
    },
    /// Routed into a content view, never a popup (integrity-finding → Verify/Doctor).
    RoutedIntoView {
        /// The view to route into.
        target: Target,
        /// The verbatim finding text.
        message: String,
    },
    /// Belongs in the pre-execution confirmation, not after failure (limits).
    InConfirmation {
        /// The verbatim limit message.
        message: String,
    },
    /// A plain statement with the failing detail and, if any, the original cause (environment).
    PlainStatement {
        /// stikk's description of what went wrong.
        detail: String,
        /// The underlying cause's text, if any (NFR-I03).
        original: Option<String>,
    },
    /// A fault screen: repository untouched, session preserved, read-only continuation (ER-04).
    FaultScreen {
        /// The internal invariant that was violated.
        detail: String,
    },
    /// The repository changed between a preview and its confirmation, or between confirmation and
    /// execution (`OPL-02`/`CT-05`; RFC 013 decision 3). **Never [`Presentation::RefusalOverlay`]**:
    /// prikk was never asked, so there is no prikk message to show under a "prikk reported" label
    /// (design-review C1) — every string here is stikk's own, and the renderer must say so.
    Stale {
        /// The operation whose preview no longer matches (stikk's own short name, e.g. `"commit"`).
        operation: String,
        /// stikk's explanation, in its own voice — never attributed to prikk.
        gloss: String,
        /// Next-steps that exist, stikk-authored (`C-T2b`) — today always exactly one: re-preview
        /// (`NFR-S04` forbids any that re-runs the execution).
        next_steps: Vec<NextStep>,
    },
}

/// Map an error to its presentation, given the operation that produced it (design ER-03/OP-03).
#[must_use]
pub fn present(error: &StikkError, op: OperationContext) -> Presentation {
    match error {
        StikkError::Refusal { message } => {
            // Envelope-schema skew (RFC 012 F-e) can occur from *any* read — Orient, LoadHistory,
            // LoadChanges, ... — so it is recognized by the message's stable shape and overrides the
            // (class, operation) gloss/next-steps below, rather than being layered onto them the way
            // `.prikkignore`'s next-step (RFC 009 F5) is added unconditionally for one operation only.
            // The next-step's label and target are still entirely stikk-authored (C-T2b) — only the
            // *decision to show this one instead of the generic pair* looks at the message text.
            let is_schema_skew = message.contains(glossary::SCHEMA_SKEW_CODE);
            Presentation::RefusalOverlay(RefusalCard {
                verbatim: message.clone(),
                gloss: if is_schema_skew {
                    Some(SCHEMA_SKEW_GLOSS.to_string())
                } else {
                    refusal_gloss(op)
                },
                next_steps: if is_schema_skew {
                    vec![NextStep {
                        label: "Upgrade prikk (resolve outside stikk)".to_string(),
                        target: NextTarget::DismissAndResolveExternally,
                    }]
                } else {
                    refusal_next_steps(op)
                },
                glossary_codes: glossary::codes_in(message)
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            })
        }
        StikkError::LockConflict { message } => Presentation::Banner {
            message: message.clone(),
            // A jump to the Lock inspector lands with FR-102; None until then.
            jump: None,
        },
        StikkError::NotReady { detail } => Presentation::InlineGuidance {
            detail: detail.clone(),
            // `NotReady` is overloaded for two unrelated conditions (RFC 012 F-b): absent signing
            // readiness (Trust & Keys is genuinely the fix), and version skew (`changes_view`'s < 0.28
            // gate, where signing has nothing to do with it). Disambiguated by `OperationContext`, never
            // by the message text (C-T2b) — `LoadChanges` is, today, the only operation that constructs
            // `NotReady` for the version-gate reason; every other `NotReady` still means Trust & Keys.
            toward: match op {
                OperationContext::LoadChanges => Target::PrikkVersion,
                _ => Target::TrustKeys,
            },
        },
        StikkError::IntegrityFinding { message } => Presentation::RoutedIntoView {
            target: Target::Verify,
            message: message.clone(),
        },
        StikkError::Limits { message } => Presentation::InConfirmation {
            message: message.clone(),
        },
        StikkError::Environment { detail, source } => Presentation::PlainStatement {
            detail: detail.clone(),
            original: source.as_ref().map(std::string::ToString::to_string),
        },
        StikkError::Internal { detail } => Presentation::FaultScreen {
            detail: detail.clone(),
        },
        StikkError::Stale { operation } => Presentation::Stale {
            operation: operation.clone(),
            gloss: "Another writer moved something in this repository between your preview and now. \
                 This is not a retry: previewing again re-reads the repository's current state, which \
                 is the only safe way forward."
                .to_string(),
            // RFC 013 §5 / NFR-S04: the only correct next step re-runs the *preview* (a read), never
            // the execution — `Refresh` already means exactly that everywhere else it is used.
            next_steps: vec![NextStep {
                label: "Preview again".to_string(),
                target: NextTarget::Refresh,
            }],
        },
        StikkError::Declined { detail } => Presentation::InConfirmation {
            message: detail.clone(),
        },
        StikkError::CrossRef { message } => Presentation::RefusalOverlay(RefusalCard {
            // `message` is prikk's own words (RFC 014 F2) — unlike `Stale`, this genuinely satisfies
            // `RefusalCard.verbatim`'s invariant. A fixed override independent of `op` (like the
            // schema-skew shape above): this class only ever comes from a commit, and stikk prevents it
            // client-side (`Orientation::queued_target`) before arming one, so reaching here at all is
            // already the rare race case (RFC 014 decision 2) — the same gloss and next-steps apply
            // regardless of which read happened to be in flight when it hit.
            verbatim: message.clone(),
            gloss: Some(
                "prikk's active queue belongs to a different ref than the one you tried to commit \
                 to. Seal the queue first, or switch focus to the ref it targets."
                    .to_string(),
            ),
            next_steps: vec![
                NextStep {
                    label: "Choose another ref".to_string(),
                    target: NextTarget::OpenView(Target::RefPicker),
                },
                NextStep {
                    label: "Refresh".to_string(),
                    target: NextTarget::Refresh,
                },
            ],
            glossary_codes: Vec::new(),
        }),
        // `StikkError` is `#[non_exhaustive]`: a class added later degrades to an honest plain
        // statement carrying its message, never a panic (RR-5 discipline applied to our own type).
        other => Presentation::PlainStatement {
            detail: other.to_string(),
            original: None,
        },
    }
}

/// The gloss for an envelope-schema-skew refusal (RFC 012 F-e), shown regardless of which operation
/// triggered it. `FR-003` requires stikk to refuse-and-explain version skew rather than let RR-5's safe
/// verbatim-only degradation stand where a better explanation is actually possible.
const SCHEMA_SKEW_GLOSS: &str = "This repository holds content sealed by a newer prikk than the one \
     currently running here. prikk's compatibility guarantee runs one way — a newer prikk can always \
     read what an older one wrote, not the reverse — so an older prikk cannot read this. stikk cannot \
     translate the schema; upgrading the prikk binary this session uses is the only fix.";

/// The plain-language gloss for a refusal, chosen by the surface it came from. Additive to prikk's
/// message, never a replacement (ER-02). `None` where stikk has nothing honest to add (verbatim-only).
fn refusal_gloss(op: OperationContext) -> Option<String> {
    let text = match op {
        OperationContext::LoadHistory => {
            "prikk declined to read this ref's history. The ref may not exist, or its name may be \
             mistyped. stikk cannot override prikk's decision."
        }
        OperationContext::ListRefs => {
            "prikk declined to list refs for this repository. stikk shows prikk's reason above."
        }
        OperationContext::LoadBlockState => {
            "prikk declined to replay this ref's state. stikk shows prikk's reason above."
        }
        OperationContext::LoadChanges => {
            "prikk declined to report worktree changes for this ref. stikk shows prikk's reason above."
        }
        OperationContext::Orient => {
            "prikk declined to open this repository. Its message above is the authoritative reason."
        }
        OperationContext::Commit => {
            "prikk declined this commit. stikk shows prikk's reason above; the usual preconditions \
             (a matching queue target, a non-empty worktree) were already checked before this was \
             attempted."
        }
        OperationContext::Other => return None,
    };
    Some(text.to_string())
}

/// Next-steps for a refusal, from the operation context. Navigational only (NFR-S04); never derived
/// from the message text (C-T2b).
fn refusal_next_steps(op: OperationContext) -> Vec<NextStep> {
    match op {
        OperationContext::LoadHistory | OperationContext::LoadBlockState => vec![
            NextStep {
                label: "Choose another ref".to_string(),
                target: NextTarget::OpenView(Target::RefPicker),
            },
            NextStep {
                label: "Refresh".to_string(),
                target: NextTarget::Refresh,
            },
        ],
        OperationContext::LoadChanges => vec![
            NextStep {
                label: "Choose another ref".to_string(),
                target: NextTarget::OpenView(Target::RefPicker),
            },
            NextStep {
                label: "Refresh".to_string(),
                target: NextTarget::Refresh,
            },
            // RFC 009 F5: a malformed `.prikkignore` is one cause of a Changes refusal that neither of
            // the above resolves. This is offered unconditionally for every LoadChanges refusal — never
            // derived from prikk's message text (C-T2b) — and is guidance, not an action: stikk must
            // not edit a repository file (CON-1, INV-1).
            NextStep {
                label: "Check `.prikkignore` for a malformed rule (edit it outside stikk)"
                    .to_string(),
                target: NextTarget::DismissAndResolveExternally,
            },
        ],
        OperationContext::ListRefs | OperationContext::Orient => vec![NextStep {
            label: "Refresh".to_string(),
            target: NextTarget::Refresh,
        }],
        OperationContext::Commit => vec![
            NextStep {
                label: "Back to Changes".to_string(),
                target: NextTarget::OpenView(Target::Changes),
            },
            NextStep {
                label: "Refresh".to_string(),
                target: NextTarget::Refresh,
            },
        ],
        OperationContext::Other => vec![NextStep {
            label: "Dismiss".to_string(),
            target: NextTarget::DismissAndResolveExternally,
        }],
    }
}

#[cfg(test)]
mod tests;

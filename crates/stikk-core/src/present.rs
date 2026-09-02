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
    /// Listing refs (the ref picker).
    ListRefs,
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
    /// The glossary / help browser.
    Glossary,
    /// The lock inspector (FR-102) — renderer lands with recovery.
    LockInspector,
    /// Trust &amp; Keys (FR-104) — renderer lands with trust.
    TrustKeys,
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
    /// prikk's message, verbatim (shown inert and quoted by the frontend). Never rewritten (ER-02).
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
}

/// Map an error to its presentation, given the operation that produced it (design ER-03/OP-03).
#[must_use]
pub fn present(error: &StikkError, op: OperationContext) -> Presentation {
    match error {
        StikkError::Refusal { message } => Presentation::RefusalOverlay(RefusalCard {
            verbatim: message.clone(),
            gloss: refusal_gloss(op),
            next_steps: refusal_next_steps(op),
            glossary_codes: glossary::codes_in(message)
                .into_iter()
                .map(str::to_string)
                .collect(),
        }),
        StikkError::LockConflict { message } => Presentation::Banner {
            message: message.clone(),
            // A jump to the Lock inspector lands with FR-102; None until then.
            jump: None,
        },
        StikkError::NotReady { detail } => Presentation::InlineGuidance {
            detail: detail.clone(),
            toward: Target::TrustKeys,
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
        // `StikkError` is `#[non_exhaustive]`: a class added later degrades to an honest plain
        // statement carrying its message, never a panic (RR-5 discipline applied to our own type).
        other => Presentation::PlainStatement {
            detail: other.to_string(),
            original: None,
        },
    }
}

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
        OperationContext::Orient => {
            "prikk declined to open this repository. Its message above is the authoritative reason."
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
        OperationContext::ListRefs | OperationContext::Orient => vec![NextStep {
            label: "Refresh".to_string(),
            target: NextTarget::Refresh,
        }],
        OperationContext::Other => vec![NextStep {
            label: "Dismiss".to_string(),
            target: NextTarget::DismissAndResolveExternally,
        }],
    }
}

#[cfg(test)]
mod tests;

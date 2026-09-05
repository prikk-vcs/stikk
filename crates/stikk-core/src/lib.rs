//! The operation layer — one shared operation set both frontends drive (design `stikk-04` AR-03,
//! MOD-05).
//!
//! This crate owns no I/O and no widgets. It turns a user intent into a sequence of seam requests
//! plus state reads and view-model productions, applying capability gating and (in later increments)
//! the preview-first rule and confirmation tiers. Because both the TUI and the GUI drive *this* API
//! and neither defines operations of its own, an operation present in one frontend and not the other
//! is impossible (the mechanical guarantee behind TUI/GUI parity, FR-123).
//!
//! This foundation increment implements one operation — [`orient`] — the read-only orientation a
//! session opens with. More operation families land against the same shape.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod change_token;
pub mod changes;
pub mod commit;
pub mod confirm;
pub mod glossary;
pub mod history;
pub mod orient;
pub mod palette;
pub mod present;
pub mod refusal_history;

pub use change_token::{change_token, staleness_notice};
pub use changes::{ChangeEntry, ChangeKind, ChangesView, changes_view};
pub use commit::{
    COMMIT_OPERATION, CommitPreview, CommitPreviewOutcome, commit_confirm_and_execute,
    commit_preview,
};
pub use confirm::{
    ConfirmationSummary, ConfirmedToken, Evidence, Intent, Outcome, PreviewToken, capability_gate,
    confirm, execute, preview,
};
pub use glossary::{GlossaryEntry, TermMapping};
pub use history::{BlockDetailView, HistoryView, block_detail, history_view, list_refs};
pub use orient::{OrientationView, orient};
pub use palette::Command;
pub use present::{
    NextStep, NextTarget, OperationContext, Presentation, RefusalCard, Target, present,
};
pub use refusal_history::{RefusalHistory, RefusalRecord};

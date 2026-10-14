//! Verify: whether the repository is sound, in prikk's own verdict (design `FR-100`; RFC 040).
//!
//! **prikk decides; this module shapes the answer for a frontend to draw.** Nothing here decides
//! that a repository is sound, re-derives a verdict from the stages, or infers which object failed.
//! prikk's `verify` is the only authority, and its verdict and its conditions arrive from one
//! declaration in prikk (RFC 040 F1), so there is nothing here to reconcile.
//!
//! # The two things this module is careful about
//!
//! - **Unreported is not sound** (`C-T2c'`). Below prikk 0.39 stikk spawns nothing, and the view
//!   says so in stikk's own voice (`C-T4d`). A repository nobody checked must never read like one
//!   prikk found no fault in — which is the same mistake in the opposite direction from the one
//!   RFC 040 F2 is about.
//! - **The standing sentence is always shown** (decision 3). prikk's own `item-failure` message
//!   says *"see item outcomes above"*, and in JSON there is no "above": the item outcomes exist
//!   only in the prose report (F3). stikk names where "above" actually is. It is shown on a sound
//!   report too, because it is true there as well, and a sentence that appeared only on failure
//!   would read as part of the failure rather than as a standing limit of the JSON.

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{
    ActiveSessions, FailedCondition, Prikk, StageStatus, Verification, VerifyReport, VerifyStage,
};

use crate::patch::bounded_rows;
use crate::piece::Piece;

/// The first prikk minor whose verify report stikk reads, re-exported so a view says the same
/// number the seam gates on — the one-constant rule the other views follow.
pub use stikk_prikk::JSON_REPORTS_FROM_MINOR as VERIFY_FROM_MINOR;

/// **stikk's standing sentence about what prikk's JSON leaves out** (RFC 040 decision 3, F3).
///
/// Always shown, sound or damaged. prikk's `item-failure` condition ends *"see item outcomes
/// above"*, and a JSON consumer has no "above" to look at: prikk's emitter puts item-level
/// findings and counts deliberately outside `verify-report-v1`. Naming the terminal is what makes
/// repeating prikk's sentence honest rather than a dead pointer.
pub const JSON_OMITS: &str = "prikk's JSON report does not list which objects or refs failed, or \
                             count unverifiable signatures. Run `prikk verify` in a terminal for \
                             those.";

/// One verify run, ready to render (RFC 040 decision 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyView {
    /// prikk ran, and this is what it said.
    Checked(Checked),
    /// **Below prikk 0.39**, carrying prikk's minor. stikk's own voice (`C-T4d`): it reads prikk's
    /// verify report from 0.39 on. **Never a sound verdict**, which would say stikk had checked.
    Unreported(u32),
}

impl VerifyView {
    /// The run, or `None` where stikk did not check.
    #[must_use]
    pub fn checked(&self) -> Option<&Checked> {
        match self {
            Self::Checked(checked) => Some(checked),
            Self::Unreported(_) => None,
        }
    }

    /// Why there is no verdict, in stikk's own words — `None` when there is one.
    #[must_use]
    pub fn unreported_reason(&self) -> Option<String> {
        match self {
            Self::Checked(_) => None,
            Self::Unreported(minor) => Some(format!(
                "stikk reads prikk's verify report from prikk 0.{VERIFY_FROM_MINOR} on; this is \
                 0.{minor}."
            )),
        }
    }
}

/// One verify run, shaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Whether prikk found the repository sound.
    pub ok: bool,
    /// Every failing condition, in prikk's order.
    pub conditions: Vec<ConditionRow>,
    /// Every stage prikk ran, in prikk's order.
    pub stages: Vec<StageRow>,
    /// prikk's session coverage, when it reported any.
    pub sessions: Option<ActiveSessions>,
    /// prikk's own JSON, as bounded rows for `v`.
    pub raw: Vec<String>,
}

impl Checked {
    /// **The verdict, in stikk's words** — never prikk's, which has no one sentence for "sound".
    ///
    /// The count is of prikk's own failing conditions, so it cannot drift from what is listed
    /// beneath it.
    #[must_use]
    pub fn verdict(&self) -> String {
        if self.ok {
            return "prikk reports this repository sound".to_string();
        }
        format!("prikk reports {} problem(s)", self.conditions.len())
    }

    /// prikk's session coverage as a sentence, or `None` where prikk reported none.
    ///
    /// **Not reported is silence, never `0 of 0`** (`C-T2c'`).
    #[must_use]
    pub fn sessions_sentence(&self) -> Option<String> {
        self.sessions.map(|s| {
            format!(
                "prikk verified {} of {} active session(s)",
                s.verified_count, s.count
            )
        })
    }
}

/// One failing condition: prikk's id and prikk's sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionRow {
    /// prikk's identifier for the condition.
    pub id: String,
    /// prikk's sentence, verbatim.
    pub message: String,
}

impl ConditionRow {
    /// The row as pieces: the id is a **name** the repository's tooling chose, the message is
    /// prikk's prose. The frontend renders each under its own rule (RFC 036).
    #[must_use]
    pub fn row(&self) -> Vec<Piece> {
        vec![
            Piece::name(&self.id),
            Piece::said("  "),
            Piece::said(&self.message),
        ]
    }
}

/// One stage: prikk's name, prikk's status word, and whatever prikk paired with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRow {
    /// prikk's stage name.
    pub stage: String,
    /// prikk's status word, exactly as prikk wrote it.
    pub status: String,
    /// prikk's message, `blocked_by` or `after`, where it gave one.
    pub detail: Option<String>,
}

impl StageRow {
    /// The row as pieces: the stage name is a **name**, the status word is prikk's, and the
    /// detail — a message, or another stage's name — follows as prose.
    #[must_use]
    pub fn row(&self) -> Vec<Piece> {
        let mut pieces = vec![
            Piece::name(&self.stage),
            Piece::said("  "),
            Piece::said(&self.status),
        ];
        if let Some(detail) = &self.detail {
            pieces.push(Piece::said("  \u{2014}  "));
            pieces.push(Piece::said(detail));
        }
        pieces
    }

    /// Whether this stage is prikk's plain `evaluated`.
    #[must_use]
    pub fn is_evaluated(&self) -> bool {
        self.status == "evaluated"
    }
}

/// Run `verify` and shape the answer.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises. **A refusal is propagated, not
/// carried**, as in Compare: a Verify screen *is* the report, so there is nothing left to show
/// beside the failure. **An exit 1 that carried a report is not a refusal** and never reaches
/// here as one (RFC 040 decision 1) — that is the seam's rule, and the whole point of it.
pub fn verify(prikk: &impl Prikk, repo: &Path) -> Result<VerifyView> {
    match prikk.verify(repo)? {
        VerifyReport::Unreported => Ok(VerifyView::Unreported(prikk.handshake()?.version.minor)),
        VerifyReport::Listed(report) => Ok(VerifyView::Checked(checked(report))),
    }
}

/// Shape one report. Separate from [`verify`] so a test can drive it without a backend.
#[must_use]
pub fn checked(report: Verification) -> Checked {
    Checked {
        ok: report.verdict.ok,
        conditions: report
            .verdict
            .failed_conditions
            .iter()
            .map(|FailedCondition { id, message }| ConditionRow {
                id: id.clone(),
                message: message.clone(),
            })
            .collect(),
        stages: report.stages.iter().map(stage_row).collect(),
        sessions: report.active_sessions,
        // The same bound and the same helper as Patch detail's and Compare's raw views: a toggle
        // must not be the way around a bound that exists to keep a large report off the screen.
        raw: bounded_rows(&report.raw),
    }
}

/// One stage, flattened to prikk's word plus the one field prikk paired with it.
///
/// **The status word is prikk's in every case**, including one stikk has not measured — which is
/// why this returns a `String` rather than re-deriving an enum the view would have to match on.
fn stage_row(stage: &VerifyStage) -> StageRow {
    let (status, detail) = match &stage.status {
        StageStatus::Evaluated => ("evaluated", None),
        StageStatus::Failed { message } => ("failed", Some(message.clone())),
        StageStatus::NotEvaluated { blocked_by } => {
            ("not_evaluated", Some(format!("blocked by {blocked_by}")))
        }
        StageStatus::Halted { after } => ("halted", Some(format!("after {after}"))),
        StageStatus::Other(word) => (word.as_str(), None),
    };
    StageRow {
        stage: stage.stage.clone(),
        status: status.to_string(),
        detail,
    }
}

#[cfg(test)]
mod tests;

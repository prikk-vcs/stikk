//! The session refusal history (design DM-06, FR-112; RFC 007).
//!
//! A capped, in-memory ring of the session's refusals, so a user can revisit an explanation after
//! closing it (FR-112). This increment persists nothing, which honours the private/ephemeral-session
//! rule (LC-8) trivially; a persisted store lands with `stikk-state` and its privacy gate.
//!
//! The content is diagnostic text — prikk's verbatim message, the classified class, the attempted
//! operation, and stikk's own capture time — never repository authority (DM-06, C-R1: the timestamp is
//! stikk's record, not fabricated repository time).

use crate::present::OperationContext;

/// The most refusals stikk keeps for the session. Old entries fall off the front.
const CAPACITY: usize = 50;

/// One remembered refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalRecord {
    /// prikk's verbatim message.
    pub verbatim: String,
    /// The classified error class (CT-04 name, e.g. `"refusal"`).
    pub class: &'static str,
    /// What was attempted.
    pub operation: OperationContext,
    /// A monotonic sequence number within the session (stikk's own ordering, not repository time).
    pub seq: u64,
}

/// A capped ring of the session's refusals, newest last internally; [`RefusalHistory::recent`] yields
/// them newest-first for display.
#[derive(Debug, Clone, Default)]
pub struct RefusalHistory {
    records: Vec<RefusalRecord>,
    next_seq: u64,
}

impl RefusalHistory {
    /// A new, empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a refusal. Drops the oldest when at capacity.
    pub fn record(
        &mut self,
        verbatim: impl Into<String>,
        class: &'static str,
        operation: OperationContext,
    ) {
        let record = RefusalRecord {
            verbatim: verbatim.into(),
            class,
            operation,
            seq: self.next_seq,
        };
        self.next_seq += 1;
        self.records.push(record);
        if self.records.len() > CAPACITY {
            self.records.remove(0);
        }
    }

    /// The remembered refusals, newest first.
    #[must_use]
    pub fn recent(&self) -> Vec<&RefusalRecord> {
        self.records.iter().rev().collect()
    }

    /// How many refusals are remembered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests;

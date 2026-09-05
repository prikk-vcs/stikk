//! The command registry behind the palette (design TU-07, FR-125; RFC 007).
//!
//! Every view and operation registers one [`Command`] — a name, a default binding, and the minimum
//! capability it needs. The palette lists them all; entries the session cannot perform stay **visible
//! but disabled with a reason** (FR-104), never hidden. This is the mechanical guarantee of
//! discoverability parity (FR-125): a future operation appears in the palette the moment it registers,
//! and the TUI/GUI share one list.
//!
//! Increment 4 seeded the registry with Viewer-level reads only, gated on a bare [`Capability`]. RFC
//! 014 §6 unifies that affordance check with [`crate::confirm::capability_gate`] — the same tier-aware
//! check `confirm` enforces — because commit is the first mutating command entering this registry, and
//! RFC 013 deferred the unification with a deadline of exactly this moment: left as a bare `Capability`
//! comparison, the palette would grey out on signing capability alone and could offer a commit that
//! `confirm` then refuses under `STIKK_READ_ONLY=1`, since capability alone does not see read-only.

use stikk_model::{Readiness, StikkError, Tier};

use crate::confirm::capability_gate;
use crate::present::Target;

/// A palette-listable command: a view to open or an action to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// A stable id (for tests and, later, the action-id binding map).
    pub id: &'static str,
    /// The human name shown in the palette.
    pub name: &'static str,
    /// The default key binding, shown beside the name.
    pub binding: &'static str,
    /// stikk's own short name for [`crate::confirm::capability_gate`]'s message (e.g. `"commit"`,
    /// matching [`crate::commit::COMMIT_OPERATION`]) — **not** `name`: `capability_gate`'s reason
    /// already reads `"<operation> needs …"`, and repeating the full display name there produced a
    /// redundant, overlong disabled-entry line the palette's fixed width then truncated (caught while
    /// verifying RFC 014 §6's own regression test against the render). Irrelevant for a `Tier::One`
    /// command, which never reaches `capability_gate`'s error text.
    pub operation: &'static str,
    /// The confirmation tier this command requires (RFC 013 decision 4) — `Tier::One` for every
    /// navigation command (free, always available); a mutating command's real tier otherwise. Below it
    /// the entry is visible-but-disabled (FR-104).
    pub tier: Tier,
    /// The view/overlay the command opens, if it is a navigation command.
    pub opens: Option<Target>,
}

impl Command {
    /// Whether a session with `readiness` may run this command — the same check
    /// [`crate::confirm::confirm`] itself enforces (RFC 014 §6), so nothing offered here is refused
    /// there.
    #[must_use]
    pub fn available_to(&self, readiness: Readiness) -> bool {
        capability_gate(self.operation, self.tier, readiness).is_ok()
    }

    /// The reason this command is unavailable for `readiness`, for the disabled-entry label (FR-104),
    /// or `None` when it is available. The entry stays **visible** either way (TU-07).
    #[must_use]
    pub fn unmet_reason(&self, readiness: Readiness) -> Option<String> {
        match capability_gate(self.operation, self.tier, readiness) {
            Ok(()) => None,
            Err(StikkError::NotReady { detail }) => Some(detail),
            // `capability_gate` only ever returns `NotReady`; kept exhaustive (not `_`) so a change to
            // that contract fails here loudly rather than silently swallowing a new error shape.
            Err(other) => Some(other.to_string()),
        }
    }
}

/// Every registered command. Future operations add entries here.
static COMMANDS: &[Command] = &[
    Command {
        id: "view.orientation",
        name: "Go to Orientation",
        binding: "g o",
        operation: "orientation",
        tier: Tier::One,
        opens: Some(Target::Orientation),
    },
    Command {
        id: "view.history",
        name: "Open History",
        binding: "Enter",
        operation: "history",
        tier: Tier::One,
        opens: Some(Target::History),
    },
    Command {
        id: "ref.pick",
        name: "Choose ref…",
        binding: "b",
        operation: "ref-pick",
        tier: Tier::One,
        opens: Some(Target::RefPicker),
    },
    Command {
        id: "view.changes",
        name: "Open Changes (worktree)",
        binding: "w",
        operation: "changes",
        tier: Tier::One,
        opens: Some(Target::Changes),
    },
    Command {
        id: "op.commit",
        name: "Commit worktree changes",
        binding: "C",
        // Matches `stikk_core::commit::COMMIT_OPERATION` — the same word `confirm`/`execute` use for
        // this operation, so a `capability_gate` message reads identically wherever it is shown.
        operation: "commit",
        // Derived from `RequestCategory::QueueMutation` (RFC 013 decision 4) — commit's own category,
        // stated once in `stikk_core::commit::commit_preview`'s `Intent`; matched here by hand since a
        // palette entry has no `Intent` of its own to derive it from. Both must agree or this comment
        // is wrong, not the code — there is no shared source of truth to enforce it mechanically today.
        tier: Tier::Two,
        opens: None,
    },
    Command {
        id: "view.glossary",
        name: "Glossary & Help",
        binding: "?",
        operation: "glossary",
        tier: Tier::One,
        opens: Some(Target::Glossary),
    },
    Command {
        id: "session.refusals",
        name: "Recent refusals",
        binding: "R",
        operation: "refusals",
        tier: Tier::One,
        opens: None,
    },
    Command {
        id: "view.refresh",
        name: "Refresh (re-read from prikk)",
        binding: "r",
        operation: "refresh",
        tier: Tier::One,
        opens: None,
    },
    Command {
        id: "app.quit",
        name: "Quit",
        binding: "q",
        operation: "quit",
        tier: Tier::One,
        opens: None,
    },
];

/// All registered commands, in registration order.
#[must_use]
pub fn commands() -> &'static [Command] {
    COMMANDS
}

/// Commands whose name matches `filter` (case-insensitive substring; empty matches all). Order is
/// preserved so the list is stable; disabled entries are included (the caller styles them, FR-104).
#[must_use]
pub fn matching(filter: &str) -> Vec<&'static Command> {
    let needle = filter.to_ascii_lowercase();
    COMMANDS
        .iter()
        .filter(|command| needle.is_empty() || command.name.to_ascii_lowercase().contains(&needle))
        .collect()
}

#[cfg(test)]
mod tests;

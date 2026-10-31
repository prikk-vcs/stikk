//! The command registry behind the palette (design TU-07, FR-125; RFC 007).
//!
//! Every view and operation registers one [`Command`] — a name, a default binding, and the minimum
//! capability it needs. The palette lists them all; entries the session cannot perform stay **visible
//! but disabled with a reason** (FR-104), never hidden. This is the mechanical guarantee of
//! discoverability parity (FR-125): a future operation appears in the palette the moment it registers,
//! and the TUI/GUI share one list.
//!
//! Increment 4 seeded the registry with Viewer-level reads only, gated on a bare
//! [`stikk_model::Capability`]. RFC
//! 014 §6 unifies that affordance check with [`crate::confirm::capability_gate`] — the same tier-aware
//! check `confirm` enforces — because commit is the first mutating command entering this registry, and
//! RFC 013 deferred the unification with a deadline of exactly this moment: left as a bare `Capability`
//! comparison, the palette would grey out on signing capability alone and could offer a commit that
//! `confirm` then refuses under `STIKK_READ_ONLY=1`, since capability alone does not see read-only.

use stikk_model::{Readiness, RequestCategory, StikkError, Tier};

use crate::confirm::capability_gate;
use crate::present::Target;

/// The reason every action that needs a focused ref gives when none is focused (RFC 029 Handoff B §2):
/// the banner a key sets and a palette entry's disabled reason are these same words.
pub const NO_FOCUSED_REF_REASON: &str = "No ref focused. Press b to pick one.";

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
    /// The request category this command belongs to, or `None` for a command that makes no prikk
    /// request of its own (the Glossary, the refusal history, Quit).
    ///
    /// **The category, not the tier** (RFC 045 §2b). The tier and the signing requirement are
    /// both read off it, so a palette entry can no longer disagree with the `Intent` the
    /// operation itself carries — which the two comments this field replaced had to apologise
    /// for, having "no shared source of truth to enforce it mechanically today". There is one
    /// now. Where the gate refuses, the entry stays visible and disabled (FR-104).
    pub category: Option<RequestCategory>,
    /// The view/overlay the command opens, if it is a navigation command.
    pub opens: Option<Target>,
    /// Whether the command acts on the focused ref (RFC 029 Handoff B §2): History, Changes, commit and
    /// seal. With no ref focused it stays visible but disabled, with [`NO_FOCUSED_REF_REASON`]
    /// (`C-T4d`).
    pub needs_focused_ref: bool,
}

impl Command {
    /// The confirmation tier this command requires (RFC 013 decision 4), from its category —
    /// [`Tier::One`] for a command that makes no prikk request at all.
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.category.map_or(Tier::One, RequestCategory::tier)
    }

    /// Whether a session with `readiness` may run this command — the same check
    /// [`crate::confirm::confirm`] itself enforces (RFC 014 §6), so nothing offered here is refused
    /// there.
    #[must_use]
    pub fn available_to(&self, readiness: Readiness) -> bool {
        self.category
            .is_none_or(|category| capability_gate(self.operation, category, readiness).is_ok())
    }

    /// The reason this command is unavailable for `readiness`, for the disabled-entry label (FR-104),
    /// or `None` when it is available. The entry stays **visible** either way (TU-07).
    #[must_use]
    pub fn unmet_reason(&self, readiness: Readiness) -> Option<String> {
        let category = self.category?;
        match capability_gate(self.operation, category, readiness) {
            Ok(()) => None,
            Err(StikkError::NotReady { detail }) => Some(detail),
            // `capability_gate` only ever returns `NotReady`; kept exhaustive (not `_`) so a change to
            // that contract fails here loudly rather than silently swallowing a new error shape.
            Err(other) => Some(other.to_string()),
        }
    }

    /// Whether this command may run now: [`Self::available_to`], and a focused ref when it needs one
    /// (RFC 029 Handoff B §2). The palette's `Enter` checks this, so it never runs what it lists as
    /// disabled.
    #[must_use]
    pub fn available(&self, readiness: Readiness, ref_focused: bool) -> bool {
        self.unavailable_reason(readiness, ref_focused).is_none()
    }

    /// Why this command cannot run now, or `None` when it can.
    ///
    /// **Capability first.** A session that cannot commit at all is told that, rather than to choose a
    /// ref it still could not commit to. Then the focused ref, in [`NO_FOCUSED_REF_REASON`]'s words.
    #[must_use]
    pub fn unavailable_reason(&self, readiness: Readiness, ref_focused: bool) -> Option<String> {
        self.unmet_reason(readiness).or_else(|| {
            (self.needs_focused_ref && !ref_focused).then(|| NO_FOCUSED_REF_REASON.to_string())
        })
    }
}

/// Every registered command. Future operations add entries here.
static COMMANDS: &[Command] = &[
    Command {
        id: "view.orientation",
        name: "Go to Orientation",
        binding: "g o",
        operation: "orientation",
        category: Some(RequestCategory::ReadHistory),
        opens: Some(Target::Orientation),
        needs_focused_ref: false,
    },
    Command {
        id: "view.history",
        name: "Open History",
        binding: "Enter",
        operation: "history",
        category: Some(RequestCategory::ReadHistory),
        opens: Some(Target::History),
        needs_focused_ref: true,
    },
    Command {
        id: "ref.pick",
        name: "Choose ref…",
        binding: "b",
        operation: "ref-pick",
        category: Some(RequestCategory::ReadHistory),
        opens: Some(Target::RefPicker),
        needs_focused_ref: false,
    },
    Command {
        id: "view.changes",
        name: "Open Changes (worktree)",
        binding: "w",
        operation: "changes",
        category: Some(RequestCategory::WorktreeAnalysis),
        opens: Some(Target::Changes),
        needs_focused_ref: true,
    },
    Command {
        id: "view.queue",
        name: "Open Queue",
        binding: "Q",
        operation: "queue",
        category: Some(RequestCategory::ReadHistory),
        opens: Some(Target::Queue),
        // The queue is repository-wide, so it needs no focused ref (RFC 028 Handoff A §6).
        needs_focused_ref: false,
    },
    Command {
        id: "view.verify",
        name: "Verify repository",
        binding: "V",
        operation: "verify",
        // **`Integrity`, which is Tier One: it reads and changes nothing** (RFC 040 decision 4).
        // Verify repairs nothing and clears no lock, so it needs no signing readiness -- a
        // repository too damaged to commit in is exactly where a reader most needs to run it.
        category: Some(RequestCategory::Integrity),
        opens: Some(Target::Verify),
        // Verification is repository-wide, as the queue is.
        needs_focused_ref: false,
    },
    Command {
        id: "op.commit",
        name: "Commit worktree changes",
        binding: "C",
        // Matches `stikk_core::commit::COMMIT_OPERATION` — the same word `confirm`/`execute` use for
        // this operation, so a `capability_gate` message reads identically wherever it is shown.
        operation: "commit",
        // Commit's own category, the same one `stikk_core::commit::commit_preview`'s `Intent`
        // carries. The tier and the signing requirement are both read off it (RFC 045 §2b), so
        // this entry and that operation can no longer disagree about either.
        category: Some(RequestCategory::QueueMutation),
        opens: None,
        needs_focused_ref: true,
    },
    Command {
        id: "op.seal",
        name: "Seal the active WAL",
        binding: "S",
        // Matches `stikk_core::seal::SEAL_OPERATION` — the same word `confirm`/`execute` use for this
        // operation, so a `capability_gate` message reads identically wherever it is shown.
        operation: "seal",
        // Seal's own category (RFC 016 §7), as `stikk_core::seal::seal_preview`'s `Intent`
        // states it; same single source as `op.commit` above.
        category: Some(RequestCategory::Publication),
        opens: None,
        needs_focused_ref: true,
    },
    Command {
        id: "op.undo-block",
        name: "Undo this block",
        binding: "U",
        // Matches `stikk_core::undo::UNDO_OPERATION`, so a `capability_gate` message reads
        // identically wherever it is shown.
        operation: crate::undo::UNDO_OPERATION,
        // **Commit's category, because commit's is the gate this operation takes** (RFC 058
        // §3.2): the undo ends in a `commit` and needs AUTHOR signing readiness, which is what
        // this entry must say when it greys out. The operation's *own* category is
        // `Materialization` -- Tier Three, no key -- and that is what the card is confirmed at;
        // it is carried by the `Intent` inside `undo_preview`, not by this field, which the
        // palette reads only for the gate.
        category: Some(RequestCategory::QueueMutation),
        opens: None,
        // The block comes from History or Block detail, which exist only for a focused ref.
        needs_focused_ref: true,
    },
    Command {
        id: "view.glossary",
        name: "Glossary & Help",
        binding: "?",
        operation: "glossary",
        category: None,
        opens: Some(Target::Glossary),
        needs_focused_ref: false,
    },
    Command {
        id: "session.refusals",
        name: "Recent refusals",
        binding: "R",
        operation: "refusals",
        category: None,
        opens: None,
        needs_focused_ref: false,
    },
    Command {
        id: "view.refresh",
        name: "Refresh (re-read from prikk)",
        binding: "r",
        operation: "refresh",
        category: Some(RequestCategory::ReadHistory),
        opens: None,
        needs_focused_ref: false,
    },
    Command {
        id: "app.quit",
        name: "Quit",
        binding: "q",
        operation: "quit",
        category: None,
        opens: None,
        needs_focused_ref: false,
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

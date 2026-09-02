//! The command registry behind the palette (design TU-07, FR-125; RFC 007).
//!
//! Every view and operation registers one [`Command`] — a name, a default binding, and the minimum
//! capability it needs. The palette lists them all; entries the session cannot perform stay **visible
//! but disabled with a reason** (FR-104), never hidden. This is the mechanical guarantee of
//! discoverability parity (FR-125): a future operation appears in the palette the moment it registers,
//! and the TUI/GUI share one list.
//!
//! Increment 4 seeds the registry with what exists; the `min_capability` field is exercised now (all
//! current commands are Viewer-level reads) and is the gate mutating commands will register behind.

use stikk_model::Capability;

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
    /// The minimum capability required; below it the entry is visible-but-disabled (FR-104).
    pub min_capability: Capability,
    /// The view/overlay the command opens, if it is a navigation command.
    pub opens: Option<Target>,
}

impl Command {
    /// Whether a session at `capability` may run this command.
    #[must_use]
    pub fn available_to(&self, capability: Capability) -> bool {
        capability >= self.min_capability
    }

    /// The reason this command is unavailable at `capability`, for the disabled-entry label (FR-104),
    /// or `None` when it is available. The entry stays **visible** either way (TU-07).
    #[must_use]
    pub fn unmet_reason(&self, capability: Capability) -> Option<String> {
        if self.available_to(capability) {
            None
        } else {
            Some(format!("needs {} readiness", self.min_capability.name()))
        }
    }
}

/// Every registered command (increment 4's set). Future operations add entries here.
static COMMANDS: &[Command] = &[
    Command {
        id: "view.orientation",
        name: "Go to Orientation",
        binding: "g o",
        min_capability: Capability::Viewer,
        opens: Some(Target::Orientation),
    },
    Command {
        id: "view.history",
        name: "Open History",
        binding: "Enter",
        min_capability: Capability::Viewer,
        opens: Some(Target::History),
    },
    Command {
        id: "ref.pick",
        name: "Choose ref…",
        binding: "b",
        min_capability: Capability::Viewer,
        opens: Some(Target::RefPicker),
    },
    Command {
        id: "view.glossary",
        name: "Glossary & Help",
        binding: "?",
        min_capability: Capability::Viewer,
        opens: Some(Target::Glossary),
    },
    Command {
        id: "session.refusals",
        name: "Recent refusals",
        binding: "R",
        min_capability: Capability::Viewer,
        opens: None,
    },
    Command {
        id: "view.refresh",
        name: "Refresh (re-read from prikk)",
        binding: "r",
        min_capability: Capability::Viewer,
        opens: None,
    },
    Command {
        id: "app.quit",
        name: "Quit",
        binding: "q",
        min_capability: Capability::Viewer,
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

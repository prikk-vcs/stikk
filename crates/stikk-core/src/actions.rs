//! **One catalog of every action stikk offers** (design `TU-05`/`TU-06`/`TU-07`, `FR-125`; RFC 002's
//! rules 1–4, delivered by RFC 061 decision 1).
//!
//! # Why one list
//!
//! There used to be four, and they disagreed. `palette.rs` listed 13 commands where `keys.rs` bound
//! 37 actions, and named a key — *"Go to Orientation — `g o`"* — that nothing bound, so a reader who
//! followed the palette pressed `g` and got nothing (RFC 061 F2). `TU-05` said `b` was background
//! operations, where the code had it open the ref picker, and listed a find, a copy and a pane cycle
//! that do not exist (F3). Each copy drifted because there was no single source.
//!
//! **This is the source.** The palette, the action lists ([`Where::Row`]), the key line, the
//! Glossary's *Keys* and `keys.rs`'s own key map all read it, and none keeps a list of its own. The
//! guide's key reference is prose, so it cannot be generated — a test reads it and asserts it names
//! exactly the keys bound here.
//!
//! # One letter, one action
//!
//! A letter is matched **without regard to case** outside a text prompt (RFC 061 decision 2), so
//! nine letters that used to mean one thing in lower case and another in upper — `q`/`Q`, `s`/`S`,
//! `u`/`U`, `a`/`A`, `i`/`I`, `m`/`M`, `v`/`V`, `r`/`R`, `t`/`T` — mean one thing each. Caps Lock on
//! is no longer a different keyboard. The fourteen actions that gave up their letter are found by
//! name, in the list of the row or screen they apply to and in the palette.
//!
//! `TU-06`'s *"mutating actions are uppercase"* is withdrawn: the confirmation card is what makes a
//! write deliberate (`FR-121`), and Caps Lock made the case meaningless.

use stikk_model::{Readiness, RequestCategory, StikkError, Tier};

use crate::confirm::capability_gate;

/// The reason every action that needs a focused ref gives when none is focused (RFC 029 Handoff B §2):
/// the banner a key sets and a disabled entry's reason are these same words.
pub const NO_FOCUSED_REF_REASON: &str = "No ref focused. Press b to pick one.";

/// A key the catalog binds (RFC 061 decision 2).
///
/// **A letter is one action in either case.** The named keys are here too, because the key line and
/// the action lists name them beside the letters, and a key that exists only in a renderer is the
/// drift this catalog removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A letter, matched without regard to case outside a text prompt.
    Letter(char),
    /// `Enter` — the actions for the selected row.
    Enter,
    /// `→` — the selected row's first action, at once.
    Right,
    /// `←`, `Esc` or `q` — back, and at the root, quit.
    Back,
    /// `↑` `↓`, or `k` `j`.
    Move,
    /// `?`.
    Help,
    /// `:`.
    Palette,
    /// The space bar.
    Space,
}

impl Key {
    /// What a reader sees: the letter itself, or the named key's own symbol.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Letter(c) => c.to_string(),
            Self::Enter => "Enter".to_string(),
            Self::Right => "\u{2192}".to_string(),
            // **The one a reader is most likely to reach for**, of the three that go back.
            Self::Back => "q".to_string(),
            Self::Move => "\u{2191}\u{2193}".to_string(),
            Self::Help => "?".to_string(),
            Self::Palette => ":".to_string(),
            Self::Space => "Space".to_string(),
        }
    }

    /// The letter this key is, if it is one — what `keys.rs` matches, folded to lower case.
    #[must_use]
    pub fn letter(self) -> Option<char> {
        match self {
            Self::Letter(c) => Some(c),
            _ => None,
        }
    }
}

/// A screen stikk can show — what an action means by *"here"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenKind {
    /// The Orientation root.
    Orientation,
    /// A ref's lineage.
    History,
    /// One block's files.
    BlockDetail,
    /// The worktree against the baseline.
    Changes,
    /// The patches waiting to be sealed.
    Queue,
    /// One patch.
    PatchDetail,
    /// Two points, compared.
    Compare,
    /// prikk's verdict.
    Verify,
    /// The keys this repository trusts.
    Trust,
    /// prikk's merge plan.
    MergePlan,
    /// An opened bundle.
    Bundle,
}

/// A kind of selected row — what Enter's list is *about* (RFC 061 decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A branch or tag, in the picker.
    Ref,
    /// A block, in History.
    Block,
    /// A file, in Block detail.
    File,
    /// A queued patch, in the Queue.
    QueuedPatch,
    /// A changed path, in Changes. **It has no action of its own** (RFC 061 decision 3): what `d`
    /// compares is the whole worktree, not the row.
    ChangesRow,
    /// An adopted key, in Trust.
    TrustKey,
}

/// Where an action applies. An entry may name several.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// Every screen: the navigation keys and the two operations that act on the focused ref.
    Everywhere,
    /// One screen, whatever is selected on it.
    Screen(ScreenKind),
    /// One kind of selected row — the first part of Enter's list.
    Row(RowKind),
    /// Inside a card that offers it, and nowhere else: the card names its own key, so this never
    /// reaches the key line or an action list.
    Card,
}

/// One action a user can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// A stable, kebab-case id (RFC 002 rule 1): `ref.switch`, `block.undo`. Tests and the key map
    /// name actions by this, never by their words.
    pub id: &'static str,
    /// What a reader reads, in the palette and in an action list.
    pub name: &'static str,
    /// The two or three words the key line has room for, or `None` for an action the key line never
    /// shows (one with no key, or a card's own).
    pub hint: Option<&'static str>,
    /// Its key, or `None` for an action found by name.
    pub key: Option<Key>,
    /// Everywhere it applies.
    pub applies: &'static [Where],
    /// stikk's own short name for [`capability_gate`]'s message (`"commit"`, `"seal"`) — **not**
    /// [`Self::name`]: the gate's reason already reads *"<operation> needs …"*, and the full display
    /// name there produced an overlong disabled line.
    pub operation: &'static str,
    /// The request category, or `None` for an action that makes no prikk request of its own. **The
    /// category, not the tier** (RFC 045 §2b): the tier and the signing gate are both read off it, so
    /// an entry cannot disagree with the `Intent` the operation itself carries.
    pub category: Option<RequestCategory>,
    /// Whether it acts on the focused ref (RFC 029 Handoff B §2). With none focused it stays visible
    /// and disabled, with [`NO_FOCUSED_REF_REASON`].
    pub needs_focused_ref: bool,
    /// Its place in the key line (RFC 061 decision 5): the line drops **whole entries, lowest rank
    /// first**. `255` never drops; `0` never appears.
    pub rank: u8,
}

impl Action {
    /// The confirmation tier this action requires (RFC 013 decision 4), from its category —
    /// [`Tier::One`] for an action that makes no prikk request at all.
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.category.map_or(Tier::One, RequestCategory::tier)
    }

    /// Whether a session with `readiness` may run this — the same check
    /// [`crate::confirm::confirm`] enforces (RFC 014 §6), so nothing offered is refused there.
    #[must_use]
    pub fn available_to(&self, readiness: Readiness) -> bool {
        self.category
            .is_none_or(|category| capability_gate(self.operation, category, readiness).is_ok())
    }

    /// The reason this is unavailable for `readiness`, for the disabled label (`FR-104`), or `None`.
    /// The entry stays **visible** either way (`TU-07`).
    #[must_use]
    pub fn unmet_reason(&self, readiness: Readiness) -> Option<String> {
        let category = self.category?;
        match capability_gate(self.operation, category, readiness) {
            Ok(()) => None,
            Err(StikkError::NotReady { detail }) => Some(detail),
            // `capability_gate` only ever returns `NotReady`; kept exhaustive (not `_`) so a change
            // to that contract fails here loudly rather than swallowing a new error shape.
            Err(other) => Some(other.to_string()),
        }
    }

    /// Whether this may run now: [`Self::available_to`], and a focused ref where it needs one.
    #[must_use]
    pub fn available(&self, readiness: Readiness, ref_focused: bool) -> bool {
        self.unavailable_reason(readiness, ref_focused).is_none()
    }

    /// Why this cannot run now, or `None` when it can.
    ///
    /// **Capability first.** A session that cannot commit at all is told that, rather than to choose
    /// a ref it still could not commit to.
    #[must_use]
    pub fn unavailable_reason(&self, readiness: Readiness, ref_focused: bool) -> Option<String> {
        self.unmet_reason(readiness).or_else(|| {
            (self.needs_focused_ref && !ref_focused).then(|| NO_FOCUSED_REF_REASON.to_string())
        })
    }

    /// Whether it applies in one place.
    #[must_use]
    pub fn applies_to(&self, place: Where) -> bool {
        self.applies.contains(&place)
    }
}

/// Everywhere.
const ANY: &[Where] = &[Where::Everywhere];

/// **Every action stikk offers.** The order is the order a reader meets them: the keys that move
/// around first, then the views, then the operations, then what belongs to one row or one screen.
static ACTIONS: &[Action] = &[
    // ----- moving around (RFC 061 decision 2's table) -----
    Action {
        id: "row.actions",
        name: "What can be done here",
        hint: Some("actions"),
        key: Some(Key::Enter),
        applies: ANY,
        operation: "actions",
        category: None,
        needs_focused_ref: false,
        // Dropped only after every other entry but help and back (decision 5).
        rank: 254,
    },
    Action {
        id: "row.open",
        name: "Open the selected row",
        // Never in the key line: `Enter actions` is how a reader finds it, and the list's first
        // entry says the arrow beside itself.
        hint: None,
        key: Some(Key::Right),
        applies: ANY,
        operation: "open",
        category: None,
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "app.move",
        name: "Move the selection",
        hint: Some("move"),
        key: Some(Key::Move),
        applies: ANY,
        operation: "move",
        category: None,
        needs_focused_ref: false,
        rank: 200,
    },
    // ----- the views -----
    Action {
        id: "ref.pick",
        name: "Branches and tags",
        hint: Some("branches"),
        key: Some(Key::Letter('b')),
        applies: ANY,
        operation: "ref-pick",
        category: Some(RequestCategory::ReadHistory),
        needs_focused_ref: false,
        rank: 180,
    },
    Action {
        id: "view.changes",
        name: "Changes in the worktree",
        hint: Some("changes"),
        key: Some(Key::Letter('w')),
        applies: ANY,
        operation: "changes",
        category: Some(RequestCategory::WorktreeAnalysis),
        needs_focused_ref: true,
        rank: 170,
    },
    Action {
        id: "op.commit",
        name: "Commit worktree changes",
        hint: Some("commit"),
        key: Some(Key::Letter('c')),
        // **One of Changes' own actions as well** (RFC 061 decision 3): a Changes row has no action
        // of its own, so its list is what Changes offers.
        applies: &[Where::Everywhere, Where::Screen(ScreenKind::Changes)],
        operation: "commit",
        category: Some(RequestCategory::QueueMutation),
        needs_focused_ref: true,
        rank: 160,
    },
    Action {
        id: "op.seal",
        name: "Seal the active WAL",
        hint: Some("seal"),
        key: Some(Key::Letter('s')),
        applies: ANY,
        operation: "seal",
        category: Some(RequestCategory::Publication),
        needs_focused_ref: true,
        rank: 150,
    },
    Action {
        id: "view.queue",
        name: "Patches waiting to be sealed",
        hint: Some("patches"),
        key: Some(Key::Letter('p')),
        applies: ANY,
        operation: "queue",
        category: Some(RequestCategory::ReadHistory),
        // The queue is repository-wide, so it needs no focused ref (RFC 028 Handoff A §6).
        needs_focused_ref: false,
        rank: 140,
    },
    Action {
        id: "view.verify",
        name: "Verify the repository",
        hint: Some("verify"),
        key: Some(Key::Letter('v')),
        applies: ANY,
        operation: "verify",
        // **`Integrity`, which is Tier One: it reads and changes nothing** (RFC 040 decision 4). A
        // repository too damaged to commit in is exactly where a reader most needs to run it.
        category: Some(RequestCategory::Integrity),
        needs_focused_ref: false,
        rank: 130,
    },
    Action {
        id: "view.trust",
        name: "Keys this repository trusts",
        hint: Some("trust"),
        key: Some(Key::Letter('t')),
        applies: ANY,
        operation: "trust",
        // The read, which is Tier One; adopting and removing carry `Trust`'s own category below.
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 120,
    },
    Action {
        id: "bundle.open",
        name: "Open a bundle file",
        hint: Some("bundle"),
        key: Some(Key::Letter('o')),
        applies: ANY,
        operation: "bundle-open",
        // **Reads only**: nothing is imported and nothing is written (RFC 050 decision 3).
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 110,
    },
    Action {
        id: "view.refresh",
        name: "Refresh (re-read from prikk)",
        hint: Some("refresh"),
        key: Some(Key::Letter('r')),
        applies: ANY,
        operation: "refresh",
        category: Some(RequestCategory::ReadHistory),
        needs_focused_ref: false,
        rank: 100,
    },
    Action {
        id: "view.palette",
        name: "Every action, by name",
        hint: Some("all actions"),
        key: Some(Key::Palette),
        applies: ANY,
        operation: "palette",
        category: None,
        needs_focused_ref: false,
        rank: 70,
    },
    Action {
        id: "view.glossary",
        name: "Glossary & Help",
        hint: Some("help"),
        key: Some(Key::Help),
        applies: ANY,
        operation: "glossary",
        category: None,
        needs_focused_ref: false,
        // **Never dropped** (RFC 061 decision 5): the one key that explains the others.
        rank: 255,
    },
    Action {
        id: "app.back",
        name: "Back",
        hint: Some("back"),
        key: Some(Key::Back),
        applies: ANY,
        operation: "back",
        category: None,
        needs_focused_ref: false,
        // **Never dropped**: the way out.
        rank: 255,
    },
    // ----- a ref, in the picker (RFC 061 decision 3) -----
    Action {
        id: "ref.open-history",
        name: "Open its history",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "history",
        category: Some(RequestCategory::ReadHistory),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.switch",
        name: "Switch to this branch",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "switch",
        category: Some(RequestCategory::Materialization),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.merge-plan",
        name: "Merge plan: the marked branch into this one",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "merge-plan",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.branch",
        name: "New branch at its tip",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "create-branch",
        category: Some(RequestCategory::Publication),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.tag",
        name: "New tag at its tip",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "create-tag",
        category: Some(RequestCategory::Publication),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.close",
        name: "Close this branch",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "close-branch",
        category: Some(RequestCategory::Publication),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "ref.export",
        name: "Export it to a bundle file",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Ref)],
        operation: "export",
        category: Some(RequestCategory::Exchange),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- a block, in History -----
    Action {
        id: "block.open",
        name: "Open this block",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::Block)],
        operation: "block-detail",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "block.mark",
        name: "Mark it, to compare",
        hint: Some("mark"),
        key: Some(Key::Letter('m')),
        // **The picker's `m` marks a ref, and History's marks a block** (RFC 041 decision 1): one
        // key, one meaning — *mark this row* — and the two marks are stored apart.
        applies: &[Where::Row(RowKind::Block), Where::Row(RowKind::Ref)],
        operation: "mark",
        category: None,
        needs_focused_ref: false,
        rank: 85,
    },
    Action {
        id: "op.compare",
        name: "Compare",
        hint: Some("compare"),
        key: Some(Key::Letter('d')),
        // What is compared depends on where you are (`FR-033`; RFC 038 decision 3): the marked block
        // against the selected one, or the worktree against what `commit` would author against.
        applies: &[
            Where::Row(RowKind::Block),
            Where::Screen(ScreenKind::Changes),
        ],
        operation: "compare",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 90,
    },
    Action {
        id: "block.undo",
        name: "Undo this block",
        hint: None,
        key: None,
        applies: &[
            Where::Row(RowKind::Block),
            Where::Screen(ScreenKind::BlockDetail),
        ],
        operation: crate::undo::UNDO_OPERATION,
        // **Commit's category, because commit's is the gate this operation takes** (RFC 058 §3.2):
        // the undo ends in a `commit` and needs AUTHOR signing readiness, which is what this entry
        // must say when it greys out. The operation's own category is `Materialization`, carried by
        // the `Intent` inside `undo_preview`.
        category: Some(RequestCategory::QueueMutation),
        needs_focused_ref: true,
        rank: 0,
    },
    Action {
        id: "block.tag",
        name: "New tag at this block",
        hint: None,
        key: None,
        applies: &[
            Where::Row(RowKind::Block),
            Where::Screen(ScreenKind::BlockDetail),
        ],
        operation: "create-tag",
        category: Some(RequestCategory::Publication),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- a file, in Block detail -----
    Action {
        id: "file.open-patch",
        name: "Open the patch that wrote it",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::File)],
        operation: "patch-detail",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "file.save",
        name: "Save this file, as it was here",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::File)],
        operation: "save-file",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "file.save-all",
        name: "Save every file at this block",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::File)],
        operation: "save-all",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- a queued patch, in the Queue -----
    Action {
        id: "patch.open",
        name: "Open this patch",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::QueuedPatch)],
        operation: "patch-detail",
        category: Some(RequestCategory::ReadState),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- Changes -----
    Action {
        id: "changes.untracked",
        name: "Show untracked files",
        hint: Some("untracked"),
        key: Some(Key::Letter('u')),
        applies: &[Where::Screen(ScreenKind::Changes)],
        operation: "untracked",
        category: None,
        needs_focused_ref: false,
        rank: 75,
    },
    // ----- Trust -----
    Action {
        id: "trust.remove",
        name: "Remove this key",
        hint: None,
        key: None,
        applies: &[Where::Row(RowKind::TrustKey)],
        operation: "remove-key",
        category: Some(RequestCategory::Trust),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "trust.adopt",
        name: "Adopt a key",
        hint: None,
        key: None,
        applies: &[Where::Screen(ScreenKind::Trust)],
        operation: "adopt-key",
        category: Some(RequestCategory::Trust),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- the Bundle view -----
    Action {
        id: "bundle.import",
        name: "Import this bundle",
        hint: None,
        key: None,
        applies: &[Where::Screen(ScreenKind::Bundle)],
        operation: "import",
        category: Some(RequestCategory::Exchange),
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- prikk's own fields, where a view has them -----
    Action {
        id: "view.raw",
        name: "prikk's own fields",
        hint: Some("fields"),
        key: Some(Key::Letter('f')),
        applies: &[
            Where::Screen(ScreenKind::PatchDetail),
            Where::Screen(ScreenKind::Compare),
            Where::Screen(ScreenKind::Verify),
        ],
        operation: "raw",
        category: None,
        needs_focused_ref: false,
        rank: 80,
    },
    // ----- found by name -----
    Action {
        id: "view.orientation",
        name: "Go to Orientation",
        hint: None,
        key: None,
        applies: ANY,
        operation: "orientation",
        category: Some(RequestCategory::ReadHistory),
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "view.history",
        name: "Open History",
        hint: None,
        key: None,
        applies: ANY,
        operation: "history",
        category: Some(RequestCategory::ReadHistory),
        needs_focused_ref: true,
        rank: 0,
    },
    Action {
        id: "view.operations",
        name: "Background operations",
        hint: None,
        key: None,
        applies: ANY,
        operation: "operations",
        category: None,
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "session.refusals",
        name: "Recent refusals",
        hint: None,
        key: None,
        applies: ANY,
        operation: "refusals",
        category: None,
        needs_focused_ref: false,
        rank: 0,
    },
    Action {
        id: "app.quit",
        name: "Quit",
        hint: None,
        key: None,
        applies: ANY,
        operation: "quit",
        category: None,
        needs_focused_ref: false,
        rank: 0,
    },
    // ----- a card's own key -----
    Action {
        id: "seal.consent",
        name: "Acknowledge, so the seal may run with no audit",
        hint: None,
        key: Some(Key::Space),
        // **Only inside the card that offers it** (RFC 016 §8): the card names the key itself, so
        // this never reaches the key line or an action list.
        applies: &[Where::Card],
        operation: "seal-consent",
        category: None,
        needs_focused_ref: false,
        rank: 0,
    },
];

/// Every action, in the order a reader meets them.
#[must_use]
pub fn all() -> &'static [Action] {
    ACTIONS
}

/// The action a letter runs, **ignoring case** (RFC 061 decision 2) — `keys.rs`'s whole key map for
/// letters.
#[must_use]
pub fn by_letter(letter: char) -> Option<&'static Action> {
    let folded = letter.to_ascii_lowercase();
    ACTIONS
        .iter()
        .find(|action| action.key.and_then(Key::letter) == Some(folded))
}

/// The action a named key runs.
#[must_use]
pub fn by_key(key: Key) -> Option<&'static Action> {
    ACTIONS.iter().find(|action| action.key == Some(key))
}

/// The one action with this id.
#[must_use]
pub fn by_id(id: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|action| action.id == id)
}

/// Every action that applies in one place, in catalog order.
#[must_use]
pub fn applying_to(place: Where) -> Vec<&'static Action> {
    ACTIONS
        .iter()
        .filter(|action| action.applies_to(place))
        .collect()
}

/// Actions whose name matches `filter` (case-insensitive substring; empty matches all) — the
/// palette's filter. Order is preserved so the list is stable; **disabled entries are included**
/// (the caller styles them, `FR-104`), and a card's own key is left out, since it belongs to a card
/// and not to a screen.
#[must_use]
pub fn matching(filter: &str) -> Vec<&'static Action> {
    let needle = filter.to_ascii_lowercase();
    ACTIONS
        .iter()
        .filter(|action| !action.applies_to(Where::Card))
        .filter(|action| needle.is_empty() || action.name.to_ascii_lowercase().contains(&needle))
        .collect()
}

#[cfg(test)]
mod tests;

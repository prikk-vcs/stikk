//! Undoing one block: putting back what it changed, as a new change (design `FR-085`; RFC 058).
//!
//! **The first stikk operation that writes worktree files itself** (decision 7). prikk has no
//! single-block undo and does not plan one, so stikk assembles it from four things prikk does
//! give: `diff` says what the block changed, `cat --output` gives the earlier bytes, `mv` renames
//! a path back, and `commit` records the result. Nothing is erased — the undo is a new change
//! after the block, and History shows both (`CON-3`).
//!
//! Four things are worth reading here.
//!
//! - **The plan is the inverse of prikk's own comparison, entry by entry** (F2, measured at
//!   0.54.1 and 0.55.0): `added` is removed, `deleted`/`modified`/`binary`/`mode` are written back
//!   with the parent's bytes and the parent's mode, and `renamed` is renamed back. Inverting one
//!   measured block that did all of those gave a tip **identical to its parent — every path,
//!   every mode, every byte** — and `verify` passed. The mode is the diff entry's own `from.mode`;
//!   stikk reads no `tree` for it.
//! - **Seven guards, in RFC 058 decision 4's order, all inside [`crate::confirm::preview`]'s
//!   `compute`** — so they are taken again at the moment the confirmation is armed, and each
//!   declines with **nothing written** and no token minted (`C-T4d`).
//! - **The order of execution is the safety argument** (decision 6). Every earlier file is fetched
//!   into a fresh directory outside the repository *before* the worktree is touched at all, so a
//!   failed fetch has changed nothing. A failure after that stops at once, commits nothing, and
//!   says which paths it changed.
//! - **Every path stikk writes comes from prikk's own report**, and is checked before any of them
//!   is used: relative, no `..`, nothing rooted or prefixed, inside the root, and never under
//!   `.prikk/`. A plan that exists has passed that check — [`UndoPlan::from_diff`] is the only way
//!   to build one.
//!
//! **The parent is the row below the block in its branch's own `log`** (F4), never
//! `previous_ref_state`, which is a RefState and not a block. prikk 0.57.0 reports
//! `parent_block_ids` itself; a later re-baseline reads it from there.

use std::path::{Path, PathBuf};

use stikk_model::{Capability, RequestCategory, Result, StikkError, Tier};
use stikk_prikk::{BlockRow, Comparison, Diff, DiffReport, FileSaved, Prikk, Renamed};

use crate::changes::{self, ChangeKind};
use crate::confirm::{
    self, Aside, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken, TargetId,
};

/// stikk's own short name for the operation, used in `Intent` and in stale/declined messages.
pub const UNDO_OPERATION: &str = "undo-block";

/// The first prikk minor that offers an undo, re-exported so a view says the number the seam gates
/// on — the one-constant rule every other band follows.
///
/// It is `cat --output`'s own band (decision 2): an undo cannot be assembled without the earlier
/// bytes, and below 0.54 there is no way to ask prikk for them.
pub use stikk_prikk::FILES_FROM_MINOR as UNDO_FROM_MINOR;

/// How deep the undo reads the current branch's lineage to find the block and its parent.
///
/// **The depth History's own view reads** (`stikk-tui`'s `HISTORY_LIMIT`), so a block a reader can
/// select is a block the undo can find. The two numbers are the same number on purpose; if one
/// moves, the other must.
pub const UNDO_HISTORY_LIMIT: usize = 200;

/// **stikk's own reason, below the band** (decision 2; `C-T4d`), given the minor prikk reported.
#[must_use]
pub fn undo_unavailable(minor: u32) -> String {
    format!(
        "Undoing a block needs prikk ≥ 0.{UNDO_FROM_MINOR} — this prikk is 0.{minor}. Below \
         0.{UNDO_FROM_MINOR} stikk cannot ask prikk for the earlier bytes, and an undo cannot be \
         assembled without them. The rest of stikk works."
    )
}

/// **Why an undo is offered on the current branch only** (decision 3).
pub const NOT_THE_CURRENT_BRANCH: &str = "Undo works on the branch you are on. Switch to it first.";

/// **Guard 1** (decision 4): the worktree holds changes an undo could overwrite.
pub const WORKTREE_HAS_CHANGES: &str = "Undo needs a clean worktree, so it cannot overwrite your work. Commit or remove your \
     changes first.";

/// **Guard 2**: work is queued, and the undo must be recorded after it.
pub const WORK_IS_QUEUED: &str =
    "Work is waiting to be sealed. Seal it first, so the undo is recorded after it.";

/// **Guard 3**: a merge has two parents, and `log` names only one of them (F4).
pub const A_MERGE_BLOCK: &str = "stikk cannot undo a merge yet.";

/// **Guard 4**: no row lies below this block in its branch's history (F4).
pub const THE_FIRST_BLOCK: &str =
    "This is the first block on this branch, so there is nothing before it to go back to.";

/// **Guard 4's other half**, which the RFC's sentence would have claimed wrongly.
///
/// A block sitting at the bottom of a lineage that came back **full** is not knowably the first
/// block on its branch — stikk has only run out of the history it asked for
/// ([`UNDO_HISTORY_LIMIT`]). Saying *"this is the first block"* there would be stikk stating a
/// fact it does not have, so it says what is true instead. [`crate::merge_baseline`] reads a full
/// lineage the same way, and for the same reason.
pub const DEEPER_THAN_STIKK_LOOKED: &str = "stikk read the newest blocks of this branch and this is the oldest of them, so it cannot \
     see what comes before it. Undo a newer block, or use prikk's own commands.";

/// **Why stikk will not offer an undo while a materialization is interrupted.**
///
/// Not one of decision 4's seven: it is a precondition of the *commit* the undo ends in. prikk's
/// own `commit` refuses while the marker is set (RFC 034 §7.2), so an undo offered here would
/// write the whole worktree and then fail at the last step — writes for nothing. The routes out
/// are prikk's own, carried verbatim by [`crate::interrupted_sentence`].
pub const INTERRUPTED: &str = "An interrupted materialization must be finished before an undo, because the commit it ends \
     in would refuse.";

/// The two sentences the card opens with (decision 5).
pub const UNDO_LEAD: &str =
    "This adds a new change that puts back what this block changed. History keeps both.";

/// The card's consequence, in stikk's own words and naming nothing (RFC 044 C1).
pub const UNDO_CONSEQUENCE: &str =
    "After this, seal as usual to make the undo part of the history.";

/// **What the result says after a recorded undo** (decision 5): the undo is a commit, and a commit
/// is not history until it is sealed. Names the key, as every other next step in stikk does.
pub const SEAL_THE_UNDO: &str = "Seal as usual (`S`) to make the undo part of the history.";

/// **What the result says after a stop** (decision 6): the worktree holds the changes and nothing
/// was recorded, so the next thing to look at is Changes.
pub const NOTHING_WAS_COMMITTED: &str = "Nothing was committed. Changes (`w`) shows them.";

/// The message an undo is prefilled with (decision 5): `Undo: ` and the block's **first** patch
/// message, which is prikk's text and is rendered inert like any other.
///
/// A block sealed entirely below prikk 0.32 carries no message at all (RFC 015 F4) -- the prompt
/// then opens with the lead alone, for the user to finish, rather than with a sentence stikk made
/// up about a block it cannot describe.
#[must_use]
pub fn prefilled_undo_message(block: &BlockRow) -> String {
    match block.messages.first() {
        Some(first) => format!("Undo: {}", first.message),
        None => "Undo: ".to_string(),
    }
}

/// **The one line a stopped undo shows of its cause** -- prikk's own words where they are prikk's,
/// and stikk's own where the filesystem failed (`ER-02`).
///
/// Not [`crate::present()`]'s card: nothing was refused *to* the user here. The undo ran, stopped
/// part-way, and this sentence sits under the paths it had already changed, where a next-step list
/// about re-previewing would be wrong -- there is nothing to re-preview until the worktree is
/// looked at.
#[must_use]
pub fn stopped_text(cause: &StikkError) -> String {
    match cause {
        StikkError::Refusal { message }
        | StikkError::LockConflict { message }
        | StikkError::CrossRef { message }
        | StikkError::Limits { message }
        | StikkError::IntegrityFinding { message } => message.clone(),
        StikkError::NotReady { detail }
        | StikkError::Internal { detail }
        | StikkError::Declined { detail }
        | StikkError::Environment { detail, .. } => detail.clone(),
        // `StikkError` is `#[non_exhaustive]`, and `Stale` cannot be reached from a stop -- the
        // token checks run before step 1. Whatever arrives here still says something true.
        other => other.to_string(),
    }
}

/// **Guard 5** (F3): the paths a later block changed too, named — every one, sorted.
///
/// A plain inverse would overwrite that later change, and the measurement shows exactly that: with
/// `three` changing `a.txt` again, undoing `two` would put `one`'s `a.txt` back over it.
#[must_use]
pub fn later_block_changed(paths: &[String]) -> String {
    match paths {
        [one] => format!(
            "A later block changed `{one}` too. Undoing this one would overwrite that change."
        ),
        many => format!(
            "A later block changed these paths too: {}. Undoing this one would overwrite those \
             changes.",
            backticked(many)
        ),
    }
}

/// **Guard 6**: a file the undo would write is already in the worktree, untracked.
#[must_use]
pub fn in_the_way(paths: &[String]) -> String {
    match paths {
        [one] => format!(
            "`{one}` is in the way: it is not part of this branch, and the undo would replace it."
        ),
        many => format!(
            "These are in the way: {}. They are not part of this branch, and the undo would \
             replace them.",
            backticked(many)
        ),
    }
}

/// **Guard 7**: an entry stikk cannot invert exactly, with the reason naming the path.
#[must_use]
pub fn cannot_invert(reason: &str) -> String {
    format!("stikk cannot undo this block exactly: {reason}.")
}

/// **The path check's refusal** (decision 6): a path stikk will not join to the repository root.
///
/// It refuses the **whole** undo rather than that one entry, because a plan with a path removed
/// from it would no longer be the inverse of the block.
#[must_use]
pub fn unsafe_path(path: &str) -> String {
    format!(
        "stikk will not undo this block: prikk's comparison names a path stikk will not write, \
         `{path}`. A path stikk writes must be inside this worktree, must not climb out of it, and \
         must not be in `.prikk`."
    )
}

/// Repository paths in a stikk sentence, each in backticks, in the order given.
fn backticked(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What the undo does to one path — stikk's inverse of one of prikk's entries (F2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoAction {
    /// Put the parent's bytes and mode back (`deleted`, `modified`, `binary`).
    PutBack,
    /// Put the parent's mode back (`mode`). The bytes go back too — they are the same bytes, and
    /// prikk's `cat --output` is how a mode is applied at all at 0.55 (F2) — but the card says
    /// what actually differs, which is the permissions.
    PutBackPermissions,
    /// Remove the file the block added (`added`).
    Remove,
    /// Rename it back, to the name the parent holds it under (`renamed`), **and put the parent's
    /// bytes and mode back under that name**.
    ///
    /// F2 writes the bytes only *"if the content changed too"*, and stikk writes them always,
    /// because the report cannot be asked which happened: a text side carries no `content_id`,
    /// and two different texts of the same length are an ordinary thing. Writing them is exact
    /// either way — they are the parent's own bytes at the parent's own path — and the measured
    /// undo block still records a rename as a rename, because identical bytes author no content
    /// operation.
    RenameBack {
        /// The parent's path for this file — prikk's `from_path`.
        earlier: String,
    },
}

/// One step of an undo: what to do, and the path the block holds it at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoStep {
    /// The inverse to apply.
    pub action: UndoAction,
    /// The path **as the block has it** — prikk's `path`. Repository text (`C-T2a`).
    pub path: String,
    /// The parent's full mode for this path, from the diff entry's own `from.mode` (F2, as
    /// clarified at acceptance) — `None` on [`UndoAction::Remove`], where there is nothing to put
    /// back and prikk reports no earlier side.
    pub mode: Option<u32>,
}

impl UndoStep {
    /// stikk's own verb for this step, for the card (decision 5).
    #[must_use]
    pub fn verb(&self) -> &'static str {
        match self.action {
            UndoAction::PutBack => "put back",
            UndoAction::PutBackPermissions => "put back the permissions",
            UndoAction::Remove => "remove",
            UndoAction::RenameBack { .. } => "rename back",
        }
    }

    /// The path this file has **at the parent** — both where prikk fetches its bytes from and
    /// where they go back to. The block's own path, except for a rename, where it is the earlier
    /// one.
    #[must_use]
    pub fn parent_path(&self) -> &str {
        match &self.action {
            UndoAction::RenameBack { earlier } => earlier,
            UndoAction::PutBack | UndoAction::PutBackPermissions | UndoAction::Remove => &self.path,
        }
    }

    /// Whether this step needs the parent's bytes fetched for it.
    #[must_use]
    pub fn puts_bytes_back(&self) -> bool {
        match self.action {
            UndoAction::PutBack
            | UndoAction::PutBackPermissions
            | UndoAction::RenameBack { .. } => true,
            UndoAction::Remove => false,
        }
    }
}

/// Every step of one undo, in prikk's own entry order.
///
/// **A plan that exists has passed the path check** (decision 6): [`Self::from_diff`] is the only
/// constructor, every path in the plan went through [`unsafe_path`]'s rules there, and the token
/// carries the plan the preview built — no later step re-derives it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UndoPlan {
    /// The steps, in the order prikk listed the entries.
    pub steps: Vec<UndoStep>,
}

impl UndoPlan {
    /// Invert `diff` — prikk's own `parent`→`block` comparison — into a plan, or say why stikk
    /// cannot (**guard 7**, then the path check).
    ///
    /// The `Err` is the sentence to show, already in stikk's words: [`cannot_invert`] for anything
    /// that would not come out exact, and [`unsafe_path`] for a path stikk will not write.
    ///
    /// **Windows refuses two of F2's rows** (F5): prikk's worktree authoring there *"never derives
    /// a node's recorded mode"*, so a `mode` entry has no inverse at all, and putting back a
    /// deleted file whose recorded mode is executable would record it non-executable. Both are
    /// refused rather than done approximately.
    ///
    /// # Errors
    /// The sentence to show, never a [`StikkError`]: neither case is a fault or a refusal — they
    /// are ordinary blocked previews (`C-T4d`).
    pub fn from_diff(repo: &Path, diff: &Diff) -> std::result::Result<Self, String> {
        // **prikk could not look at a path**, which is not the same fact as having looked and
        // found nothing (`C-T2c′`). prikk's own refusal names it, verbatim (`ER-02`).
        if let Some(unsupported) = diff.unsupported_paths.first() {
            return Err(cannot_invert(&format!(
                "prikk could not compare `{}` — {}",
                unsupported.path, unsupported.refusal
            )));
        }
        let mut steps = Vec::with_capacity(diff.entries.len());
        for entry in &diff.entries {
            let path = entry.path.clone();
            // The parent's side, which carries the mode F2 applies. Absent exactly on `added`,
            // where there is nothing earlier; absent anywhere else is a report stikk cannot
            // invert, not a mode to guess at.
            let earlier_mode = entry.from.as_ref().map(|file| file.mode);
            let action = match entry.status.as_str() {
                "added" => UndoAction::Remove,
                "deleted" | "modified" | "binary" => UndoAction::PutBack,
                "mode" => UndoAction::PutBackPermissions,
                "renamed" => match entry.from_path.clone() {
                    Some(earlier) => UndoAction::RenameBack { earlier },
                    None => {
                        return Err(cannot_invert(&format!(
                            "prikk reports `{path}` as renamed without naming where it came from"
                        )));
                    }
                },
                // A status word stikk has not measured — `symlink` among them, which prikk's own
                // `diff.rs` calls *"unreachable today"*. Compare shows it, because there a word
                // about a difference costs a reader nothing; here it would be a hole in the
                // inverse, and the inverse has to be whole.
                other => {
                    return Err(cannot_invert(&format!(
                        "prikk reports `{path}` as `{other}`, which stikk has not measured"
                    )));
                }
            };
            if action != UndoAction::Remove && earlier_mode.is_none() {
                return Err(cannot_invert(&format!(
                    "prikk's comparison carries no earlier file for `{path}`"
                )));
            }
            if cfg!(windows)
                && let Some(reason) = windows_refusal(entry.status.as_str(), &path, earlier_mode)
            {
                return Err(cannot_invert(&reason));
            }
            steps.push(UndoStep {
                // **No mode on a removal.** prikk reports no earlier side for an `added` entry at
                // all, and a mode carried on a step that writes nothing could only mislead.
                mode: match action {
                    UndoAction::Remove => None,
                    UndoAction::PutBack
                    | UndoAction::PutBackPermissions
                    | UndoAction::RenameBack { .. } => earlier_mode,
                },
                action,
                path,
            });
        }
        // **The path check, before any token exists** (decision 6). Every path the plan would
        // touch, the rename's earlier name included.
        for step in &steps {
            for path in [step.path.as_str(), step.parent_path()] {
                if !writable_path(repo, path) {
                    return Err(unsafe_path(path));
                }
            }
        }
        Ok(Self { steps })
    }

    /// True when the plan has no steps — a comparison with no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// **F5's two rows, on Windows only.** `cfg!(windows)` decides whether this is asked, so the
/// function itself is testable on every platform.
fn windows_refusal(status: &str, path: &str, earlier_mode: Option<u32>) -> Option<String> {
    let executable = earlier_mode.is_some_and(|mode| mode & 0o111 != 0);
    match status {
        "mode" => Some(format!(
            "`{path}` changes only its permissions, and prikk cannot record a mode from the \
             worktree on Windows"
        )),
        "deleted" if executable => Some(format!(
            "putting `{path}` back needs its executable permission, and prikk cannot record a \
             mode from the worktree on Windows"
        )),
        _ => None,
    }
}

/// **Whether stikk will join `path` to `repo` and write it** (decision 6).
///
/// Relative, every component an ordinary name — so no `..`, no root, no Windows prefix and no
/// bare `.` — the first component not `.prikk`, and the join still inside the root. The last two
/// follow from the first on every platform stikk runs on; they are checked anyway, because this
/// is the one function standing between prikk's report and stikk's own `fs::write`.
fn writable_path(repo: &Path, path: &str) -> bool {
    use std::path::Component;

    let candidate = Path::new(path);
    let mut components = candidate.components().peekable();
    if components.peek().is_none() {
        return false;
    }
    for component in candidate.components() {
        if !matches!(component, Component::Normal(_)) {
            return false;
        }
    }
    if candidate
        .components()
        .next()
        .is_some_and(|first| first.as_os_str() == PRIKK_METADATA_DIR)
    {
        return false;
    }
    repo.join(candidate).starts_with(repo)
}

/// prikk's own metadata directory, which stikk never writes into (`CON-1`, `C-E2`).
const PRIKK_METADATA_DIR: &str = ".prikk";

/// What an undo will do, as the card restates it (decision 5).
///
/// **stikk's own derivation from prikk's `diff`, and labelled so** (RFC 014's own posture): prikk
/// has no plan for this operation, so the card says whose plan this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoPreview {
    /// The block being undone, as prikk's `log` reported it. Repository text (`C-T2a`).
    pub block_id: String,
    /// The parent — the row below it (F4).
    pub parent_id: String,
    /// The branch the undo will be committed to: prikk's current branch (decision 3).
    pub reff: String,
    /// The block's patch messages, prikk's text verbatim. Empty for a block sealed entirely below
    /// prikk 0.32, which is normal, not an error.
    pub messages: Vec<String>,
    /// The inverse, step by step.
    pub plan: UndoPlan,
    /// The commit message the undo will carry — **the one string here the user wrote**, prefilled
    /// by the frontend and editable there (decision 5).
    pub message: String,
    /// The key id that will sign the commit, if one is configured (`FL-05` step 5), read in the
    /// preview exactly as commit's card reads it.
    pub signing_key_id: Option<String>,
    /// What the card may honestly claim about [`Self::signing_key_id`] (RFC 026 §5).
    pub signing_key_claim: confirm::KeyClaim,
    /// Set when that key is one prikk publishes as an example (`C-S2`).
    pub signing_key_is_published_example: bool,
}

impl UndoPreview {
    /// The fact set the confirmation restates — the card's lines, in decision 5's order: the two
    /// sentences, the block id and its messages, the plan, then the consequence.
    #[must_use]
    pub fn summary(&self) -> ConfirmationSummary {
        ConfirmationSummary {
            operation: "Undo this block".to_string(),
            preamble: Some(UNDO_LEAD.to_string()),
            target_ids: vec![TargetId::bare(self.block_id.clone())],
            // **No counts line.** The plan names every path with its own verb, and "3 paths"
            // above it would be a number for a list the reader is already looking at.
            counts: Vec::new(),
            // **AUTHOR, because the undo ends in a commit** (decision 5; `FR-050`).
            capability: Some(Capability::Author),
            consequence: UNDO_CONSEQUENCE.to_string(),
            target_name: None,
            signing_key_id: self.signing_key_id.clone(),
            signing_key_claim: self.signing_key_claim,
            signing_key_is_published_example: self.signing_key_is_published_example,
            branch_notice: None,
            freezes: None,
            history_notice: None,
            rename_note: None,
            declaration_notices: Vec::new(),
            notices: Vec::new(),
            // **The message is not restated here** (decision 5's list, and commit's own card):
            // the prompt that collected it was the step before this one, and the card's rows are
            // the plan. Restating it cost the card four rows, which at 80x24 pushed the plan's
            // own rows off the bottom -- measured, not assumed.
            message_note: None,
            materializes: None,
            merge_plan: None,
            undo_plan: Some(self.plan.clone()),
            // The block's messages, under the id they belong to and apart from the targets: a row
            // under `Targets:` reads as something the operation acts on, and a patch message is
            // not that (RFC 050 review v1 C1).
            asides: if self.messages.is_empty() {
                Vec::new()
            } else {
                vec![Aside {
                    before: "Its messages:",
                    name: String::new(),
                    after: "",
                    body: self.messages.clone(),
                }]
            },
        }
    }
}

/// What [`undo_preview`] found.
#[derive(Debug)]
pub enum UndoPreviewOutcome {
    /// stikk will not undo this block, and says why in its own voice — **no token exists for this
    /// outcome; nothing was armed** (`C-T4d`).
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The preview to restate before the user confirms. Boxed to keep the variants close in
        /// size (`clippy::large_enum_variant`).
        preview: Box<UndoPreview>,
        /// Feed this into [`undo_confirm_and_execute`].
        token: Box<UndoToken>,
    },
}

/// An undo's armed confirmation: the preview's [`PreviewToken`], **and the plan, the parent, the
/// ref and the message it was built for**, all private.
///
/// **Private because a substitute has no correct meaning.** A caller that could hand in another
/// plan would write paths no preview ever showed, and the path check is the preview's own
/// ([`UndoPlan::from_diff`]). The only way to get one is [`undo_preview`].
#[derive(Debug)]
pub struct UndoToken {
    preview: PreviewToken,
    reff: String,
    parent_id: String,
    plan: UndoPlan,
    message: String,
}

impl UndoToken {
    /// The tier this confirmation must satisfy — Tier Three, like a switch (decision 5).
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.preview.tier()
    }

    /// The fact set the confirmation surface renders, stamped at preview time.
    #[must_use]
    pub fn summary(&self) -> &ConfirmationSummary {
        self.preview.summary()
    }

    /// The branch this undo will be committed to: the previewed one, and only that.
    #[must_use]
    pub fn reff(&self) -> &str {
        &self.reff
    }
}

/// What an undo did.
#[derive(Debug)]
pub enum UndoResult {
    /// The undo was recorded: prikk's own commit result, shown as commit shows it.
    Done(Box<stikk_prikk::CommitResult>),
    /// **A failure after the worktree was touched** (decision 6): step 2 or step 4 stopped, and
    /// **nothing was committed**. Never retried (`NFR-S04`).
    Stopped(UndoStopped),
}

/// An undo that stopped part-way, with what it had already changed.
#[derive(Debug)]
pub struct UndoStopped {
    /// Every worktree path the undo changed before it stopped, in the order it changed them — a
    /// rename contributing both of its names. Repository text: render each inert (`C-T2a`).
    pub changed: Vec<String>,
    /// **The failure itself**, so the frontend presents it the way it presents every other one,
    /// with prikk's words verbatim where they are prikk's (`ER-02`). A filesystem failure carries
    /// stikk's own line and names no program.
    pub cause: StikkError,
}

/// Preview undoing `block_id`, committed with `message`.
///
/// **Every read is inside [`crate::confirm::preview`]'s `compute`**, so the guards and the token
/// describe one instant: orientation, the current branch's lineage, the worktree status, and —
/// only once the first four guards have passed — prikk's two comparisons. An undo stikk will not
/// make never costs a `diff`.
///
/// # Errors
/// Propagates every [`StikkError`] the reads raise. A comparison prikk **refused** is not a
/// blocked preview here, unlike a switch's: a refusal between two blocks of the branch's own
/// history is not an expected shape, and answering it with a sentence of stikk's own would hide
/// prikk's.
pub fn undo_preview(
    prikk: &impl Prikk,
    repo: &Path,
    block_id: &str,
    message: &str,
) -> Result<UndoPreviewOutcome> {
    let intent = Intent {
        // **Materialization, because this writes the worktree from history** (RFC 045 decision
        // 2's own words), which makes it Tier Three. The AUTHOR requirement the commit needs is
        // taken in `undo_confirm_and_execute` with commit's own category, so no new category is
        // invented for the pair (§3.2).
        category: RequestCategory::Materialization,
        operation: UNDO_OPERATION,
    };
    let block_id = block_id.to_string();
    let message = message.to_string();
    let (view, token) = confirm::preview(prikk, repo, intent, || {
        compute(prikk, repo, &block_id, &message)
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => UndoPreviewOutcome::Blocked(reason),
        ReadView::Ready(preview) => {
            let UndoPreview {
                parent_id,
                reff,
                plan,
                message,
                ..
            } = preview.as_ref();
            let token = UndoToken {
                preview: token,
                reff: reff.clone(),
                parent_id: parent_id.clone(),
                plan: plan.clone(),
                message: message.clone(),
            };
            UndoPreviewOutcome::Ready {
                preview,
                token: Box::new(token),
            }
        }
    })
}

/// The two shapes the read can produce — never exposed; [`undo_preview`] unwraps it, attaching the
/// token to `Ready` alone (switch's own shape, for its own reason).
enum ReadView {
    Blocked(String),
    Ready(Box<UndoPreview>),
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: String) -> (ReadView, ConfirmationSummary) {
    let summary = ConfirmationSummary {
        operation: "Blocked".to_string(),
        preamble: None,
        target_ids: Vec::new(),
        counts: Vec::new(),
        capability: None,
        consequence: reason.clone(),
        target_name: None,
        signing_key_id: None,
        signing_key_claim: confirm::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        notices: Vec::new(),
        message_note: None,
        materializes: None,
        merge_plan: None,
        undo_plan: None,
        asides: Vec::new(),
    };
    (ReadView::Blocked(reason), summary)
}

/// The `compute` closure, which is where every read and every guard lives.
#[allow(clippy::too_many_lines)]
fn compute(
    prikk: &impl Prikk,
    repo: &Path,
    block_id: &str,
    message: &str,
) -> Result<(ReadView, ConfirmationSummary)> {
    let minor = prikk.handshake()?.version.minor;
    if minor < UNDO_FROM_MINOR {
        return Ok(blocked(undo_unavailable(minor)));
    }

    // **Orientation and the lineage** (§3.1 step 1). The branch is prikk's own current branch,
    // never `.prikk/current-branch` read by stikk (`CON-1`).
    let orientation = prikk.orientation(repo)?;
    let stikk_model::CurrentBranch::Branch(current) = &orientation.current_branch else {
        return Ok(blocked(NOT_THE_CURRENT_BRANCH.to_string()));
    };
    let reff = current.as_str().to_string();
    let lineage = prikk.history(repo, &reff, UNDO_HISTORY_LIMIT)?;
    let Some(at) = lineage
        .blocks
        .iter()
        .position(|row| row.block_id == block_id)
    else {
        // Decision 3: a block that is not in this branch's history is not a block this operation
        // offers, whatever ref it does belong to.
        return Ok(blocked(NOT_THE_CURRENT_BRANCH.to_string()));
    };
    let Some(block) = lineage.blocks.get(at) else {
        return Ok(blocked(NOT_THE_CURRENT_BRANCH.to_string()));
    };
    let parent = lineage.blocks.get(at + 1);

    // **The worktree status** (§3.1 step 3), read before the guards so guard 1 is the first
    // sentence a reader can meet, in decision 4's order.
    let status = changes::from_status(prikk.worktree_status(repo, &reff)?);

    // **The commit's own precondition, before the seven.** Not one of them: it is about the last
    // step rather than the undo, and an undo that cannot finish must not write at all.
    if let Some(routes) = orientation.interrupted_materialization.as_deref() {
        return Ok(blocked(format!(
            "{INTERRUPTED} {}",
            crate::interrupted_sentence(routes)
        )));
    }

    // **Guard 1** — exactly as decision 4 defines it, as clarified at acceptance. An untracked
    // file *elsewhere* does not block, because an editor's temporary file must not stop an undo;
    // one that is in the way is guard 6's.
    let unclean = status.missing > 0
        || status.modified > 0
        || status
            .entries
            .iter()
            .any(|entry| entry.kind != ChangeKind::Untracked)
        || status.refused_declarations.is_some_and(|count| count > 0)
        || status
            .declarations
            .iter()
            .any(|declaration| declaration.refusal.is_some());
    if unclean {
        return Ok(blocked(WORKTREE_HAS_CHANGES.to_string()));
    }

    // **Guard 2.**
    if orientation.queued_patches > 0 {
        return Ok(blocked(WORK_IS_QUEUED.to_string()));
    }

    // **Guard 3** (F4): `log` names only one parent of a merge, so there is no comparison to
    // invert.
    if block.kind == "Merge" || block.parents != 1 {
        return Ok(blocked(A_MERGE_BLOCK.to_string()));
    }

    // **Guard 4** (F4), and the truncation case the sentence would otherwise claim wrongly.
    let Some(parent) = parent else {
        let reason = if lineage.blocks.len() >= UNDO_HISTORY_LIMIT {
            DEEPER_THAN_STIKK_LOOKED
        } else {
            THE_FIRST_BLOCK
        };
        return Ok(blocked(reason.to_string()));
    };

    // **The two comparisons** (§3.1 step 2), read only now: the four guards above cost no `diff`.
    let diff = listed(prikk.diff(
        repo,
        &Comparison::Points {
            from: parent.block_id.clone(),
            to: block.block_id.clone(),
        },
    )?)?;
    // The tip of the branch. When the block *is* the tip, there is nothing later to compare.
    let later = match lineage.blocks.first() {
        Some(tip) if tip.block_id != block.block_id => Some(listed(prikk.diff(
            repo,
            &Comparison::Points {
                from: block.block_id.clone(),
                to: tip.block_id.clone(),
            },
        )?)?),
        _ => None,
    };

    // **Guard 5** (F3). Both sides of every rename count (§7 trap 4): a rename back onto a path a
    // later block re-created would otherwise slip through.
    if let Some(later) = &later {
        let mut overlapping: Vec<String> = touched(&diff)
            .into_iter()
            .filter(|path| touched(later).contains(path))
            .collect();
        overlapping.sort_unstable();
        overlapping.dedup();
        if !overlapping.is_empty() {
            return Ok(blocked(later_block_changed(&overlapping)));
        }
    }

    // **Guard 6**: a path the undo would *create* is already there, untracked.
    let created: Vec<&str> = diff
        .entries
        .iter()
        .filter_map(|entry| match entry.status.as_str() {
            "deleted" => Some(entry.path.as_str()),
            "renamed" => entry.from_path.as_deref(),
            _ => None,
        })
        .collect();
    let mut in_the_way_paths: Vec<String> = status
        .entries
        .iter()
        .filter(|entry| entry.kind == ChangeKind::Untracked)
        .filter(|entry| created.contains(&entry.path.as_str()))
        .map(|entry| entry.path.clone())
        .collect();
    in_the_way_paths.sort_unstable();
    in_the_way_paths.dedup();
    if !in_the_way_paths.is_empty() {
        return Ok(blocked(in_the_way(&in_the_way_paths)));
    }

    // **Guard 7, and the path check with it** — the one place a plan is built.
    let plan = match UndoPlan::from_diff(repo, &diff) {
        Ok(plan) => plan,
        Err(reason) => return Ok(blocked(reason)),
    };

    // RFC 026 §5: the id and how firmly the card may claim it, read as commit's preview reads it.
    let report = prikk.readiness(repo)?;
    let (signing_key_id, signing_key_claim) = confirm::signing_key_claim(
        report.readiness.author,
        &report.author,
        stikk_prikk::key_id::author_key_id(),
    );
    let preview = UndoPreview {
        block_id: block.block_id.clone(),
        parent_id: parent.block_id.clone(),
        reff,
        messages: messages(block),
        plan,
        message: message.to_string(),
        signing_key_id,
        signing_key_claim,
        signing_key_is_published_example: report
            .author
            .public_key
            .as_deref()
            .and_then(stikk_model::published_example)
            .is_some(),
    };
    let summary = preview.summary();
    Ok((ReadView::Ready(Box::new(preview)), summary))
}

/// The block's patch messages, prikk's text verbatim (RFC 015 F1).
fn messages(block: &BlockRow) -> Vec<String> {
    block
        .messages
        .iter()
        .map(|message| message.message.clone())
        .collect()
}

/// Every path a comparison names, **both sides of a rename included** (§7 trap 4).
fn touched(diff: &Diff) -> Vec<String> {
    let mut paths = Vec::with_capacity(diff.entries.len());
    for entry in &diff.entries {
        paths.push(entry.path.clone());
        if let Some(from) = &entry.from_path {
            paths.push(from.clone());
        }
    }
    paths
}

/// A comparison between two blocks of the branch's own history, which is listed or nothing.
///
/// `Unreported` is below the band, and the band was checked before any read — so reaching it here
/// is an internal inconsistency, not a state to word a sentence for.
fn listed(report: DiffReport) -> Result<Diff> {
    match report {
        DiffReport::Listed(diff) => Ok(diff),
        DiffReport::Unreported => Err(StikkError::Internal {
            detail: format!(
                "{UNDO_OPERATION}: prikk reported no comparison at or above 0.{UNDO_FROM_MINOR}"
            ),
        }),
    }
}

/// Confirm and execute an undo, in decision 6's order.
///
/// **The gate commit uses** (§3.2) is taken first: the undo ends in a `commit`, so it needs AUTHOR
/// signing readiness, which is [`RequestCategory::QueueMutation`]'s own requirement. Its own
/// category is [`RequestCategory::Materialization`], which is Tier Three and needs no key — so the
/// tier comes from the one and the key from the other, and no new category was invented for the
/// pair.
///
/// # Errors
/// [`StikkError::NotReady`] if read-only mode or AUTHOR capability is insufficient;
/// [`StikkError::Stale`] if the repository changed since the preview or the confirmation;
/// [`StikkError::Declined`] if `evidence` does not satisfy Tier Three; and **every failure of
/// step 1**, which has written nothing — prikk's refusal among them, verbatim. A failure in step
/// 2 or step 4 is *not* an error: it is [`UndoResult::Stopped`], because by then the worktree has
/// changed and the paths it changed are the thing the user needs.
pub fn undo_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: UndoToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
) -> Result<Outcome<UndoResult>> {
    let UndoToken {
        preview,
        reff,
        parent_id,
        plan,
        message,
    } = token;
    confirm::capability_gate(UNDO_OPERATION, RequestCategory::QueueMutation, readiness)?;
    let confirmed = confirm::confirm(prikk, repo, preview, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        run(prikk, repo, &reff, &parent_id, &plan, &message)
    })
}

/// Decision 6's four steps, in decision 6's order. **Nothing here touches the worktree until step
/// 1 has fetched every byte** (§7 trap 1); that ordering is the whole safety argument for a failed
/// fetch.
fn run(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
    parent: &str,
    plan: &UndoPlan,
    message: &str,
) -> Result<UndoResult> {
    // **Step 1: fetch, outside the repository.** A fresh directory, created by this process, never
    // an existing one; numbered files, so no nested directory is needed and no name from the
    // report is reused.
    let scratch = Scratch::fresh(repo)?;
    let mut fetched: Vec<(PathBuf, &UndoStep)> = Vec::new();
    for (n, step) in plan
        .steps
        .iter()
        .filter(|step| step.puts_bytes_back())
        .enumerate()
    {
        let output = scratch.dir.join(n.to_string());
        match prikk.save_file(repo, parent, step.parent_path(), &output)? {
            FileSaved::Done => fetched.push((output, step)),
            FileSaved::Unavailable => return Err(below_the_band()),
        }
    }

    // **Step 2: apply.** Renames, then removals, then the fetched bytes, then the directories this
    // undo emptied. Every path that changes is recorded as it changes, because a failure from here
    // on is reported by what it already did.
    let mut changed: Vec<String> = Vec::new();
    for step in &plan.steps {
        if let UndoAction::RenameBack { earlier } = &step.action {
            match prikk.rename_path(repo, &step.path, earlier) {
                Ok(Renamed::Done(_)) => {
                    changed.push(step.path.clone());
                    changed.push(earlier.clone());
                }
                Ok(Renamed::Unavailable) => return Ok(stopped(changed, below_the_band())),
                Err(error) => return Ok(stopped(changed, error)),
            }
        }
    }
    let mut emptied: Vec<&str> = Vec::new();
    for step in &plan.steps {
        match &step.action {
            UndoAction::Remove => {
                let target = repo.join(&step.path);
                if let Err(error) = std::fs::remove_file(&target) {
                    return Ok(stopped(changed, io_failure("remove", &step.path, &error)));
                }
                changed.push(step.path.clone());
                emptied.push(step.path.as_str());
            }
            // A rename moved the file out of its directory, which may now be empty too.
            UndoAction::RenameBack { .. } => emptied.push(step.path.as_str()),
            UndoAction::PutBack | UndoAction::PutBackPermissions => {}
        }
    }
    for (source, step) in &fetched {
        let relative = step.parent_path();
        let target = repo.join(relative);
        if let Some(parent_dir) = target.parent()
            && let Err(error) = std::fs::create_dir_all(parent_dir)
        {
            return Ok(stopped(changed, io_failure("create", relative, &error)));
        }
        if let Err(error) = std::fs::copy(source, &target) {
            return Ok(stopped(changed, io_failure("write", relative, &error)));
        }
        // **The mode is the parent's own** (F2): `cat --output` does not keep one at 0.55, and
        // prikk applies it from 0.57.0 — applying it here is harmless once prikk does too. On
        // Windows nothing is set: guard 7 refused every case where it would have mattered (F5).
        #[cfg(unix)]
        if let Some(mode) = step.mode {
            use std::os::unix::fs::PermissionsExt;
            if let Err(error) =
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode & 0o777))
            {
                return Ok(stopped(
                    changed,
                    io_failure("set the mode of", relative, &error),
                ));
            }
        }
        if !changed.iter().any(|path| path == relative) {
            changed.push(relative.to_string());
        }
    }
    // **Only the directories this undo emptied** (§3.4), each walked upward and stopped at the
    // first one that is not empty. Never any other directory.
    for path in emptied {
        if let Err(error) = prune_emptied(repo, path) {
            return Ok(stopped(changed, io_failure("remove", path, &error)));
        }
    }

    // **Step 3: delete the scratch directory, whatever happened.** `Scratch` does it on every
    // early return above too, and once more when it drops.
    scratch.remove();

    // **Step 4: commit.** A refusal here is not retried (`NFR-S04`), and nothing was committed.
    match prikk.commit(repo, reff, message) {
        Ok(result) => Ok(UndoResult::Done(Box::new(result))),
        Err(error) => Ok(stopped(changed, error)),
    }
}

/// An undo that stopped with the worktree already changed.
fn stopped(changed: Vec<String>, cause: StikkError) -> UndoResult {
    UndoResult::Stopped(UndoStopped { changed, cause })
}

/// Below the band at execution — the band was read in the preview, so this is a prikk that changed
/// underneath the session rather than a state to word a sentence for.
fn below_the_band() -> StikkError {
    StikkError::Internal {
        detail: format!(
            "{UNDO_OPERATION}: this prikk is below 0.{UNDO_FROM_MINOR}, which the preview read \
             otherwise"
        ),
    }
}

/// A filesystem failure, in **stikk's own words**, naming the path and no program: stikk's own
/// `fs` call failed, and prikk said nothing about it.
fn io_failure(what: &str, path: &str, error: &std::io::Error) -> StikkError {
    StikkError::environment_msg(format!(
        "stikk could not {what} `{path}` while undoing this block: {error}"
    ))
}

/// Remove the directories **this undo emptied** above `path`, upward, stopping at the first one
/// that is not empty (§3.4).
///
/// A directory that cannot be read is left alone: not knowing it is empty is not knowing it can
/// go. Only a `remove_dir` that fails on a directory stikk had just found empty is a failure, and
/// it stops the undo like any other.
fn prune_emptied(repo: &Path, path: &str) -> std::io::Result<()> {
    let mut relative = Path::new(path).parent();
    while let Some(directory) = relative {
        if directory.as_os_str().is_empty() {
            break;
        }
        let absolute = repo.join(directory);
        match std::fs::read_dir(&absolute) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    break;
                }
            }
            Err(_) => break,
        }
        std::fs::remove_dir(&absolute)?;
        relative = directory.parent();
    }
    Ok(())
}

/// The fresh directory step 1 fetches into: **outside the repository**, created by this process,
/// and deleted whatever happens.
#[derive(Debug)]
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    /// Create one under [`std::env::temp_dir`], with a name no other process holds.
    ///
    /// **`create_dir`, never `create_dir_all` on an existing path** (§3.3): the point of a fresh
    /// directory is that nothing else is in it, and a directory that was already there is not
    /// fresh. A name that is taken is tried again with the next one, a few times.
    fn fresh(repo: &Path) -> Result<Self> {
        let base = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        for attempt in 0..ATTEMPTS {
            let dir = base.join(format!(
                "stikk-undo-{}-{stamp}-{attempt}",
                std::process::id()
            ));
            // **`C-E2`'s own check** (RFC 053 decision 4's, applied the other way round): a
            // temporary directory inside the repository would put the fetched bytes in the
            // worktree, where the undo's own commit would then capture them.
            if stikk_state::paths::ensure_outside_repository(&dir, Some(repo)).is_err() {
                return Err(StikkError::environment_msg(format!(
                    "stikk would have fetched this block's earlier files into {}, which is inside \
                     this repository. Set TMPDIR to a directory outside it.",
                    base.display()
                )));
            }
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(Self { dir }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(StikkError::environment(
                        format!(
                            "stikk could not create a temporary directory in {} for this block's \
                             earlier files",
                            base.display()
                        ),
                        error,
                    ));
                }
            }
        }
        Err(StikkError::environment_msg(format!(
            "stikk could not find an unused temporary directory name in {}",
            base.display()
        )))
    }

    /// Delete it. Idempotent, so [`Drop`] can run after this and find nothing.
    fn remove(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// How many names [`Scratch::fresh`] tries before giving up.
const ATTEMPTS: u32 = 8;

impl Drop for Scratch {
    /// **Step 3 on every path out of [`run`]**, including the failures that return early.
    fn drop(&mut self) {
        self.remove();
    }
}

#[cfg(test)]
mod tests;

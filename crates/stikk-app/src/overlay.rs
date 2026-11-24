//! **What an overlay is** (design TU-02/07/08; RFC 007; RFC 010) — the state, not the drawing.
//!
//! An overlay is a card above the active view: the glossary/help browser (FR-111), the ref picker
//! (RFC 006), the refusal explanation (TU-08/FR-110), the command palette (TU-07/FR-125), the
//! recent-refusals list (FR-112), the Background Operations listing (TU-01; RFC 010), and every
//! prompt, confirmation and result card the operations open. Overlays form a stack; the top one is
//! the one a front end draws and the one that takes keys.
//!
//! [`Overlay::Loading`] is the pending-overlay counterpart to [`crate::app::Screen::Loading`] (RFC
//! 010 §5): pushed at once for an overlay-bound request, replaced or removed by `App::apply`, and
//! popped by `back()` like any other overlay.
//!
//! The refusal overlay is the load-bearing one: prikk's message is carried **verbatim and inert**
//! (C-T2a/C-T2b), and the gloss and next-steps are stikk's own, separate from it (ER-02/C-T4c).
//! [`Overlay::Stale`] (RFC 013 §5) looks similar but is a **distinct type**, not a `Refusal`
//! carrying stikk's own words: it exists precisely so nothing stikk itself says can be labelled as
//! prikk's (design-review C1).
//!
//! **How any of this looks is not here** (RFC 067 decision 2): the drawing lives in
//! `stikk_tui::overlay`, which matches on these variants and owns every width, border and colour.

use stikk_core::{ConfirmationSummary, NextStep, RefusalCard, RefusalRecord, RefusedPath};
use stikk_model::Tier;

use crate::app::Operation;

/// Which of RFC 053's two saves [`Overlay::SavePath`] is prompting for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavePathKind {
    /// `F`: one file, as it was at this block.
    File {
        /// The block the file is read at.
        block: String,
        /// The repository path, as prikk names it.
        path: String,
    },
    /// `A`: every file at this block, into a new folder.
    All {
        /// The block the files are read at.
        block: String,
    },
}

/// One overlay. Data-carrying variants own their state; `Glossary` reads the static asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    /// The glossary / help browser (terminology mapping, key reference, code index).
    ///
    /// **It scrolls like every other card**, through the offsets the drawing keeps for its surface
    /// (RFC 060 decision 3 and RFC 067 decision 4, as
    /// Handoff B corrects it). It used to keep an `offset: Cell<u16>` of its own, which `nav_down`
    /// never reached — so `↓` moved nothing, and the only key that scrolled the help was `Enter`,
    /// which the foot line never named (RFC 060 Handoff B's defect). One mechanism, one set of keys.
    Glossary,
    /// An overlay-bound request asked for but not yet arrived (RFC 010 §5) — currently only the ref
    /// picker's read. Replaced or removed by `App::apply`; popped directly by `back()`.
    Loading {
        /// A short label for what is loading (e.g. `"refs"`).
        what: &'static str,
        /// The request this placeholder is waiting on; a response for any other `seq` is stale here.
        seq: u64,
    },
    /// The Background Operations listing (TU-01; RFC 010) — a snapshot of running and finished
    /// requests this session, taken when the overlay was opened (the same convention
    /// [`Overlay::Refusals`] uses). No cancel action this increment (RFC 010 decision 6).
    Operations {
        /// The operations at the moment this overlay was opened, oldest-first.
        operations: Vec<Operation>,
    },
    /// A ref chooser: the ref names and the highlighted index.
    RefPicker {
        /// The selectable ref names (repository-sourced; rendered as identifiers).
        ///
        /// **When prikk names an unpublished current branch it is the first entry**, and
        /// `unpublished_current` is set. It is selectable like any other row, because focusing it
        /// *is* like any other focus: History shows its queued tier and no sealed blocks, and
        /// Changes works as it does today (RFC 037 Handoff B §3).
        refs: Vec<String>,
        /// The highlighted entry.
        cursor: usize,
        /// Set only when prikk reported no refs at all **and could not name a current branch**
        /// (RFC 029 Handoff B §4): the picker then offers one row, `heads/main (not published)`,
        /// so a brand-new repository below prikk 0.47 can still be focused and make its first
        /// commit. **Where prikk answers, this guess is off** — see `unpublished_current`.
        unpublished_main: bool,
        /// Whether `refs[0]` is the branch prikk named as the current one before its first seal
        /// (RFC 037 decision 4). The row is marked in stikk's own words; the name itself is
        /// repository text and is rendered as an identifier.
        unpublished_current: bool,
    },
    /// A refusal explanation (TU-08): verbatim message + gloss + next-steps + glossary links.
    Refusal {
        /// The card content (all stikk-owned or verbatim prikk).
        card: RefusalCard,
        /// The highlighted next-step.
        cursor: usize,
    },
    /// The repository moved between a preview and its confirmation, or confirmation and execution
    /// (`OPL-02`/`CT-05`; RFC 013 decision 3; design-review C1). Deliberately **not** [`Self::Refusal`]:
    /// every string here is stikk's own, and this overlay's renderer says so, never "prikk reported".
    Stale {
        /// The operation whose preview no longer matches (stikk's own short name).
        operation: String,
        /// Which check found the preview stale (RFC 030 amendment A3).
        cause: stikk_model::StaleCause,
        /// stikk's headline, rendered after the operation name — from `stikk-core`, never a literal here.
        headline: String,
        /// stikk's explanation, in its own voice.
        gloss: String,
        /// Next-steps, stikk-authored — today always exactly one: re-preview.
        next_steps: Vec<NextStep>,
        /// The highlighted next-step.
        cursor: usize,
    },
    /// **Commit is unavailable because prikk reports it would refuse** (RFC 027 decision 5; prikk ≥
    /// 0.39) — [`stikk_core::CommitPreviewOutcome::WouldRefuse`].
    ///
    /// Drawn in the refusal card's visual language — prikk's reasons verbatim and inert under quote
    /// bars, stikk's next steps below — but deliberately **not** a [`Self::Refusal`]: nothing was
    /// attempted, so there is no refusal to present, and this never passes through `present()` (RFC 014
    /// decided a blocked preview is not an error). **Not the shell banner either**, which holds one line
    /// and cannot list a path and a reason per entry at 80 columns. No selection: the next steps are
    /// things to do in the worktree, not actions stikk can take.
    CommitWouldRefuse {
        /// The entries prikk would refuse, each with prikk's reason.
        paths: Vec<RefusedPath>,
    },
    /// **Commit is unavailable because prikk would refuse a rename declaration** (RFC 034 §5; prikk ≥
    /// 0.44). The same overlay shape as [`Self::CommitWouldRefuse`], and a different fact: that one is
    /// about paths, this one about declarations, and prikk reports them apart.
    CommitWouldRefuseDeclarations {
        /// The declarations prikk would refuse, each with prikk's own refusal.
        declarations: Vec<stikk_core::RefusedDeclaration>,
    },
    /// **What can be done with the selected row** (RFC 061 decision 3).
    ///
    /// Enter opens it, its first entry is what the right arrow runs at once, and `Esc`, the left
    /// arrow and `q` close it. It holds no list of its own: the entries are catalog entries, so what
    /// they are called and what keys they have cannot drift from the palette or the key line.
    ActionList {
        /// What the list is about, in stikk's words and the repository's own string: *"Block
        /// 1a2b3c4d"*, *"Branch heads/dev"*. Rendered inert (RFC 061 trap 6).
        subject: String,
        /// The entries, from [`stikk_core::actions::here`].
        entries: Vec<&'static stikk_core::Action>,
        /// The highlighted entry; `0`, the row's own first action, when it opens.
        cursor: usize,
        /// The session's signing readiness, for the disabled entries' reasons (`FR-104`), as the
        /// palette carries it.
        readiness: stikk_model::Readiness,
        /// Whether a ref is focused, for the entries that need one (RFC 029 Handoff B).
        ref_focused: bool,
    },
    /// The command palette (`TU-07`): a filter and the highlighted match. **It lists every action
    /// the catalog holds** (RFC 061 decision 1; `FR-125`).
    Palette {
        /// The current filter text.
        filter: String,
        /// The highlighted match.
        cursor: usize,
        /// The session's signing readiness, for the disabled-entry reasons (FR-104; RFC 014 §6) — a
        /// bare `Capability` cannot see read-only, so it stopped being enough the moment a mutating
        /// command (`op.commit`) entered the registry.
        readiness: stikk_model::Readiness,
        /// Whether a ref is focused, for the commands that need one (RFC 029 Handoff B §2).
        ref_focused: bool,
    },
    /// The session refusal history (FR-112): the remembered refusals and the highlighted one.
    Refusals {
        /// The remembered refusals, newest-first.
        records: Vec<RefusalRecord>,
        /// The highlighted entry.
        cursor: usize,
    },
    /// The `TU-09` confirmation overlay (RFC 013 §6): restates the operation from the preview's
    /// [`ConfirmationSummary`] — never a fresh read, since the summary is the one true copy stamped at
    /// preview time — and collects whatever evidence the tier requires. Chrome stays visibly stikk's
    /// (`C-T2b`): a content pane must not be able to look like this. **Commit is this overlay's first
    /// consumer** (RFC 014 §3): `App` drives it by pairing this overlay's display state with a
    /// [`crate::app::App`]-owned [`stikk_core::PreviewToken`] it does not carry itself, since a token
    /// has no public constructor and this type derives `Clone`/`PartialEq`.
    Confirmation {
        /// What to restate. Composed at preview time; never re-derived here (RFC 013 §3). Boxed only to keep
        /// this enum's variants close in size (`clippy::large_enum_variant`), since the summary gained RFC 032's
        /// lines.
        summary: Box<ConfirmationSummary>,
        /// Which evidence shape this tier needs.
        tier: Tier,
        /// The user's typed input so far — meaningful only for [`Tier::ThreeTyped`].
        typed: String,
        /// An inline message from a declined confirmation attempt
        /// ([`stikk_model::StikkError::Declined`]), shown in place — never a separate popup
        /// (RFC 013 §4: the user is still mid-confirmation, not facing a new failure).
        error: Option<String>,
    },
    /// `FL-05` step 2: the commit message prompt — required non-empty, before any confirmation exists
    /// to restate (RFC 014 §3). Its own overlay, not folded into [`Self::Confirmation`], because the
    /// message is mutable input the user is still composing, not a fact to restate.
    CommitMessage {
        /// The ref this commit will target (snapshotted at open time, like every other overlay here).
        reff: String,
        /// The message typed so far.
        typed: String,
        /// Whether this session's prikk persists the message (schema 4, upstream RFC 123) rather than
        /// validating and discarding it (`UD-01`, retired at prikk 0.32 — RFC 015 F2). Snapshotted at
        /// open time from [`stikk_core::OrientationView::prikk_persists_messages`] — stikk supports
        /// both sides of that boundary, so this copy is version-conditional, not a blanket claim.
        messages_persist: bool,
    },
    /// RFC 058 decision 5: the message an undo's commit will carry, **prefilled** with
    /// `Undo: <the block's first patch message>` and editable.
    ///
    /// Modelled on [`Self::CommitMessage`], and its own variant for the same reason that one is
    /// not folded into [`Self::Confirmation`]: this is input the user is still composing. It
    /// carries no ref — the undo commits to the branch the repository is on, which the preview
    /// reads for itself (decision 3) — and no notice about whether prikk persists a message: by
    /// 0.54, the band an undo needs, prikk does.
    UndoMessage {
        /// The block being undone, for the heading. Repository text, rendered as an identifier.
        block: String,
        /// The message so far, prefilled at open time.
        typed: String,
    },
    /// RFC 058 decision 5: a recorded undo — prikk's own commit result, exactly as
    /// [`Self::CommitResult`] shows one, with stikk's own next step under it.
    UndoRecorded {
        /// prikk's result, transported unchanged. Boxed to keep this enum's variants close in
        /// size (`clippy::large_enum_variant`).
        result: Box<stikk_prikk::CommitResult>,
    },
    /// RFC 058 decision 6: an undo that stopped after the worktree had changed. **Not a refusal
    /// overlay**: the operation ran, and what the reader needs is the list of paths it changed,
    /// not a next-step list about re-previewing.
    UndoStopped {
        /// Every path the undo changed before it stopped, in the order it changed them.
        changed: Vec<String>,
        /// The one line of the cause — prikk's words where they are prikk's (`ER-02`), stikk's
        /// own where the filesystem failed, decided by [`stikk_core::stopped_text`].
        text: String,
    },
    /// `FL-05` step 4's tail: prikk's own commit result, shown verbatim (`C-T4a`/`C-T4c`) — patch id,
    /// operation counts, and every `note:` line it printed, in order (RFC 014 F4).
    CommitResult {
        /// prikk's result, transported unchanged.
        result: stikk_prikk::CommitResult,
    },
    /// `FL-06`'s consent step (RFC 016 §8) — the no-audit acknowledgement, its own distinct act after
    /// [`Self::Confirmation`]'s evidence and before execution, because the act it precedes is
    /// irreversible. **Unchecked and cannot be defaulted**: `acknowledged` starts `false`, only a
    /// dedicated toggle key sets it, and `Enter` here does nothing while it is still `false` — a bare
    /// `Enter` carried over by habit from the previous screen confirms nothing.
    SealConsent {
        /// The ref this seal will target (snapshotted at open time, like every other overlay here).
        reff: String,
        /// Whether the user has explicitly acknowledged [`stikk_core::SEAL_CONSENT_COPY`]. Toggled by
        /// its own key, never implied by reaching this screen or by any other input.
        acknowledged: bool,
    },
    /// `FL-06`'s tail: prikk's own seal result, shown verbatim (`C-T4a`/`C-T4c`) — the new block id,
    /// the ref's new `RefState`, and every `note:` line it printed, in order (RFC 016 §5).
    SealResult {
        /// prikk's result, transported unchanged.
        result: stikk_prikk::SealResult,
    },
    /// RFC 044 decision 5: the name of a branch or tag about to be created, typed after a **fixed
    /// prefix the user cannot edit**. Modelled on [`Self::CommitMessage`] — mutable input the user
    /// is still composing, so its own overlay rather than part of the confirmation that restates it.
    ///
    /// **stikk adds no naming rules.** An empty name is not sent, and that is the only one; every
    /// other judgement is prikk's, and its refusal is shown verbatim (`ER-02`).
    RefName {
        /// Which ref is being named, which is what fixes the prefix.
        kind: RefNameKind,
        /// The ref the branch will start at, or the ref or block the tag will point at — repository
        /// text, rendered as an identifier (RFC 036), shown so the prompt names what it targets.
        source: String,
        /// The part after the prefix, typed so far.
        typed: String,
    },
    /// RFC 050 decision 5: where to write a branch's bundle, prefilled with stikk's suggestion.
    ///
    /// **A second prompt, not [`Self::BundlePath`] with a flag.** The two ask for a path and
    /// nothing else is the same: one opens empty and reads, the other opens filled and writes,
    /// and the notice a reader needs is different in each. One overlay with a `kind` would
    /// carry a branch name no opening prompt has.
    ExportPath {
        /// The branch being exported, for the heading.
        reff: String,
        /// The path typed so far, prefilled with stikk's suggestion.
        typed: String,
    },
    /// RFC 053 decision 4: where to write a file, or a block's files — **prefilled** with
    /// stikk's suggestion, which is outside the repository by construction.
    ///
    /// **One overlay for both keys**, carrying which it is: the two prompts differ in their
    /// heading and in what the answer becomes, and nothing else. A second overlay would be a
    /// second renderer to keep in step with the first.
    SavePath {
        /// Which save this is, and what it needs to build its target.
        what: SavePathKind,
        /// What the heading names — the file's path for `F`, the block's short id for `A`.
        heading: String,
        /// The path typed so far, prefilled with stikk's suggestion.
        typed: String,
    },
    /// RFC 053 decision 6: what stikk wrote, in stikk's own words.
    ///
    /// **stikk's sentence, not prikk's lines** — unlike every other result card here. prikk
    /// prints nothing at all on a successful `cat --output` (F1), and `extract`'s JSON report is
    /// a report rather than prose. There is nothing of prikk's to show verbatim.
    SaveResult {
        /// The sentence, and the provisional notice where the report carried one.
        lines: Vec<String>,
    },
    /// RFC 056 decision 2: the **public key** of the maintainer key being adopted, asked first
    /// because the id prompt is prefilled from it.
    ///
    /// **Opens empty.** There is nothing to suggest: the key comes from its owner, and the one
    /// thing a newcomer gets wrong is taking it from the bundle they were sent.
    TrustPublicKey {
        /// What has been typed or pasted so far, checked on submission rather than per key.
        typed: String,
        /// stikk's own sentence from the last attempt, where the shape was wrong -- the prompt
        /// stays open and says why (decision 2).
        error: Option<String>,
    },
    /// RFC 056 decision 2: the **key id**, prefilled with the key's own default id and editable.
    TrustKeyId {
        /// The public key the previous prompt accepted, carried so the preview has both halves.
        public_key: String,
        /// The id typed so far, prefilled with `default_key_id`.
        typed: String,
        /// stikk's own sentence from the last attempt, as above.
        error: Option<String>,
    },
    /// **The security event** (RFC 056 decision 4, generalised by RFC 063 decision 8).
    ///
    /// **Not a confirmation**, and it carries no confirm key: `Esc` is the only way out. There
    /// is nothing here for the reader to approve -- what they need is to see the keys and go
    /// back to the key's owner, or to the bundle's sender.
    ///
    /// **One renderer, three callers**: stikk's own guard from the trust list, prikk's two typed
    /// key-id conflicts on an adopt, and a bundle whose seal does not verify against the key
    /// trusted under its id. A twin overlay that looked the same would drift, and looking
    /// identical wherever it comes from is what makes this card recognizable as the event it is.
    TrustConflict {
        /// The heading, rows, body and -- where prikk refused -- prikk's own sentence.
        warning: Box<stikk_core::SecurityWarning>,
    },
    /// RFC 056 decision 1: what a trust write left -- prikk's own lines, then the pointer a
    /// removal carries.
    TrustResult {
        /// prikk's printed lines, verbatim.
        printed: Vec<String>,
        /// After a removal, `REMOVED_SEE_VERIFY`; after an adopt, `None`.
        see_verify: Option<&'static str>,
    },
    /// RFC 050 decision 3: the path of a bundle file to open.
    ///
    /// **Its own overlay, not [`Self::RefName`]'s.** That prompt fixes a prefix (`heads/` or
    /// `tags/`) from its `kind` and exists to build a ref name; a file path takes no prefix, is
    /// not a ref, and is resolved rather than parsed. Reusing it would have meant a third
    /// `RefNameKind` whose prefix is empty and whose name is not a name.
    BundlePath {
        /// The path typed so far, exactly as typed — resolved on submission, not while typing.
        typed: String,
    },
    /// RFC 044 decision 5: a tag's message, prompted after its name and **allowed to be empty** —
    /// an empty one sends no `-m`, which is prikk's own shape for a tag without a message.
    TagMessage {
        /// The full tag name, prefix included, as it will be passed to prikk.
        name: String,
        /// The ref or block the tag will point at.
        target: String,
        /// The message typed so far.
        typed: String,
    },
    /// RFC 050 decision 5: prikk's own lines after an export, **verbatim and in order** — the
    /// tip block, the object count and its own continuity note about the author keys. There is
    /// no `--format json` for the command, so nothing is parsed.
    ExportResult {
        /// prikk's printed lines, exactly as it printed them.
        printed: Vec<String>,
    },
    /// RFC 050 decision 4: prikk's own lines after an import, **verbatim and in order** —
    /// including its note that no local ref was created or advanced and no MAINTAINER key was
    /// trusted, which is the part a reader most needs. There is no `--format json` for the
    /// command, so there is nothing to parse and nothing is parsed.
    ImportResult {
        /// prikk's printed lines, exactly as it printed them.
        printed: Vec<String>,
    },
    /// RFC 044 decision 8: prikk's own lines after a ref write, **verbatim and in order**. There is
    /// no `--format json` for these three commands (F1), so there is nothing to parse and nothing
    /// is parsed: the new state is read back from the ref lists, and this card only shows what
    /// prikk said.
    RefWriteResult {
        /// Which write this was — stikk's own word for it, in the title and the heading.
        kind: RefWriteKind,
        /// prikk's printed lines, exactly as it printed them.
        lines: Vec<String>,
    },
    /// RFC 045 decision 8: what prikk printed after a switch, **both streams, verbatim and
    /// apart**. prikk can name a snapshot fallback on stderr at exit 0, and that is a different
    /// kind of statement from the counts on stdout.
    SwitchResult {
        /// stdout's lines, in order.
        printed: Vec<String>,
        /// stderr's lines at exit 0, in order — usually empty.
        noted: Vec<String>,
    },
    /// A merge that landed: prikk's own lines, and stikk's one line about the worktree
    /// (RFC 047 decision 6).
    MergeResult {
        /// prikk's printed lines, in order and read by nothing.
        printed: Vec<String>,
    },
}

/// Which ref an [`Overlay::RefName`] prompt is naming (RFC 044 decision 5). It decides the fixed
/// prefix and the prompt's own words, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefNameKind {
    /// A new branch: `heads/`.
    Branch,
    /// A new tag: `tags/`.
    Tag,
}

impl RefNameKind {
    /// The prefix shown fixed in front of what the user types — prikk's own namespace, never
    /// stikk's invention (F2: a name without it is refused, verbatim).
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Branch => "heads/",
            Self::Tag => "tags/",
        }
    }
}

/// Which of RFC 044's three writes an [`Overlay::RefWriteResult`] is reporting — stikk's own
/// naming of its own operation, never a reading of prikk's prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefWriteKind {
    /// `branch create`.
    CreateBranch,
    /// `tag create`.
    CreateTag,
    /// `branch close`.
    CloseBranch,
}

impl RefWriteKind {
    /// The title bar text for the result card.
    #[must_use]
    pub fn done_title(self) -> &'static str {
        match self {
            Self::CreateBranch => " Branch created ",
            Self::CreateTag => " Tag created ",
            Self::CloseBranch => " Branch closed ",
        }
    }
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Glossary => " Glossary & Help ",
            Self::Loading { .. } => " Loading ",
            Self::CommitWouldRefuse { .. } | Self::CommitWouldRefuseDeclarations { .. } => {
                " Commit unavailable "
            }
            Self::Operations { .. } => " Background operations ",
            Self::RefPicker { .. } => " Choose ref ",
            Self::Refusal { .. } => " prikk refused ",
            Self::Stale { .. } => " stikk stopped ",
            // **The rendered title is the row's own** (*"Block 1a2b3c4d"*); this is the generic
            // name, for a caller that wants one without a subject.
            Self::ActionList { .. } => " What can be done here ",
            Self::Palette { .. } => " Command palette ",
            Self::Refusals { .. } => " Recent refusals ",
            Self::Confirmation { .. } => " Confirm ",
            Self::CommitMessage { .. } => " Commit message ",
            Self::CommitResult { .. } => " Commit recorded ",
            Self::UndoMessage { .. } => " Undo message ",
            Self::UndoRecorded { .. } => " Undo recorded ",
            Self::UndoStopped { .. } => " Undo stopped ",
            Self::SealConsent { .. } => " Before you seal ",
            Self::SealResult { .. } => " Sealed ",
            Self::RefName {
                kind: RefNameKind::Branch,
                ..
            } => " New branch ",
            Self::RefName {
                kind: RefNameKind::Tag,
                ..
            } => " New tag ",
            Self::TrustPublicKey { .. } => " Adopt a key · public key ",
            Self::TrustKeyId { .. } => " Adopt a key · key id ",
            Self::TrustConflict { .. } => " Two keys, one id ",
            Self::TrustResult { .. } => " Trust changed ",
            Self::BundlePath { .. } => " Open a bundle ",
            Self::SavePath { what, .. } => match what {
                SavePathKind::File { .. } => " Save this file ",
                SavePathKind::All { .. } => " Save this block's files ",
            },
            Self::SaveResult { .. } => " Saved ",
            Self::ExportPath { .. } => " Export a bundle ",
            Self::TagMessage { .. } => " Tag message ",
            Self::RefWriteResult { kind, .. } => kind.done_title(),
            Self::SwitchResult { .. } => " Switched ",
            Self::MergeResult { .. } => " Merged ",
            Self::ImportResult { .. } => " Imported ",
            Self::ExportResult { .. } => " Exported ",
        }
    }
}

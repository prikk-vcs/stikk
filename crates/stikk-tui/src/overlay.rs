//! The overlay layer (design TU-02/07/08; RFC 007; RFC 010).
//!
//! An overlay is drawn above the active view without destroying it. This increment ships: the
//! **glossary / help** browser (FR-111), the **ref picker** (RFC 006), the **refusal explanation**
//! overlay (TU-08/FR-110), the **command palette** (TU-07/FR-125), the **recent-refusals** list
//! (FR-112), and the **Background Operations** listing (TU-01; RFC 010 — no cancel action this
//! increment). Overlays form a stack; the top one renders and receives navigation keys.
//!
//! [`Overlay::Loading`] is the pending-overlay counterpart to [`crate::app::Screen::Loading`] (RFC 010
//! §5): pushed immediately for an overlay-bound request (currently only the ref picker's), replaced or
//! removed by `App::apply` (crate-private), and popped directly by `back()` like any other overlay.
//!
//! The refusal overlay is the load-bearing one: prikk's message is shown **verbatim and inert**, in a
//! quoted content region visibly distinct from stikk's chrome (C-T2a/C-T2b); the gloss and the
//! next-steps are stikk's own, separate from and below the message (ER-02/C-T4c). [`Overlay::Stale`]
//! (RFC 013 §5) looks similar but is a **distinct type**, not a `Refusal` carrying stikk's own words:
//! it exists precisely so nothing stikk itself says can be labelled as prikk's (design-review C1).

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use stikk_core::{
    ConfirmationSummary, KeyClaim, NextStep, RefusalCard, RefusalRecord, RefusedPath, glossary,
    palette,
};
use stikk_model::{Capability, Tier};

use crate::app::{Operation, OperationStatus};
use std::cell::Cell;

use crate::text::{identifier, inert, wrap_indented};
use crate::theme::Palette;

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
    /// **The offset is a `Cell` on purpose** (RFC 023 F2). Only the renderer knows the viewport height
    /// and the wrapped content height, so only the renderer can say what "the end" is; it clamps the
    /// offset it was handed and **writes the clamped value back**, which is what makes ↓ held past the
    /// bottom self-correct instead of banking invisible presses that ↑ then has to undo one at a time.
    /// The alternative — teaching `App` the terminal geometry, or making `shell::render` take `&mut App`
    /// — changes a public signature for a scrollbar. Nothing outside the render path writes through
    /// this; `nav_up`/`nav_down` own it through `&mut self` like every other overlay's cursor.
    Glossary {
        /// First content line drawn, counting from zero after wrapping.
        offset: std::cell::Cell<u16>,
    },
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
    /// The command palette (TU-07): a filter and the highlighted match.
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
            Self::Glossary { .. } => " Glossary & Help ",
            Self::Loading { .. } => " Loading ",
            Self::CommitWouldRefuse { .. } | Self::CommitWouldRefuseDeclarations { .. } => {
                " Commit unavailable "
            }
            Self::Operations { .. } => " Background operations ",
            Self::RefPicker { .. } => " Choose ref ",
            Self::Refusal { .. } => " prikk refused ",
            Self::Stale { .. } => " stikk stopped ",
            Self::Palette { .. } => " Command palette ",
            Self::Refusals { .. } => " Recent refusals ",
            Self::Confirmation { .. } => " Confirm ",
            Self::CommitMessage { .. } => " Commit message ",
            Self::CommitResult { .. } => " Commit recorded ",
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

/// Render the top overlay centred over `area`, clearing the region beneath it first (TU-02).
/// Draw the top overlay.
///
/// `marked_ref` is the ref the picker has marked for a comparison (RFC 041 decision 1), or `None`.
/// **It is passed in rather than stored in [`Overlay::RefPicker`]** because the mark outlives the
/// picker -- it survives closing and reopening, and only a ref change ends it -- so `App` owns it
/// and there is one source of truth. This is the shape History's renderer already takes for its
/// block mark, for the same reason.
pub fn render(
    overlay: &Overlay,
    marked_ref: Option<&str>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    match overlay {
        Overlay::Glossary { offset } => render_glossary(offset, palette, frame, area),
        Overlay::Loading { what, .. } => render_loading(what, palette, frame, area),
        Overlay::Operations { operations } => render_operations(operations, palette, frame, area),
        Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        } => render_ref_picker(
            RefPickerRows {
                refs,
                cursor: *cursor,
                unpublished_main: *unpublished_main,
                unpublished_current: *unpublished_current,
                marked_ref,
            },
            palette,
            frame,
            area,
        ),
        Overlay::BundlePath { typed } => render_bundle_path(typed, palette, frame, area),
        Overlay::SavePath {
            what,
            heading,
            typed,
        } => render_save_path(what, heading, typed, palette, frame, area),
        Overlay::SaveResult { lines } => render_save_result(lines, palette, frame, area),
        Overlay::ExportPath { reff, typed } => {
            render_export_path(reff, typed, palette, frame, area);
        }
        Overlay::Refusal { card, cursor } => render_refusal(card, *cursor, palette, frame, area),
        Overlay::Stale {
            operation,
            cause: _,
            headline,
            gloss,
            next_steps,
            cursor,
        } => render_stale(
            operation, headline, gloss, next_steps, *cursor, palette, frame, area,
        ),
        Overlay::CommitWouldRefuse { paths } => {
            render_commit_would_refuse(paths, palette, frame, area);
        }
        Overlay::CommitWouldRefuseDeclarations { declarations } => {
            render_commit_would_refuse_declarations(declarations, palette, frame, area);
        }
        Overlay::Palette {
            filter,
            cursor,
            readiness,
            ref_focused,
        } => render_palette(
            filter,
            *cursor,
            *readiness,
            *ref_focused,
            palette,
            frame,
            area,
        ),
        Overlay::Refusals { records, cursor } => {
            render_refusals(records, *cursor, palette, frame, area);
        }
        Overlay::Confirmation {
            summary,
            tier,
            typed,
            error,
        } => render_confirmation(
            summary,
            *tier,
            typed,
            error.as_deref(),
            palette,
            frame,
            area,
        ),
        Overlay::CommitMessage {
            reff,
            typed,
            messages_persist,
        } => {
            render_commit_message(reff, typed, *messages_persist, palette, frame, area);
        }
        Overlay::CommitResult { result } => render_commit_result(result, palette, frame, area),
        Overlay::SealConsent { reff, acknowledged } => {
            render_seal_consent(reff, *acknowledged, palette, frame, area);
        }
        Overlay::SealResult { result } => render_seal_result(result, palette, frame, area),
        Overlay::RefName {
            kind,
            source,
            typed,
        } => render_ref_name(*kind, source, typed, palette, frame, area),
        Overlay::TagMessage {
            name,
            target,
            typed,
        } => render_tag_message(name, target, typed, palette, frame, area),
        Overlay::RefWriteResult { kind, lines } => {
            render_ref_write_result(*kind, lines, palette, frame, area);
        }
        Overlay::SwitchResult { printed, noted } => {
            render_switch_result(printed, noted, palette, frame, area);
        }
        Overlay::MergeResult { printed } => render_merge_result(printed, palette, frame, area),
        Overlay::ImportResult { printed } => render_import_result(printed, palette, frame, area),
        Overlay::ExportResult { printed } => render_export_result(printed, palette, frame, area),
    }
}

/// The Glossary & Help overlay: key reference, Git → prikk terminology, and — since RFC 023 F2 — the
/// **code explanations**, which shipped authored, reviewed, tested and rendered nowhere.
///
/// **Wrap, scroll and the explanations are one change, not three.** RFC 018 shipped wrapping alone and
/// reverted it: without somewhere to scroll to, wrapping turns *truncated-but-present* into *absent*,
/// and at 80×24 left exactly one of eleven terms reachable. That revert is why the explanations waited
/// too — `code_entries()` roughly doubles this panel's height, so adding them to an unscrollable pane
/// would have made the terminology worse to reach in order to make the codes reachable at all.
///
/// The wrap is stikk's own ([`crate::text::wrap_indented`]) rather than `Paragraph::wrap`, so the line
/// count is exact and the scroll clamp cannot overshoot; see that function for why that matters.
///
/// **Does not use [`Panel`], deliberately** (RFC 024 §2): this is the one overlay the user *scrolls*,
/// so its viewport position is state (`offset`) rather than a consequence of the content's height, and
/// it has no action region to anchor — every row is prose. It satisfies the gate's first assertion by
/// the other branch, advertising `lines X–Y of Z` in its title, which is the idiom `Panel` and
/// `ListPanel` then adopted from it.
fn render_glossary(offset: &Cell<u16>, palette: &Palette, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(palette.fg));
    let region = centered(74, area.height.saturating_sub(2), area);
    // Inside the border. `MIN_WIDTH` is 80 and this overlay is 74 wide, so `text_width` is 72 in every
    // terminal stikk renders in at all — but it is derived rather than written, because a width that is
    // "always N" until someone changes a constant is exactly the shape this project keeps correcting.
    // `- 2` for the border columns, `- 1` for a right gutter: wrapping to the full inner width puts
    // prose flush against the box edge, which reads as if it were cut off.
    let text_width = usize::from(region.width.saturating_sub(3));
    let viewport = region.height.saturating_sub(2);

    let lines = glossary_lines(palette, text_width);

    // The clamp, and the write-back that makes it stick (see `Overlay::Glossary`). `saturating_sub`
    // gives 0 when everything fits, so a tall terminal simply cannot scroll.
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // `NFR-A03`: whatever scrolls this panel is discoverable *from* this panel — the Keys section below
    // names the keys, and the title says where you are, so "there is more" is never something a user has
    // to guess. Only shown when there is somewhere to go.
    let title = if max_offset == 0 {
        " Glossary & Help ".to_string()
    } else {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        format!(
            " Glossary & Help — ↑/↓ to scroll · lines {first}–{last} of {} ",
            lines.len()
        )
    };

    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block.title(title))
            .scroll((scroll, 0)),
        region,
    );
}

/// Every line of the Glossary panel, already wrapped to `text_width`, so the caller's scroll clamp is
/// exact. Split out from the renderer so a test can count and inspect the lines without a terminal.
fn glossary_lines<'a>(palette: &Palette, text_width: usize) -> Vec<Line<'a>> {
    // RFC 018 F1: the line this replaced ("stikk reads prikk; it never writes your repository") was
    // true when written and false the moment stikk gained a mutation — anchored to a feature set, not
    // to anything that could not change. A user-facing claim may rest on an invariant this project
    // enforces (CON-1, C-E2); it must never rest on the set of features that happen to exist today. The
    // replacement rests on CON-1: every repository write happens inside prikk itself, through its
    // public surface, never as a direct write stikk performs — true before commit/seal existed, true
    // after, and true of whatever mutation lands next.
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Every repository write happens inside prikk itself, never in stikk.",
            Style::default().fg(palette.dim),
        )),
        Line::from(""),
        section(palette, "Keys"),
        key_line(palette, "↑/↓ or j/k", "scroll this panel"),
        key_line(palette, "Enter", "open / drill in / activate"),
        key_line(palette, "b", "choose which ref to view"),
        key_line(palette, "w", "changes — worktree vs baseline"),
        key_line(palette, "u", "toggle untracked (in Changes)"),
        key_line(
            palette,
            "v",
            "prikk's own fields (in Patch detail, Compare)",
        ),
        key_line(
            palette,
            "m",
            "mark this block for a comparison (in History)",
        ),
        key_line(palette, "d", "compare — what changed between two points"),
        key_line(palette, "Q", "queue — patches waiting to be sealed"),
        key_line(palette, "V", "verify — prikk's verdict on the repository"),
        key_line(palette, "C", "commit worktree changes"),
        key_line(palette, "S", "seal the active WAL"),
        key_line(
            palette,
            "n",
            "new branch at this ref's tip (in the ref picker)",
        ),
        key_line(
            palette,
            "t",
            "tag this ref's tip, or this block (History, Block detail)",
        ),
        key_line(palette, "x", "close this branch (in the ref picker)"),
        key_line(
            palette,
            "s",
            "switch to this branch — rewrites the worktree (ref picker)",
        ),
        key_line(
            palette,
            "M",
            "merge plan — the marked branch into this one (ref picker)",
        ),
        key_line(
            palette,
            "i",
            "open a bundle file — what it is, and what it would change",
        ),
        key_line(
            palette,
            "I",
            "import the open bundle as a received branch (in Bundle)",
        ),
        key_line(
            palette,
            "E",
            "export this branch to a bundle file (in the ref picker)",
        ),
        key_line(
            palette,
            "T",
            "the maintainer keys this repository trusts (read only)",
        ),
        key_line(
            palette,
            "F",
            "save the selected file as it was at this block (in Block)",
        ),
        key_line(
            palette,
            "A",
            "save all of this block's files into a new folder (in Block)",
        ),
        key_line(palette, ":", "command palette"),
        key_line(palette, "R", "recent refusals"),
        key_line(palette, "o", "background operations"),
        key_line(palette, "r", "refresh — re-read from prikk"),
        key_line(palette, "? / Esc / q", "close · back · quit at root"),
        Line::from(""),
        section(palette, "Git → prikk"),
    ];
    // Review v1 C1: a literal `22` broke the moment a term ("checkout / switch branch", "merge conflict
    // / resolve") ran 24 chars long — the prikk half ran straight into it, unseparated. Computed from
    // the actual terms so a future long one still lines up — `+ 2` for a real gap, since padding a term
    // to *exactly* its own length (the first fix attempt here) leaves zero space before the next column
    // for whichever term is longest, the same defect in miniature.
    let git_column_width = glossary::terminology()
        .iter()
        .map(|term| term.git.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    for term in glossary::terminology() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<git_column_width$}", term.git),
                Style::default().fg(palette.accent),
            ),
            Span::styled(term.prikk, Style::default().fg(palette.fg)),
        ]));
        // Wrapped, not truncated — the RFC 018 fix, now that there is somewhere for the overflow to go.
        for line in wrap_indented(term.note, text_width, "    ") {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(palette.dim),
            )));
        }
    }

    // RFC 035 review v2 §2: quoted content rows are **stikk's own notation**, not prikk's word and
    // not a Git word, so the Glossary names them. A reader meets `\u202e` on a row in Patch detail
    // with nothing else to ask; without this section they would be right to suspect stikk mangled
    // the file.
    for notation in glossary::notations() {
        lines.push(Line::from(""));
        lines.push(section(palette, notation.title));
        for line in wrap_indented(notation.note, text_width, "  ") {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(palette.dim),
            )));
        }
    }

    // RFC 023 F2: `code_entries()` had **zero call sites** outside `glossary.rs`. A refusal card's
    // `glossary: <code>` line named a code with nowhere to read it; this is the somewhere.
    lines.push(Line::from(""));
    lines.push(section(palette, "Codes you may see on a refusal"));
    for entry in glossary::code_entries() {
        lines.push(Line::from(Span::styled(
            format!("  {}", entry.code),
            Style::default().fg(palette.accent),
        )));
        lines.push(Line::from(Span::styled(
            format!("    {}", entry.title),
            Style::default().fg(palette.fg),
        )));
        for line in wrap_indented(entry.explanation, text_width, "    ") {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(palette.dim),
            )));
        }
        if !entry.see_also.is_empty() {
            for line in wrap_indented(
                &format!("see also: {}", entry.see_also.join(" · ")),
                text_width,
                "    ",
            ) {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(palette.dim),
                )));
            }
        }
        lines.push(Line::from(""));
    }
    lines
}

/// Everything the ref picker's renderer needs. One struct rather than five positional arguments,
/// for the reason [`crate::app::CompareFocus`] is one: four of these are `bool` and `usize` side
/// by side, which is where a caller swaps two without the compiler noticing.
struct RefPickerRows<'a> {
    refs: &'a [String],
    cursor: usize,
    unpublished_main: bool,
    unpublished_current: bool,
    /// The ref marked for a comparison (RFC 041 decision 1), or `None`.
    marked_ref: Option<&'a str>,
}

fn render_ref_picker(picker: RefPickerRows<'_>, palette: &Palette, frame: &mut Frame, area: Rect) {
    let RefPickerRows {
        refs,
        cursor,
        unpublished_main,
        unpublished_current,
        marked_ref,
    } = picker;
    // RFC 029 Handoff B §4. The row states a fact — nothing named `heads/main` is published — and makes
    // no claim about what prikk would do there.
    if unpublished_main {
        ListPanel {
            title: " Choose ref ",
            width: 52,
            header: vec![Line::from(Span::styled(
                "  no published refs",
                Style::default().fg(palette.dim),
            ))],
            items: vec![selectable(
                palette,
                true,
                "heads/main (not published)".to_string(),
            )],
            cursor: Some(0),
            style: Style::default().fg(palette.fg),
        }
        .render(frame, area);
        return;
    }
    // **The sentence goes in the header, the tag on the row** (RFC 037 decision 4) — the pattern
    // the `unpublished_main` branch above already uses, and for the same reason: this panel is a
    // fixed-width list with one row per entry, so prose appended to a ref name is prose that gets
    // cut off. The header says it once, in full; the row carries a tag short enough to survive
    // beside a name.
    let header = if unpublished_current {
        vec![Line::from(Span::styled(
            "  the first is the branch you are on, before its first seal",
            Style::default().fg(palette.dim),
        ))]
    } else {
        Vec::new()
    };
    let (items, selection) = if refs.is_empty() {
        (
            vec![Line::from(Span::styled(
                "  no refs reported",
                Style::default().fg(palette.dim),
            ))],
            None,
        )
    } else {
        (
            refs.iter()
                .enumerate()
                .map(|(i, name)| {
                    // **The tag is stikk's own words; the name is the repository's** (RFC 037
                    // decision 4, RFC 036). prikk reports the branch, not a sentence about it, so
                    // stikk says what it means in its own voice rather than inventing a quotation.
                    let label = if unpublished_current && i == 0 {
                        format!("{}  (not yet published)", identifier(name))
                    } else {
                        identifier(name)
                    };
                    // **Two marks in two columns**, as History draws its own (RFC 038 decision
                    // 3): the cursor is where you are, `\u{25cf}` is what you set, and one row can
                    // be both -- which is the ordinary case right before `d`.
                    let marked = marked_ref == Some(name.as_str());
                    selectable_marked(palette, i == cursor, marked, label)
                })
                .collect(),
            Some(cursor),
        )
    };
    // **The title says a mark is set** (RFC 041 decision 1), because the marked row can be
    // scrolled out of sight -- the same reason History's title says it.
    let title = if marked_ref.is_some() {
        " Choose ref \u{00b7} \u{25cf} marked \u{2014} d compares it with the selected ref "
    } else {
        " Choose ref \u{00b7} m marks a ref to compare "
    };
    ListPanel {
        title,
        // Wider than the 52 this panel used, so a ref name and its tag fit side by side. The
        // picker's names were already the longest thing this overlay draws.
        width: 64,
        header,
        items,
        cursor: selection,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

fn render_refusal(
    card: &RefusalCard,
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Two regions, not one `Paragraph` sized by a guess (review v2, C2): `lines.len()` counts
    // *logical* lines, but a long verbatim or gloss wraps to several rows each, and a `+N` headroom
    // guess starves whichever section it under-counts — which turned out to be the next-step list,
    // the one thing this card exists to make actionable, silently clipped since 0.3.0 on any card long
    // enough to wrap (schema-skew's `Upgrade prikk` step among them). Laying the next-steps out as
    // their own region, sized to their exact known height and anchored at the bottom, makes them
    // structurally unclippable: whatever runs out of room under pressure is prose, in the region above,
    // never an action.

    // ① prikk's message, verbatim and inert, in a quoted region distinct from stikk chrome (C-T2b).
    //
    // **Wrapped in stikk** (RFC 024 §2) rather than by `Paragraph::wrap`, so the prose height is a fact
    // and the panel can say when it is showing only part of it.
    //
    // **Every row of a wrapped verbatim line carries the quote bar** (RFC 026 Handoff C §1). It used
    // to stop at the first row, so a refusal long enough to wrap lost the bar exactly where the reader
    // most needs to know they are still inside prikk's words and not stikk's. RFC 024 left it
    // deliberately — fixing framing inside a sizing change was the bundling that increment ruled out —
    // and left it as a one-argument change.
    //
    // **The wrap call is unchanged, on purpose.** `QUOTE_BAR` is four display columns and so was the
    // four-space indent it replaces, so the wrap points cannot move; keeping the same
    // `wrap_indented(.., "    ")` call and re-labelling each row afterwards makes that true by
    // construction rather than by arithmetic, and `the_quote_bar_does_not_reflow_the_text` asserts it.
    let mut prose: Vec<Line> = vec![Line::from(Span::styled(
        "  prikk reported —",
        Style::default().fg(palette.dim),
    ))];
    for raw in card.verbatim.lines() {
        let inert_raw = inert(raw);
        for row in wrap_indented(&inert_raw, REFUSAL_TEXT_WIDTH, QUOTE_INDENT) {
            prose.push(Line::from(vec![
                Span::styled(QUOTE_BAR, Style::default().fg(palette.warn)),
                // The text, with the placeholder indent removed — the bar occupies exactly those
                // columns. Styled `fg` on every row, first and continuation alike: a half-styled
                // continuation reads as a rendering bug rather than as a quote.
                Span::styled(
                    row.get(QUOTE_INDENT.len()..)
                        .unwrap_or_default()
                        .to_string(),
                    Style::default().fg(palette.fg),
                ),
            ]));
        }
    }
    prose.push(Line::from(""));

    // ② the gloss — stikk's own voice, separate and below (ER-02). Absent ⇒ verbatim-only (RR-5).
    if let Some(gloss) = &card.gloss {
        for row in wrap_indented(gloss, REFUSAL_PROSE_WIDTH, "  ") {
            prose.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.dim),
            )));
        }
        prose.push(Line::from(""));
    }

    // ④ glossary links for any named code (FR-111).
    if !card.glossary_codes.is_empty() {
        for row in wrap_indented(
            &format!("glossary: {}", card.glossary_codes.join(", ")),
            REFUSAL_PROSE_WIDTH,
            "  ",
        ) {
            prose.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.accent),
            )));
        }
    }

    // ③ next-steps — stikk-authored, selectable (C-T2b). Their own region, below.
    let mut actions: Vec<Line> = vec![Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    ))];
    for (i, step) in card.next_steps.iter().enumerate() {
        actions.push(selectable(palette, i == cursor, step.label.clone()));
    }
    // The `+ 12` headroom estimate this used to carry is gone: `prose` is wrapped, so `Panel` sizes it
    // exactly instead of guessing generously and hoping.
    Panel {
        title: " prikk refused ",
        width: REFUSAL_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

/// Render [`Overlay::Stale`] — deliberately its own function, not a `Stale`-flavoured
/// [`render_refusal`]: every line here is stikk's own voice, so the label must say so, never "prikk
/// reported" (design-review C1, RFC 013).
#[allow(clippy::too_many_arguments)] // one more than render_refusal: the headline now comes from core
fn render_stale(
    operation: &str,
    headline: &str,
    gloss: &str,
    next_steps: &[NextStep],
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Same two-region layout `render_refusal` now uses (review v3, "before you push"): prose absorbs
    // any shortfall in the height estimate, the next-step list gets its own exact height and cannot be
    // clipped by construction. Today this card's gloss is always a fixed, stikk-authored constant (RFC
    // 013 §5), so the old single-`Paragraph` shape happened to survive at ordinary sizes — but this
    // overlay follows a mutation stopped mid-flight, and a card that clips its one action under
    // pressure is not a card to leave fragile just because nothing has lengthened it yet.

    // ① stikk's own explanation — attributed to stikk, never to prikk (C-T2b/design-review C1).
    let mut prose: Vec<Line> = vec![
        Line::from(Span::styled(
            "  stikk stopped —",
            Style::default().fg(palette.dim),
        )),
        Line::from(vec![
            Span::styled("  │ ", Style::default().fg(palette.warn)),
            Span::styled(
                inert(operation),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            // RFC 030 amendment A3: the headline follows the cause and comes from `stikk-core`
            // (`stale_headline`), so a worktree change never reads as a repository change here.
            Span::styled(headline.to_string(), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
    ];

    // ② the gloss, in stikk's own voice — never quoted as if it were prikk's. Wrapped here (RFC 024).
    for row in wrap_indented(gloss, REFUSAL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }

    // ③ next-steps — stikk-authored, selectable (C-T2b); always exactly one this increment (`Refresh`).
    // Their own region, below.
    let mut actions: Vec<Line> = vec![Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    ))];
    for (i, step) in next_steps.iter().enumerate() {
        actions.push(selectable(palette, i == cursor, step.label.clone()));
    }
    Panel {
        title: " stikk stopped ",
        width: REFUSAL_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

/// Render [`Overlay::CommitWouldRefuseDeclarations`] (RFC 034 §5).
///
/// The same shape as [`render_commit_would_refuse`], with one deliberate difference: **stikk offers no
/// next steps of its own.** prikk's refusal for a declaration already names its measured ways out — run
/// `prikk mv` again, or move the destination back — and RFC 032 A3's stikk-authored suffix is for below
/// prikk 0.44, where prikk said nothing at all. Repeating prikk's advice in stikk's voice would make it
/// stikk's claim (`ER-02`), and inventing a second route would be worse.
fn render_commit_would_refuse_declarations(
    declarations: &[stikk_core::RefusedDeclaration],
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut prose: Vec<Line> = Vec::new();
    for row in wrap_indented(
        "Commit is unavailable: prikk reports that it would refuse the declared rename below, and a \
         refused declaration refuses the whole commit. Nothing was armed.",
        REFUSAL_PROSE_WIDTH,
        "  ",
    ) {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.fg),
        )));
    }
    prose.push(Line::from(""));

    let actions = vec![
        Line::from(Span::styled(
            "  prikk's refusal above names what to do.",
            Style::default().fg(palette.dim),
        )),
        Line::from(Span::styled(
            "  Esc: close",
            Style::default().fg(palette.dim),
        )),
    ];

    // **The budget, exactly as `Panel` will spend it** (RFC 034 Handoff C): it draws
    // `area.height − 2 − actions` prose rows, and the intro above is already wrapped, so what is
    // left for the declarations is a count rather than a guess.
    let prose_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(actions.len());
    let available = prose_rows.saturating_sub(prose.len());
    prose.extend(declaration_lines(declarations, available, palette));

    Panel {
        title: " Commit unavailable ",
        width: REFUSAL_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

/// The refused declarations, fitted to `available` rows **whole** (RFC 034 Handoff C), in
/// [`frozen_lines`]'s shape and for the same reason.
///
/// **This overlay does not scroll.** `↓` falls through it (`app::App::nav_down`), so a row past
/// the fold is a row the reader can never reach — the card's honest `lines 1–20 of 42` told them
/// something was missing and gave them no way to see it. Worse, the cut landed anywhere: the last
/// visible rows could be a declaration's heading and `prikk reported —` with nothing under it.
///
/// So the card drops **whole declarations** and says how many, and **the first is always kept
/// whole** even if it alone overflows: it is the one prikk's commit would stop on first, and a
/// card showing none of them would say nothing at all. That last case is the only one where this
/// card still clips, and `Panel` still says so in the title.
///
/// **Rows are counted as drawn** — cells, not characters (`wrap_cells`) — so a path in a wide
/// script cannot make the count disagree with what is on screen.
fn declaration_lines(
    declarations: &[stikk_core::RefusedDeclaration],
    available: usize,
    palette: &Palette,
) -> Vec<Line<'static>> {
    // One declaration's rows: its heading, stikk's attribution, prikk's reason quoted, a blank.
    let block = |declaration: &stikk_core::RefusedDeclaration| -> Vec<Line<'static>> {
        let mut out: Vec<Line<'static>> = Vec::new();
        // The rule per piece, before formatting (RFC 036 §3): two paths, then stikk's words.
        let heading = format!(
            "declared rename {} → {}",
            identifier(&declaration.old_path),
            identifier(&declaration.new_path)
        );
        for row in wrap_cells(&heading, REFUSAL_TEXT_WIDTH, "  ") {
            out.push(Line::from(Span::styled(
                row,
                Style::default().add_modifier(Modifier::BOLD),
            )));
        }
        out.push(Line::from(Span::styled(
            "  prikk reported —",
            Style::default().fg(palette.dim),
        )));
        for row in wrap_cells(
            &inert(&declaration.reason),
            REFUSAL_TEXT_WIDTH,
            QUOTE_INDENT,
        ) {
            out.push(Line::from(vec![
                Span::styled(QUOTE_BAR, Style::default().fg(palette.warn)),
                Span::styled(
                    row.get(QUOTE_INDENT.len()..)
                        .unwrap_or_default()
                        .to_string(),
                    Style::default().fg(palette.fg),
                ),
            ]));
        }
        out.push(Line::from(""));
        out
    };
    let remainder = |unshown: usize| -> Vec<Line<'static>> {
        wrap_cells(
            &stikk_core::unshown_declarations_line(unshown),
            REFUSAL_PROSE_WIDTH,
            "  ",
        )
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(palette.dim))))
        .collect()
    };

    let blocks: Vec<Vec<Line<'static>>> = declarations.iter().map(block).collect();
    if blocks.iter().map(Vec::len).sum::<usize>() <= available {
        return blocks.into_iter().flatten().collect();
    }
    // Room for the remainder is kept **before** any declaration is laid out: a list that stops
    // without saying so is the defect this shape exists to prevent.
    let foot = remainder(blocks.len());
    let room = available.saturating_sub(foot.len());
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut shown = 0usize;
    for rows in &blocks {
        // **The first is kept whatever its size**, and every later one only if it fits whole.
        if shown > 0 && out.len() + rows.len() > room {
            break;
        }
        out.extend(rows.clone());
        shown += 1;
    }
    out.extend(remainder(blocks.len() - shown));
    out
}

/// Render [`Overlay::CommitWouldRefuse`] (RFC 027 decision 5).
///
/// Three parts, each in one voice:
/// 1. **stikk's statement** of why commit is unavailable — stikk's words, above everything prikk said.
/// 2. **Each refused entry**: its kind and path (inert, hard-wrapped so a long path stays whole), then
///    `prikk reported —` and prikk's reason verbatim, under the quote bar on every wrapped row, exactly
///    as the refusal card quotes prikk (RFC 026 Handoff C §1).
/// 3. **stikk's next steps** ([`stikk_core::would_refuse_next_steps`]) in the panel's action region,
///    which [`Panel`] never clips while a row remains — when there are more refused paths than rows,
///    the prose is what gives way, and the title says so.
fn render_commit_would_refuse(
    paths: &[RefusedPath],
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut prose: Vec<Line> = Vec::new();
    for row in wrap_indented(
        "Commit is unavailable: prikk reports that it would refuse the paths below, and one refused \
         path refuses the whole commit. Nothing was armed.",
        REFUSAL_PROSE_WIDTH,
        "  ",
    ) {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.fg),
        )));
    }
    prose.push(Line::from(""));

    for path in paths {
        let heading = format!(
            "{} {}",
            identifier(path.kind.label()),
            identifier(&path.path)
        );
        for row in wrap_indented(&heading, REFUSAL_TEXT_WIDTH, "  ") {
            prose.push(Line::from(Span::styled(
                row,
                Style::default().add_modifier(Modifier::BOLD),
            )));
        }
        prose.push(Line::from(Span::styled(
            "  prikk reported —",
            Style::default().fg(palette.dim),
        )));
        let reason = inert(&path.reason);
        for row in wrap_indented(&reason, REFUSAL_TEXT_WIDTH, QUOTE_INDENT) {
            prose.push(Line::from(vec![
                Span::styled(QUOTE_BAR, Style::default().fg(palette.warn)),
                Span::styled(
                    row.get(QUOTE_INDENT.len()..)
                        .unwrap_or_default()
                        .to_string(),
                    Style::default().fg(palette.fg),
                ),
            ]));
        }
        prose.push(Line::from(""));
    }

    let mut actions: Vec<Line> = vec![Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    ))];
    for step in stikk_core::would_refuse_next_steps(paths) {
        for (i, row) in wrap_indented(&step, REFUSAL_PROSE_WIDTH, "    ")
            .into_iter()
            .enumerate()
        {
            let text = if i == 0 {
                format!("  · {}", row.get(4..).unwrap_or_default())
            } else {
                row
            };
            actions.push(Line::from(Span::styled(
                text,
                Style::default().fg(palette.fg),
            )));
        }
    }
    actions.push(Line::from(Span::styled(
        "  Esc: close",
        Style::default().fg(palette.dim),
    )));

    Panel {
        title: " Commit unavailable ",
        width: REFUSAL_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

fn render_palette(
    filter: &str,
    cursor: usize,
    readiness: stikk_model::Readiness,
    ref_focused: bool,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let hits = palette::matching(filter);
    // The filter box is chrome the list is read *through* — it stays put while the list scrolls under
    // it, which is why it is the `ListPanel`'s header rather than its first row.
    let header: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("  › ", Style::default().fg(palette.accent)),
            Span::styled(
                if filter.is_empty() {
                    "type to filter…"
                } else {
                    filter
                },
                Style::default().fg(if filter.is_empty() {
                    palette.dim
                } else {
                    palette.fg
                }),
            ),
        ]),
        Line::from(Span::styled(
            "  ─────────",
            Style::default().fg(palette.dim),
        )),
    ];
    let mut lines: Vec<Line> = Vec::new();
    if hits.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no matching command",
            Style::default().fg(palette.dim),
        )));
    }
    for (i, cmd) in hits.iter().enumerate() {
        let selected = i == cursor;
        let reason = cmd.unavailable_reason(readiness, ref_focused);
        let disabled = reason.is_some();
        let name_style = match (selected, disabled) {
            (_, true) => Style::default().fg(palette.dim),
            (true, false) => Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            (false, false) => Style::default().fg(palette.fg),
        };
        let marker = if selected { "▶ " } else { "  " };
        let mut spans = vec![
            Span::styled(marker, Style::default().fg(palette.accent)),
            Span::styled(format!("{:<28}", cmd.name), name_style),
            Span::styled(
                format!("[{}]", cmd.binding),
                Style::default().fg(palette.dim),
            ),
        ];
        if let Some(reason) = reason {
            spans.push(Span::styled(
                format!("  — {reason}"),
                Style::default().fg(palette.warn),
            ));
        }
        lines.push(Line::from(spans));
    }
    ListPanel {
        title: " Command palette ",
        // Widened from 64 (RFC 013 v1) to fit a disabled entry's name + binding + reason on one line
        // without truncating it (RFC 014 §6: "Commit worktree changes" is the first command whose
        // disabled reason is long enough to hit the old width) — `unmet_reason`'s text must actually be
        // readable, not merely present in the `Line`.
        width: 80,
        header,
        items: lines,
        cursor: (!hits.is_empty()).then_some(cursor),
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// A small centred note for a pending overlay-bound request (RFC 010 §5) — the overlay counterpart to
/// `shell`'s `Focus::Loading` rendering for a pending screen.
///
/// **Does not use [`Panel`], and the reason is that it cannot overflow** (RFC 024 §2's requirement to
/// say so rather than leave it inferred): its content is one line of stikk's own text in a fixed
/// three-row box, with nothing that grows — `what` is a `&'static str` from a closed set of request
/// labels. There is no prose to lose and no affordance to anchor. The gate classifies it
/// `Coverage::CannotOverflow` for the same reason, in the same words.
fn render_loading(what: &str, palette: &Palette, frame: &mut Frame, area: Rect) {
    let lines = vec![Line::from(Span::styled(
        format!("  loading {what}…"),
        Style::default().fg(palette.dim),
    ))];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Loading ")
        .style(Style::default().fg(palette.fg));
    let region = centered(40, 3, area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
}

/// The Background Operations listing (TU-01; RFC 010): running and finished requests this session,
/// newest-first. A listing only — no cancel action this increment (RFC 010 decision 6).
fn render_operations(operations: &[Operation], palette: &Palette, frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = if operations.is_empty() {
        vec![Line::from(Span::styled(
            "  no background operations this session",
            Style::default().fg(palette.dim),
        ))]
    } else {
        operations
            .iter()
            .rev()
            .map(|op| {
                let (status, style) = match op.status {
                    OperationStatus::Running => ("running", Style::default().fg(palette.accent)),
                    OperationStatus::Finished { ok: true } => {
                        ("done", Style::default().fg(palette.ok))
                    }
                    OperationStatus::Finished { ok: false } => {
                        ("failed", Style::default().fg(palette.warn))
                    }
                };
                Line::from(vec![
                    Span::styled(
                        format!("  {:<14}", op.label),
                        Style::default().fg(palette.fg),
                    ),
                    Span::styled(status, style),
                ])
            })
            .collect()
    };
    // No cursor: this listing is a snapshot, not a chooser (RFC 010 decision 6 — no cancel action, so
    // nothing to select). It still windows and still says when it is showing only part of the list,
    // which is assertion 1 of the gate.
    ListPanel {
        title: " Background operations ",
        width: 56,
        header: Vec::new(),
        items: lines,
        cursor: None,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// The session's refusal history (`LC-8`) — one row per record, newest first.
///
/// **Checked for `render_refusal`'s quote-bar defect and does not share it** (RFC 026 Handoff C §1,
/// which asked rather than assumed from the name). This renders the **first line only** of each
/// record, through `selectable`, with no quote bar and no wrapping: there is no continuation row for
/// a bar to be missing from. A long headline is truncated at the panel edge instead, which is what a
/// one-row-per-entry index is for — the full message, quoted and wrapped, is one `Enter` away in
/// [`render_refusal`].
fn render_refusals(
    records: &[RefusalRecord],
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let lines: Vec<Line> = if records.is_empty() {
        vec![Line::from(Span::styled(
            "  no refusals this session",
            Style::default().fg(palette.dim),
        ))]
    } else {
        records
            .iter()
            .enumerate()
            .map(|(i, record)| {
                let head = record.verbatim.lines().next().unwrap_or("");
                selectable(palette, i == cursor, inert(head))
            })
            .collect()
    };
    ListPanel {
        title: " Recent refusals ",
        width: 72,
        header: Vec::new(),
        items: lines,
        cursor: (!records.is_empty()).then_some(cursor),
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// The `TU-09` confirmation overlay (RFC 013 §6): restates `summary` from the preview — never a fresh
/// read (RFC 013 §3) — and shows the evidence shape `tier` requires. `typed` is the user's in-progress
/// input for a tier-3-typed exact match (unused otherwise); `error` is an inline message from a
/// declined confirmation attempt (`StikkError::Declined`), shown in place rather than as a separate
/// popup (RFC 013 §4 — the user is still mid-confirmation, not facing a new failure).
///
/// `target_ids`, `target_name`, and the user's own `typed` input are rendered through [`inert`]
/// (`C-T2a`/`C-T4e`): the first two are prikk-authoritative and a hostile repository must not be able
/// to forge chrome through them; `typed` is defensive against a pasted control character. `operation`
/// and `consequence` are stikk's own words and need no such treatment.
fn render_confirmation(
    summary: &ConfirmationSummary,
    tier: Tier,
    typed: &str,
    error: Option<&str>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("  {}", summary.operation),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if !summary.target_ids.is_empty()
        || summary.branch_notice.is_some()
        || summary.history_notice.is_some()
    {
        if !summary.target_ids.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Targets:",
                Style::default().fg(palette.dim),
            )));
        }
        // **The role is chrome, the id is repository text** (RFC 047 review v1 C1): the label is
        // written in `stikk-core` and drawn as itself, the value goes through `identifier` like
        // every other repository string. They are two spans so the escape cannot reach the one
        // and skip the other. The column is computed from the labels actually present, so a
        // longer word later still lines up.
        let role_width = summary
            .target_ids
            .iter()
            .filter_map(|target| target.role)
            .map(|role| role.chars().count())
            .max();
        for target in &summary.target_ids {
            let value = Span::styled(identifier(&target.id), Style::default().fg(palette.accent));
            match (role_width, target.role) {
                (Some(width), Some(role)) => lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {role:<width$}  "),
                        Style::default().fg(palette.dim),
                    ),
                    value,
                ])),
                // A labelled list with an unlabelled row keeps the column, so the ids still line
                // up under each other rather than one of them stepping left.
                (Some(width), None) => lines.push(Line::from(vec![
                    Span::raw(format!("    {:width$}  ", "")),
                    value,
                ])),
                (None, _) => lines.push(Line::from(vec![Span::raw("    "), value])),
            }
        }
        // Safeguard 3 (RFC 029 Handoff B §5): directly under the targets and in warn, because it is about
        // where the change goes — above the counts and `C-S2`'s line. The words carry it in a monochrome
        // terminal (`NFR-A03`); the colour only adds. Inert, since it names refs and may carry prikk's
        // own text (`C-T2a`).
        if let Some(notice) = &summary.branch_notice {
            for row in wrap_indented(&inert(notice), PANEL_PROSE_WIDTH, "    ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.warn),
                )));
            }
        }
        // RFC 032 decision 5, A2: beside the branch notice, in warn — also about where the change goes.
        if let Some(notice) = &summary.history_notice {
            for row in wrap_indented(&inert(notice), PANEL_PROSE_WIDTH, "    ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.warn),
                )));
            }
        }
        lines.push(Line::from(""));
    }

    // **Labelled asides, outside Targets** (RFC 050 review v1 C1). A row under `Targets:` reads
    // as something the operation acts on; these are facts about something it does not touch, so
    // each carries its own label and sits in its own block. The label is stikk's chrome and the
    // name is repository text, drawn as two spans so the escape cannot reach one and skip the
    // other -- the rule the target rows follow.
    for aside in &summary.asides {
        let mut heading = vec![Span::styled(
            format!("  {} ", aside.before),
            Style::default().fg(palette.dim),
        )];
        if !aside.name.is_empty() {
            heading.push(Span::styled(
                identifier(&aside.name),
                Style::default().fg(palette.accent),
            ));
            heading.push(Span::raw(" "));
        }
        heading.push(Span::styled(
            aside.after.to_string(),
            Style::default().fg(palette.dim),
        ));
        lines.push(Line::from(heading));
        for sentence in &aside.body {
            for row in wrap_indented(&inert(sentence), PANEL_PROSE_WIDTH, "    ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.fg),
                )));
            }
        }
        lines.push(Line::from(""));
    }

    if !summary.counts.is_empty() {
        let counts_line = summary
            .counts
            .iter()
            .map(|(label, count)| format!("{count} {label}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(Span::styled(
            format!("  {counts_line}"),
            Style::default().fg(palette.fg),
        )));
        // RFC 032 decision 4: what the `renames` count means, then each declaration prikk will not author
        // as a rename, in warn. Prose rows like the rest, so they never displace the consequence or footer.
        if let Some(note) = &summary.rename_note {
            for row in wrap_indented(note, PANEL_PROSE_WIDTH, "  ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.dim),
                )));
            }
        }
        for notice in &summary.declaration_notices {
            for row in wrap_indented(&inert(notice), PANEL_PROSE_WIDTH, "  ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.warn),
                )));
            }
        }
        lines.push(Line::from(""));
    }
    // RFC 028 decision 4: what the seal freezes goes directly under the counts line. It is spliced in last,
    // once everything it must never displace has been laid out (Handoff B §4).
    let frozen_at = if summary.counts.is_empty() {
        lines.len()
    } else {
        lines.len() - 1
    };

    // **An operation that consumes no capability says so** (RFC 045 §3c). A branch switch signs
    // nothing, and prikk needs no key for it — an absent row would read as an omission, and a
    // `Consumes:` row naming a capability would state a requirement stikk invented.
    lines.push(Line::from(Span::styled(
        match summary.capability {
            Some(capability) => format!("  Consumes: {}", capability_label(capability)),
            None => "  Consumes: no signing key".to_string(),
        },
        Style::default().fg(palette.dim),
    )));
    // `FL-05` step 5, unmet since before 0.4.0: the confirmation must name **which key** will sign,
    // not only which capability is consumed. `FL-06` asks the same of seal, as amended by RFC 023
    // Handoff B — RFC 016 decision 3 removed a typed key-id *act*, not the information.
    //
    // Absent renders as nothing at all. There is no placeholder line, because an operation cannot
    // reach a confirmation without the readiness this id accompanies (`capability_gate` refuses on
    // `NotReady` first), so any text here would be stikk inventing a fact.
    if let Some(id) = &summary.signing_key_id {
        lines.push(Line::from(vec![
            Span::styled("  Signing key id: ", Style::default().fg(palette.dim)),
            // An **identifier** (RFC 036 decision 3), like every other id: it is a label a user
            // or a repository's conventions chose, and two different key ids must never look alike
            // in the one place that names which key will sign.
            Span::styled(identifier(id), Style::default().fg(palette.accent)),
        ]));
        // **The id alone is not a statement** (RFC 026 §5). Naming it plainly claims a binding, and on
        // a fresh repository there is none yet — which is the first commit every new user makes.
        for row in claim_rows(summary.signing_key_claim, palette) {
            lines.push(row);
        }
        // `C-S2`. **Above the consequence and in warn**, because it is not a note about the key — it
        // is a statement that this signature would be worthless, and it must not read as one more
        // detail in a list the user is scanning past.
        if summary.signing_key_is_published_example {
            for row in wrap_indented(
                "This key is published in prikk's own documentation as an example. It is public, so \
                 anyone can forge this signature — generate your own key before signing anything you \
                 intend to be trusted.",
                PANEL_PROSE_WIDTH,
                "    ",
            ) {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.warn),
                )));
            }
        }
    }
    lines.push(Line::from(""));

    // **Cautions about what confirming will produce** (RFC 053 decision 5), above the
    // consequence and in warn. Unconditional, unlike `declaration_notices`, which is drawn only
    // inside the counts block: a card with no counts carried those and showed nothing.
    if !summary.notices.is_empty() {
        for notice in &summary.notices {
            for row in wrap_indented(notice, PANEL_PROSE_WIDTH, "  ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.warn),
                )));
            }
        }
        lines.push(Line::from(""));
    }

    // The consequence — stikk's own words. The operation's own plan/content (prikk's verbatim text,
    // for Class A previews — RFC 013 F1) lives in the preview view itself, never restated here
    // (RFC 013 Q1: the confirmation restates the fixed summary, not the preview).
    //
    // **Wrapped here, in stikk** (RFC 024 §2). This is the line that made seal's confirmation ship for
    // three releases without its confirm affordance: seal's `Unknown` maintainer consequence
    // is one *logical* line that draws as four rows, the old `lines.len() + 4` headroom was spent on
    // it, and the footer fell outside the box at every terminal height. Wrapping it makes the count a
    // fact — and the footer is in `actions` below, so even a future mis-measurement costs prose.
    for line in wrap_indented(&summary.consequence, PANEL_PROSE_WIDTH, "  ") {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(palette.fg),
        )));
    }
    // RFC 044 decision 6: the tag message the write will carry, restated under the consequence
    // that said what the write does. **Inert and on its own rows**, because unlike everything else
    // on this card it is text the user wrote, and it is about to become repository content.
    if let Some(message) = &summary.message_note {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Message:",
            Style::default().fg(palette.dim),
        )));
        for row in wrap_indented(&inert(message), PANEL_PROSE_WIDTH, "    ") {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.fg),
            )));
        }
    }
    lines.push(Line::from(""));

    // The affordances, anchored: whatever runs out of room is prose, never the thing the user must
    // press (RFC 016 C2's rule, now held structurally for this overlay too).
    let mut actions: Vec<Line> = Vec::new();
    match tier {
        Tier::One => {} // never reaches this overlay in practice — tier 1 has no confirmation
        Tier::Two | Tier::Three => {
            actions.push(Line::from(Span::styled(
                "  Enter to confirm · Esc to cancel",
                Style::default().fg(palette.accent),
            )));
        }
        Tier::ThreeTyped => {
            let target = summary.target_name.as_deref().unwrap_or("");
            actions.push(Line::from(Span::styled(
                format!("  Type \"{}\" to confirm:", identifier(target)),
                Style::default().fg(palette.dim),
            )));
            actions.push(Line::from(vec![
                Span::styled("  › ", Style::default().fg(palette.accent)),
                Span::styled(inert(typed), Style::default().fg(palette.fg)),
            ]));
        }
    }

    // An inline error belongs with the affordances, not above them: it is the reason the user is being
    // asked again, and it is useless if it is the row that falls off.
    if let Some(error) = error {
        for line in wrap_indented(error, PANEL_PROSE_WIDTH, "  ") {
            actions.push(Line::from(Span::styled(
                line,
                Style::default().fg(palette.warn),
            )));
        }
    }

    // **The list is the one part of this card that gives up height** (RFC 028 Handoff B §4). Exact, not
    // estimated: `Panel` draws `area.height − 2 − actions` prose rows when the card does not fit, and every
    // line above is already wrapped, so what remains for the list is a count. The consequence, its
    // trust-refusal warning and the affordances are laid out first and never displaced.
    if let Some(freezes) = &summary.freezes {
        let prose_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(actions.len());
        let available = prose_rows.saturating_sub(lines.len());
        let list = frozen_lines(freezes, available, palette);
        lines.splice(frozen_at..frozen_at, list);
    }
    // RFC 045 §3c: a switch's file plan gives up height the same way, in the same place, for the
    // same reason — and **never silently**: what it gives up, it says.
    if let Some(plan) = &summary.materializes {
        let prose_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(actions.len());
        let available = prose_rows.saturating_sub(lines.len());
        let list = plan_lines(plan, available, palette);
        lines.splice(frozen_at..frozen_at, list);
    }
    // RFC 047 decision 6: prikk's merge plan gives up height in the same place, by the same
    // rule, and says what it gave up.
    if let Some(plan) = &summary.merge_plan {
        let prose_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(actions.len());
        let available = prose_rows.saturating_sub(lines.len());
        let list = merge_plan_lines(plan, available, palette);
        lines.splice(frozen_at..frozen_at, list);
    }

    Panel {
        title: " Confirm ",
        width: CONFIRM_WIDTH,
        prose: lines,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

/// The rows naming the files a switch will write and delete, fitted to `available` rows
/// (RFC 045 §3c) — [`frozen_lines`]'s shape, for the same reason and with the same rule.
///
/// - **Every row fits:** every row.
/// - **Otherwise:** as many rows as fit with room kept for the remainder line, then that line —
///   its count exactly the rows not shown, in [`stikk_core::unshown_paths_line`]'s words.
/// - **Not even one row and the remainder fit:** the remainder line alone.
///
/// **The mark is stikk's and the path is prikk's.** The mark is a fixed word this module styles;
/// the path is repository text and is rendered inert (`C-T2a`), wrapped by cells so a path in a
/// wide script cannot draw past the card while counting as one row.
fn plan_lines(
    plan: &stikk_core::FilePlan,
    available: usize,
    palette: &Palette,
) -> Vec<Line<'static>> {
    let rows = plan.rows();
    if rows.is_empty() {
        return Vec::new();
    }
    let drawn = |(mark, path): &(&'static str, &str)| -> Vec<Line<'static>> {
        let style = if *mark == "deleted" {
            Style::default().fg(palette.warn)
        } else {
            Style::default().fg(palette.fg)
        };
        let head = format!("    {mark}  ");
        let text = inert(path);
        if Span::raw(text.as_str()).width() + Span::raw(head.as_str()).width() <= PANEL_TEXT_WIDTH {
            return vec![Line::from(vec![
                Span::styled(head, Style::default().fg(palette.dim)),
                Span::styled(text, style),
            ])];
        }
        // Too wide beside its mark: the mark takes its own row so the path keeps the full width.
        let mut out = vec![Line::from(Span::styled(
            format!("    {mark}"),
            Style::default().fg(palette.dim),
        ))];
        for row in wrap_cells(&text, PANEL_TEXT_WIDTH, "      ") {
            out.push(Line::from(Span::styled(row, style)));
        }
        out
    };
    let remainder = |unshown: usize| -> Vec<Line<'static>> {
        wrap_indented(
            &stikk_core::unshown_paths_line(unshown, rows.len()),
            PANEL_PROSE_WIDTH,
            "    ",
        )
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(palette.dim))))
        .collect()
    };
    let every: Vec<Vec<Line<'static>>> = rows.iter().map(drawn).collect();
    if every.iter().map(Vec::len).sum::<usize>() <= available {
        return every.into_iter().flatten().collect();
    }
    // Room is kept for the remainder line **first**: a list that silently stops is the defect
    // this whole shape exists to prevent.
    let foot = remainder(rows.len());
    let room = available.saturating_sub(foot.len());
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut shown = 0usize;
    for row in &every {
        if out.len() + row.len() > room {
            break;
        }
        out.extend(row.clone());
        shown += 1;
    }
    out.extend(remainder(rows.len() - shown));
    out
}

/// prikk's merge plan under the confirmation card, fitted to `available` rows **whole lines
/// first** (RFC 047 decision 6), in [`plan_lines`]'s shape and for the same reason.
///
/// A plan line is `label: value`, and prikk's indented report rows are part of its layout, so a
/// line wraps hard rather than reflowing and is kept or dropped **entire**. Room for the
/// remainder line is kept first: a plan that silently stops is the defect this shape exists to
/// prevent, and a confirmation showing half a plan is worse than one showing none and saying so.
fn merge_plan_lines(plan: &[String], available: usize, palette: &Palette) -> Vec<Line<'static>> {
    if plan.is_empty() {
        return Vec::new();
    }
    let drawn = |line: &String| -> Vec<Line<'static>> {
        if line.trim().is_empty() {
            return vec![Line::from("")];
        }
        crate::text::wrap_verbatim(&inert(line), PANEL_TEXT_WIDTH, "    ")
            .into_iter()
            .map(|row| Line::from(Span::styled(row, Style::default().fg(palette.fg))))
            .collect()
    };
    let remainder = |unshown: usize| -> Vec<Line<'static>> {
        wrap_indented(
            &stikk_core::unshown_plan_lines_line(unshown, plan.len()),
            PANEL_PROSE_WIDTH,
            "    ",
        )
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(palette.dim))))
        .collect()
    };
    let every: Vec<Vec<Line<'static>>> = plan.iter().map(drawn).collect();
    if every.iter().map(Vec::len).sum::<usize>() <= available {
        return every.into_iter().flatten().collect();
    }
    let foot = remainder(plan.len());
    let room = available.saturating_sub(foot.len());
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut shown = 0usize;
    for row in &every {
        if out.len() + row.len() > room {
            break;
        }
        out.extend(row.clone());
        shown += 1;
    }
    out.extend(remainder(plan.len() - shown));
    out
}

/// The rows naming what a seal freezes, fitted to `available` rows (RFC 028 Handoff B §4), in core's words.
///
/// - **Every row fits:** every row, then the foot if any.
/// - **Otherwise:** as many rows as fit with room kept for the remainder line, then that line — its count
///   exactly the rows not shown — then the foot, **kept whole before any patch row is given up**, since it
///   says what the rows cannot.
/// - **Not even one row and the remainder fit:** the remainder line alone (`and {n} more — …`), with the foot
///   only if both fit.
/// - **Unlisted** (below prikk 0.39): one line, which never yields.
///
/// Every row is repository text, so every one is inert (`C-T2a`), and wrapped here so the count is exact.
fn frozen_lines(
    freezes: &stikk_core::FrozenPatches,
    available: usize,
    palette: &Palette,
) -> Vec<Line<'static>> {
    // A row that fits is drawn exactly as core wrote it: wrapping joins words with single spaces, which
    // would turn core's `{short id}  {message}` into one space. Only a row too wide for the card is wrapped
    // — and either way the rows are counted as drawn, so the fit stays exact. **Widths are cells** (review
    // v1 §2.2): a message in a wide script is two cells a character, and counting characters would call a
    // row fitting that draws past the card, so the rows drawn would stop matching the rows counted.
    let wrap = |text: &str| {
        let text = inert(text);
        if Span::raw(text.as_str()).width() + 4 <= PANEL_PROSE_WIDTH {
            vec![format!("    {text}")]
        } else {
            wrap_cells(&text, PANEL_PROSE_WIDTH, "    ")
        }
    };
    let styled = |rows: Vec<String>, style: Style| -> Vec<Line<'static>> {
        rows.into_iter()
            .map(|row| Line::from(Span::styled(row, style)))
            .collect()
    };
    let (rows, foot) = match freezes {
        stikk_core::FrozenPatches::Unlisted(line) => {
            return styled(wrap(line), Style::default().fg(palette.dim));
        }
        stikk_core::FrozenPatches::Listed { rows, foot } => (rows, foot),
    };
    let wrapped: Vec<Vec<String>> = rows.iter().map(|row| wrap(row)).collect();
    let foot_rows = foot.as_deref().map(wrap).unwrap_or_default();
    let every_row: usize = wrapped.iter().map(Vec::len).sum();

    let mut out = Vec::new();
    if every_row + foot_rows.len() <= available {
        for row in wrapped {
            out.extend(styled(row, Style::default().fg(palette.fg)));
        }
        out.extend(styled(foot_rows, Style::default().fg(palette.dim)));
        return out;
    }

    let total = rows.len();
    let remainder = |unshown: usize| wrap(&stikk_core::unshown_patches_line(unshown, total));
    let mut shown = 0;
    let mut used = 0;
    for row in &wrapped {
        let with_row = used + row.len();
        if with_row + remainder(total - shown - 1).len() + foot_rows.len() > available {
            break;
        }
        used = with_row;
        shown += 1;
    }
    for row in wrapped.into_iter().take(shown) {
        out.extend(styled(row, Style::default().fg(palette.fg)));
    }
    let remainder_rows = remainder(total - shown);
    let keep_foot = shown > 0 || remainder_rows.len() + foot_rows.len() <= available;
    out.extend(styled(remainder_rows, Style::default().fg(palette.dim)));
    if keep_foot {
        out.extend(styled(foot_rows, Style::default().fg(palette.dim)));
    }
    out
}

/// `text` wrapped to `width` terminal **cells**, every row prefixed with `indent` (RFC 028 B review v1 §2.2).
///
/// `wrap_indented` counts characters, which is exact for the Latin text and box characters most of stikk
/// renders, and wrong for a seal row whose message is in a wide script. Here words are kept whole where
/// they fit, a word wider than a line is split between characters, and a character wider than the whole
/// line still takes a row of its own rather than looping. Returns at least one row.
fn wrap_cells(text: &str, width: usize, indent: &str) -> Vec<String> {
    let cells = |s: &str| Span::raw(s).width();
    let available = width.saturating_sub(cells(indent)).max(1);
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word;
        while cells(word) > available {
            if !current.is_empty() {
                rows.push(format!("{indent}{current}"));
                current.clear();
            }
            let mut split_at = word.len();
            let mut head_cells = 0;
            for (index, ch) in word.char_indices() {
                let ch_cells = Span::raw(ch.encode_utf8(&mut [0; 4]).to_string()).width();
                if head_cells + ch_cells > available && index > 0 {
                    split_at = index;
                    break;
                }
                head_cells += ch_cells;
            }
            rows.push(format!("{indent}{}", &word[..split_at]));
            word = &word[split_at..];
        }
        if word.is_empty() {
            continue;
        }
        let joined = if current.is_empty() {
            cells(word)
        } else {
            cells(&current) + 1 + cells(word)
        };
        if joined > available && !current.is_empty() {
            rows.push(format!("{indent}{current}"));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() || rows.is_empty() {
        rows.push(format!("{indent}{current}"));
    }
    rows
}

/// `FL-05` step 2: the commit message prompt, required non-empty (`UD-01`) — its own step, before any
/// confirmation exists to restate (RFC 014 §3). `reff` is shown so the prompt names what it targets.
fn render_commit_message(
    reff: &str,
    typed: &str,
    messages_persist: bool,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // `UD-01` retired at prikk 0.32 (RFC 015 F2): stikk supports prikk on both sides of that boundary,
    // so neither a blanket "not persisted" nor a blanket "persisted" claim is true — this session's own
    // handshake decides which sentence is honest here.
    let notice = if messages_persist {
        "  A message is required — it is stored and will appear in `prikk log`."
    } else {
        "  A message is required — core does not yet persist it (it will not appear in `prikk log`)."
    };
    // RFC 024 F2: the notice is one logical line that wraps to two rows at this width, which cost the
    // footer its place under `lines.len() + 2`. Wrapped here so the count is exact.
    let mut prose = vec![
        Line::from(Span::styled(
            format!("  Commit to {}", identifier(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for line in wrap_indented(notice.trim_start(), PANEL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            line,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    // The typed input and the footer are affordances, not prose: the prompt is the whole point of this
    // overlay, and a user who cannot see what they have typed cannot decide whether to press Enter.
    let actions = vec![
        Line::from(vec![
            Span::styled("  › ", Style::default().fg(palette.accent)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to continue · Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title: " Commit message ",
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// `FL-05` step 4's tail: prikk's own commit result, verbatim (`C-T4a`/`C-T4c`) — the patch id and
/// counts prikk reported, every changed path, and every `note:` line, never summarised (`ER-02`).
fn render_commit_result(
    result: &stikk_prikk::CommitResult,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  recorded worktree patch in active WAL",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(labelled(
        "patch id: ",
        &identifier(&result.patch_id),
        SEAL_TEXT_WIDTH,
        Style::default().fg(palette.dim),
        Style::default().fg(palette.accent),
    ));
    lines.push(Line::from(Span::styled(
        format!(
            "  operations {} · referenced blobs {} · text edits {}",
            result.operations, result.referenced_blobs, result.text_edits
        ),
        Style::default().fg(palette.fg),
    )));
    if !result.changes.is_empty() {
        lines.push(Line::from(""));
        for change in &result.changes {
            // A repository path can be long; wrapped with a deeper hanging indent so a continued path
            // is visibly a continuation rather than a second entry.
            for row in wrap_indented(
                &format!(
                    "{} {}",
                    identifier(&change.operation),
                    identifier(&change.path)
                ),
                SEAL_TEXT_WIDTH,
                "    ",
            ) {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.fg),
                )));
            }
        }
    }
    if !result.notes.is_empty() {
        lines.push(Line::from(""));
        for note in &result.notes {
            for row in wrap_indented(&inert(note), SEAL_TEXT_WIDTH, "  ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.dim),
                )));
            }
        }
    }
    lines.push(Line::from(""));
    Panel {
        title: " Commit recorded ",
        width: SEAL_WIDTH,
        prose: lines,
        // The dismiss hint is the only affordance here; a result card the user cannot leave is the
        // worst thing this overlay could clip.
        actions: vec![Line::from(Span::styled(
            "  Enter · Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// `FL-06`'s consent step (RFC 016 §8): its own act, unchecked and undefaultable. The copy is
/// [`stikk_core::SEAL_CONSENT_COPY`] verbatim — stikk's own words, not repository content, so it needs
/// no `inert()` — tied to the ceremony's own irreversibility, never to prikk's `--allow-no-audit` flag
/// (that flag is scaffolding prikk may retire; a claim tied to it would go false the day it does).
fn render_seal_consent(
    reff: &str,
    acknowledged: bool,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let (mark, mark_style) = if acknowledged {
        ("[x]", Style::default().fg(palette.ok))
    } else {
        ("[ ]", Style::default().fg(palette.warn))
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  Seal {}", identifier(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    // RFC 024: `SEAL_CONSENT_COPY` is one logical line that wraps to several rows, which is why this
    // renderer carried a hand-tuned `+ 6` — added after its own render tests caught the mark and the
    // Enter hint being clipped. Wrapping it makes the count exact and the tuning unnecessary.
    for row in wrap_indented(stikk_core::SEAL_CONSENT_COPY, SEAL_PROSE_WIDTH, "  ") {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.fg),
        )));
    }
    lines.push(Line::from(""));

    // The acknowledgement mark and the key hint are the ceremony's two affordances — the second act
    // RFC 016 decision 3 made distinct. Anchored: they are the last thing that may be lost, not the
    // first.
    let actions = vec![
        Line::from(vec![
            Span::styled(format!("  {mark} "), mark_style),
            Span::styled(
                "I understand — Space to toggle",
                Style::default().fg(palette.dim),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            if acknowledged {
                "  Enter to seal · Esc to cancel"
            } else {
                "  Space to acknowledge before Enter will do anything · Esc to cancel"
            },
            Style::default().fg(if acknowledged {
                palette.accent
            } else {
                palette.dim
            }),
        )),
    ];
    Panel {
        title: " Before you seal ",
        width: SEAL_WIDTH,
        prose: lines,
        actions,
        style: Style::default().fg(palette.warn),
    }
    .render(frame, area);
}

/// `FL-06`'s tail: prikk's own seal result, verbatim (`C-T4a`/`C-T4c`) — the block id, the ref's new
/// `RefState`, and every `note:` line, never summarised (`ER-02`).
fn render_seal_result(
    result: &stikk_prikk::SealResult,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  sealed active WAL into block",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(labelled(
        "block id: ",
        &identifier(&result.block_id),
        SEAL_TEXT_WIDTH,
        Style::default().fg(palette.dim),
        Style::default().fg(palette.accent),
    ));
    lines.push(Line::from(Span::styled(
        format!("  patches {}", result.patches),
        Style::default().fg(palette.fg),
    )));
    lines.extend(labelled(
        &format!("{} RefState: ", identifier(&result.reff)),
        &identifier(&result.ref_state),
        SEAL_TEXT_WIDTH,
        Style::default().fg(palette.dim),
        Style::default().fg(palette.accent),
    ));
    if !result.notes.is_empty() {
        lines.push(Line::from(""));
        for note in &result.notes {
            for row in wrap_indented(&inert(note), SEAL_TEXT_WIDTH, "  ") {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.dim),
                )));
            }
        }
    }
    lines.push(Line::from(""));
    Panel {
        title: " Sealed ",
        width: SEAL_WIDTH,
        prose: lines,
        // The dismiss hint is the only affordance here; a result card the user cannot leave is the
        // worst thing this overlay could clip.
        actions: vec![Line::from(Span::styled(
            "  Enter · Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 044 decision 5: the name prompt for a new branch or tag — the prefix fixed, the rest typed.
///
/// **The prefix is shown, never typed.** prikk refuses an unprefixed name (F2), and a prompt that
/// made the user remember `heads/` would be collecting that refusal on purpose. Everything after it
/// is the user's: stikk applies no rule of its own beyond refusing to send nothing at all.
fn render_ref_name(
    kind: RefNameKind,
    source: &str,
    typed: &str,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let (heading, notice) = match kind {
        RefNameKind::Branch => (
            "Branch from",
            "A name is required. It will start at that ref's tip; nothing in the worktree \
             changes. stikk adds no naming rules of its own — prikk judges the name, and says so \
             in its own words.",
        ),
        RefNameKind::Tag => (
            "Tag",
            "A name is required. The tag will point there and nothing else moves. stikk adds no \
             naming rules of its own — prikk judges the name, and says so in its own words.",
        ),
    };
    let mut prose = labelled(
        &format!("{heading} "),
        &identifier(source),
        PANEL_TEXT_WIDTH,
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        Style::default().fg(palette.accent),
    );
    prose.push(Line::from(""));
    for row in wrap_indented(notice, PANEL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    // The prompt is the whole point of this overlay, so it sits with the footer in the region that
    // is never clipped — the commit message prompt's own reasoning.
    let actions = vec![
        Line::from(vec![
            Span::styled("  › ", Style::default().fg(palette.accent)),
            Span::styled(kind.prefix(), Style::default().fg(palette.dim)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to continue · Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title: match kind {
            RefNameKind::Branch => " New branch ",
            RefNameKind::Tag => " New tag ",
        },
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 050 decision 3: the path of a bundle file to open.
///
/// **No prefix, and nothing is validated here.** A ref prompt fixes `heads/` or `tags/` and this
/// one fixes nothing: what a path may contain is the filesystem's business, and whether the file
/// is a bundle is prikk's. The notice says what will happen — a read, and nothing written —
/// because that is the question a reader typing a path into a version-control tool is actually
/// asking.
fn render_bundle_path(typed: &str, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose: Vec<Line> = vec![Line::from(Span::styled(
        "Open a bundle file",
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
    ))];
    prose.push(Line::from(""));
    for row in wrap_indented(
        "A path is required. stikk asks prikk what the file is and what importing it would \
         change, and writes nothing. A relative path is taken from the directory stikk was \
         started in, and the view shows the path it used.",
        PANEL_PROSE_WIDTH,
        "  ",
    ) {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    let actions = vec![
        Line::from(vec![
            Span::styled("  › ", Style::default().fg(palette.accent)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to open · Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title: " Open a bundle ",
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 050 decision 5: where to write a branch's bundle, **prefilled** with stikk's suggestion.
///
/// **The suggestion is outside the repository by construction** (the repository's parent), so a
/// reader who accepts it never meets the one rule stikk adds. The notice says both rules anyway,
/// because the reader editing the path is the one who needs them.
fn render_export_path(reff: &str, typed: &str, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose = labelled(
        "Export ",
        &identifier(reff),
        PANEL_TEXT_WIDTH,
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        Style::default().fg(palette.accent),
    );
    prose.push(Line::from(""));
    for row in wrap_indented(
        "A path is required. stikk writes nowhere inside the repository -- the bundle would sit \
         in your worktree as an untracked file -- and never over a file that is already there. \
         A relative path is taken from the directory stikk was started in.",
        PANEL_PROSE_WIDTH,
        "  ",
    ) {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    let actions = vec![
        Line::from(vec![
            Span::styled("  \u{203a} ", Style::default().fg(palette.accent)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to continue \u{b7} Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title: " Export a bundle ",
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 053 decision 4: where to write a copy, **prefilled** with stikk's suggestion.
///
/// **The suggestion is outside the repository by construction**, so a reader who accepts it
/// never meets the one rule stikk adds. The notice says both rules anyway, because the reader
/// editing the path is the one who needs them.
fn render_save_path(
    what: &SavePathKind,
    heading: &str,
    typed: &str,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let (label, title, notice) = match what {
        SavePathKind::File { .. } => (
            "Save ",
            " Save this file ",
            "A path is required. stikk writes nowhere inside the repository -- the copy would \
             sit in your worktree as untracked files -- and never over a path that is already \
             there. A relative path is taken from the directory stikk was started in. Nothing \
             in the repository or your worktree changes.",
        ),
        SavePathKind::All { .. } => (
            "Save every file at ",
            " Save this block's files ",
            "A folder is required, and stikk creates it. It writes nowhere inside the \
             repository, and never over a path that is already there. A relative path is taken \
             from the directory stikk was started in. Nothing in the repository or your \
             worktree changes.",
        ),
    };
    let mut prose = labelled(
        label,
        &identifier(heading),
        PANEL_TEXT_WIDTH,
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        Style::default().fg(palette.accent),
    );
    prose.push(Line::from(""));
    for row in wrap_indented(notice, PANEL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    let actions = vec![
        Line::from(vec![
            Span::styled("  \u{203a} ", Style::default().fg(palette.accent)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to continue \u{b7} Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title,
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 053 decision 6: what stikk wrote.
///
/// **stikk's own sentence, and no quote bar** — unlike every other result card here. prikk
/// prints nothing at all on a successful `cat --output`, and `extract`'s answer is a JSON report
/// rather than prose, so there is nothing of prikk's to show verbatim and a `prikk reported --`
/// heading over stikk's own words would be a lie.
fn render_save_result(lines: &[String], palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose: Vec<Line> = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if at > 0 {
            prose.push(Line::from(""));
        }
        let style = if at == 0 {
            Style::default().fg(palette.fg)
        } else {
            // The provisional notice, which is a caution rather than a result.
            Style::default().fg(palette.warn)
        };
        for row in wrap_indented(&inert(line), SEAL_PROSE_WIDTH, "  ") {
            prose.push(Line::from(Span::styled(row, style)));
        }
    }

    Panel {
        title: " Saved ",
        width: SEAL_WIDTH,
        prose,
        actions: vec![Line::from(Span::styled(
            "  Enter \u{b7} Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 044 decision 5: a tag's message, prompted after its name. **Empty is a valid answer** — it
/// sends no `-m`, which is prikk's own shape for a tag that carries no message, so the footer says
/// so rather than leaving the reader to guess that Enter on an empty line is allowed here and
/// refused on the commit prompt.
fn render_tag_message(
    name: &str,
    target: &str,
    typed: &str,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut prose = labelled(
        "Tag ",
        &identifier(name),
        PANEL_TEXT_WIDTH,
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        Style::default().fg(palette.accent),
    );
    prose.extend(labelled(
        "at ",
        &identifier(target),
        PANEL_TEXT_WIDTH,
        Style::default().fg(palette.dim),
        Style::default().fg(palette.accent),
    ));
    prose.push(Line::from(""));
    for row in wrap_indented(
        "A message is optional. Leave it empty and the tag carries none.",
        PANEL_PROSE_WIDTH,
        "  ",
    ) {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    prose.push(Line::from(""));

    let actions = vec![
        Line::from(vec![
            Span::styled("  › ", Style::default().fg(palette.accent)),
            Span::styled(inert(typed), Style::default().fg(palette.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter to continue · Esc to cancel",
            Style::default().fg(palette.accent),
        )),
    ];

    Panel {
        title: " Tag message ",
        width: CONFIRM_WIDTH,
        prose,
        actions,
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 044 decision 8: what prikk printed after a ref write, **verbatim** (`C-T4a`/`ER-02`).
///
/// Unlike the commit and seal result cards, which relabel fields prikk reported in JSON, there is
/// no JSON here (F1) — this is prikk's own prose, unparsed. So it is rendered the way the refusal
/// card renders prikk's own prose: a quoted region, inert, visibly distinct from stikk's chrome
/// (`C-T2a`/`C-T2b`), under one line of stikk's own saying whose words these are.
///
/// **Wrapped verbatim, not word-wrapped**: these lines are mostly 64-hex ids beside a short label,
/// and the word wrap collapses runs of whitespace, which would make prikk's line a line prikk did
/// not print.
fn render_ref_write_result(
    kind: RefWriteKind,
    lines: &[String],
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut prose = vec![
        Line::from(Span::styled(
            match kind {
                RefWriteKind::CreateBranch => "  the branch exists",
                RefWriteKind::CreateTag => "  the tag exists",
                RefWriteKind::CloseBranch => "  the branch is closed",
            },
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  prikk reported —",
            Style::default().fg(palette.dim),
        )),
    ];
    for line in lines {
        let inert_line = inert(line);
        for row in crate::text::wrap_verbatim(&inert_line, SEAL_TEXT_WIDTH, QUOTE_INDENT) {
            prose.push(Line::from(vec![
                Span::styled(QUOTE_BAR, Style::default().fg(palette.accent)),
                Span::styled(
                    row.get(QUOTE_INDENT.len()..)
                        .unwrap_or_default()
                        .to_string(),
                    Style::default().fg(palette.fg),
                ),
            ]));
        }
    }
    prose.push(Line::from(""));

    Panel {
        title: kind.done_title(),
        width: SEAL_WIDTH,
        prose,
        // The dismiss hint is the only affordance here; a result card the user cannot leave is the
        // worst thing this overlay could clip.
        actions: vec![Line::from(Span::styled(
            "  Enter · Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// RFC 045 decision 8: what prikk printed after a switch, **verbatim** (`C-T4a`/`ER-02`), in the
/// quoted region [`render_ref_write_result`] uses and for the same reason: there is no JSON here
/// (F1), so this is prikk's own prose, unparsed.
///
/// **The two streams are labelled and kept apart.** stdout carries the result — which branch, and
/// the three counts. stderr, at exit 0, carries a note about *how* prikk got there (a snapshot
/// fallback). Running them together would make one sentence out of two different statements.
fn render_switch_result(
    printed: &[String],
    noted: &[String],
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut prose = vec![
        Line::from(Span::styled(
            "  the worktree is the new branch's files",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    let quoted = |label: &str, lines: &[String], prose: &mut Vec<Line<'static>>| {
        if lines.is_empty() {
            return;
        }
        prose.push(Line::from(Span::styled(
            format!("  {label}"),
            Style::default().fg(palette.dim),
        )));
        for line in lines {
            let inert_line = inert(line);
            for row in crate::text::wrap_verbatim(&inert_line, SEAL_TEXT_WIDTH, QUOTE_INDENT) {
                prose.push(Line::from(vec![
                    Span::styled(QUOTE_BAR, Style::default().fg(palette.accent)),
                    Span::styled(
                        row.get(QUOTE_INDENT.len()..)
                            .unwrap_or_default()
                            .to_string(),
                        Style::default().fg(palette.fg),
                    ),
                ]));
            }
        }
        prose.push(Line::from(""));
    };
    quoted("prikk reported \u{2014}", printed, &mut prose);
    quoted("and noted \u{2014}", noted, &mut prose);

    Panel {
        title: " Switched ",
        width: SEAL_WIDTH,
        prose,
        actions: vec![Line::from(Span::styled(
            "  Enter \u{b7} Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// The result of a merge that landed (RFC 047 decision 6).
///
/// **prikk's lines verbatim, and one line of stikk's.** prikk names the baseline, the parent and
/// adopted blocks, the adopted patches, the new block and the new RefState; stikk adds only what
/// prikk does not say and the reader will otherwise discover by surprise — that the worktree is
/// unchanged, and where the merged files are.
fn render_merge_result(printed: &[String], palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose = vec![
        Line::from(Span::styled(
            "  the merge block is sealed",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if !printed.is_empty() {
        prose.push(Line::from(Span::styled(
            "  prikk reported \u{2014}",
            Style::default().fg(palette.dim),
        )));
        for line in printed {
            let inert_line = inert(line);
            for row in crate::text::wrap_verbatim(&inert_line, SEAL_TEXT_WIDTH, QUOTE_INDENT) {
                prose.push(Line::from(vec![
                    Span::styled(QUOTE_BAR, Style::default().fg(palette.accent)),
                    Span::styled(
                        row.get(QUOTE_INDENT.len()..)
                            .unwrap_or_default()
                            .to_string(),
                        Style::default().fg(palette.fg),
                    ),
                ]));
            }
        }
        prose.push(Line::from(""));
    }
    // **stikk's own sentence, and it is not optional.** prikk writes no file here, and a reader
    // who looks for the merged files in the worktree will not find them (F3's neighbour).
    for row in wrap_indented(stikk_core::SWITCH_TO_SEE_THE_FILES, SEAL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }

    Panel {
        title: " Merged ",
        width: SEAL_WIDTH,
        prose,
        actions: vec![Line::from(Span::styled(
            "  Enter \u{b7} Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// The result of an import that landed (RFC 050 decision 4).
///
/// **prikk's lines verbatim, and one line of stikk's.** prikk names the received ref, the
/// RefState, the object counts and -- in its own note -- that no local ref was created or
/// advanced and no MAINTAINER key was trusted. stikk adds only what prikk does not say and the
/// reader will otherwise go looking for: that taking this work into a branch of theirs is a
/// merge stikk does not offer from a received ref yet.
fn render_import_result(printed: &[String], palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose = vec![
        Line::from(Span::styled(
            "  the bundle is stored as a received branch",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if !printed.is_empty() {
        prose.push(Line::from(Span::styled(
            "  prikk reported \u{2014}",
            Style::default().fg(palette.dim),
        )));
        for line in printed {
            let inert_line = inert(line);
            for row in crate::text::wrap_verbatim(&inert_line, SEAL_TEXT_WIDTH, QUOTE_INDENT) {
                prose.push(Line::from(vec![
                    Span::styled(QUOTE_BAR, Style::default().fg(palette.accent)),
                    Span::styled(
                        row.get(QUOTE_INDENT.len()..)
                            .unwrap_or_default()
                            .to_string(),
                        Style::default().fg(palette.fg),
                    ),
                ]));
            }
        }
        prose.push(Line::from(""));
    }
    for row in wrap_indented(stikk_core::RECEIVED_IS_READ_ONLY, SEAL_PROSE_WIDTH, "  ") {
        prose.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }

    Panel {
        title: " Imported ",
        width: SEAL_WIDTH,
        prose,
        actions: vec![Line::from(Span::styled(
            "  Enter \u{b7} Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

/// The result of an export that landed (RFC 050 decision 5).
///
/// **prikk's lines verbatim, and nothing of stikk's.** prikk names the tip block, the object
/// count and its own continuity note about the author keys; there is nothing here a reader will
/// discover by surprise, so stikk adds no sentence -- unlike a merge, where the worktree's being
/// unchanged is exactly such a surprise.
fn render_export_result(printed: &[String], palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut prose = vec![
        Line::from(Span::styled(
            "  the bundle is written",
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if !printed.is_empty() {
        prose.push(Line::from(Span::styled(
            "  prikk reported \u{2014}",
            Style::default().fg(palette.dim),
        )));
        for line in printed {
            let inert_line = inert(line);
            for row in crate::text::wrap_verbatim(&inert_line, SEAL_TEXT_WIDTH, QUOTE_INDENT) {
                prose.push(Line::from(vec![
                    Span::styled(QUOTE_BAR, Style::default().fg(palette.accent)),
                    Span::styled(
                        row.get(QUOTE_INDENT.len()..)
                            .unwrap_or_default()
                            .to_string(),
                        Style::default().fg(palette.fg),
                    ),
                ]));
            }
        }
    }

    Panel {
        title: " Exported ",
        width: SEAL_WIDTH,
        prose,
        actions: vec![Line::from(Span::styled(
            "  Enter \u{b7} Esc to dismiss",
            Style::default().fg(palette.accent),
        ))],
        style: Style::default().fg(palette.fg),
    }
    .render(frame, area);
}

fn capability_label(capability: Capability) -> &'static str {
    match capability {
        Capability::Viewer => "read-only (no signing needed)",
        Capability::Author => "AUTHOR",
        Capability::Maintainer => "MAINTAINER",
    }
}

/// A selectable list row with a marker and consistent highlight styling.
fn selectable<'a>(palette: &Palette, selected: bool, text: String) -> Line<'a> {
    let marker = if selected { "▶ " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    Line::from(vec![
        Span::styled(marker, Style::default().fg(palette.accent)),
        Span::styled(text, style),
    ])
}

/// A selectable row that can also carry a **mark** (RFC 041 decision 1).
///
/// Two columns, never one: the cursor says where you are and the mark says what you set, and a
/// row is routinely both at once -- which is exactly the moment before `d`. This is the scheme
/// History's own rows use.
fn selectable_marked<'a>(
    palette: &Palette,
    selected: bool,
    marked: bool,
    text: String,
) -> Line<'a> {
    let cursor = if selected { "\u{25b6} " } else { "  " };
    let mark = if marked { "\u{25cf} " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    Line::from(vec![
        Span::styled(cursor, Style::default().fg(palette.accent)),
        Span::styled(mark, Style::default().fg(palette.warn)),
        Span::styled(text, style),
    ])
}

fn section<'a>(palette: &Palette, title: &'a str) -> Line<'a> {
    Line::from(Span::styled(
        format!("  {title}"),
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_line<'a>(palette: &Palette, key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key:<13}"), Style::default().fg(palette.accent)),
        Span::styled(desc, Style::default().fg(palette.fg)),
    ])
}

/// The overlay widths this module uses, named so the wrap width beside each one cannot drift from it.
const CONFIRM_WIDTH: u16 = 70;
/// The result/consent card width.
const SEAL_WIDTH: u16 = 78;
/// Text columns inside a [`SEAL_WIDTH`] panel — see [`PANEL_TEXT_WIDTH`].
const SEAL_TEXT_WIDTH: usize = SEAL_WIDTH as usize - 2;
/// Prose columns inside a [`SEAL_WIDTH`] panel — see [`PANEL_PROSE_WIDTH`].
const SEAL_PROSE_WIDTH: usize = SEAL_TEXT_WIDTH - 1;

/// The quote bar prefixing every row of prikk's verbatim message, and the placeholder indent the
/// wrap is computed against.
///
/// **Both are four display columns**, which is what lets the bar be substituted for the indent after
/// wrapping without moving a single wrap point.
const QUOTE_BAR: &str = "  │ ";
/// See [`QUOTE_BAR`].
const QUOTE_INDENT: &str = "    ";

/// The refusal/stale card width.
const REFUSAL_WIDTH: u16 = 72;
/// Text columns inside a [`REFUSAL_WIDTH`] panel — see [`PANEL_TEXT_WIDTH`].
const REFUSAL_TEXT_WIDTH: usize = REFUSAL_WIDTH as usize - 2;
/// Prose columns inside a [`REFUSAL_WIDTH`] panel — see [`PANEL_PROSE_WIDTH`].
const REFUSAL_PROSE_WIDTH: usize = REFUSAL_TEXT_WIDTH - 1;

/// Text columns available inside a [`CONFIRM_WIDTH`] panel — the outer width less its two border
/// columns, which is exactly what [`Panel`] gives the prose region.
///
/// **Two widths, and what a call holds decides which it takes** (RFC 013 Handoff B). Every panel
/// kind has both, and the pair is the same rule each time.
///
/// **This one is the full inner width, for identifiers and prikk's verbatim lines**: a target row,
/// a `block id:` line, a repository path, anything drawn through `identifier` or `wrap_verbatim`.
/// It must stay full, because a gutter would cost a 64-hex id its last character — `block id: `
/// plus 64 hex is exactly the inner width of a 78-wide card — and fold it onto a second row for one
/// column. Fitting prikk's identifiers on one row wins, and prikk's lines are prikk's to wrap.
///
/// **Prose stikk writes takes [`PANEL_PROSE_WIDTH`]**: a consequence, a note, a notice, an error
/// sentence, anything stikk composed. Measured on the merge card at 80×24 before this existed, a
/// word of stikk's could land in the last inner column and read as cut off — `…from this
/// baseline. A│`. Nothing stikk writes needs that column, so prose gives it up and identifiers
/// do not.
const PANEL_TEXT_WIDTH: usize = CONFIRM_WIDTH as usize - 2;

/// Prose columns inside a [`CONFIRM_WIDTH`] panel: [`PANEL_TEXT_WIDTH`] less a one-column right
/// gutter, so no word stikk wrote sits against the border. See [`PANEL_TEXT_WIDTH`] for which
/// calls take which, and `the_prose_gutter_*` tests for what holds it.
const PANEL_PROSE_WIDTH: usize = PANEL_TEXT_WIDTH - 1;

/// What to say under a signing key id, given how firmly it may be claimed (RFC 026 §5).
///
/// `Bound` says nothing extra: prikk confirmed the key binds, and a plain id is the honest rendering.
/// The other two each add one sentence, wrapped, because the sentence is the point — a user meeting
/// `Unbound` is looking at the first commit in a new repository and should be told, in the affirmative,
/// what is about to happen rather than warned about a hedge.
fn claim_rows<'a>(claim: KeyClaim, palette: &Palette) -> Vec<Line<'a>> {
    let text = match claim {
        KeyClaim::Bound | KeyClaim::None => return Vec::new(),
        KeyClaim::Unbound => {
            "Nothing has signed under this id in this repository yet — this signature binds it."
        }
        KeyClaim::Unchecked => {
            "This prikk cannot report which key will sign, so stikk is naming the id it will \
             pass, not one prikk has confirmed."
        }
    };
    wrap_indented(text, PANEL_PROSE_WIDTH, "    ")
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(palette.dim))))
        .collect()
}

/// The shared prose-plus-actions panel every overlay of that shape renders through (RFC 024 §2).
///
/// # Why one shape, and why these two mechanisms together
///
/// **RFC 016 C2 found this defect once and fixed one instance.** `lines.len()` counts *logical* lines
/// while the widget draws *wrapped* rows, so a `+ N` headroom guess starves whichever region it
/// under-counts. C2 fixed `render_refusal` by anchoring the actions; the fix reached three of fourteen
/// renderers and eleven kept guessing, which is how seal's confirmation shipped three releases without
/// ever showing `Enter to confirm` (RFC 024 F1) and the commit prompt lost its footer (F2).
///
/// C2 had to choose anchoring over measuring, and was right to: `Paragraph::wrap` wraps inside the
/// widget and ratatui's `line_count` is behind an unstable feature, so the rendered height could not be
/// known. **RFC 023 F2 built [`crate::text::wrap_indented`]**, which wraps in stikk so `lines.len()` is
/// a fact. The two now compose, and this type uses both:
///
/// - **Measured** — callers wrap their prose with `wrap_indented` before building `prose`, so the
///   height this computes is exact rather than estimated.
/// - **Anchored** — `actions` is laid out at the bottom at its own exact height, so even a caller that
///   forgets to wrap loses *prose* rather than the affordance the overlay exists to offer.
///
/// # And when it still does not fit
///
/// A terminal can always be shorter than the content. Then the panel **says so** in its title —
/// `lines 1–18 of 24`, the same idiom the Glossary uses (RFC 024 §4: follow it, do not invent a second) (RFC 023 F2) — rather than clipping in silence.
/// That is assertion 1 of the gate in `tests.rs`: clipped-and-honest is fine, clipped-and-quiet is the
/// defect.
pub(crate) struct Panel<'a> {
    /// Title text, without the viewport indicator — this type appends that when it is needed.
    pub(crate) title: &'a str,
    /// Outer width; clamped to the terminal by [`centered`].
    pub(crate) width: u16,
    /// The body. Pre-wrapped by the caller so its length is the number of rows it will occupy.
    pub(crate) prose: Vec<Line<'a>>,
    /// The affordances — footer, next-steps, prompt. **Never clipped while any row remains.**
    pub(crate) actions: Vec<Line<'a>>,
    /// The block's style (`palette.fg` for ordinary overlays, `palette.warn` for refusals).
    pub(crate) style: Style,
}

impl Panel<'_> {
    /// Render the panel centred in `area`.
    fn render(self, frame: &mut Frame, area: Rect) {
        let actions_height = u16::try_from(self.actions.len()).unwrap_or(u16::MAX);
        let prose_len = u16::try_from(self.prose.len()).unwrap_or(u16::MAX);
        // Exact, not estimated: `prose` is already wrapped, so this is the height that fits everything.
        let wanted = prose_len.saturating_add(actions_height).saturating_add(2);
        let region = centered(self.width, wanted.min(area.height), area);

        let inner_height = region.height.saturating_sub(2);
        // The actions keep their exact height; prose takes what is left. When even the actions do not
        // fit, they take everything — losing the last affordance is worse than losing all prose, and
        // this is the direction C2's rule points.
        let actions_area_height = actions_height.min(inner_height);
        let prose_area_height = inner_height.saturating_sub(actions_area_height);

        let title = if prose_area_height < prose_len {
            // Clipped — say so, in the Glossary's idiom, rather than in silence.
            format!(
                "{} — lines 1–{} of {} ",
                self.title.trim_end(),
                prose_area_height,
                prose_len
            )
        } else {
            self.title.to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(self.style);
        frame.render_widget(Clear, region);
        let inner = block.inner(region);
        frame.render_widget(block, region);
        let [prose_area, actions_area] = Layout::vertical([
            Constraint::Length(prose_area_height),
            Constraint::Length(actions_area_height),
        ])
        .areas(inner);
        frame.render_widget(Paragraph::new(self.prose), prose_area);
        frame.render_widget(Paragraph::new(self.actions), actions_area);
    }
}

/// The shared list panel: a fixed header, a scrollable list, and a window that follows the cursor
/// (RFC 024 §3/F5).
///
/// The sibling of [`Panel`] for the other overlay shape. Where `Panel`'s rule is *whatever runs out of
/// room is prose, never an action*, this one's is **whatever runs out of room is not the selection**:
/// the window moves to keep the cursor on screen, and the title says how much of the list is showing.
///
/// `render_ref_picker` had neither. It drew from row zero and clipped, so with forty refs at 80×24 a
/// user holding ↓ watched nothing move while the selection travelled somewhere invisible — shipped
/// that way since 0.1.0.
pub(crate) struct ListPanel<'a> {
    /// Title text, without the viewport indicator.
    pub(crate) title: &'a str,
    /// Outer width.
    pub(crate) width: u16,
    /// Rows pinned above the list — a filter prompt, a separator. Never scrolled out of view.
    pub(crate) header: Vec<Line<'a>>,
    /// The list itself, one row per entry.
    pub(crate) items: Vec<Line<'a>>,
    /// The selected index into `items`, when this list has a selection.
    pub(crate) cursor: Option<usize>,
    /// The block's style.
    pub(crate) style: Style,
}

impl ListPanel<'_> {
    fn render(self, frame: &mut Frame, area: Rect) {
        let header_len = u16::try_from(self.header.len()).unwrap_or(u16::MAX);
        let items_len = u16::try_from(self.items.len()).unwrap_or(u16::MAX);
        let wanted = header_len.saturating_add(items_len).saturating_add(2);
        let region = centered(self.width, wanted.min(area.height), area);

        let inner_height = region.height.saturating_sub(2);
        // The header is chrome the list is read *through* — a filter box the user is typing into, above
        // all else. It keeps its rows; the list takes what remains.
        let header_height = header_len.min(inner_height);
        let viewport = inner_height.saturating_sub(header_height);

        let start = self.cursor.map_or(0, |cursor| {
            window_start(self.items.len(), cursor, usize::from(viewport))
        });
        let end = (start + usize::from(viewport)).min(self.items.len());
        let visible: Vec<Line> = self
            .items
            .get(start..end)
            .map(<[Line]>::to_vec)
            .unwrap_or_default();

        let title = if items_len > viewport {
            format!(
                "{} — lines {}–{} of {} ",
                self.title.trim_end(),
                start + 1,
                end,
                items_len
            )
        } else {
            self.title.to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(self.style);
        frame.render_widget(Clear, region);
        let inner = block.inner(region);
        frame.render_widget(block, region);
        let [header_area, list_area] = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Length(viewport),
        ])
        .areas(inner);
        frame.render_widget(Paragraph::new(self.header), header_area);
        frame.render_widget(Paragraph::new(visible), list_area);
    }
}

/// A `label: value` row, wrapped so a long value continues on the next row instead of being cut off at
/// the border.
///
/// [`Panel`] does not wrap — that is what makes its height exact — so anything it is handed must
/// already fit. Most prose goes through [`wrap_indented`] directly; this exists for the two-span rows
/// where a dim label introduces an accented value, because those cannot be wrapped as one string
/// without losing the distinction. A 64-hex block id under a `heads/main RefState:` label is 87
/// columns, which is where this was needed: `Paragraph::wrap` used to fold it and the switch to exact
/// sizing would otherwise have truncated it.
///
/// A value that does not fit beside its label **moves to its own row whole** rather than being split
/// across the label's remaining columns. That is what `Paragraph::wrap` did here before, and it is the
/// right behaviour for prikk's identifiers: a 64-hex id broken at column 53 is two fragments a reader
/// has to reassemble, while the same id alone on the next row can be read and compared in one glance.
/// (`stikk-tui`'s own test asserted exactly that — 64 contiguous characters on screen — and caught this
/// when the first version of this helper split them.) Only a value too long for a whole row of its own
/// wraps at all.
fn labelled<'a>(
    label: &str,
    value: &str,
    width: usize,
    label_style: Style,
    value_style: Style,
) -> Vec<Line<'a>> {
    let head = format!("  {label}");
    let beside = width.saturating_sub(head.chars().count());
    if value.chars().count() <= beside {
        return vec![Line::from(vec![
            Span::styled(head, label_style),
            Span::styled(value.to_string(), value_style),
        ])];
    }
    let mut out = vec![Line::from(Span::styled(head, label_style))];
    for row in wrap_indented(value, width, "  ") {
        out.push(Line::from(Span::styled(row, value_style)));
    }
    out
}

/// The first row a list should draw so that `cursor` is on screen (RFC 024 F5).
///
/// `render_ref_picker` rendered from row zero and clipped: with forty refs at 80×24 the cursor could
/// travel past the fold while nothing on screen moved, and it has shipped that way since 0.1.0. A list
/// that cannot show you what is selected is worse than one that cannot show you everything, which is
/// why the gate's second assertion has no "or an indicator" clause.
///
/// Keeps the cursor on screen with the window moving as little as possible: scroll only when the cursor
/// would fall outside it.
fn window_start(total: usize, cursor: usize, viewport: usize) -> usize {
    if viewport == 0 || total <= viewport {
        return 0;
    }
    let cursor = cursor.min(total.saturating_sub(1));
    // Show the cursor with the rest of the window following it.
    cursor
        .saturating_sub(viewport.saturating_sub(1))
        .min(total - viewport)
}

/// A `width`×`height` rectangle centred within `area` (clamped to fit).
fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let [h] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [v] = Layout::vertical([Constraint::Length(height.min(area.height))])
        .flex(Flex::Center)
        .areas(h);
    v
}

#[cfg(test)]
mod tests;

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
    ConfirmationSummary, KeyClaim, NextStep, RefusalCard, RefusalRecord, glossary, palette,
};
use stikk_model::{Capability, Tier};

use crate::app::{Operation, OperationStatus};
use std::cell::Cell;

use crate::text::{inert, wrap_indented};
use crate::theme::Palette;

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
        /// The selectable ref names (repository-sourced; rendered inert).
        refs: Vec<String>,
        /// The highlighted entry.
        cursor: usize,
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
        /// stikk's explanation, in its own voice.
        gloss: String,
        /// Next-steps, stikk-authored — today always exactly one: re-preview.
        next_steps: Vec<NextStep>,
        /// The highlighted next-step.
        cursor: usize,
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
        /// What to restate. Composed at preview time; never re-derived here (RFC 013 §3).
        summary: ConfirmationSummary,
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
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Glossary { .. } => " Glossary & Help ",
            Self::Loading { .. } => " Loading ",
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
        }
    }
}

/// Render the top overlay centred over `area`, clearing the region beneath it first (TU-02).
pub fn render(overlay: &Overlay, palette: &Palette, frame: &mut Frame, area: Rect) {
    match overlay {
        Overlay::Glossary { offset } => render_glossary(offset, palette, frame, area),
        Overlay::Loading { what, .. } => render_loading(what, palette, frame, area),
        Overlay::Operations { operations } => render_operations(operations, palette, frame, area),
        Overlay::RefPicker { refs, cursor } => {
            render_ref_picker(refs, *cursor, palette, frame, area)
        }
        Overlay::Refusal { card, cursor } => render_refusal(card, *cursor, palette, frame, area),
        Overlay::Stale {
            operation,
            gloss,
            next_steps,
            cursor,
        } => render_stale(operation, gloss, next_steps, *cursor, palette, frame, area),
        Overlay::Palette {
            filter,
            cursor,
            readiness,
        } => render_palette(filter, *cursor, *readiness, palette, frame, area),
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
        key_line(palette, "C", "commit worktree changes"),
        key_line(palette, "S", "seal the active WAL"),
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

fn render_ref_picker(
    refs: &[String],
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
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
                .map(|(i, name)| selectable(palette, i == cursor, inert(name)))
                .collect(),
            Some(cursor),
        )
    };
    ListPanel {
        title: " Choose ref ",
        width: 52,
        header: Vec::new(),
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
        for row in wrap_indented(gloss, REFUSAL_TEXT_WIDTH, "  ") {
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
            REFUSAL_TEXT_WIDTH,
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
fn render_stale(
    operation: &str,
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
            Span::styled(
                ": the repository changed since this was last previewed.",
                Style::default().fg(palette.fg),
            ),
        ]),
        Line::from(""),
    ];

    // ② the gloss, in stikk's own voice — never quoted as if it were prikk's. Wrapped here (RFC 024).
    for row in wrap_indented(gloss, REFUSAL_TEXT_WIDTH, "  ") {
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

fn render_palette(
    filter: &str,
    cursor: usize,
    readiness: stikk_model::Readiness,
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
        let reason = cmd.unmet_reason(readiness);
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

    if !summary.target_ids.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Targets:",
            Style::default().fg(palette.dim),
        )));
        for id in &summary.target_ids {
            lines.push(Line::from(Span::styled(
                format!("    {}", inert(id)),
                Style::default().fg(palette.accent),
            )));
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
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        format!("  Consumes: {}", capability_label(summary.capability)),
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
            // Inert like every other string that did not originate in stikk (`C-T2a`): an id is a
            // label a user or a repository's conventions chose, and it reaches a cell the same way a
            // ref name does.
            Span::styled(inert(id), Style::default().fg(palette.accent)),
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
                PANEL_TEXT_WIDTH,
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

    // The consequence — stikk's own words. The operation's own plan/content (prikk's verbatim text,
    // for Class A previews — RFC 013 F1) lives in the preview view itself, never restated here
    // (RFC 013 Q1: the confirmation restates the fixed summary, not the preview).
    //
    // **Wrapped here, in stikk** (RFC 024 §2). This is the line that made seal's confirmation ship for
    // three releases without its confirm affordance: seal's `Unknown` maintainer consequence
    // is one *logical* line that draws as four rows, the old `lines.len() + 4` headroom was spent on
    // it, and the footer fell outside the box at every terminal height. Wrapping it makes the count a
    // fact — and the footer is in `actions` below, so even a future mis-measurement costs prose.
    for line in wrap_indented(&summary.consequence, PANEL_TEXT_WIDTH, "  ") {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(palette.fg),
        )));
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
                format!("  Type \"{}\" to confirm:", inert(target)),
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
        for line in wrap_indented(error, PANEL_TEXT_WIDTH, "  ") {
            actions.push(Line::from(Span::styled(
                line,
                Style::default().fg(palette.warn),
            )));
        }
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
            format!("  Commit to {}", inert(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for line in wrap_indented(notice.trim_start(), PANEL_TEXT_WIDTH, "  ") {
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
        &inert(&result.patch_id),
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
                &format!("{} {}", inert(&change.operation), inert(&change.path)),
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
            format!("  Seal {}", inert(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    // RFC 024: `SEAL_CONSENT_COPY` is one logical line that wraps to several rows, which is why this
    // renderer carried a hand-tuned `+ 6` — added after its own render tests caught the mark and the
    // Enter hint being clipped. Wrapping it makes the count exact and the tuning unnecessary.
    for row in wrap_indented(stikk_core::SEAL_CONSENT_COPY, SEAL_TEXT_WIDTH, "  ") {
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
        &inert(&result.block_id),
        SEAL_TEXT_WIDTH,
        Style::default().fg(palette.dim),
        Style::default().fg(palette.accent),
    ));
    lines.push(Line::from(Span::styled(
        format!("  patches {}", result.patches),
        Style::default().fg(palette.fg),
    )));
    lines.extend(labelled(
        &format!("{} RefState: ", inert(&result.reff)),
        &inert(&result.ref_state),
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

/// Text columns available inside a [`CONFIRM_WIDTH`] panel — the outer width less its two border
/// columns, which is exactly what [`Panel`] gives the prose region.
///
/// **No right gutter, deliberately.** A one-column gutter reads slightly better for prose, and the
/// Glossary keeps one for that reason. Here it would cost a 64-hex id its last character: `block id: `
/// plus 64 hex is exactly the inner width of a 78-wide card, and a gutter would fold it onto a second
/// row for one column. Fitting prikk's identifiers on one row wins.
const PANEL_TEXT_WIDTH: usize = CONFIRM_WIDTH as usize - 2;

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
            "This prikk cannot report which key will sign, so stikk is naming the id it will pass,              not one prikk has confirmed."
        }
    };
    wrap_indented(text, PANEL_TEXT_WIDTH, "    ")
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

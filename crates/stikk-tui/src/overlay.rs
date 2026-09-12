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
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use stikk_core::{ConfirmationSummary, NextStep, RefusalCard, RefusalRecord, glossary, palette};
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
    let lines: Vec<Line> = if refs.is_empty() {
        vec![Line::from(Span::styled(
            "  no refs reported",
            Style::default().fg(palette.dim),
        ))]
    } else {
        refs.iter()
            .enumerate()
            .map(|(i, name)| selectable(palette, i == cursor, inert(name)))
            .collect()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Choose ref ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(52, height, area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
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
    let mut prose: Vec<Line> = vec![Line::from(Span::styled(
        "  prikk reported —",
        Style::default().fg(palette.dim),
    ))];
    for raw in card.verbatim.lines() {
        prose.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(palette.warn)),
            Span::styled(inert(raw), Style::default().fg(palette.fg)),
        ]));
    }
    prose.push(Line::from(""));

    // ② the gloss — stikk's own voice, separate and below (ER-02). Absent ⇒ verbatim-only (RR-5).
    if let Some(gloss) = &card.gloss {
        prose.push(Line::from(Span::styled(
            format!("  {gloss}"),
            Style::default().fg(palette.dim),
        )));
        prose.push(Line::from(""));
    }

    // ④ glossary links for any named code (FR-111).
    if !card.glossary_codes.is_empty() {
        prose.push(Line::from(Span::styled(
            format!("  glossary: {}", card.glossary_codes.join(", ")),
            Style::default().fg(palette.accent),
        )));
    }

    // ③ next-steps — stikk-authored, selectable (C-T2b). Their own region, below.
    let mut actions: Vec<Line> = vec![Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    ))];
    for (i, step) in card.next_steps.iter().enumerate() {
        actions.push(selectable(palette, i == cursor, step.label.clone()));
    }
    let actions_height = actions.len() as u16;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" prikk refused ")
        .style(Style::default().fg(palette.warn));
    // Headroom for the prose region's own wrapping — still an estimate (`lines.len()` counts logical
    // lines, a long gloss wraps to several rows), but no longer load-bearing for the actions'
    // visibility the way it was before this fix: `Constraint::Length(actions_height)` below reserves
    // their exact height regardless of how wrong this estimate turns out to be, so an under-estimate
    // here costs prose, never a next-step. Sized generously (comfortably above the longest gloss this
    // codebase ships today, the trust-refusal one at ~280 characters/5 wrapped rows) so the ordinary
    // case shows everything; a future gloss long enough to exceed this would still degrade safely.
    let height = (prose.len() as u16 + actions_height + 12).min(area.height.saturating_sub(2));
    let region = centered(72, height.max(10), area);
    frame.render_widget(Clear, region);
    let inner = block.inner(region);
    frame.render_widget(block, region);
    let [prose_area, actions_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(actions_height)]).areas(inner);
    frame.render_widget(Paragraph::new(prose).wrap(Wrap { trim: false }), prose_area);
    frame.render_widget(Paragraph::new(actions), actions_area);
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

    // ② the gloss, in stikk's own voice — never quoted as if it were prikk's.
    prose.push(Line::from(Span::styled(
        format!("  {gloss}"),
        Style::default().fg(palette.dim),
    )));

    // ③ next-steps — stikk-authored, selectable (C-T2b); always exactly one this increment (`Refresh`).
    // Their own region, below.
    let mut actions: Vec<Line> = vec![Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    ))];
    for (i, step) in next_steps.iter().enumerate() {
        actions.push(selectable(palette, i == cursor, step.label.clone()));
    }
    let actions_height = actions.len() as u16;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" stikk stopped ")
        .style(Style::default().fg(palette.warn));
    let height = (prose.len() as u16 + actions_height + 12).min(area.height.saturating_sub(2));
    let region = centered(72, height.max(10), area);
    frame.render_widget(Clear, region);
    let inner = block.inner(region);
    frame.render_widget(block, region);
    let [prose_area, actions_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(actions_height)]).areas(inner);
    frame.render_widget(Paragraph::new(prose).wrap(Wrap { trim: false }), prose_area);
    frame.render_widget(Paragraph::new(actions), actions_area);
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
    let mut lines: Vec<Line> = vec![
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Command palette ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    // Widened from 64 (RFC 013 v1) to fit a disabled entry's name + binding + reason on one line
    // without truncating it (RFC 014 §6: "Commit worktree changes" is the first command whose disabled
    // reason is long enough to hit the old width) — `unmet_reason`'s text must actually be readable,
    // not merely present in the `Line`.
    let region = centered(80, height.max(6), area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
}

/// A small centred note for a pending overlay-bound request (RFC 010 §5) — the overlay counterpart to
/// `shell`'s `Focus::Loading` rendering for a pending screen.
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Background operations ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(56, height.max(4), area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
}

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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Recent refusals ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(72, height.max(4), area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
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
    }
    lines.push(Line::from(""));

    // The consequence — stikk's own words. The operation's own plan/content (prikk's verbatim text,
    // for Class A previews — RFC 013 F1) lives in the preview view itself, never restated here
    // (RFC 013 Q1: the confirmation restates the fixed summary, not the preview).
    lines.push(Line::from(Span::styled(
        format!("  {}", summary.consequence),
        Style::default().fg(palette.fg),
    )));
    lines.push(Line::from(""));

    match tier {
        Tier::One => {} // never reaches this overlay in practice — tier 1 has no confirmation
        Tier::Two | Tier::Three => {
            lines.push(Line::from(Span::styled(
                "  Enter to confirm · Esc to cancel",
                Style::default().fg(palette.accent),
            )));
        }
        Tier::ThreeTyped => {
            let target = summary.target_name.as_deref().unwrap_or("");
            lines.push(Line::from(Span::styled(
                format!("  Type \"{}\" to confirm:", inert(target)),
                Style::default().fg(palette.dim),
            )));
            lines.push(Line::from(vec![
                Span::styled("  › ", Style::default().fg(palette.accent)),
                Span::styled(inert(typed), Style::default().fg(palette.fg)),
            ]));
        }
    }

    if let Some(error) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {error}"),
            Style::default().fg(palette.warn),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .style(Style::default().fg(palette.warn));
    let height = (lines.len() as u16 + 4).min(area.height.saturating_sub(2));
    let region = centered(70, height.max(8), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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
    let lines = vec![
        Line::from(Span::styled(
            format!("  Commit to {}", inert(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(notice, Style::default().fg(palette.dim))),
        Line::from(""),
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commit message ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(70, height.max(9), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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
        Line::from(vec![
            Span::styled("  patch id: ", Style::default().fg(palette.dim)),
            Span::styled(inert(&result.patch_id), Style::default().fg(palette.accent)),
        ]),
        Line::from(Span::styled(
            format!(
                "  operations {} · referenced blobs {} · text edits {}",
                result.operations, result.referenced_blobs, result.text_edits
            ),
            Style::default().fg(palette.fg),
        )),
    ];
    if !result.changes.is_empty() {
        lines.push(Line::from(""));
        for change in &result.changes {
            lines.push(Line::from(Span::styled(
                format!("    {} {}", inert(&change.operation), inert(&change.path)),
                Style::default().fg(palette.fg),
            )));
        }
    }
    if !result.notes.is_empty() {
        lines.push(Line::from(""));
        for note in &result.notes {
            lines.push(Line::from(Span::styled(
                format!("  {}", inert(note)),
                Style::default().fg(palette.dim),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter · Esc to dismiss",
        Style::default().fg(palette.accent),
    )));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commit recorded ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(78, height.max(8), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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
    let lines = vec![
        Line::from(Span::styled(
            format!("  Seal {}", inert(reff)),
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", stikk_core::SEAL_CONSENT_COPY),
            Style::default().fg(palette.fg),
        )),
        Line::from(""),
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Before you seal ")
        .style(Style::default().fg(palette.warn));
    // `lines.len()` counts `Line` values, not the rows `Wrap` actually renders — `SEAL_CONSENT_COPY`
    // is long enough to wrap across several rows in a 78-wide box, so a few extra rows of headroom are
    // added here rather than sized to the unwrapped count (which clipped the acknowledgement mark and
    // the Enter hint below it, caught by this overlay's own render tests).
    let height = (lines.len() as u16 + 6).min(area.height);
    let region = centered(78, height.max(13), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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
        Line::from(vec![
            Span::styled("  block id: ", Style::default().fg(palette.dim)),
            Span::styled(inert(&result.block_id), Style::default().fg(palette.accent)),
        ]),
        Line::from(Span::styled(
            format!("  patches {}", result.patches),
            Style::default().fg(palette.fg),
        )),
        Line::from(vec![
            Span::styled(
                format!("  {} RefState: ", inert(&result.reff)),
                Style::default().fg(palette.dim),
            ),
            Span::styled(
                inert(&result.ref_state),
                Style::default().fg(palette.accent),
            ),
        ]),
    ];
    if !result.notes.is_empty() {
        lines.push(Line::from(""));
        for note in &result.notes {
            lines.push(Line::from(Span::styled(
                format!("  {}", inert(note)),
                Style::default().fg(palette.dim),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Enter · Esc to dismiss",
        Style::default().fg(palette.accent),
    )));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Sealed ")
        .style(Style::default().fg(palette.fg));
    let height = (lines.len() as u16 + 2).min(area.height);
    let region = centered(78, height.max(8), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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

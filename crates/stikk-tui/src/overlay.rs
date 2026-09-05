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
//! removed by [`crate::app::App::apply`], and popped directly by `back()` like any other overlay.
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
use crate::text::inert;
use crate::theme::Palette;

/// One overlay. Data-carrying variants own their state; `Glossary` reads the static asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    /// The glossary / help browser (terminology mapping, key reference, code index).
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
        /// The session capability, for the disabled-entry reasons (FR-104).
        capability: Capability,
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
    /// (`C-T2b`): a content pane must not be able to look like this. **No mutation is wired to this
    /// overlay yet** (RFC 013: the machinery, not a consumer) — it exists to be pushed and rendered by
    /// whichever real operation previews first.
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
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Glossary => " Glossary & Help ",
            Self::Loading { .. } => " Loading ",
            Self::Operations { .. } => " Background operations ",
            Self::RefPicker { .. } => " Choose ref ",
            Self::Refusal { .. } => " prikk refused ",
            Self::Stale { .. } => " stikk stopped ",
            Self::Palette { .. } => " Command palette ",
            Self::Refusals { .. } => " Recent refusals ",
            Self::Confirmation { .. } => " Confirm ",
        }
    }
}

/// Render the top overlay centred over `area`, clearing the region beneath it first (TU-02).
pub fn render(overlay: &Overlay, palette: &Palette, frame: &mut Frame, area: Rect) {
    match overlay {
        Overlay::Glossary => render_glossary(palette, frame, area),
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
            capability,
        } => render_palette(filter, *cursor, *capability, palette, frame, area),
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
    }
}

fn render_glossary(palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  stikk reads prikk; it never writes your repository.",
            Style::default().fg(palette.dim),
        )),
        Line::from(""),
        section(palette, "Keys"),
        key_line(palette, "Enter", "open / drill in / activate"),
        key_line(palette, "↑/↓ or j/k", "move selection"),
        key_line(palette, "b", "choose which ref to view"),
        key_line(palette, "w", "changes — worktree vs baseline"),
        key_line(palette, "u", "toggle untracked (in Changes)"),
        key_line(palette, ":", "command palette"),
        key_line(palette, "R", "recent refusals"),
        key_line(palette, "o", "background operations"),
        key_line(palette, "r", "refresh — re-read from prikk"),
        key_line(palette, "? / Esc / q", "close · back · quit at root"),
        Line::from(""),
        section(palette, "Git → prikk"),
    ];
    for term in glossary::terminology() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<22}", term.git),
                Style::default().fg(palette.accent),
            ),
            Span::styled(term.prikk, Style::default().fg(palette.fg)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("    {}", term.note),
            Style::default().fg(palette.dim),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Glossary & Help ")
        .style(Style::default().fg(palette.fg));
    // Tall content: give it most of the height and let it scroll from the top.
    let region = centered(74, area.height.saturating_sub(2).min(30), area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block).scroll((0, 0)), region);
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
    let mut lines: Vec<Line> = Vec::new();

    // ① prikk's message, verbatim and inert, in a quoted region distinct from stikk chrome (C-T2b).
    lines.push(Line::from(Span::styled(
        "  prikk reported —",
        Style::default().fg(palette.dim),
    )));
    for raw in card.verbatim.lines() {
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(palette.warn)),
            Span::styled(inert(raw), Style::default().fg(palette.fg)),
        ]));
    }
    lines.push(Line::from(""));

    // ② the gloss — stikk's own voice, separate and below (ER-02). Absent ⇒ verbatim-only (RR-5).
    if let Some(gloss) = &card.gloss {
        lines.push(Line::from(Span::styled(
            format!("  {gloss}"),
            Style::default().fg(palette.dim),
        )));
        lines.push(Line::from(""));
    }

    // ④ glossary links for any named code (FR-111).
    if !card.glossary_codes.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  glossary: {}", card.glossary_codes.join(", ")),
            Style::default().fg(palette.accent),
        )));
        lines.push(Line::from(""));
    }

    // ③ next-steps — stikk-authored, selectable (C-T2b).
    lines.push(Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    )));
    for (i, step) in card.next_steps.iter().enumerate() {
        lines.push(selectable(palette, i == cursor, step.label.clone()));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" prikk refused ")
        .style(Style::default().fg(palette.warn));
    // Headroom for wrapped lines (the gloss and a long verbatim can each wrap): budget a few extra
    // rows so no next-step is clipped.
    let height = (lines.len() as u16 + 6).min(area.height.saturating_sub(2));
    let region = centered(72, height.max(10), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
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
    let mut lines: Vec<Line> = Vec::new();

    // ① stikk's own explanation — attributed to stikk, never to prikk (C-T2b/design-review C1).
    lines.push(Line::from(Span::styled(
        "  stikk stopped —",
        Style::default().fg(palette.dim),
    )));
    lines.push(Line::from(vec![
        Span::styled("  │ ", Style::default().fg(palette.warn)),
        Span::styled(
            inert(operation),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ": the repository changed since this was last previewed.",
            Style::default().fg(palette.fg),
        ),
    ]));
    lines.push(Line::from(""));

    // ② the gloss, in stikk's own voice — never quoted as if it were prikk's.
    lines.push(Line::from(Span::styled(
        format!("  {gloss}"),
        Style::default().fg(palette.dim),
    )));
    lines.push(Line::from(""));

    // ③ next-steps — stikk-authored, selectable (C-T2b); always exactly one this increment (`Refresh`).
    lines.push(Line::from(Span::styled(
        "  What you can do:",
        Style::default().fg(palette.dim),
    )));
    for (i, step) in next_steps.iter().enumerate() {
        lines.push(selectable(palette, i == cursor, step.label.clone()));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" stikk stopped ")
        .style(Style::default().fg(palette.warn));
    let height = (lines.len() as u16 + 6).min(area.height.saturating_sub(2));
    let region = centered(72, height.max(10), area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        region,
    );
}

fn render_palette(
    filter: &str,
    cursor: usize,
    capability: Capability,
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
        let reason = cmd.unmet_reason(capability);
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
    let region = centered(64, height.max(6), area);
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

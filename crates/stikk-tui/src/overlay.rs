//! The overlay layer (design TU-02/07/08; RFC 007).
//!
//! An overlay is drawn above the active view without destroying it. This increment ships: the
//! **glossary / help** browser (FR-111), the **ref picker** (RFC 006), the **refusal explanation**
//! overlay (TU-08/FR-110), the **command palette** (TU-07/FR-125), and the **recent-refusals** list
//! (FR-112). Overlays form a stack; the top one renders and receives navigation keys.
//!
//! The refusal overlay is the load-bearing one: prikk's message is shown **verbatim and inert**, in a
//! quoted content region visibly distinct from stikk's chrome (C-T2a/C-T2b); the gloss and the
//! next-steps are stikk's own, separate from and below the message (ER-02/C-T4c).

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use stikk_core::{RefusalCard, RefusalRecord, glossary, palette};
use stikk_model::Capability;

use crate::text::inert;
use crate::theme::Palette;

/// One overlay. Data-carrying variants own their state; `Glossary` reads the static asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    /// The glossary / help browser (terminology mapping, key reference, code index).
    Glossary,
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
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Glossary => " Glossary & Help ",
            Self::RefPicker { .. } => " Choose ref ",
            Self::Refusal { .. } => " prikk refused ",
            Self::Palette { .. } => " Command palette ",
            Self::Refusals { .. } => " Recent refusals ",
        }
    }
}

/// Render the top overlay centred over `area`, clearing the region beneath it first (TU-02).
pub fn render(overlay: &Overlay, palette: &Palette, frame: &mut Frame, area: Rect) {
    match overlay {
        Overlay::Glossary => render_glossary(palette, frame, area),
        Overlay::RefPicker { refs, cursor } => {
            render_ref_picker(refs, *cursor, palette, frame, area)
        }
        Overlay::Refusal { card, cursor } => render_refusal(card, *cursor, palette, frame, area),
        Overlay::Palette {
            filter,
            cursor,
            capability,
        } => render_palette(filter, *cursor, *capability, palette, frame, area),
        Overlay::Refusals { records, cursor } => {
            render_refusals(records, *cursor, palette, frame, area);
        }
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

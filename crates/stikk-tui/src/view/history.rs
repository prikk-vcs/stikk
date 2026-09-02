//! The History view (design VW-03, FR-010/011; RFC 006).
//!
//! Renders a ref's block lineage newest-first, preceded by a "queued" tier — the patches in the active
//! WAL that are *not yet history* (FR-010). prikk's history is block-granular (RFC 006, UD-09), so a
//! row is one sealed block: its short id, update-seq, kind, patch count, and lineage markers. Every
//! repository-sourced string is routed through [`crate::text::inert`] (threat model C-T2a).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::HistoryView;
use stikk_prikk::BlockRow;

use crate::text::inert;
use crate::theme::Palette;

/// Render the History view for `view`, with `cursor` marking the selected block, into `area`.
pub fn render(view: &HistoryView, cursor: usize, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // The "not yet history" tier: the active WAL's unsealed patches (FR-010).
    let queue_style = if view.queued == 0 {
        Style::default().fg(palette.dim)
    } else {
        Style::default().fg(palette.warn)
    };
    lines.push(Line::from(vec![
        Span::styled("  queued   ", Style::default().fg(palette.dim)),
        Span::styled(
            format!(
                "{} patch(es) in the active WAL — not yet sealed",
                view.queued
            ),
            queue_style,
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "  ─────────",
        Style::default().fg(palette.dim),
    )));

    if view.blocks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no sealed blocks on this ref yet",
            Style::default().fg(palette.dim),
        )));
    } else {
        for (i, row) in view.blocks.iter().enumerate() {
            lines.push(block_line(row, i == cursor, i == 0, palette));
        }
    }

    let title = format!(" History · {} ", inert(&view.reff));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));

    // Keep the selected row visible: rows start two lines below the top (queue tier + rule).
    let inner_height = area.height.saturating_sub(2); // borders
    let selected_line = cursor as u16 + 2; // offset past the queue tier and rule
    let scroll = selected_line.saturating_sub(inner_height.saturating_sub(1));

    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// One block row: a selection marker, short id, update-seq, kind, patch count, and lineage markers.
fn block_line<'a>(row: &'a BlockRow, selected: bool, is_tip: bool, palette: &Palette) -> Line<'a> {
    let marker = if selected { "▶ " } else { "  " };
    let id_style = if selected {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };

    let mut spans = vec![
        Span::styled(marker, Style::default().fg(palette.accent)),
        Span::styled(short_id(&row.block_id), id_style),
        Span::styled(
            format!("  #{:<4}", row.update_seq),
            Style::default().fg(palette.dim),
        ),
        Span::styled(
            format!("{:<8}", inert(&row.kind)),
            Style::default().fg(palette.fg),
        ),
        Span::styled(
            format!("{} patch(es)", row.patches),
            Style::default().fg(palette.dim),
        ),
    ];
    if row.rollback_block {
        spans.push(Span::styled(
            "  [rollback]",
            Style::default().fg(palette.warn),
        ));
    }
    if row.required_attestations > 0 {
        spans.push(Span::styled(
            format!("  needs {} attestation(s)", row.required_attestations),
            Style::default().fg(palette.warn),
        ));
    }
    if is_tip {
        spans.push(Span::styled("  ← tip", Style::default().fg(palette.ok)));
    }
    Line::from(spans)
}

/// A short, display-only prefix of an object id (never fabricated — a plain prefix, inert).
fn short_id(id: &str) -> String {
    let inert_id = inert(id);
    let short: String = inert_id.chars().take(12).collect();
    short
}

#[cfg(test)]
mod tests;

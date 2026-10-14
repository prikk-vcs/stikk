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

use crate::text::{compose, identifier, wrap_indented};
use crate::theme::Palette;

/// The queued tier's label, and the indent its continuation rows line up under — the same columns,
/// so a wrapped sentence reads as one tier rather than as a second row of the lineage.
const TIER_LABEL: &str = "  queued   ";
const TIER_INDENT: &str = "           ";

/// Render the History view for `view`, with `cursor` marking the selected block, into `area`.
pub fn render(view: &HistoryView, cursor: usize, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // The "not yet history" tier (FR-010), in core's words: whose queue it is, and no line at all when
    // nothing is queued (RFC 028 decision 5). Core composes a ref name into its own sentence, so this
    // is a string of both kinds and takes **prose** — the rule that cannot drop a character (RFC 036
    // decision 4). The same ref is an identifier in this view's title.
    let text_width = usize::from(area.width.saturating_sub(2));
    if let Some(tier) = view.queued_tier() {
        // **Wrapped, never clipped** (RFC 024; carried into RFC 037 Handoff A §5 from RFC 036's
        // review). This was one `Line`, so at 80 columns the sentence was cut at the border
        // mid-word — and a long ref name in it was cut with it, which is the same defect RFC 036
        // fixed one view over in Changes.
        //
        // `wrap_indented` is the word wrap, and it is safe for this string even though it collapses
        // runs of whitespace: the prose around the name is stikk's own and holds single spaces, and
        // the name itself comes through the identifier rule, whose quoted form spells every space
        // ` ` precisely so that it survives a wrap (RFC 036 decision 3). A name too long for a
        // row is hard-split by `wrap_indented` rather than dropped, so every character of it is on
        // screen, in order.
        let sentence = compose(&tier);
        let first_row_room = text_width.saturating_sub(TIER_LABEL.chars().count()).max(1);
        let mut rows = wrap_indented(&sentence, first_row_room, "").into_iter();
        let head = rows.next().unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(TIER_LABEL, Style::default().fg(palette.dim)),
            Span::styled(head, Style::default().fg(palette.warn)),
        ]));
        for row in rows {
            lines.push(Line::from(Span::styled(
                format!("{TIER_INDENT}{row}"),
                Style::default().fg(palette.warn),
            )));
        }
    }
    lines.push(Line::from(Span::styled(
        "  ─────────",
        Style::default().fg(palette.dim),
    )));
    // Block rows start below the tier (when there is one) and the rule.
    let header_rows = u16::try_from(lines.len()).unwrap_or(u16::MAX);

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

    let title = format!(" History · {} ", identifier(&view.reff));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));

    // Keep the selected row visible: rows start `header_rows` lines below the top.
    let inner_height = area.height.saturating_sub(2); // borders
    let selected_line = (cursor as u16).saturating_add(header_rows);
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
            format!("{:<8}", identifier(&row.kind)),
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

/// A short, display-only prefix of an object id (never fabricated — a plain prefix, rendered as the
/// identifier it is).
fn short_id(id: &str) -> String {
    let inert_id = identifier(id);
    // The same form a seal confirmation names a patch by (`stikk_core::confirm::SHORT_ID_CHARS`).
    let short: String = inert_id
        .chars()
        .take(stikk_core::confirm::SHORT_ID_CHARS)
        .collect();
    short
}

#[cfg(test)]
mod tests;

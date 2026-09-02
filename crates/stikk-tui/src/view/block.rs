//! The Block-detail view (design FR-031/032 at block granularity; RFC 006).
//!
//! Renders one block's metadata and — for the ref tip only — the replayed state file set. prikk
//! exposes no per-block state for older blocks and no per-patch content at all (RFC 006, UD-09), so
//! this view is honest about that ceiling rather than fabricating a diff. Every repository-sourced
//! string is routed through [`crate::text::inert`] (threat model C-T2a).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::BlockDetailView;

use crate::text::inert;
use crate::theme::Palette;

/// Render the Block-detail view for `detail` into `area`.
pub fn render(detail: &BlockDetailView, palette: &Palette, frame: &mut Frame, area: Rect) {
    let row = &detail.row;
    let mut lines: Vec<Line> = vec![
        field(palette, "block", inert(&row.block_id)),
        field(palette, "ref-state", inert(&row.ref_state_id)),
        field(palette, "update-seq", row.update_seq.to_string()),
        field(palette, "kind", inert(&row.kind)),
        field(palette, "parents", row.parents.to_string()),
        field(palette, "patches", row.patches.to_string()),
    ];
    if row.rollback_patches > 0 || row.rollback_block {
        lines.push(field(
            palette,
            "rollback",
            format!(
                "{}{} rollback patch(es)",
                if row.rollback_block {
                    "rollback block · "
                } else {
                    ""
                },
                row.rollback_patches
            ),
        ));
    }
    if row.required_attestations > 0 {
        lines.push(field(
            palette,
            "attestations",
            format!("{} required", row.required_attestations),
        ));
    }
    let prev = match &row.previous_ref_state {
        Some(id) => inert(id),
        None => "<none — root>".to_string(),
    };
    lines.push(field(palette, "prev-ref-state", prev));

    lines.push(Line::from(""));

    // State files: prikk can replay only to the tip (RFC 006), so older blocks show no file set.
    match &detail.state {
        Some(state) if detail.is_tip => {
            lines.push(Line::from(Span::styled(
                format!(
                    "  state at tip — {} file(s), {} byte(s):",
                    state.files.len(),
                    state.total_bytes
                ),
                Style::default().fg(palette.dim),
            )));
            if state.files.is_empty() {
                lines.push(Line::from(Span::styled(
                    "    (empty state)",
                    Style::default().fg(palette.dim),
                )));
            } else {
                for file in &state.files {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(inert(file), Style::default().fg(palette.fg)),
                    ]));
                }
            }
        }
        _ => {
            lines.push(Line::from(Span::styled(
                "  state files: prikk replays only to the ref tip — not shown for an older block",
                Style::default().fg(palette.dim),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  per-patch content inspection awaits prikk support (UD-09).",
        Style::default()
            .fg(palette.dim)
            .add_modifier(Modifier::ITALIC),
    )));

    let title = if detail.is_tip {
        " Block · tip ".to_string()
    } else {
        " Block ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// A `label: value` line with the label dimmed and left-padded to a column.
fn field<'a>(palette: &Palette, label: &'a str, value: String) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {label:<15}"), Style::default().fg(palette.dim)),
        Span::styled(value, Style::default().fg(palette.fg)),
    ])
}

#[cfg(test)]
mod tests;

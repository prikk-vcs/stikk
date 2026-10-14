//! The Block-detail view (design FR-031/032 at block granularity; RFC 006; RFC 015).
//!
//! Renders one block's metadata and — for the ref tip only — the replayed state file set. prikk
//! exposes no per-block state for older blocks, and `UD-09` narrows rather than retires (RFC 015 F3):
//! per-patch *content* (operations, preimages, `show`/`diff`) and `log --format json` are still
//! absent, but a patch authored by prikk ≥ 0.32 carries an id and a message, and this view names both.
//!
//! **Since RFC 035 it also lists the block's patches** (`FR-030`), from the block's own
//! `show <block-id>` report: one row per patch in prikk's order, with the message joined **by id** from
//! the `log` row, the operation count, and a cursor whose `Enter` opens Patch detail for that patch —
//! out of the report already read, so drilling in spawns nothing. Below stikk's JSON floor the section
//! says, in stikk's own voice, that stikk reads a patch's content from prikk 0.39 on.
//!
//! **The patch count and the message list are never shown one without the other** (RFC 015 decision
//! 2): a block can legitimately report more patches than it lists messages for — a patch authored
//! below prikk 0.32 carries none, which is absence, not an empty message (RFC 015 F4) — and rendering
//! the list alone would tell a user a block contains fewer patches than it does (`T-T4`). Every
//! repository-sourced string (a message included) is routed through [`crate::text::inert`]
//! (`C-T2a`) and is a display string only — never a next-step, never a glossary trigger (`C-T2b`).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use stikk_core::BlockDetailView;

use crate::text::{identifier, inert};
use crate::theme::Palette;

/// Render the Block-detail view for `detail` into `area`, with `cursor` selecting one of the block's
/// patches.
pub fn render(
    detail: &BlockDetailView,
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let row = &detail.row;
    let mut lines: Vec<Line> = vec![
        field(palette, "block", identifier(&row.block_id)),
        field(palette, "ref-state", identifier(&row.ref_state_id)),
        field(palette, "update-seq", row.update_seq.to_string()),
        field(palette, "kind", identifier(&row.kind)),
        field(palette, "parents", row.parents.to_string()),
        field(
            palette,
            "patches",
            patches_summary(row.patches, row.messages.len()),
        ),
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
        Some(id) => identifier(id),
        None => "<none — root>".to_string(),
    };
    lines.push(field(palette, "prev-ref-state", prev));

    // The message list always travels with the count above it (RFC 015 decision 2) — never rendered
    // on its own, since on its own it would understate the block the moment any patch predates 0.32.
    //
    // **Shown only where the Patches section is not** (RFC 035 §4.1): where the patches are listed,
    // each row already carries its own message, joined by id, beside the operation count — and this
    // list would say the same thing a second time, by a weaker join. Below stikk's JSON floor the
    // Patches section has nothing to list, and this is the only place a message appears at all.
    if detail.patches.unreported_reason().is_some() && !row.messages.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  messages:",
            Style::default().fg(palette.dim),
        )));
        for patch in &row.messages {
            // The id and message are separate lines, not one — a 64-hex id plus a message routinely
            // exceeds a normal terminal's width, and wrapping one long line can split a message's own
            // text across two rows (caught while testing this exact rendering).
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    identifier(&patch.patch_id),
                    Style::default().fg(palette.accent),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(inert(&patch.message), Style::default().fg(palette.fg)),
            ]));
        }
    }

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
                        Span::styled(identifier(file), Style::default().fg(palette.fg)),
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

    // The block's own patches (RFC 035 §4.1). This replaces the line that said per-patch content
    // "still awaits prikk support (UD-09)" — it was true when written, retired at prikk 0.36, and
    // built here.
    lines.push(Line::from(""));
    match detail.patches.unreported_reason() {
        Some(reason) => {
            lines.push(Line::from(Span::styled(
                format!("  patches: {reason}"),
                Style::default()
                    .fg(palette.dim)
                    .add_modifier(Modifier::ITALIC),
            )));
        }
        None => {
            let rows = detail.patches.rows();
            lines.push(Line::from(Span::styled(
                format!("  patches — {} listed · Enter opens one:", rows.len()),
                Style::default().fg(palette.dim),
            )));
            for (at, row) in rows.iter().enumerate() {
                // The selected row is marked the way History's own cursor marks one.
                let selected = at == cursor;
                let marker = if selected { "▸ " } else { "  " };
                let style = if selected {
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.fg)
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("    {marker}"), Style::default()),
                    // Core composes this: a short hex id, the patch's message and stikk's own
                    // count. A sentence holding both kinds, so it takes **prose** — the rule that
                    // cannot drop a character (RFC 036 decision 4). The id is hex and the message
                    // is prose in its own right, so nothing is lost by it; the patch id in full is
                    // an identifier on the row above, and in Patch detail's heading.
                    Span::styled(inert(&row.summary()), style),
                ]));
            }
        }
    }

    let title = if detail.is_tip {
        " Block · tip ".to_string()
    } else {
        " Block ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    // Wrapped: a 64-hex patch id plus its message routinely exceeds a normal terminal's width on one
    // line (caught while testing RFC 015's own message rendering) — every other field here is short
    // enough never to need it, but a message is arbitrary-length repository content.
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// The `patches` field's value: the count alone when it agrees with the message list, or the count
/// plus an explanation when it does not (RFC 015 F4/decision 2, the acceptance-critical behaviour) —
/// a patch authored below prikk 0.32 carries no message, which is absence, not an empty one.
fn patches_summary(patches: u64, message_count: usize) -> String {
    let message_count = message_count as u64;
    if patches == message_count {
        patches.to_string()
    } else {
        format!(
            "{patches} · {message_count} with a message — patches written before prikk 0.32 carry \
             none, which is absence, not an empty message"
        )
    }
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

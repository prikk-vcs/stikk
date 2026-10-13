//! The Patch-detail view (design `FR-030`, `TU-01`; RFC 035 §4.3).
//!
//! Renders the [`stikk_core::PatchDetailView`] the operation layer produces, in its order: the patch
//! id, prikk's `queued`/sealed word with the block it is in, the message, then each operation. **Every
//! word is core's**; this module decides nothing but layout.
//!
//! Every line is repository-adjacent text — paths, spans, file content, key ids, messages — so every
//! one goes through [`inert`] (`C-T2a`). **Long lines wrap, never clip** (RFC 024), and a tall patch
//! scrolls in the Queue view's idiom: the lines are wrapped here first, so their count is exact; the
//! offset is clamped to `rows − viewport` and written back; and the title says where you are.
//!
//! **`v` toggles prikk's own fields** (RFC 035 decision 5). It is the same report either way — a
//! toggle, never a second read — and it is where an `unresolved_node_id`, a blob id and a mode as
//! prikk's own integer are legible rather than apologised for (F6).

use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::PatchDetailView;

use crate::text::{inert, wrap_indented};
use crate::theme::Palette;

/// Indents, by level: the patch id and foot, the heading rows, an operation's title, and its rows.
const TOP: &str = "  ";
const HEADING: &str = "    ";
const OPERATION: &str = "  ";
const DETAIL: &str = "      ";

/// Render Patch detail for `view` into `area`, scrolled to `offset` (clamped here and written back).
/// `raw` shows prikk's own fields for each operation instead of stikk's rendering of them.
pub fn render(
    view: &PatchDetailView,
    raw: bool,
    offset: &Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border, less a one-column gutter so wrapped text does not sit against the edge.
    let width = usize::from(area.width.saturating_sub(3));
    let lines = patch_lines(view, raw, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // `NFR-A03`: which view is showing, what switches it, and that there is more, are all said rather
    // than left to guess. **The `v` hint lives in the title, not in a row at the foot**: this view
    // scrolls, and a hint at the foot is a hint below the fold exactly when the patch is big enough to
    // need it (caught by this module's own capture test).
    let mut title = if raw {
        " Patch · prikk's own fields — v: stikk's rendering ".to_string()
    } else {
        " Patch — v: prikk's own fields ".to_string()
    };
    if max_offset > 0 {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        title.push_str(&format!(
            "· ↑/↓ to scroll · lines {first}–{last} of {} ",
            lines.len()
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Every row of the view, already wrapped to `width`, so the caller's scroll clamp is exact.
fn patch_lines<'a>(
    view: &PatchDetailView,
    raw: bool,
    palette: &Palette,
    width: usize,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let mut push = |text: &str, indent: &str, style: Style| {
        for row in wrap_indented(&inert(text), width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    };

    push(
        &view.patch_id,
        TOP,
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    for row in &view.heading {
        push(row, HEADING, Style::default().fg(palette.fg));
    }
    for operation in &view.operations {
        push("", TOP, Style::default());
        push(
            &operation.title,
            OPERATION,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        );
        // The same operation, two ways, out of the one report: stikk's rendering, or prikk's fields.
        if raw {
            for row in &operation.raw {
                // **prikk's JSON keeps its own nesting.** `wrap_indented` word-wraps, which collapses
                // runs of whitespace — right for a sentence, wrong for a report whose shape is part of
                // what the raw view is for (decision 5). So each row's own leading spaces become part
                // of its indent, and a long value still wraps under itself rather than clipping.
                let lead: String = row.chars().take_while(|ch| *ch == ' ').collect();
                push(
                    row.trim_start(),
                    &format!("{DETAIL}{lead}"),
                    Style::default().fg(palette.fg),
                );
            }
        } else {
            for row in &operation.rows {
                push(row, DETAIL, Style::default().fg(palette.fg));
            }
        }
    }
    if let Some(foot) = &view.foot {
        push("", TOP, Style::default());
        push(
            foot,
            TOP,
            Style::default()
                .fg(palette.dim)
                .add_modifier(Modifier::ITALIC),
        );
    }
    lines
}

#[cfg(test)]
mod tests;

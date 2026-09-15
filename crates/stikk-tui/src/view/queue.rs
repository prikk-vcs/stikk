//! The Queue view (design `TU-01`'s Queue row, `FR-051`; RFC 028 Handoff A §6).
//!
//! Renders the `stikk_core::QueueView` the operation layer produces, in its order: the heading, the
//! thresholds, each patch (its id, its message line, its operations), and the one line at the foot. **Every
//! word is core's**; this module decides nothing but layout. Every line is repository-adjacent text — ref
//! names, paths, messages, key ids — so every one goes through [`inert`] (`C-T2a`).
//!
//! **Long lines wrap, never clip** (RFC 024): a message or a path wider than the view continues on the
//! next row at the same indent, so nothing prikk reported is cut off at the right edge.
//!
//! **And a tall queue scrolls** (RFC 028 A review v1 §2.1), in the Glossary's idiom: the lines are wrapped
//! here first, so their count is exact; the offset is clamped to `rows − viewport` and written back; and the
//! title says where you are whenever there is more than fits.

use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::QueueView;

use crate::text::{inert, wrap_indented};
use crate::theme::Palette;

/// Indents, by level: the heading and foot, a patch's message, and its operations.
const TOP: &str = "  ";
const MESSAGE: &str = "    ";
const OPERATION: &str = "      ";

/// Render the Queue view for `view` into `area`, scrolled to `offset` (clamped here and written back).
pub fn render(
    view: &QueueView,
    offset: &Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border, less a one-column gutter so wrapped text does not sit against the edge.
    let width = usize::from(area.width.saturating_sub(3));
    let lines = queue_lines(view, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // `NFR-A03`: "there is more" is said, never left to guess — only when there is somewhere to go.
    let title = if max_offset == 0 {
        " Queue ".to_string()
    } else {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        format!(
            " Queue — ↑/↓ to scroll · lines {first}–{last} of {} ",
            lines.len()
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Every row of the view, already wrapped to `width`, so the scroll clamp is exact.
fn queue_lines<'a>(view: &QueueView, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let mut push = |text: &str, indent: &str, style: Style| {
        for row in wrap_indented(&inert(text), width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    };

    push(
        &view.heading,
        TOP,
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
    );
    for threshold in &view.thresholds {
        push(threshold, TOP, Style::default().fg(palette.dim));
    }
    for patch in &view.patches {
        push("", TOP, Style::default());
        push(&patch.id, TOP, Style::default().fg(palette.accent));
        if let Some(message) = &patch.message {
            push(message, MESSAGE, Style::default().fg(palette.fg));
        }
        for operation in &patch.operations {
            push(operation, OPERATION, Style::default().fg(palette.fg));
        }
    }
    if let Some(foot) = &view.foot {
        push("", TOP, Style::default());
        push(foot, TOP, Style::default().fg(palette.dim));
    }
    lines
}

#[cfg(test)]
mod tests;

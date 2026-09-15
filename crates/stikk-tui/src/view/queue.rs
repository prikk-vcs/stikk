//! The Queue view (design `TU-01`'s Queue row, `FR-051`; RFC 028 Handoff A §6).
//!
//! Renders the `stikk_core::QueueView` the operation layer produces, in its order: the heading, the
//! thresholds, each patch (its id, its message line, its operations), and the one line at the foot. **Every
//! word is core's**; this module decides nothing but layout. Every line is repository-adjacent text — ref
//! names, paths, messages, key ids — so every one goes through [`inert`] (`C-T2a`).
//!
//! **Long lines wrap, never clip** (RFC 024): a message or a path wider than the view continues on the
//! next row at the same indent, so nothing prikk reported is cut off at the right edge.

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

/// Render the Queue view for `view` into `area`.
pub fn render(view: &QueueView, palette: &Palette, frame: &mut Frame, area: Rect) {
    // Inside the border, less a one-column gutter so wrapped text does not sit against the edge.
    let width = usize::from(area.width.saturating_sub(3));
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

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Queue ")
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests;

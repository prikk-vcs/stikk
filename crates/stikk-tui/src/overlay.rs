//! The overlay layer (design TU-02; handoff §2 `overlay.rs`).
//!
//! An overlay is drawn above the active view without destroying it. This increment ships one — Help —
//! and the machinery the refusal-explanation, glossary, and command-palette overlays plug into next.
//! Overlays form a stack; the top one renders and receives the "close" key.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Palette;

/// One overlay. Deliberately a small enum this increment; it grows as overlays are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// A static key reference.
    Help,
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Help => " Help ",
        }
    }
}

/// Render the top overlay centred over `area`, clearing the region beneath it first so the view
/// below shows through only outside the overlay (design TU-02: never destroys the view state, only
/// paints above it).
pub fn render(overlay: Overlay, palette: &Palette, frame: &mut Frame, area: Rect) {
    match overlay {
        Overlay::Help => render_help(palette, frame, area),
    }
}

fn render_help(palette: &Palette, frame: &mut Frame, area: Rect) {
    let lines = vec![
        key_line(palette, "?", "toggle this help"),
        key_line(palette, "r", "refresh — re-read from prikk"),
        key_line(palette, "q / Esc", "close overlay, or quit"),
        key_line(palette, "Ctrl-C", "quit"),
        Line::from(""),
        Line::from(Span::styled(
            "stikk reads prikk; it never writes your repository.",
            ratatui::style::Style::default().fg(palette.dim),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Overlay::Help.title())
        .style(ratatui::style::Style::default().fg(palette.fg));
    let region = centered(60, lines.len() as u16 + 2, area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
}

fn key_line<'a>(palette: &Palette, key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {key:<9}"),
            ratatui::style::Style::default().fg(palette.accent),
        ),
        Span::styled(desc, ratatui::style::Style::default().fg(palette.fg)),
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

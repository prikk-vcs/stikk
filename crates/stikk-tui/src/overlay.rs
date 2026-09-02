//! The overlay layer (design TU-02; handoff §2 `overlay.rs`).
//!
//! An overlay is drawn above the active view without destroying it. This increment ships three — Help,
//! a ref picker, and a transient notice — and the machinery the refusal-explanation, glossary, and
//! command-palette overlays plug into next. Overlays form a stack; the top one renders and receives
//! navigation keys.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::text::inert;
use crate::theme::Palette;

/// One overlay. It grows as overlays are added; `RefPicker` and `Notice` carry their own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    /// A static key reference.
    Help,
    /// A ref chooser: the ref names and the highlighted index.
    RefPicker {
        /// The selectable ref names (repository-sourced; rendered inert).
        refs: Vec<String>,
        /// The highlighted entry.
        cursor: usize,
    },
    /// A transient message carrying prikk's verbatim text (design NFR-I03).
    Notice(String),
}

impl Overlay {
    /// The overlay's title bar text.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Help => " Help ",
            Self::RefPicker { .. } => " Choose ref ",
            Self::Notice(_) => " prikk reported ",
        }
    }
}

/// Render the top overlay centred over `area`, clearing the region beneath it first so the view
/// below shows through only outside the overlay (design TU-02: never destroys the view state, only
/// paints above it).
pub fn render(overlay: &Overlay, palette: &Palette, frame: &mut Frame, area: Rect) {
    match overlay {
        Overlay::Help => render_help(palette, frame, area),
        Overlay::RefPicker { refs, cursor } => {
            render_ref_picker(refs, *cursor, palette, frame, area)
        }
        Overlay::Notice(message) => render_notice(message, palette, frame, area),
    }
}

fn render_help(palette: &Palette, frame: &mut Frame, area: Rect) {
    let lines = vec![
        key_line(palette, "Enter", "open / drill in"),
        key_line(palette, "↑/↓ or j/k", "move selection"),
        key_line(palette, "b", "choose which ref to view"),
        key_line(palette, "Esc / q", "back, or quit at the top"),
        key_line(palette, "r", "refresh — re-read from prikk"),
        key_line(palette, "?", "toggle this help"),
        key_line(palette, "Ctrl-C", "quit"),
        Line::from(""),
        Line::from(Span::styled(
            "stikk reads prikk; it never writes your repository.",
            Style::default().fg(palette.dim),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Overlay::Help.title())
        .style(Style::default().fg(palette.fg));
    let region = centered(62, lines.len() as u16 + 2, area);
    frame.render_widget(Clear, region);
    frame.render_widget(Paragraph::new(lines).block(block), region);
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
            .map(|(i, name)| {
                let selected = i == cursor;
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
                    Span::styled(inert(name), style),
                ])
            })
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

fn render_notice(message: &str, palette: &Palette, frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            inert(message),
            Style::default().fg(palette.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "press Esc to dismiss",
            Style::default().fg(palette.dim),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" prikk reported ")
        .style(Style::default().fg(palette.warn));
    let region = centered(64, 7, area);
    frame.render_widget(Clear, region);
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        region,
    );
}

fn key_line<'a>(palette: &Palette, key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key:<11}"), Style::default().fg(palette.accent)),
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

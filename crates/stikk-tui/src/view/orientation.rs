//! The Orientation view (design VW-01, FR-002; use case UC-01).
//!
//! Renders the `stikk_core::OrientationView` the operation layer produces: prikk version and support,
//! the session's derived capability, signing readiness, queue depth, a torn-tail warning, and the
//! `heads/main` state. Every repository-sourced string is routed through [`crate::text::inert`] before
//! it reaches a cell (threat model C-T2a).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::OrientationView;

use crate::text::inert;
use crate::theme::Palette;

/// Render the Orientation view into `area`.
pub fn render(view: &OrientationView, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    let (support_text, support_style) = if view.prikk_supported {
        ("supported", Style::default().fg(palette.dim))
    } else {
        (
            "outside stikk's validated range — read-only",
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )
    };
    lines.push(field(
        palette,
        "prikk",
        vec![
            Span::styled(inert(&view.prikk_version), Style::default().fg(palette.fg)),
            Span::raw("  "),
            Span::styled(support_text, support_style),
        ],
    ));

    lines.push(field(
        palette,
        "capability",
        vec![Span::styled(
            view.capability.name(),
            Style::default().fg(palette.accent),
        )],
    ));

    lines.push(field(
        palette,
        "signing",
        vec![Span::raw(signing_line(view))],
    ));

    lines.push(field(
        palette,
        "queued",
        vec![Span::styled(
            view.queued_patches.to_string(),
            Style::default().fg(palette.fg),
        )],
    ));

    if view.trailing_partial_wal_bytes != 0 {
        lines.push(field(
            palette,
            "warning",
            vec![Span::styled(
                format!(
                    "{} trailing partial WAL byte(s) — an interrupted commit left a torn tail",
                    view.trailing_partial_wal_bytes
                ),
                Style::default().fg(palette.warn),
            )],
        ));
    }

    let main_ref = match &view.main_ref_state {
        Some(id) => inert(id),
        None => "<unpublished>".to_string(),
    };
    lines.push(field(
        palette,
        "heads/main",
        vec![Span::styled(main_ref, Style::default().fg(palette.fg))],
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Orientation ")
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// A `label: value` line with the label dimmed and left-padded to a column.
fn field<'a>(palette: &Palette, label: &'a str, mut value: Vec<Span<'a>>) -> Line<'a> {
    let mut spans = vec![Span::styled(
        format!("  {label:<12}"),
        Style::default().fg(palette.dim),
    )];
    spans.append(&mut value);
    Line::from(spans)
}

/// The signing-readiness summary (never key material — presence only, design C-I1).
fn signing_line(view: &OrientationView) -> String {
    let ready = |flag: bool| if flag { "ready" } else { "not ready" };
    let mut s = format!(
        "author {} · maintainer {}",
        ready(view.readiness.author_ready),
        ready(view.readiness.maintainer_ready)
    );
    if view.readiness.read_only {
        s.push_str(" · read-only");
    }
    s
}

#[cfg(test)]
mod tests;

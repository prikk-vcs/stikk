//! The status bar (design TU-03; handoff §2 `status_bar.rs`; RFC 010).
//!
//! One line: repository, focused ref (never "HEAD" — it does not exist; this increment shows the
//! literal `heads/main`), queue depth, worktree marker, the `⟳ n` background-operation indicator
//! (TU-03; RFC 010 — the count of requests the worker has not yet answered), and the
//! capability/readiness badges. Every badge has a text form so a monochrome terminal loses nothing
//! (design NFR-A03).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use stikk_model::Readiness;

use crate::app::{App, OrientationState};
use crate::text::inert;
use crate::theme::Palette;

/// Render the status bar for `app` into the one-line `area`.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let palette = app.palette();
    let repo = app
        .repo()
        .file_name()
        .map(|n| inert(&n.to_string_lossy()))
        .unwrap_or_else(|| "?".to_string());

    let (readiness, queued, loaded) = match app.state() {
        OrientationState::Loaded(view) => (view.readiness, view.queued_patches, true),
        _ => (Readiness::none(), 0, false),
    };

    let mut spans = vec![
        Span::styled(
            repo,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        ),
        sep(palette),
        // The focused ref — a client-side pointer, not a HEAD (design FR-055).
        Span::styled(
            inert(app.focused_ref()),
            Style::default().fg(palette.accent),
        ),
    ];

    if !loaded {
        spans.push(sep(palette));
        spans.push(Span::styled(
            match app.state() {
                OrientationState::Failed(_) => "(error)",
                _ => "(loading)",
            },
            Style::default().fg(palette.dim),
        ));
    } else {
        if queued > 0 {
            spans.push(sep(palette));
            spans.push(Span::styled(
                format!("●{queued} queued"),
                Style::default().fg(palette.warn),
            ));
        }
        spans.push(sep(palette));
        spans.append(&mut badges(palette, readiness));
    }

    let in_flight = app.in_flight_count();
    if in_flight > 0 {
        spans.push(sep(palette));
        spans.push(Span::styled(
            format!("⟳ {in_flight}"),
            Style::default().fg(palette.accent),
        ));
    }

    spans.push(Span::styled(
        "   :palette  ?:help  q:back",
        Style::default().fg(palette.dim),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn sep(palette: &Palette) -> Span<'static> {
    Span::styled("  ·  ", Style::default().fg(palette.dim))
}

/// The capability/readiness badges (design TU-03): `[RO]` when read-only, then author and maintainer
/// readiness. Text-forward so colour is never load-bearing.
fn badges(palette: &Palette, readiness: Readiness) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    if readiness.read_only {
        out.push(Span::styled(
            "[RO] ",
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        ));
    }
    out.push(badge(palette, "AUT", readiness.author_ready));
    out.push(Span::raw(" "));
    out.push(badge(palette, "MNT", readiness.maintainer_ready));
    out
}

fn badge(palette: &Palette, label: &str, ready: bool) -> Span<'static> {
    let (mark, style) = if ready {
        ("✓", Style::default().fg(palette.ok))
    } else {
        ("–", Style::default().fg(palette.dim))
    };
    Span::styled(format!("[{label} {mark}]"), style)
}

#[cfg(test)]
mod tests;

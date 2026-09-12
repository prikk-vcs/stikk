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

use stikk_model::{Binding, Readiness, RoleReadiness};

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
    out.push(role_badge(palette, "AUT", readiness.author));
    out.push(Span::raw(" "));
    out.push(role_badge(palette, "MNT", readiness.maintainer));
    out
}

/// One role's badge (RFC 016 §4, extended to both roles by RFC 026 §3): every state rendered
/// distinctly, never collapsed to two.
///
/// **`?` and `✓` must never be shared** — `C-T2c′`: a claim stikk cannot verify must not render as a
/// pass. Three glyphs carry four ideas:
///
/// | state | glyph | why |
/// |---|---|---|
/// | `Known(Matches)` | `✓` | prikk answered and the key binds |
/// | `Known(Unrecorded)` | `✓` | prikk answered and will bind on first signature — it *can* sign |
/// | `Unknown` | `?` | key material present, binding unanswerable on this prikk |
/// | `Unverifiable` | `?` | stikk cannot see whether there is key material at all (prikk 0.40) |
/// | `Known(NotAdopted \| Mismatch)` | `✗` | prikk answered, and the answer is that it will refuse |
/// | `NotReady` | `–` | nothing to sign with |
///
/// `?` covers both unknowns deliberately at *this* size — a one-cell badge cannot carry the
/// distinction, and both mean "stikk is not claiming". The distinction that matters is in what the
/// session can *do*, which `Capability::derive` already separates, and in the Orientation view's
/// full-sentence rendering, which has room to say which unknown it is.
fn role_badge(palette: &Palette, label: &str, readiness: RoleReadiness) -> Span<'static> {
    let (mark, style) = match readiness {
        RoleReadiness::Known(Binding::Matches | Binding::Unrecorded) => {
            ("✓", Style::default().fg(palette.ok))
        }
        RoleReadiness::Known(Binding::NotAdopted | Binding::Mismatch) => {
            ("✗", Style::default().fg(palette.warn))
        }
        RoleReadiness::Known(Binding::Absent) | RoleReadiness::NotReady => {
            ("–", Style::default().fg(palette.dim))
        }
        RoleReadiness::Unknown | RoleReadiness::Unverifiable => {
            ("?", Style::default().fg(palette.warn))
        }
    };
    Span::styled(format!("[{label} {mark}]"), style)
}

#[cfg(test)]
mod tests;

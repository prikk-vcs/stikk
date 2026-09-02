//! The Changes view (design VW-06, FR-034; RFC 008).
//!
//! Renders `stikk_core::ChangesView` — worktree-vs-baseline for the focused ref, at the **path level**
//! prikk's `worktree-status` reports (changed / missing / untracked / unsupported). It is honest about
//! two ceilings: per-file **content** diffs await prikk support (UD-09, so none is faked — threat
//! T-T4), and commits are **whole-worktree** (UD-06). The untracked group has a display-only filter
//! (UD-08) that never hides that a commit would still capture those files. Every worktree path is
//! routed through [`crate::text::inert`] (threat model C-T2a).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{ChangeEntry, ChangeKind, ChangesView};

use crate::text::inert;
use crate::theme::Palette;

/// Render the Changes view for `view` into `area`. `hide_untracked` applies the UD-08 display filter.
pub fn render(
    view: &ChangesView,
    hide_untracked: bool,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let mut lines: Vec<Line> = Vec::new();

    // Headline: clean vs changed (text-forward — colour is never the only signal, NFR-A03).
    if view.clean {
        lines.push(Line::from(Span::styled(
            "  clean against baseline",
            Style::default().fg(palette.ok),
        )));
    } else {
        let changed = view.modified + view.missing + view.untracked + view.unsupported;
        lines.push(Line::from(Span::styled(
            format!("  {changed} change(s) against baseline"),
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // Counts summary.
    lines.push(Line::from(Span::styled(
        format!(
            "  tracked {} · unchanged {} · modified {} · missing {} · untracked {} · unsupported {}",
            view.tracked, view.unchanged, view.modified, view.missing, view.untracked, view.unsupported
        ),
        Style::default().fg(palette.dim),
    )));
    lines.push(Line::from(Span::styled(
        "  ─────────",
        Style::default().fg(palette.dim),
    )));

    // Entries, grouped so the most actionable come first; untracked hidden when filtered (UD-08).
    let mut untracked_hidden = 0u64;
    for entry in &view.entries {
        if hide_untracked && entry.kind.is_untracked() {
            untracked_hidden += 1;
            continue;
        }
        lines.push(entry_line(entry, palette));
    }
    if view.entries.is_empty() && view.clean {
        lines.push(Line::from(Span::styled(
            "  the worktree matches the baseline",
            Style::default().fg(palette.dim),
        )));
    }

    // UD-08: the filter is display-only and always says a commit still captures the hidden files.
    if hide_untracked && untracked_hidden > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {untracked_hidden} untracked hidden (display only) — a commit still captures them",
            ),
            Style::default().fg(palette.warn),
        )));
    }

    // UD-06 and UD-09 honesty.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  commits are whole-worktree — there is no staging (u: toggle untracked)",
        Style::default().fg(palette.dim),
    )));
    lines.push(Line::from(Span::styled(
        "  per-file content diff awaits prikk support (UD-09)",
        Style::default()
            .fg(palette.dim)
            .add_modifier(Modifier::ITALIC),
    )));

    let title = format!(" Changes · {} ", inert(&view.reff));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// One change row: a text-forward kind tag, the inert path, and prikk's dimmed note.
fn entry_line<'a>(entry: &'a ChangeEntry, palette: &Palette) -> Line<'a> {
    let (tag, tag_style) = match &entry.kind {
        ChangeKind::Modified => ("modified   ", Style::default().fg(palette.warn)),
        ChangeKind::Missing => ("missing    ", Style::default().fg(palette.warn)),
        ChangeKind::Untracked => ("untracked  ", Style::default().fg(palette.dim)),
        ChangeKind::Unsupported => ("unsupported", Style::default().fg(palette.warn)),
        ChangeKind::Other(_) => ("changed    ", Style::default().fg(palette.warn)),
    };
    Line::from(vec![
        Span::styled(format!("  {tag} "), tag_style),
        Span::styled(inert(&entry.path), Style::default().fg(palette.fg)),
        Span::styled(
            format!("  — {}", inert(&entry.note)),
            Style::default().fg(palette.dim),
        ),
    ])
}

#[cfg(test)]
mod tests;

//! The Changes view (design VW-06, FR-034; RFC 008; RFC 009 F4; RFC 027).
//!
//! Renders `stikk_core::ChangesView` — worktree-vs-baseline for the focused ref, at the **path level**
//! prikk's `worktree-status` reports (changed / missing / untracked / unsupported). It is honest about
//! two ceilings: per-file **content** diffs await prikk support (UD-09, so none is faked — threat
//! T-T4), and commits are **whole-worktree** (UD-06). The untracked group has a display-only filter
//! (UD-08) that never hides that a commit would still capture those files. Every worktree path is
//! routed through [`crate::text::inert`] (threat model C-T2a).
//!
//! **prikk's commit verdict** (RFC 027 decisions 3 and 4). At prikk ≥ 0.39 an entry `commit` would
//! refuse carries a text marker on its own row, under the path, with prikk's reason verbatim and inert —
//! never colour alone (`NFR-A03`) — and the header counts refusals. Below 0.39 one line says the verdict
//! is not reported; `refused 0` never appears there.
//!
//! When [`stikk_core::ChangesView::queued_elsewhere`] is present, it renders as a distinct warning band
//! **above** the entries. Below prikk 0.39 that band is prikk's own sentence, verbatim and inert, under
//! "prikk reported —" (ER-02/C-T2a). At ≥ 0.39 prikk reports only the queued ref, so the band is
//! **stikk's words** ([`stikk_core::queued_elsewhere_clauses`]), labelled as stikk's and drawn outside
//! the quote band, which stays reserved for prikk's text (RFC 027 F6, `C-T2b`). Either way it is prikk's
//! statement that some "untracked" paths below may be committed-but-unsealed work queued on another
//! ref, and while it is present the untracked filter's "a commit still captures them" claim is
//! **suppressed and replaced** by a pointer to the warning: the two would otherwise contradict each
//! other, and prikk's fact is the true one (RFC 009 decision 3) — showing both would be exactly the
//! confident-but-wrong picture (`T-T4`) this project treats as its worst failure.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{Authoring, ChangeEntry, ChangeKind, ChangesView, QueuedElsewhere};

use crate::text::{inert, wrap_indented};
use crate::theme::Palette;

/// The columns an entry row spends before its path: two of margin, an 11-column kind tag, one space.
/// A refused entry's marker row starts here, so the marker costs the path row nothing.
const ENTRY_INDENT: &str = "              ";
/// The refused marker, in stikk's words; prikk's reason follows it.
const REFUSED_MARKER: &str = "refused — ";

/// Render the Changes view for `view` into `area`. `hide_untracked` applies the UD-08 display filter.
pub fn render(
    view: &ChangesView,
    hide_untracked: bool,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border. Rows that carry prikk's prose wrap to this rather than clip.
    let text_width = usize::from(area.width.saturating_sub(2));
    let dim = Style::default().fg(palette.dim);
    let mut lines: Vec<Line> = Vec::new();

    // Headline: clean vs changed (text-forward — colour is never the only signal, NFR-A03). **It counts
    // what is listed** (RFC 027 §5): every entry, an unmodelled kind's included, so the number matches
    // the list below it rather than the sum of the four kinds stikk names.
    if view.clean {
        lines.push(Line::from(Span::styled(
            "  clean against baseline",
            Style::default().fg(palette.ok),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {} change(s) against baseline", view.entries.len()),
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // Counts, on two rows (RFC 027 Handoff A's ask 2). On one row the counts clipped `unsupported N` at
    // 80 columns, and `refused N` makes the line longer still — every number is now on screen.
    lines.push(Line::from(Span::styled(
        format!(
            "  tracked {} · unchanged {} · modified {} · missing {}",
            view.tracked, view.unchanged, view.modified, view.missing
        ),
        dim,
    )));
    let second = match view.refused {
        Some(refused) => format!(
            "  untracked {} · unsupported {} · refused {refused}",
            view.untracked, view.unsupported
        ),
        None => format!(
            "  untracked {} · unsupported {}",
            view.untracked, view.unsupported
        ),
    };
    lines.push(Line::from(Span::styled(second, dim)));
    // Below prikk 0.39 the verdict is unreported, which is not zero (`C-T2c′`). Said once, and only
    // where there are entries a verdict would be about.
    if view.refused.is_none() && !view.clean {
        for row in wrap_indented(
            "refused: not reported by this prikk (commit's verdict needs prikk ≥ 0.39)",
            text_width,
            "  ",
        ) {
            lines.push(Line::from(Span::styled(row, dim)));
        }
    }
    lines.push(Line::from(Span::styled("  ─────────", dim)));

    match &view.queued_elsewhere {
        // RFC 009 F4: prikk's queued-elsewhere warning, verbatim and inert, in a quoted band clearly
        // distinct from stikk's own chrome (C-T2a/C-T2b) — the same "prikk reported" pattern the
        // refusal overlay uses for prikk's own text.
        //
        // **Wrapped, with the bar on every row** (RFC 027 B). Unwrapped, prikk's sentence clipped at 80
        // columns after "not hea", so a user below prikk 0.39 never saw "real, committed work" or "do
        // not delete" — the claims this band exists for. The words are unchanged; only where the rows
        // break is stikk's, exactly as the refusal card wraps prikk's text (RFC 026 Handoff C §1).
        Some(QueuedElsewhere::Note(note)) => {
            lines.push(Line::from(Span::styled("  prikk reported —", dim)));
            for raw in note.lines() {
                for row in wrap_indented(&inert(raw), text_width, "    ") {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(palette.warn)),
                        Span::styled(
                            row.get(4..).unwrap_or_default().to_string(),
                            Style::default().fg(palette.fg),
                        ),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
        // RFC 027 F6: prikk ≥ 0.39 reports the queued ref and not its sentence. This band is stikk's
        // words, says so in its label, and uses its own bar — never the `│` of the quote band above.
        Some(QueuedElsewhere::Ref(queued_ref)) => {
            lines.push(Line::from(Span::styled(
                "  stikk's warning, from prikk's queued-work report —",
                dim,
            )));
            for clause in
                stikk_core::queued_elsewhere_clauses(&inert(queued_ref), &inert(&view.reff))
            {
                for row in wrap_indented(&clause, text_width, "    ") {
                    lines.push(Line::from(vec![
                        Span::styled("  ! ", Style::default().fg(palette.warn)),
                        Span::styled(
                            row.get(4..).unwrap_or_default().to_string(),
                            Style::default().fg(palette.fg),
                        ),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
        None => {}
    }

    // Entries, in prikk's order; untracked hidden when filtered (UD-08).
    let mut untracked_hidden = 0u64;
    for entry in &view.entries {
        if hide_untracked && entry.kind.is_untracked() {
            untracked_hidden += 1;
            continue;
        }
        lines.extend(entry_lines(entry, palette, text_width));
    }
    if view.entries.is_empty() && view.clean {
        lines.push(Line::from(Span::styled(
            "  the worktree matches the baseline",
            dim,
        )));
    }

    // UD-08: the filter is display-only. Its usual claim — "a commit still captures the hidden
    // files" — is suppressed while a queued-elsewhere warning is present (RFC 009 decision 3): that
    // claim can be the opposite of true here (the files may already be committed, queued elsewhere),
    // so pointing back at the warning is the only honest thing to say. The pointer names whose words
    // the warning is in.
    if hide_untracked && untracked_hidden > 0 {
        lines.push(Line::from(""));
        let text = match &view.queued_elsewhere {
            Some(QueuedElsewhere::Note(_)) => format!(
                "{untracked_hidden} untracked hidden (display only) — see prikk's warning above \
                 before assuming these are safe to lose",
            ),
            Some(QueuedElsewhere::Ref(_)) => format!(
                "{untracked_hidden} untracked hidden (display only) — see the queued-work warning \
                 above before assuming these are safe to lose",
            ),
            None => format!(
                "{untracked_hidden} untracked hidden (display only) — a commit still captures them",
            ),
        };
        for row in wrap_indented(&text, text_width, "  ") {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.warn),
            )));
        }
    }

    // UD-06 and UD-09 honesty.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  commits are whole-worktree — there is no staging (u: toggle untracked)",
        dim,
    )));
    lines.push(Line::from(Span::styled(
        "  per-file content diff awaits prikk support (UD-09)",
        dim.add_modifier(Modifier::ITALIC),
    )));

    let title = format!(" Changes · {} ", inert(&view.reff));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// One change: a text-forward kind tag, the inert path, and prikk's dimmed note — then, for an entry
/// prikk reports `commit` would refuse, a marker row under the path with prikk's reason.
///
/// **The marker has its own row** (RFC 027 Handoff A's ask 1): the path row already spends 14 columns
/// on the tag, and a marker there would take more from the path at 80 columns. Here the path row is
/// unchanged, and the reason — prikk's, verbatim — wraps under the marker instead of clipping. Its kind
/// stays on the path row: refused is orthogonal to kind (RFC 027 F1).
fn entry_lines(entry: &ChangeEntry, palette: &Palette, text_width: usize) -> Vec<Line<'static>> {
    let (tag, tag_style) = match &entry.kind {
        ChangeKind::Modified => ("modified".to_string(), Style::default().fg(palette.warn)),
        ChangeKind::Missing => ("missing".to_string(), Style::default().fg(palette.warn)),
        ChangeKind::Untracked => ("untracked".to_string(), Style::default().fg(palette.dim)),
        ChangeKind::Unsupported => ("unsupported".to_string(), Style::default().fg(palette.warn)),
        // Handoff A's ask 3: prikk's own word, inert, in the tag position — not stikk's `changed`,
        // which would paraphrase the one thing prikk said about a kind stikk does not know (`ER-02`).
        ChangeKind::Other(word) => (inert(word).to_string(), Style::default().fg(palette.warn)),
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(format!("  {tag:<11} "), tag_style),
        Span::styled(
            inert(&entry.path).to_string(),
            Style::default().fg(palette.fg),
        ),
        Span::styled(
            format!("  — {}", inert(&entry.note)),
            Style::default().fg(palette.dim),
        ),
    ])];

    if let Authoring::Refused(reason) = &entry.authoring {
        // Wrapped with the marker's width as indent, then the first row's indent replaced by the
        // marker — so every row of prikk's reason starts in the same column, and the marker (stikk's)
        // and the reason (prikk's) carry different styles.
        let reason_indent = " ".repeat(ENTRY_INDENT.len() + REFUSED_MARKER.chars().count());
        let reason = inert(reason);
        for (i, row) in wrap_indented(&reason, text_width, &reason_indent)
            .into_iter()
            .enumerate()
        {
            let text = row
                .get(reason_indent.len()..)
                .unwrap_or_default()
                .to_string();
            let lead = if i == 0 {
                Span::styled(
                    format!("{ENTRY_INDENT}{REFUSED_MARKER}"),
                    Style::default()
                        .fg(palette.warn)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(reason_indent.clone())
            };
            lines.push(Line::from(vec![
                lead,
                Span::styled(text, Style::default().fg(palette.fg)),
            ]));
        }
    }
    lines
}

#[cfg(test)]
mod tests;

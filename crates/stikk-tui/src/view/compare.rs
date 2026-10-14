//! The Compare view (design `FR-033`; RFC 038 decision 5).
//!
//! Renders the [`stikk_core::CompareView`] the operation layer produces. **Every word is core's or
//! prikk's**; this module decides layout, colour and wrapping, and nothing else. stikk computes no
//! part of a comparison (RFC 038 §7 trap 1).
//!
//! **Two wraps, and the difference is the whole point** (Patch detail's rule, applied to a view
//! whose entire content is whitespace-sensitive). A hunk's content rows go through
//! [`wrap_verbatim`], which changes nothing; stikk's own sentences word-wrap through
//! [`wrap_indented`]. A diff is where whitespace edits live, so a renderer that rebuilt a content
//! row from its whitespace-separated words would show a re-indent, a tab turned into spaces or a
//! stripped trailing space as *no change at all* — `T-T4`'s confident-but-wrong picture, in the one
//! view built to show exactly those edits.
//!
//! **Markers are coloured, content is not.** `-` takes the theme's warn, `+` its accent, context
//! its ordinary foreground. The colour is on the marker column, which is stikk's chrome; the quoted
//! content beside it reads the same whichever side it is on.
//!
//! It scrolls in Patch detail's idiom — rows are wrapped here first, so the count is exact, the
//! offset is clamped and written back, and the title says where you are — and `v` shows prikk's own
//! JSON for the selected entry, the same toggle Patch detail has.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{CompareView, ComparedEntry, HunkRow};

use crate::text::{compose, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

/// Indents, by level: the heading and foot, an entry's title, and the rows under it.
const TOP: &str = "  ";
const ENTRY: &str = "  ";
const DETAIL: &str = "    ";

/// Render Compare for `view` into `area`, scrolled to `offset` (clamped here and written back).
/// `cursor` selects an entry — which `v` then shows prikk's own fields for.
pub fn render(
    focus: crate::app::CompareFocus<'_>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let crate::app::CompareFocus {
        view,
        opened,
        cursor,
        raw,
        offset,
    } = focus;
    let width = usize::from(area.width.saturating_sub(3));
    let lines = compare_lines(view, opened, cursor, raw, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    let mut title = if raw {
        " Compare · prikk's own fields — v: stikk's rendering ".to_string()
    } else {
        " Compare — v: prikk's own fields ".to_string()
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

/// Every row, already wrapped to `width`, so the caller's scroll clamp is exact.
fn compare_lines<'a>(
    view: &CompareView,
    opened: &crate::app::CompareSource,
    cursor: usize,
    raw: bool,
    palette: &Palette,
    width: usize,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let say = |lines: &mut Vec<Line<'a>>, text: &str, indent: &str, style: Style| {
        for row in wrap_indented(text, width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    };

    let Some(compared) = view.listed() else {
        // Below the band: stikk's own voice about stikk's own limit (`C-T4d`).
        if let Some(reason) = view.unreported_reason() {
            say(&mut lines, &reason, TOP, Style::default().fg(palette.dim));
        }
        return lines;
    };

    // The heading names both sides. Core returns it as pieces, so a ref name renders under the
    // identifier rule and stikk's own words under the prose one (RFC 036).
    say(
        &mut lines,
        &compose(&compared.heading()),
        TOP,
        Style::default().fg(palette.accent),
    );
    // **Which side is which, in stikk's own words.** prikk's report names two points and says
    // nothing about why they were chosen; only the entry point knows (RFC 038 decision 3).
    say(
        &mut lines,
        opened.describe(),
        TOP,
        Style::default().fg(palette.dim),
    );
    lines.push(Line::from(""));

    if compared.is_empty() {
        // **stikk's words, and true only here** — prikk listed no entries and no unsupported path.
        say(
            &mut lines,
            "no differences",
            TOP,
            Style::default().fg(palette.ok),
        );
        return lines;
    }

    for (at, entry) in compared.entries.iter().enumerate() {
        entry_lines(&mut lines, entry, at == cursor, raw, palette, width);
    }

    if !compared.unsupported.is_empty() {
        lines.push(Line::from(""));
        say(
            &mut lines,
            "paths prikk could not compare — `commit` would refuse them:",
            TOP,
            Style::default().fg(palette.warn),
        );
        for path in &compared.unsupported {
            lines.push(Line::from(vec![
                Span::styled(ENTRY.to_string(), Style::default()),
                Span::styled(
                    crate::text::identifier(&path.path),
                    Style::default().fg(palette.fg),
                ),
            ]));
            // prikk's refusal, verbatim and as prose (`ER-02`).
            say(
                &mut lines,
                &inert(&path.refusal),
                DETAIL,
                Style::default().fg(palette.dim),
            );
        }
    }
    lines
}

/// One entry's section.
fn entry_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    entry: &ComparedEntry,
    selected: bool,
    raw: bool,
    palette: &Palette,
    width: usize,
) {
    let marker = if selected { "\u{25b8} " } else { "  " };
    let title_style = if selected {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    lines.push(Line::from(vec![
        Span::styled(marker.to_string(), Style::default().fg(palette.accent)),
        // Core composes the status word and the path(s) as pieces (RFC 036 decision 8).
        Span::styled(compose(&entry.title), title_style),
    ]));

    if raw && selected {
        // prikk's own fields for this entry, exactly as prikk gave them — verbatim, so the JSON's
        // own spacing survives.
        for row in wrap_verbatim(&inert(&entry.raw), width, DETAIL) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.dim),
            )));
        }
        lines.push(Line::from(""));
        return;
    }

    for fact in &entry.facts {
        lines.push(Line::from(vec![
            Span::styled(DETAIL.to_string(), Style::default()),
            Span::styled(compose(fact), Style::default().fg(palette.dim)),
        ]));
    }
    if let Some(note) = &entry.minimal_note {
        for row in wrap_indented(note, width, DETAIL) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.warn),
            )));
        }
    }
    for row in &entry.rows {
        hunk_row(lines, row, palette, width);
    }
    lines.push(Line::from(""));
}

/// One row of a hunk. **The marker is stikk's chrome and is coloured; the content beside it is
/// prikk's and is wrapped verbatim.**
fn hunk_row<'a>(lines: &mut Vec<Line<'a>>, row: &HunkRow, palette: &Palette, width: usize) {
    match row {
        // prikk's `@@` header and its no-newline note are prikk's words, not content, so they are
        // not quoted and they word-wrap like any other sentence (§7 trap 4).
        HunkRow::Header(text) => {
            for wrapped in wrap_indented(&inert(text), width, DETAIL) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.accent),
                )));
            }
        }
        HunkRow::Note(text) => {
            for wrapped in wrap_indented(&inert(text), width, DETAIL) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.dim),
                )));
            }
        }
        HunkRow::Withheld(text) => {
            for wrapped in wrap_indented(text, width, DETAIL) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.warn),
                )));
            }
        }
        HunkRow::Line { marker, content } => {
            let style = match marker {
                '-' => Style::default().fg(palette.warn),
                '+' => Style::default().fg(palette.accent),
                _ => Style::default().fg(palette.fg),
            };
            // The marker takes two columns of the indent so the content lines up across rows; a
            // continuation row keeps the full indent and no marker, which is what makes a wrapped
            // content row readable as a continuation rather than as context.
            let room = width.saturating_sub(DETAIL.chars().count() + 2).max(1);
            let inert_content = inert(content);
            let mut rows = wrap_verbatim(&inert_content, room, "").into_iter();
            let head = rows.next().unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(format!("{DETAIL}{marker} "), style),
                Span::styled(head, Style::default().fg(palette.fg)),
            ]));
            for rest in rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("{DETAIL}  "), Style::default()),
                    Span::styled(rest, Style::default().fg(palette.fg)),
                ]));
            }
        }
    }
}

#[cfg(test)]
mod tests;

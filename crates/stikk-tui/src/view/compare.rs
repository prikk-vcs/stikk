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
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{CompareView, ComparedEntry, HunkRow};

use crate::text::{compose, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;
use crate::title::{Position, Title};

/// Indents, by level: the heading and foot, an entry's title, and the rows under it.
const TOP: &str = "  ";
const ENTRY: &str = "  ";
const DETAIL: &str = "    ";
/// The heading's hanging indent: under the first side's name, past the opening margin.
const HEADING_HANG: &str = "      ";

/// Render Compare for `view` into `area`, scrolled to `offset` (clamped here and written back).
/// `cursor` selects an entry — which `v` then shows prikk's own fields for.
pub fn render(
    focus: stikk_app::CompareFocus<'_>,
    scrolled: &crate::overlay::Scroll,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let stikk_app::CompareFocus {
        view,
        opened,
        cursor,
        raw,
        // The surface names the offsets; the drawing looked them up and passed them in.
        surface: _,
    } = focus;
    let width = usize::from(area.width.saturating_sub(3));
    // The width the rows are drawn in, one wider than they wrap to (RFC 061 decision 4).
    let inner = usize::from(area.width.saturating_sub(2));
    let lines = compare_lines(view, opened, cursor, raw, palette, width, inner);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = scrolled.offset().min(max_offset);
    // **The program is told how far this goes** (RFC 067 decision 4): it holds no
    // offsets any more, so without the max it could not know the end was on screen.
    scrolled.note_max(max_offset);
    scrolled.set_offset(scroll);

    // One title builder, for every frame (RFC 060 review v1 C1). `↑/↓` is named only where something
    // is hidden, as it always has been: an affordance for a surface that does not scroll is noise.
    let position = Position::of(scroll, viewport, lines.len());
    // **The name says which it is showing**, and the key that toggles it is on the key line
    // (RFC 061 decision 5).
    let name = if raw {
        "Compare \u{00b7} prikk's own fields"
    } else {
        "Compare"
    };
    let title = Title::of(name).position(position).render(area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    // **No row may be wider than the area it is drawn in** (RFC 060 decision 2). This view
    // wraps its own rows to a width taken from `area`, and draws them through a `Paragraph`
    // with no `Wrap`: a row too wide would be cut, so it is recorded in tests instead.
    crate::text::record_overflow("Compare", &lines, usize::from(area.width.saturating_sub(2)));
    crate::text::record_cut(
        "Compare",
        lines.len(),
        usize::from(viewport),
        position.is_some(),
    );
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Every row, already wrapped to `width`, so the caller's scroll clamp is exact.
#[allow(clippy::too_many_arguments)] // one more: the width the rows are drawn in (RFC 061)
fn compare_lines<'a>(
    view: &CompareView,
    opened: &stikk_app::CompareSource,
    cursor: usize,
    raw: bool,
    palette: &Palette,
    width: usize,
    inner: usize,
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
    //
    // **A pair of refs gets its own heading** (RFC 041 decision 3): each ref with the block prikk
    // resolved it to, and which of the two the reader marked. The *names* are still core's
    // pieces; only the role words are added here, because only this layer knows which key chose
    // which side. `(marked)` and `(selected)` are stikk's own words, so they are `Said`.
    let heading = match opened {
        stikk_app::CompareSource::RefPair { .. } => {
            let mut pieces = compared.from.label_with_block();
            pieces.push(stikk_core::Piece::said("  (marked)"));
            pieces.push(stikk_core::Piece::said("  \u{2192}  "));
            pieces.extend(compared.to.label_with_block());
            pieces.push(stikk_core::Piece::said("  (selected)"));
            pieces
        }
        _ => compared.heading(),
    };
    // **Wrapped with a hanging indent**, because a two-ref heading is long: two names, two block
    // ids and two role words rarely fit 80 columns. Wrapped flat, the continuation starts at the
    // left margin and `(selected)` reads as a new field rather than as the tail of the one above
    // -- which is the column-0 fault RFC 037 Handoff C fixed in two other views, arriving here by
    // a different route (seen in this view's own capture).
    for row in crate::view::wrap_hanging(&compose(&heading), width, TOP, HEADING_HANG) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.accent),
        )));
    }
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
        entry_lines(&mut lines, entry, at == cursor, raw, palette, width, inner);
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
#[allow(clippy::too_many_arguments)] // one more: the width the rows are drawn in (RFC 061)
fn entry_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    entry: &ComparedEntry,
    selected: bool,
    raw: bool,
    palette: &Palette,
    width: usize,
    inner: usize,
) {
    let marker = crate::theme::marker(selected);
    let title_style = if selected {
        crate::theme::selected(palette)
    } else {
        Style::default().fg(palette.fg)
    };
    lines.push(Line::from(vec![
        Span::styled(marker.to_string(), Style::default().fg(palette.accent)),
        // Core composes the status word and the path(s) as pieces (RFC 036 decision 8).
        Span::styled(compose(&entry.title), title_style),
    ]));
    if selected {
        // The whole row, to the view's edge (RFC 061 decision 4).
        let row = lines.len() - 1;
        if let Some(rows) = lines.get_mut(row..) {
            crate::view::select_rows(rows, inner);
        }
    }

    if raw && selected {
        // **One line per row, each wrapped verbatim** — Patch detail's own raw path, for the same
        // reason: `Json::render` writes multi-line indented JSON, and prikk's nesting and the
        // whitespace inside its string values are part of what the raw view is *for*. Passing the
        // whole string through `inert` would turn every newline into the prose escape `\n` and
        // flatten the entry into one run of text.
        for row in &entry.raw {
            for wrapped in wrap_verbatim(&inert(row), width, DETAIL) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.dim),
                )));
            }
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

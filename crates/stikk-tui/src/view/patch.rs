//! The Patch-detail view (design `FR-030`, `TU-01`; RFC 035 §4.3).
//!
//! Renders the [`stikk_core::PatchDetailView`] the operation layer produces, in its order: the patch
//! id, prikk's `queued`/sealed word with the block it is in, the message, then each operation. **Every
//! word is core's**; this module decides nothing but layout.
//!
//! Every line is repository-adjacent text — paths, spans, file content, key ids, messages — so every
//! one goes through [`inert`] (`C-T2a`). **Long lines wrap, never clip** (RFC 024), and a tall patch
//! scrolls in the Queue view's idiom: the lines are wrapped here first, so their count is exact; the
//! offset is clamped to `rows − viewport` and written back; and the title says where you are.
//!
//! **Two wraps, and the difference matters** (review v1 §2.1). stikk's own words — labels, paths,
//! sentences — word-wrap through [`wrap_indented`]. **Repository content and prikk's raw JSON go
//! through [`wrap_verbatim`]**, which changes nothing: `wrap_indented` rebuilds a line from its
//! whitespace-separated words, so sending a span through it would render a re-indent, a tab turned into
//! spaces or a removed trailing newline as *no change at all*. Core says which rows are content
//! ([`OperationRow`]); this module honours it.
//!
//! **`v` toggles prikk's own fields** (RFC 035 decision 5). It is the same report either way — a
//! toggle, never a second read — and it is where an `unresolved_node_id`, a blob id and a mode as
//! prikk's own integer are legible rather than apologised for (F6).

use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{OperationRow, PatchDetailView};

use crate::text::{identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

/// Indents, by level: the patch id and foot, the heading rows, an operation's title, and its rows.
const TOP: &str = "  ";
const HEADING: &str = "    ";
const OPERATION: &str = "  ";
const DETAIL: &str = "      ";

/// Render Patch detail for `view` into `area`, scrolled to `offset` (clamped here and written back).
/// `raw` shows prikk's own fields for each operation instead of stikk's rendering of them.
pub fn render(
    view: &PatchDetailView,
    raw: bool,
    offset: &Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border, less a one-column gutter so wrapped text does not sit against the edge.
    let width = usize::from(area.width.saturating_sub(3));
    let lines = patch_lines(view, raw, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // `NFR-A03`: which view is showing, what switches it, and that there is more, are all said rather
    // than left to guess. **The `v` hint lives in the title, not in a row at the foot**: this view
    // scrolls, and a hint at the foot is a hint below the fold exactly when the patch is big enough to
    // need it (caught by this module's own capture test).
    let mut title = if raw {
        " Patch · prikk's own fields — v: stikk's rendering ".to_string()
    } else {
        " Patch — v: prikk's own fields ".to_string()
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

/// Push one **verbatim** row: repository content, or prikk's raw JSON. A function rather than a
/// closure, because the closure that pushes stikk's own words holds `lines` mutably.
///
/// `inert` still applies (`C-T2a`), and costs nothing here: core quotes content with the escapes
/// prikk's own JSON uses, so a control character is already spelled `\u001b` by the time it arrives.
fn push_verbatim<'a>(lines: &mut Vec<Line<'a>>, width: usize, text: &str, style: Style) {
    for row in wrap_verbatim(&inert(text), width, DETAIL) {
        lines.push(Line::from(Span::styled(row, style)));
    }
}

/// Every row of the view, already wrapped to `width`, so the caller's scroll clamp is exact.
fn patch_lines<'a>(
    view: &PatchDetailView,
    raw: bool,
    palette: &Palette,
    width: usize,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    // Functions rather than closures over `lines`, so the verbatim half can push to the same vector —
    // the Queue view's own renderer took this shape for the same reason.
    //
    // **Prose** (RFC 036 decision 2) for everything but the patch id. An operation's title and its
    // `Said` rows are composed by core from stikk's words and prikk's values together — `create-file
    // src/a.txt`, `node <id> — as at the end of this block` — and decision 4 sends a string of both
    // kinds through the rule that cannot drop a character. A *content* row is neither: it arrives
    // already quoted and escaped by `stikk-core`, and goes through `push_verbatim`.
    fn push<'a>(lines: &mut Vec<Line<'a>>, width: usize, text: &str, indent: &str, style: Style) {
        for row in wrap_indented(&inert(text), width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    }

    /// The patch id, the one bare identifier this view renders. Hex, so always plain.
    fn push_identifier<'a>(
        lines: &mut Vec<Line<'a>>,
        width: usize,
        value: &str,
        indent: &str,
        style: Style,
    ) {
        for row in wrap_indented(&identifier(value), width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    }

    push_identifier(
        &mut lines,
        width,
        &view.patch_id,
        TOP,
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    for row in &view.heading {
        push(
            &mut lines,
            width,
            row,
            HEADING,
            Style::default().fg(palette.fg),
        );
    }
    for operation in &view.operations {
        push(&mut lines, width, "", TOP, Style::default());
        push(
            &mut lines,
            width,
            &operation.title,
            OPERATION,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
        );
        // The same operation, two ways, out of the one report: stikk's rendering, or prikk's fields.
        if raw {
            // **prikk's JSON keeps its own nesting and its own whitespace** — the shape is part of
            // what the raw view is for (decision 5), and a string value's spaces are prikk's bytes.
            for row in &operation.raw {
                push_verbatim(&mut lines, width, row, Style::default().fg(palette.fg));
            }
        } else {
            for row in &operation.rows {
                match row {
                    OperationRow::Said(text) => {
                        push(
                            &mut lines,
                            width,
                            text,
                            DETAIL,
                            Style::default().fg(palette.fg),
                        );
                    }
                    // Byte-faithful: quoted by core, hard-split here, nothing collapsed.
                    OperationRow::Content(text) => {
                        push_verbatim(&mut lines, width, text, Style::default().fg(palette.fg));
                    }
                }
            }
        }
    }
    if let Some(foot) = &view.foot {
        push(&mut lines, width, "", TOP, Style::default());
        push(
            &mut lines,
            width,
            foot,
            TOP,
            Style::default()
                .fg(palette.dim)
                .add_modifier(Modifier::ITALIC),
        );
    }
    lines
}

#[cfg(test)]
mod tests;

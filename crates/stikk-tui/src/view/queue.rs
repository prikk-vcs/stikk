//! The Queue view (design `TU-01`'s Queue row, `FR-051`; RFC 028 Handoff A §6).
//!
//! Renders the `stikk_core::QueueView` the operation layer produces, in its order: the heading, the
//! thresholds, each patch (its id, its message line, its operations), and the one line at the foot. **Every
//! word is core's**; this module decides nothing but layout. Every line is repository-adjacent text — ref
//! names, paths, messages, key ids — so every one goes through [`inert`] (`C-T2a`).
//!
//! **Long lines wrap, never clip** (RFC 024): a message or a path wider than the view continues on the
//! next row at the same indent, so nothing prikk reported is cut off at the right edge.
//!
//! **And a tall queue scrolls** (RFC 028 A review v1 §2.1), in the Glossary's idiom: the lines are wrapped
//! here first, so their count is exact; the offset is clamped to `rows − viewport` and written back; and the
//! title says where you are whenever there is more than fits.
//!
//! **Since RFC 035 §4.2 it has a selection too**, over patches, and the scroll **follows** it: the offset
//! moves only as far as it must to keep the selected patch's first row on screen, so `↑`/`↓` walk patches
//! rather than rows and `Enter` opens the one that is marked. The clamp and the write-back are unchanged,
//! because only this function knows the wrapped height.

use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::QueueView;

use crate::text::{identifier, inert, wrap_indented};
use crate::theme::Palette;
use crate::title::{Position, Title};

/// Indents, by level: the heading and foot, a patch's id (selected or not), its message, and its
/// operations. The two id indents are the same width, so the ids line up whichever is marked.
const TOP: &str = "  ";
/// The patch id's own indent, which the marker goes inside — the glyph itself is `theme::marker`
/// (RFC 061 review v1 C1), so there is one marker in stikk and not six.
const ID_INDENT: &str = "  ";
const MESSAGE: &str = "    ";
const OPERATION: &str = "      ";

/// Render the Queue view for `view` into `area`, scrolled to `offset` (clamped here and written back)
/// and with `cursor` selecting one patch.
pub fn render(
    view: &QueueView,
    offset: &Cell<u16>,
    cursor: usize,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border, less a one-column gutter so wrapped text does not sit against the edge.
    let width = usize::from(area.width.saturating_sub(3));
    // **The width the rows are drawn in**, one column wider than they wrap to: the selected row's
    // highlight runs to the border (RFC 061 decision 4).
    let inner = usize::from(area.width.saturating_sub(2));
    let (lines, patch_rows) = queue_lines(view, cursor, palette, width, inner);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let mut scroll = offset.get().min(max_offset);
    // The scroll follows the cursor (§4.2), by the least it can: the selected patch's own rows are
    // brought on screen, and nothing else moves. `patch_rows` is where each patch's block of rows
    // starts and ends in the wrapped output, which only this function knows.
    if let Some((from, to)) = patch_rows.get(cursor).copied() {
        let from = u16::try_from(from).unwrap_or(u16::MAX);
        let to = u16::try_from(to).unwrap_or(u16::MAX);
        if from < scroll {
            scroll = from;
        } else if viewport > 0 && to >= scroll.saturating_add(viewport) {
            scroll = to.saturating_sub(viewport).saturating_add(1);
        }
        scroll = scroll.min(max_offset);
    }
    offset.set(scroll);

    // `NFR-A03`: "there is more" is said, never left to guess — only when there is somewhere to go.
    // One title builder, for every frame (RFC 060 review v1 C1).
    let position = Position::of(scroll, viewport, lines.len());
    let title = Title::of("Queue").position(position).render(area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    // **No row may be wider than the area it is drawn in** (RFC 060 decision 2). This view
    // wraps its own rows to a width taken from `area`, and draws them through a `Paragraph`
    // with no `Wrap`: a row too wide would be cut, so it is recorded in tests instead.
    crate::text::record_overflow("Queue", &lines, usize::from(area.width.saturating_sub(2)));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Every row of the view, already wrapped to `width`, so the scroll clamp is exact — and, beside it,
/// the first and last wrapped row of each patch, in the view's order, so the caller can scroll to the
/// selected one.
#[allow(clippy::too_many_arguments)] // one more: the width the rows are drawn in (RFC 061)
fn queue_lines<'a>(
    view: &QueueView,
    cursor: usize,
    palette: &Palette,
    width: usize,
    inner: usize,
) -> (Vec<Line<'a>>, Vec<(usize, usize)>) {
    let mut lines: Vec<Line> = Vec::new();
    let mut patch_rows: Vec<(usize, usize)> = Vec::new();
    // A function rather than a closure over `lines`: the loop below needs to read `lines.len()` to
    // record where each patch's rows begin, which a closure holding it mutably would forbid.
    //
    // **Prose** (RFC 036 decision 2). Everything this pushes is either stikk's own words or a string
    // core composed from both kinds — a heading naming a ref, an operation line naming a path — and
    // decision 4 sends those through the rule that cannot drop a character. The one bare identifier
    // here, a patch id, is pushed through `push_identifier` instead.
    fn push<'a>(lines: &mut Vec<Line<'a>>, width: usize, text: &str, indent: &str, style: Style) {
        for row in wrap_indented(&inert(text), width, indent) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    }

    /// The same, for a bare identifier: a patch id. Hex, so it is always plain and nothing changes
    /// visibly — but it goes through the rule that owns it, not the one next to it.
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

    // **The head, above everything** (RFC 063 decision 9): an interrupted seal changes what the
    // list below means — these patches are already in history, and Seal finishes the block rather
    // than making another. In `warn`, because it is a state to act on, and on its own rows so the
    // heading below it still reads as the heading.
    if let Some(head) = &view.head {
        push(
            &mut lines,
            width,
            head,
            TOP,
            Style::default().fg(palette.warn),
        );
        push(&mut lines, width, "", TOP, Style::default());
    }
    push(
        &mut lines,
        width,
        &view.heading,
        TOP,
        crate::theme::heading(palette.fg),
    );
    for threshold in &view.thresholds {
        push(
            &mut lines,
            width,
            threshold,
            TOP,
            Style::default().fg(palette.dim),
        );
    }
    for (at, patch) in view.patches.iter().enumerate() {
        push(&mut lines, width, "", TOP, Style::default());
        // The **first** patch's block starts at the top of the view, not at its own first row: the
        // heading and thresholds above it are this patch's context, and scrolling to row 4 to reveal
        // the first patch would push the heading off screen for no gain — the rows are already there.
        let from = if at == 0 { 0 } else { lines.len() };
        // The selected patch is marked the way History and Block detail mark theirs, and `Enter` opens
        // the one that carries the marker.
        let selected = at == cursor;
        // The marker goes in the **indent**, not in the text: `wrap_indented` word-wraps, so leading
        // spaces inside the text would be collapsed and the two kinds of row would not line up.
        let marker = format!("{ID_INDENT}{}", crate::theme::marker(selected));
        let id_row = lines.len();
        push_identifier(
            &mut lines,
            width,
            &patch.id,
            &marker,
            if selected {
                crate::theme::selected(palette)
            } else {
                Style::default().fg(palette.accent)
            },
        );
        if selected {
            // The whole row, to the view's edge (RFC 061 decision 4).
            if let Some(rows) = lines.get_mut(id_row..) {
                crate::view::select_rows(rows, inner);
            }
        }
        if let Some(message) = &patch.message {
            push(
                &mut lines,
                width,
                message,
                MESSAGE,
                Style::default().fg(palette.fg),
            );
        }
        for operation in &patch.operations {
            push(
                &mut lines,
                width,
                operation,
                OPERATION,
                Style::default().fg(palette.fg),
            );
        }
        patch_rows.push((from, lines.len().saturating_sub(1)));
    }
    if let Some(foot) = &view.foot {
        push(&mut lines, width, "", TOP, Style::default());
        push(
            &mut lines,
            width,
            foot,
            TOP,
            Style::default().fg(palette.dim),
        );
    }
    (lines, patch_rows)
}

#[cfg(test)]
mod tests;

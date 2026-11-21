//! The Trust view: which maintainer keys this repository trusts (RFC 054 decision 2), and
//! choosing them (RFC 056 decisions 2 and 3).
//!
//! # What this view is careful to say
//!
//! - **The title says which keys apply.** Where there is a list, it names `a` and `x`; where
//!   there is none — below the band, or after a refused read — it says *read only*, because
//!   neither key does anything there (`adoptable`). The view's own keys are the cursor, `a`,
//!   `x`, `r` and `Esc`; every write is previewed and confirmed by typing the key id.
//! - **What trust means is stikk's own sentence**, because prikk says it in the prose listing
//!   and not in the JSON stikk reads (RFC 054 F1). Saying it costs no second spawn, and leaving
//!   it out would let a reader take *trusted* for authority over their own branches.
//! - **No key material.** Ids and public keys, and nothing else — not a seed, not a seed path,
//!   not `key status`'s own `path` field (`NFR-S03`). `nothing_in_the_view_carries_a_path`
//!   asserts that against the rendered buffer, not against the model.
//! - **The empty state is an answer, with what to do.** prikk's own command as *text*; stikk
//!   runs nothing, and says where the public key should come from, which is the one thing a
//!   newcomer gets wrong.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{HOW_TO_ADOPT, NONE_ADOPTED, TrustView, WHAT_TRUST_MEANS, WHERE_TO_GET_THE_KEY};

use crate::text::{identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;
use crate::title::{Position, Title};

/// Indents: the heading rows, and the values under them.
const TOP: &str = "  ";
const UNDER: &str = "    ";

/// Render `view` into `area`, scrolled to `offset` (clamped here and written back).
pub fn render(
    view: &TrustView,
    cursor: usize,
    offset: &std::cell::Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let width = usize::from(area.width.saturating_sub(3));
    let (lines, cursor_row) = lines(view, cursor, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let mut scroll = offset.get().min(max_offset);
    // **The selected row is always drawn** (RFC 056 decision 3). `x` acts on it, so a cursor
    // scrolled out of sight would leave a reader confirming a key they cannot see -- and the
    // typed id is the only thing that would have told them which one it was.
    if let Some(row) = cursor_row {
        let row = u16::try_from(row).unwrap_or(u16::MAX);
        if row < scroll {
            scroll = row;
        } else if viewport > 0 && row >= scroll.saturating_add(viewport) {
            scroll = row.saturating_sub(viewport.saturating_sub(1));
        }
        scroll = scroll.min(max_offset);
    }
    offset.set(scroll);

    // One title builder, for every frame (RFC 060 review v1 C1). `a adopts` and `x removes` are key
    // hints, so they go with the group.
    let position = Position::of(scroll, viewport, lines.len());
    let scrolls: &[&str] = if position.is_some() {
        &["\u{2191}/\u{2193}"]
    } else {
        &[]
    };
    let title = if adoptable(view) {
        let mut keys = vec!["a adopts", "x removes"];
        keys.extend_from_slice(scrolls);
        Title::of("Trust").keys(&keys)
    } else {
        Title::of("Trust \u{00b7} read only").keys(scrolls)
    }
    .position(position)
    .render(area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    // **No row may be wider than the area it is drawn in** (RFC 060 decision 2). This view
    // wraps its own rows to a width taken from `area`, and draws them through a `Paragraph`
    // with no `Wrap`: a row too wide would be cut, so it is recorded in tests instead.
    crate::text::record_overflow("Trust", &lines, usize::from(area.width.saturating_sub(2)));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Whether this view has keys to act on -- the title says `a`/`x` only where it does.
fn adoptable(view: &TrustView) -> bool {
    matches!(view, TrustView::Listed { .. })
}

/// The rows, and the index of the one the cursor is on -- `None` where nothing is selectable.
fn lines<'a>(
    view: &TrustView,
    cursor: usize,
    palette: &Palette,
    width: usize,
) -> (Vec<Line<'a>>, Option<usize>) {
    match view {
        TrustView::Unavailable(message) | TrustView::Failed(message) => {
            (said(message, width, palette.warn), None)
        }
        TrustView::Listed {
            rows,
            own_key_unknown,
        } => {
            let mut lines = vec![Line::from(Span::styled(
                "  Keys this repository trusts",
                crate::theme::heading(palette.fg),
            ))];
            // **stikk's sentence, above the list** — what being on it means, and what it does
            // not mean. A reader who skips it is the reader this view is for.
            for row in wrap_indented(WHAT_TRUST_MEANS, width, TOP) {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default().fg(palette.dim),
                )));
            }
            lines.push(Line::from(""));

            if let Some(note) = own_key_unknown {
                for row in wrap_indented(note, width, TOP) {
                    lines.push(Line::from(Span::styled(
                        row,
                        Style::default().fg(palette.warn),
                    )));
                }
                lines.push(Line::from(""));
            }

            if rows.is_empty() {
                lines.extend(empty_state(palette, width));
                return (lines, None);
            }
            let mut cursor_row = None;
            for (at, key) in rows.iter().enumerate() {
                // **The marker History and the patch list already use**, so a selection looks
                // the same wherever stikk offers one.
                let selected = at == cursor.min(rows.len().saturating_sub(1));
                if selected {
                    cursor_row = Some(lines.len());
                }
                let mut spans = vec![
                    Span::styled(
                        if selected { "\u{25b8} " } else { TOP },
                        Style::default().fg(palette.accent),
                    ),
                    // The id. prikk's id is repository text.
                    Span::styled(identifier(&key.key_id), Style::default().fg(palette.accent)),
                ];
                if key.yours {
                    spans.push(Span::styled(
                        "  \u{2014} yours",
                        Style::default().fg(palette.fg),
                    ));
                }
                lines.push(Line::from(spans));
                // The public key, **hard-split** the way Block detail splits a 64-hex id: it is
                // one value, and a word wrap would break it on nothing.
                for row in wrap_verbatim(&identifier(&key.public_key), width, UNDER) {
                    lines.push(Line::from(Span::styled(
                        row,
                        Style::default().fg(palette.dim),
                    )));
                }
            }
            (lines, cursor_row)
        }
    }
}

/// What to show, and what to do, where nothing is adopted.
fn empty_state<'a>(palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for row in wrap_indented(NONE_ADOPTED, width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.fg),
        )));
    }
    lines.push(Line::from(""));
    // **prikk's command as text.** stikk does not run it, and a key that did would be a write
    // this view has no business offering.
    lines.push(Line::from(Span::styled(
        format!("{UNDER}{HOW_TO_ADOPT}"),
        Style::default().fg(palette.accent),
    )));
    lines.push(Line::from(""));
    for row in wrap_indented(WHERE_TO_GET_THE_KEY, width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.warn),
        )));
    }
    lines
}

/// One message, wrapped — a band's reason, or prikk's refusal, and nothing else.
fn said<'a>(message: &str, width: usize, colour: ratatui::style::Color) -> Vec<Line<'a>> {
    wrap_indented(&inert(message), width, TOP)
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(colour))))
        .collect()
}

#[cfg(test)]
mod tests;

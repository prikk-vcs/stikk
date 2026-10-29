//! The Trust view: which maintainer keys this repository trusts (RFC 054 decision 2).
//!
//! # What this view is careful to say
//!
//! - **It is read-only, and the title says so.** Nothing here adopts or removes a key; `Esc`
//!   is the only key that does anything.
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
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{HOW_TO_ADOPT, NONE_ADOPTED, TrustView, WHAT_TRUST_MEANS, WHERE_TO_GET_THE_KEY};

use crate::text::{identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

/// Indents: the heading rows, and the values under them.
const TOP: &str = "  ";
const UNDER: &str = "    ";

/// Render `view` into `area`, scrolled to `offset` (clamped here and written back).
pub fn render(
    view: &TrustView,
    offset: &std::cell::Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let width = usize::from(area.width.saturating_sub(3));
    let lines = lines(view, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    let mut title = " Trust \u{00b7} read only ".to_string();
    if max_offset > 0 {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        title.push_str(&format!(
            "\u{00b7} \u{2191}/\u{2193} \u{00b7} lines {first}\u{2013}{last} of {} ",
            lines.len()
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

fn lines<'a>(view: &TrustView, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    match view {
        TrustView::Unavailable(message) | TrustView::Failed(message) => {
            said(message, width, palette.warn)
        }
        TrustView::Listed {
            rows,
            own_key_unknown,
        } => {
            let mut lines = vec![Line::from(Span::styled(
                "  Keys this repository trusts",
                Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
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
                return lines;
            }
            for key in rows {
                // The id, with **yours** marked. prikk's id is repository text.
                let mut spans = vec![
                    Span::styled(TOP, Style::default()),
                    Span::styled(identifier(&key.key_id), Style::default().fg(palette.accent)),
                ];
                if key.yours {
                    spans.push(Span::styled(
                        "  \u{2014} yours",
                        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
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
            lines
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

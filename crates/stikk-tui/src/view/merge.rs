//! The Merge plan view: prikk's own plan for bringing one branch into another (`FR-080`/`FR-081`;
//! RFC 047 decision 4).
//!
//! # What this view is careful to say
//!
//! - **The plan is prikk's, word for word.** Exactly one line of it is read anywhere in stikk
//!   (`evidence outcome:`), and that read decides only whether a merge is offered. Everything
//!   drawn here is drawn verbatim, wrapped and never reflowed, and nothing is summarised,
//!   ranked or coloured by what it says.
//! - **The baseline is shown because the reader should be able to check it.** stikk chose it
//!   (decision 3), prikk did not, and a plan is only as true as the block it was computed from.
//! - **Nothing here acts.** The merge is RFC 047's Handoff B; until it lands, this view says
//!   nothing about a key that does nothing.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::MergePlanView;

use crate::text::{identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

/// Indents: the heading rows, and prikk's own lines under them.
const TOP: &str = "  ";
const PLAN: &str = "    ";
/// The heading's hanging indent, under the first name and past the opening margin.
const HEADING_HANG: &str = "      ";

/// Render the Merge plan for `view` into `area`, scrolled to `offset` (clamped here and written
/// back).
pub fn render(
    view: &MergePlanView,
    offset: &std::cell::Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let width = usize::from(area.width.saturating_sub(3));
    let lines = merge_lines(view, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // **"prikk's plan"**, not "the merge": nothing here merges, and a title that implied one
    // would be the view promising what the keys do not do.
    let mut title = " Merge plan \u{00b7} prikk's own ".to_string();
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

/// Every row, already wrapped to `width`, so the caller's scroll clamp is exact (RFC 023 F2).
fn merge_lines<'a>(view: &MergePlanView, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // The two names under the identifier rule and stikk's role words under the prose one (RFC
    // 036). `(marked)` and `(selected)` are the picker's own words, said here because only this
    // layer knows which key chose which side -- Compare's heading, for Compare's reason.
    let heading = format!(
        "{} (marked)  \u{2192}  {} (selected)",
        identifier(&view.from),
        identifier(&view.into)
    );
    for row in crate::view::wrap_hanging(&heading, width, TOP, HEADING_HANG) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }
    // **stikk's sentence, naming stikk's own choice.** The baseline is not prikk's answer to a
    // question stikk asked; it is stikk's answer, which prikk then computed the plan from.
    for row in wrap_indented(
        &format!(
            "from the baseline block {} \u{2014} the latest block both branches hold, which \
             stikk found and prikk was given",
            identifier(&view.baseline)
        ),
        width,
        TOP,
    ) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }

    lines.push(Line::from(""));
    for row in wrap_indented("prikk's plan, as prikk printed it:", width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    // **Verbatim, in order, and hard-wrapped** (`ER-02`, RFC 024): prikk's plan is laid out --
    // `label: value` rows and an indented report -- so re-flowing it on whitespace would lose the
    // shape prikk gave it. A long line wraps rather than clips, as every other verbatim block in
    // stikk does.
    for line in &view.lines {
        if line.trim().is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        for row in wrap_verbatim(&inert(line), width, PLAN) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.fg),
            )));
        }
    }
    lines
}

#[cfg(test)]
mod tests;

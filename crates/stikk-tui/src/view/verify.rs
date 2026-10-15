//! The Verify view: prikk's verdict on whether the repository is sound (`FR-100`; RFC 040).
//!
//! **Opened on demand only** (`V`, or the palette), because `verify` scans the whole repository.
//! `r` re-runs it, `v` shows prikk's own JSON.
//!
//! # What this view is careful to say
//!
//! - **The verdict is from this run.** A verdict is about the moment it was taken (RFC 040
//!   decision 4), so the heading says so and nothing is cached across runs.
//! - **stikk never says *prikk refused* about a damaged repository.** prikk exits 1 *with* a
//!   complete report when it finds damage (F2), and the seam reads that as the report it is. By
//!   the time anything reaches this view, the only question left is what prikk said.
//! - **The standing sentence is always here**, sound or damaged (decision 3). prikk's
//!   `item-failure` message ends *"see item outcomes above"*, and in JSON there is no "above":
//!   those outcomes live only in the prose report. Naming the terminal is what makes showing
//!   prikk's pointer honest instead of a dead end.
//! - **Nothing is acted on.** Verify repairs nothing and clears no lock (decision 4).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{JSON_OMITS, VerifyView};

use crate::text::{compose, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

const TOP: &str = "  ";
const ROW: &str = "    ";
const DETAIL: &str = "      ";

/// Render the Verify view into `area`.
pub fn render(
    view: &VerifyView,
    raw: bool,
    refreshing: bool,
    offset: &std::cell::Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let width = usize::from(area.width.saturating_sub(3));
    let lines = verify_lines(view, raw, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // **"from this run"**, never a time: stikk has no clock policy beyond its own session (RFC
    // 040 decision 4), and a wall-clock time would be a precision it has not earned.
    // **Every one of these fits 80 columns with the scroll indicator on the end** (RFC 042 review
    // v1 §2). They used to run past the border, where ratatui cuts them: a reader at 80 columns
    // saw the view's name and lost the keys. Each word below is stikk's own and was shortened to
    // pay for the indicator; **no word of prikk's and no name is ever shortened**.
    let mut title = if refreshing {
        " Verify · re-running — the previous verdict ".to_string()
    } else if raw {
        // The body labels the raw block itself, so the title does not repeat it.
        " Verify · this run — v: rendered · r: re-run ".to_string()
    } else {
        " Verify · this run — v: raw · r: re-run ".to_string()
    };
    if max_offset > 0 {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        title.push_str(&format!(
            "· \u{2191}/\u{2193} · lines {first}\u{2013}{last} of {} ",
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
fn verify_lines<'a>(
    view: &VerifyView,
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

    let Some(checked) = view.checked() else {
        // Below the band: stikk's own voice about stikk's own limit (`C-T4d`). **Never a
        // verdict** -- a repository nobody checked is not a repository found sound.
        if let Some(reason) = view.unreported_reason() {
            say(&mut lines, &reason, TOP, Style::default().fg(palette.dim));
        }
        return lines;
    };

    // **The verdict first**, in stikk's words: prikk has no single sentence for "sound".
    say(
        &mut lines,
        &checked.verdict(),
        TOP,
        Style::default()
            .fg(if checked.ok { palette.ok } else { palette.warn })
            .add_modifier(Modifier::BOLD),
    );

    // Then each failing condition, in prikk's own words (`ER-02`). The id is a name and the
    // message is prose, so `compose` renders each under its own rule (RFC 036).
    if !checked.conditions.is_empty() {
        lines.push(Line::from(""));
        for condition in &checked.conditions {
            // **Wrapped to its own gutter, not to the panel's width.** prikk's condition
            // messages are a sentence long and wrap at 80 columns; laying them out to the full
            // width and then pushing them right by the marker lost a word off the border, which
            // is how this was caught.
            let text = compose(&condition.row());
            let mut rows =
                crate::view::wrap_hanging(&text, width, &format!("{ROW}! "), DETAIL).into_iter();
            if let Some(head) = rows.next() {
                let (marker, rest) = head.split_at(ROW.len() + 2);
                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), Style::default().fg(palette.warn)),
                    Span::styled(rest.to_string(), Style::default().fg(palette.fg)),
                ]));
            }
            for rest in rows {
                lines.push(Line::from(Span::styled(
                    rest,
                    Style::default().fg(palette.fg),
                )));
            }
        }
    }

    // **Always, sound or damaged** (RFC 040 decision 3). It is a standing property of prikk's
    // JSON, not of this verdict, and a sentence that appeared only on failure would read as part
    // of the failure instead of as a limit of the report.
    lines.push(Line::from(""));
    say(
        &mut lines,
        JSON_OMITS,
        TOP,
        Style::default().fg(palette.dim),
    );

    // prikk's session coverage, where prikk reported any. Silence means prikk said nothing, never
    // zero sessions (`C-T2c'`).
    if let Some(sentence) = checked.sessions_sentence() {
        lines.push(Line::from(""));
        say(&mut lines, &sentence, TOP, Style::default().fg(palette.dim));
    }

    // The stages, in prikk's order. **Nothing counts or names them**: 0.39 reports 14 and 0.51
    // reports 17, both measured.
    lines.push(Line::from(""));
    say(
        &mut lines,
        &format!(
            "stages \u{2014} {} reported by prikk:",
            checked.stages.len()
        ),
        TOP,
        Style::default().fg(palette.dim),
    );
    for stage in &checked.stages {
        let text = compose(&stage.row());
        let style = if stage.is_evaluated() {
            Style::default().fg(palette.fg)
        } else {
            // Anything that is not prikk's plain `evaluated` is worth a reader's eye -- including
            // a word stikk has not measured, which is exactly the case it must not quietly
            // normalise.
            Style::default().fg(palette.warn)
        };
        // A `failed` stage carries prikk's whole message, so these wrap too.
        for row in crate::view::wrap_hanging(&text, width, ROW, DETAIL) {
            lines.push(Line::from(Span::styled(row, style)));
        }
    }

    if raw {
        // prikk's own report, bounded by core and laid out verbatim: its nesting and the
        // whitespace inside its strings are part of what a raw view is for.
        lines.push(Line::from(""));
        say(
            &mut lines,
            "prikk's own report:",
            TOP,
            Style::default().fg(palette.dim),
        );
        for row in &checked.raw {
            for wrapped in wrap_verbatim(row, width, ROW) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.fg),
                )));
            }
        }
    }

    lines
}

#[cfg(test)]
mod tests;

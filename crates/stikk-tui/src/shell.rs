//! The shell frame (design TU-02; handoff §2 `shell.rs`): header, active view, status bar, and the
//! overlay layer above them. Layout, and the one wrap only the renderer can do: the banner's own
//! height (RFC 023 F2 — only the renderer knows how many rows a sentence occupies).

use ratatui::Frame;
use ratatui::layout::{Constraint, HorizontalAlignment, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus, OrientationState};
use crate::text::{identifier, inert};
use crate::theme::Palette;
use crate::{overlay, status_bar, view};

/// Minimum usable terminal size (design TU-10).
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;

/// Render the whole shell for `app` into the frame.
pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();
    let palette = app.palette();

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(palette, frame, area);
        return;
    }

    // A banner appears between the body and the status bar only when there is one, and it is as
    // tall as its own text needs (RFC 024: wrap, never clip). **Wrapped once, here**, so the rows
    // reserved and the rows drawn are the same list and cannot drift — the fault RFC 024 found in
    // every overlay that guessed its height instead.
    let banner_lines = app
        .banner()
        .map(|text| banner_lines(palette, text, area.width as usize))
        .unwrap_or_default();
    // A banner is a notice, never the screen: the header, a row of body and the status bar keep
    // their places whatever a caller puts in it.
    let banner_height = u16::try_from(banner_lines.len())
        .unwrap_or(1)
        .min(area.height.saturating_sub(3));
    let [header, body, banner, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(banner_height),
        Constraint::Length(1),
    ])
    .areas(area);

    render_header(app, frame, header);
    if let Some(detail) = app.fault() {
        render_fault(palette, frame, body, detail);
    } else {
        render_body(app, frame, body);
    }
    if !banner_lines.is_empty() {
        frame.render_widget(Paragraph::new(banner_lines), banner);
    }
    status_bar::render(app, frame, status);

    if let Some(top) = app.top_overlay() {
        overlay::render(top, app.marked_ref(), palette, frame, area);
    }
}

/// The banner's mark, the indent its wrapped rows hang at, and the dismiss hint. **The first two
/// are the same three columns**, which is what lets a continuation line up under the text rather
/// than under the mark.
const BANNER_MARK: &str = " ! ";
/// See [`BANNER_MARK`].
const BANNER_HANG: &str = "   ";
/// See [`BANNER_MARK`].
const BANNER_HINT: &str = "   (Esc dismisses)";

/// A non-modal banner (design OP-03 banner/inline classes), **wrapped rather than clipped**.
///
/// It was one row, and a sentence longer than the terminal lost its tail — which is tolerable for
/// *"mark cleared"* and is not for a refusal that names the way out of the state it describes
/// (RFC 044 decision 7's is 118 characters and ends in the command to run). RFC 024's rule applies
/// here as everywhere else: wrap, never clip.
///
/// **A banner that fits keeps exactly the shape it always had** — mark, text and hint on one row —
/// so nothing that was already short moves. Only a banner that did not fit changes, and what
/// changes is that the reader can now read it.
///
/// prikk-sourced text is inert (`C-T2a`).
fn banner_lines<'a>(palette: &Palette, text: &str, width: usize) -> Vec<Line<'a>> {
    let text = inert(text);
    let mark = Style::default()
        .fg(palette.warn)
        .add_modifier(Modifier::BOLD);
    let body = Style::default().fg(palette.warn);
    let hint = Span::styled(BANNER_HINT, Style::default().fg(palette.dim));
    if BANNER_MARK.len() + text.chars().count() + BANNER_HINT.len() <= width {
        return vec![Line::from(vec![
            Span::styled(BANNER_MARK, mark),
            Span::styled(text, body),
            hint,
        ])];
    }
    let mut lines: Vec<Line<'a>> = view::wrap_hanging(&text, width, BANNER_MARK, BANNER_HANG)
        .into_iter()
        .map(|row| {
            // `BANNER_MARK` and `BANNER_HANG` are three ASCII columns each, so the split is a
            // byte split at the same place on every row.
            let (head, rest) = row.split_at(BANNER_MARK.len());
            Line::from(vec![
                Span::styled(head.to_string(), mark),
                Span::styled(rest.to_string(), body),
            ])
        })
        .collect();
    // On its own row once the text has taken more than one: beside the last word it would read as
    // part of the sentence.
    lines.push(Line::from(hint));
    lines
}

/// The stikk-internal fault screen (design ER-04): the repository was not touched; continue read-only.
fn render_fault(palette: &Palette, frame: &mut Frame, area: Rect, detail: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" stikk fault — your repository was not touched ")
        .style(Style::default().fg(palette.warn));
    let text = vec![
        Line::from(Span::styled(inert(detail), Style::default().fg(palette.fg))),
        Line::from(""),
        Line::from(Span::styled(
            "Only prikk writes repositories, and only on a confirmed mutation — none happened here.",
            Style::default().fg(palette.dim),
        )),
        Line::from(Span::styled(
            "Your session is preserved; you may continue read-only. Press Esc to dismiss.",
            Style::default().fg(palette.dim),
        )),
    ];
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let palette = app.palette();
    let repo = app
        .repo()
        .file_name()
        .map(|n| identifier(&n.to_string_lossy()))
        .unwrap_or_else(|| identifier(&app.repo().to_string_lossy()));
    let line = Line::from(vec![
        Span::styled(
            "stikk",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  —  ", Style::default().fg(palette.dim)),
        Span::styled(repo, Style::default().fg(palette.fg)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_body(app: &App, frame: &mut Frame, area: Rect) {
    let palette = app.palette();
    match app.focus() {
        Focus::Orientation(state) => match state {
            OrientationState::Loading => centered_note(palette, frame, area, "loading…", false),
            OrientationState::Failed(message) => render_failure(palette, frame, area, message),
            OrientationState::Loaded(v) => view::orientation::render(v, palette, frame, area),
        },
        Focus::Loading(what) => {
            centered_note(palette, frame, area, &format!("loading {what}…"), false)
        }
        Focus::History(v, cursor) => {
            // A mark made in another lineage is not this view's to draw (RFC 038 decision 3).
            let marked = app
                .marked_block()
                .filter(|m| m.reff == v.reff)
                .map(|m| m.block_id.as_str());
            view::history::render(v, cursor, marked, palette, frame, area);
        }
        Focus::BlockDetail(detail, cursor) => {
            view::block::render(detail, cursor, palette, frame, area);
        }
        Focus::Bundle(opened, offset) => {
            view::bundle::render(opened, offset, palette, frame, area);
        }
        Focus::Changes(v, hide_untracked, history) => {
            // RFC 032: the `renames` count is shown at prikk ≥ 0.38, where a rename can be declared.
            let renames_reported =
                matches!(app.state(), OrientationState::Loaded(o) if o.prikk_minor >= 38);
            // RFC 038: `d` is offered only where prikk can answer it.
            let compares = matches!(
                app.state(),
                OrientationState::Loaded(o) if o.prikk_minor >= stikk_core::COMPARE_FROM_MINOR
            );
            view::changes::render(
                v,
                history,
                hide_untracked,
                view::changes::Bands {
                    renames_reported,
                    compares,
                },
                palette,
                frame,
                area,
            );
        }
        Focus::Queue(v, offset, cursor) => {
            view::queue::render(v, offset, cursor, palette, frame, area);
        }
        Focus::PatchDetail(v, raw, offset) => {
            view::patch::render(v, raw, offset, palette, frame, area);
        }
        Focus::Compare(focus) => view::compare::render(focus, palette, frame, area),
        Focus::MergePlan(v, offset) => view::merge::render(v, offset, palette, frame, area),
        Focus::Verify(v, raw, refreshing, offset) => {
            view::verify::render(v, raw, refreshing, offset, palette, frame, area);
        }
    }
}

fn render_failure(palette: &Palette, frame: &mut Frame, area: Rect, message: &str) {
    // prikk's verbatim message is preserved (design NFR-I03); the full refusal overlay with
    // next-steps is the next increment.
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Cannot open repository ")
        .style(Style::default().fg(palette.warn));
    let text = vec![
        Line::from(Span::styled(
            inert(message),
            Style::default().fg(palette.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "press r to retry, q to quit",
            Style::default().fg(palette.dim),
        )),
    ];
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn centered_note(palette: &Palette, frame: &mut Frame, area: Rect, note: &str, warn: bool) {
    let style = if warn {
        Style::default().fg(palette.warn)
    } else {
        Style::default().fg(palette.dim)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(note.to_string(), style)))
            .alignment(HorizontalAlignment::Center),
        area,
    );
}

fn render_too_small(palette: &Palette, frame: &mut Frame, area: Rect) {
    let msg = format!(
        "terminal too small — need at least {MIN_WIDTH}×{MIN_HEIGHT}, have {}×{}",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(palette.warn),
        )))
        .alignment(HorizontalAlignment::Center)
        .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests;

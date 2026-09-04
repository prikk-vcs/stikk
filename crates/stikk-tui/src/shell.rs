//! The shell frame (design TU-02; handoff §2 `shell.rs`): header, active view, status bar, and the
//! overlay layer above them. Pure layout — it draws whatever the app hands it and computes nothing.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus, OrientationState};
use crate::text::inert;
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

    // A banner row appears between the body and the status bar only when there is a banner.
    let banner_height = u16::from(app.banner().is_some());
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
    if let Some(text) = app.banner() {
        render_banner(palette, frame, banner, text);
    }
    status_bar::render(app, frame, status);

    if let Some(top) = app.top_overlay() {
        overlay::render(top, palette, frame, area);
    }
}

/// A one-line non-modal banner (design OP-03 banner/inline classes). prikk-sourced text is inert.
fn render_banner(palette: &Palette, frame: &mut Frame, area: Rect, text: &str) {
    let line = Line::from(vec![
        Span::styled(
            " ! ",
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(inert(text), Style::default().fg(palette.warn)),
        Span::styled("   (Esc dismisses)", Style::default().fg(palette.dim)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
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
        .map(|n| inert(&n.to_string_lossy()))
        .unwrap_or_else(|| inert(&app.repo().to_string_lossy()));
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
        Focus::History(v, cursor) => view::history::render(v, cursor, palette, frame, area),
        Focus::BlockDetail(detail) => view::block::render(detail, palette, frame, area),
        Focus::Changes(v, hide_untracked) => {
            view::changes::render(v, hide_untracked, palette, frame, area);
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
            .alignment(Alignment::Center),
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
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests;

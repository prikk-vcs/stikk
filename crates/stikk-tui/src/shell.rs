//! The shell frame (design TU-02; handoff §2 `shell.rs`): header, active view, status bar, and the
//! overlay layer above them. Layout, and the one wrap only the renderer can do: the banner's own
//! height (RFC 023 F2 — only the renderer knows how many rows a sentence occupies).

use ratatui::Frame;
use ratatui::layout::{Constraint, HorizontalAlignment, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus, OrientationState};
use crate::text::{identifier, inert};
use crate::theme::{Palette, Resolved};
use crate::viewports::Viewports;
use crate::{overlay, status_bar, view};

/// Minimum usable terminal size (design TU-10, as RFC 060 decision 1 amends it).
///
/// **60 × 15, lowered from 80 × 24**, which is what lets the text be made about 2.3 times bigger on
/// every screen RFC 060 F1 measured — a 1366 × 768 laptop was locked out by one row at 1.6 times.
/// Rows run out long before columns: a terminal cell is about 2.3 times taller than it is wide, and
/// screens are wider than they are tall.
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 15;

/// Whether the too-small screen is owed for this size.
///
/// (RFC 060's own tests used to draw through a `cfg(test)` seam here, because the sizes they check
/// were below the minimum. The minimum is 60 × 15 now, so they draw at it like anything else.)
fn too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

/// Render the whole shell for `app`, in the colours `look` settled on, into the frame.
///
/// **The colours arrive as a parameter** (RFC 067 decision 3). `App` used to hold them, which made
/// the program's state depend on what one terminal could display; what a screen can show is the
/// front end's business, and the run loop owns the [`Resolved`] it detected at startup.
pub fn render(app: &App, look: &Resolved, viewports: &mut Viewports, frame: &mut Frame) {
    let area = frame.area();
    render_frame(app, look, viewports, frame, area);
    // **Last, so it is under everything** — see [`paint_the_themes_own_pair`].
    if let Some(bg) = look.palette.bg {
        paint_the_themes_own_pair(frame, area, look.palette.fg, bg);
    }
    // **Then forget the surfaces nobody can return to** (review v1 F1). Here, at the end of the one
    // function every drawing path goes through, so the tests prune exactly as the run loop does.
    viewports.retain(&app.live_surfaces());
}

/// **The theme's own pair, over the whole frame** (RFC 059 decision 8). `high-contrast` is the only
/// theme that paints one; the other four leave the terminal's own background alone.
///
/// **Applied after everything else is drawn, to every cell nobody gave a colour**, which is what
/// *"under everything, overlays included"* has to mean in practice: an overlay clears its region
/// before it draws, and `Clear` resets a cell to the terminal's own pair — so a background painted
/// before the overlays would be punched out under every one of them. Painting what is still unpainted
/// is the same result from one place, and it cannot be forgotten by a renderer that clears.
///
/// **The pair, not half of it.** A span drawn with no explicit colour (`Span::raw`) would otherwise
/// keep the terminal's own text colour over stikk's painted background, which on the wrong terminal is
/// the unreadable combination this RFC exists to remove. With `high-contrast` stikk owes the reader
/// both halves.
fn paint_the_themes_own_pair(frame: &mut Frame, area: Rect, fg: Color, bg: Color) {
    let buffer = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                if cell.bg == Color::Reset {
                    cell.set_bg(bg);
                }
                if cell.fg == Color::Reset {
                    cell.set_fg(fg);
                }
            }
        }
    }
}

/// The frame itself: header, body, banner, key line, status bar, and the overlay layer above them.
fn render_frame(
    app: &App,
    look: &Resolved,
    viewports: &mut Viewports,
    frame: &mut Frame,
    area: Rect,
) {
    let palette = &look.palette;

    if too_small(area) {
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
    // The header, a row of body, the key line and the status bar keep their places whatever a
    // caller puts in the banner.
    let banner_height = u16::try_from(banner_lines.len())
        .unwrap_or(1)
        .min(area.height.saturating_sub(4));
    let [header, body, banner, keys, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(banner_height),
        // **The key line** (RFC 061 decision 5): one row, above the status bar, at every size from
        // the minimum.
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    render_header(app, palette, frame, header);
    if let Some(detail) = app.fault() {
        render_fault(palette, frame, body, detail);
    } else {
        render_body(app, look, viewports, frame, body);
    }
    if !banner_lines.is_empty() {
        frame.render_widget(Paragraph::new(banner_lines), banner);
    }
    frame.render_widget(
        Paragraph::new(crate::key_line::line(
            &app.keys_here(),
            palette,
            usize::from(keys.width),
        )),
        keys,
    );
    status_bar::render(app, palette, frame, status);

    // **The card and its surface together** (RFC 067 decision 4): a card on top is the current
    // surface, so its offsets are the ones the program's scroll decisions were made against.
    if let Some((top, surface)) = app.top_card() {
        overlay::render(
            top,
            app.marked_ref(),
            palette,
            viewports.of(surface),
            frame,
            area,
        );
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
    let mark = Style::default().fg(palette.warn);
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
        .title(
            crate::title::Title::of("stikk fault \u{2014} your repository was not touched")
                .render(area.width),
        )
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

fn render_header(app: &App, palette: &Palette, frame: &mut Frame, area: Rect) {
    let repo = app
        .repo()
        .file_name()
        .map(|n| identifier(&n.to_string_lossy()))
        .unwrap_or_else(|| identifier(&app.repo().to_string_lossy()));
    let line = Line::from(vec![
        Span::styled("stikk", crate::theme::heading(palette.accent)),
        Span::styled("  —  ", Style::default().fg(palette.dim)),
        Span::styled(repo, Style::default().fg(palette.fg)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_body(
    app: &App,
    look: &Resolved,
    viewports: &mut Viewports,
    frame: &mut Frame,
    area: Rect,
) {
    let palette = &look.palette;
    match app.focus() {
        Focus::Orientation(state) => match state {
            OrientationState::Loading => centered_note(palette, frame, area, "loading…", false),
            OrientationState::Failed(message) => render_failure(palette, frame, area, message),
            OrientationState::Loaded(v) => {
                // **Composed here, from the same `Resolved`** (RFC 067 decision 3 / §3.2): the
                // `colours` row is a fact about this session's terminal, so the drawing says it.
                view::orientation::render(v, &look.colours_row(), palette, frame, area);
            }
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
        Focus::BlockDetail(detail, cursor, surface) => {
            view::block::render(detail, cursor, viewports.of(surface), palette, frame, area);
        }
        Focus::Bundle(opened, surface) => {
            view::bundle::render(opened, viewports.of(surface), palette, frame, area);
        }
        Focus::Trust(trust, cursor, surface) => {
            view::trust::render(trust, cursor, viewports.of(surface), palette, frame, area);
        }
        Focus::Changes(v, hide_untracked, history, surface) => {
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
                viewports.of(surface),
                palette,
                frame,
                area,
            );
        }
        Focus::Queue(v, surface, cursor) => {
            view::queue::render(v, viewports.of(surface), cursor, palette, frame, area);
        }
        Focus::PatchDetail(v, raw, surface) => {
            view::patch::render(v, raw, viewports.of(surface), palette, frame, area);
        }
        Focus::Compare(focus) => {
            view::compare::render(focus, viewports.of(focus.surface), palette, frame, area);
        }
        Focus::MergePlan(v, surface) => {
            view::merge::render(v, viewports.of(surface), palette, frame, area);
        }
        Focus::Verify(v, raw, refreshing, surface) => {
            view::verify::render(
                v,
                raw,
                refreshing,
                viewports.of(surface),
                palette,
                frame,
                area,
            );
        }
    }
}

fn render_failure(palette: &Palette, frame: &mut Frame, area: Rect, message: &str) {
    // prikk's verbatim message is preserved (design NFR-I03); the full refusal overlay with
    // next-steps is the next increment.
    let block = Block::default()
        .borders(Borders::ALL)
        .title(crate::title::Title::of("Cannot open repository").render(area.width))
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

/// **The screen that says what to do** (RFC 060 decision 6), in three sentences, most important
/// first.
///
/// - **The window comes first**, because a reader who made the text bigger did it on purpose, and
///   telling them to undo it is telling them to give up what they came for.
/// - **The keys are "often"**, because they differ by terminal — kitty's are Ctrl, Shift and −, and
///   ⌘ and − is Apple's own documented *"Make fonts smaller"*.
/// - **The guide is named by its address.** *"The guide"* alone tells a reader nothing about where it
///   is.
///
/// The numbers come from [`MIN_WIDTH`] and [`MIN_HEIGHT`], never written twice: a sentence that
/// states a constant is a sentence that will one day state the wrong one.
fn too_small_sentences(area: Rect) -> [String; 3] {
    [
        format!(
            "This window is too small for stikk: it needs {MIN_WIDTH} columns and {MIN_HEIGHT} \
             rows, and has {} \u{d7} {}.",
            area.width, area.height
        ),
        "Make the window bigger, or the text a little smaller (often Ctrl and \u{2212}; on a Mac, \
         \u{2318} and \u{2212})."
            .to_string(),
        "Help: https://prikk-vcs.github.io/stikk/ \u{2192} \"Making the text bigger\".".to_string(),
    ]
}

/// Render [`too_small_sentences`] into `area`: wrapped to the window, **in the terminal's own
/// colours** (`Color::Reset`), and **dropping sentences from the last until what is left fits**.
///
/// It must itself fit down to 20 × 5 (RFC 060 decision 6), and below that it draws as much of the
/// first sentence as there is room for — which is still the sentence that names the size needed.
/// Left-aligned from the top row, because three sentences read as a paragraph and a centred
/// paragraph in a tiny window reads as a shrug.
fn render_too_small(_palette: &Palette, frame: &mut Frame, area: Rect) {
    let width = usize::from(area.width).max(1);
    let height = usize::from(area.height);
    let sentences = too_small_sentences(area);
    // Wrap each on its own, so dropping one cannot change where another breaks.
    let wrapped: Vec<Vec<String>> = sentences
        .iter()
        .map(|sentence| crate::text::wrap_indented(sentence, width, ""))
        .collect();
    let mut rows: Vec<String> = Vec::new();
    for sentence in &wrapped {
        if rows.len() + sentence.len() > height {
            break;
        }
        rows.extend(sentence.iter().cloned());
    }
    if rows.is_empty() {
        // Not even the first sentence fits: as much of it as there is room for, still wrapped.
        rows = wrapped
            .first()
            .map(|sentence| sentence.iter().take(height).cloned().collect())
            .unwrap_or_default();
    }
    let lines: Vec<Line> = rows
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default())))
        .collect();
    crate::text::record_overflow("terminal too small", &lines, width);
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests;

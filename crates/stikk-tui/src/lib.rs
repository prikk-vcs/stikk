//! The terminal (TUI) frontend for stikk (design `stikk-04` MOD-06; RFC 001; handoff v1).
//!
//! The TUI is thin (design AR-03/FE-01): it translates key events into `stikk-core` operations and
//! renders the view-models `stikk-core` returns, computing nothing about the repository itself. It is
//! built on `ratatui` over the `crossterm` backend (RFC 001), used through `ratatui`'s own re-export
//! so the versions cannot drift.
//!
//! This increment ships the shell (header, active view, status bar, overlay layer) and three views —
//! Orientation (design VW-01, FR-002), History (VW-03, FR-010/011), and Block detail (FR-031/032 at
//! block granularity, RFC 006) — plus a ref picker. [`run`] is the entry point the launcher calls when
//! stdout is a TTY; the scripted-backend examples (`examples/orientation_demo.rs`,
//! `examples/history_demo.rs`) drive the same shell with no prikk binary and no repository.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod app;
pub mod overlay;
pub mod text;

mod keys;
mod shell;
mod status_bar;
mod terminal;
mod theme;
mod view;

#[cfg(test)]
pub(crate) mod test_util;

pub use app::{App, Focus, OrientationState, Screen};
pub use overlay::Overlay;
pub use terminal::stdout_is_tty;
pub use theme::Palette;

use std::path::Path;
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use stikk_model::{Result, StikkError};
use stikk_prikk::Prikk;
use stikk_state::Config;

use crate::keys::Action;
use crate::terminal::TerminalGuard;

/// Poll interval for input; also the cap on how long a frame waits before it can redraw (design
/// NFR-P01 — input stays responsive).
const POLL: Duration = Duration::from_millis(250);

/// Run the interactive TUI over `repo`, driving prikk through the seam and rendering with the palette
/// from `config`. The caller must have confirmed stdout is a TTY ([`stdout_is_tty`]); the terminal is
/// restored on every exit path including panic (handoff §6).
///
/// # Errors
/// [`StikkError::Environment`] if the terminal cannot be set up or driven. Repository-level failures
/// (a refusal, a lock conflict) do **not** end the run — they render as a failure state the user can
/// retry, since stikk owns no repository authority and a bad read is not a crash.
pub fn run(repo: &Path, prikk: &impl Prikk, config: &Config) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))
        .map_err(|e| StikkError::environment("could not initialize the terminal", e))?;
    let mut app = App::open(repo, prikk, config);

    while !app.should_quit() {
        terminal
            .draw(|frame| shell::render(&app, frame))
            .map_err(|e| StikkError::environment("failed to draw the terminal", e))?;

        if event::poll(POLL).map_err(|e| StikkError::environment("input poll failed", e))? {
            let ev = event::read().map_err(|e| StikkError::environment("input read failed", e))?;
            // Only key *presses* — on Windows, crossterm also emits release events.
            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Press {
                    match keys::dispatch(key, app.wants_text_input()) {
                        Action::Quit => app.quit(),
                        Action::Back => app.back(),
                        Action::Select => app.select(prikk),
                        Action::Up => app.nav_up(),
                        Action::Down => app.nav_down(),
                        Action::OpenRefPicker => app.open_ref_picker(prikk),
                        Action::OpenChanges => app.open_changes(prikk),
                        Action::ToggleUntracked => app.toggle_untracked(),
                        Action::OpenGlossary => app.open_glossary(),
                        Action::OpenPalette => app.open_palette(),
                        Action::OpenRefusals => app.open_refusals(),
                        Action::Refresh => app.reload(prikk),
                        Action::Input(c) => app.input_char(c),
                        Action::Backspace => app.backspace(),
                        Action::None => {}
                    }
                }
            }
        }
    }
    Ok(())
}

//! The terminal (TUI) frontend for stikk (design `stikk-04` MOD-06; RFC 001; handoff v1; RFC 010).
//!
//! The TUI is thin (design AR-03/FE-01): it translates key events into `stikk-core` operations and
//! renders the view-models `stikk-core` returns, computing nothing about the repository itself. It is
//! built on `ratatui` over the `crossterm` backend (RFC 001), used through `ratatui`'s own re-export
//! so the versions cannot drift.
//!
//! **Every seam-driving read runs on a worker thread (RFC 010).** [`run`] spawns one via
//! `std::thread::scope` — which lets it *borrow* `prikk` and `repo` rather than requiring `Arc` or a
//! `'static` bound, so this function's signature is unchanged — and drives the render/input loop on
//! the calling thread throughout, applying results as they arrive (`worker::Response`) without ever
//! blocking on them. `stikk-core` itself stays synchronous (RFC 010 decision 3); the worker is simply
//! the thread that calls it.
//!
//! This increment ships the shell (header, active view, status bar, overlay layer), the Orientation,
//! History, Block detail, and Changes views, a ref picker, the explanation surface, the command
//! palette, and the Background Operations overlay. [`run`] is the entry point the launcher calls when
//! stdout is a TTY; the scripted-backend examples (`examples/*.rs`) drive the same shell with no prikk
//! binary and no repository.

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
mod worker;

#[cfg(test)]
pub(crate) mod test_util;

pub use app::{App, Focus, Operation, OperationStatus, OrientationState, Screen};
pub use overlay::Overlay;
pub use terminal::stdout_is_tty;
pub use theme::Palette;

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use stikk_model::{Result, StikkError};
use stikk_prikk::Prikk;
use stikk_state::Config;

use crate::keys::Action;
use crate::terminal::TerminalGuard;

/// Poll interval for input; also the cap on how long a frame waits before it can redraw and pick up a
/// worker response (design NFR-P01 — input stays responsive). Lowered from 250ms (RFC 001) to 50ms:
/// with the seam now off-thread (RFC 010), this bounds only how long a *response* waits to be applied,
/// and 250ms of visible latency after a fast read would be a stutter that did not exist before.
const POLL: Duration = Duration::from_millis(50);

/// Run the interactive TUI over `repo`, driving prikk through the seam on a worker thread and
/// rendering with the palette from `config`. The caller must have confirmed stdout is a TTY
/// ([`stdout_is_tty`]); the terminal is restored on every exit path including panic (handoff §6).
///
/// # Errors
/// [`StikkError::Environment`] if the terminal cannot be set up or driven. Repository-level failures
/// (a refusal, a lock conflict) do **not** end the run — they render as a failure state the user can
/// retry, since stikk owns no repository authority and a bad read is not a crash.
pub fn run(repo: &Path, prikk: &impl Prikk, config: &Config) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))
        .map_err(|e| StikkError::environment("could not initialize the terminal", e))?;

    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();

    std::thread::scope(|scope| {
        // Caught here, not propagated: a worker panic must surface as a fault screen (ER-04), not a
        // crash of the whole process. Catching it turns the panic into a normal (non-panicking) thread
        // exit from `thread::scope`'s point of view; either way `res_tx` is dropped when this closure
        // returns, which `ui_loop` observes as a disconnected channel (RFC 010 §7).
        scope.spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                worker::run(prikk, repo, req_rx, res_tx);
            }));
        });
        ui_loop(&mut terminal, repo, config, req_tx, &res_rx)
    })
}

/// The render/input/apply loop, run on the calling thread while the worker (spawned by [`run`]) answers
/// requests on its own thread. Never blocks on the response channel (`res_rx.try_recv()` only).
fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repo: &Path,
    config: &Config,
    req_tx: mpsc::Sender<worker::Request>,
    res_rx: &mpsc::Receiver<worker::Response>,
) -> Result<()> {
    let mut app = App::open(repo, config, req_tx);
    // Tracked locally, not on `App`: once the worker is gone there is nothing left to poll, and this
    // must flip at most once so the fault it records stays dismissible (§ `App::worker_stopped`).
    let mut worker_alive = true;

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
                        Action::Select => app.select(),
                        Action::Up => app.nav_up(),
                        Action::Down => app.nav_down(),
                        Action::OpenRefPicker => app.open_ref_picker(),
                        Action::OpenChanges => app.open_changes(),
                        Action::ToggleUntracked => app.toggle_untracked(),
                        Action::OpenGlossary => app.open_glossary(),
                        Action::OpenPalette => app.open_palette(),
                        Action::OpenRefusals => app.open_refusals(),
                        Action::OpenOperations => app.open_operations(),
                        Action::Refresh => app.reload(),
                        Action::Input(c) => app.input_char(c),
                        Action::Backspace => app.backspace(),
                        Action::None => {}
                    }
                }
            }
        }

        if worker_alive {
            loop {
                match res_rx.try_recv() {
                    Ok(response) => app.apply(response),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        app.worker_stopped();
                        worker_alive = false;
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

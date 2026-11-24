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

mod key_line;
mod keys;
mod shell;
mod status_bar;
mod terminal;
mod theme;
mod title;
mod view;
mod worker;

#[cfg(test)]
mod golden;
#[cfg(test)]
mod readme_visuals;
#[cfg(test)]
pub(crate) mod test_util;

pub use app::{App, Effect, Focus, Operation, OperationStatus, OrientationState, RefFocus, Screen};
pub use keys::{Intent, handle_key};
pub use overlay::Overlay;
pub use terminal::stdout_is_tty;
// `SchemeKind` is re-exported because [`Palette::kind`] is a public field: a field whose type a
// caller cannot name is not really public (RFC 059).
pub use theme::{Palette, SchemeKind};

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use stikk_model::{Result, StikkError};
use stikk_prikk::Prikk;
use stikk_state::Config;

use crate::terminal::TerminalGuard;

/// Poll interval for input; also the cap on how long a frame waits before it can redraw and pick up a
/// worker response (design NFR-P01 — input stays responsive). Lowered from 250ms (RFC 001) to 50ms:
/// with the seam now off-thread (RFC 010), this bounds only how long a *response* waits to be applied,
/// and 250ms of visible latency after a fast read would be a stutter that did not exist before.
const POLL: Duration = Duration::from_millis(50);

/// Where stikk may remember this repository between launches (RFC 046).
///
/// **The caller decides whether there is one at all.** `None` to [`run`] means no record is read
/// and none is written: `STIKK_PRIVATE=1` (decision 6), and a host with no resolvable state
/// directory, reach the TUI the same way — as a session that simply does not remember.
#[derive(Debug, Clone)]
pub struct Session {
    /// Where `sessions/` lives — [`stikk_state::paths::state_dir`]'s answer.
    pub state_dir: std::path::PathBuf,
    /// The repository's **canonical** root, which names the record and is stored inside it.
    pub canonical_root: std::path::PathBuf,
}

/// Run the interactive TUI over `repo`, driving prikk through the seam on a worker thread and
/// rendering with the palette resolved from `config` and the terminal itself (RFC 059). The caller
/// must have confirmed stdout is a TTY ([`stdout_is_tty`]); the terminal is restored on every exit
/// path including panic (handoff §6).
///
/// `session` is where this repository is remembered, or `None` for a launch that remembers
/// nothing (RFC 046).
///
/// # Errors
/// [`StikkError::Environment`] if the terminal cannot be set up or driven. Repository-level failures
/// (a refusal, a lock conflict) do **not** end the run — they render as a failure state the user can
/// retry, since stikk owns no repository authority and a bad read is not a crash.
pub fn run(
    repo: &Path,
    prikk: &impl Prikk,
    config: &Config,
    session: Option<Session>,
) -> Result<()> {
    // **The colours are resolved first, and the terminal is asked here or nowhere** (RFC 059
    // decision 5; Handoff A trap 1): the query goes out before the alternate screen and raw mode,
    // never from inside the UI loop, and never twice. Every environment read is here, so
    // `theme::resolve` itself reads nothing and its bands are plain unit tests (trap 2).
    let mut environment = theme::Environment {
        theme: config.theme,
        no_color: std::env::var("NO_COLOR").ok(),
        colorterm: std::env::var("COLORTERM").ok(),
        wt_session: std::env::var_os("WT_SESSION").is_some(),
        colorfgbg: std::env::var("COLORFGBG").ok(),
        windows: cfg!(windows),
        answer: None,
    };
    // Asked of the resolver, not decided here a second time: `dark`, `light` and `plain` send no
    // query, and neither does `NO_COLOR` with `system`.
    if theme::resolve(&environment).wants_an_answer {
        environment.answer = theme::detect();
    }
    let resolved = theme::resolve(&environment);

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
        ui_loop(&mut terminal, repo, resolved, session, req_tx, &res_rx)
    })
}

/// The render/input/apply loop, run on the calling thread while the worker (spawned by [`run`]) answers
/// requests on its own thread. Never blocks on the response channel (`res_rx.try_recv()` only).
fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    repo: &Path,
    resolved: theme::Resolved,
    session: Option<Session>,
    req_tx: mpsc::Sender<worker::Request>,
    res_rx: &mpsc::Receiver<worker::Response>,
) -> Result<()> {
    let mut app = App::open(repo, resolved, session, req_tx);
    // Tracked locally, not on `App`: once the worker is gone there is nothing left to poll, and this
    // must flip at most once so the fault it records stays dismissible (§ `App::worker_stopped`).
    let mut worker_alive = true;

    while !app.should_quit() {
        terminal
            .draw(|frame| shell::render(&app, frame))
            .map_err(|e| StikkError::environment("failed to draw the terminal", e))?;

        if event::poll(POLL).map_err(|e| StikkError::environment("input poll failed", e))? {
            let ev = event::read().map_err(|e| StikkError::environment("input read failed", e))?;
            // RFC 031 §5: focus returning checks for an outside change at once. `FocusLost` is ignored.
            if let Event::FocusGained = ev {
                app.focus_gained(Instant::now());
            }
            // Only key *presses* — on Windows, crossterm also emits release events.
            if let Event::Key(key) = ev
                && key.kind == KeyEventKind::Press
            {
                // **One function turns a key into what it does** (RFC 060 Handoff B §2, RFC 067
                // decision 1): the run loop and the tests both go through it, so a key that reaches
                // no handler — as `↓` did not reach the help's own scroll — is a thing a test can
                // catch. The effect is what the program asks for in return; there is nothing to ask
                // for until RFC 067's fourth commit.
                let Effect::None = keys::handle_key(&mut app, key);
            }
        }

        // Sends a silent change check once the interval has passed (RFC 031 §5); no I/O otherwise.
        app.tick(Instant::now());

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

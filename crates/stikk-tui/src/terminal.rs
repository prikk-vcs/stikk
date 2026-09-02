//! Terminal setup, teardown, and the panic-safe restore guard (handoff §6; design CL-06, NFR-R01).
//!
//! Raw mode and the alternate screen are entered once, guarded by [`TerminalGuard`], whose `Drop`
//! restores the terminal. A panic hook restores it too, *before* the default hook prints, so a bug
//! surfaces as a readable message on a working terminal rather than a wedged one. The two paths are
//! idempotent.

use std::io::{self, IsTerminal};

use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use stikk_model::{Result, StikkError};

/// Whether stdout is a real terminal. The interactive TUI starts only when this is true (design
/// CL-06); otherwise the launcher takes the one-shot path instead of emitting control sequences into
/// a pipe.
#[must_use]
pub fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

/// An RAII guard that puts the terminal into raw mode + the alternate screen on construction and
/// restores it on drop. Also installs the panic-restore hook.
#[derive(Debug)]
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen, installing the panic-restore hook.
    ///
    /// # Errors
    /// [`StikkError::Environment`] if the terminal cannot be put into raw mode or switched screens.
    pub fn enter() -> Result<Self> {
        enable_raw_mode().map_err(|e| env("could not enter terminal raw mode", e))?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            // Undo the raw-mode change so we do not leave the terminal half-configured.
            let _ = disable_raw_mode();
            return Err(env("could not enter the alternate screen", error));
        }
        install_panic_hook();
        Ok(Self { _private: () })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}

/// Restore cooked mode and the main screen. Idempotent: safe to call more than once.
fn restore() -> io::Result<()> {
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    disable_raw_mode()
}

/// Install a panic hook that restores the terminal before the previous hook runs.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        previous(info);
    }));
}

fn env(context: &str, error: io::Error) -> StikkError {
    StikkError::environment(context.to_string(), error)
}

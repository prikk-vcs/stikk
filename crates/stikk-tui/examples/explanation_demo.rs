//! Runnable demo of the explanation &amp; discovery surface against a scripted `NullBackend` — no prikk
//! binary and no repository required (handoff §11; RFC 007).
//!
//! Run it from the workspace root:
//!
//! ```sh
//! cargo run -p stikk-tui --example explanation_demo
//! ```
//!
//! The backend is scripted so that **opening History refuses** — so the interesting surfaces are one
//! keypress away:
//!
//! - `Enter` on Orientation tries to open History; the scripted refusal **auto-opens the refusal
//!   overlay** — prikk's message verbatim, a plain-language gloss, and stikk-authored next-steps
//!   (`↑/↓` to move, `Enter` to activate). None of the next-steps mutates or auto-retries.
//! - `:` opens the **command palette** — type to filter; every command shows its binding.
//! - `?` opens the **glossary / help** browser — the Git → prikk terminology mapping.
//! - `R` opens the **recent refusals** for the session; `Enter` re-opens one.
//! - `Esc`/`q` steps back; `Ctrl-C` quits.

use std::path::Path;
use std::process::ExitCode;

use stikk_prikk::{NullBackend, Orientation, RefEntry};
use stikk_state::Config;

fn main() -> ExitCode {
    if !stikk_tui::stdout_is_tty() {
        eprintln!("explanation_demo needs a real terminal (a TTY).");
        return ExitCode::from(4);
    }

    let backend = NullBackend::supported()
        .with_orientation(Orientation {
            queued_patches: 0,
            queued_target: None,
            main_ref_state: Some(
                "76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0".to_string(),
            ),
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
        })
        // History refuses — the whole point of the demo: a real refusal to explain.
        .with_history_refusal("ref \"heads/main\" does not exist at this revision")
        .with_refs(vec![
            RefEntry {
                name: "heads/main".to_string(),
                id: "76cee1dc".to_string(),
                closed: false,
                received: false,
            },
            RefEntry {
                name: "tags/v1".to_string(),
                id: "32276e15".to_string(),
                closed: false,
                received: false,
            },
        ]);

    match stikk_tui::run(Path::new("demo-repo"), &backend, &Config::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

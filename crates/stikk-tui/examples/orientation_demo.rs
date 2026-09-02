//! Runnable demo of the stikk TUI shell + Orientation view against a scripted `NullBackend` — no
//! prikk binary and no repository required (handoff §8).
//!
//! Run it from the workspace root:
//!
//! ```sh
//! cargo run -p stikk-tui --example orientation_demo
//! ```
//!
//! The orientation (a couple of queued patches, a published `heads/main`) is scripted. Signing
//! readiness still reflects your real environment — set `PRIKK_MAINTAINER_KEY_ID` and
//! `PRIKK_MAINTAINER_SEED` (any values) before running to see the maintainer badge light up, since
//! stikk reads only their *presence*, never their values.
//!
//! Keys: `?` help · `r` refresh · `q`/`Esc`/`Ctrl-C` quit.

use std::path::Path;
use std::process::ExitCode;

use stikk_prikk::{NullBackend, Orientation};
use stikk_state::Config;

fn main() -> ExitCode {
    if !stikk_tui::stdout_is_tty() {
        eprintln!("orientation_demo needs a real terminal (a TTY).");
        return ExitCode::from(4);
    }
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 2,
        main_ref_state: Some(
            "237d0681acace31e17f80dee61d386c8d13529056721ba9c188e42ee4a13d5f8".to_string(),
        ),
        trailing_partial_wal_bytes: 0,
    });
    match stikk_tui::run(Path::new("demo-repo"), &backend, &Config::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

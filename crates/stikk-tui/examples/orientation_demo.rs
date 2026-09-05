//! Runnable demo of the stikk TUI shell + Orientation view against a scripted `NullBackend` — no
//! prikk binary and no repository required (handoff §8; RFC 010 §9).
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
//! **The initial orientation read is deliberately slowed down** (`SlowOrientation`, below) — a real
//! `NullBackend` answers instantly, which would make RFC 010's whole point (the Orientation
//! `Loading` state is now real and the UI stays responsive while it is showing) invisible in a casual
//! run. Watch for "loading orientation…" for about a second on launch, and note that `?` (help) still
//! opens immediately during it — the seam call is on a worker thread, not the UI thread.
//!
//! Keys: `?` help · `r` refresh · `q`/`Esc`/`Ctrl-C` quit.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use stikk_model::{ChangeToken, Result};
use stikk_prikk::{
    CommitResult, Handshake, History, NullBackend, Orientation, Prikk, RefEntry, StateFiles,
    WorktreeStatus,
};
use stikk_state::Config;

/// Delegates every read to a `NullBackend`, except `orientation`, which sleeps first — see the module
/// doc for why. The sleep runs on the worker thread (RFC 010), never the UI thread that draws and polls
/// input, which is exactly the property this demo exists to show.
struct SlowOrientation(NullBackend);

impl Prikk for SlowOrientation {
    fn handshake(&self) -> Result<Handshake> {
        self.0.handshake()
    }

    fn orientation(&self, repo: &Path) -> Result<Orientation> {
        std::thread::sleep(Duration::from_millis(900));
        self.0.orientation(repo)
    }

    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<History> {
        self.0.history(repo, reff, limit)
    }

    fn block_state(&self, repo: &Path, reff: &str) -> Result<StateFiles> {
        self.0.block_state(repo, reff)
    }

    fn tags(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        self.0.tags(repo)
    }

    fn refs(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        self.0.refs(repo)
    }

    fn worktree_status(&self, repo: &Path, reff: &str) -> Result<WorktreeStatus> {
        self.0.worktree_status(repo, reff)
    }

    fn change_token(&self, repo: &Path) -> Result<ChangeToken> {
        self.0.change_token(repo)
    }

    fn commit(&self, repo: &Path, reff: &str, message: &str) -> Result<CommitResult> {
        self.0.commit(repo, reff, message)
    }
}

fn main() -> ExitCode {
    if !stikk_tui::stdout_is_tty() {
        eprintln!("orientation_demo needs a real terminal (a TTY).");
        return ExitCode::from(4);
    }
    let backend = SlowOrientation(NullBackend::supported().with_orientation(Orientation {
        queued_patches: 2,
        queued_target: Some("heads/main".to_string()),
        main_ref_state: Some(
            "237d0681acace31e17f80dee61d386c8d13529056721ba9c188e42ee4a13d5f8".to_string(),
        ),
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
    }));
    match stikk_tui::run(Path::new("demo-repo"), &backend, &Config::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

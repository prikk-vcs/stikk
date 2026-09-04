//! Runnable demo of the Changes (worktree-vs-baseline) view against a scripted `NullBackend` — no
//! prikk binary and no repository required (handoff §8; RFC 008).
//!
//! Run it from the workspace root:
//!
//! ```sh
//! cargo run -p stikk-tui --example changes_demo
//! ```
//!
//! The backend reports prikk 0.28.1 (where `worktree-status` is fixed — RFC 008/UD-03) and a dirty
//! worktree on `heads/main`: a modified file, a missing file, and two untracked files — plus prikk's
//! own **queued-elsewhere warning** (RFC 009 F4): the active WAL holds queued work on `heads/main`
//! itself, so the two "untracked" files might actually be exactly that work seen from another ref's
//! baseline.
//!
//! - `w` opens Changes; the queued-elsewhere warning renders as a distinct band above the entries,
//!   verbatim and inert. While it is present, the untracked filter's "a commit still captures them"
//!   claim is suppressed and replaced by a pointer to the warning — the two would otherwise contradict
//!   each other (RFC 009 decision 3).
//! - `u` toggles the display-only untracked filter (UD-08) and the whole-worktree reminder (UD-06).
//! - Per-file content diff is named as awaiting prikk support (UD-09) — never faked.
//! - `:` palette · `?` glossary · `Esc`/`q` back · `Ctrl-C` quit.
//!
//! To see the version-gate guidance instead, change `with_version(0, 28, 1)` to `with_version(0, 27, 1)`:
//! opening Changes then shows "needs prikk ≥ 0.28" rather than running the pre-fix command.

use std::path::Path;
use std::process::ExitCode;

use stikk_prikk::{NullBackend, Orientation, WorktreeEntry, WorktreeStatus};
use stikk_state::Config;

fn entry(kind: &str, path: &str, note: &str) -> WorktreeEntry {
    WorktreeEntry {
        kind: kind.to_string(),
        path: path.to_string(),
        note: note.to_string(),
    }
}

fn main() -> ExitCode {
    if !stikk_tui::stdout_is_tty() {
        eprintln!("changes_demo needs a real terminal (a TTY).");
        return ExitCode::from(4);
    }

    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_orientation(Orientation {
            queued_patches: 1,
            queued_target: Some("heads/main".to_string()),
            main_ref_state: Some(
                "76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0".to_string(),
            ),
            trailing_partial_wal_bytes: 0,
        })
        .with_worktree_status(WorktreeStatus {
            reff: "heads/main".to_string(),
            clean: false,
            tracked: 3,
            unchanged: 1,
            missing: 1,
            modified: 1,
            untracked: 2,
            unsupported: 0,
            entries: vec![
                entry(
                    "modified",
                    "src/main.rs",
                    "tracked file bytes differ from the baseline",
                ),
                entry(
                    "missing",
                    "docs/guide.md",
                    "tracked file is absent from the worktree",
                ),
                entry(
                    "untracked",
                    "notes.tmp",
                    "worktree file is not in the baseline",
                ),
                entry(
                    "untracked",
                    "target/debug/build.log",
                    "worktree file is not in the baseline",
                ),
            ],
            queued_elsewhere: None,
        })
        // RFC 009 F4 — the state that caused the defect, made drivable with no prikk and no
        // repository: prikk's own warning that the active WAL holds queued work for *this* ref, so an
        // "untracked" file above may actually be that work seen from a different baseline.
        .with_queued_elsewhere(
            "note: the active WAL has queued (unsealed) patches for heads/main, not heads/other -- \
             that is real, committed work, not shown above; any \"untracked\" file here may be exactly \
             that work seen from this ref's own baseline, so do not delete based on this report alone \
             (see `prikk status`)",
        );

    match stikk_tui::run(Path::new("demo-repo"), &backend, &Config::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

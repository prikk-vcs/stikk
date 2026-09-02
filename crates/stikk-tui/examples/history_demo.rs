//! Runnable demo of the History and Block-detail views against a scripted `NullBackend` — no prikk
//! binary and no repository required (handoff §8; RFC 006).
//!
//! Run it from the workspace root:
//!
//! ```sh
//! cargo run -p stikk-tui --example history_demo
//! ```
//!
//! A three-block lineage on `heads/main`, a couple of extra refs (a tag, a closed branch), and a
//! scripted tip state are wired up. Everything is interactive through the real run loop:
//!
//! - `Enter` opens History from Orientation, then drills into the selected block.
//! - `↑`/`↓` (or `k`/`j`) move the selection.
//! - `b` opens the ref picker; `Enter` there switches the focused ref.
//! - `Esc`/`q` steps back out (and quits at the top); `?` toggles help.
//!
//! Note the honest ceilings the views state plainly: prikk replays state only to the ref *tip*, and
//! per-patch content inspection awaits prikk support (UD-09).

use std::path::Path;
use std::process::ExitCode;

use stikk_prikk::{BlockRow, History, NullBackend, Orientation, RefEntry, StateFiles};
use stikk_state::Config;

fn block(
    id: &str,
    seq: u64,
    kind: &str,
    patches: u64,
    parents: u64,
    prev: Option<&str>,
) -> BlockRow {
    BlockRow {
        block_id: id.to_string(),
        ref_state_id: format!("rs-{id}"),
        update_seq: seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents,
        patches,
        rollback_patches: 0,
        required_attestations: 0,
        previous_ref_state: prev.map(str::to_string),
    }
}

fn main() -> ExitCode {
    if !stikk_tui::stdout_is_tty() {
        eprintln!("history_demo needs a real terminal (a TTY).");
        return ExitCode::from(4);
    }

    let backend = NullBackend::supported()
        .with_orientation(Orientation {
            queued_patches: 2,
            main_ref_state: Some(
                "76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0".to_string(),
            ),
            trailing_partial_wal_bytes: 0,
        })
        .with_history(History {
            reff: "heads/main".to_string(),
            blocks: vec![
                block(
                    "76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0",
                    3,
                    "Normal",
                    4,
                    1,
                    Some("3f5f846e39f1f1f841faf7d673b98337db9783ca24eba1c210a9f28897330fbf"),
                ),
                block(
                    "3f5f846e39f1f1f841faf7d673b98337db9783ca24eba1c210a9f28897330fbf",
                    2,
                    "Normal",
                    2,
                    1,
                    Some("3b0d20d04949092224ed973c08af2777f495121d8b0fcef3bcc5a099c46bf127"),
                ),
                block(
                    "3b0d20d04949092224ed973c08af2777f495121d8b0fcef3bcc5a099c46bf127",
                    1,
                    "Root",
                    1,
                    0,
                    None,
                ),
            ],
        })
        .with_state(StateFiles {
            target_block: "76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0"
                .to_string(),
            files: vec![
                "readme.txt".to_string(),
                "src/main.rs".to_string(),
                "docs/guide.md".to_string(),
            ],
            total_bytes: 4_213,
        })
        .with_refs(vec![
            RefEntry {
                name: "heads/main".to_string(),
                id: "76cee1dc".to_string(),
                closed: false,
                received: false,
            },
            RefEntry {
                name: "heads/oldwork".to_string(),
                id: "75a22546".to_string(),
                closed: true,
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

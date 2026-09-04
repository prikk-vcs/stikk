//! Full-shell render tests (design TS-01/TS-02), driven by the scripted backend.
//!
//! `App` sends requests to a worker thread rather than calling `Prikk` directly (RFC 010), so these
//! tests drive it through the same [`crate::worker::run`] function the real worker thread calls —
//! synchronously, on the test thread, one batch of queued requests at a time via [`drain`] — rather than
//! spinning up a real thread. That is exactly the split the design calls for: the async plumbing itself
//! is proven in `app::tests` and `worker::tests`; these tests only care that a loaded (or still-loading)
//! view renders correctly.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_prikk::{NullBackend, Orientation};
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;
use crate::worker::Request;

fn draw(app: &App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f)).unwrap();
    buffer_text(terminal.backend().buffer())
}

fn open(repo: &str, config: &Config) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::open(repo, config, tx), rx)
}

/// Answer every request currently queued against `backend`, applying each response in turn.
fn drain(app: &mut App, rx: &mpsc::Receiver<Request>, backend: &NullBackend) {
    while let Ok(req) = rx.try_recv() {
        let (tmp_tx, tmp_rx) = mpsc::channel();
        tmp_tx.send(req).unwrap();
        drop(tmp_tx);
        let (res_tx, res_rx) = mpsc::channel();
        crate::worker::run(backend, app.repo(), tmp_rx, res_tx);
        if let Ok(response) = res_rx.try_recv() {
            app.apply(response);
        }
    }
}

#[test]
fn renders_header_orientation_and_status_together() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 1,
        queued_target: Some("heads/main".into()),
        main_ref_state: Some("237d0681".into()),
        trailing_partial_wal_bytes: 0,
    });
    let (mut app, rx) = open("/home/dev/sample-repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("stikk")); // header
    assert!(text.contains("sample-repo")); // header repo name
    assert!(text.contains("Orientation")); // body
    assert!(text.contains("prikk 0.30.0")); // orientation content (RFC 009: the new NullBackend default)
    assert!(text.contains("q:back")); // status bar
}

#[test]
fn a_refusal_renders_the_failure_body_verbatim() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("Cannot open repository"));
    assert!(text.contains("retired format 3"));
}

#[test]
fn overlay_draws_over_the_view() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_glossary();
    let text = draw(&app, 90, 24);
    assert!(text.contains("Glossary"));
    // The status bar still shows beneath the centred overlay.
    assert!(text.contains("q:back"));
}

#[test]
fn a_banner_renders_above_the_status_bar() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let err = stikk_model::StikkError::LockConflict {
        message: "lock held by another writer".into(),
    };
    app.surface_error(&err, stikk_core::OperationContext::Orient);
    let text = draw(&app, 90, 24);
    assert!(text.contains("another writer"));
    assert!(text.contains("Esc dismisses"));
}

#[test]
fn a_fault_renders_the_untouched_repository_screen() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let err = stikk_model::StikkError::Internal {
        detail: "invariant X violated".into(),
    };
    app.surface_error(&err, stikk_core::OperationContext::Other);
    let text = draw(&app, 90, 24);
    assert!(text.contains("was not touched"));
    assert!(text.contains("read-only"));
}

#[test]
fn a_tiny_terminal_shows_the_too_small_notice() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 40, 10);
    assert!(text.contains("terminal too small"));
}

#[test]
fn a_pending_history_load_renders_as_loading() {
    // RFC 010 §5: a load that used to be instantaneous (and so unobservable) is now a real gap the
    // shell must render something for — proof that the `Screen::Loading` path actually reaches pixels.
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    // Deliberately not drained: the response has not arrived yet.
    let text = draw(&app, 90, 24);
    assert!(text.contains("loading history"));
}

#[test]
fn a_pending_ref_picker_renders_as_loading() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_ref_picker();
    let text = draw(&app, 90, 24);
    assert!(text.contains("loading refs"));
}

#[test]
fn history_screen_renders_the_lineage_and_queue_tier() {
    use stikk_prikk::{BlockRow, History};
    let backend = NullBackend::supported()
        .with_orientation(Orientation {
            queued_patches: 3,
            queued_target: Some("heads/main".into()),
            main_ref_state: Some("bbbb".into()),
            trailing_partial_wal_bytes: 0,
        })
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![
                BlockRow {
                    block_id: "bbbbbbbbbbbbbbbb".into(),
                    ref_state_id: "rs-b".into(),
                    update_seq: 2,
                    kind: "Normal".into(),
                    rollback_block: false,
                    parents: 1,
                    patches: 4,
                    rollback_patches: 0,
                    required_attestations: 0,
                    previous_ref_state: Some("rs-a".into()),
                },
                BlockRow {
                    block_id: "aaaaaaaaaaaaaaaa".into(),
                    ref_state_id: "rs-a".into(),
                    update_seq: 1,
                    kind: "Root".into(),
                    rollback_block: false,
                    parents: 0,
                    patches: 1,
                    rollback_patches: 0,
                    required_attestations: 0,
                    previous_ref_state: None,
                },
            ],
        });
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("History")); // view title
    assert!(text.contains("heads/main")); // ref in title
    assert!(text.contains("not yet sealed")); // queue tier
    assert!(text.contains("Normal")); // tip block kind
    assert!(text.contains("Root")); // root block kind
    assert!(text.contains("tip")); // tip marker
}

#[test]
fn block_detail_screen_shows_tip_state_and_the_ud09_note() {
    use stikk_prikk::{BlockRow, History, StateFiles};
    let backend = NullBackend::supported()
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![BlockRow {
                block_id: "bbbbbbbbbbbbbbbb".into(),
                ref_state_id: "rs-b".into(),
                update_seq: 2,
                kind: "Normal".into(),
                rollback_block: false,
                parents: 1,
                patches: 4,
                rollback_patches: 0,
                required_attestations: 0,
                previous_ref_state: Some("rs-a".into()),
            }],
        })
        .with_state(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec!["readme.txt".into(), "src/main.rs".into()],
            total_bytes: 128,
        });
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    drain(&mut app, &rx, &backend);
    app.select(); // drill into the tip
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("Block")); // view title
    assert!(text.contains("readme.txt")); // tip state file
    assert!(text.contains("src/main.rs"));
    assert!(text.contains("UD-09")); // honest ceiling note
}

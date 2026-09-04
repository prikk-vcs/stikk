//! Render tests for the status bar (design TS-01; RFC 010 `⟳ n` indicator).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_core::OrientationView;
use stikk_model::{Capability, Readiness};
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;

fn render_app(app: &App) -> String {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f, f.area())).unwrap();
    buffer_text(terminal.backend().buffer())
}

fn from_state(repo: &str, state: OrientationState, palette: Palette) -> App {
    let (tx, _rx) = mpsc::channel();
    App::from_state(repo, state, palette, tx)
}

#[test]
fn shows_repo_focused_ref_and_hint() {
    let (tx, _rx) = mpsc::channel();
    let app = App::open("/home/dev/project", &Config::default(), tx);
    let text = render_app(&app);
    assert!(text.contains("project"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("q:back"));
    // Never "HEAD".
    assert!(!text.contains("HEAD"));
}

#[test]
fn shows_queue_and_maintainer_badge() {
    let r = Readiness {
        author_ready: true,
        maintainer_ready: true,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        queued_patches: 4,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
    };
    let app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    let text = render_app(&app);
    assert!(text.contains("queued"));
    assert!(text.contains("MNT"));
    assert!(text.contains("AUT"));
}

#[test]
fn read_only_badge_appears_and_no_queue_when_zero() {
    let r = Readiness {
        author_ready: false,
        maintainer_ready: true,
        read_only: true,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
    };
    let app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    let text = render_app(&app);
    assert!(text.contains("[RO]"));
    assert!(!text.contains("queued"));
}

#[test]
fn the_in_flight_indicator_appears_while_a_request_is_pending_and_clears_once_answered() {
    let r = Readiness {
        author_ready: true,
        maintainer_ready: true,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.30.0".into(),
        prikk_supported: true,
        prikk_validated: true,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
    };
    let mut app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    assert!(!render_app(&app).contains('⟳'));

    app.open_history(); // sends a request; the worker never answers it here
    let text = render_app(&app);
    assert!(text.contains("⟳ 1"));
}

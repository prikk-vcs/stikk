//! Render tests for the status bar (design TS-01).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_core::OrientationView;
use stikk_model::{Capability, Readiness};
use stikk_prikk::NullBackend;
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;

fn render_app(app: &App) -> String {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f, f.area())).unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn shows_repo_focused_ref_and_hint() {
    let backend = NullBackend::supported();
    let app = App::open("/home/dev/project", &backend, &Config::default());
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
    let app = App::from_state(
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
    let app = App::from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    let text = render_app(&app);
    assert!(text.contains("[RO]"));
    assert!(!text.contains("queued"));
}

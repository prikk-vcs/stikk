//! Full-shell render tests (design TS-01/TS-02), driven by the scripted backend.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_prikk::{NullBackend, Orientation};
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;

fn draw(app: &App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f)).unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn renders_header_orientation_and_status_together() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 1,
        main_ref_state: Some("237d0681".into()),
        trailing_partial_wal_bytes: 0,
    });
    let app = App::open("/home/dev/sample-repo", &backend, &Config::default());
    let text = draw(&app, 90, 24);
    assert!(text.contains("stikk")); // header
    assert!(text.contains("sample-repo")); // header repo name
    assert!(text.contains("Orientation")); // body
    assert!(text.contains("prikk 0.27.1")); // orientation content
    assert!(text.contains("q:quit")); // status bar
}

#[test]
fn a_refusal_renders_the_failure_body_verbatim() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let app = App::open("/repo", &backend, &Config::default());
    let text = draw(&app, 90, 24);
    assert!(text.contains("Cannot open repository"));
    assert!(text.contains("retired format 3"));
}

#[test]
fn overlay_draws_over_the_view() {
    let backend = NullBackend::supported();
    let mut app = App::open("/repo", &backend, &Config::default());
    app.toggle_help();
    let text = draw(&app, 90, 24);
    assert!(text.contains("Help"));
    // The status bar still shows beneath the centred overlay.
    assert!(text.contains("q:quit"));
}

#[test]
fn a_tiny_terminal_shows_the_too_small_notice() {
    let backend = NullBackend::supported();
    let app = App::open("/repo", &backend, &Config::default());
    let text = draw(&app, 40, 10);
    assert!(text.contains("terminal too small"));
}

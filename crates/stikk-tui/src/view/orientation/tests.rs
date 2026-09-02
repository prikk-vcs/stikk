//! Render tests for the Orientation view (design TS-01, using `TestBackend`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_model::{Capability, Readiness};

use super::*;
use crate::test_util::buffer_text;

fn view(readiness: Readiness, supported: bool, queued: u64, partial: u64) -> OrientationView {
    OrientationView {
        prikk_version: "prikk 0.27.1".to_string(),
        prikk_supported: supported,
        queued_patches: queued,
        trailing_partial_wal_bytes: partial,
        main_ref_state: Some("237d0681".to_string()),
        capability: Capability::derive(readiness),
        readiness,
    }
}

fn render_to_text(v: &OrientationView) -> String {
    let backend = TestBackend::new(90, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(v, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn shows_version_capability_and_readiness() {
    let r = Readiness {
        author_ready: true,
        maintainer_ready: true,
        read_only: false,
    };
    let text = render_to_text(&view(r, true, 0, 0));
    assert!(text.contains("prikk 0.27.1"));
    assert!(text.contains("supported"));
    assert!(text.contains("maintainer"));
    assert!(text.contains("author ready"));
    assert!(text.contains("Orientation"));
}

#[test]
fn viewer_when_no_readiness() {
    let text = render_to_text(&view(Readiness::none(), true, 0, 0));
    assert!(text.contains("viewer"));
    assert!(text.contains("not ready"));
}

#[test]
fn surfaces_queue_and_torn_tail() {
    let text = render_to_text(&view(Readiness::none(), true, 3, 7));
    assert!(text.contains("queued"));
    assert!(text.contains('3'));
    assert!(text.contains("torn tail"));
}

#[test]
fn unsupported_prikk_is_flagged() {
    let text = render_to_text(&view(Readiness::none(), false, 0, 0));
    assert!(text.contains("outside stikk's validated range"));
}

#[test]
fn a_hostile_ref_state_is_rendered_inert() {
    // C-T2a: a control sequence in a repository-sourced string must not survive to the terminal.
    let mut v = view(Readiness::none(), true, 0, 0);
    v.main_ref_state = Some("\u{1b}[2Jhello".to_string());
    let text = render_to_text(&v);
    assert!(!text.contains('\u{1b}'), "the ESC must be neutralized");
}

//! Tests for app state transitions (handoff §7), driven by the scripted backend.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use stikk_prikk::{NullBackend, Orientation};
use stikk_state::Config;

use super::*;
use crate::overlay::Overlay;

#[test]
fn open_loads_orientation_from_the_backend() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 2,
        main_ref_state: Some("abc".into()),
        trailing_partial_wal_bytes: 0,
    });
    let app = App::open("/repo", &backend, &Config::default());
    match app.state() {
        OrientationState::Loaded(view) => assert_eq!(view.queued_patches, 2),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn a_refusal_becomes_a_failed_state_with_verbatim_message() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let app = App::open("/repo", &backend, &Config::default());
    match app.state() {
        OrientationState::Failed(msg) => assert!(msg.contains("retired format 3")),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn help_overlay_toggles() {
    let mut app = App::from_state("/repo", OrientationState::Loading, Palette::default());
    assert!(!app.has_overlay());
    app.toggle_help();
    assert_eq!(app.top_overlay(), Some(Overlay::Help));
    app.toggle_help();
    assert!(!app.has_overlay());
}

#[test]
fn close_overlay_and_quit_flags() {
    let mut app = App::from_state("/repo", OrientationState::Loading, Palette::default());
    app.toggle_help();
    app.close_overlay();
    assert!(!app.has_overlay());
    assert!(!app.should_quit());
    app.quit();
    assert!(app.should_quit());
}

#[test]
fn reload_re_sources_from_the_backend() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        queued_patches: 5,
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
    });
    let mut app = App::from_state(
        Path::new("/repo"),
        OrientationState::Loading,
        Palette::default(),
    );
    app.reload(&backend);
    match app.state() {
        OrientationState::Loaded(view) => assert_eq!(view.queued_patches, 5),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

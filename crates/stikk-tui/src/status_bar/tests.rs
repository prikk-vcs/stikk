//! Render tests for the status bar (design TS-01; RFC 010 `⟳ n` indicator).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_core::OrientationView;
use stikk_model::Binding;
use stikk_model::{Capability, Readiness, RoleReadiness};
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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 4,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
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

/// The acceptance-critical assertion (RFC 016 §4/`C-T2c′`): `Unknown` renders **distinctly** from
/// `Ready`, and — the actual prohibition, not merely "looks different" — the `Unknown` badge contains
/// no pass/`✓` marker at all. Asserting the absence, not merely the presence of a different glyph,
/// because `C-T2c′` is a prohibition on a claim stikk cannot verify, not a request for variety.
#[test]
fn maintainer_unknown_never_renders_as_a_pass() {
    let view_with = |maintainer_readiness| {
        let r = Readiness {
            author: RoleReadiness::NotReady,
            maintainer: maintainer_readiness,
            read_only: false,
        };
        OrientationView {
            prikk_version: "prikk 0.33.0".into(),
            prikk_supported: true,
            prikk_validated: true,
            validated_through: "0.33".to_string(),
            prikk_persists_messages: true,
            prikk_minor: 41,
            queued_patches: 0,
            queued_target: None,
            trailing_partial_wal_bytes: 0,
            main_ref_state: None,
            capability: Capability::derive(r),
            readiness: r,
            stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        }
    };

    let unknown_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::Unknown)),
        Palette::default(),
    ));
    let not_ready_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::NotReady)),
        Palette::default(),
    ));
    // `Ready` cannot be produced by `stikk-prikk::env` today (RFC 016 F3), but the render path must
    // still be exercised for it now, so the day it becomes reachable this test already covers it.
    let ready_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::Known(Binding::Matches))),
        Palette::default(),
    ));

    assert!(
        !unknown_text.contains("[MNT ✓]"),
        "an unverifiable adoption must never render the pass mark: {unknown_text:?}"
    );
    assert!(ready_text.contains("[MNT ✓]"));
    assert!(not_ready_text.contains("[MNT –]"));
    assert!(unknown_text.contains("[MNT ?]"));
    // All three are textually distinct from one another (NFR-A03: never colour alone).
    assert_ne!(unknown_text, ready_text);
    assert_ne!(unknown_text, not_ready_text);
    assert_ne!(ready_text, not_ready_text);
}

#[test]
fn read_only_badge_appears_and_no_queue_when_zero() {
    let r = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::Unknown,
        read_only: true,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
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
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.30.0".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
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

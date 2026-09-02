//! Tests for app state transitions (handoff §7), driven by the scripted backend.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use stikk_prikk::{BlockRow, History, NullBackend, Orientation, RefEntry, StateFiles};
use stikk_state::Config;

use super::*;
use crate::overlay::Overlay;

fn block(id: &str, seq: u64) -> BlockRow {
    BlockRow {
        block_id: id.to_string(),
        ref_state_id: format!("rs-{id}"),
        update_seq: seq,
        kind: "Normal".to_string(),
        rollback_block: false,
        parents: 1,
        patches: 1,
        rollback_patches: 0,
        required_attestations: 0,
        previous_ref_state: Some("prev".to_string()),
    }
}

fn two_block_backend() -> NullBackend {
    NullBackend::supported()
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![block("bbbb", 2), block("aaaa", 1)],
        })
        .with_state(StateFiles {
            target_block: "bbbb".into(),
            files: vec!["readme.txt".into()],
            total_bytes: 12,
        })
}

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
    assert_eq!(app.top_overlay(), Some(&Overlay::Help));
    app.toggle_help();
    assert!(!app.has_overlay());
}

#[test]
fn back_closes_overlay_then_pops_screen_then_quits() {
    let mut app = App::from_state("/repo", OrientationState::Loading, Palette::default());
    app.push_screen(Screen::History {
        view: HistoryView {
            reff: "heads/main".into(),
            queued: 0,
            blocks: vec![block("bbbb", 1)],
        },
        cursor: 0,
    });
    app.toggle_help();
    // 1) overlay closes first
    app.back();
    assert!(!app.has_overlay());
    assert!(!app.should_quit());
    // 2) then the screen pops
    app.back();
    assert!(matches!(app.focus(), Focus::Orientation(_)));
    assert!(!app.should_quit());
    // 3) then it quits at the root
    app.back();
    assert!(app.should_quit());
}

#[test]
fn select_from_orientation_opens_history_then_a_block() {
    let backend = two_block_backend();
    let mut app = App::open("/repo", &backend, &Config::default());
    // Orientation root → Select opens History for the focused ref.
    app.select(&backend);
    match app.focus() {
        Focus::History(view, cursor) => {
            assert_eq!(view.blocks.len(), 2);
            assert_eq!(cursor, 0);
        }
        other => panic!("expected History, got {other:?}"),
    }
    // Select again → the tip block's detail (state present because cursor 0 is the tip).
    app.select(&backend);
    match app.focus() {
        Focus::BlockDetail(detail) => {
            assert!(detail.is_tip);
            assert_eq!(detail.state.as_ref().map(|s| s.files.len()), Some(1));
        }
        other => panic!("expected BlockDetail, got {other:?}"),
    }
}

#[test]
fn nav_down_then_select_opens_a_non_tip_block_without_state() {
    let backend = two_block_backend();
    let mut app = App::open("/repo", &backend, &Config::default());
    app.open_history(&backend);
    app.nav_down(); // cursor 0 → 1 (the root block)
    app.select(&backend);
    match app.focus() {
        Focus::BlockDetail(detail) => {
            assert!(!detail.is_tip);
            assert!(detail.state.is_none()); // prikk replays only to the tip (RFC 006)
            assert_eq!(detail.row.update_seq, 1);
        }
        other => panic!("expected BlockDetail, got {other:?}"),
    }
}

#[test]
fn nav_down_is_clamped_to_the_last_block() {
    let backend = two_block_backend();
    let mut app = App::open("/repo", &backend, &Config::default());
    app.open_history(&backend);
    app.nav_down();
    app.nav_down();
    app.nav_down(); // past the end
    match app.focus() {
        Focus::History(_, cursor) => assert_eq!(cursor, 1),
        other => panic!("expected History, got {other:?}"),
    }
}

#[test]
fn ref_picker_selects_a_ref_and_reopens_history() {
    let backend = two_block_backend().with_refs(vec![
        RefEntry {
            name: "heads/main".into(),
            id: "x".into(),
            closed: false,
            received: false,
        },
        RefEntry {
            name: "tags/v1".into(),
            id: "y".into(),
            closed: false,
            received: false,
        },
    ]);
    let mut app = App::open("/repo", &backend, &Config::default());
    app.open_ref_picker(&backend);
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));
    app.nav_down(); // highlight tags/v1
    app.select(&backend); // pick it
    assert!(!app.has_overlay());
    assert_eq!(app.focused_ref(), "tags/v1");
    assert!(matches!(app.focus(), Focus::History(_, _)));
}

#[test]
fn a_history_refusal_surfaces_as_a_notice_overlay() {
    let backend = NullBackend::supported().with_history_refusal("ref does not exist");
    let mut app = App::open("/repo", &backend, &Config::default());
    app.open_history(&backend);
    match app.top_overlay() {
        Some(Overlay::Notice(msg)) => assert!(msg.contains("ref does not exist")),
        other => panic!("expected a Notice overlay, got {other:?}"),
    }
    // The root is undisturbed: no History screen was pushed.
    assert!(matches!(app.focus(), Focus::Orientation(_)));
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

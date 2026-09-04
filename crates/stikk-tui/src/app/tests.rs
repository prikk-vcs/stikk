//! Tests for app state transitions (handoff §7; RFC 006/007; RFC 010).
//!
//! `App` no longer calls the seam directly, so these tests drive it the way the design's own test plan
//! asks: send an action, pull the `Request` it produced off the channel, and feed a constructed
//! `Response` into `apply` — never sleeping or spinning up a real worker thread (RFC 010 §8: "the
//! threading is `std` and does not need proving; what needs proving is the state machine").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, ChangeEntry, ChangeKind, ChangesView, HistoryView, OperationContext,
};
use stikk_model::{Capability, Readiness, StikkError};
use stikk_prikk::{BlockRow, StateFiles};
use stikk_state::Config;

use super::*;
use crate::overlay::Overlay;
use crate::worker::{Request, RequestKind, Response, ResponseKind};

fn open(repo: &str, config: &Config) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::open(repo, config, tx), rx)
}

fn from_state(
    repo: &str,
    state: OrientationState,
    palette: Palette,
) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::from_state(repo, state, palette, tx), rx)
}

/// Pull the one request expected to have been sent so far. Panics (with a clear message) if none was
/// sent — exactly what a broken action-to-request mapping should do to a test.
fn next_request(rx: &mpsc::Receiver<Request>) -> Request {
    rx.try_recv().expect("expected a request to have been sent")
}

fn orientation_view(
    queued_patches: u64,
    queued_target: Option<&str>,
    main_ref_state: Option<&str>,
) -> stikk_core::OrientationView {
    let readiness = Readiness::none();
    stikk_core::OrientationView {
        prikk_version: "prikk 0.30.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        trailing_partial_wal_bytes: 0,
        main_ref_state: main_ref_state.map(str::to_string),
        capability: Capability::derive(readiness),
        readiness,
    }
}

fn loaded(view: stikk_core::OrientationView) -> OrientationState {
    OrientationState::Loaded(view)
}

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

fn two_block_history() -> HistoryView {
    HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        blocks: vec![block("bbbb", 2), block("aaaa", 1)],
    }
}

fn dirty_changes() -> ChangesView {
    ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        entries: vec![
            ChangeEntry {
                kind: ChangeKind::Modified,
                path: "readme.txt".into(),
                note: "bytes differ".into(),
            },
            ChangeEntry {
                kind: ChangeKind::Untracked,
                path: "notes.tmp".into(),
                note: "not in the baseline".into(),
            },
        ],
        queued_elsewhere: None,
    }
}

#[test]
fn open_sends_an_orientation_request_and_apply_loads_it() {
    let (mut app, rx) = open("/repo", &Config::default());
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    let view = orientation_view(2, Some("heads/main"), Some("abc"));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(Ok(view)),
    });
    match app.state() {
        OrientationState::Loaded(view) => {
            assert_eq!(view.queued_patches, 2);
            assert_eq!(view.queued_target.as_deref(), Some("heads/main"));
        }
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn an_orientation_refusal_becomes_a_failed_state_with_verbatim_message() {
    let (mut app, rx) = open("/repo", &Config::default());
    let req = next_request(&rx);
    let err = StikkError::Refusal {
        message: "repository is retired format 3".into(),
    };
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(Err(err)),
    });
    match app.state() {
        OrientationState::Failed(msg) => assert!(msg.contains("retired format 3")),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn a_stale_orientation_response_is_discarded_when_superseded_by_reload() {
    let (mut app, rx) = open("/repo", &Config::default());
    let first = next_request(&rx);
    // A refresh is requested before the first response arrives (e.g. `r` pressed immediately).
    app.reload();
    let second = next_request(&rx);
    assert_ne!(first.seq, second.seq);

    // The stale first response arrives first and must be discarded — still Loading, not Loaded.
    app.apply(Response {
        seq: first.seq,
        kind: ResponseKind::Orient(Ok(orientation_view(1, None, None))),
    });
    assert!(matches!(app.state(), OrientationState::Loading));

    // The current (second) response resolves it.
    app.apply(Response {
        seq: second.seq,
        kind: ResponseKind::Orient(Ok(orientation_view(2, None, None))),
    });
    match app.state() {
        OrientationState::Loaded(view) => assert_eq!(view.queued_patches, 2),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn glossary_overlay_toggles() {
    let (mut app, _rx) = from_state("/repo", OrientationState::Loading, Palette::default());
    assert!(!app.has_overlay());
    app.open_glossary();
    assert_eq!(app.top_overlay(), Some(&Overlay::Glossary));
    app.open_glossary();
    assert!(!app.has_overlay());
}

#[test]
fn back_closes_overlay_then_pops_screen_then_quits() {
    let (mut app, _rx) = from_state("/repo", OrientationState::Loading, Palette::default());
    app.push_screen(Screen::History {
        view: HistoryView {
            reff: "heads/main".into(),
            queued: 0,
            blocks: vec![block("bbbb", 1)],
        },
        cursor: 0,
        refreshing: None,
    });
    app.open_glossary();
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
fn back_pops_a_pending_loading_screen_the_stop_waiting_semantics() {
    // RFC 010 decision 4: leaving a view whose read is in flight is "stop waiting", not "cancel" — it
    // costs nothing but popping the placeholder; the eventual stale response is discarded (proven
    // separately below).
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    assert!(matches!(app.focus(), Focus::Loading("history")));
    app.back();
    assert!(matches!(app.focus(), Focus::Orientation(_)));
}

#[test]
fn a_stale_history_response_is_discarded_after_navigating_away() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    assert!(matches!(app.focus(), Focus::Loading("history")));

    // The user leaves before the response arrives.
    app.back();
    assert!(matches!(app.focus(), Focus::Orientation(_)));

    // The now-stale response lands: nothing must be pushed, and no overlay opened for its error path
    // either — it is silently discarded, per RFC 010 §4.
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    assert!(matches!(app.focus(), Focus::Orientation(_)));
    assert!(!app.has_overlay());
}

#[test]
fn a_stale_history_refusal_response_is_also_discarded() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.back();

    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Err(StikkError::Refusal {
            message: "ref does not exist".into(),
        })),
    });
    // No refusal overlay was opened over the (unrelated) current view, and nothing was recorded.
    assert!(!app.has_overlay());
    assert!(app.refusals().is_empty());
}

#[test]
fn a_pending_screen_shows_as_loading_until_it_resolves() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_changes();
    assert!(matches!(app.focus(), Focus::Loading("changes")));
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Ok(dirty_changes())),
    });
    match app.focus() {
        Focus::Changes(view, hide) => {
            assert!(!view.clean);
            assert!(!hide);
        }
        other => panic!("expected Changes, got {other:?}"),
    }
}

#[test]
fn select_from_orientation_opens_history_then_a_block() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    // Orientation root → Select opens History for the focused ref.
    app.select();
    let req = next_request(&rx);
    match &req.kind {
        RequestKind::History { reff } => assert_eq!(reff, "heads/main"),
        other => panic!("expected a History request, got {other:?}"),
    }
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    match app.focus() {
        Focus::History(view, cursor) => {
            assert_eq!(view.blocks.len(), 2);
            assert_eq!(cursor, 0);
        }
        other => panic!("expected History, got {other:?}"),
    }

    // Select again → the tip block's detail (state present because cursor 0 is the tip).
    app.select();
    let req = next_request(&rx);
    match &req.kind {
        RequestKind::BlockState { is_tip, .. } => assert!(*is_tip),
        other => panic!("expected a BlockState request, got {other:?}"),
    }
    let detail = BlockDetailView {
        row: block("bbbb", 2),
        is_tip: true,
        state: Some(StateFiles {
            target_block: "bbbb".into(),
            files: vec!["readme.txt".into()],
            total_bytes: 12,
        }),
    };
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::BlockState(Ok(detail)),
    });
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
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    app.nav_down(); // cursor 0 → 1 (the root block)
    app.select();
    let req = next_request(&rx);
    let RequestKind::BlockState { is_tip, row, .. } = &req.kind else {
        panic!("expected a BlockState request, got {:?}", req.kind);
    };
    assert!(!is_tip);
    let detail = BlockDetailView {
        row: row.clone(),
        is_tip: false,
        state: None,
    };
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::BlockState(Ok(detail)),
    });
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
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
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
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    assert!(matches!(app.top_overlay(), Some(Overlay::Loading { .. })));
    let refs_req = next_request(&rx);
    assert!(matches!(refs_req.kind, RequestKind::Refs));
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(vec![
            stikk_prikk::RefEntry {
                name: "heads/main".into(),
                id: "x".into(),
                closed: false,
                received: false,
            },
            stikk_prikk::RefEntry {
                name: "tags/v1".into(),
                id: "y".into(),
                closed: false,
                received: false,
            },
        ])),
    });
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));

    app.nav_down(); // highlight tags/v1
    app.select(); // pick it
    assert!(!app.has_overlay());
    assert_eq!(app.focused_ref(), "tags/v1");

    let history_req = next_request(&rx);
    match &history_req.kind {
        RequestKind::History { reff } => assert_eq!(reff, "tags/v1"),
        other => panic!("expected a History request, got {other:?}"),
    }
    assert!(matches!(app.focus(), Focus::Loading("history")));
}

#[test]
fn a_history_refusal_surfaces_as_a_refusal_overlay_and_is_remembered() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Err(StikkError::Refusal {
            message: "ref does not exist".into(),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::Refusal { card, .. }) => {
            assert_eq!(card.verbatim, "ref does not exist"); // verbatim (ER-02)
            assert!(card.gloss.is_some()); // a gloss for LoadHistory
            assert!(!card.next_steps.is_empty());
        }
        other => panic!("expected a Refusal overlay, got {other:?}"),
    }
    // The root is undisturbed: no History screen was pushed (the Loading placeholder was removed).
    assert!(matches!(app.focus(), Focus::Orientation(_)));
    // And the refusal is remembered (FR-112).
    assert_eq!(app.refusals().len(), 1);
}

#[test]
fn activating_the_refresh_next_step_closes_the_overlay_and_requests_a_reload() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Err(StikkError::Refusal {
            message: "ref does not exist".into(),
        })),
    });
    // Move to the "Refresh" next-step (LoadHistory offers: Choose another ref, Refresh) and activate.
    app.nav_down();
    app.select();
    assert!(!app.has_overlay()); // the refusal overlay closed
    // `reload()` re-requests Orientation (there is no History screen to also refresh).
    let reload_req = next_request(&rx);
    assert!(matches!(reload_req.kind, RequestKind::Orient));
    assert_eq!(app.refusals().len(), 1); // no new refusal recorded yet
}

#[test]
fn a_lock_conflict_surfaces_as_a_banner_not_an_overlay() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    let err = StikkError::LockConflict {
        message: "lock held by another writer".into(),
    };
    app.surface_error(&err, OperationContext::Orient);
    assert!(!app.has_overlay());
    assert!(app.banner().unwrap().contains("another writer"));
    // Back dismisses the banner before touching screens.
    app.back();
    assert!(app.banner().is_none());
}

#[test]
fn palette_opens_and_filters() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_palette();
    assert!(app.wants_text_input());
    app.input_char('h');
    app.input_char('i');
    app.input_char('s'); // "his" → History
    match app.top_overlay() {
        Some(Overlay::Palette { filter, .. }) => assert_eq!(filter, "his"),
        other => panic!("expected Palette, got {other:?}"),
    }
    app.backspace();
    match app.top_overlay() {
        Some(Overlay::Palette { filter, .. }) => assert_eq!(filter, "hi"),
        other => panic!("expected Palette, got {other:?}"),
    }
}

#[test]
fn recent_refusals_overlay_reopens_a_card() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history(); // will record a refusal
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Err(StikkError::Refusal {
            message: "ref does not exist".into(),
        })),
    });
    app.back(); // close the card
    app.open_refusals();
    match app.top_overlay() {
        Some(Overlay::Refusals { records, .. }) => assert_eq!(records.len(), 1),
        other => panic!("expected Refusals, got {other:?}"),
    }
    app.select(); // re-open the remembered refusal
    assert!(matches!(app.top_overlay(), Some(Overlay::Refusal { .. })));
}

#[test]
fn open_changes_pushes_the_view_and_toggle_hides_untracked() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_changes();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Ok(dirty_changes())),
    });
    match app.focus() {
        Focus::Changes(view, hide) => {
            assert!(!view.clean);
            assert_eq!(view.entries.len(), 2);
            assert!(!hide);
        }
        other => panic!("expected Changes, got {other:?}"),
    }
    app.toggle_untracked();
    match app.focus() {
        Focus::Changes(_, hide) => assert!(hide),
        other => panic!("expected Changes, got {other:?}"),
    }
}

#[test]
fn open_changes_error_response_surfaces_as_a_banner_not_a_screen() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_changes();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Err(StikkError::NotReady {
            detail: "Worktree review needs prikk ≥ 0.28 — this prikk is 0.27.1.".into(),
        })),
    });
    // No Changes screen was pushed (the Loading placeholder was removed); the guidance is a banner
    // (inline class), not a broken screen.
    assert!(matches!(app.focus(), Focus::Orientation(_)));
    assert!(app.banner().unwrap().contains("0.28"));
}

#[test]
fn reload_requests_a_fresh_orientation_and_apply_updates_it() {
    let (mut app, rx) = from_state("/repo", OrientationState::Loading, Palette::default());
    app.reload();
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(Ok(orientation_view(5, None, None))),
    });
    match app.state() {
        OrientationState::Loaded(view) => assert_eq!(view.queued_patches, 5),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn reload_keeps_the_stale_history_view_visible_while_refreshing() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    assert!(matches!(app.focus(), Focus::History(_, _)));

    app.reload();
    // The view stays visible (never replaced by a Loading placeholder) while the refresh is pending —
    // only the `⟳ n` indicator (tested in status_bar) shows anything is happening (RFC 010 §5).
    match app.focus() {
        Focus::History(view, _) => assert_eq!(view.blocks.len(), 2),
        other => panic!("expected History (still), got {other:?}"),
    }
    let _orient_req = next_request(&rx); // reload also re-requests Orientation
    let history_req = next_request(&rx);
    match &history_req.kind {
        RequestKind::History { reff } => assert_eq!(reff, "heads/main"),
        other => panic!("expected a History request, got {other:?}"),
    }

    let refreshed = HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        blocks: vec![block("cccc", 3)],
    };
    app.apply(Response {
        seq: history_req.seq,
        kind: ResponseKind::History(Ok(refreshed)),
    });
    match app.focus() {
        Focus::History(view, _) => assert_eq!(view.blocks.len(), 1),
        other => panic!("expected the refreshed History, got {other:?}"),
    }
}

#[test]
fn a_stale_history_refresh_response_leaves_the_view_untouched() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });

    app.reload();
    let _orient_req = next_request(&rx);
    let stale_refresh = next_request(&rx);
    // A second reload supersedes the first refresh before it resolves.
    app.reload();
    let _orient_req_2 = next_request(&rx);
    let current_refresh = next_request(&rx);
    assert_ne!(stale_refresh.seq, current_refresh.seq);

    // The stale refresh arrives first and must not touch the (still 2-block) view.
    app.apply(Response {
        seq: stale_refresh.seq,
        kind: ResponseKind::History(Ok(HistoryView {
            reff: "heads/main".into(),
            queued: 0,
            blocks: vec![block("stale", 9)],
        })),
    });
    match app.focus() {
        Focus::History(view, _) => assert_eq!(view.blocks.len(), 2),
        other => panic!("expected the original History (unaffected), got {other:?}"),
    }

    // The current refresh resolves normally.
    app.apply(Response {
        seq: current_refresh.seq,
        kind: ResponseKind::History(Ok(HistoryView {
            reff: "heads/main".into(),
            queued: 0,
            blocks: vec![block("fresh", 4)],
        })),
    });
    match app.focus() {
        Focus::History(view, _) => assert_eq!(view.blocks[0].block_id, "fresh"),
        other => panic!("expected the refreshed History, got {other:?}"),
    }
}

#[test]
fn in_flight_count_and_operations_track_running_and_finished_requests() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    assert_eq!(app.in_flight_count(), 0);
    assert!(app.operations().is_empty());

    app.open_history();
    assert_eq!(app.in_flight_count(), 1);
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    assert_eq!(app.in_flight_count(), 0);
    assert_eq!(app.operations().len(), 1);
    assert_eq!(app.operations()[0].label, "history");
    assert!(matches!(
        app.operations()[0].status,
        OperationStatus::Finished { ok: true }
    ));
}

#[test]
fn a_failed_operation_is_recorded_as_finished_not_ok() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Err(StikkError::Refusal {
            message: "ref does not exist".into(),
        })),
    });
    assert!(matches!(
        app.operations()[0].status,
        OperationStatus::Finished { ok: false }
    ));
}

#[test]
fn open_operations_snapshots_the_list_and_toggles() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history(); // one running operation
    app.open_operations();
    match app.top_overlay() {
        Some(Overlay::Operations { operations }) => assert_eq!(operations.len(), 1),
        other => panic!("expected Operations, got {other:?}"),
    }
    app.open_operations(); // toggles closed
    assert!(!app.has_overlay());
}

#[test]
fn worker_stopped_records_a_dismissible_fault() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.worker_stopped();
    assert!(app.fault().is_some());
    app.back();
    assert!(app.fault().is_none());
}

//! Tests for app state transitions (handoff §7; RFC 006/007; RFC 010).
//!
//! `App` no longer calls the seam directly, so these tests drive it the way the design's own test plan
//! asks: send an action, pull the `Request` it produced off the channel, and feed a constructed
//! `Response` into `apply` — never sleeping or spinning up a real worker thread (RFC 010 §8: "the
//! threading is `std` and does not need proving; what needs proving is the state machine").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, ChangeEntry, ChangeKind, ChangesView, CommitPreviewOutcome,
    ConfirmationSummary, HistoryView, OperationContext, Outcome, commit_preview,
};
use stikk_model::{Capability, Readiness, StikkError, Tier};
use stikk_prikk::{BlockRow, CommitResult, NullBackend, Orientation, StateFiles, WorktreeStatus};
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

/// An orientation view with AUTHOR signing readiness present (RFC 014's commit tests need this — the
/// default `orientation_view` is a Viewer, since most of this file's tests are read-only).
fn author_orientation_view() -> stikk_core::OrientationView {
    let readiness = Readiness {
        author_ready: true,
        maintainer_ready: false,
        read_only: false,
    };
    stikk_core::OrientationView {
        prikk_version: "prikk 0.30.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(readiness),
        readiness,
    }
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
    let banner = app.banner().unwrap();
    assert!(banner.contains("0.28"));
    // RFC 012 F-b: version skew must not point at signing keys — they were never the problem.
    assert!(!banner.contains("Trust & Keys"));
}

#[test]
fn a_signing_readiness_not_ready_still_points_at_trust_and_keys() {
    // RFC 012 F-b's disambiguation is narrow: only the LoadChanges version-gate reroutes (proven above
    // in `open_changes_error_response_surfaces_as_a_banner_not_a_screen`); an ordinary signing-readiness
    // `NotReady` from any other context is unaffected.
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.surface_error(
        &StikkError::NotReady {
            detail: "no signing key configured".into(),
        },
        OperationContext::Other,
    );
    assert!(app.banner().unwrap().contains("Trust & Keys"));
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

fn confirmation_summary(target_name: Option<&str>) -> ConfirmationSummary {
    ConfirmationSummary {
        operation: "Test operation".to_string(),
        target_ids: vec!["abc123".to_string()],
        counts: vec![("items", 1)],
        capability: Capability::Author,
        consequence: "Nothing real happens".to_string(),
        target_name: target_name.map(str::to_string),
    }
}

#[test]
fn a_tier_three_typed_confirmation_wants_text_input_but_tier_two_does_not() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: String::new(),
        error: None,
    });
    assert!(app.wants_text_input());

    app.close_overlay();
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(None),
        tier: Tier::Two,
        typed: String::new(),
        error: None,
    });
    assert!(!app.wants_text_input());
}

#[test]
fn typing_into_a_tier_three_typed_confirmation_builds_and_erases_the_typed_name() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: String::new(),
        error: None,
    });
    app.input_char('h');
    app.input_char('i');
    match app.top_overlay() {
        Some(Overlay::Confirmation { typed, .. }) => assert_eq!(typed, "hi"),
        other => panic!("expected Confirmation, got {other:?}"),
    }
    app.backspace();
    match app.top_overlay() {
        Some(Overlay::Confirmation { typed, .. }) => assert_eq!(typed, "h"),
        other => panic!("expected Confirmation, got {other:?}"),
    }
}

#[test]
fn typing_into_a_tier_two_confirmation_does_nothing() {
    // Only tier-3-typed collects typed input (RFC 013 §6); tiers 2/3 take a plain yes/no.
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(None),
        tier: Tier::Three,
        typed: String::new(),
        error: None,
    });
    app.input_char('x');
    match app.top_overlay() {
        Some(Overlay::Confirmation { typed, .. }) => assert!(typed.is_empty()),
        other => panic!("expected Confirmation, got {other:?}"),
    }
}

#[test]
fn nav_up_and_down_do_not_panic_or_move_anything_on_a_confirmation_overlay() {
    // A confirmation is a single prompt, not a list — nav keys must be safely absorbed as no-ops.
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: "partial".to_string(),
        error: None,
    });
    app.nav_up();
    app.nav_down();
    match app.top_overlay() {
        Some(Overlay::Confirmation { typed, .. }) => assert_eq!(typed, "partial"),
        other => panic!("expected Confirmation unchanged, got {other:?}"),
    }
}

#[test]
fn select_on_a_confirmation_overlay_with_no_pending_commit_is_a_defensive_no_op() {
    // RFC 014 §3 wires `select()` to `App`'s own `pending_commit` state; a `Confirmation` overlay
    // pushed directly (as every other test in this file does, bypassing the real preview flow) has no
    // such state, so `select()` must not panic or silently dismiss it — there is nothing to confirm.
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.push_overlay(Overlay::Confirmation {
        summary: confirmation_summary(None),
        tier: Tier::Two,
        typed: String::new(),
        error: None,
    });
    app.select();
    assert!(app.has_overlay(), "select() must not silently dismiss it");
}

// RFC 014 §3: the commit flow, end to end through `App`.

fn dirty_worktree_status() -> WorktreeStatus {
    WorktreeStatus {
        reff: "heads/main".to_string(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        entries: Vec::new(),
        queued_elsewhere: None,
    }
}

fn commit_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(Orientation {
            queued_patches: 0,
            queued_target: None,
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
        })
        .with_worktree_status(dirty_worktree_status())
}

#[test]
fn begin_commit_opens_the_message_prompt_when_author_ready() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    match app.top_overlay() {
        Some(Overlay::CommitMessage { reff, typed }) => {
            assert_eq!(reff, "heads/main");
            assert!(typed.is_empty());
        }
        other => panic!("expected CommitMessage, got {other:?}"),
    }
}

#[test]
fn begin_commit_refuses_with_a_banner_when_not_author_ready() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)), // Viewer: no signing readiness
        Palette::default(),
    );
    app.begin_commit();
    assert!(!app.has_overlay(), "no message prompt should open");
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("needs AUTHOR signing readiness"))
    );
}

#[test]
fn submitting_an_empty_commit_message_does_nothing() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.select(); // Enter with an empty typed field
    assert!(
        matches!(app.top_overlay(), Some(Overlay::CommitMessage { .. })),
        "the prompt must stay open — UD-01 requires a non-empty message"
    );
    assert!(
        rx.try_recv().is_err(),
        "no preview request without a message"
    );
}

#[test]
fn submitting_a_commit_message_dispatches_a_preview_request_and_shows_loading() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('h');
    app.input_char('i');
    app.select();
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::CommitPreview { reff } if reff == "heads/main"));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading {
            what: "commit preview",
            ..
        })
    ));
}

#[test]
fn a_blocked_commit_preview_shows_a_banner_and_closes_the_overlay() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::CommitPreview(Ok(CommitPreviewOutcome::Blocked(
            "the worktree matches this ref's replay baseline — there is nothing to commit"
                .to_string(),
        ))),
    });
    assert!(!app.has_overlay());
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("nothing to commit"))
    );
}

#[test]
fn a_ready_commit_preview_opens_the_confirmation_overlay_with_the_restated_summary() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.select();
    let req = next_request(&rx);
    let backend = commit_backend();
    let outcome = commit_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("a scripted backend's preview always succeeds");
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });
    match app.top_overlay() {
        Some(Overlay::Confirmation { summary, tier, .. }) => {
            assert_eq!(*tier, Tier::Two);
            assert!(summary.target_ids.contains(&"heads/main".to_string()));
        }
        other => panic!("expected Confirmation, got {other:?}"),
    }
}

#[test]
fn confirming_a_commit_dispatches_confirm_execute_and_a_success_shows_the_result_verbatim() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.select(); // message step -> preview request
    let preview_req = next_request(&rx);
    let backend = commit_backend();
    let outcome = commit_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });

    app.select(); // confirm step -> confirm+execute request
    let confirm_req = next_request(&rx);
    assert!(matches!(
        confirm_req.kind,
        RequestKind::CommitConfirmExecute { .. }
    ));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading { what: "commit", .. })
    ));

    let result = CommitResult {
        baseline_ref: "heads/main".to_string(),
        patch_id: "1".repeat(64),
        wal_sequence: 1,
        operations: 1,
        referenced_blobs: 1,
        text_edits: 0,
        changes: Vec::new(),
        notes: vec!["note: a note prikk printed".to_string()],
    };
    app.apply(Response {
        seq: confirm_req.seq,
        kind: ResponseKind::CommitConfirmExecute(Ok(Outcome {
            operation: "commit".to_string(),
            result: result.clone(),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::CommitResult { result: shown }) => assert_eq!(shown, &result),
        other => panic!("expected CommitResult, got {other:?}"),
    }
    // A successful commit refreshes Orientation so the queue count updates.
    let orient_req = next_request(&rx);
    assert!(matches!(orient_req.kind, RequestKind::Orient));
}

#[test]
fn a_failed_commit_confirm_execute_surfaces_through_present_not_silently() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.select();
    let preview_req = next_request(&rx);
    let backend = commit_backend();
    let outcome = commit_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });
    app.select();
    let confirm_req = next_request(&rx);
    app.apply(Response {
        seq: confirm_req.seq,
        kind: ResponseKind::CommitConfirmExecute(Err(StikkError::CrossRef {
            message: "lock conflict: active WAL is owned by heads/main; requested ref heads/other"
                .to_string(),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::Refusal { card, .. }) => {
            assert!(card.verbatim.contains("active WAL is owned by"));
        }
        other => panic!("expected a Refusal-shaped overlay for CrossRef, got {other:?}"),
    }
}

#[test]
fn back_on_the_message_prompt_clears_the_pending_commit() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.back();
    assert!(!app.has_overlay());
    // Re-opening and submitting a fresh message must still work — nothing from the abandoned attempt
    // lingers to interfere.
    app.begin_commit();
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::CommitMessage { .. })
    ));
}

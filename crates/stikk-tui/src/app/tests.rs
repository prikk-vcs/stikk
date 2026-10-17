//! Tests for app state transitions (handoff §7; RFC 006/007; RFC 010).
//!
//! `App` no longer calls the seam directly, so these tests drive it the way the design's own test plan
//! asks: send an action, pull the `Request` it produced off the channel, and feed a constructed
//! `Response` into `apply` — never sleeping or spinning up a real worker thread (RFC 010 §8: "the
//! threading is `std` and does not need proving; what needs proving is the state machine").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, BlockPatches, ChangeEntry, ChangeKind, ChangesView, CommitPreviewOutcome,
    ConfirmationSummary, HistoryView, OperationContext, Outcome, SealPreviewOutcome,
    commit_preview, seal_preview,
};
use stikk_model::{Capability, Readiness, RoleReadiness, StikkError, Tier};
use stikk_prikk::{
    BlockRow, CommitResult, NullBackend, Orientation, SealResult, StateFiles, WorktreeStatus,
};
use stikk_state::Config;

use super::*;
use crate::overlay::Overlay;
use crate::worker::{OrientRead, Request, RequestKind, Response, ResponseKind};

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
        interrupted_materialization: None,
        prikk_version: "prikk 0.30.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        trailing_partial_wal_bytes: 0,
        main_ref_state: main_ref_state.map(str::to_string),
        capability: Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

fn loaded(view: stikk_core::OrientationView) -> OrientationState {
    OrientationState::Loaded(view)
}

/// An orientation view with AUTHOR signing readiness present (RFC 014's commit tests need this — the
/// default `orientation_view` is a Viewer, since most of this file's tests are read-only).
fn author_orientation_view() -> stikk_core::OrientationView {
    let readiness = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    };
    stikk_core::OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.30.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

/// An orientation view with MAINTAINER readiness `Unknown` (RFC 016's seal ceremony tests need this —
/// `Unknown` is the only value `stikk-prikk::env` can ever produce for MAINTAINER presence, RFC 016
/// F3, and `Capability::derive` grants `Maintainer` on it, RFC 016 Q1).
fn maintainer_orientation_view(
    queued_patches: u64,
    queued_target: Option<&str>,
) -> stikk_core::OrientationView {
    let readiness = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    stikk_core::OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.33.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.33".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches,
        queued_target: queued_target.map(str::to_string),
        trailing_partial_wal_bytes: 0,
        main_ref_state: Some("237d0681".repeat(8)),
        capability: Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
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
        messages: Vec::new(),
        previous_ref_state: Some("prev".to_string()),
    }
}

fn two_block_history() -> HistoryView {
    HistoryView {
        reff: "heads/main".into(),
        queued: 0,
        queued_target: None,
        blocks: vec![block("bbbb", 2), block("aaaa", 1)],
    }
}

fn dirty_changes() -> ChangesView {
    ChangesView {
        refused_declarations: None,
        reff: "heads/main".into(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        refused: None,
        entries: vec![
            ChangeEntry {
                kind: ChangeKind::Modified,
                path: "readme.txt".into(),
                note: "bytes differ".into(),
                authoring: stikk_core::Authoring::Unreported,
                rename: None,
            },
            ChangeEntry {
                kind: ChangeKind::Untracked,
                path: "notes.tmp".into(),
                note: "not in the baseline".into(),
                authoring: stikk_core::Authoring::Unreported,
                rename: None,
            },
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
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
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(view))),
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
        kind: ResponseKind::Orient(OrientRead::stamped(Err(err))),
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
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(orientation_view(1, None, None)))),
    });
    assert!(matches!(app.state(), OrientationState::Loading));

    // The current (second) response resolves it.
    app.apply(Response {
        seq: second.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(orientation_view(2, None, None)))),
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
    assert_eq!(
        app.top_overlay(),
        Some(&Overlay::Glossary {
            offset: std::cell::Cell::new(0)
        })
    );
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
            queued_target: None,
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
        kind: ResponseKind::Changes(Ok(stikk_core::ChangesRead {
            view: dirty_changes(),
            history: stikk_core::RefHistory::Published,
        })),
    });
    match app.focus() {
        Focus::Changes(view, hide, _) => {
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
        state: stikk_core::BlockState::Listed(StateFiles {
            target_block: "bbbb".into(),
            files: vec![stikk_prikk::StateFile::file("readme.txt", 0o100_644)],
            total_bytes: 12,
        }),
        patches: BlockPatches::Unreported(30),
    };
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::BlockState {
            reff: "heads/main".into(),
            result: Ok(detail),
        },
    });
    match app.focus() {
        Focus::BlockDetail(detail, _) => {
            assert!(detail.is_tip);
            assert_eq!(detail.state.listed().map(|s| s.files.len()), Some(1));
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
        state: stikk_core::BlockState::Unreported(30),
        patches: BlockPatches::Unreported(30),
    };
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::BlockState {
            reff: "heads/main".into(),
            result: Ok(detail),
        },
    });
    match app.focus() {
        Focus::BlockDetail(detail, _) => {
            assert!(!detail.is_tip);
            // Below prikk 0.46 an older block's files are not reported (RFC 037 decision 2).
            assert!(detail.state.listed().is_none());
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
        kind: ResponseKind::Refs(Ok(ref_list(vec![
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
        ]))),
    });
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));

    app.nav_down(); // highlight tags/v1
    app.select(); // pick it
    assert!(!app.has_overlay());
    assert_eq!(app.focused_ref(), Some("tags/v1"));

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
        kind: ResponseKind::Changes(Ok(stikk_core::ChangesRead {
            view: dirty_changes(),
            history: stikk_core::RefHistory::Published,
        })),
    });
    match app.focus() {
        Focus::Changes(view, hide, _) => {
            assert!(!view.clean);
            assert_eq!(view.entries.len(), 2);
            assert!(!hide);
        }
        other => panic!("expected Changes, got {other:?}"),
    }
    app.toggle_untracked();
    match app.focus() {
        Focus::Changes(_, hide, _) => assert!(hide),
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
fn a_signing_readiness_not_ready_does_not_point_at_a_nonexistent_glossary_section() {
    // RFC 018 F2: this used to append "— see Glossary → Trust & Keys", a section that does not exist
    // (`Keys` is the keybinding table). Until `FR-104`'s Trust & Keys view exists to point at, the
    // banner states `detail` plainly and directs nobody anywhere.
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
    let banner = app.banner().unwrap();
    assert!(banner.contains("no signing key configured"));
    assert!(!banner.contains("Trust & Keys"));
    assert!(!banner.contains("Glossary"));
}

#[test]
fn reload_requests_a_fresh_orientation_and_apply_updates_it() {
    let (mut app, rx) = from_state("/repo", OrientationState::Loading, Palette::default());
    app.reload();
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(orientation_view(5, None, None)))),
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
        queued_target: None,
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
            queued_target: None,
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
            queued_target: None,
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
        capability: Some(Capability::Author),
        consequence: "Nothing real happens".to_string(),
        target_name: target_name.map(str::to_string),
        signing_key_id: None,
        signing_key_claim: stikk_core::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        message_note: None,
        materializes: None,
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
        summary: Box::new(confirmation_summary(Some("heads/main"))),
        tier: Tier::ThreeTyped,
        typed: String::new(),
        error: None,
    });
    assert!(app.wants_text_input());

    app.close_overlay();
    app.push_overlay(Overlay::Confirmation {
        summary: Box::new(confirmation_summary(None)),
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
        summary: Box::new(confirmation_summary(Some("heads/main"))),
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
        summary: Box::new(confirmation_summary(None)),
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
        summary: Box::new(confirmation_summary(Some("heads/main"))),
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
        summary: Box::new(confirmation_summary(None)),
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
        refused_declarations: None,
        reff: "heads/main".to_string(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        refused: None,
        entries: Vec::new(),
        queued_elsewhere: None,
        declarations: Vec::new(),
    }
}

fn commit_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 0,
            queued_target: None,
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
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
        Some(Overlay::CommitMessage { reff, typed, .. }) => {
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

fn seal_backend() -> NullBackend {
    NullBackend::supported()
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 1,
            queued_target: Some("heads/main".to_string()),
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
        })
        // Seal reads the queue for its blocks and count (RFC 028 decision 4).
        .with_queue(stikk_prikk::QueueReport::Unreported {
            count: 1,
            target: Some("heads/main".to_string()),
        })
}

#[test]
fn begin_seal_dispatches_a_preview_request_when_maintainer_ready() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::SealPreview { reff } if reff == "heads/main"));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading {
            what: "seal preview",
            ..
        })
    ));
}

#[test]
fn begin_seal_refuses_with_a_banner_when_not_maintainer_ready() {
    let (mut app, _rx) = from_state(
        "/repo",
        loaded(author_orientation_view()), // AUTHOR only — not enough for seal's tier 3
        Palette::default(),
    );
    app.begin_seal();
    assert!(!app.has_overlay(), "no preview should be requested");
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("needs MAINTAINER signing readiness"))
    );
}

#[test]
fn a_blocked_seal_preview_shows_a_banner_and_closes_the_overlay() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(0, None)), // empty queue
        Palette::default(),
    );
    app.begin_seal();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::SealPreview(Ok(SealPreviewOutcome::Blocked(
            "nothing is queued for this ref — there is nothing to seal".to_string(),
        ))),
    });
    assert!(!app.has_overlay());
    assert!(app.banner().is_some_and(|b| b.contains("nothing to seal")));
}

#[test]
fn a_ready_seal_preview_opens_the_confirmation_overlay_at_tier_three() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("a scripted backend's preview always succeeds");
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    match app.top_overlay() {
        Some(Overlay::Confirmation { summary, tier, .. }) => {
            assert_eq!(*tier, Tier::Three);
            assert!(summary.target_ids.contains(&"heads/main".to_string()));
        }
        other => panic!("expected Confirmation, got {other:?}"),
    }
}

/// The acceptance-critical sequencing test (RFC 016 §8): confirming does **not** dispatch
/// confirm+execute directly — it opens the consent step, and only the consent step's own `Enter`,
/// with `acknowledged` true, dispatches anything.
#[test]
fn confirming_a_seal_opens_consent_not_confirm_execute() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });

    app.select(); // the confirmation's own Enter — the ceremony's first act
    match app.top_overlay() {
        Some(Overlay::SealConsent { reff, acknowledged }) => {
            assert_eq!(reff, "heads/main");
            assert!(
                !acknowledged,
                "must start unacknowledged — cannot be defaulted"
            );
        }
        other => panic!("expected SealConsent, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "no confirm+execute request before the consent act"
    );
}

#[test]
fn an_unacknowledged_enter_on_consent_does_nothing() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select(); // -> SealConsent, unacknowledged
    app.select(); // Enter again, still unacknowledged
    assert!(
        matches!(app.top_overlay(), Some(Overlay::SealConsent { .. })),
        "the consent step must stay open"
    );
    assert!(
        rx.try_recv().is_err(),
        "an unacknowledged Enter must dispatch nothing — this is the whole point of the step"
    );
}

#[test]
fn toggling_then_confirming_consent_dispatches_confirm_execute() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select(); // -> SealConsent
    app.toggle_seal_consent();
    match app.top_overlay() {
        Some(Overlay::SealConsent { acknowledged, .. }) => assert!(*acknowledged),
        other => panic!("expected SealConsent, got {other:?}"),
    }
    app.select(); // consent's own Enter — the ceremony's second act
    let confirm_req = next_request(&rx);
    assert!(matches!(
        confirm_req.kind,
        RequestKind::SealConfirmExecute { .. }
    ));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading { what: "seal", .. })
    ));
}

#[test]
fn a_successful_seal_shows_the_result_verbatim_and_refreshes_orientation() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select();
    app.toggle_seal_consent();
    app.select();
    let confirm_req = next_request(&rx);

    let result = SealResult {
        patches: 1,
        block_id: "1".repeat(64),
        reff: "heads/main".to_string(),
        ref_state: "2".repeat(64),
        notes: vec!["note: audit plugins remain later PRs".to_string()],
    };
    app.apply(Response {
        seq: confirm_req.seq,
        kind: ResponseKind::SealConfirmExecute(Ok(Outcome {
            operation: "seal".to_string(),
            result: result.clone(),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::SealResult { result: shown }) => assert_eq!(shown, &result),
        other => panic!("expected SealResult, got {other:?}"),
    }
    let orient_req = next_request(&rx);
    assert!(matches!(orient_req.kind, RequestKind::Orient));
}

#[test]
fn a_trust_refusal_mid_ceremony_surfaces_with_adoption_guidance() {
    // RFC 016 §9 (corrected in review v2, C1): the trust refusal must reach the ceremony as `NotReady`
    // with adoption guidance in a `RefusalOverlay` it can actually be read in — not a `NotReady` bare
    // detail, and not the cramped one-line banner v1 shipped, which cut the guidance off entirely.
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select();
    app.toggle_seal_consent();
    app.select();
    let confirm_req = next_request(&rx);
    app.apply(Response {
        seq: confirm_req.seq,
        kind: ResponseKind::SealConfirmExecute(Err(StikkError::NotReady {
            detail: "invalid signature: maintainer signer key id x is not trusted by policy"
                .to_string(),
        })),
    });
    assert!(app.banner().is_none());
    match app.top_overlay() {
        Some(Overlay::Refusal { card, .. }) => {
            assert!(card.verbatim.contains("is not trusted by policy"));
            assert!(card.gloss.as_deref().is_some_and(|g| g.contains("adopted")));
        }
        other => panic!("expected a Refusal-shaped overlay for the trust refusal, got {other:?}"),
    }
}

#[test]
fn back_on_the_consent_step_clears_the_pending_seal() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(maintainer_orientation_view(1, Some("heads/main"))),
        Palette::default(),
    );
    app.begin_seal();
    let preview_req = next_request(&rx);
    let backend = seal_backend();
    let outcome = seal_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select(); // -> SealConsent
    app.back();
    assert!(!app.has_overlay());
    // Re-opening must still work — nothing from the abandoned attempt lingers to interfere.
    app.begin_seal();
    let second_req = next_request(&rx);
    assert!(matches!(second_req.kind, RequestKind::SealPreview { .. }));
}

#[test]
fn a_would_refuse_commit_preview_opens_an_overlay_not_a_banner() {
    // RFC 027 decision 5: the refused entries need a path and a reason each — not the one-line banner,
    // and not `present()`, since nothing was attempted.
    let (mut app, rx) = from_state(
        "/repo",
        loaded(author_orientation_view()),
        Palette::default(),
    );
    app.begin_commit();
    app.input_char('x');
    app.select();
    let req = next_request(&rx);
    let paths = vec![stikk_core::RefusedPath {
        kind: ChangeKind::Untracked,
        path: "link.txt".into(),
        reason: "precondition not met: link.txt: worktree symlink authoring is out of scope".into(),
    }];
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::CommitPreview(Ok(CommitPreviewOutcome::WouldRefuse(paths.clone()))),
    });
    assert!(
        matches!(app.top_overlay(), Some(Overlay::CommitWouldRefuse { paths: shown }) if *shown == paths),
        "expected the would-refuse overlay, got {:?}",
        app.top_overlay()
    );
    assert!(app.banner().is_none(), "not the banner");
    // Nothing to confirm: dismissing it leaves no pending commit and no overlay.
    app.back();
    assert!(!app.has_overlay());
}

// ---------------------------------------------------------------------------------------------
// RFC 029 Handoff B §2 and §4: where focus starts, and what happens without one.
// ---------------------------------------------------------------------------------------------

use stikk_model::{CurrentBranch, RefName};

/// One of `App`'s user-facing actions, as a key or the palette reaches it.
type AppAction = fn(&mut App);

fn branch(name: &str) -> CurrentBranch {
    CurrentBranch::Branch(RefName::parse(name).expect("a valid ref name"))
}

/// An author- and maintainer-ready view (so the capability check passes and only focus is under test),
/// with prikk's current branch and whether `heads/main` is published.
fn view_on(current: CurrentBranch, main_published: bool) -> stikk_core::OrientationView {
    let mut view = maintainer_orientation_view(0, None);
    view.current_branch = current;
    if !main_published {
        view.main_ref_state = None;
    }
    view
}

/// Open an app and answer its first Orientation read with `result`.
fn open_with_first_read(
    result: stikk_model::Result<stikk_core::OrientationView>,
) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = open("/repo", &Config::default());
    assert_eq!(app.ref_focus(), &RefFocus::Pending);
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(result)),
    });
    (app, rx)
}

fn answer_reload(app: &mut App, rx: &mpsc::Receiver<Request>, view: stikk_core::OrientationView) {
    app.reload();
    let req = next_request(rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(view))),
    });
}

/// The status bar at 120 columns: wide enough that nothing is shed, so these tests see prikk's text
/// whole. What the line sheds when it is full is `status_bar::tests`' subject.
fn status_line(app: &App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 1)).unwrap();
    terminal
        .draw(|f| crate::status_bar::render(app, f, f.area()))
        .unwrap();
    crate::test_util::buffer_text(terminal.backend().buffer())
}

fn top_overlay_text(app: &App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|f| {
            crate::overlay::render(
                app.top_overlay().unwrap(),
                app.marked_ref(),
                app.palette(),
                f,
                f.area(),
            )
        })
        .unwrap();
    crate::test_util::buffer_text(terminal.backend().buffer())
}

/// A `RefList` with no unpublished current branch reported — the shape every test here used
/// before RFC 037, so each one keeps asserting exactly what it asserted.
fn ref_list(refs: Vec<stikk_prikk::RefEntry>) -> stikk_core::RefList {
    stikk_core::RefList {
        refs,
        unpublished_current: stikk_prikk::UnpublishedBranch::NotReported,
    }
}

fn ref_entry(name: &str) -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: name.into(),
        id: "x".into(),
        closed: false,
        received: false,
    }
}

#[test]
fn the_first_read_focuses_the_branch_prikk_names() {
    let (app, rx) = open_with_first_read(Ok(view_on(branch("heads/dev"), true)));
    assert_eq!(app.focused_ref(), Some("heads/dev"));
    assert!(
        rx.try_recv().is_err(),
        "a branch prikk names opens no picker"
    );
    assert!(!app.has_overlay());
}

#[test]
fn a_branch_prikk_names_is_followed_even_when_unpublished() {
    // A fresh 0.42 repository: `current branch: heads/main` beside `heads/main RefState: <not published>`.
    let (app, rx) = open_with_first_read(Ok(view_on(branch("heads/main"), false)));
    assert_eq!(app.focused_ref(), Some("heads/main"));
    assert!(
        rx.try_recv().is_err(),
        "no fallback when prikk names a branch"
    );
}

#[test]
fn a_later_read_never_moves_focus_and_the_status_bar_names_prikks_default() {
    let (mut app, rx) = open_with_first_read(Ok(view_on(branch("heads/dev"), true)));
    answer_reload(&mut app, &rx, view_on(branch("heads/x"), true));
    assert_eq!(
        app.focused_ref(),
        Some("heads/dev"),
        "`r` never moves focus"
    );
    let line = status_line(&app);
    assert!(line.contains("heads/dev"), "{line:?}");
    assert!(line.contains("prikk's default: heads/x"), "{line:?}");
    assert!(!line.contains("HEAD"), "{line:?}");
}

#[test]
fn not_reported_focuses_heads_main_only_when_published_and_otherwise_opens_the_picker() {
    let (app, rx) = open_with_first_read(Ok(view_on(CurrentBranch::NotReported, true)));
    assert_eq!(app.focused_ref(), Some("heads/main"));
    assert!(rx.try_recv().is_err());

    let (app, rx) = open_with_first_read(Ok(view_on(CurrentBranch::NotReported, false)));
    assert_eq!(app.ref_focus(), &RefFocus::Unfocused);
    assert!(matches!(next_request(&rx).kind, RequestKind::Refs));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading { what: "refs", .. })
    ));
}

#[test]
fn unresolved_takes_the_same_fallback_and_shows_prikks_text_verbatim() {
    let unresolved = || CurrentBranch::Unresolved("<unresolved; run `prikk doctor`>".to_string());
    let (app, rx) = open_with_first_read(Ok(view_on(unresolved(), true)));
    assert_eq!(app.focused_ref(), Some("heads/main"));
    assert!(rx.try_recv().is_err());
    let line = status_line(&app);
    assert!(
        line.contains("heads/main  ·  prikk's default: <unresolved; run `prikk doctor`>"),
        "{line:?}"
    );
    assert!(!line.contains("HEAD"), "{line:?}");

    let (app, rx) = open_with_first_read(Ok(view_on(unresolved(), false)));
    assert_eq!(app.ref_focus(), &RefFocus::Unfocused);
    assert!(matches!(next_request(&rx).kind, RequestKind::Refs));
}

#[test]
fn a_pick_made_while_pending_survives_the_first_read() {
    let (mut app, rx) = open("/repo", &Config::default());
    let orient_req = next_request(&rx);
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(vec![ref_entry("tags/v1")]))),
    });
    app.select();
    assert_eq!(app.focused_ref(), Some("tags/v1"));
    assert!(matches!(
        next_request(&rx).kind,
        RequestKind::History { .. }
    ));

    app.apply(Response {
        seq: orient_req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(view_on(branch("heads/dev"), true)))),
    });
    assert_eq!(
        app.focused_ref(),
        Some("tags/v1"),
        "the first read resolves nothing"
    );
}

#[test]
fn a_failed_first_read_leaves_focus_pending_and_a_later_success_resolves_it() {
    let (mut app, rx) = open_with_first_read(Err(StikkError::Refusal {
        message: "repository is locked".into(),
    }));
    assert_eq!(app.ref_focus(), &RefFocus::Pending);
    assert!(rx.try_recv().is_err());
    answer_reload(&mut app, &rx, view_on(branch("heads/dev"), true));
    assert_eq!(app.focused_ref(), Some("heads/dev"));
}

#[test]
fn without_a_focused_ref_every_focus_needing_action_dispatches_nothing_and_says_why() {
    let (mut app, rx) = open_with_first_read(Ok(view_on(CurrentBranch::NotReported, false)));
    assert!(matches!(next_request(&rx).kind, RequestKind::Refs));
    app.back(); // the picker's loading placeholder: escaping it leaves focus unfocused
    assert_eq!(app.ref_focus(), &RefFocus::Unfocused);

    let actions: [(&str, AppAction); 5] = [
        ("History (the Enter at the root)", App::select),
        ("History (the palette's target)", App::open_history),
        ("Changes (w)", App::open_changes),
        ("commit (C)", App::begin_commit),
        ("seal (S)", App::begin_seal),
    ];
    for (label, act) in actions {
        act(&mut app);
        assert_eq!(
            app.banner(),
            Some("No ref focused. Press b to pick one."),
            "{label}"
        );
        assert!(rx.try_recv().is_err(), "{label} dispatched a request");
        assert!(!app.has_overlay(), "{label} opened an overlay");
        assert!(matches!(app.focus(), Focus::Orientation(_)), "{label}");
        app.back(); // dismiss the banner
    }

    app.open_palette();
    let Some(Overlay::Palette {
        readiness,
        ref_focused,
        ..
    }) = app.top_overlay()
    else {
        panic!("expected the palette");
    };
    let (readiness, ref_focused) = (*readiness, *ref_focused);
    assert!(!ref_focused);
    for command in stikk_core::palette::commands() {
        let expected = matches!(
            command.id,
            "view.history" | "view.changes" | "op.commit" | "op.seal"
        )
        .then_some("No ref focused. Press b to pick one.");
        assert_eq!(
            command
                .unavailable_reason(readiness, ref_focused)
                .as_deref(),
            expected,
            "{}",
            command.id
        );
    }
    // And the palette's Enter runs nothing it lists as disabled.
    for ch in "his".chars() {
        app.input_char(ch);
    }
    app.select();
    assert!(rx.try_recv().is_err(), "the palette ran a disabled command");
    assert!(matches!(app.top_overlay(), Some(Overlay::Palette { .. })));
}

#[test]
fn an_empty_ref_list_offers_an_unpublished_heads_main_and_picking_it_focuses_it() {
    let (mut app, rx) = open_with_first_read(Ok(view_on(CurrentBranch::NotReported, false)));
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(Vec::new()))),
    });
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::RefPicker {
            unpublished_main: true,
            ..
        })
    ));
    let text = top_overlay_text(&app);
    println!("--- empty ref picker, 80×24\n{text}");
    assert!(text.contains("no published refs"), "{text}");
    assert!(text.contains("▶ heads/main (not published)"), "{text}");

    app.nav_down(); // one row: stays on it
    app.select();
    assert_eq!(app.focused_ref(), Some("heads/main"));
    assert!(!app.has_overlay());
    // Like any other pick (review v1 §2.3): prikk reports an unpublished ref's history as empty.
    match next_request(&rx).kind {
        RequestKind::History { reff } => assert_eq!(reff, "heads/main"),
        other => panic!("expected a History request, got {other:?}"),
    }
    assert!(matches!(app.focus(), Focus::Loading("history")));
}

#[test]
fn a_non_empty_ref_list_offers_no_unpublished_row() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(vec![ref_entry("heads/dev")]))),
    });
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::RefPicker {
            unpublished_main: false,
            ..
        })
    ));
    let text = top_overlay_text(&app);
    assert!(text.contains("heads/dev"), "{text}");
    assert!(!text.contains("not published"), "{text}");
    assert!(!text.contains("no published refs"), "{text}");
}

// ---------------------------------------------------------------------------------------------
// RFC 028 Handoff A §6: the Queue screen.
// ---------------------------------------------------------------------------------------------

fn queue_view_with(heading: &str) -> stikk_core::QueueView {
    stikk_core::QueueView {
        heading: heading.to_string(),
        thresholds: Vec::new(),
        // Four patches, so the selection RFC 035 §4.2 added has somewhere to move.
        patches: (1..=4)
            .map(|i| stikk_core::QueuedPatchView {
                id: format!("{i:064x}"),
                message: Some(format!("patch {i}")),
                detail_message: stikk_core::PatchMessageState::Text(format!("patch {i}")),
                operations: vec![format!("create-file p{i}.txt")],
            })
            .collect(),
        foot: None,
    }
}

#[test]
fn the_queue_opens_without_a_focused_ref() {
    // Unfocused: the first read named no branch and nothing is published.
    let (mut app, rx) = open_with_first_read(Ok(view_on(CurrentBranch::NotReported, false)));
    assert!(matches!(next_request(&rx).kind, RequestKind::Refs));
    app.back();
    assert_eq!(app.focused_ref(), None);

    app.open_queue();
    assert_eq!(app.banner(), None, "the queue needs no focused ref");
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Queue));
    assert!(matches!(app.focus(), Focus::Loading("queue")));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("Nothing is queued."))),
    });
    match app.focus() {
        Focus::Queue(view, _, _) => assert_eq!(view.heading, "Nothing is queued."),
        other => panic!("expected the Queue view, got {other:?}"),
    }

    // The palette lists it available with no focused ref.
    let queue = stikk_core::palette::commands()
        .iter()
        .find(|c| c.id == "view.queue")
        .expect("view.queue is registered");
    assert_eq!(queue.binding, "Q");
    assert_eq!(queue.unavailable_reason(Readiness::none(), false), None);
}

#[test]
fn refreshing_the_queue_keeps_its_view_visible_until_the_new_one_arrives() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_queue();
    let first = next_request(&rx);
    app.apply(Response {
        seq: first.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("1 patch(es) queued for heads/main"))),
    });

    app.reload();
    let mut requests = vec![next_request(&rx), next_request(&rx)];
    requests.sort_by_key(|r| matches!(r.kind, RequestKind::Queue));
    let refresh = requests.pop().unwrap();
    assert!(matches!(refresh.kind, RequestKind::Queue));
    match app.focus() {
        Focus::Queue(view, _, _) => assert_eq!(view.heading, "1 patch(es) queued for heads/main"),
        other => panic!("the old view stays while refreshing, got {other:?}"),
    }
    app.apply(Response {
        seq: refresh.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("2 patch(es) queued for heads/main"))),
    });
    match app.focus() {
        Focus::Queue(view, _, _) => assert_eq!(view.heading, "2 patch(es) queued for heads/main"),
        other => panic!("expected the refreshed Queue view, got {other:?}"),
    }
}

#[test]
fn up_and_down_move_the_queues_selection_and_a_refresh_keeps_it() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_queue();
    let first = next_request(&rx);
    app.apply(Response {
        seq: first.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("12 patch(es) queued for heads/main"))),
    });
    // RFC 035 §4.2: `↑`/`↓` move the **selection** now, and the renderer scrolls to follow it. The
    // offset is no longer something a key sets directly, so it stays where the last render left it.
    let cursor = |app: &App| match app.focus() {
        Focus::Queue(_, _, cursor) => cursor,
        other => panic!("expected the Queue view, got {other:?}"),
    };
    assert_eq!(cursor(&app), 0);
    app.nav_down();
    app.nav_down();
    app.nav_down();
    app.nav_up();
    assert_eq!(
        cursor(&app),
        2,
        "three down, one up, over the listed patches"
    );

    app.reload();
    let mut requests = vec![next_request(&rx), next_request(&rx)];
    requests.sort_by_key(|r| matches!(r.kind, RequestKind::Queue));
    let refresh = requests.pop().unwrap();
    app.apply(Response {
        seq: refresh.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("13 patch(es) queued for heads/main"))),
    });
    assert_eq!(
        cursor(&app),
        2,
        "a refresh keeps the selection; the next render scrolls to it"
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 035 — Patch detail: where it opens from, what it spawns, and what `v` and `r` do.
// ---------------------------------------------------------------------------------------------

const RFC035_PATCH_ONE: &str = "4db1669a176b64136838975ef7e2724e2161ad78b89c6dabc4efa24cd6818f66";
const RFC035_PATCH_TWO: &str = "c23a5d7d4d5d639b3e41e76806158e4bc475f113f3ed85858501010c2d830999";

fn rfc035_shown(id: &str, path: &str) -> stikk_prikk::ShownPatch {
    stikk_prikk::ShownPatch {
        patch_id: id.to_string(),
        queued: false,
        operations: vec![stikk_prikk::PatchOperation {
            kind: "edit-text".to_string(),
            paths: vec![stikk_prikk::QueuedPath::Path(path.to_string())],
            content: stikk_prikk::OperationContent::TextEdit {
                old_span: String::new(),
                replacement: "more\n".to_string(),
            },
            raw: "{\n  \"kind\": \"edit-text\"\n}".to_string(),
        }],
    }
}

/// A Block detail whose report holds two patches, as prikk answers `show <block-id>` (A1 F7).
fn rfc035_block_detail() -> BlockDetailView {
    let mut row = block("bbbb", 2);
    row.patches = 2;
    row.messages = vec![stikk_prikk::PatchMessage {
        patch_id: RFC035_PATCH_TWO.to_string(),
        message: "patch two".to_string(),
    }];
    BlockDetailView {
        row,
        is_tip: true,
        state: stikk_core::BlockState::Unreported(30),
        patches: BlockPatches::Listed(vec![
            stikk_core::BlockPatchRow {
                patch: rfc035_shown(RFC035_PATCH_ONE, "z.txt"),
                message: stikk_core::PatchMessageState::NotReported,
            },
            stikk_core::BlockPatchRow {
                patch: rfc035_shown(RFC035_PATCH_TWO, "z.txt"),
                message: stikk_core::PatchMessageState::Text("patch two".to_string()),
            },
        ]),
    }
}

fn rfc035_on_block_detail() -> (App, mpsc::Receiver<Request>) {
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
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::BlockState {
            reff: "heads/main".into(),
            result: Ok(rfc035_block_detail()),
        },
    });
    (app, rx)
}

/// **Opening a patch from Block detail spawns nothing** (RFC 035 decision 6, §5 trap 7): the block's
/// report is already in hand, and Patch detail renders one patch from it.
///
/// It also proves decision 3a's label is on screen, because the block holds two patches — and that the
/// patch stikk opens is the one the cursor marks.
#[test]
fn opening_a_patch_from_block_detail_spawns_nothing() {
    let (mut app, rx) = rfc035_on_block_detail();
    app.nav_down(); // the second patch
    app.select();
    assert!(
        rx.try_recv().is_err(),
        "no seam request may be sent: the block's report is already read"
    );
    match app.focus() {
        Focus::PatchDetail(view, raw, _) => {
            assert_eq!(view.patch_id, RFC035_PATCH_TWO);
            assert!(!raw, "the rendered view first");
            assert_eq!(view.heading[1], "patch two", "joined by id");
            assert_eq!(
                stikk_core::flatten(&view.operations[0].title),
                format!("edit-text z.txt — {}", stikk_core::AS_AT_BLOCK_END),
                "two patches in the block, so the path is labelled"
            );
        }
        other => panic!("expected Patch detail, got {other:?}"),
    }
}

/// `v` toggles the raw view in place — **no request, either way** (decision 5).
#[test]
fn v_toggles_the_raw_view_without_a_second_read() {
    let (mut app, rx) = rfc035_on_block_detail();
    app.select();
    let raw_now = |app: &App| match app.focus() {
        Focus::PatchDetail(_, raw, _) => raw,
        other => panic!("expected Patch detail, got {other:?}"),
    };
    assert!(!raw_now(&app));
    app.toggle_raw();
    assert!(raw_now(&app));
    app.toggle_raw();
    assert!(!raw_now(&app));
    assert!(rx.try_recv().is_err(), "a toggle is never a read");
}

/// `v` is a no-op anywhere else, as `u` is outside Changes.
#[test]
fn v_does_nothing_outside_patch_detail() {
    let (mut app, rx) = rfc035_on_block_detail();
    app.toggle_raw();
    assert!(matches!(app.focus(), Focus::BlockDetail(_, _)));
    assert!(rx.try_recv().is_err());
}

/// **`r` on a patch opened from a block re-reads Orientation and nothing else**: a sealed block's
/// patches are fixed, the same reason an older block's detail is never refreshed (RFC 031 §7).
#[test]
fn r_on_a_block_opened_patch_re_reads_only_orientation() {
    let (mut app, rx) = rfc035_on_block_detail();
    app.select();
    app.reload();
    let first = next_request(&rx);
    assert!(
        matches!(first.kind, RequestKind::Orient),
        "{:?}",
        first.kind
    );
    assert!(
        rx.try_recv().is_err(),
        "nothing else: the block's patches cannot change"
    );
}

/// **Opening a patch from the Queue is one `show <patch-id>` read** (§4.2), and `r` re-reads that same
/// id — which is how a patch sealed while the screen was open comes back as prikk now reports it
/// (§4.3, A1 F7 row 2).
#[test]
fn opening_a_patch_from_the_queue_reads_once_and_r_re_reads_the_same_id() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_queue();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Queue(Ok(queue_view_with("4 patch(es) queued for heads/main"))),
    });
    app.nav_down(); // the second queued patch
    app.select();
    let req = next_request(&rx);
    let RequestKind::PatchDetail { patch_id, message } = &req.kind else {
        panic!("expected one PatchDetail request, got {:?}", req.kind);
    };
    assert_eq!(patch_id, &format!("{:064x}", 2));
    assert_eq!(
        message,
        &stikk_core::PatchMessageState::Text("patch 2".to_string()),
        "the message the Queue already read, carried across rather than re-read"
    );
    assert!(rx.try_recv().is_err(), "exactly one read");

    let queued = stikk_prikk::ShownPatch {
        patch_id: patch_id.clone(),
        queued: true,
        operations: vec![stikk_prikk::PatchOperation {
            kind: "edit-text".to_string(),
            paths: vec![stikk_prikk::QueuedPath::Path("a.txt".to_string())],
            content: stikk_prikk::OperationContent::TextEdit {
                old_span: String::new(),
                replacement: "more\n".to_string(),
            },
            raw: "{}".to_string(),
        }],
    };
    let view = stikk_core::patch_detail_in_block(
        &queued,
        "unused",
        &stikk_core::PatchMessageState::Text("patch 2".to_string()),
        1,
        Some(51),
    );
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::PatchDetail(Ok(view)),
    });
    match app.focus() {
        Focus::PatchDetail(view, _, _) => assert_eq!(view.heading[0], "queued"),
        other => panic!("expected Patch detail, got {other:?}"),
    }

    // `r`: Orientation, and the same patch id again.
    app.reload();
    let mut kinds: Vec<RequestKind> = vec![next_request(&rx).kind, next_request(&rx).kind];
    assert!(rx.try_recv().is_err(), "two requests, no more");
    kinds.sort_by_key(|kind| matches!(kind, RequestKind::PatchDetail { .. }));
    let RequestKind::PatchDetail {
        patch_id: again, ..
    } = &kinds[1]
    else {
        panic!("expected the same patch id to be re-read, got {kinds:?}");
    };
    assert_eq!(again, patch_id);
}

mod change_awareness;

/// **The ref picker offers the branch you are on before its first seal** (RFC 037 decision 4), as
/// its own row, first, and selecting it focuses that ref like any other — which is the whole point:
/// a brand-new repository can be opened, committed in and sealed without the picker pretending the
/// branch does not exist.
#[test]
fn the_picker_offers_the_unpublished_current_branch_and_selecting_it_focuses_that_ref() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(stikk_core::RefList {
            refs: Vec::new(),
            unpublished_current: stikk_prikk::UnpublishedBranch::Branch("heads/main".into()),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::RefPicker {
            refs,
            unpublished_current,
            unpublished_main,
            ..
        }) => {
            assert_eq!(refs, &["heads/main".to_string()]);
            assert!(unpublished_current, "the row is marked as the branch");
            assert!(
                !unpublished_main,
                "and RFC 029's guess is off, because prikk answered"
            );
        }
        other => panic!("expected a RefPicker, got {other:?}"),
    }
    app.select();
    assert!(!app.has_overlay());
    assert_eq!(app.focused_ref(), Some("heads/main"));
}

/// **The branch goes first, above the published refs**, so in a repository that has both, the row
/// for the branch you are standing on is the one at the top.
#[test]
fn the_unpublished_branch_is_the_first_row() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(stikk_core::RefList {
            refs: vec![ref_entry("heads/dev"), ref_entry("tags/v1")],
            unpublished_current: stikk_prikk::UnpublishedBranch::Branch("heads/new".into()),
        })),
    });
    match app.top_overlay() {
        Some(Overlay::RefPicker { refs, .. }) => {
            assert_eq!(refs, &["heads/new", "heads/dev", "tags/v1"]);
        }
        other => panic!("expected a RefPicker, got {other:?}"),
    }
}

/// **Where prikk reports none, nothing says "none"** — the picker is exactly as it is today, and
/// RFC 029's `heads/main (not published)` guess does **not** appear, because prikk has just denied
/// it (Handoff B §3).
#[test]
fn a_reported_none_offers_no_row_and_silences_the_older_guess() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(stikk_core::RefList {
            refs: Vec::new(),
            unpublished_current: stikk_prikk::UnpublishedBranch::None,
        })),
    });
    match app.top_overlay() {
        Some(Overlay::RefPicker {
            refs,
            unpublished_current,
            unpublished_main,
            ..
        }) => {
            assert!(refs.is_empty());
            assert!(!unpublished_current);
            assert!(
                !unpublished_main,
                "stikk does not guess at a branch prikk just said is not there"
            );
        }
        other => panic!("expected a RefPicker, got {other:?}"),
    }
}

/// **Below the band the picker is exactly as it was**, RFC 029's one-row fallback included: that
/// row exists because prikk would not answer, and below 0.47 it still will not.
#[test]
fn below_the_band_the_picker_is_unchanged() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(Vec::new()))),
    });
    match app.top_overlay() {
        Some(Overlay::RefPicker {
            unpublished_main,
            unpublished_current,
            ..
        }) => {
            assert!(unpublished_main, "RFC 029 Handoff B §4's one row");
            assert!(!unpublished_current);
        }
        other => panic!("expected a RefPicker, got {other:?}"),
    }
}

/// Open History with the two-block lineage loaded, for the Compare entry-point tests.
fn history_loaded() -> (App, std::sync::mpsc::Receiver<Request>) {
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
    (app, rx)
}

/// **With nothing marked, `d` compares a block with the previous block in this ref's history** —
/// the next older row in the loaded lineage (RFC 038 decision 3). Older is the left side.
#[test]
fn d_with_nothing_marked_compares_with_the_previous_block_in_the_lineage() {
    let (mut app, rx) = history_loaded();
    app.compare(); // cursor 0 = the tip, `bbbb`; the previous row is `aaaa`
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, opened } = &req.kind else {
        panic!("expected a Compare request, got {:?}", req.kind)
    };
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::Points {
            from: "aaaa".into(),
            to: "bbbb".into()
        }
    );
    assert_eq!(opened, &CompareSource::PreviousInHistory);
}

/// **On the oldest loaded block `d` is unavailable, with stikk's reason, and nothing is fetched**
/// (Handoff A §3). More history might hold a previous block; reading it behind a keypress that
/// meant "compare" would be stikk doing something other than it was asked.
#[test]
fn d_on_the_oldest_loaded_block_says_why_and_reads_nothing() {
    let (mut app, rx) = history_loaded();
    app.nav_down(); // the root block, the oldest row loaded
    app.compare();
    assert!(
        rx.try_recv().is_err(),
        "no request is sent for a comparison stikk cannot make"
    );
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("the previous block is not in the loaded history")),
        "{:?}",
        app.banner()
    );
}

/// **`m` marks, `d` compares the pair, and the order is older → newer whichever was marked first**
/// (§7 trap 5) — by prikk's own `update_seq`, not by the row index.
#[test]
fn a_marked_pair_is_ordered_older_to_newer_whichever_was_marked_first() {
    // Mark the tip (newer), then select the root (older).
    let (mut app, rx) = history_loaded();
    app.mark_block();
    app.nav_down();
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, opened } = &req.kind else {
        panic!("expected a Compare request")
    };
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::Points {
            from: "aaaa".into(),
            to: "bbbb".into()
        },
        "the older block is the left side even though the newer was marked first"
    );
    assert_eq!(opened, &CompareSource::MarkedPair);

    // And the other way round: mark the root (older), select the tip (newer).
    let (mut app, rx) = history_loaded();
    app.nav_down();
    app.mark_block();
    app.nav_up();
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, .. } = &req.kind else {
        panic!("expected a Compare request")
    };
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::Points {
            from: "aaaa".into(),
            to: "bbbb".into()
        }
    );
}

/// `m` on the marked block clears the mark, and `d` then falls back to the previous block.
#[test]
fn marking_the_same_block_twice_clears_the_mark() {
    let (mut app, rx) = history_loaded();
    app.mark_block();
    assert!(app.marked_block().is_some());
    app.mark_block();
    assert!(app.marked_block().is_none());
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { opened, .. } = &req.kind else {
        panic!("expected a Compare request")
    };
    assert_eq!(opened, &CompareSource::PreviousInHistory);
}

/// **Marking and selecting the same row is a comparison of a block with itself**, which prikk
/// answers as "no differences" like any other — it is not special-cased (Handoff A §3).
#[test]
fn comparing_a_block_with_itself_is_sent_to_prikk_like_any_other() {
    let (mut app, rx) = history_loaded();
    app.mark_block();
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, .. } = &req.kind else {
        panic!("expected a Compare request")
    };
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::Points {
            from: "bbbb".into(),
            to: "bbbb".into()
        }
    );
}

/// **A mark made in another lineage is not a pair.** The ref picker can move between the two
/// presses, and `m` never offered a comparison across refs.
#[test]
fn a_mark_from_another_ref_is_not_used_as_a_pair() {
    let (mut app, rx) = history_loaded();
    app.mark_block();
    // Another ref's lineage on top — what the ref picker does between the two presses.
    app.push_screen(Screen::History {
        view: HistoryView {
            reff: "heads/other".into(),
            queued: 0,
            queued_target: None,
            blocks: vec![block("dddd", 2), block("cccc", 1)],
        },
        cursor: 0,
        refreshing: None,
    });
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, opened } = &req.kind else {
        panic!("expected a Compare request")
    };
    assert_eq!(opened, &CompareSource::PreviousInHistory, "not the pair");
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::Points {
            from: "cccc".into(),
            to: "dddd".into()
        }
    );
}

/// **From Changes, `d` is the bare `diff`**: the worktree against what `commit` would author
/// against.
#[test]
fn d_from_changes_compares_the_worktree_with_its_baseline() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_changes();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Ok(stikk_core::ChangesRead {
            view: dirty_changes(),
            history: stikk_core::RefHistory::Published,
        })),
    });
    app.compare();
    let req = next_request(&rx);
    let RequestKind::Compare { comparison, opened } = &req.kind else {
        panic!("expected a Compare request, got {:?}", req.kind)
    };
    assert_eq!(
        comparison,
        &stikk_prikk::Comparison::WorktreeAgainstBaseline
    );
    assert_eq!(opened, &CompareSource::WorktreeAgainstBaseline);
}

// ---------------------------------------------------------------------------------------------
// RFC 041: compare two refs, from the ref picker.
// ---------------------------------------------------------------------------------------------

/// A picker holding three refs, with the cursor on the first.
fn picker_with_refs(names: &[&str]) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(names.iter().copied().map(ref_entry).collect()))),
    });
    (app, rx)
}

#[test]
fn m_marks_a_ref_and_a_second_m_on_the_same_row_unmarks_it() {
    let (mut app, _rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    assert_eq!(app.marked_ref(), None);

    app.mark_block(); // `m` — the picker is open, so it marks a ref
    assert_eq!(app.marked_ref(), Some("heads/main"));
    let text = top_overlay_text(&app);
    println!("--- RFC 041: the ref picker with a mark set, 80×24\n{text}");
    // The mark is on its row, and the title says one is set (the row can scroll out of sight).
    assert!(text.contains("● heads/main"), "{text}");
    assert!(text.contains("● marked"), "{text}");

    app.mark_block(); // `m` again on the same row
    assert_eq!(app.marked_ref(), None);
    let text = top_overlay_text(&app);
    assert!(!text.contains("● heads/main"), "{text}");
    assert!(text.contains("m marks a ref"), "{text}");
}

#[test]
fn m_on_another_row_moves_the_mark_rather_than_adding_one() {
    let (mut app, _rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    app.mark_block();
    app.nav_down();
    app.mark_block();
    assert_eq!(app.marked_ref(), Some("heads/feature"));
    let text = top_overlay_text(&app);
    // Exactly one row carries the mark.
    assert_eq!(
        text.lines().filter(|l| l.contains('●')).count(),
        // The row, plus the title's own `● marked`.
        2,
        "{text}"
    );
    assert!(text.contains("● heads/feature"), "{text}");
}

#[test]
fn d_with_a_mark_compares_the_marked_ref_with_the_selected_one() {
    let (mut app, rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    app.mark_block(); // mark heads/main
    app.nav_down(); // select heads/feature
    app.compare();

    // The picker is closed and the comparison is on its way.
    assert!(!app.has_overlay());
    match next_request(&rx).kind {
        RequestKind::Compare { comparison, opened } => {
            // **Marked is `from`, selected is `to`** (RFC 041 decision 2) — the reader's order,
            // not a lineage's.
            assert_eq!(
                comparison,
                stikk_prikk::Comparison::Points {
                    from: "heads/main".into(),
                    to: "heads/feature".into(),
                }
            );
            assert_eq!(
                opened,
                CompareSource::RefPair {
                    from: "heads/main".into(),
                    to: "heads/feature".into(),
                }
            );
        }
        other => panic!("expected a Compare request, got {other:?}"),
    }
    assert!(matches!(app.focus(), Focus::Loading("compare")));
}

#[test]
fn d_with_no_mark_says_to_mark_one_and_sends_nothing() {
    let (mut app, rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    app.compare();
    // The picker stays open, so the reader can act on what the banner says.
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));
    assert_eq!(app.banner(), Some("mark a ref with m first"));
    assert!(rx.try_recv().is_err(), "nothing should have been sent");
}

#[test]
fn the_unpublished_current_branch_refuses_both_keys_and_points_at_changes() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    // The branch prikk named is **prepended** by `apply_refs`, so it is not in `refs` here.
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(stikk_core::RefList {
            refs: vec![ref_entry("heads/main")],
            unpublished_current: stikk_prikk::UnpublishedBranch::Branch("heads/new".into()),
        })),
    });
    // Land on the unpublished row, wherever the focused ref put the cursor.
    app.nav_up();
    app.nav_up();

    app.mark_block();
    assert_eq!(app.marked_ref(), None, "it must not become a side");
    let banner = app.banner().expect("a reason in stikk's own words");
    println!("--- RFC 041: the unpublished row's sentence\n{banner}");
    assert!(
        banner.contains("no published history to compare yet"),
        "{banner}"
    );
    // **It points at the comparison that branch does have** (RFC 041 decision 4).
    assert!(banner.contains("worktree"), "{banner}");
    // Never prikk's words: measured at 0.51, prikk says the ref "does not exist", which is false
    // and alarming for a branch the reader is standing on.
    assert!(!banner.contains("does not exist"), "{banner}");

    app.compare();
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));
    assert!(rx.try_recv().is_err(), "nothing should have been sent");

    // And it cannot be the **selected** side of a pair whose other half is fine either.
    app.nav_down(); // heads/main
    app.mark_block();
    assert_eq!(app.marked_ref(), Some("heads/main"));
    app.nav_up(); // back onto the unpublished row, as the selected side
    app.compare();
    assert!(matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })));
    assert!(rx.try_recv().is_err(), "nothing should have been sent");
}

#[test]
fn the_ref_mark_survives_reopening_the_picker_and_dies_on_a_ref_change() {
    // **The lifetime I chose** (RFC 041 decision 1's *"until the next ref change"*): a mark
    // survives closing and reopening, because the picker is where both halves of a pair are
    // chosen and a mark that died on `Esc` could never be paired with anything.
    let (mut app, rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    app.mark_block();
    assert_eq!(app.marked_ref(), Some("heads/main"));

    app.back(); // close the picker
    assert!(!app.has_overlay());
    assert_eq!(
        app.marked_ref(),
        Some("heads/main"),
        "a reopen must find it"
    );

    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(vec![
            ref_entry("heads/main"),
            ref_entry("heads/feature"),
        ]))),
    });
    assert_eq!(app.marked_ref(), Some("heads/main"));
    assert!(top_overlay_text(&app).contains("● heads/main"));

    // **A ref change ends it.** A stale pair is worse than an empty one.
    app.nav_down();
    app.select(); // focus heads/feature
    assert_eq!(app.focused_ref(), Some("heads/feature"));
    assert_eq!(app.marked_ref(), None);
}

#[test]
fn re_picking_the_ref_already_focused_is_not_a_ref_change() {
    // The mark dies on a ref **change**, not on every `Enter`. Confirming the ref you are already
    // on has moved nothing, so there is nothing to forget — and the pair you were building is
    // still the pair you meant.
    let (mut app, _rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    assert_eq!(app.focused_ref(), Some("heads/main"));

    app.nav_down();
    app.mark_block(); // mark heads/feature
    assert_eq!(app.marked_ref(), Some("heads/feature"));

    app.nav_up();
    app.select(); // pick heads/main, which is already the focused ref
    assert_eq!(app.focused_ref(), Some("heads/main"));
    assert_eq!(
        app.marked_ref(),
        Some("heads/feature"),
        "re-picking the focused ref is not a ref change"
    );
}

#[test]
fn a_ref_mark_and_a_block_mark_are_never_read_for_each_other() {
    // **Two marks, kept apart** (handoff §2). `d` in History must never use a ref mark, and the
    // picker's `d` must never use a block mark.
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    app.open_history();
    let hist = next_request(&rx);
    app.apply(Response {
        seq: hist.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });

    // A block mark, made in History.
    app.mark_block();
    assert!(app.marked_block().is_some());
    assert_eq!(app.marked_ref(), None, "History's `m` marks no ref");

    // A ref mark, made in the picker, leaves the block mark alone.
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(vec![
            ref_entry("heads/main"),
            ref_entry("heads/feature"),
        ]))),
    });
    app.mark_block(); // the picker is open: this marks a ref
    assert_eq!(app.marked_ref(), Some("heads/main"));
    assert!(
        app.marked_block().is_some(),
        "the block mark is untouched by the picker's `m`"
    );

    // The picker's `d` sends a **ref** pair, never the block mark.
    app.nav_down();
    app.compare();
    match next_request(&rx).kind {
        RequestKind::Compare { opened, .. } => assert!(
            matches!(opened, CompareSource::RefPair { .. }),
            "got {opened:?}"
        ),
        other => panic!("expected a Compare request, got {other:?}"),
    }
}

// =============================================================================================
// RFC 044 — create a branch, create a tag, close a branch.
//
// The three keys are checked where they are decided: **what reaches prikk** (the prefixed name as
// one argument, `--from` always present, `-m` only when a message was typed) and **what never
// reaches it at all** (an empty name, a tag asked to close, a branch already closed, a row with no
// published tip, a prikk below the band, a session without a maintainer key).
//
// The current-branch refusal is core's, not this layer's, and `stikk-core::refs::tests` owns it:
// only the preview's own read knows which branch that is when the key is pressed.
// =============================================================================================

/// A session that may write refs: MAINTAINER readiness and a prikk at the band.
fn ref_write_view(minor: u32) -> stikk_core::OrientationView {
    let mut view = maintainer_orientation_view(0, None);
    view.prikk_minor = minor;
    view
}

fn entry(name: &str, closed: bool) -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: name.to_string(),
        id: "e".repeat(64),
        closed,
        received: false,
    }
}

/// The picker, open and populated, with `cursor` on the row the keys will act on.
fn picker(
    view: stikk_core::OrientationView,
    rows: Vec<stikk_prikk::RefEntry>,
    unpublished: stikk_prikk::UnpublishedBranch,
    cursor: usize,
) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(view), Palette::default());
    app.open_ref_picker();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Refs(Ok(stikk_core::RefList {
            refs: rows,
            unpublished_current: unpublished,
        })),
    });
    for _ in 0..cursor {
        app.nav_down();
    }
    (app, rx)
}

fn open_picker(cursor: usize) -> (App, mpsc::Receiver<Request>) {
    picker(
        ref_write_view(52),
        vec![
            entry("heads/main", false),
            entry("heads/old", true),
            entry("tags/v1", false),
        ],
        stikk_prikk::UnpublishedBranch::None,
        cursor,
    )
}

/// `n` opens the prompt and **dispatches nothing**: a name has to exist before there is a call to
/// preview.
#[test]
fn n_opens_the_branch_name_prompt_with_the_prefix_fixed() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    match app.top_overlay() {
        Some(Overlay::RefName {
            kind,
            source,
            typed,
        }) => {
            assert_eq!(*kind, crate::overlay::RefNameKind::Branch);
            assert_eq!(kind.prefix(), "heads/");
            assert_eq!(source, "heads/main", "the selected ref is the --from");
            assert!(typed.is_empty());
        }
        other => panic!("expected a RefName prompt, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "nothing is dispatched by the key");
}

/// **An empty name is not sent** — the one rule stikk applies to a name (decision 5).
#[test]
fn an_empty_name_is_not_sent() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    app.select();
    assert!(
        matches!(app.top_overlay(), Some(Overlay::RefName { .. })),
        "the prompt stays open"
    );
    assert!(rx.try_recv().is_err(), "and nothing is dispatched");
}

/// The prefix is joined on here, and `--from` carries the ref the user actually selected (trap 1).
#[test]
fn a_branch_name_reaches_the_preview_prefixed_and_with_its_source() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    for ch in "bad name".chars() {
        app.input_char(ch);
    }
    app.select();
    match next_request(&rx).kind {
        RequestKind::RefWritePreview {
            op: RefWriteOp::CreateBranch { name, from_ref },
        } => {
            assert_eq!(
                name, "heads/bad name",
                "a space is prikk's to judge, not stikk's"
            );
            assert_eq!(from_ref, "heads/main");
        }
        other => panic!("expected a CreateBranch preview, got {other:?}"),
    }
}

/// A tag's message prompt follows its name, and **an empty message is a valid answer**: it sends
/// no `-m`, which is prikk's own shape for a tag that carries none.
#[test]
fn a_tag_is_named_then_messaged_and_an_empty_message_is_allowed() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_tag();
    for ch in "v1".chars() {
        app.input_char(ch);
    }
    app.select();
    match app.top_overlay() {
        Some(Overlay::TagMessage {
            name,
            target,
            typed,
        }) => {
            assert_eq!(name, "tags/v1");
            assert_eq!(target, "heads/main");
            assert!(typed.is_empty());
        }
        other => panic!("expected a TagMessage prompt, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "the name step dispatches nothing");
    app.select(); // Enter on an empty message
    match next_request(&rx).kind {
        RequestKind::RefWritePreview {
            op:
                RefWriteOp::CreateTag {
                    name,
                    target,
                    message,
                },
        } => {
            assert_eq!(name, "tags/v1");
            assert_eq!(target, "heads/main");
            assert!(message.is_empty(), "no -m is what an empty message means");
        }
        other => panic!("expected a CreateTag preview, got {other:?}"),
    }
}

/// In History, `t` tags **the selected block, by block id** (decision 4) — not the ref.
#[test]
fn t_in_history_tags_the_selected_block_by_id() {
    let (mut app, rx) = from_state("/repo", loaded(ref_write_view(52)), Palette::default());
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(HistoryView {
            reff: "heads/main".to_string(),
            queued: 0,
            queued_target: None,
            blocks: vec![block(&"a".repeat(64), 2), block(&"b".repeat(64), 1)],
        })),
    });
    app.nav_down(); // the second block
    app.begin_create_tag();
    match app.top_overlay() {
        Some(Overlay::RefName { kind, source, .. }) => {
            assert_eq!(*kind, crate::overlay::RefNameKind::Tag);
            assert_eq!(source, &"b".repeat(64), "the block id is the --target");
        }
        other => panic!("expected a RefName prompt, got {other:?}"),
    }
}

/// `x` on a tag, and `x` on a branch the list already marks closed: **stikk's own words, and
/// prikk is never asked** (RFC 044 §3).
#[test]
fn x_declines_a_tag_and_an_already_closed_branch_without_running_prikk() {
    for (cursor, fragment) in [(1, "already closed"), (2, "only a branch can be closed")] {
        let (mut app, rx) = open_picker(cursor);
        app.begin_close_branch();
        assert!(
            app.banner().is_some_and(|b| b.contains(fragment)),
            "cursor {cursor}: got {:?}",
            app.banner()
        );
        assert!(
            rx.try_recv().is_err(),
            "cursor {cursor}: nothing was dispatched"
        );
        assert!(
            matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })),
            "the picker stays open"
        );
    }
}

/// The unpublished current branch has no tip, so **none of the three is offered on it** — with
/// stikk's own reason, never prikk's *"does not exist"* (RFC 041 decision 4's precedent).
#[test]
fn none_of_the_three_is_offered_on_a_row_with_no_published_tip() {
    for key in ["n", "t", "x"] {
        let (mut app, rx) = picker(
            ref_write_view(52),
            Vec::new(),
            stikk_prikk::UnpublishedBranch::Branch("heads/main".into()),
            0,
        );
        match key {
            "n" => app.begin_create_branch(),
            "t" => app.begin_create_tag(),
            _ => app.begin_close_branch(),
        }
        assert!(
            app.banner()
                .is_some_and(|b| b.contains("no published history yet")),
            "{key}: got {:?}",
            app.banner()
        );
        assert!(rx.try_recv().is_err(), "{key}: nothing was dispatched");
    }
}

/// Below the band the keys answer in stikk's words and **spawn nothing** (`C-T4d`).
#[test]
fn below_the_band_the_keys_give_stikks_own_reason() {
    let (mut app, rx) = picker(
        ref_write_view(41),
        vec![entry("heads/main", false)],
        stikk_prikk::UnpublishedBranch::None,
        0,
    );
    app.begin_create_branch();
    let banner = app.banner().expect("a reason").to_string();
    assert!(banner.contains("0.42"), "{banner}");
    assert!(banner.contains("this prikk is 0.41"), "{banner}");
    assert!(
        !banner.contains("signature"),
        "prikk's words are not stikk's: {banner}"
    );
    assert!(rx.try_recv().is_err(), "nothing was dispatched");
}

/// **Readiness first** (trap 5): without a maintainer key the reason is stikk's, before prikk runs.
#[test]
fn without_a_maintainer_key_the_keys_say_so_before_prikk_runs() {
    let mut view = author_orientation_view();
    view.prikk_minor = 52;
    let (mut app, rx) = picker(
        view,
        vec![entry("heads/main", false)],
        stikk_prikk::UnpublishedBranch::None,
        0,
    );
    app.begin_create_branch();
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("needs MAINTAINER signing readiness")),
        "got {:?}",
        app.banner()
    );
    assert!(rx.try_recv().is_err(), "nothing was dispatched");
}

/// A ready preview arms the confirmation at Tier Three, as seal's does.
#[test]
fn a_ready_ref_write_preview_opens_the_confirmation_at_tier_three() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    for ch in "feature".chars() {
        app.input_char(ch);
    }
    app.select();
    let req = next_request(&rx);
    let backend = NullBackend::supported();
    let preview = stikk_core::create_branch_preview(
        &backend,
        std::path::Path::new("/repo"),
        "heads/feature",
        "heads/main",
    )
    .expect("a scripted backend's preview always succeeds");
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::RefWritePreview(Ok(preview)),
    });
    match app.top_overlay() {
        Some(Overlay::Confirmation { tier, summary, .. }) => {
            assert_eq!(*tier, Tier::Three);
            assert!(summary.target_ids.contains(&"heads/feature".to_string()));
        }
        other => panic!("expected a Confirmation, got {other:?}"),
    }
}

/// Success: prikk's lines verbatim, and **the ref lists are re-read** (decision 8).
#[test]
fn a_successful_ref_write_shows_prikks_lines_and_re_reads_the_refs() {
    let (mut app, rx) = open_picker(0);
    app.begin_close_branch();
    // `heads/main` is open and not the current branch here, so the picker's guards pass.
    let preview_req = next_request(&rx);
    let backend = NullBackend::supported();
    let preview =
        stikk_core::close_branch_preview(&backend, std::path::Path::new("/repo"), "heads/main")
            .expect("preview succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::RefWritePreview(Ok(preview)),
    });
    app.select(); // confirm
    let exec_req = next_request(&rx);
    let lines = vec![
        "closed branch heads/main".to_string(),
        "nothing was reclaimed; the pointer, its history, and every object remain".to_string(),
    ];
    app.apply(Response {
        seq: exec_req.seq,
        kind: ResponseKind::RefWriteConfirmExecute {
            kind: crate::overlay::RefWriteKind::CloseBranch,
            result: Ok(Outcome {
                operation: stikk_core::CLOSE_BRANCH_OPERATION.to_string(),
                result: stikk_prikk::RefWrite::Done(lines.clone()),
            }),
        },
    });
    match app.top_overlay() {
        Some(Overlay::RefWriteResult { kind, lines: shown }) => {
            assert_eq!(*kind, crate::overlay::RefWriteKind::CloseBranch);
            assert_eq!(shown, &lines, "prikk's lines, in order, unaltered");
        }
        other => panic!("expected a RefWriteResult, got {other:?}"),
    }
    assert!(
        matches!(next_request(&rx).kind, RequestKind::Refs),
        "the ref lists are re-read"
    );
}

/// **A failure keeps the user's typed name** (§3), so the correction is one edit and not a retype.
#[test]
fn a_refused_create_hands_the_typed_name_back() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    for ch in "feature".chars() {
        app.input_char(ch);
    }
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::RefWritePreview(Err(StikkError::Refusal {
            message: "branch heads/feature already exists".to_string(),
        })),
    });
    // prikk's refusal is on top; the prompt is underneath it, still holding what was typed.
    app.back();
    match app.top_overlay() {
        Some(Overlay::RefName { typed, source, .. }) => {
            assert_eq!(
                typed, "feature",
                "without the prefix the prompt shows itself"
            );
            assert_eq!(source, "heads/main");
        }
        other => panic!("expected the prompt back, got {other:?}"),
    }
}

/// Cancelling any step abandons the write, so no armed token outlives it.
#[test]
fn back_on_a_ref_write_prompt_clears_the_pending_write() {
    let (mut app, rx) = open_picker(0);
    app.begin_create_branch();
    app.back();
    assert!(!app.has_overlay());
    // A stray Enter at the root opens History (that is what `Enter` means there) — what it must
    // never do is resurrect the write the user just cancelled.
    app.select();
    while let Ok(req) = rx.try_recv() {
        assert!(
            !matches!(req.kind, RequestKind::RefWritePreview { .. }),
            "the cancelled write came back: {:?}",
            req.kind
        );
    }
}

// =============================================================================================
// RFC 045 — `s`, the switch key.
//
// What is checked here is what the key decides **before** anything is dispatched: the rows it
// declines, each with its own reason, and the fact that none of them spawns. The plan itself,
// and every blocked case that needs a read, are `stikk-core::switch::tests`'.
// =============================================================================================

/// A session that may switch: prikk 0.52 and a current branch.
fn switch_view(minor: u32, current: &str) -> stikk_core::OrientationView {
    let mut view = maintainer_orientation_view(0, None);
    view.prikk_minor = minor;
    view.current_branch = stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse(current).expect("a ref name"),
    );
    view
}

fn switch_picker(
    view: stikk_core::OrientationView,
    cursor: usize,
) -> (App, mpsc::Receiver<Request>) {
    picker(
        view,
        vec![
            entry("heads/main", false),
            entry("heads/feature", false),
            entry("heads/old", true),
            entry("tags/v1", false),
        ],
        stikk_prikk::UnpublishedBranch::None,
        cursor,
    )
}

/// `s` on an open branch that is not the current one dispatches the preview and nothing else.
#[test]
fn s_previews_the_selected_branch() {
    let (mut app, rx) = switch_picker(switch_view(52, "heads/main"), 1);
    app.begin_switch_branch();
    match next_request(&rx).kind {
        RequestKind::SwitchPreview { target } => assert_eq!(target, "heads/feature"),
        other => panic!("expected a SwitchPreview, got {other:?}"),
    }
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Loading {
            what: "switch preview",
            ..
        })
    ));
}

/// **Each row `s` declines has its own reason, and none of them spawns** (decision 7).
#[test]
fn s_declines_each_row_kind_without_dispatching() {
    for (cursor, fragment) in [
        (0, "already the current branch"),
        (2, "this branch is closed"),
        (3, "only a branch can be switched to"),
    ] {
        let (mut app, rx) = switch_picker(switch_view(52, "heads/main"), cursor);
        app.begin_switch_branch();
        assert!(
            app.banner().is_some_and(|b| b.contains(fragment)),
            "cursor {cursor}: got {:?}",
            app.banner()
        );
        assert!(
            rx.try_recv().is_err(),
            "cursor {cursor}: nothing dispatched"
        );
        assert!(
            matches!(app.top_overlay(), Some(Overlay::RefPicker { .. })),
            "cursor {cursor}: the picker stays open"
        );
    }
}

/// The unpublished current branch has no tip, so `s` declines it too — with the same words `n`,
/// `t` and `x` use (RFC 037 B; RFC 041 decision 4's precedent).
#[test]
fn s_declines_the_unpublished_row() {
    let (mut app, rx) = picker(
        switch_view(52, "heads/main"),
        Vec::new(),
        stikk_prikk::UnpublishedBranch::Branch("heads/main".into()),
        0,
    );
    app.begin_switch_branch();
    assert!(
        app.banner()
            .is_some_and(|b| b.contains("no published history yet")),
        "got {:?}",
        app.banner()
    );
    assert!(rx.try_recv().is_err());
}

/// Below the band `s` gives stikk's reason and spawns nothing (`C-T4d`).
#[test]
fn below_the_band_s_gives_stikks_own_reason() {
    let (mut app, rx) = switch_picker(switch_view(45, "heads/main"), 1);
    app.begin_switch_branch();
    let banner = app.banner().expect("a reason").to_string();
    assert!(banner.contains("0.46"), "{banner}");
    assert!(banner.contains("0.45"), "{banner}");
    assert!(rx.try_recv().is_err());
}

/// A ready preview arms the confirmation at Tier Three, with **no capability row to consume**.
#[test]
fn a_ready_switch_preview_opens_the_confirmation_at_tier_three() {
    let (mut app, rx) = switch_picker(switch_view(52, "heads/main"), 1);
    app.begin_switch_branch();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::SwitchPreview(Ok(switch_core_preview())),
    });
    match app.top_overlay() {
        Some(Overlay::Confirmation { tier, summary, .. }) => {
            assert_eq!(*tier, Tier::Three);
            assert_eq!(summary.capability, None, "a switch signs nothing");
            assert!(summary.materializes.is_some(), "the plan is on the card");
        }
        other => panic!("expected a Confirmation, got {other:?}"),
    }
}

/// A real preview from `stikk-core`, so this file never hand-builds one.
fn switch_core_preview() -> stikk_core::SwitchPreview {
    let current = stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse("heads/main").expect("a ref name"),
    );
    let backend = NullBackend::supported()
        .with_version(0, 52, 0)
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 0,
            queued_target: None,
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: current.clone(),
        })
        .with_change_token(stikk_model::ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &current,
        ))
        .with_diff(stikk_prikk::Diff {
            from: switch_side("heads/main"),
            to: switch_side("heads/feature"),
            entries: vec![
                switch_entry("a.txt", "modified"),
                switch_entry("b.txt", "deleted"),
            ],
            unsupported_paths: Vec::new(),
        });
    stikk_core::switch_preview(&backend, std::path::Path::new("/repo"), "heads/feature")
        .expect("a scripted backend's preview always succeeds")
}

fn switch_side(point: &str) -> stikk_prikk::DiffSide {
    stikk_prikk::DiffSide {
        point: point.to_string(),
        target: stikk_prikk::DiffTarget::Block("b".repeat(64)),
        queued: stikk_prikk::QueuedOnSide::NotApplicable,
    }
}

fn switch_entry(path: &str, status: &str) -> stikk_prikk::DiffEntry {
    stikk_prikk::DiffEntry {
        path: path.to_string(),
        status: status.to_string(),
        from_path: None,
        from: None,
        to: None,
        minimal: true,
        hunks: Vec::new(),
        raw: "{}".to_string(),
    }
}

/// **Success moves focus and re-reads** (decision 8): `FR-055` keeps a later *read* from moving
/// focus; this is the user's own action, and History on the branch left behind would describe a
/// worktree that is no longer its files.
#[test]
fn a_successful_switch_moves_focus_and_re_reads() {
    let (mut app, rx) = switch_picker(switch_view(52, "heads/main"), 1);
    app.begin_switch_branch();
    let preview_req = next_request(&rx);
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SwitchPreview(Ok(switch_core_preview())),
    });
    app.select(); // confirm
    let exec_req = next_request(&rx);
    app.apply(Response {
        seq: exec_req.seq,
        kind: ResponseKind::SwitchConfirmExecute(Ok(Outcome {
            operation: stikk_core::SWITCH_OPERATION.to_string(),
            result: stikk_prikk::SwitchResult::Done {
                printed: vec![
                    "switched from heads/main to heads/feature".to_string(),
                    "files written: 1".to_string(),
                ],
                noted: Vec::new(),
            },
        })),
    });
    match app.top_overlay() {
        Some(Overlay::SwitchResult { printed, noted }) => {
            assert_eq!(printed[0], "switched from heads/main to heads/feature");
            assert!(noted.is_empty());
        }
        other => panic!("expected a SwitchResult, got {other:?}"),
    }
    assert_eq!(
        app.focused_ref(),
        Some("heads/feature"),
        "focus follows the switch"
    );
    let kinds: Vec<&'static str> = std::iter::from_fn(|| rx.try_recv().ok())
        .map(|r| r.kind.label())
        .collect();
    assert!(kinds.contains(&"orientation"), "{kinds:?}");
    assert!(kinds.contains(&"refs"), "{kinds:?}");
}

/// **A refusal hands the reader back to the picker** (§3e), with the list re-read: a dirty
/// worktree is something they fix and try again from where they chose.
#[test]
fn a_refused_switch_hands_back_to_the_picker() {
    let (mut app, rx) = switch_picker(switch_view(52, "heads/main"), 1);
    app.begin_switch_branch();
    let preview_req = next_request(&rx);
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SwitchPreview(Ok(switch_core_preview())),
    });
    app.select();
    let exec_req = next_request(&rx);
    app.apply(Response {
        seq: exec_req.seq,
        kind: ResponseKind::SwitchConfirmExecute(Err(StikkError::Refusal {
            message: "error: precondition not met: the worktree is not clean against heads/main"
                .to_string(),
        })),
    });
    // prikk's refusal is on top; the picker's own read is underneath it.
    let refs_req = next_request(&rx);
    assert!(matches!(refs_req.kind, RequestKind::Refs));
    app.back();
    assert!(
        matches!(
            app.top_overlay(),
            Some(Overlay::Loading { what: "refs", .. })
        ),
        "got {:?}",
        app.top_overlay()
    );
}

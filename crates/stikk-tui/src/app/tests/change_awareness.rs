//! RFC 031: seeing a change made outside stikk. The stamp, the silent check, when it runs, a detected
//! change, and `r` on Changes and Block detail — all over scripted responses with synthetic `Instant`s
//! (handoff §8). **No test here sleeps.**

use std::time::{Duration, Instant};

use stikk_model::{ChangeToken, StaleCause};

use super::*;

const NOTICE: &str = "repository changed outside stikk — refreshed";

/// A token distinct for each `n`.
fn token(n: u64) -> ChangeToken {
    ChangeToken::compose(std::iter::empty(), n, None, &CurrentBranch::NotReported)
}

/// An app loaded with `view` and stamped with `stamp` through a real Orientation read, with no request
/// left pending and nothing left on the channel.
fn stamped(
    view: stikk_core::OrientationView,
    stamp: ChangeToken,
) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(view.clone()), Palette::default());
    app.reload();
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Ok(stamp),
            view: Ok(view),
        }),
    });
    assert_eq!(app.stamp, Some(stamp));
    assert_eq!(app.in_flight_count(), 0);
    assert!(rx.try_recv().is_err());
    (app, rx)
}

/// Expect exactly one check to have been sent, and return its `seq`.
fn sent_check(rx: &mpsc::Receiver<Request>) -> u64 {
    let req = next_request(rx);
    assert!(
        matches!(req.kind, RequestKind::ChangeCheck),
        "expected a change check, got {:?}",
        req.kind
    );
    req.seq
}

fn answer_check(app: &mut App, seq: u64, result: stikk_model::Result<ChangeToken>) {
    app.apply(Response {
        seq,
        kind: ResponseKind::ChangeCheck(result),
    });
}

/// Every request on the channel, drained.
fn drain(rx: &mpsc::Receiver<Request>) -> Vec<Request> {
    rx.try_iter().collect()
}

/// Author-ready, with a commit's Confirmation open and `pending_commit` armed.
fn with_commit_confirmation(stamp: ChangeToken) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = stamped(author_orientation_view(), stamp);
    app.begin_commit();
    app.input_char('x');
    app.select();
    let preview_req = next_request(&rx);
    let outcome = commit_preview(&commit_backend(), Path::new("/repo"), "heads/main")
        .expect("a scripted backend's preview always succeeds");
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Confirmation { .. })
    ));
    assert!(app.pending_commit.is_some());
    (app, rx)
}

/// Assert test 7's refresh: Orientation re-requested, then whatever the top screen needs.
fn assert_refreshed_with_notice(app: &App, requests: &[Request]) {
    assert!(
        matches!(requests.first().map(|r| &r.kind), Some(RequestKind::Orient)),
        "a detected change re-reads Orientation first, got {requests:?}"
    );
    assert!(app.orientation_pending.is_some());
    assert_eq!(app.banner(), Some(NOTICE));
}

// 1
#[test]
fn no_check_is_sent_before_the_first_stamp_and_the_first_stamp_says_nothing() {
    let (mut app, rx) = from_state(
        "/repo",
        loaded(orientation_view(0, None, None)),
        Palette::default(),
    );
    let t0 = Instant::now();
    app.tick(t0);
    app.tick(t0 + Duration::from_secs(60));
    app.focus_gained(t0 + Duration::from_secs(61));
    assert!(rx.try_recv().is_err(), "no stamp, so no check");

    app.reload();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Ok(token(1)),
            view: Ok(orientation_view(0, None, None)),
        }),
    });
    assert_eq!(app.stamp, Some(token(1)));
    assert_eq!(app.banner(), None, "the first stamp is not a change");
    assert!(app.top_overlay().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_failed_token_read_keeps_the_previous_stamp_whatever_orientation_did() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.reload();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Err(StikkError::Refusal {
                message: "branch list failed".into(),
            }),
            view: Ok(orientation_view(3, None, None)),
        }),
    });
    assert_eq!(app.stamp, Some(token(1)));

    // And a successful token read stamps even when Orientation itself failed.
    app.reload();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Ok(token(2)),
            view: Err(StikkError::Refusal {
                message: "status failed".into(),
            }),
        }),
    });
    assert_eq!(app.stamp, Some(token(2)));
}

// 2
#[test]
fn the_interval_sends_one_check_at_five_seconds_and_none_while_it_is_pending() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    let t0 = Instant::now();
    app.tick(t0); // the first tick after a stamp starts the clock
    app.tick(t0 + Duration::from_millis(4900));
    assert!(rx.try_recv().is_err(), "4.9 s is not yet the interval");

    app.tick(t0 + CHANGE_CHECK_INTERVAL);
    let seq = sent_check(&rx);
    assert!(rx.try_recv().is_err(), "exactly one");
    assert_eq!(app.check_pending, Some(seq));

    app.tick(t0 + Duration::from_secs(30));
    assert!(rx.try_recv().is_err(), "at most one check in flight");
}

// 3
#[test]
fn focus_gained_checks_at_once_and_resets_the_interval() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    let t0 = Instant::now();
    app.tick(t0);
    let focus_at = t0 + Duration::from_secs(1);
    app.focus_gained(focus_at);
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(1)));

    app.tick(focus_at + Duration::from_millis(4900));
    assert!(
        rx.try_recv().is_err(),
        "the interval counts from the focus check, not from t0"
    );
    app.tick(focus_at + CHANGE_CHECK_INTERVAL);
    sent_check(&rx);
}

// 4
#[test]
fn no_check_is_sent_while_a_request_runs_or_orientation_is_pending() {
    let (mut app, rx) = stamped(author_orientation_view(), token(1));
    let now = Instant::now();

    // A History read running.
    app.open_history();
    let history_req = next_request(&rx);
    app.focus_gained(now);
    assert!(rx.try_recv().is_err(), "no check while History runs");
    app.apply(Response {
        seq: history_req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    app.back();

    // An Orientation read pending.
    app.reload();
    let orient_req = next_request(&rx);
    assert!(app.orientation_pending.is_some());
    app.focus_gained(now);
    assert!(
        rx.try_recv().is_err(),
        "no check while Orientation is pending"
    );
    app.apply(Response {
        seq: orient_req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Ok(token(1)),
            view: Ok(author_orientation_view()),
        }),
    });

    // A commit preview running.
    app.begin_commit();
    app.input_char('x');
    app.select();
    let preview_req = next_request(&rx);
    assert!(matches!(
        preview_req.kind,
        RequestKind::CommitPreview { .. }
    ));
    app.focus_gained(now);
    assert!(rx.try_recv().is_err(), "no check while a preview runs");

    // A confirm-and-execute running is test 9's.
    let outcome = commit_preview(&commit_backend(), Path::new("/repo"), "heads/main").unwrap();
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });
    app.focus_gained(now);
    sent_check(&rx); // and with nothing running, one is sent: an open confirmation does not suppress it
}

// 5
#[test]
fn a_check_never_touches_the_operations_list_or_the_in_flight_count() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    let now = Instant::now();
    let before = app.operations().to_vec();
    let in_flight = app.in_flight_count();
    let unchanged = |app: &App| {
        assert_eq!(app.operations(), before.as_slice());
        assert_eq!(app.in_flight_count(), in_flight);
    };

    // Equal.
    app.focus_gained(now);
    let seq = sent_check(&rx);
    unchanged(&app);
    answer_check(&mut app, seq, Ok(token(1)));
    unchanged(&app);

    // Failed.
    app.focus_gained(now);
    let seq = sent_check(&rx);
    unchanged(&app);
    answer_check(
        &mut app,
        seq,
        Err(StikkError::Refusal {
            message: "tag list failed".into(),
        }),
    );
    unchanged(&app);

    // Different: the check itself still adds nothing. What does appear is the refresh it causes, the
    // same entries `r` adds.
    app.focus_gained(now);
    let seq = sent_check(&rx);
    unchanged(&app);
    answer_check(&mut app, seq, Ok(token(2)));
    let refresh = drain(&rx);
    assert_eq!(app.operations().len(), before.len() + refresh.len());
    assert_eq!(app.in_flight_count(), in_flight + refresh.len());
    assert!(
        app.operations().iter().all(|op| op.label != "change check"),
        "no check was ever recorded"
    );
    assert!(
        app.operations()[before.len()..]
            .iter()
            .all(|op| op.label == "orientation")
    );
}

// 6
#[test]
fn an_equal_token_shows_nothing_dispatches_nothing_and_changes_no_overlay() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.open_glossary();
    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(1)));
    assert_eq!(app.banner(), None);
    assert!(rx.try_recv().is_err());
    assert!(matches!(app.top_overlay(), Some(Overlay::Glossary { .. })));
    assert!(app.orientation_pending.is_none());
    assert_eq!(app.check_pending, None);
}

// 7
#[test]
fn a_different_token_restamps_refreshes_the_top_screen_and_banners_op_04s_words() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.open_history();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });

    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));

    assert_eq!(app.stamp, Some(token(2)));
    let requests = drain(&rx);
    assert_refreshed_with_notice(&app, &requests);
    assert_eq!(requests.len(), 2);
    assert!(matches!(&requests[1].kind, RequestKind::History { reff } if reff == "heads/main"));
    assert!(
        matches!(app.focus(), Focus::History(view, _) if view.blocks.len() == 2),
        "the view stays visible while it refreshes"
    );
}

// 8
#[test]
fn a_failed_check_is_silent_and_the_next_interval_sends_another() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    let t0 = Instant::now();
    app.tick(t0);
    app.tick(t0 + CHANGE_CHECK_INTERVAL);
    let seq = sent_check(&rx);
    answer_check(
        &mut app,
        seq,
        Err(StikkError::Refusal {
            message: "status failed".into(),
        }),
    );
    assert_eq!(app.banner(), None);
    assert!(app.top_overlay().is_none());
    assert!(rx.try_recv().is_err());
    assert_eq!(app.stamp, Some(token(1)));

    app.tick(t0 + CHANGE_CHECK_INTERVAL * 2);
    sent_check(&rx);
}

// 9
#[test]
fn stikks_own_commit_is_never_taken_for_an_outside_change() {
    let (mut app, rx) = with_commit_confirmation(token(1));
    let t0 = Instant::now();

    // A check already in flight when Enter is pressed.
    app.focus_gained(t0);
    let early_check = sent_check(&rx);

    app.select(); // Enter: confirm-and-execute
    let confirm_req = next_request(&rx);
    assert!(matches!(
        confirm_req.kind,
        RequestKind::CommitConfirmExecute { .. }
    ));

    // The worker is FIFO: that check ran before the commit, so it carries the pre-commit token.
    answer_check(&mut app, early_check, Ok(token(1)));
    assert_eq!(app.banner(), None);
    assert!(rx.try_recv().is_err());

    // No check while the commit runs.
    app.tick(t0 + Duration::from_secs(60));
    app.focus_gained(t0 + Duration::from_secs(61));
    assert!(rx.try_recv().is_err(), "no check while the commit runs");

    // Success dispatches Orientation; no check while it is pending.
    let result = CommitResult {
        baseline_ref: "heads/main".to_string(),
        patch_id: "1".repeat(64),
        wal_sequence: 1,
        operations: 1,
        referenced_blobs: 1,
        text_edits: 0,
        changes: Vec::new(),
        notes: Vec::new(),
    };
    app.apply(Response {
        seq: confirm_req.seq,
        kind: ResponseKind::CommitConfirmExecute(Ok(Outcome {
            operation: "commit".to_string(),
            result,
        })),
    });
    let orient_req = next_request(&rx);
    assert!(matches!(orient_req.kind, RequestKind::Orient));
    app.tick(t0 + Duration::from_secs(120));
    app.focus_gained(t0 + Duration::from_secs(121));
    assert!(
        rx.try_recv().is_err(),
        "no check while Orientation is pending"
    );

    // That read stamps the post-commit token, silently.
    app.apply(Response {
        seq: orient_req.seq,
        kind: ResponseKind::Orient(OrientRead {
            token: Ok(token(2)),
            view: Ok(author_orientation_view()),
        }),
    });
    assert_eq!(app.stamp, Some(token(2)));
    assert_eq!(app.banner(), None);

    // A following check sees the same post-commit token: nothing to say.
    app.focus_gained(t0 + Duration::from_secs(122));
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));
    assert_eq!(app.banner(), None);
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::CommitResult { .. })
    ));
    assert!(rx.try_recv().is_err());
}

// 10
#[test]
fn a_change_detected_under_a_commit_confirmation_makes_it_stale_at_once() {
    let (mut app, rx) = with_commit_confirmation(token(1));
    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));

    match app.top_overlay() {
        Some(Overlay::Stale {
            operation, cause, ..
        }) => {
            assert_eq!(operation, "commit");
            assert_eq!(*cause, StaleCause::Repository);
        }
        other => panic!("expected the stale overlay, got {other:?}"),
    }
    assert_eq!(
        app.overlays.len(),
        1,
        "the confirmation was replaced, not covered"
    );
    assert!(app.pending_commit.is_none(), "nothing stays armed");
    let requests = drain(&rx);
    assert_refreshed_with_notice(&app, &requests);
    assert_eq!(app.stamp, Some(token(2)));
}

// 11
#[test]
fn a_change_detected_at_seals_consent_step_makes_it_stale_at_once() {
    let (mut app, rx) = stamped(maintainer_orientation_view(1, Some("heads/main")), token(1));
    app.begin_seal();
    let preview_req = next_request(&rx);
    let outcome = seal_preview(&seal_backend(), Path::new("/repo"), "heads/main").unwrap();
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.select(); // the first act: -> SealConsent
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::SealConsent { .. })
    ));
    assert!(app.pending_seal.is_some());

    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));

    match app.top_overlay() {
        Some(Overlay::Stale {
            operation, cause, ..
        }) => {
            assert_eq!(operation, "seal");
            assert_eq!(*cause, StaleCause::Repository);
        }
        other => panic!("expected the stale overlay, got {other:?}"),
    }
    assert_eq!(app.overlays.len(), 1);
    assert!(app.pending_seal.is_none(), "nothing stays armed");
    let requests = drain(&rx);
    assert_refreshed_with_notice(&app, &requests);
}

#[test]
fn a_change_detected_under_a_seal_confirmation_makes_it_stale_too() {
    let (mut app, rx) = stamped(maintainer_orientation_view(1, Some("heads/main")), token(1));
    app.begin_seal();
    let preview_req = next_request(&rx);
    let outcome = seal_preview(&seal_backend(), Path::new("/repo"), "heads/main").unwrap();
    app.apply(Response {
        seq: preview_req.seq,
        kind: ResponseKind::SealPreview(Ok(outcome)),
    });
    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::Stale { operation, .. }) if operation == "seal"
    ));
    assert!(app.pending_seal.is_none());
}

// 12
#[test]
fn a_commit_message_prompt_is_not_armed_and_stays_open() {
    let (mut app, rx) = stamped(author_orientation_view(), token(1));
    app.begin_commit();
    app.input_char('x');
    app.focus_gained(Instant::now());
    let seq = sent_check(&rx);
    answer_check(&mut app, seq, Ok(token(2)));
    match app.top_overlay() {
        Some(Overlay::CommitMessage { typed, .. }) => assert_eq!(typed, "x"),
        other => panic!("expected the message prompt, still open, got {other:?}"),
    }
    let requests = drain(&rx);
    assert_refreshed_with_notice(&app, &requests);
}

// 13
#[test]
fn r_refreshes_changes_in_place_and_keeps_the_untracked_filter() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.open_changes();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Ok(dirty_changes())),
    });
    app.toggle_untracked();

    app.reload();
    let requests = drain(&rx);
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0].kind, RequestKind::Orient));
    let RequestKind::Changes { reff } = &requests[1].kind else {
        panic!("expected a Changes request, got {:?}", requests[1].kind);
    };
    assert_eq!(reff, "heads/main");
    assert!(
        matches!(app.focus(), Focus::Changes(view, true) if !view.clean),
        "the view stays visible while it refreshes"
    );

    let mut clean = dirty_changes();
    clean.clean = true;
    clean.entries.clear();
    app.apply(Response {
        seq: requests[1].seq,
        kind: ResponseKind::Changes(Ok(clean)),
    });
    match app.focus() {
        Focus::Changes(view, hide) => {
            assert!(view.clean);
            assert!(hide, "hide_untracked is kept as the user set it");
        }
        other => panic!("expected Changes, got {other:?}"),
    }
    assert!(matches!(
        app.screens.last(),
        Some(Screen::Changes {
            refreshing: None,
            ..
        })
    ));
}

#[test]
fn a_failed_changes_refresh_keeps_the_view_and_surfaces_the_error() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.open_changes();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Changes(Ok(dirty_changes())),
    });
    app.reload();
    let requests = drain(&rx);
    app.apply(Response {
        seq: requests[1].seq,
        kind: ResponseKind::Changes(Err(StikkError::Refusal {
            message: "worktree-status failed".into(),
        })),
    });
    assert!(matches!(app.focus(), Focus::Changes(view, false) if !view.clean));
    assert!(matches!(app.top_overlay(), Some(Overlay::Refusal { .. })));
}

fn block_detail_screen(is_tip: bool) -> Screen {
    Screen::BlockDetail {
        view: BlockDetailView {
            row: block("bbbb", 2),
            is_tip,
            state: None,
        },
        refreshing: None,
    }
}

#[test]
fn r_refreshes_the_tips_block_detail_in_place() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.push_screen(block_detail_screen(true));
    app.reload();
    let requests = drain(&rx);
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0].kind, RequestKind::Orient));
    match &requests[1].kind {
        RequestKind::BlockState { reff, row, is_tip } => {
            assert_eq!(reff, "heads/main");
            assert_eq!(row.block_id, "bbbb");
            assert!(*is_tip);
        }
        other => panic!("expected a BlockState request, got {other:?}"),
    }
    assert!(matches!(app.focus(), Focus::BlockDetail(view) if view.row.block_id == "bbbb"));

    app.apply(Response {
        seq: requests[1].seq,
        kind: ResponseKind::BlockState(Ok(BlockDetailView {
            row: block("cccc", 3),
            is_tip: true,
            state: None,
        })),
    });
    assert!(matches!(
        app.screens.last(),
        Some(Screen::BlockDetail { view, refreshing: None }) if view.row.block_id == "cccc"
    ));
}

#[test]
fn r_leaves_an_older_blocks_detail_as_it_is() {
    let (mut app, rx) = stamped(orientation_view(0, None, None), token(1));
    app.push_screen(block_detail_screen(false));
    app.reload();
    let requests = drain(&rx);
    assert_eq!(requests.len(), 1, "Orientation only, got {requests:?}");
    assert!(matches!(requests[0].kind, RequestKind::Orient));
    assert!(matches!(
        app.screens.last(),
        Some(Screen::BlockDetail {
            refreshing: None,
            ..
        })
    ));
}

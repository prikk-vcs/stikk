//! See [`crate::wiring`]: the tests that drive the program and the drawing together.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::mpsc;

use stikk_app::{
    App, Focus, OrientRead, OrientationState, Overlay, RefFocus, Request, RequestKind, Response,
    ResponseKind,
};
use stikk_core::Seen;
use stikk_core::{BlockDetailView, BlockPatches, HistoryView, Outcome, commit_preview};
use stikk_model::{Capability, CurrentBranch, Readiness, RefName, RoleReadiness, Tier};
use stikk_prikk::{BlockRow, NullBackend, Orientation, StateFiles, WorktreeStatus};

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
    let text = top_overlay_text(&mut app);
    println!("--- empty ref picker, 80×24\n{text}");
    assert!(text.contains("no published refs"), "{text}");
    assert!(text.contains("▶ heads/main (not published)"), "{text}");

    app.nav_down(); // one row: stays on it
    enter_twice(&mut app);
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
    let (mut app, rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
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
    let text = top_overlay_text(&mut app);
    assert!(text.contains("heads/dev"), "{text}");
    assert!(!text.contains("not published"), "{text}");
    assert!(!text.contains("no published refs"), "{text}");
}

#[test]
fn m_marks_a_ref_and_a_second_m_on_the_same_row_unmarks_it() {
    let (mut app, _rx) = picker_with_refs(&["heads/main", "heads/feature"]);
    assert_eq!(app.marked_ref(), None);

    app.mark_block(); // `m` — the picker is open, so it marks a ref
    assert_eq!(app.marked_ref(), Some("heads/main"));
    let text = top_overlay_text(&mut app);
    println!("--- RFC 041: the ref picker with a mark set, 80×24\n{text}");
    // The mark is on its row, and the title says one is set (the row can scroll out of sight).
    assert!(text.contains("● heads/main"), "{text}");
    assert!(text.contains("● marked"), "{text}");

    app.mark_block(); // `m` again on the same row
    assert_eq!(app.marked_ref(), None);
    let text = top_overlay_text(&mut app);
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
    let text = top_overlay_text(&mut app);
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
    assert!(top_overlay_text(&mut app).contains("● heads/main"));

    // **A ref change ends it.** A stale pair is worse than an empty one.
    app.nav_down();
    enter_twice(&mut app); // focus heads/feature
    assert_eq!(app.focused_ref(), Some("heads/feature"));
    assert_eq!(app.marked_ref(), None);
}

/// **prikk's plan is inert, and `↑`/`↓` scroll it** as Patch detail's do. `Enter` is the merge
/// (RFC 047 decision 6), and it is offered only from a `Confluent` plan.
#[test]
fn the_merge_plan_is_inert_and_scrolls() {
    let (mut app, rx) = merge_picker(
        merge_view(53, "heads/other", 0, None),
        Some("heads/feature"),
        0,
    );
    app.begin_merge_plan();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::MergePlan(Ok(stikk_core::MergePlanRead::Read(Box::new(
            stikk_core::MergePlanView {
                from: "heads/feature".into(),
                into: "heads/main".into(),
                baseline: "88f34150".into(),
                lines: vec!["merge plan".into()],
                outcome: stikk_prikk::MergeOutcome::Confluent,
            },
        )))),
    });
    let mut viewports = crate::viewports::Viewports::default();
    perform(&mut viewports, app.nav_down());
    match app.focus() {
        Focus::MergePlan(_, surface) => assert_eq!(viewports.of(surface).offset(), 1),
        other => panic!("expected the Merge plan, got {other:?}"),
    }
    perform(&mut viewports, app.nav_up());
    match app.focus() {
        Focus::MergePlan(_, surface) => assert_eq!(viewports.of(surface).offset(), 0),
        other => panic!("expected the Merge plan, got {other:?}"),
    }
    // `r` re-reads nothing: a plan is about the tips as they stood when it was read.
    app.reload();
    let reloaded = next_request(&rx);
    assert!(matches!(reloaded.kind, RequestKind::Orient));
    assert!(rx.try_recv().is_err(), "no second request for the plan");
}

/// **Capture: the into-current refusal at 80×24**, drawn through the same `shell::render` the run
/// loop calls — the banner is what the reader actually meets, so that is what is captured.
#[test]
fn capture_the_into_current_refusal_at_80x24() {
    let (mut app, rx) = merge_picker(
        merge_view(53, "heads/main", 0, None),
        Some("heads/feature"),
        0,
    );
    app.begin_merge_plan();
    assert!(rx.try_recv().is_err(), "nothing was spawned to say this");

    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).expect("a terminal");
    terminal
        .draw(|frame| {
            crate::shell::render(
                &app,
                &look(),
                &mut crate::viewports::Viewports::default(),
                frame,
            )
        })
        .expect("a draw");
    let text = crate::test_util::buffer_text(terminal.backend().buffer());
    println!("{text}");
    // **The whole sentence is on screen**, wrapped rather than clipped (RFC 024).
    for fragment in [
        "merging into the branch you are on",
        "would leave the worktree without the merged files",
        "prikk would then refuse a switch",
        "Switch to another branch first",
    ] {
        assert!(
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains(&fragment.split_whitespace().collect::<Vec<_>>().join(" ")),
            "missing {fragment:?} in:\n{text}"
        );
    }
}

/// **Verify's `↑`/`↓` move its report** (RFC 047 Handoff A review §6).
///
/// The renderer has always clamped an offset and its title has always offered `↑/↓` once the
/// report outgrew the panel — but no arm in `nav_up`/`nav_down` moved it, so the keys did
/// nothing and a long report's tail could not be reached. This drives the keys and then draws
/// through the same `shell::render` the run loop calls, because an offset that advances while
/// the screen does not would pass a test on the number alone.
#[test]
fn verifys_report_scrolls_when_it_is_taller_than_the_panel() {
    let (mut app, rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    app.open_verify();
    let seq = next_request(&rx).seq;
    // Twenty failing conditions: far taller than 24 rows, and the last one is the proof.
    let conditions: Vec<stikk_prikk::FailedCondition> = (0..20)
        .map(|n| stikk_prikk::FailedCondition {
            id: format!("condition-{n:02}"),
            message: format!("the {n:02}th thing prikk found wrong"),
        })
        .collect();
    app.apply(Response {
        seq,
        kind: ResponseKind::Verify(Ok(stikk_core::VerifyView::Checked(stikk_core::checked(
            stikk_prikk::Verification {
                verdict: stikk_prikk::Verdict {
                    ok: false,
                    failed_conditions: conditions,
                },
                active_sessions: None,
                stages: Vec::new(),
                raw: String::new(),
            },
        )))),
    });

    // **One set of offsets for the whole test** (RFC 067 decision 4): the presses and the draws
    // have to be about the same text, which is what the run loop arranges.
    let mut viewports = crate::viewports::Viewports::default();
    let top = shell_text(&mut app, &mut viewports, 24);
    assert!(top.contains("condition-00"), "{top}");
    assert!(
        !top.contains("condition-19"),
        "the tail is below the fold at 24 rows: {top}"
    );

    // `↓` advances the offset, and the screen moves with it.
    perform(&mut viewports, app.nav_down());
    match app.focus() {
        Focus::Verify(_, _, _, surface) => assert_eq!(viewports.of(surface).offset(), 1),
        other => panic!("expected Verify, got {other:?}"),
    }
    let scrolled = shell_text(&mut app, &mut viewports, 24);
    assert_ne!(scrolled, top, "the report moved");

    // And the tail is reachable: press until the renderer's own clamp parks it at the end.
    for _ in 0..200 {
        perform(&mut viewports, app.nav_down());
    }
    let end = shell_text(&mut app, &mut viewports, 24);
    assert!(end.contains("condition-19"), "{end}");

    // `↑` comes back, and never past the top.
    for _ in 0..500 {
        perform(&mut viewports, app.nav_up());
    }
    match app.focus() {
        Focus::Verify(_, _, _, surface) => assert_eq!(viewports.of(surface).offset(), 0),
        other => panic!("expected Verify, got {other:?}"),
    }
    assert_eq!(
        shell_text(&mut app, &mut viewports, 24),
        top,
        "back where it started"
    );

    // A report that fits does not scroll: the renderer clamps the press away rather than
    // banking it, the Glossary's own rule.
    let tall = shell_text(&mut app, &mut viewports, 60);
    perform(&mut viewports, app.nav_down());
    assert_eq!(
        shell_text(&mut app, &mut viewports, 60),
        tall,
        "nothing to scroll, nothing scrolled"
    );
}

/// **`Enter` does nothing on the Bundle view** -- the view is not a list and has nothing to
/// drill into; `I` is the one key that acts there (Handoff B §3b), and it is its own key for
/// that reason. `↑`/`↓` scroll it as they scroll Verify.
#[test]
fn rfc050_enter_does_nothing_on_the_bundle_view_and_the_arrows_scroll_it() {
    let (mut app, rx) = bundle_app(53);
    app.begin_open_bundle();
    type_in(&mut app, "/tmp/v1.bundle");
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::OpenBundle(Ok(stikk_core::BundleOpened::Refused {
            path: "/tmp/v1.bundle".to_string(),
            message: "error: malformed persisted data: invalid bundle magic".to_string(),
        })),
    });

    // **The right arrow dispatches nothing on a refused bundle** (RFC 050 Handoff A §4c): there is
    // nothing to open. `Enter` now opens the list, whose *Import this bundle* this very bundle
    // refuses, so the list is closed again before the arrows are tried.
    app.select_screen_or_row();
    assert!(rx.try_recv().is_err(), "the arrow dispatches nothing here");
    assert!(matches!(app.focus(), Focus::Bundle(..)));

    let mut viewports = crate::viewports::Viewports::default();
    let Focus::Bundle(_, surface) = app.focus() else {
        panic!("the Bundle view");
    };
    assert_eq!(viewports.of(surface).offset(), 0);
    perform(&mut viewports, app.nav_down());
    assert_eq!(
        viewports.of(surface).offset(),
        1,
        "the same keys that scroll Verify"
    );
    perform(&mut viewports, app.nav_up());
    assert_eq!(viewports.of(surface).offset(), 0);
}

/// **The security event is its own overlay, with no confirmation** (decision 4) — and `Enter`
/// on it sends nothing at all.
#[test]
fn the_security_event_has_no_confirmation_and_sends_nothing() {
    let (mut app, rx) = on_trust();
    app.begin_adopt_key();
    for ch in RFC056_NEW_KEY.chars() {
        app.input_char(ch);
    }
    app.select();
    app.select();
    let seq = next_request(&rx).seq;
    app.apply(Response {
        seq,
        kind: ResponseKind::TrustAdoptPreview(Ok(stikk_core::AdoptPreview::TwoKeys(Box::new(
            stikk_core::two_keys_warning(&stikk_core::TwoKeysOneId {
                key_id: RFC056_NEW_ID.to_string(),
                adopted: RFC056_THEIRS_KEY.to_string(),
                given: RFC056_NEW_KEY.to_string(),
            }),
        )))),
    });
    assert!(matches!(
        app.top_overlay(),
        Some(Overlay::TrustConflict { .. })
    ));
    let text = flattened(&top_overlay_text(&mut app));
    assert!(text.contains("Two keys cannot share an id"), "{text}");
    assert!(!text.contains("to confirm"), "nothing to approve: {text}");
    app.select();
    assert!(rx.try_recv().is_err(), "Enter does nothing here");
    app.back();
    assert!(app.top_overlay().is_none());
}

/// **The armed card is Tier Three Typed, and the typed id confirms it** (§7 trap 1) — then the
/// result carries prikk's lines and, for a removal, the pointer at Verify.
#[test]
fn a_removal_is_confirmed_by_typing_the_id_and_points_at_verify() {
    let (mut app, rx) = on_trust();
    app.nav_down();
    assert!(app.run_action("trust.remove"));
    let seq = next_request(&rx).seq;
    let backend = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys(vec![
            stikk_prikk::TrustedKey {
                key_id: RFC056_MINE.to_string(),
                public_key: RFC056_MINE_KEY.to_string(),
            },
            stikk_prikk::TrustedKey {
                key_id: RFC056_THEIRS.to_string(),
                public_key: RFC056_THEIRS_KEY.to_string(),
            },
        ])
        .with_trust_write(vec![
            "revoked maintainer key: ed25519-9c0a44b1d2e3f501".to_string(),
        ]);
    let preview = stikk_core::remove_preview(
        &backend,
        std::path::Path::new("/repo"),
        Readiness {
            read_only: false,
            ..Readiness::none()
        },
        RFC056_THEIRS,
    )
    .expect("the preview answers");
    let stikk_core::RemovePreview::Ready { token } = preview else {
        panic!("expected an armed removal");
    };
    app.apply(Response {
        seq,
        kind: ResponseKind::TrustRemovePreview(Ok(stikk_core::RemovePreview::Ready { token })),
    });
    match app.top_overlay() {
        Some(Overlay::Confirmation { tier, summary, .. }) => {
            assert_eq!(*tier, Tier::ThreeTyped, "RFC 056's first typed card");
            assert_eq!(summary.target_name.as_deref(), Some(RFC056_THEIRS));
        }
        other => panic!("expected a confirmation, got {other:?}"),
    }
    let card = flattened(&top_overlay_text(&mut app));
    assert!(card.contains("Verify will fail them"), "{card}");
    assert!(
        card.contains(&format!("Type \"{RFC056_THEIRS}\"")),
        "the typed evidence is the key id: {card}"
    );

    for ch in RFC056_THEIRS.chars() {
        app.input_char(ch);
    }
    app.select();
    let Request { seq, kind, .. } = next_request(&rx);
    match kind {
        RequestKind::TrustRemoveConfirmExecute {
            evidence, key_id, ..
        } => {
            assert_eq!(key_id, RFC056_THEIRS);
            assert!(
                matches!(evidence, stikk_core::Evidence::TypedName(ref typed, Seen::WholeCard) if typed == RFC056_THEIRS)
            );
        }
        other => panic!("expected the removal, got {other:?}"),
    }
    app.apply(Response {
        seq,
        kind: ResponseKind::TrustWrite(Ok(Outcome {
            operation: stikk_core::REMOVE_OPERATION.to_string(),
            result: stikk_core::TrustWriteResult {
                printed: vec!["revoked maintainer key: ed25519-9c0a44b1d2e3f501".to_string()],
                view: stikk_core::TrustView::Listed {
                    rows: vec![trust_row(RFC056_MINE, RFC056_MINE_KEY, true)],
                    own_key_unknown: None,
                },
                see_verify: Some(stikk_core::REMOVED_SEE_VERIFY),
                warning: None,
            },
        })),
    });
    let text = flattened(&top_overlay_text(&mut app));
    assert!(
        text.contains("revoked maintainer key"),
        "prikk's line: {text}"
    );
    // The view, not a key (RFC 061 decision 5).
    assert!(text.contains("Open Verify"), "{text}");
    // **The list beneath is the one that was re-read**, so the screen is not stale.
    // Through `focus()`, the public question: `App::screens` is the program's own business.
    match app.focus() {
        Focus::Trust(view, cursor, _) => {
            let stikk_core::TrustView::Listed { rows, .. } = view else {
                panic!("expected a list");
            };
            assert_eq!(rows.len(), 1, "the removed key is gone");
            assert_eq!(cursor, 0);
        }
        other => panic!("expected the Trust screen, got {other:?}"),
    }
}

/// **No key material anywhere these surfaces draw** (`NFR-S03`, §7 trap 4) — asserted against the
/// rendered buffer, not the model.
#[test]
fn no_trust_surface_draws_key_material() {
    let (mut app, _rx) = on_trust();
    let mut drawn = Vec::new();
    app.begin_adopt_key();
    drawn.push(top_overlay_text(&mut app));
    for ch in RFC056_NEW_KEY.chars() {
        app.input_char(ch);
    }
    app.select();
    drawn.push(top_overlay_text(&mut app));
    for text in drawn {
        for forbidden in ["seed", "SEED", "PRIKK_", "/.local/share", "private key"] {
            assert!(!text.contains(forbidden), "{forbidden} in {text}");
        }
    }
}

/// **The warning opens over the card, and `Esc` returns to the card** — which still lists what
/// prikk would refuse (RFC 063 decision 6).
#[test]
fn rfc063_an_invalid_seal_opens_the_warning_over_the_bundle_card() {
    let (mut app, rx) = bundle_app(58);
    bundle_screen_at_0_58(&mut app, &rx, true);
    assert!(
        matches!(app.top_overlay(), Some(Overlay::TrustConflict { .. })),
        "the security event is the first thing the reader meets"
    );
    app.close_overlay();
    assert!(!app.has_overlay(), "Esc leaves the card beneath");
    // The card beneath, drawn tall enough to hold all of it.
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 60)).unwrap();
    terminal
        .draw(|f| {
            crate::shell::render(
                &app,
                &look(),
                &mut crate::viewports::Viewports::default(),
                f,
            )
        })
        .unwrap();
    let text = crate::test_util::buffer_text(terminal.backend().buffer());
    assert!(
        flattened(&text).contains("prikk would refuse this import"),
        "and the card still says why: {text}"
    );
}

/// **prikk's own key-id conflict opens the security warning, not the generic gloss** (RFC 063
/// decision 7) — including for a **removed** id, which stikk's own guard cannot see, so the
/// preview is armed and prikk is the one who refuses.
#[test]
fn rfc063_a_refused_adopt_opens_the_warning_card() {
    let (mut app, rx) = on_trust();
    // Arm the adopt the way the removal test arms its own: a real preview, from a backend whose
    // list does not hold this id -- which is exactly the removed-id case.
    let backend = NullBackend::supported()
        .with_version(0, 56, 0)
        .with_trusted_keys(vec![stikk_prikk::TrustedKey {
            key_id: RFC056_MINE.to_string(),
            public_key: RFC056_MINE_KEY.to_string(),
        }]);
    let stikk_core::AdoptPreview::Ready { token } = stikk_core::adopt_preview(
        &backend,
        std::path::Path::new("/repo"),
        Readiness {
            read_only: false,
            ..Readiness::none()
        },
        RFC056_NEW_ID,
        RFC056_NEW_KEY,
    )
    .expect("the preview answers") else {
        panic!("expected an armed adopt: the id is not in the list");
    };
    app.begin_adopt_key();
    type_in(&mut app, RFC056_NEW_KEY);
    read_the_card(&mut app);
    app.select();
    read_the_card(&mut app);
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::TrustAdoptPreview(Ok(stikk_core::AdoptPreview::Ready { token })),
    });
    type_in(&mut app, RFC056_NEW_ID);
    read_the_card(&mut app);
    app.select();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::TrustWrite(Ok(Outcome {
            operation: stikk_core::ADOPT_OPERATION.to_string(),
            result: stikk_core::TrustWriteResult {
                printed: Vec::new(),
                view: stikk_core::TrustView::Listed {
                    rows: vec![trust_row(RFC056_MINE, RFC056_MINE_KEY, true)],
                    own_key_unknown: None,
                },
                see_verify: None,
                warning: Some(Box::new(stikk_core::key_id_conflict_warning(
                    stikk_prikk::KeyIdConflict::BoundToRemovedKey,
                    RFC056_NEW_ID,
                    RFC056_NEW_KEY,
                    "precondition not met: maintainer key id is not currently adopted (it was \
                     removed), but it stays bound to the public key it was first adopted with"
                        .to_string(),
                ))),
            },
        })),
    });
    let text = flattened(&top_overlay_text(&mut app));
    assert!(
        matches!(app.top_overlay(), Some(Overlay::TrustConflict { .. })),
        "the security event, not the generic gloss: {text}"
    );
    assert!(
        text.contains(stikk_core::ID_REMOVED_BUT_BOUND),
        "and it says why the id is not in the list: {text}"
    );
    assert!(text.contains("prikk refused"), "prikk's own words: {text}");
}

/// **The confirm key does nothing, and dispatches nothing, until the card has been read to the end**
/// (RFC 060 decision 3) — then it works.
///
/// Driven through the request recorder: the proof that nothing happened is that no request was sent.
#[test]
fn enter_on_an_unread_confirmation_dispatches_nothing() {
    // One set of offsets, drawn through and reported from: the run loop's own arrangement.
    let mut viewports = crate::viewports::Viewports::default();
    let (mut app, rx) = commit_awaiting_confirmation();
    // Drawn at a size the card cannot fit in, which is what leaves it unread.
    let small = draw_top_overlay_at(&mut app, &mut viewports, 60, 10);
    assert!(
        small.contains("read to the end to confirm"),
        "the card says what is missing:\n{small}"
    );

    app.select();
    assert!(
        rx.try_recv().is_err(),
        "an unread card must dispatch nothing at all"
    );
    assert!(
        matches!(app.top_overlay(), Some(Overlay::Confirmation { .. })),
        "and the card stays open"
    );

    // Now read it: `↓` to the end, as a reader would.
    for _ in 0..200 {
        let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 10);
        if app.at_end() {
            break;
        }
        perform(&mut viewports, app.nav_down());
    }
    let read = draw_top_overlay_at(&mut app, &mut viewports, 60, 10);
    assert!(
        read.contains("Enter to confirm"),
        "and now it is offered:\n{read}"
    );

    app.select();
    let request = next_request(&rx);
    assert!(
        matches!(request.kind, RequestKind::CommitConfirmExecute { .. }),
        "the confirmation went through: {:?}",
        request.kind
    );
}

/// **`↓` scrolls the text before it moves the highlight, and `↑` comes back the same way** (RFC 060
/// decision 3's one reading path), on a card that has both text and an action list.
#[test]
fn down_scrolls_the_text_before_it_moves_the_highlight() {
    // One set of offsets, drawn through and reported from: the run loop's own arrangement.
    let mut viewports = crate::viewports::Viewports::default();
    let (mut app, _rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    app.push_overlay_for_test(Overlay::Refusal {
        card: stikk_core::RefusalCard {
            verbatim: (1..=20)
                .map(|n| format!("prikk said line {n} of something long"))
                .collect::<Vec<_>>()
                .join("\n"),
            gloss: None,
            glossary_codes: Vec::new(),
            next_steps: vec![
                stikk_core::NextStep {
                    label: "Choose another ref".to_string(),
                    target: stikk_core::NextTarget::OpenView(stikk_core::Target::RefPicker),
                },
                stikk_core::NextStep {
                    label: "Refresh".to_string(),
                    target: stikk_core::NextTarget::Refresh,
                },
            ],
        },
        cursor: 0,
    });
    let cursor = |app: &App| match app.top_overlay() {
        Some(Overlay::Refusal { cursor, .. }) => *cursor,
        other => panic!("expected the refusal card, got {other:?}"),
    };

    // The text does not fit at this height, so the first `↓`s scroll it and the highlight stays put.
    let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 12);
    assert_eq!(cursor(&app), 0);
    let mut scrolls = 0;
    while !app.at_end() {
        perform(&mut viewports, app.nav_down());
        let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 12);
        scrolls += 1;
        assert_eq!(cursor(&app), 0, "the highlight waits for the text");
        assert!(scrolls < 100);
    }
    assert!(scrolls > 0, "the card had text to scroll");

    // At the end, `↓` moves into the actions.
    perform(&mut viewports, app.nav_down());
    let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 12);
    assert_eq!(cursor(&app), 1, "and only then the highlight moves");

    // `↑` walks back through the actions to the first, and then up the text.
    perform(&mut viewports, app.nav_up());
    let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 12);
    assert_eq!(cursor(&app), 0);
    perform(&mut viewports, app.nav_up());
    let _ = draw_top_overlay_at(&mut app, &mut viewports, 60, 12);
    assert_eq!(
        cursor(&app),
        0,
        "the cursor stays while the text comes back"
    );
    assert!(
        app.more_above() || scrolls == 1,
        "the text scrolled back up"
    );
}

/// **A new card is a new reader** (RFC 060 decision 3): the latch belongs to the overlay that earned
/// it, and does not pass to the next one.
#[test]
fn the_reading_latch_does_not_outlive_its_card() {
    let (mut app, _rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    app.open_glossary();
    let mut viewports = crate::viewports::Viewports::default();
    let _ = draw_top_overlay_at(&mut app, &mut viewports, 90, 240); // the whole Glossary on screen
    assert!(app.seen_whole_card(), "the whole Glossary was on screen");
    app.back();
    app.open_refusals();
    assert!(
        !app.seen_whole_card(),
        "the next card starts unread, whatever was read before it"
    );
}

/// **What the list is titled, per row kind** (decision 3): the row itself, in stikk's word and the
/// repository's own string.
#[test]
fn each_row_kinds_list_is_titled_by_its_row() {
    // A block, in History.
    let (mut app, rx) = from_state("/repo", loaded(files_orientation_view()));
    app.select_screen_or_row();
    let req = next_request(&rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    app.select();
    let text = top_overlay_text(&mut app);
    assert!(text.contains("Block bbbb"), "{text}");
    assert!(text.contains("Open this block"), "{text}");
    assert!(text.contains("Mark it, to compare"), "{text}");
    // Its key comes from the catalog, and nothing else names it.
    assert!(text.contains('m'), "{text}");

    // A file, in Block detail: its own two actions, then the block's.
    let (mut app, rx) = from_state("/repo", loaded(files_orientation_view()));
    block_with_files(&mut app, &rx, 55);
    app.nav_down();
    app.select();
    let text = top_overlay_text(&mut app);
    // `block_with_files` leaves the cursor on the block's first row; one down is its second file.
    assert!(text.contains("File run.sh"), "{text}");
    assert!(text.contains("Save this file, as it was here"), "{text}");
    assert!(text.contains("Save every file at this block"), "{text}");
    // **Not the patch rows' action**, which is what the rows above offer.
    assert!(!text.contains("Open this patch"), "{text}");
}

/// **The offsets do not pile up** (review v1 F1).
///
/// A surface is minted by every card opened, so a reader who opens and closes a card a hundred
/// times mints a hundred surfaces. Ninety-nine of them are unreachable for ever, and the drawing
/// must not still be holding offsets for them: `shell::render` prunes to what the program says is
/// live.
#[test]
fn a_card_opened_and_closed_a_hundred_times_leaves_no_offsets_behind() {
    use ratatui::crossterm::event::KeyCode;

    let (mut app, _rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    let mut viewports = crate::viewports::Viewports::default();

    for _ in 0..100 {
        crate::keys::handle_key(&mut app, key(KeyCode::Char('?')));
        let _ = shell_text(&mut app, &mut viewports, 24);
        assert!(app.top_overlay().is_some(), "the card opened");
        crate::keys::handle_key(&mut app, key(KeyCode::Char('q')));
        let _ = shell_text(&mut app, &mut viewports, 24);
        assert!(app.top_overlay().is_none(), "and closed again");
    }

    let live = app.live_surfaces();
    assert!(
        viewports.len() <= live.len(),
        "the drawing holds {} offsets for {} live surfaces",
        viewports.len(),
        live.len()
    );
    // On the Orientation root with no card on top there is nothing to scroll at all, so the sharp
    // form of the same statement: a hundred cards left nothing behind.
    assert!(live.is_empty(), "nothing is live here");
    assert_eq!(viewports.len(), 0, "so nothing is held");
}

/// **Pruning does not forget what is still live** (review v1 F1): a scrolled view keeps its place
/// while a card is open over it, and while the reader walks into a deeper screen and back.
///
/// **Verify, not Block detail** (which review v1 F1 suggested). A Block detail with rows takes its
/// offset from its cursor -- the renderer scrolls to follow the selection -- and the cursor is on
/// the screen, which is program state that survives anything the drawing forgets. So its offset is
/// recomputed from the cursor even when the entry *is* dropped, and the test would pass against a
/// `retain` that keeps only the current surface. I checked: it does. Verify scrolls by effect
/// alone, so its offset exists nowhere but here, and dropping the entry loses the reader's place.
///
/// The offset is asserted, not a frame: the frames are what the golden manifest is for, and what
/// this is about is the number the drawing holds.
#[test]
fn a_scrolled_view_keeps_its_offset_under_a_card_and_a_deeper_screen() {
    use ratatui::crossterm::event::KeyCode;

    let (mut app, rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    app.open_verify();
    let seq = next_request(&rx).seq;
    // Twenty failing conditions: far taller than the window, so there is somewhere to scroll to.
    let conditions: Vec<stikk_prikk::FailedCondition> = (0..20)
        .map(|n| stikk_prikk::FailedCondition {
            id: format!("condition-{n:02}"),
            message: format!("the {n:02}th thing prikk found wrong"),
        })
        .collect();
    app.apply(Response {
        seq,
        kind: ResponseKind::Verify(Ok(stikk_core::VerifyView::Checked(stikk_core::checked(
            stikk_prikk::Verification {
                verdict: stikk_prikk::Verdict {
                    ok: false,
                    failed_conditions: conditions,
                },
                active_sessions: None,
                stages: Vec::new(),
                raw: String::new(),
            },
        )))),
    });

    let mut viewports = crate::viewports::Viewports::default();
    let surface = match app.focus() {
        Focus::Verify(_, _, _, surface) => surface,
        other => panic!("expected Verify, got {other:?}"),
    };

    // Five presses of the key path, each carried out as the run loop carries it out.
    let _ = shell_text(&mut app, &mut viewports, 24);
    for _ in 0..5 {
        viewports.apply(crate::keys::handle_key(&mut app, key(KeyCode::Down)));
        let _ = shell_text(&mut app, &mut viewports, 24);
    }
    let scrolled = viewports.of(surface).offset();
    assert!(scrolled > 0, "the report scrolled, or this proves nothing");

    // A card opened over it, and closed again.
    crate::keys::handle_key(&mut app, key(KeyCode::Char('?')));
    let _ = shell_text(&mut app, &mut viewports, 24);
    assert!(app.top_overlay().is_some(), "a card is on top");
    crate::keys::handle_key(&mut app, key(KeyCode::Char('q')));
    let _ = shell_text(&mut app, &mut viewports, 24);
    assert_eq!(
        viewports.of(surface).offset(),
        scrolled,
        "a card opened over a view does not lose the reader's place"
    );

    // A deeper screen entered, and left.
    app.open_trust();
    let seq = next_request(&rx).seq;
    app.apply(Response {
        seq,
        kind: ResponseKind::TrustRead(Ok(stikk_core::TrustView::Listed {
            rows: vec![trust_row(RFC056_MINE, RFC056_MINE_KEY, true)],
            own_key_unknown: None,
        })),
    });
    let _ = shell_text(&mut app, &mut viewports, 24);
    assert!(
        matches!(app.focus(), Focus::Trust(..)),
        "a deeper screen is on top"
    );
    app.back();
    let _ = shell_text(&mut app, &mut viewports, 24);
    assert!(
        matches!(app.focus(), Focus::Verify(..)),
        "and the reader is back on the report"
    );
    assert_eq!(
        viewports.of(surface).offset(),
        scrolled,
        "a deeper screen entered and left does not lose the reader's place"
    );
}

/// **The drawing's latch and the program's say the same thing, after every report.**
///
/// RFC 067 gives the evidence one source: `Seen::WholeCard` is made in the program, from a report.
/// The drawing keeps a latch of its own for a different question -- may the confirm line *say* it is
/// ready, about the frame it has just laid out -- and the argument that the two cannot disagree is
/// that the run loop reports immediately after each draw and before the next key. **An argument is
/// not a test**, so this is the test: drive a card taller than the window through that cycle, and
/// after every report hold the two to each other.
#[test]
fn the_two_latches_agree_after_every_report_on_a_card_taller_than_the_window() {
    use ratatui::crossterm::event::KeyCode;

    let (mut app, _rx) = from_state("/repo", loaded(author_orientation_view()));
    app.push_overlay_for_test(Overlay::Confirmation {
        summary: Box::new(long_confirmation()),
        tier: Tier::Two,
        typed: String::new(),
        error: None,
    });
    let mut viewports = crate::viewports::Viewports::default();

    let mut scrolls = 0;
    loop {
        // Draw, then report: the run loop's order.
        let _ = shell_text_at(&mut app, &mut viewports, 60, 15);
        let surface = app.reading_surface().expect("a card is on top");
        assert_eq!(
            app.seen_whole_card(),
            viewports.of(surface).seen_whole_card(),
            "after {scrolls} scrolls the program and the drawing disagree about whether the card \
             has been read"
        );
        if app.at_end() {
            break;
        }
        viewports.apply(crate::keys::handle_key(&mut app, key(KeyCode::Down)));
        scrolls += 1;
        assert!(scrolls < 200, "the card never reached its end");
    }
    assert!(
        scrolls > 0,
        "the card must not fit at 60x15, or this proves nothing about scrolling"
    );
    assert!(app.seen_whole_card(), "and reaching the end reads it");

    // **Past the end**, where `↓` moves the highlight instead of the text: still agreeing.
    for _ in 0..3 {
        viewports.apply(crate::keys::handle_key(&mut app, key(KeyCode::Down)));
        let _ = shell_text_at(&mut app, &mut viewports, 60, 15);
        let surface = app.reading_surface().expect("a card is on top");
        assert_eq!(
            app.seen_whole_card(),
            viewports.of(surface).seen_whole_card(),
            "past the end the two must still agree"
        );
    }
}

/// The same, for a card that fits on its first draw: **both latches are true in that first frame**.
///
/// This is the case that made the two latches necessary. The report arrives after the draw, so a
/// program-only latch would have the confirm line say *"read to the end to confirm"* on a card
/// whose every row is on screen, and correct itself one frame later -- a flicker, and two golden
/// frames that would have changed.
#[test]
fn the_two_latches_agree_on_the_first_draw_of_a_card_that_fits() {
    let (mut app, _rx) = commit_awaiting_confirmation();
    let mut viewports = crate::viewports::Viewports::default();

    // The commit card is thirteen rows at 60 columns, so at 80 x 60 every row is on screen in the
    // frame it is first drawn in.
    let _ = shell_text(&mut app, &mut viewports, 60);
    let surface = app.reading_surface().expect("a card is on top");
    assert!(
        viewports.of(surface).seen_whole_card(),
        "the card fits at 80x60, so the drawing latched it in the frame it drew"
    );
    assert_eq!(
        app.seen_whole_card(),
        viewports.of(surface).seen_whole_card(),
        "and the report that followed told the program the same"
    );
}

const RFC056_MINE: &str = "ed25519-31f369d3abb7f64c";

const RFC056_MINE_KEY: &str = "31f369d3abb7f64cd2cf70be31158181e361f25444d8de122cdff59389e7e946";

const RFC056_NEW_ID: &str = "ed25519-6762b6359a650c0a";

const RFC056_NEW_KEY: &str = "6762b6359a650c0a5acaa723a5fc2ad78b354fcac296495b9cbf46c0d32a3f7f";

const RFC056_THEIRS: &str = "ed25519-9c0a44b1d2e3f501";

const RFC056_THEIRS_KEY: &str = "9c0a44b1d2e3f5012b7c0a4e55d2117ab3340ef6627c118e2b4a9d0cf3e21847";

fn answer_reload(app: &mut App, rx: &mpsc::Receiver<Request>, view: stikk_core::OrientationView) {
    app.reload();
    let req = next_request(rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(Ok(view))),
    });
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

/// A Block detail with two patches and two files, pushed straight onto the screen stack.
fn block_with_files(app: &mut App, rx: &mpsc::Receiver<Request>, minor: u32) {
    let _ = minor;
    // The right arrow, twice: open the focused ref's history, then its tip block (RFC 061
    // decision 2). `Enter` now asks what can be done first.
    app.select_screen_or_row();
    let req = next_request(rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::History(Ok(two_block_history())),
    });
    app.select_screen_or_row();
    let req = next_request(rx);
    let detail = BlockDetailView {
        row: block("bbbb", 2),
        is_tip: true,
        state: stikk_core::BlockState::Listed(StateFiles {
            target_block: "bbbb".into(),
            files: vec![
                stikk_prikk::StateFile::file("a.txt", 0o100_644),
                stikk_prikk::StateFile::file("run.sh", 0o100_755),
            ],
            total_bytes: 22,
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
}

fn branch(name: &str) -> CurrentBranch {
    CurrentBranch::Branch(RefName::parse(name).expect("a valid ref name"))
}

/// A session that may open a bundle: prikk 0.53 and nothing else required -- no key, no branch.
fn bundle_app(minor: u32) -> (App, mpsc::Receiver<Request>) {
    let mut view = orientation_view(0, None, Some("abc123"));
    view.prikk_minor = minor;
    from_state("/repo", loaded(view))
}

/// The Bundle view as prikk 0.58 reads it, with an `invalid` seal and prikk's own refusal.
fn bundle_screen_at_0_58(app: &mut App, rx: &mpsc::Receiver<Request>, refused: bool) {
    let verified: Vec<String> = vec![
        "bundle verifies: heads/main".to_string(),
        format!("tip block: {TIP}"),
    ];
    let mut preview = stikk_prikk::BundlePreviewRead {
        bundle_ref: "heads/main".to_string(),
        local_ref: "heads/main".to_string(),
        connectivity: stikk_prikk::Connectivity::FastForward,
        conflict: Some(stikk_prikk::BundleConflict::AppliesCleanly),
        conflict_detail: None,
        sealed_by: vec!["shared".to_string()],
        sealed_by_note: "recorded signer identity, continuity only -- not a trust decision"
            .to_string(),
        effects: Vec::new(),
        rename_note: "a moved path is not yet distinguishable".to_string(),
        seal_outcomes: Some(vec![stikk_prikk::SealOutcome {
            key_id: "shared".to_string(),
            outcome: if refused {
                stikk_prikk::SealVerdict::Invalid
            } else {
                stikk_prikk::SealVerdict::Sound
            },
        }]),
        import_refusals: Some(Vec::new()),
    };
    if refused {
        preview.import_refusals = Some(vec![stikk_prikk::ImportRefusal {
            id: "forged-publication-signature".to_string(),
            message: "precondition not met: ref-state carries a MAINTAINER signature by adopted \
                      key shared that does not verify"
                .to_string(),
        }]);
    }
    let warning =
        refused.then(|| stikk_core::invalid_seal_warning("shared", Some(&"e".repeat(64))));
    app.begin_open_bundle();
    type_in(app, "/tmp/v1.bundle");
    app.select();
    let req = next_request(rx);
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::OpenBundle(Ok(stikk_core::BundleOpened::Read(Box::new(
            stikk_core::BundleView {
                path: "/tmp/v1.bundle".to_string(),
                verified,
                preview,
                sentence: stikk_core::WOULD_BRING_FORWARD,
                sealers: stikk_core::SealerTrustRead::Judged(vec![stikk_core::Sealer {
                    key_id: "shared".to_string(),
                    trust: stikk_core::SealerTrust::Checked(if refused {
                        stikk_prikk::SealVerdict::Invalid
                    } else {
                        stikk_prikk::SealVerdict::Sound
                    }),
                    trusted_here: Some("e".repeat(64)),
                }]),
                warning,
            },
        )))),
    });
}

/// An app with a commit's confirmation card open and its token armed — the shape
/// `confirming_a_commit_dispatches_confirm_execute_and_a_success_shows_the_result_verbatim` drives,
/// stopped one key earlier.
fn commit_awaiting_confirmation() -> (App, std::sync::mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(author_orientation_view()));
    app.begin_commit();
    app.input_char('x');
    read_the_card(&mut app);
    app.select(); // the message step -> a preview request
    let preview = next_request(&rx);
    let backend = commit_backend();
    let outcome = commit_preview(&backend, std::path::Path::new("/repo"), "heads/main")
        .expect("preview succeeds");
    app.apply(Response {
        seq: preview.seq,
        kind: ResponseKind::CommitPreview(Ok(outcome)),
    });
    (app, rx)
}

/// Draw the top overlay at an explicit size, so a test can put the app in front of a window too
/// small for its card.
fn draw_top_overlay_at(
    app: &mut App,
    viewports: &mut crate::viewports::Viewports,
    w: u16,
    h: u16,
) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
    terminal
        .draw(|f| {
            crate::overlay::render(
                app.top_overlay().unwrap(),
                app.marked_ref(),
                &palette(),
                viewports.of(app.reading_surface().expect("a card on top is a surface")),
                f,
                f.area(),
            )
        })
        .unwrap();
    if let Some(reading) = app.reading_surface().and_then(|s| viewports.reading(s)) {
        app.report_reading(reading);
    }
    crate::test_util::buffer_text(terminal.backend().buffer())
}

/// **Read the card on top the way a reader does: by looking at it** (RFC 060 decision 3).
///
/// A confirmation is offered only once its whole text has been on screen, and the renderer is what
/// latches that — so a test that presses `Enter` without ever drawing the card is a reader who
/// confirmed with their eyes shut, and `select` rightly does nothing. At 80 × 24 every card in these
/// tests fits, so one draw is all it takes.
/// **Enter, then Enter** — what `Enter` alone did before RFC 061 decision 3: the first opens the
/// action list for the selected row, and the second runs its first entry, which is the row's own
/// open action.
fn enter_twice(app: &mut App) {
    app.select();
    app.select();
}

/// An Orientation at a prikk that offers the saves.
fn files_orientation_view() -> stikk_core::OrientationView {
    stikk_core::OrientationView {
        prikk_minor: 55,
        ..orientation_view(0, None, None)
    }
}

/// A rendered panel with its borders dropped and its whitespace collapsed, so an assertion can
/// name a sentence the card wrapped across rows.
fn flattened(screen: &str) -> String {
    screen
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{2502}' | '\u{250c}' | '\u{2510}' | '\u{2514}' | '\u{2518}' | '\u{2500}'
            )
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn from_state(repo: &str, state: OrientationState) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::from_state(repo, state, tx), rx)
}

/// A key event, for a test that drives the program the way the run loop does.
fn key(code: ratatui::crossterm::event::KeyCode) -> ratatui::crossterm::event::KeyEvent {
    ratatui::crossterm::event::KeyEvent::from(code)
}

fn loaded(view: stikk_core::OrientationView) -> OrientationState {
    OrientationState::Loaded(view)
}

/// A confirmation whose consequence is too long to fit any window these tests use — the shape the
/// golden module's `scene_confirmation` draws, which is why it exists there too.
fn long_confirmation() -> stikk_core::ConfirmationSummary {
    stikk_core::ConfirmationSummary {
        operation: "Commit worktree changes".to_string(),
        preamble: Some("What a commit will author:".to_string()),
        target_ids: Vec::new(),
        counts: vec![("files", 9)],
        capability: Some(Capability::Author),
        consequence: (1..=14)
            .map(|n| format!("Consequence line {n}: what this will do, said once and in full."))
            .collect::<Vec<_>>()
            .join(" "),
        target_name: Some("heads/main".to_string()),
        signing_key_id: Some("author-1".to_string()),
        signing_key_claim: stikk_core::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        notices: Vec::new(),
        message_note: None,
        materializes: None,
        undo_plan: None,
        merge_plan: None,
        asides: Vec::new(),
    }
}

/// The colours a whole-shell drawing test draws in, from the same [`palette`].
fn look() -> crate::theme::Resolved {
    crate::theme::Resolved::drawing_with(palette())
}

/// The picker `M` is pressed in, with `mark` already marked and the cursor on `cursor`.
fn merge_picker(
    view: stikk_core::OrientationView,
    mark: Option<&str>,
    cursor: usize,
) -> (App, mpsc::Receiver<Request>) {
    let rows = vec![
        entry("heads/main", false),
        entry("heads/feature", false),
        entry("heads/old", true),
        entry("tags/v1", false),
    ];
    let (mut app, rx) = picker(view, rows.clone(), stikk_prikk::UnpublishedBranch::None, 0);
    if let Some(mark) = mark {
        let at = rows
            .iter()
            .position(|row| row.name == mark)
            .expect("a row to mark");
        for _ in 0..at {
            app.nav_down();
        }
        app.mark_block(); // `m` — the picker is open, so it marks a ref
        assert_eq!(app.marked_ref(), Some(mark));
        // Back to the top, then down to the row `M` is pressed on.
        for _ in 0..at {
            app.nav_up();
        }
    }
    for _ in 0..cursor {
        app.nav_down();
    }
    (app, rx)
}

/// A session that may merge: prikk 0.53, a current branch, and whatever the queue holds.
fn merge_view(
    minor: u32,
    current: &str,
    queued: u64,
    target: Option<&str>,
) -> stikk_core::OrientationView {
    let mut view = maintainer_orientation_view(queued, target);
    view.prikk_minor = minor;
    view.current_branch = stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse(current).expect("a ref name"),
    );
    view
}

/// Pull the one request expected to have been sent so far. Panics (with a clear message) if none was
/// sent — exactly what a broken action-to-request mapping should do to a test.
fn next_request(rx: &mpsc::Receiver<Request>) -> Request {
    rx.try_recv().expect("expected a request to have been sent")
}

/// An app on the Trust view, with two adopted keys.
fn on_trust() -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(trust_band_orientation()));
    app.open_trust();
    let Request { seq, kind, .. } = next_request(&rx);
    assert!(matches!(kind, RequestKind::TrustRead), "{kind:?}");
    app.apply(Response {
        seq,
        kind: ResponseKind::TrustRead(Ok(stikk_core::TrustView::Listed {
            rows: vec![
                trust_row(RFC056_MINE, RFC056_MINE_KEY, true),
                trust_row(RFC056_THEIRS, RFC056_THEIRS_KEY, false),
            ],
            own_key_unknown: None,
        })),
    });
    (app, rx)
}

/// Open an app and answer its first Orientation read with `result`.
fn open_with_first_read(
    result: stikk_model::Result<stikk_core::OrientationView>,
) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = open("/repo");
    assert_eq!(app.ref_focus(), &RefFocus::Pending);
    let req = next_request(&rx);
    assert!(matches!(req.kind, RequestKind::Orient));
    app.apply(Response {
        seq: req.seq,
        kind: ResponseKind::Orient(OrientRead::stamped(result)),
    });
    (app, rx)
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

/// **Press a key and carry out what the program asked for**, as the run loop does (RFC 067
/// decision 4): the program decides, the front end moves the text.
fn perform(viewports: &mut crate::viewports::Viewports, effect: stikk_app::Effect) {
    viewports.apply(effect);
}

/// A picker holding three refs, with the cursor on the first.
fn picker_with_refs(names: &[&str]) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(orientation_view(0, None, None)));
    app.open_ref_picker();
    let refs_req = next_request(&rx);
    app.apply(Response {
        seq: refs_req.seq,
        kind: ResponseKind::Refs(Ok(ref_list(names.iter().copied().map(ref_entry).collect()))),
    });
    (app, rx)
}

fn read_the_card(app: &mut App) {
    // Nothing on top is nothing to read: the key under test is then a screen's, not a card's.
    if app.top_overlay().is_some() {
        let _ = top_overlay_text(app);
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

/// A `RefList` with no unpublished current branch reported — the shape every test here used
/// before RFC 037, so each one keeps asserting exactly what it asserted.
fn ref_list(refs: Vec<stikk_prikk::RefEntry>) -> stikk_core::RefList {
    stikk_core::RefList {
        refs,
        unpublished_current: stikk_prikk::UnpublishedBranch::NotReported,
    }
}

/// Draw the whole shell at 80 x `height` through `viewports`, then report what was seen — the run
/// loop's order (RFC 067 decision 4), for a test whose presses and draws are about the same text.
fn shell_text(app: &mut App, viewports: &mut crate::viewports::Viewports, height: u16) -> String {
    shell_text_at(app, viewports, 80, height)
}

/// As [`shell_text`], at an explicit width — for a card that only overflows once it has to wrap.
fn shell_text_at(
    app: &mut App,
    viewports: &mut crate::viewports::Viewports,
    width: u16,
    height: u16,
) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
        .expect("a terminal");
    terminal
        .draw(|frame| crate::shell::render(app, &look(), viewports, frame))
        .expect("a draw");
    if let Some(reading) = app.reading_surface().and_then(|s| viewports.reading(s)) {
        app.report_reading(reading);
    }
    crate::test_util::buffer_text(terminal.backend().buffer())
}

/// The status bar at 120 columns: wide enough that nothing is shed, so these tests see prikk's text
/// whole. What the line sheds when it is full is `status_bar::tests`' subject.
fn status_line(app: &App) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 1)).unwrap();
    terminal
        .draw(|f| crate::status_bar::render(app, &palette(), f, f.area()))
        .unwrap();
    crate::test_util::buffer_text(terminal.backend().buffer())
}

fn top_overlay_text(app: &mut App) -> String {
    draw_card(app, &mut crate::viewports::Viewports::default(), 80, 24)
}

fn trust_row(id: &str, public: &str, yours: bool) -> stikk_core::TrustRow {
    stikk_core::TrustRow {
        key_id: id.to_string(),
        public_key: public.to_string(),
        yours,
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

fn type_in(app: &mut App, text: &str) {
    for ch in text.chars() {
        app.input_char(ch);
    }
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

const TIP: &str = "ff00000000000000000000000000000000000000000000000000000000000000";

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

/// **Draw the top card, then tell the program what was seen** — one turn of the run loop's own
/// order (RFC 067 decision 4 / handoff §4.5).
///
/// The reading is no longer latched by drawing into state the program owns: the drawing measures,
/// reports, and the program decides. A test that draws a card and then presses `Enter` on it has to
/// do both halves, because that is what the run loop does between the two.
fn draw_card(app: &mut App, viewports: &mut crate::viewports::Viewports, w: u16, h: u16) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
    terminal
        .draw(|f| {
            crate::overlay::render(
                app.top_overlay().unwrap(),
                app.marked_ref(),
                &palette(),
                viewports.of(app.reading_surface().expect("a card on top is a surface")),
                f,
                f.area(),
            )
        })
        .unwrap();
    if let Some(reading) = app.reading_surface().and_then(|s| viewports.reading(s)) {
        app.report_reading(reading);
    }
    crate::test_util::buffer_text(terminal.backend().buffer())
}

fn entry(name: &str, closed: bool) -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: name.to_string(),
        id: "e".repeat(64),
        closed,
        received: false,
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

fn open(repo: &str) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::open(repo, None, tx), rx)
}

/// The colours a drawing test draws in.
///
/// **`App` holds none** (RFC 067 decision 3), so a test that draws says which colours it wants,
/// exactly as the run loop does.
fn palette() -> crate::theme::Palette {
    crate::theme::Palette::default()
}

/// The picker, open and populated, with `cursor` on the row the keys will act on.
fn picker(
    view: stikk_core::OrientationView,
    rows: Vec<stikk_prikk::RefEntry>,
    unpublished: stikk_prikk::UnpublishedBranch,
    cursor: usize,
) -> (App, mpsc::Receiver<Request>) {
    let (mut app, rx) = from_state("/repo", loaded(view));
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

/// An Orientation in the trust band, with an operator who may write.
fn trust_band_orientation() -> stikk_core::OrientationView {
    let readiness = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    stikk_core::OrientationView {
        prikk_version: "prikk 0.55.0".to_string(),
        prikk_minor: 55,
        capability: Capability::derive(readiness),
        readiness,
        ..orientation_view(0, None, None)
    }
}

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

// ---------------------------------------------------------------------------------------------
// The reading gate holds again (RFC 067 Handoff C; the 0.22.0 regression)
// ---------------------------------------------------------------------------------------------

/// **A card hidden behind the "window too small" notice cannot be confirmed** (Handoff C §4).
///
/// At 60 × 14 `shell::render` draws the too-small notice and returns before any card is laid out.
/// In 0.22.0 the run loop reported *"the end is on screen"* anyway, the program latched it, and one
/// `Enter` committed a card nobody had seen. The real binary did exactly that: `60x14 … queued
/// patches 1`, where 0.21.1 gave 0.
#[test]
fn a_card_behind_the_too_small_notice_cannot_be_confirmed() {
    let (mut app, rx) = commit_awaiting_confirmation();
    let mut viewports = crate::viewports::Viewports::default();

    // One turn of the run loop at a window too small to draw anything but the notice.
    let notice = shell_text_at(&mut app, &mut viewports, 60, 14);
    assert!(
        !notice.contains("Confirm"),
        "the card is not on screen at 60x14:\n{notice}"
    );
    assert!(
        !app.seen_whole_card(),
        "and nothing drew it, so nothing can have read it"
    );

    crate::keys::handle_key(&mut app, key(ratatui::crossterm::event::KeyCode::Enter));
    assert!(
        rx.try_recv().is_err(),
        "a card nobody has seen must dispatch nothing at all"
    );
    assert!(
        matches!(app.top_overlay(), Some(Overlay::Confirmation { .. })),
        "and the card stays open, found {:?}",
        app.top_overlay()
    );
}

/// **The two latches agree through a Loading → Confirmation swap** (Handoff C §4).
///
/// This is the case the 0.22.0 agreement test never drove, and it is exactly where the two
/// disagreed: the drawing said *"read to the end to confirm"* while the program said *seen*,
/// because the confirmation had kept the short Loading card's surface.
#[test]
fn the_two_latches_agree_through_a_loading_to_confirmation_swap() {
    use ratatui::crossterm::event::KeyCode;

    let (mut app, _rx) = from_state("/repo", loaded(author_orientation_view()));
    let mut viewports = crate::viewports::Viewports::default();

    // The message step dispatches the preview, so a Loading card is on top. Draw it — it is one
    // line, so this really does put its end on screen and really does latch it.
    app.begin_commit();
    app.input_char('x');
    app.select();
    assert!(matches!(app.top_overlay(), Some(Overlay::Loading { .. })));
    let _ = shell_text_at(&mut app, &mut viewports, 60, 15);

    // The answer replaces it with a card too tall for this window.
    app.push_overlay_for_test(Overlay::Confirmation {
        summary: Box::new(long_confirmation()),
        tier: Tier::Two,
        typed: String::new(),
        error: None,
    });

    let mut scrolls = 0;
    loop {
        let _ = shell_text_at(&mut app, &mut viewports, 60, 15);
        let surface = app.reading_surface().expect("a card is on top");
        assert_eq!(
            app.seen_whole_card(),
            viewports.of(surface).seen_whole_card(),
            "after {scrolls} scrolls the program and the drawing disagree"
        );
        if app.at_end() {
            break;
        }
        viewports.apply(crate::keys::handle_key(&mut app, key(KeyCode::Down)));
        scrolls += 1;
        assert!(scrolls < 200, "the card never reached its end");
    }
    assert!(scrolls > 0, "the card must not fit, or this proves nothing");
    assert!(app.seen_whole_card(), "and reaching the end reads it");
}

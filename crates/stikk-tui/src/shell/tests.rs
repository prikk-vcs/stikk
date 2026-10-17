//! Full-shell render tests (design TS-01/TS-02), driven by the scripted backend.
//!
//! `App` sends requests to a worker thread rather than calling `Prikk` directly (RFC 010), so these
//! tests drive it through the same [`crate::worker::run`] function the real worker thread calls —
//! synchronously, on the test thread, one batch of queued requests at a time via [`drain`] — rather than
//! spinning up a real thread. That is exactly the split the design calls for: the async plumbing itself
//! is proven in `app::tests` and `worker::tests`; these tests only care that a loaded (or still-loading)
//! view renders correctly.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_prikk::{NullBackend, Orientation};
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;
use crate::worker::Request;

fn draw(app: &App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f)).unwrap();
    buffer_text(terminal.backend().buffer())
}

/// An Orientation whose current branch is `heads/main`, so the first read focuses it (RFC 029 Handoff B
/// §2). `NullBackend::supported()` reports no current branch and no published `heads/main`, which now
/// resolves to no focus — and a test about rendering History needs one.
fn orientation_naming_heads_main() -> Orientation {
    Orientation {
        interrupted_materialization: None,
        queued_patches: 0,
        queued_target: None,
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: stikk_model::CurrentBranch::Branch(
            stikk_model::RefName::parse("heads/main").expect("a valid ref name"),
        ),
    }
}

fn open(repo: &str, config: &Config) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::open(repo, config, tx), rx)
}

/// Answer every request currently queued against `backend`, applying each response in turn.
fn drain(app: &mut App, rx: &mpsc::Receiver<Request>, backend: &NullBackend) {
    while let Ok(req) = rx.try_recv() {
        let (tmp_tx, tmp_rx) = mpsc::channel();
        tmp_tx.send(req).unwrap();
        drop(tmp_tx);
        let (res_tx, res_rx) = mpsc::channel();
        crate::worker::run(backend, app.repo(), tmp_rx, res_tx);
        if let Ok(response) = res_rx.try_recv() {
            app.apply(response);
        }
    }
}

#[test]
fn renders_header_orientation_and_status_together() {
    let backend = NullBackend::supported().with_orientation(Orientation {
        interrupted_materialization: None,
        queued_patches: 1,
        queued_target: Some("heads/main".into()),
        main_ref_state: Some("237d0681".into()),
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: stikk_model::CurrentBranch::NotReported,
    });
    let (mut app, rx) = open("/home/dev/sample-repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("stikk")); // header
    assert!(text.contains("sample-repo")); // header repo name
    assert!(text.contains("Orientation")); // body
    assert!(text.contains("prikk 0.30.0")); // orientation content (RFC 009: the new NullBackend default)
    assert!(text.contains("q:back")); // status bar
}

#[test]
fn a_refusal_renders_the_failure_body_verbatim() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("Cannot open repository"));
    assert!(text.contains("retired format 3"));
}

#[test]
fn overlay_draws_over_the_view() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_glossary();
    let text = draw(&app, 90, 24);
    assert!(text.contains("Glossary"));
    // The status bar still shows beneath the centred overlay.
    assert!(text.contains("q:back"));
}

#[test]
fn a_banner_renders_above_the_status_bar() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let err = stikk_model::StikkError::LockConflict {
        message: "lock held by another writer".into(),
    };
    app.surface_error(&err, stikk_core::OperationContext::Orient);
    let text = draw(&app, 90, 24);
    assert!(text.contains("another writer"));
    assert!(text.contains("Esc dismisses"));
}

#[test]
fn a_fault_renders_the_untouched_repository_screen() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let err = stikk_model::StikkError::Internal {
        detail: "invariant X violated".into(),
    };
    app.surface_error(&err, stikk_core::OperationContext::Other);
    let text = draw(&app, 90, 24);
    assert!(text.contains("was not touched"));
    assert!(text.contains("read-only"));
}

#[test]
fn a_tiny_terminal_shows_the_too_small_notice() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 40, 10);
    assert!(text.contains("terminal too small"));
}

#[test]
fn a_pending_history_load_renders_as_loading() {
    // RFC 010 §5: a load that used to be instantaneous (and so unobservable) is now a real gap the
    // shell must render something for — proof that the `Screen::Loading` path actually reaches pixels.
    let backend = NullBackend::supported().with_orientation(orientation_naming_heads_main());
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    // Deliberately not drained: the response has not arrived yet.
    let text = draw(&app, 90, 24);
    assert!(text.contains("loading history"));
}

#[test]
fn a_pending_ref_picker_renders_as_loading() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_ref_picker();
    let text = draw(&app, 90, 24);
    assert!(text.contains("loading refs"));
}

#[test]
fn history_screen_renders_the_lineage_and_queue_tier() {
    use stikk_prikk::{BlockRow, History};
    let backend = NullBackend::supported()
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 3,
            queued_target: Some("heads/main".into()),
            main_ref_state: Some("bbbb".into()),
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
        })
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![
                BlockRow {
                    block_id: "bbbbbbbbbbbbbbbb".into(),
                    ref_state_id: "rs-b".into(),
                    update_seq: 2,
                    kind: "Normal".into(),
                    rollback_block: false,
                    parents: 1,
                    patches: 4,
                    rollback_patches: 0,
                    required_attestations: 0,
                    messages: Vec::new(),
                    previous_ref_state: Some("rs-a".into()),
                },
                BlockRow {
                    block_id: "aaaaaaaaaaaaaaaa".into(),
                    ref_state_id: "rs-a".into(),
                    update_seq: 1,
                    kind: "Root".into(),
                    rollback_block: false,
                    parents: 0,
                    patches: 1,
                    rollback_patches: 0,
                    required_attestations: 0,
                    messages: Vec::new(),
                    previous_ref_state: None,
                },
            ],
        });
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("History")); // view title
    assert!(text.contains("heads/main")); // ref in title
    assert!(text.contains("not yet sealed")); // queue tier
    assert!(text.contains("Normal")); // tip block kind
    assert!(text.contains("Root")); // root block kind
    assert!(text.contains("tip")); // tip marker
}

#[test]
fn block_detail_screen_shows_tip_state_and_where_patch_content_comes_from() {
    use stikk_prikk::{BlockRow, History, StateFile, StateFiles};
    let backend = NullBackend::supported()
        .with_orientation(orientation_naming_heads_main())
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![BlockRow {
                block_id: "bbbbbbbbbbbbbbbb".into(),
                ref_state_id: "rs-b".into(),
                update_seq: 2,
                kind: "Normal".into(),
                rollback_block: false,
                parents: 1,
                patches: 4,
                rollback_patches: 0,
                required_attestations: 0,
                messages: Vec::new(),
                previous_ref_state: Some("rs-a".into()),
            }],
        })
        .with_state(StateFiles {
            target_block: "bbbbbbbbbbbbbbbb".into(),
            files: vec![
                StateFile::file("readme.txt", 0o100_644),
                StateFile::file("src/main.rs", 0o100_755),
            ],
            total_bytes: 128,
        });
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_history();
    drain(&mut app, &rx, &backend);
    app.select(); // drill into the tip
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("Block")); // view title
    assert!(text.contains("readme.txt")); // tip state file
    assert!(text.contains("src/main.rs"));
    // The NullBackend reports prikk 0.30, below stikk's JSON floor, so the Patches section says so
    // in stikk's own voice (RFC 035 §4.1) where the UD-09 note used to be.
    assert!(
        text.contains("stikk reads a patch's content from prikk 0.39 on"),
        "{text}"
    );
}

/// RFC 023 F1 at **80×24** — the gloss and prikk's verbatim words are both on screen at the smallest
/// terminal stikk supports.
///
/// **This is the RFC 016 C1 lesson applied before it can bite again**: a card with prose on it is not
/// delivered by constructing the prose. C1 was a gloss that existed and did not fit; asserting only
/// that `present()` returned it would have passed then too. So this asserts pixels, at 80×24, for both
/// halves — stikk's sentence *and* prikk's, which `ER-02` requires to survive beside it.
#[test]
fn the_backslash_refusal_shows_gloss_and_prikks_words_at_80x24() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    // Surfaced through the **commit** path, which is the only path that produces this message: prikk
    // refuses it while authoring worktree nodes. (An orientation-time refusal takes the "Cannot open
    // repository" screen instead, which is a different presentation entirely and never reaches a card.)
    let err = stikk_model::StikkError::Refusal {
        message: "error: invalid name: backslashes are not allowed in repository paths".into(),
    };
    app.surface_error(&err, stikk_core::OperationContext::Commit);
    let text = draw(&app, 80, 24);

    // prikk's own words, verbatim (`ER-02`) — the clause, not a paraphrase of it.
    assert!(
        text.contains("backslashes are not allowed in repository paths"),
        "prikk's verbatim message must be on screen at 80x24:\n{text}"
    );
    // And stikk's sentence beside it. The two platforms say different true things; each is checked
    // against what it actually claims rather than a substring both happen to share.
    if cfg!(windows) {
        assert!(
            text.contains("typed no backslash"),
            "the gloss's point must be on screen, not merely constructed:\n{text}"
        );
    } else {
        assert!(
            text.contains("backslash in its name"),
            "the gloss's point must be on screen, not merely constructed:\n{text}"
        );
    }
}

/// RFC 029 Handoff B review v1 §2.3: picking the empty picker's `heads/main (not published)` opens History
/// like any other pick, and prikk's empty history renders as one. Captured at 80×24.
#[test]
fn picking_the_unpublished_heads_main_opens_its_empty_history_at_80x24() {
    let backend = NullBackend::supported()
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 0,
            queued_target: None,
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
        })
        .with_refs(Vec::new())
        .with_tags(Vec::new())
        .with_history(stikk_prikk::History {
            reff: "heads/main".into(),
            blocks: Vec::new(),
        });
    let (mut app, rx) = open("/home/dev/new-repo", &Config::default());
    // The first read resolves to no focus and opens the picker; its `Refs` read answers empty.
    drain(&mut app, &rx, &backend);
    assert!(
        matches!(
            app.top_overlay(),
            Some(crate::overlay::Overlay::RefPicker {
                unpublished_main: true,
                ..
            })
        ),
        "{:?}",
        app.top_overlay()
    );
    app.select();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 80, 24);
    println!("--- History after picking heads/main (not published), 80×24\n{text}");
    assert!(!app.has_overlay(), "{text}");
    assert!(text.contains("no sealed blocks on this ref yet"), "{text}");
    assert!(text.contains("heads/main"), "{text}");
}

/// **RFC 044 decision 7's capture, at exactly 80 columns**: `x` on the branch the repository is
/// on, refused in stikk's own voice and **without prikk running** — driven end to end through the
/// real preview, so the sentence on screen is the one `stikk-core` decided. Reproduce with
/// `cargo test -p stikk-tui --lib capture_ -- --nocapture`.
#[test]
fn capture_rfc044_closing_the_current_branch_at_80_columns() {
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
        .with_readiness(stikk_prikk::ReadinessReport {
            readiness: stikk_model::Readiness {
                author: stikk_model::RoleReadiness::Unknown,
                maintainer: stikk_model::RoleReadiness::Unknown,
                read_only: false,
            },
            author: stikk_prikk::RoleDetail::default(),
            maintainer: stikk_prikk::RoleDetail::default(),
        })
        .with_refs(vec![stikk_prikk::RefEntry {
            name: "heads/main".to_string(),
            id: "e".repeat(64),
            closed: false,
            received: false,
        }])
        // **The guard that matters**: if the preview ever let this through, the write would
        // succeed here and the assertion below would fail on a screen showing prikk's lines.
        .with_ref_write(vec!["closed branch heads/main".to_string()]);
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_ref_picker();
    drain(&mut app, &rx, &backend);
    app.begin_close_branch();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 80, 24);
    println!("--- closing the current branch, refused (80 columns) ---\n{text}");
    assert!(text.contains("the branch you are on"), "{text}");
    // RFC 045 decision 9: the way out is stikk's own key now, not prikk's command.
    assert!(text.contains("`s` in the ref picker"), "{text}");
    assert!(!text.contains("prikk branch switch"), "{text}");
    assert!(
        !text.contains("closed branch heads/main"),
        "nothing ran: {text}"
    );
}

/// **A banner wraps rather than clipping** (RFC 024). A short one keeps exactly the shape it
/// always had; a long one keeps every word.
#[test]
fn a_long_banner_wraps_and_a_short_one_does_not_move() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);

    app.surface_error(
        &stikk_model::StikkError::LockConflict {
            message: "lock held".into(),
        },
        stikk_core::OperationContext::Orient,
    );
    let short = draw(&app, 80, 24);
    assert!(
        short
            .lines()
            .any(|l| l.contains("lock held") && l.contains("Esc dismisses")),
        "a banner that fits keeps the mark, the text and the hint on one row:\n{short}"
    );

    app.surface_error(
        &stikk_model::StikkError::LockConflict {
            message: stikk_core::CLOSING_THE_CURRENT_BRANCH.into(),
        },
        stikk_core::OperationContext::Orient,
    );
    let long = draw(&app, 80, 24);
    assert!(long.contains("`s` in the ref picker"), "{long}");
    assert!(
        long.lines().any(|l| l.trim() == "(Esc dismisses)"),
        "the hint takes its own row once the text has taken more than one:\n{long}"
    );
    assert!(
        long.lines().filter(|l| l.contains("closed branch")).count() == 1,
        "wrapped once, not repeated:\n{long}"
    );
}

/// **RFC 045 F5's capture, at exactly 80 columns**: `s` on a branch the queue holds work for,
/// refused in stikk's own voice and **without prikk running** — driven end to end through the
/// real preview, so the sentence on screen is the one `stikk-core` decided. Reproduce with
/// `cargo test -p stikk-tui --lib capture_ -- --nocapture`.
#[test]
fn capture_rfc045_work_queued_for_the_target_at_80_columns() {
    let current = stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse("heads/main").expect("a ref name"),
    );
    let backend = NullBackend::supported()
        .with_version(0, 52, 0)
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 1,
            queued_target: Some("heads/feature".to_string()),
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: current.clone(),
        })
        .with_change_token(stikk_model::ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            1,
            Some("heads/feature"),
            &current,
        ))
        .with_refs(vec![
            stikk_prikk::RefEntry {
                name: "heads/main".to_string(),
                id: "e".repeat(64),
                closed: false,
                received: false,
            },
            stikk_prikk::RefEntry {
                name: "heads/feature".to_string(),
                id: "f".repeat(64),
                closed: false,
                received: false,
            },
        ])
        // **The guard that matters**: if the preview ever let this through, the switch would
        // succeed here and the assertion below would fail on a screen showing prikk's lines.
        .with_switch(
            vec!["switched from heads/main to heads/feature".to_string()],
            Vec::new(),
        );
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    app.open_ref_picker();
    drain(&mut app, &rx, &backend);
    app.nav_down(); // heads/feature
    app.begin_switch_branch();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 80, 24);
    println!("--- work queued for the target, refused (80 columns) ---\n{text}");
    assert!(text.contains("work is queued for this branch"), "{text}");
    // The tail wraps onto the next row, so the fragment checked here must not cross the wrap.
    assert!(text.contains("the worktree would not show"), "{text}");
    assert!(
        !text.contains("switched from heads/main"),
        "nothing ran: {text}"
    );
}

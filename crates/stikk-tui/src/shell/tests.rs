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
    // The colours this `Config` resolves to with no terminal to ask (RFC 059): an explicit theme
    // gives its own palette, and `system` -- the default here -- gives `plain`, which is what a
    // session that cannot ask really does get.
    let resolved = crate::theme::resolve(&crate::theme::Environment::configured(config.theme));
    (App::open(repo, resolved, None, tx), rx)
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
    // RFC 060 decision 6: it says what to do, and names the size it needs.
    assert!(text.contains("too small for stikk"), "{text}");
    assert!(text.contains("60 columns and 15 rows"), "{text}");
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
    // **RFC 063 decision 3's words**: what a switch would *do*, not what the report would show.
    assert!(text.contains("seal it first"), "{text}");
    assert!(text.contains("out of"), "{text}");
    assert!(
        !text.contains("switched from heads/main"),
        "nothing ran: {text}"
    );
}

/// **Capture: the two refusals RFC 050 Handoff B adds, as the reader meets them** — banners, at
/// exactly 80 columns, over the screen they were on.
///
/// Both are stikk's own sentences and both wrap, so what is captured here is the whole of each:
/// a refusal that clipped would be the one thing `RFC 024` forbids on a message that exists to
/// explain itself.
#[test]
fn capture_the_two_bundle_refusals_at_80x24() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);

    for (name, sentence) in [
        (
            "older bundle",
            stikk_core::WOULD_MOVE_THE_RECEIVED_BRANCH_BACK,
        ),
        ("inside the repository", stikk_core::INSIDE_THE_REPOSITORY),
    ] {
        app.surface_error(
            &stikk_model::StikkError::LockConflict {
                message: sentence.into(),
            },
            stikk_core::OperationContext::Bundle,
        );
        let text = draw(&app, 80, 24);
        println!("== {name}\n{text}");
        let flowed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let wanted: String = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flowed.contains(&wanted),
            "{name}: every word of the refusal is on screen:\n{text}"
        );
    }
}

/// **RFC 063 decision 4's capture: `C` on a branch that is not the one your folder is on**, driven
/// end to end through the real preview — so the sentence on screen is the one `stikk-core` decided,
/// and the commit never reaches prikk. At both sizes and both themes the evidence plan names.
///
/// Reproduce with `cargo test -p stikk-tui --lib capture_ -- --nocapture`.
#[test]
fn capture_rfc063_a_commit_off_the_current_branch_at_both_sizes_and_themes() {
    let current = stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse("heads/dev").expect("a ref name"),
    );
    let backend = NullBackend::supported()
        .with_version(0, 58, 0)
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 0,
            queued_target: None,
            main_ref_state: Some("e".repeat(64)),
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
        .with_refs(vec![
            stikk_prikk::RefEntry {
                name: "heads/main".to_string(),
                id: "e".repeat(64),
                closed: false,
                received: false,
            },
            stikk_prikk::RefEntry {
                name: "heads/dev".to_string(),
                id: "f".repeat(64),
                closed: false,
                received: false,
            },
        ])
        // A commit needs AUTHOR readiness before the preview is ever asked for, and at 0.58 prikk
        // answers `binding` — so this is `Known`, not the `Unknown` of the ≤ 0.39 band.
        .with_readiness(stikk_prikk::ReadinessReport {
            readiness: stikk_model::Readiness {
                author: stikk_model::RoleReadiness::Known(stikk_model::Binding::Matches),
                maintainer: stikk_model::RoleReadiness::Known(stikk_model::Binding::Matches),
                read_only: false,
            },
            author: stikk_prikk::RoleDetail::default(),
            maintainer: stikk_prikk::RoleDetail::default(),
        });
    for (theme, name) in [
        (stikk_state::config::Theme::Dark, "dark"),
        (stikk_state::config::Theme::Light, "light"),
    ] {
        let config = Config {
            theme,
            ..Config::default()
        };
        for (w, h) in [(80, 24), (120, 40)] {
            let (mut app, rx) = open("/repo", &config);
            drain(&mut app, &rx, &backend);
            // Focus `heads/main` through the picker, which is the one keypress F4 describes: the
            // cursor opens on the branch the repository is on (`heads/dev`), so `↑` is what moves
            // the reader to the other branch — and this capture is of what happens next.
            app.open_ref_picker();
            drain(&mut app, &rx, &backend);
            app.nav_up();
            app.select();
            drain(&mut app, &rx, &backend);
            assert_eq!(
                app.focused_ref(),
                Some("heads/main"),
                "the picker focused it"
            );
            app.begin_commit();
            drain(&mut app, &rx, &backend);
            // The message prompt comes first, and needs a message: `UD-01` refuses an empty one,
            // so an empty `Enter` would dispatch nothing and capture an unrefused screen.
            for c in "a note".chars() {
                app.input_char(c);
            }
            app.select();
            drain(&mut app, &rx, &backend);
            let text = draw(&app, w, h);
            println!("--- a commit off the current branch, {w}×{h}, {name}\n{text}");
            let flat = text
                .lines()
                .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
                .collect::<Vec<_>>()
                .join(" ");
            for fragment in [
                "your folder is on heads/dev",
                "To commit to heads/main, switch to it first",
                "b, then the branch, then s",
            ] {
                assert!(
                    flat.contains(fragment),
                    "{w}×{h} {name}: {fragment:?} is not whole on screen:\n{text}"
                );
            }
            assert!(
                !text.contains("Commit worktree changes"),
                "{w}×{h} {name}: no card was armed:\n{text}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// `high-contrast` and `plain` (RFC 059 decisions 7 and 8; Handoff A §3)
// ---------------------------------------------------------------------------------------------

/// The whole rendered buffer, not its text: these tests are about colour, cell by cell.
fn drawn(app: &App, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f)).unwrap();
    terminal.backend().buffer().clone()
}

/// An app on a loaded Orientation, drawing with `resolved` — and, when `overlay` is set, with the
/// Glossary open over it, which is the case that matters: an overlay clears its region before it
/// draws, so a painted frame has to survive the clear.
fn app_drawing_with(
    resolved: crate::theme::Resolved,
    overlay: bool,
) -> (App, mpsc::Receiver<Request>) {
    let backend = NullBackend::supported().with_orientation(orientation_naming_heads_main());
    let (tx, rx) = mpsc::channel();
    let mut app = App::open("/repo", resolved, None, tx);
    drain(&mut app, &rx, &backend);
    if overlay {
        app.open_glossary();
    }
    (app, rx)
}

/// What the resolver answers for one configured theme and one terminal answer, at 256 colours.
fn resolved_for(
    theme: stikk_state::config::Theme,
    answer: Option<crate::theme::Answer>,
) -> crate::theme::Resolved {
    resolved_at(theme, answer, false)
}

/// As [`resolved_for`], at the depth `truecolour` asks for.
fn resolved_at(
    theme: stikk_state::config::Theme,
    answer: Option<crate::theme::Answer>,
    truecolour: bool,
) -> crate::theme::Resolved {
    crate::theme::resolve(&crate::theme::Environment {
        theme,
        answer,
        colorterm: truecolour.then(|| "truecolor".to_string()),
        ..crate::theme::Environment::default()
    })
}

/// Decision 8: **stikk paints its own pair over the whole frame**, in both directions, and an overlay
/// cannot punch a hole in it.
#[test]
fn high_contrast_paints_every_cell_in_both_directions() {
    use ratatui::style::Color;

    // The pair at each depth: the exact cube entries at 256 colours (16 and 231 **are** black and
    // white, so nothing is rounded), and the plain 24-bit values where the terminal promises them.
    for (truecolour, white, black) in [
        (false, Color::Indexed(231), Color::Indexed(16)),
        (true, Color::Rgb(0xff, 0xff, 0xff), Color::Rgb(0, 0, 0)),
    ] {
        for (light, fg, bg) in [(true, black, white), (false, white, black)] {
            let answer = Some(crate::theme::Answer {
                light,
                background: if light {
                    (0xff, 0xff, 0xff)
                } else {
                    (0x1e, 0x1e, 0x1e)
                },
            });
            let resolved =
                resolved_at(stikk_state::config::Theme::HighContrast, answer, truecolour);
            for overlay in [false, true] {
                let (app, _rx) = app_drawing_with(resolved, overlay);
                for (w, h) in [(80, 24), (120, 40)] {
                    let buffer = drawn(&app, w, h);
                    for y in 0..h {
                        for x in 0..w {
                            let cell = buffer.cell((x, y)).expect("a cell inside the buffer");
                            assert_eq!(
                                (cell.fg, cell.bg),
                                (fg, bg),
                                "high-contrast (light background: {light}, overlay: {overlay}) at \
                             {w}x{h}, cell {x},{y} = {:?}",
                                cell.symbol()
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Decision 7: **`plain` paints nothing and colours nothing** — every cell is the terminal's own
/// pair, at every size, with and without an overlay.
#[test]
fn plain_leaves_every_cell_to_the_terminal() {
    use ratatui::style::Color;

    let resolved = resolved_for(stikk_state::config::Theme::Plain, None);
    assert_eq!(resolved.palette.bg, None, "plain paints no background");
    for overlay in [false, true] {
        let (app, _rx) = app_drawing_with(resolved, overlay);
        for (w, h) in [(80, 24), (120, 40)] {
            let buffer = drawn(&app, w, h);
            for y in 0..h {
                for x in 0..w {
                    let cell = buffer.cell((x, y)).expect("a cell inside the buffer");
                    assert_eq!(
                        (cell.fg, cell.bg),
                        (Color::Reset, Color::Reset),
                        "plain (overlay: {overlay}) at {w}x{h}, cell {x},{y} = {:?}",
                        cell.symbol()
                    );
                }
            }
        }
    }
}

/// **Capture: one view in `high-contrast` and in `plain`, at 80 x 24** (RFC 059 Handoff A §0). The
/// two new themes change no character of a screen — that is the point of them — and these captures
/// are the evidence that they change none: the view reads and fits exactly as it does in `dark`.
///
/// ```sh
/// cargo test -p stikk-tui --lib capture_rfc059_the_two_new_themes -- --nocapture
/// ```
#[test]
fn capture_rfc059_the_two_new_themes_at_80x24() {
    use stikk_state::config::Theme;

    // Every row but the `colours` row, which is the one row that is *about* the theme and so the one
    // row allowed to differ (RFC 059 decision 10).
    let apart_from_the_colours_row = |text: &str| {
        text.lines()
            .filter(|line| !line.contains("colours     "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let in_dark = {
        let (app, _rx) = app_drawing_with(resolved_for(Theme::Dark, None), false);
        apart_from_the_colours_row(&draw(&app, 80, 24))
    };
    for (theme, answer) in [
        (
            Theme::HighContrast,
            Some(crate::theme::Answer {
                light: true,
                background: (0xff, 0xff, 0xff),
            }),
        ),
        (Theme::HighContrast, None),
        (Theme::Plain, None),
    ] {
        let (app, _rx) = app_drawing_with(resolved_for(theme, answer), false);
        let text = draw(&app, 80, 24);
        println!(
            "--- RFC 059: Orientation in {theme:?} (terminal answered light: {}), 80x24\n{text}",
            answer.is_some_and(|a| a.light)
        );
        for fragment in ["Orientation", "capability", "heads/main"] {
            assert!(
                text.contains(fragment),
                "{theme:?}: {fragment:?} is whole on screen:\n{text}"
            );
        }
        assert_eq!(
            apart_from_the_colours_row(&text),
            in_dark,
            "{theme:?} draws the same characters as dark — only the colours differ"
        );
    }
}

/// **The four other themes paint no background at all** — only `high-contrast` does (decision 2).
#[test]
fn only_high_contrast_paints_a_background() {
    use stikk_state::config::Theme;

    for theme in [Theme::System, Theme::Light, Theme::Dark, Theme::Plain] {
        assert_eq!(
            resolved_for(theme, None).palette.bg,
            None,
            "{theme:?} paints no background"
        );
    }
    assert!(
        resolved_for(Theme::HighContrast, None).palette.bg.is_some(),
        "high-contrast paints one"
    );
}

// ---------------------------------------------------------------------------------------------
// The headless contrast matrix (RFC 059 Handoff A §5)
//
// **Every screen the shell can show, in every theme, at both sizes, and at both depths where the
// theme has two.** The palette's own test holds each role's value against its theme's reference
// backgrounds; this one holds every *renderer* to drawing with those roles and nothing else. A
// renderer that reached for a colour of its own — a named ANSI colour, one of the terminal's low
// sixteen — fails here with the view, the cell and the theme named.
// ---------------------------------------------------------------------------------------------

/// prikk 0.58 with something in every surface the shell can open: a lineage, a queue with two
/// patches, a dirty worktree, a verification, trusted keys, a merge plan and a diff.
///
/// Stocked rather than left at `NullBackend::supported()`'s 0.30 defaults so the views draw their
/// rows, not only their below-the-band sentences.
fn stocked() -> NullBackend {
    NullBackend::supported()
        .with_version(0, 58, 0)
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 2,
            queued_target: Some("heads/main".to_string()),
            main_ref_state: Some("237d0681".to_string()),
            trailing_partial_wal_bytes: 24,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::Branch(
                stikk_model::RefName::parse("heads/main").expect("a valid ref name"),
            ),
        })
        .with_readiness(stikk_prikk::ReadinessReport {
            readiness: stikk_model::Readiness {
                author: stikk_model::RoleReadiness::Known(stikk_model::Binding::Matches),
                maintainer: stikk_model::RoleReadiness::Known(stikk_model::Binding::NotAdopted),
                read_only: false,
            },
            author: stikk_prikk::RoleDetail::default(),
            maintainer: stikk_prikk::RoleDetail::default(),
        })
        .with_history(stikk_prikk::History {
            reff: "heads/main".to_string(),
            blocks: vec![matrix_block("a", 2), matrix_block("b", 1)],
        })
        .with_refs(vec![
            stikk_prikk::RefEntry {
                name: "heads/main".to_string(),
                id: "e".repeat(64),
                closed: false,
                received: false,
            },
            stikk_prikk::RefEntry {
                name: "heads/dev".to_string(),
                id: "f".repeat(64),
                closed: false,
                received: false,
            },
        ])
        .with_merge_plan(
            vec![
                "would merge heads/dev into heads/main".to_string(),
                "conflict: both sides changed a.txt".to_string(),
            ],
            stikk_prikk::MergeOutcome::Conflict,
        )
        .with_queue(stikk_prikk::QueueReport::Listed(stikk_prikk::Queue {
            count: 2,
            target: stikk_prikk::QueueTarget::Ref("heads/main".to_string()),
            threshold: Some(stikk_prikk::QueueThreshold {
                status: stikk_prikk::ThresholdStatus::None,
                warn: 800,
                hard_limit: 1000,
            }),
            patches: vec![
                stikk_prikk::QueuedPatch {
                    patch_id: "c".repeat(64),
                    message: stikk_prikk::QueuedMessage::Text("add b".to_string()),
                    operations: vec![stikk_prikk::QueuedOperation {
                        kind: "create-file".to_string(),
                        paths: vec![stikk_prikk::QueuedPath::Path("b.txt".to_string())],
                        author_key_id: None,
                    }],
                },
                stikk_prikk::QueuedPatch {
                    patch_id: "d".repeat(64),
                    message: stikk_prikk::QueuedMessage::Text("rename a to c".to_string()),
                    operations: vec![stikk_prikk::QueuedOperation {
                        kind: "rename-path".to_string(),
                        paths: vec![
                            stikk_prikk::QueuedPath::Path("a.txt".to_string()),
                            stikk_prikk::QueuedPath::Path("c.txt".to_string()),
                        ],
                        author_key_id: Some("ed25519-0123456789abcdef".to_string()),
                    }],
                },
            ],
            already_sealed_in: None,
        }))
        .with_verification(stikk_prikk::Verification {
            verdict: stikk_prikk::Verdict {
                ok: false,
                failed_conditions: vec![stikk_prikk::FailedCondition {
                    id: "wal-replay".to_string(),
                    message: "a torn tail at offset 4096".to_string(),
                }],
            },
            active_sessions: Some(stikk_prikk::ActiveSessions {
                count: 1,
                verified_count: 1,
            }),
            stages: ["objects", "refs", "wal-replay"]
                .into_iter()
                .map(|stage| stikk_prikk::VerifyStage {
                    stage: stage.to_string(),
                    status: stikk_prikk::StageStatus::Evaluated,
                })
                .collect(),
            raw: String::new(),
        })
        .with_diff(matrix_diff())
        .with_bundle_verify(vec!["bundle verified: 1 block, 1 ref".to_string()])
        .with_bundle_preview(stikk_prikk::BundlePreviewRead {
            bundle_ref: "heads/main".to_string(),
            local_ref: "heads/main".to_string(),
            connectivity: stikk_prikk::Connectivity::FastForward,
            conflict: Some(stikk_prikk::BundleConflict::AppliesCleanly),
            conflict_detail: None,
            sealed_by: vec!["z-maint".to_string()],
            sealed_by_note: "recorded signer identity, continuity only".to_string(),
            effects: Vec::new(),
            rename_note: "a moved path is not yet distinguishable".to_string(),
            seal_outcomes: None,
            import_refusals: None,
        })
        .with_worktree_status(stikk_prikk::WorktreeStatus {
            refused_declarations: None,
            reff: "heads/main".to_string(),
            clean: false,
            tracked: 2,
            unchanged: 0,
            missing: 0,
            modified: 1,
            untracked: 1,
            unsupported: 0,
            refused: None,
            entries: vec![
                stikk_prikk::WorktreeEntry {
                    kind: "modified".to_string(),
                    path: "a.txt".to_string(),
                    note: "content differs from the baseline".to_string(),
                    authoring: stikk_prikk::Authoring::Authored,
                },
                stikk_prikk::WorktreeEntry {
                    kind: "untracked".to_string(),
                    path: "notes/new.txt".to_string(),
                    note: "not in the baseline".to_string(),
                    authoring: stikk_prikk::Authoring::Refused(
                        "path is excluded by .prikkignore".to_string(),
                    ),
                },
            ],
            queued_elsewhere: None,
            declarations: Vec::new(),
        })
        .with_trusted_keys(vec![
            stikk_prikk::TrustedKey {
                key_id: "ed25519-0123456789abcdef".to_string(),
                public_key: "A".repeat(64),
            },
            stikk_prikk::TrustedKey {
                key_id: "ed25519-fedcba9876543210".to_string(),
                public_key: "B".repeat(64),
            },
        ])
}

/// As [`stocked`], with nothing queued: a merge plan is refused while work is waiting for the branch
/// (RFC 047 decision 5), and the matrix wants the plan itself on screen.
fn stocked_without_a_queue() -> NullBackend {
    stocked().with_orientation(Orientation {
        interrupted_materialization: None,
        queued_patches: 0,
        queued_target: None,
        main_ref_state: Some("237d0681".to_string()),
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: stikk_model::CurrentBranch::Branch(
            stikk_model::RefName::parse("heads/main").expect("a valid ref name"),
        ),
    })
}

/// As [`stocked`], with a dirty worktree prikk would commit: the confirmation card's own path, where
/// [`stocked`]'s refused entry reaches the *would-refuse* card instead.
fn stocked_committable() -> NullBackend {
    stocked().with_worktree_status(stikk_prikk::WorktreeStatus {
        refused_declarations: None,
        reff: "heads/main".to_string(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        refused: Some(0),
        entries: vec![stikk_prikk::WorktreeEntry {
            kind: "modified".to_string(),
            path: "a.txt".to_string(),
            note: "content differs from the baseline".to_string(),
            authoring: stikk_prikk::Authoring::Authored,
        }],
        queued_elsewhere: None,
        declarations: Vec::new(),
    })
}

/// One comparison, for the Compare view: a modified file with one hunk.
fn matrix_diff() -> stikk_prikk::Diff {
    let file = |size| stikk_prikk::DiffFile {
        kind: "file".to_string(),
        encoding: "text".to_string(),
        mode: 0o100_644,
        size,
        content_id: None,
    };
    stikk_prikk::Diff {
        from: stikk_prikk::DiffSide {
            point: "b".repeat(64),
            target: stikk_prikk::DiffTarget::Block("b".repeat(64)),
            queued: stikk_prikk::QueuedOnSide::NotApplicable,
        },
        to: stikk_prikk::DiffSide {
            point: "a".repeat(64),
            target: stikk_prikk::DiffTarget::Block("a".repeat(64)),
            queued: stikk_prikk::QueuedOnSide::NotApplicable,
        },
        entries: vec![stikk_prikk::DiffEntry {
            path: "a.txt".to_string(),
            status: "modified".to_string(),
            from_path: None,
            from: Some(file(42)),
            to: Some(file(61)),
            minimal: true,
            hunks: vec!["@@ -1 +1 @@\n-one\n+two\n".to_string()],
            raw: String::new(),
        }],
        unsupported_paths: Vec::new(),
    }
}

/// One block for the matrix's lineage.
fn matrix_block(tag: &str, seq: u64) -> stikk_prikk::BlockRow {
    stikk_prikk::BlockRow {
        block_id: tag.repeat(64),
        ref_state_id: tag.repeat(64),
        update_seq: seq,
        kind: "Normal".to_string(),
        rollback_block: false,
        parents: u64::from(seq > 1),
        patches: 1,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: (seq > 1).then(|| "b".repeat(64)),
    }
}

/// One screen of the matrix: a name, the backend it runs against, the keys that reach it, and **a
/// marker that proves it was reached**.
///
/// The marker is why this test cannot pass vacuously: a `reach` that stopped short — a guard
/// refusing, a key doing nothing — would leave the Orientation view on screen, which keeps 7 : 1 and
/// would quietly cover none of what the screen's name claims.
struct Screen {
    /// What a failure prints.
    name: &'static str,
    /// The backend this screen needs.
    backend: fn() -> NullBackend,
    /// Whether to let the first read land before `reach`. `false` is how the screens *before* a read
    /// are drawn.
    first_read: bool,
    /// Driven after the first read has landed, where it was let to.
    reach: fn(&mut App, &mpsc::Receiver<Request>, &NullBackend),
    /// Text that must be on the screen once `reach` is done.
    marker: &'static str,
}

/// **Every screen the shell can show.** The views come first, then the overlays that draw over
/// them; what is deliberately not here is listed in the review request.
fn screens() -> Vec<Screen> {
    vec![
        Screen {
            name: "orientation (loaded)",
            marker: "prikk 0.58.0",
            first_read: true,
            backend: stocked,
            reach: |_, _, _| {},
        },
        Screen {
            name: "orientation (loading)",
            marker: "loading",
            first_read: false,
            backend: stocked,
            // Drawn before the first read lands at all (`first_read: false`): the view a session
            // shows for as long as prikk takes to answer.
            reach: |_, _, _| {},
        },
        Screen {
            name: "orientation (failed)",
            marker: "Cannot open repository",
            first_read: true,
            backend: || {
                NullBackend::supported().with_orientation_refusal("repository is retired format 3")
            },
            reach: |_, _, _| {},
        },
        Screen {
            name: "fault",
            marker: "was not touched",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| {
                app.surface_error(
                    &stikk_model::StikkError::Internal {
                        detail: "invariant X violated".into(),
                    },
                    stikk_core::OperationContext::Other,
                );
            },
        },
        Screen {
            name: "banner",
            marker: "another writer",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| {
                app.surface_error(
                    &stikk_model::StikkError::LockConflict {
                        message: "lock held by another writer".into(),
                    },
                    stikk_core::OperationContext::Orient,
                );
            },
        },
        Screen {
            name: "loading (history)",
            marker: "loading history",
            first_read: true,
            backend: stocked,
            // Not drained: the gap itself is a screen (RFC 010 §5).
            reach: |app, _, _| app.open_history(),
        },
        Screen {
            name: "history",
            marker: "History · heads/main",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_history();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "block detail",
            marker: "Block · tip",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_history();
                drain(app, rx, backend);
                app.select();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "changes",
            marker: "2 change(s) against baseline",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_changes();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "queue",
            marker: "2 patch(es) queued for heads/main",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_queue();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "verify",
            marker: "prikk reports 1 problem(s)",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_verify();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "trust",
            marker: "ed25519-fedcba9876543210",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_trust();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "merge plan",
            marker: "Merge plan",
            first_read: true,
            backend: stocked_without_a_queue,
            reach: |app, rx, backend| {
                // `M` merges *the marked branch into the selected one* (RFC 047 decision 4), so the
                // picker marks `heads/dev` with `m` first.
                app.open_ref_picker();
                drain(app, rx, backend);
                app.nav_down();
                app.mark_block();
                app.nav_up();
                app.begin_merge_plan();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "patch detail",
            marker: "Patch \u{2014} v: prikk's own fields",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_queue();
                drain(app, rx, backend);
                app.select();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "compare",
            marker: "this block, against the previous block",
            first_read: true,
            backend: stocked,
            // `d` with nothing marked compares the selected block with the previous one in the
            // lineage (RFC 038 decision 3).
            reach: |app, rx, backend| {
                app.open_history();
                drain(app, rx, backend);
                app.compare();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "bundle",
            marker: "This bundle carries heads/main",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.begin_open_bundle();
                for ch in "/tmp/v1.bundle".chars() {
                    app.input_char(ch);
                }
                app.select();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "overlay: glossary (the help overlay)",
            marker: "Glossary & Help",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| app.open_glossary(),
        },
        Screen {
            name: "overlay: command palette",
            marker: "Command palette",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| app.open_palette(),
        },
        Screen {
            name: "overlay: refusal history",
            marker: "Recent refusals",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| app.open_refusals(),
        },
        Screen {
            name: "overlay: background operations",
            marker: "Background operations",
            first_read: true,
            backend: stocked,
            reach: |app, _, _| app.open_operations(),
        },
        Screen {
            name: "overlay: ref picker",
            marker: "Choose ref",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_ref_picker();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "overlay: the commit message prompt",
            marker: "Commit message",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.begin_commit();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "overlay: a confirmation card",
            marker: "Confirm",
            first_read: true,
            backend: stocked_committable,
            reach: |app, rx, backend| {
                app.begin_commit();
                drain(app, rx, backend);
                for ch in "a note".chars() {
                    app.input_char(ch);
                }
                app.select();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "overlay: a commit prikk would refuse",
            marker: "Commit unavailable",
            first_read: true,
            backend: stocked,
            // The card comes after the message, where the preview re-reads the worktree and finds
            // the refused path -- the same keys as the confirmation card, against a worktree prikk
            // would refuse.
            reach: |app, rx, backend| {
                app.begin_commit();
                drain(app, rx, backend);
                for ch in "a note".chars() {
                    app.input_char(ch);
                }
                app.select();
                drain(app, rx, backend);
            },
        },
        Screen {
            name: "overlay: a refusal card",
            marker: "prikk refused",
            first_read: true,
            backend: || {
                stocked().with_history_refusal(
                    "refusing to read heads/main: the WAL tail is torn; run `prikk doctor`",
                )
            },
            reach: |app, rx, backend| {
                app.open_history();
                drain(app, rx, backend);
            },
        },
    ]
}

/// Print every screen of the matrix, to show what it covered. Ignored: it is a listing, not a check.
///
/// ```sh
/// cargo test -p stikk-tui --lib list_the_matrix -- --ignored --nocapture
/// ```
#[test]
#[ignore = "prints the matrix's screens; run it on purpose"]
fn list_the_matrix_screens() {
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open(
            "/repo",
            resolved_for(stikk_state::config::Theme::Dark, None),
            None,
            tx,
        );
        drain(&mut app, &rx, &backend);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        println!("=== {}\n{}", screen.name, draw(&app, 120, 40));
    }
}

/// **Every drawn cell of one rendered screen**, held to 7 : 1 against every background it could be
/// read on — its own, where the theme painted one, and every reference background of the theme
/// otherwise. Answers the number of coloured cells it checked and the number of checks that took.
///
/// Shared by the matrix and by the too-small notice, which is the one screen the matrix's own sizes
/// cannot produce.
fn keeps_seven_to_one(
    buffer: &ratatui::buffer::Buffer,
    what: &str,
    palette: &crate::theme::Palette,
    depth: crate::theme::Depth,
) -> (usize, usize) {
    use crate::theme::{Checkable, MINIMUM_CONTRAST, checkable, contrast_ratio};

    let area = buffer.area;
    let (mut cells, mut checks) = (0_usize, 0_usize);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = buffer.cell((x, y)).expect("a cell inside the buffer");
            let fg = match checkable(cell.fg) {
                // The terminal's own pair: nothing of stikk's to check, and nothing stikk could
                // check (RFC 059 decision 2).
                Checkable::TerminalsOwn => continue,
                Checkable::Known(rgb) => rgb,
                Checkable::TerminalsGuess => panic!(
                    "{what} in {} ({}): cell {x},{y} ({:?}) is drawn in {:?}, a colour only the \
                     terminal knows — every colour comes from the palette (RFC 059)",
                    palette.kind.name(),
                    depth.words(),
                    cell.symbol(),
                    cell.fg,
                ),
            };
            let against: Vec<(u8, u8, u8)> = match checkable(cell.bg) {
                Checkable::Known(bg) => vec![bg],
                Checkable::TerminalsOwn => palette.kind.references().to_vec(),
                Checkable::TerminalsGuess => panic!(
                    "{what} in {}: cell {x},{y} has background {:?}, a colour only the terminal \
                     knows",
                    palette.kind.name(),
                    cell.bg,
                ),
            };
            assert!(
                !against.is_empty(),
                "{what} in {}: cell {x},{y} ({:?}) is drawn in {:?} with no background to check \
                 it against",
                palette.kind.name(),
                cell.symbol(),
                cell.fg,
            );
            cells += 1;
            for bg in against {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= MINIMUM_CONTRAST,
                    "{what} in {} ({}): cell {x},{y} ({:?}) is {ratio:.2}:1 — {:?} on \
                     #{:02x}{:02x}{:02x}, below {MINIMUM_CONTRAST}:1",
                    palette.kind.name(),
                    depth.words(),
                    cell.symbol(),
                    cell.fg,
                    bg.0,
                    bg.1,
                    bg.2,
                );
                checks += 1;
            }
        }
    }
    (cells, checks)
}

/// Every theme, at both depths — the matrix's outer two dimensions, which the too-small notice
/// shares.
fn every_theme_and_depth() -> Vec<crate::theme::Resolved> {
    use stikk_state::config::Theme;

    let mut out = Vec::new();
    for theme in [Theme::Dark, Theme::Light, Theme::HighContrast, Theme::Plain] {
        // Both depths for `dark` and `light`, whose roles differ between the two; `high-contrast`
        // and `plain` are drawn at both as well, which costs nothing and shows that the pair and the
        // nothing are the same either way.
        for truecolour in [false, true] {
            out.push(resolved_at(theme, None, truecolour));
        }
    }
    out
}

/// **RFC 059's headless matrix.** Every screen above, in every theme, at both sizes, at both depths
/// where the theme has two — with every drawn cell's foreground held to 7 : 1 against every
/// background it could be read on.
#[test]
fn every_screen_keeps_seven_to_one() {
    let mut cells = 0_usize;
    let mut checks = 0_usize;
    let mut screens_drawn = 0_usize;
    for resolved in every_theme_and_depth() {
        for screen in screens() {
            let backend = (screen.backend)();
            let (tx, rx) = mpsc::channel();
            let mut app = App::open("/repo", resolved, None, tx);
            if screen.first_read {
                drain(&mut app, &rx, &backend);
            }
            (screen.reach)(&mut app, &rx, &backend);
            screens_drawn += 1;
            // **The screen was reached**, or what follows would be checking the Orientation view
            // over and over and calling it coverage.
            let shown = draw(&app, 120, 40);
            assert!(
                shown.contains(screen.marker),
                "{} was not reached in {}: {:?} is not on screen\n{shown}",
                screen.name,
                resolved.palette.kind.name(),
                screen.marker,
            );
            // **60 × 15 and 72 × 20 as well** (RFC 060 §5.3): the minimum, and a size between it
            // and yesterday's.
            for (w, h) in [(60, 15), (72, 20), (80, 24), (120, 40)] {
                let buffer = drawn(&app, w, h);
                let what = format!("{} at {w}x{h}", screen.name);
                let (c, k) = keeps_seven_to_one(&buffer, &what, &resolved.palette, resolved.depth);
                cells += c;
                checks += k;
            }
        }
    }
    println!(
        "RFC 059 matrix: {screens_drawn} screens drawn ({} per theme-depth), {cells} coloured \
         cells, {checks} contrast checks",
        screens().len()
    );
    assert!(
        cells > 10_000,
        "the matrix checked only {cells} coloured cells, which is too few to have drawn the views"
    );
}

/// **The one screen the matrix's sizes cannot show**: below the minimum the shell draws the
/// too-small notice instead of a view (design TU-10, as RFC 060 decision 1 amends it), so it is held
/// to 7 : 1 here, in every theme and at both depths.
#[test]
fn the_too_small_notice_keeps_seven_to_one() {
    let backend = stocked();
    let mut cells = 0_usize;
    for resolved in every_theme_and_depth() {
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", resolved, None, tx);
        drain(&mut app, &rx, &backend);
        // One column short, one row short, and far too small for either.
        for (w, h) in [(59, 15), (60, 14), (40, 10)] {
            let text = draw(&app, w, h);
            assert!(
                text.contains("too small for stikk") || text.contains("too small"),
                "{w}x{h} is below the minimum:\n{text}"
            );
            let buffer = drawn(&app, w, h);
            let what = format!("the too-small notice at {w}x{h}");
            let (c, _) = keeps_seven_to_one(&buffer, &what, &resolved.palette, resolved.depth);
            cells += c;
        }
    }
    println!("RFC 059: the too-small notice, {cells} coloured cells");
}

// ---------------------------------------------------------------------------------------------
// No row overflows its area (RFC 060 decision 2; Handoff A §2)
// ---------------------------------------------------------------------------------------------

/// **Every screen, at a width below today's minimum, with no row wider than the area it is drawn
/// in** (RFC 060 decision 2).
///
/// **Structural, not textual.** A cut row and a row that simply ends look the same on screen, so
/// this cannot be read off the buffer: every view and every panel records the rows it is about to
/// draw against the width it has (`text::record_overflow`), and this drains the list and asserts it
/// is empty, naming each one.
///
/// Drawn at **60 × 15 and 72 × 20** — the minimum (RFC 060 decision 1) and a size between it and
/// yesterday's 80 × 24.
#[test]
fn no_row_overflows_its_area_below_the_minimum() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut found: Vec<String> = Vec::new();
    let mut screens_drawn = 0_usize;
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", resolved, None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        // Drain what reaching the screen recorded: `reach` draws, and those draws were at 120×40.
        let _ = crate::text::drain_overflows();
        for (w, h) in [(60, 15), (72, 20)] {
            let _ = drawn(&app, w, h);
            screens_drawn += 1;
            for overflow in crate::text::drain_overflows() {
                found.push(format!(
                    "{} at {w}x{h}: {} row {} wants {} columns of {}: {:?}",
                    screen.name,
                    overflow.what,
                    overflow.row,
                    overflow.drawn,
                    overflow.width,
                    overflow.text
                ));
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} row(s) overflow their area:\n{}",
        found.len(),
        found.join("\n")
    );
    println!("RFC 060: {screens_drawn} screens drawn at 60x18 and 72x20, zero overflows");
}

/// **The three overlays RFC 060 F3 found cut, read whole at 60 × 18** — the sentences F3 quotes,
/// asserted after flattening the border and the wrap.
///
/// ```sh
/// cargo test -p stikk-tui --lib capture_rfc060_the_three_overlays -- --nocapture
/// ```
#[test]
fn capture_rfc060_the_three_overlays_f3_found_cut() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    for (name, fragments) in [
        (
            "overlay: a refusal card",
            vec![
                "prikk declined to read this ref's history.",
                "The ref may not exist, or its name may be mistyped.",
                "stikk cannot override prikk's decision.",
            ],
        ),
        (
            "overlay: a commit prikk would refuse",
            vec![
                "prikk reports that it would refuse the paths below",
                "and one refused path refuses the whole commit. Nothing was armed.",
                "then preview the commit again",
            ],
        ),
        (
            "overlay: glossary (the help overlay)",
            vec![
                "Every repository write happens inside prikk itself, never in stikk.",
                "prikk's own fields (in Patch detail, Compare)",
                "mark this block for a comparison (in History)",
            ],
        ),
    ] {
        let screen = screens()
            .into_iter()
            .find(|screen| screen.name == name)
            .expect("a screen of the matrix");
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", resolved, None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        let text = draw(&app, 60, 18);
        println!("--- RFC 060 F3: {name}, 60x18\n{text}");
        // Flattened the way every capture here is: the border off, the rows joined, so a sentence
        // that wrapped is one line again and `contains` means "whole on screen".
        let flat = text
            .lines()
            .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
            .collect::<Vec<_>>()
            .join(" ");
        for fragment in fragments {
            let wanted: String = fragment.split_whitespace().collect::<Vec<_>>().join(" ");
            let flowed: String = flat.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                flowed.contains(&wanted),
                "{name}: {wanted:?} is not whole on screen:\n{text}"
            );
        }
    }
}

/// **The screen that says what to do, at every size below the minimum** (RFC 060 decision 6): it
/// wraps, it drops its sentences from the last, and down to 20 × 5 the first one is whole.
///
/// ```sh
/// cargo test -p stikk-tui --lib capture_rfc060_the_too_small_screen -- --nocapture
/// ```
#[test]
fn capture_rfc060_the_too_small_screen() {
    let backend = stocked();
    let (mut app, rx) = open("/repo", &Config::default());
    drain(&mut app, &rx, &backend);
    for (w, h) in [(59, 15), (60, 14), (40, 10), (20, 5), (10, 3)] {
        let text = draw(&app, w, h);
        println!("--- RFC 060: the too-small screen at {w}x{h}\n{text}");
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        // **The size it needs, and the size it has**, in the first sentence — which is the one
        // sentence that is never dropped.
        assert!(
            flat.contains(&format!(
                "it needs 60 columns and 15 rows, and has {w} × {h}."
            )) || (w, h) == (10, 3),
            "{w}x{h}: the first sentence is whole:\n{text}"
        );
        // **Nothing is cut**: every row was wrapped to the window.
        for row in text.lines() {
            assert!(
                row.chars().count() <= usize::from(w),
                "{w}x{h}: a row is wider than the window: {row:?}"
            );
        }
        // **The terminal's own colours** — no colour of stikk's at all (decision 6).
        let buffer = drawn(&app, w, h);
        for y in 0..h {
            for x in 0..w {
                let cell = buffer.cell((x, y)).expect("a cell inside the buffer");
                assert_eq!(
                    (cell.fg, cell.bg),
                    (ratatui::style::Color::Reset, ratatui::style::Color::Reset),
                    "{w}x{h}: cell {x},{y} is coloured"
                );
            }
        }
    }

    // At 59 × 15 there is room for all three sentences; at 20 × 5 for the first alone.
    let roomy = draw(&app, 59, 15);
    let flat = roomy.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Make the window bigger"), "{roomy}");
    assert!(
        flat.contains("https://prikk-vcs.github.io/stikk/"),
        "the guide is named by its address:\n{roomy}"
    );
    let tiny = draw(&app, 20, 5);
    assert!(
        !tiny.contains("Help:"),
        "the last sentence is the first to go:\n{tiny}"
    );
    // And at 10 × 3, as much of the first sentence as fits — still wrapped, never cut.
    let smallest = draw(&app, 10, 3);
    assert!(smallest.contains("This"), "{smallest}");
    assert_eq!(
        smallest
            .lines()
            .filter(|row| !row.trim().is_empty())
            .count(),
        3,
        "three rows, all of them used:\n{smallest}"
    );
}

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

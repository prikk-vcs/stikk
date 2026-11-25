//! Full-shell render tests (design TS-01/TS-02), driven by the scripted backend.
//!
//! `App` sends requests to a worker thread rather than calling `Prikk` directly (RFC 010), so these
//! tests drive it through the same [`stikk_app::run`] function the real worker thread calls —
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
use stikk_app::Request;

fn draw(app: &App, w: u16, h: u16) -> String {
    draw_in(app, &look(), w, h)
}

/// **A front end, for a test that scrolls** (RFC 067 decision 4).
///
/// The offsets are the front end's now, so a test that presses a key and then looks at the screen
/// has to be one: it keeps one set of offsets, reports what each draw found, and carries out what
/// the program asks for. That order — draw, report, press, perform — is the run loop's.
#[derive(Default)]
struct Front {
    viewports: crate::viewports::Viewports,
}

impl Front {
    fn draw(&mut self, app: &mut App, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|f| render(app, &look(), &mut self.viewports, f))
            .unwrap();
        if let Some(reading) = app
            .reading_surface()
            .and_then(|s| self.viewports.reading(s))
        {
            app.report_reading(reading);
        }
        buffer_text(terminal.backend().buffer())
    }

    /// Press a key and do what the program answers with.
    fn press(&mut self, app: &mut App, key: ratatui::crossterm::event::KeyEvent) {
        self.viewports.apply(crate::keys::handle_key(app, key));
    }
}

/// The colours a test draws in when it is not about colours: what a default `Config` resolves to
/// with no terminal to ask, which is exactly what these tests' apps used to hold before the colours
/// left `App` (RFC 067 decision 3; RFC 059: `system` with no answer is `plain`).
fn look() -> crate::theme::Resolved {
    crate::theme::resolve(&crate::theme::Environment::configured(
        Config::default().theme,
    ))
}

fn draw_in(app: &App, look: &crate::theme::Resolved, w: u16, h: u16) -> String {
    // **A draw of its own keeps offsets of its own** (RFC 067 decision 4): a test that draws once
    // starts every surface at the top, which is what an app with its own offsets gave it.
    let mut viewports = crate::viewports::Viewports::default();
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(app, look, &mut viewports, f))
        .unwrap();
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

fn open(repo: &str) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    (App::open(repo, None, tx), rx)
}

/// Answer every request currently queued against `backend`, applying each response in turn.
fn drain(app: &mut App, rx: &mpsc::Receiver<Request>, backend: &NullBackend) {
    while let Ok(req) = rx.try_recv() {
        let (tmp_tx, tmp_rx) = mpsc::channel();
        tmp_tx.send(req).unwrap();
        drop(tmp_tx);
        let (res_tx, res_rx) = mpsc::channel();
        stikk_app::run(backend, app.repo(), tmp_rx, res_tx);
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
    let (mut app, rx) = open("/home/dev/sample-repo");
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("stikk")); // header
    assert!(text.contains("sample-repo")); // header repo name
    assert!(text.contains("Orientation")); // body
    assert!(text.contains("prikk 0.30.0")); // orientation content (RFC 009: the new NullBackend default)
    // **The key line, above the status bar** (RFC 061 decision 5), where the status bar's hint used
    // to be the only place a key was named.
    assert!(text.contains("q back"), "{text}");
    assert!(text.contains("Enter actions"), "{text}");
}

#[test]
fn a_refusal_renders_the_failure_body_verbatim() {
    let backend =
        NullBackend::supported().with_orientation_refusal("repository is retired format 3");
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 90, 24);
    assert!(text.contains("Cannot open repository"));
    assert!(text.contains("retired format 3"));
}

#[test]
fn overlay_draws_over_the_view() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    app.open_glossary();
    let text = draw(&app, 90, 24);
    assert!(text.contains("Glossary"));
    // The status bar still shows beneath the centred overlay.
    assert!(text.contains("no ref focused"), "{text}");
    // **A tall card covers the key line**, which is why an overlay keeps a foot line of its own
    // (RFC 060 decision 3, kept by RFC 061 decision 5): it names the key that reads on, where the
    // key line cannot be seen.
    assert!(text.contains("more below"), "{text}");
}

#[test]
fn a_banner_renders_above_the_status_bar() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    app.open_history();
    // Deliberately not drained: the response has not arrived yet.
    let text = draw(&app, 90, 24);
    assert!(text.contains("loading history"));
}

#[test]
fn a_pending_ref_picker_renders_as_loading() {
    let backend = NullBackend::supported();
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    app.open_history();
    drain(&mut app, &rx, &backend);
    app.select_screen_or_row(); // the right arrow: drill into the tip
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
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/home/dev/new-repo");
    // The first read resolves to no focus and opens the picker; its `Refs` read answers empty.
    drain(&mut app, &rx, &backend);
    assert!(
        matches!(
            app.top_overlay(),
            Some(stikk_app::Overlay::RefPicker {
                unpublished_main: true,
                ..
            })
        ),
        "{:?}",
        app.top_overlay()
    );
    // Enter, Enter: the row's action list, then its first entry (RFC 061 decision 3).
    app.select();
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
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    app.open_ref_picker();
    drain(&mut app, &rx, &backend);
    app.begin_close_branch();
    drain(&mut app, &rx, &backend);
    let text = draw(&app, 80, 24);
    println!("--- closing the current branch, refused (80 columns) ---\n{text}");
    assert!(text.contains("the branch you are on"), "{text}");
    // RFC 045 decision 9: the way out is stikk's own, not prikk's command — and named as an action
    // rather than a key (RFC 061 decision 2).
    assert!(text.contains("from its own actions"), "{text}");
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
    let (mut app, rx) = open("/repo");
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
    assert!(long.contains("from its own actions"), "{long}");
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
    let (mut app, rx) = open("/repo");
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
    let (mut app, rx) = open("/repo");
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
            let (mut app, rx) = open("/repo");
            drain(&mut app, &rx, &backend);
            // Focus `heads/main` through the picker, which is the one keypress F4 describes: the
            // cursor opens on the branch the repository is on (`heads/dev`), so `↑` is what moves
            // the reader to the other branch — and this capture is of what happens next.
            app.open_ref_picker();
            drain(&mut app, &rx, &backend);
            app.nav_up();
            // Enter, Enter: the ref row's action list, then *Open its history* (RFC 061 decision 3).
            app.select();
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
            let text = draw_in(
                &app,
                &crate::theme::resolve(&crate::theme::Environment::configured(config.theme)),
                w,
                h,
            );
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
fn drawn(app: &App, look: &crate::theme::Resolved, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut viewports = crate::viewports::Viewports::default();
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(app, look, &mut viewports, f))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// An app on a loaded Orientation, drawing with `resolved` — and, when `overlay` is set, with the
/// Glossary open over it, which is the case that matters: an overlay clears its region before it
/// draws, so a painted frame has to survive the clear.
fn app_drawing_with(overlay: bool) -> (App, mpsc::Receiver<Request>) {
    let backend = NullBackend::supported().with_orientation(orientation_naming_heads_main());
    let (tx, rx) = mpsc::channel();
    let mut app = App::open("/repo", None, tx);
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
                let (app, _rx) = app_drawing_with(overlay);
                for (w, h) in [(80, 24), (120, 40)] {
                    let buffer = drawn(&app, &resolved, w, h);
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
        let (app, _rx) = app_drawing_with(overlay);
        for (w, h) in [(80, 24), (120, 40)] {
            let buffer = drawn(&app, &resolved, w, h);
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
        let (app, _rx) = app_drawing_with(false);
        apart_from_the_colours_row(&draw_in(&app, &resolved_for(Theme::Dark, None), 80, 24))
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
        let (app, _rx) = app_drawing_with(false);
        // **Each theme's own colours**, which is what this test compares: the app holds none
        // (RFC 067 decision 3), so the draw is told which to use.
        let text = draw_in(&app, &resolved_for(theme, answer), 80, 24);
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
                app.select_screen_or_row();
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
            marker: " Patch ",
            first_read: true,
            backend: stocked,
            reach: |app, rx, backend| {
                app.open_queue();
                drain(app, rx, backend);
                app.select_screen_or_row();
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
        let mut app = App::open("/repo", None, tx);
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
) -> (usize, usize, usize) {
    use crate::theme::{Checkable, MINIMUM_CONTRAST, checkable, contrast_ratio};

    let area = buffer.area;
    let (mut cells, mut checks, mut reversed_cells) = (0_usize, 0_usize, 0_usize);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = buffer.cell((x, y)).expect("a cell inside the buffer");
            // **Reverse video swaps the pair before anything is compared** (RFC 061 decision 4): a
            // selected row's cell draws its foreground as its background, so checking it unswapped
            // would check a pair the terminal never shows.
            let reversed = cell.modifier.contains(ratatui::style::Modifier::REVERSED);
            if reversed {
                reversed_cells += 1;
            }
            let (front, back) = if reversed {
                (cell.bg, cell.fg)
            } else {
                (cell.fg, cell.bg)
            };
            // A swapped cell whose text is now the terminal's own and whose background is one of
            // stikk's is **the same two values**, so it is checked as that colour against the
            // theme's references — the ratio RFC 059 already holds (decision 4). Where both ends
            // are the terminal's own, the swap is of a pair stikk did not choose, and it is skipped
            // as before.
            let (front, back) = match (checkable(front), checkable(back)) {
                (Checkable::TerminalsOwn, Checkable::Known(_)) => (back, front),
                _ => (front, back),
            };
            let fg = match checkable(front) {
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
                    front,
                ),
            };
            let against: Vec<(u8, u8, u8)> = match checkable(back) {
                Checkable::Known(bg) => vec![bg],
                Checkable::TerminalsOwn => palette.kind.references().to_vec(),
                Checkable::TerminalsGuess => panic!(
                    "{what} in {}: cell {x},{y} has background {back:?}, a colour only the \
                     terminal knows",
                    palette.kind.name(),
                ),
            };
            assert!(
                !against.is_empty(),
                "{what} in {}: cell {x},{y} ({:?}) is drawn in {front:?} with no background to \
                 check it against",
                palette.kind.name(),
                cell.symbol(),
            );
            cells += 1;
            for bg in against {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= MINIMUM_CONTRAST,
                    "{what} in {} ({}): cell {x},{y} ({:?}{}) is {ratio:.2}:1 — {front:?} on \
                     #{:02x}{:02x}{:02x}, below {MINIMUM_CONTRAST}:1",
                    palette.kind.name(),
                    depth.words(),
                    cell.symbol(),
                    if reversed { ", reversed" } else { "" },
                    bg.0,
                    bg.1,
                    bg.2,
                );
                checks += 1;
            }
        }
    }
    (cells, checks, reversed_cells)
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
    let mut reversed = 0_usize;
    let mut screens_drawn = 0_usize;
    for resolved in every_theme_and_depth() {
        for screen in screens() {
            let backend = (screen.backend)();
            let (tx, rx) = mpsc::channel();
            let mut app = App::open("/repo", None, tx);
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
                let buffer = drawn(&app, &resolved, w, h);
                let what = format!("{} at {w}x{h}", screen.name);
                let (c, k, r) =
                    keeps_seven_to_one(&buffer, &what, &resolved.palette, resolved.depth);
                cells += c;
                checks += k;
                reversed += r;
            }
        }
    }
    println!(
        "RFC 059 matrix: {screens_drawn} screens drawn ({} per theme-depth), {cells} coloured \
         cells, {checks} contrast checks, {reversed} of them reversed",
        screens().len()
    );
    assert!(
        cells > 10_000,
        "the matrix checked only {cells} coloured cells, which is too few to have drawn the views"
    );
    // **And the selection was actually drawn in reverse video** (RFC 061 decision 4): a matrix that
    // saw no reversed cell would be asserting nothing about the swap it just learned.
    assert!(
        reversed > 1_000,
        "only {reversed} reversed cells: the selected rows are not being drawn across their width"
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
        let mut app = App::open("/repo", None, tx);
        drain(&mut app, &rx, &backend);
        // One column short, one row short, and far too small for either.
        for (w, h) in [(59, 15), (60, 14), (40, 10)] {
            let text = draw(&app, w, h);
            assert!(
                text.contains("too small for stikk") || text.contains("too small"),
                "{w}x{h} is below the minimum:\n{text}"
            );
            let buffer = drawn(&app, &resolved, w, h);
            let what = format!("the too-small notice at {w}x{h}");
            let (c, _, _) = keeps_seven_to_one(&buffer, &what, &resolved.palette, resolved.depth);
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
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        // Drain what reaching the screen recorded: `reach` draws, and those draws were at 120×40.
        let _ = crate::text::drain_overflows();
        for (w, h) in [(60, 15), (72, 20)] {
            // **Scrolled, so a surface that has a position actually shows one** (RFC 060 review v1
            // C1): a title's position is the part that may never be cut, and an unscrolled screen
            // of a scrollable surface does not draw one.
            for _ in 0..3 {
                let _ = drawn(&app, &resolved, w, h);
                app.nav_down();
            }
            let _ = crate::text::drain_overflows();
            let text = draw_in(&app, &resolved, w, h);
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
            // **And every position on screen is whole.** `lines a–b` with no `of c` is the cut
            // review v1 C1 found: not a missing word but a wrong statement.
            for row in text.lines() {
                if let Some(rest) = row.split_once("lines ").map(|(_, rest)| rest) {
                    let whole = rest
                        .split_once(" of ")
                        .is_some_and(|(_, total)| total.starts_with(|c: char| c.is_ascii_digit()));
                    if !whole {
                        found.push(format!(
                            "{} at {w}x{h}: a position is cut: {row:?}",
                            screen.name
                        ));
                    }
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} row(s) overflow their area:\n{}",
        found.len(),
        found.join("\n")
    );
    println!(
        "RFC 060: {screens_drawn} screens drawn scrolled at 60x15 and 72x20, zero overflows, \
         every position whole"
    );
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
                // The catalog's own words, which the Keys section now lists (RFC 061 decision 6).
                "A letter means one action, in either case.",
                "What can be done here",
            ],
        ),
    ] {
        let screen = screens()
            .into_iter()
            .find(|screen| screen.name == name)
            .expect("a screen of the matrix");
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        let text = draw_in(&app, &resolved, 60, 18);
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
    let (mut app, rx) = open("/repo");
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
        let buffer = drawn(&app, &look(), w, h);
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

/// **A frame's title is never cut, and its position is never half said** (RFC 060 review v1 C1).
///
/// Verify's was the one the review found: at 60 columns it drew
/// `┌ Verify · this run — v: raw · r: re-run · ↑/↓ · lines 1–11┐`, and the reader was told
/// *"lines 1–11"* with no *"of 24"*.
///
/// ```sh
/// cargo test -p stikk-tui --lib capture_rfc060_c1_titles -- --nocapture
/// ```
#[test]
fn capture_rfc060_c1_titles_at_60_columns() {
    let backend = stocked();
    let (mut app, rx) = open("/repo");
    drain(&mut app, &rx, &backend);
    app.open_verify();
    drain(&mut app, &rx, &backend);
    // Scrolled, so the title carries a position at all.
    for _ in 0..3 {
        let _ = draw(&app, 60, 15);
        app.nav_down();
    }
    let text = draw(&app, 60, 15);
    println!("--- RFC 060 C1: Verify at 60x15\n{text}");
    let title = text.lines().nth(1).unwrap_or_default();
    // **The keys went, as one group; the position stayed, whole.**
    assert!(
        title.contains(" of "),
        "the position is whole: {title:?}\n{text}"
    );
    // **No title names a key** (RFC 061 decision 5): the key line does, one row above the status
    // bar, and a title is a name and a position.
    assert!(
        !title.contains("v: raw") && !title.contains("r: re-run"),
        "a title names no key: {title:?}"
    );
    assert!(title.contains("Verify"), "{title:?}");
    // At 120 columns nothing is given up, and there is still no key in it.
    let roomy = draw(&app, 120, 40);
    let roomy_title = roomy.lines().nth(1).unwrap_or_default();
    println!("--- RFC 060 C1: Verify at 120x40\n{roomy_title}");
    assert!(
        roomy_title.contains("Verify \u{00b7} this run"),
        "{roomy_title:?}"
    );
    assert!(!roomy_title.contains("r: re-run"), "{roomy_title:?}");

    // The Glossary: the title says where, the foot line says which key, neither says both.
    app.open_glossary();
    let glossary = draw(&app, 60, 15);
    println!("--- RFC 060 C1: the Glossary at 60x15\n{glossary}");
    assert!(glossary.contains("lines 1–"), "{glossary}");
    assert!(glossary.contains(" of "), "{glossary}");
    assert!(glossary.contains("↓ more below"), "{glossary}");
    assert!(
        !glossary.contains("to scroll"),
        "the title no longer names the key as well:\n{glossary}"
    );
}

/// **What Enter opens, on every screen** (RFC 061 decision 3): one capture per row kind and per
/// screen with no row, at 60 × 15 and 80 × 24.
///
/// The list is a `ListPanel`, so RFC 060's recorder covers it; this asserts the rest — that it opens
/// where a row or a screen has actions, that it is titled by what it is about, that its first entry
/// is on screen, and that an entry that cannot run now carries its reason.
#[test]
fn capture_rfc061_what_can_be_done_here_on_every_screen() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut opened = 0_usize;
    let mut found: Vec<String> = Vec::new();
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        let before = format!("{:?}", app.top_overlay().map(std::mem::discriminant));
        app.select();
        let Some(stikk_app::Overlay::ActionList {
            subject,
            entries,
            readiness,
            ref_focused,
            ..
        }) = app.top_overlay()
        else {
            // Enter is unchanged on a card (RFC 061 trap 4), and a loading screen has no actions of
            // its own: both are screens this capture has nothing to show for.
            println!("--- {}: Enter opens no list (was {before})", screen.name);
            continue;
        };
        let (subject, readiness, ref_focused) = (subject.clone(), *readiness, *ref_focused);
        let entries: Vec<_> = entries.clone();
        opened += 1;
        let _ = crate::text::drain_overflows();
        for (w, h) in [(60, 15), (80, 24)] {
            let text = draw_in(&app, &resolved, w, h);
            println!(
                "--- what can be done here: {} ({subject}), {w}x{h}\n{text}",
                screen.name
            );
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
            // The first entry is the one the right arrow runs, and it is always on screen.
            let first = entries.first().expect("a list is never empty");
            assert!(
                text.contains(first.name),
                "{} at {w}x{h}: {} is not on screen\n{text}",
                screen.name,
                first.name
            );
            // **A reason, where one is on screen** (`TU-07`): a disabled entry is dimmed and says
            // why, on the row below it.
            for action in &entries {
                if let Some(reason) = action.unavailable_reason(readiness, ref_focused)
                    && text.contains(action.name)
                {
                    let flat = text
                        .lines()
                        .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let words = reason.split_whitespace().collect::<Vec<_>>().join(" ");
                    assert!(
                        flat.contains(&words) || h == 15,
                        "{} at {w}x{h}: {} is disabled and its reason is not on screen\n{text}",
                        screen.name,
                        action.name
                    );
                }
            }
        }
        // 60 x 15 is the floor, and the list must still be readable there: its title names what it
        // is about, however short the panel gets.
        let narrow = draw_in(&app, &resolved, 60, 15);
        assert!(
            narrow.contains("What can be done")
                || narrow.contains(subject.split(' ').next().unwrap_or("")),
            "{} at 60x15: the list says nothing about what it is about\n{narrow}",
            screen.name
        );
    }
    assert!(found.is_empty(), "rows overflowed:\n{}", found.join("\n"));
    assert!(opened >= 10, "only {opened} screens opened a list");
}

/// **The selected row is reverse video, across its whole width** (RFC 061 decision 4).
///
/// Every list view and every list overlay: the row the cursor marks is reversed from the marker to
/// the panel's inner edge, with no unreversed cell in between — which is the half-highlight the
/// padding exists to prevent, where a colour on one word was all a reader had.
#[test]
fn the_selected_row_is_reversed_across_its_width() {
    use ratatui::style::Modifier;

    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut rows_checked = 0_usize;
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        for with_list in [false, true] {
            if with_list {
                // And again with Enter's own list on top, whose entries are rows like any other.
                app.select();
            }
            for (w, h) in [(80, 24), (120, 40)] {
                let buffer = drawn(&app, &resolved, w, h);
                for y in buffer.area.top()..buffer.area.bottom() {
                    let symbol = |x: u16| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol().to_string())
                            .unwrap_or_default()
                    };
                    // The cursor markers every list uses.
                    let Some(marker) = (buffer.area.left()..buffer.area.right())
                        .find(|x| matches!(symbol(*x).as_str(), "\u{25b6}" | "\u{25b8}"))
                    else {
                        continue;
                    };
                    // The row ends at the next border to its right: an inner panel's, where one is
                    // open over the view.
                    let Some(edge) =
                        (marker..buffer.area.right()).find(|x| symbol(*x) == "\u{2502}")
                    else {
                        continue;
                    };
                    rows_checked += 1;
                    for x in marker..edge {
                        let cell = buffer.cell((x, y)).expect("inside the buffer");
                        assert!(
                            cell.modifier.contains(Modifier::REVERSED),
                            "{} at {w}x{h}: the selected row at {y} is not reversed at {x} \
                             ({:?}) — the highlight stops short of the edge at {edge}",
                            screen.name,
                            cell.symbol(),
                        );
                    }
                }
            }
            if with_list {
                app.back();
            }
        }
    }
    assert!(
        rows_checked >= 20,
        "only {rows_checked} selected rows were found to check"
    );
}

/// **The key line, on every screen at every size** (RFC 061 decision 5): one row above the status
/// bar, whole entries only, and `? help` and `q back` always there.
#[test]
fn capture_rfc061_the_key_line_at_every_size() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut found: Vec<String> = Vec::new();
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        let _ = crate::text::drain_overflows();
        for (w, h) in [(60, 15), (80, 24), (120, 40)] {
            let text = draw_in(&app, &resolved, w, h);
            let rows: Vec<&str> = text.lines().collect();
            // The key line is the row above the status bar, which is the last.
            let line = rows
                .get(rows.len().saturating_sub(2))
                .copied()
                .unwrap_or_default();
            println!("--- the key line: {} at {w}x{h}\n{line}", screen.name);
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
            // **An overlay can cover it** (RFC 060 decision 3's foot line is why), so this asserts
            // the line itself only where nothing is drawn over it.
            if app.has_overlay() {
                continue;
            }
            assert!(
                line.contains("? help") && line.contains("q back"),
                "{} at {w}x{h}: the two that never drop are not on the line:\n{line}",
                screen.name
            );
            // **Whole entries only**: every `·` has a key and a word on each side of it, and no
            // entry ends in a cut word — the line is built from whole entries or not at all.
            for entry in line.trim().split(" \u{00b7} ") {
                assert!(
                    entry.split_whitespace().count() >= 2,
                    "{} at {w}x{h}: {entry:?} is half an entry:\n{line}",
                    screen.name
                );
                assert!(
                    !entry.contains('\u{2026}'),
                    "{} at {w}x{h}: {entry:?} was shortened, and entries drop whole:\n{line}",
                    screen.name
                );
            }
            assert!(
                line.chars().count() <= usize::from(w),
                "{} at {w}x{h}: the line is {} columns:\n{line}",
                screen.name,
                line.chars().count()
            );
        }
    }
    assert!(found.is_empty(), "rows overflowed:\n{}", found.join("\n"));
}

/// **No title names a key** (RFC 061 decision 5): every frame on every screen, at three sizes.
#[test]
fn every_title_names_no_key() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut titles = 0_usize;
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        for (w, h) in [(60, 15), (80, 24), (120, 40)] {
            let text = draw_in(&app, &resolved, w, h);
            for row in text.lines().filter(|row| row.contains('\u{250c}')) {
                titles += 1;
                // The shapes titles used: `v: raw`, `a adopts`, `d compares it with…`, `↑/↓`.
                for named in [
                    "v: ",
                    "r: ",
                    "a adopts",
                    "x removes",
                    "\u{2191}/\u{2193}",
                    "d com",
                ] {
                    assert!(
                        !row.contains(named),
                        "{} at {w}x{h}: a title names {named:?}:\n{row}",
                        screen.name
                    );
                }
            }
        }
    }
    assert!(titles > 50, "only {titles} titles were checked");
}

// =============================================================================================
// RFC 060 Handoff B — the help scrolls with ↓, through the keys a reader presses.
// =============================================================================================

/// A key press, as the run loop makes it.
fn key(code: ratatui::crossterm::event::KeyCode) -> ratatui::crossterm::event::KeyEvent {
    ratatui::crossterm::event::KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::NONE)
}

/// A screen with its box border dropped and its whitespace collapsed, so a phrase that wrapped still
/// matches.
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

/// The `lines a–b of c` the title says, or `None` where the surface does not say one.
fn position(text: &str) -> Option<(usize, usize, usize)> {
    let row = text.lines().find(|line| line.contains("lines "))?;
    let rest = row.split_once("lines ")?.1;
    let (first, rest) = rest.split_once('\u{2013}')?;
    let (last, rest) = rest.split_once(" of ")?;
    let total: String = rest.chars().take_while(char::is_ascii_digit).collect();
    Some((
        first.trim().parse().ok()?,
        last.trim().parse().ok()?,
        total.parse().ok()?,
    ))
}

/// **↓ scrolls the help, and ↑ comes back** — through `handle_key`, which is the path a reader's
/// finger takes (RFC 060 Handoff B).
///
/// The defect: the Glossary kept an offset of its own that `nav_down` never reached, so `↓` moved
/// nothing while the foot line said *"↓ more below"*. The architect found it in a real Alacritty at
/// 2.3× text; sixty presses moved nothing. Every scrolling test called a handler, so none could see it.
#[test]
fn the_help_scrolls_with_down_through_the_key_path() {
    use ratatui::crossterm::event::KeyCode;

    for (down, up, what) in [
        (KeyCode::Down, KeyCode::Up, "the arrows"),
        (KeyCode::Char('j'), KeyCode::Char('k'), "j and k"),
    ] {
        let backend = stocked();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        let mut front = Front::default();
        drain(&mut app, &rx, &backend);

        // `?` opens the help, as a reader opens it.
        front.press(&mut app, key(KeyCode::Char('?')));
        let opened = front.draw(&mut app, 102, 24);
        println!("--- the help, opened with ? ({what}), 102x24\n{opened}");
        let (first, _, total) = position(&opened).unwrap_or_else(|| {
            panic!("the help must say where the reader is:\n{opened}");
        });
        assert_eq!(first, 1, "it opens at the top:\n{opened}");
        assert!(total > 24, "and it must not fit, or nothing is proven");
        assert!(opened.contains("\u{2193} more below"), "{opened}");

        // Ten presses of ↓, each moving the position by one — drawn between presses, because the
        // renderer owns the clamp.
        let mut previous = first;
        for press in 1..=10 {
            front.press(&mut app, key(down));
            let text = front.draw(&mut app, 102, 24);
            let (now, _, _) = position(&text)
                .unwrap_or_else(|| panic!("press {press} lost the position:\n{text}"));
            assert_eq!(
                now,
                previous + 1,
                "press {press} with {what} must move the help by one row:\n{text}"
            );
            previous = now;
        }

        // ↑ comes back, one row at a time.
        for press in 1..=10 {
            front.press(&mut app, key(up));
            let text = front.draw(&mut app, 102, 24);
            let (now, _, _) = position(&text)
                .unwrap_or_else(|| panic!("press {press} lost the position:\n{text}"));
            assert_eq!(
                now,
                previous - 1,
                "{what} must come back one row at a time:\n{text}"
            );
            previous = now;
        }
        assert_eq!(previous, 1, "and reach the top again");

        // **Enter changes nothing** (Handoff B §1.3): the help has nothing to choose, and Enter used
        // to be the only key that scrolled it.
        let before = front.draw(&mut app, 102, 24);
        for _ in 0..3 {
            front.press(&mut app, key(KeyCode::Enter));
        }
        let after = front.draw(&mut app, 102, 24);
        assert_eq!(after, before, "Enter must not scroll the help:\n{after}");
    }
}

/// **The end of the help**, reached by `↓` alone: the foot line turns over and the last row shows.
#[test]
fn down_reaches_the_end_of_the_help_and_the_foot_line_turns_over() {
    use ratatui::crossterm::event::KeyCode;

    let backend = stocked();
    let (tx, rx) = mpsc::channel();
    let mut app = App::open("/repo", None, tx);
    let mut front = Front::default();
    drain(&mut app, &rx, &backend);
    front.press(&mut app, key(KeyCode::Char('?')));

    let mut text = front.draw(&mut app, 102, 24);
    let (_, _, total) = position(&text).expect("a position");
    let mut presses = 0;
    while text.contains("\u{2193} more below") && presses < total + 10 {
        front.press(&mut app, key(KeyCode::Down));
        text = front.draw(&mut app, 102, 24);
        presses += 1;
    }
    println!("--- the help, {presses} presses of down, 102x24\n{text}");
    assert!(
        presses > 0 && presses < total,
        "{presses} presses of {total} rows"
    );
    assert!(
        text.contains("\u{2191} more above"),
        "at the end the foot line names the way back:\n{text}"
    );
    assert!(!text.contains("\u{2193} more below"), "{text}");
    let (_, last, _) = position(&text).expect("a position");
    assert_eq!(
        last, total,
        "the last row of the help is on screen:\n{text}"
    );
}

/// **The sweep** (RFC 060 Handoff B §2.3): every surface that advertises `↓`, or says where the
/// reader is, moves when `↓` is pressed — and every list moves its selection. Through `handle_key`,
/// at 60 × 15.
///
/// This is the test the help's defect needed. A surface that names a key and does not answer it is
/// the confusion RFC 060 decision 3 removed, and nothing was checking it: every scrolling test called
/// a handler by name, so a key wired to nothing looked exactly like a key wired correctly.
///
/// **What "moves" means depends on the surface**, as the handoff says: a card or a view that scrolls
/// advances its position, and a list moves its selection. The marker on screen tells the two apart.
/// The third bucket is the honest one — a surface whose content fits at 60 × 15 and which has no
/// second row to select says nothing about `↓`, and there is nothing to hold it to; the printed
/// coverage names every one of them.
#[test]
fn capture_rfc060b_every_surface_that_advertises_down_moves_on_down() {
    use ratatui::crossterm::event::KeyCode;

    let mut scrolled: Vec<String> = Vec::new();
    let mut selected: Vec<String> = Vec::new();
    let mut quiet: Vec<String> = Vec::new();
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        let mut front = Front::default();
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);

        let before = front.draw(&mut app, 60, 15);
        let advertises = before.contains("\u{2193} more below");
        let position_before = position(&before);
        let selectable = before.contains(crate::theme::SELECTED_MARKER.trim());
        // **The key, not the handler** (trap 2).
        front.press(&mut app, key(KeyCode::Down));
        let after = front.draw(&mut app, 60, 15);

        if advertises || position_before.is_some() {
            assert_ne!(
                after, before,
                "{}: the screen says `\u{2193}` does something and it did nothing:\n{before}",
                screen.name
            );
            if selectable {
                selected.push(screen.name.to_string());
                continue;
            }
            let Some((first, _, total)) = position_before else {
                // It advertised `↓` without a position: the frame changed, which is all it promised.
                scrolled.push(format!("{} (foot line only)", screen.name));
                continue;
            };
            let (now, last, _) = position(&after).unwrap_or_else(|| {
                panic!(
                    "{}: the position vanished after one press:\n{after}",
                    screen.name
                )
            });
            if first == now && last == total {
                // Already at its end: nothing left to scroll, and the frame still changed.
                scrolled.push(format!("{} (at its end)", screen.name));
                continue;
            }
            assert_eq!(
                now,
                first + 1,
                "{}: one press of `\u{2193}` must advance the position by one row:\n{after}",
                screen.name
            );
            scrolled.push(screen.name.to_string());
            continue;
        }

        if selectable && rows_to_select(&app) > 1 {
            // **A list always answers `↓`**, whether or not anything is hidden: the highlight moves.
            assert_ne!(
                after, before,
                "{}: a list with more than one row must move its selection on `\u{2193}`:\n{before}",
                screen.name
            );
            selected.push(screen.name.to_string());
            continue;
        }
        quiet.push(screen.name.to_string());
    }
    println!(
        "--- RFC 060 Handoff B, the sweep at 60x15\nscrolled ({}): {}\nselection moved ({}): \
         {}\nnothing to move ({}): {}",
        scrolled.len(),
        scrolled.join(", "),
        selected.len(),
        selected.join(", "),
        quiet.len(),
        quiet.join(", ")
    );
    // **Every surface that advertised `↓` was held to it**, and the five that scroll are the five
    // RFC 060 decision 3 gave a foot line to.
    assert!(
        scrolled.len() >= 5,
        "only {} scrolling surfaces were held to `\u{2193}`: {scrolled:?}",
        scrolled.len()
    );
    assert!(
        selected.len() >= 5,
        "only {} lists moved their selection: {selected:?}",
        selected.len()
    );
}

/// How many rows the surface under the cursor offers — `2` is enough to prove a selection moved, and
/// the sweep only asks whether there is more than one.
fn rows_to_select(app: &App) -> usize {
    match app.top_overlay() {
        Some(stikk_app::Overlay::RefPicker { refs, .. }) => refs.len(),
        Some(stikk_app::Overlay::Refusals { records, .. }) => records.len(),
        Some(stikk_app::Overlay::Palette { filter, .. }) => {
            stikk_core::actions::matching(filter).len()
        }
        Some(stikk_app::Overlay::ActionList { entries, .. }) => entries.len(),
        Some(_) => 0,
        None => match app.focus() {
            Focus::History(view, _) => view.blocks.len(),
            Focus::Queue(view, _, _) => view.patches.len(),
            Focus::BlockDetail(view, _, _) => stikk_core::block_cursor_len(view),
            Focus::Trust(stikk_core::TrustView::Listed { rows, .. }, _, _) => rows.len(),
            Focus::Compare(focus) => focus.view.listed().map_or(0, |c| c.entries.len()),
            _ => 0,
        },
    }
}

// =============================================================================================
// RFC 060 Handoff C — no view stops without saying so.
// =============================================================================================

/// **Every surface that cannot show all its rows says so** (RFC 060 Handoff C §2), over RFC 059's
/// whole matrix at 60 × 15 and 72 × 20.
///
/// `record_cut` is `record_overflow` turned ninety degrees: the width check holds a row to its area,
/// and this holds a *surface* to its area's height. Block detail drew eleven rows of twenty-two at
/// 60 × 15 with no position and no scroll, and every width check passed.
///
/// **It is not vacuous**: the count of surfaces that really did hide rows is asserted, so a matrix
/// whose fixtures all fit would fail rather than pass quietly.
#[test]
fn capture_rfc060c_no_view_stops_without_saying_so() {
    let resolved = resolved_for(stikk_state::config::Theme::Dark, None);
    let mut silent: Vec<String> = Vec::new();
    let mut said: Vec<String> = Vec::new();
    for screen in screens() {
        let backend = (screen.backend)();
        let (tx, rx) = mpsc::channel();
        let mut app = App::open("/repo", None, tx);
        if screen.first_read {
            drain(&mut app, &rx, &backend);
        }
        (screen.reach)(&mut app, &rx, &backend);
        // What reaching the screen recorded was drawn at another size.
        let _ = crate::text::drain_overflows();
        for (w, h) in [(60, 15), (72, 20)] {
            let text = draw_in(&app, &resolved, w, h);
            for overflow in crate::text::drain_overflows() {
                // The height check's own record, which names itself.
                if overflow.text.contains("the title says nothing") {
                    silent.push(format!(
                        "{} at {w}x{h}: {} — {}",
                        screen.name, overflow.what, overflow.text
                    ));
                }
            }
            if let Some((first, last, total)) = position(&text)
                && total > last - first + 1
            {
                said.push(format!(
                    "{} at {w}x{h} ({first}–{last} of {total})",
                    screen.name
                ));
            }
        }
    }
    println!(
        "--- RFC 060 Handoff C: {} surface(s) hid rows and said so\n{}",
        said.len(),
        said.join("\n")
    );
    assert!(
        silent.is_empty(),
        "{} surface(s) stopped without saying so:\n{}",
        silent.len(),
        silent.join("\n")
    );
    assert!(
        said.len() >= 10,
        "only {} surface(s) hid rows at all, which is too few to have proven anything",
        said.len()
    );
}

/// **Block detail, by keys, at 60 × 15**: every row is reachable with `↓`, and the position says
/// where (RFC 060 Handoff C §1). Through `handle_key` (trap 2).
#[test]
fn capture_rfc060c_block_detail_reaches_every_row_at_60x15() {
    use ratatui::crossterm::event::KeyCode;

    let backend = stocked();
    let (tx, rx) = mpsc::channel();
    let mut app = App::open("/repo", None, tx);
    let mut front = Front::default();
    drain(&mut app, &rx, &backend);
    // History, then its tip block — with the right arrow, which opens a row (RFC 061 decision 2).
    front.press(&mut app, key(KeyCode::Right));
    drain(&mut app, &rx, &backend);
    front.press(&mut app, key(KeyCode::Right));
    drain(&mut app, &rx, &backend);

    let top = front.draw(&mut app, 60, 15);
    println!("--- Block detail at 60x15, at the top\n{top}");
    let (first, last, total) = position(&top).unwrap_or_else(|| {
        panic!("Block detail must say where the reader is at 60x15:\n{top}");
    });
    assert_eq!(first, 1, "it opens at the top");
    assert!(total > last, "and it must not fit, or nothing is proven");

    // Every row, with ↓ alone.
    let mut seen = flattened(&top);
    let mut presses = 0;
    let mut text = top.clone();
    while position(&text).is_some_and(|(_, last, total)| last < total) && presses < total + 10 {
        front.press(&mut app, key(KeyCode::Down));
        text = front.draw(&mut app, 60, 15);
        seen.push(' ');
        seen.push_str(&flattened(&text));
        presses += 1;
    }
    println!("--- Block detail at 60x15, after {presses} presses of down\n{text}");
    assert_eq!(
        position(&text).map(|(_, last, total)| (last, total)),
        Some((total, total)),
        "↓ must reach the last row:\n{text}"
    );
    // The rows that were out of reach before this handoff.
    for row in [
        "prev-ref-state",
        "state at tip",
        "Patches not shown",
        "Enter on a file saves it",
    ] {
        assert!(
            seen.contains(row),
            "{row:?} must be reachable at 60x15 with \u{2193}:\n{text}"
        );
    }
    // And ↑ comes back to the top.
    for _ in 0..presses {
        front.press(&mut app, key(KeyCode::Up));
    }
    let back = front.draw(&mut app, 60, 15);
    assert_eq!(
        position(&back).map(|(first, _, _)| first),
        Some(1),
        "{back}"
    );
}

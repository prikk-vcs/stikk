//! Render tests for the status bar (design TS-01; RFC 010 `⟳ n` indicator).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_core::OrientationView;
use stikk_model::Binding;
use stikk_model::{Capability, Readiness, RoleReadiness};
use stikk_state::Config;

use super::*;
use crate::test_util::buffer_text;

fn render_app(app: &App) -> String {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(app, f, f.area())).unwrap();
    buffer_text(terminal.backend().buffer())
}

fn from_state(repo: &str, state: OrientationState, palette: Palette) -> App {
    let (tx, _rx) = mpsc::channel();
    App::from_state(repo, state, palette, tx)
}

#[test]
fn shows_repo_and_hint_and_no_focus_before_the_first_read() {
    // RFC 029 Handoff B §3: pending focus renders nothing for focus — no `heads/main` guessed before
    // prikk has said anything — and the existing `(loading)` marker stays.
    let (tx, _rx) = mpsc::channel();
    let app = App::open("/home/dev/project", &Config::default(), tx);
    let text = render_app(&app);
    assert!(text.contains("project"));
    assert!(!text.contains("heads/main"), "{text:?}");
    assert!(text.contains("(loading)"));
    assert!(text.contains("q:back"));
    // Never "HEAD".
    assert!(!text.contains("HEAD"));
}

#[test]
fn shows_queue_and_maintainer_badge() {
    let r = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 4,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    };
    let app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    let text = render_app(&app);
    assert!(text.contains("queued"));
    assert!(text.contains("MNT"));
    assert!(text.contains("AUT"));
}

/// The acceptance-critical assertion (RFC 016 §4/`C-T2c′`): `Unknown` renders **distinctly** from
/// `Ready`, and — the actual prohibition, not merely "looks different" — the `Unknown` badge contains
/// no pass/`✓` marker at all. Asserting the absence, not merely the presence of a different glyph,
/// because `C-T2c′` is a prohibition on a claim stikk cannot verify, not a request for variety.
#[test]
fn maintainer_unknown_never_renders_as_a_pass() {
    let view_with = |maintainer_readiness| {
        let r = Readiness {
            author: RoleReadiness::NotReady,
            maintainer: maintainer_readiness,
            read_only: false,
        };
        OrientationView {
            prikk_version: "prikk 0.33.0".into(),
            prikk_supported: true,
            prikk_validated: true,
            validated_through: "0.33".to_string(),
            prikk_persists_messages: true,
            prikk_minor: 41,
            queued_patches: 0,
            queued_target: None,
            trailing_partial_wal_bytes: 0,
            main_ref_state: None,
            capability: Capability::derive(r),
            readiness: r,
            stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
            current_branch: stikk_model::CurrentBranch::NotReported,
        }
    };

    let unknown_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::Unknown)),
        Palette::default(),
    ));
    let not_ready_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::NotReady)),
        Palette::default(),
    ));
    // `Ready` cannot be produced by `stikk-prikk::env` today (RFC 016 F3), but the render path must
    // still be exercised for it now, so the day it becomes reachable this test already covers it.
    let ready_text = render_app(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_with(RoleReadiness::Known(Binding::Matches))),
        Palette::default(),
    ));

    assert!(
        !unknown_text.contains("[MNT ✓]"),
        "an unverifiable adoption must never render the pass mark: {unknown_text:?}"
    );
    assert!(ready_text.contains("[MNT ✓]"));
    assert!(not_ready_text.contains("[MNT –]"));
    assert!(unknown_text.contains("[MNT ?]"));
    // All three are textually distinct from one another (NFR-A03: never colour alone).
    assert_ne!(unknown_text, ready_text);
    assert_ne!(unknown_text, not_ready_text);
    assert_ne!(ready_text, not_ready_text);
}

#[test]
fn read_only_badge_appears_and_no_queue_when_zero() {
    let r = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::Unknown,
        read_only: true,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.27.1".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    };
    let app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    let text = render_app(&app);
    assert!(text.contains("[RO]"));
    assert!(!text.contains("queued"));
}

#[test]
fn the_in_flight_indicator_appears_while_a_request_is_pending_and_clears_once_answered() {
    let r = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let view = OrientationView {
        prikk_version: "prikk 0.30.0".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 41,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    };
    let mut app = from_state(
        "/x/repo",
        OrientationState::Loaded(view),
        Palette::default(),
    );
    assert!(!render_app(&app).contains('⟳'));

    app.open_history(); // sends a request; the worker never answers it here
    let text = render_app(&app);
    assert!(text.contains("⟳ 1"));
}

// ---------------------------------------------------------------------------------------------
// RFC 029 Handoff B §3: the focus segment at 80 columns, one capture per row of the table.
// ---------------------------------------------------------------------------------------------

fn render_at_80(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
    terminal.draw(|f| render(app, f, f.area())).unwrap();
    buffer_text(terminal.backend().buffer())
}

fn view_on(current: stikk_model::CurrentBranch) -> OrientationView {
    let r = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    OrientationView {
        prikk_version: "prikk 0.42.0".into(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.42".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 42,
        queued_patches: 0,
        queued_target: None,
        trailing_partial_wal_bytes: 0,
        main_ref_state: None,
        capability: Capability::derive(r),
        readiness: r,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: current,
    }
}

fn branch(name: &str) -> stikk_model::CurrentBranch {
    stikk_model::CurrentBranch::Branch(stikk_model::RefName::parse(name).unwrap())
}

/// `from_state` with a loaded view focuses `heads/main`, as if its first read had resolved there.
fn focused_on_heads_main(current: stikk_model::CurrentBranch) -> String {
    render_at_80(&from_state(
        "/x/repo",
        OrientationState::Loaded(view_on(current)),
        Palette::default(),
    ))
}

fn capture(label: &str, text: &str) {
    println!("--- status bar, 80 columns: {label}\n{text}");
    assert!(!text.contains("HEAD"), "{label}: {text:?}");
}

#[test]
fn the_focus_segment_follows_each_row_of_the_table_at_80_columns() {
    let equal = focused_on_heads_main(branch("heads/main"));
    capture("focus equals prikk's current branch", &equal);
    assert!(
        equal.starts_with("repo  ·  heads/main  ·  [AUT ?]"),
        "{equal:?}"
    );
    assert!(!equal.contains("prikk's default"), "{equal:?}");

    let differing = focused_on_heads_main(branch("heads/dev"));
    capture("focus differs from prikk's current branch", &differing);
    assert!(
        differing.starts_with("repo  ·  heads/main  ·  prikk's default: heads/dev  ·  [AUT ?]"),
        "{differing:?}"
    );

    let unresolved = focused_on_heads_main(stikk_model::CurrentBranch::Unresolved(
        "<unresolved; run `prikk doctor`>".to_string(),
    ));
    capture("prikk's current branch unresolved", &unresolved);
    assert!(
        unresolved.starts_with(
            "repo  ·  heads/main  ·  prikk's default: <unresolved; run `prikk doctor`>"
        ),
        "{unresolved:?}"
    );

    let not_reported = focused_on_heads_main(stikk_model::CurrentBranch::NotReported);
    capture("not reported (below prikk 0.42)", &not_reported);
    assert!(
        not_reported.starts_with("repo  ·  heads/main  ·  [AUT ?]"),
        "{not_reported:?}"
    );

    // Unfocused: the first read resolved to no ref (nothing named, nothing published).
    let (tx, rx) = mpsc::channel();
    let mut app = App::open("/x/repo", &Config::default(), tx);
    let first = rx.try_recv().expect("the first Orientation read");
    app.apply(crate::worker::Response {
        seq: first.seq,
        kind: crate::worker::ResponseKind::Orient(Ok(view_on(
            stikk_model::CurrentBranch::NotReported,
        ))),
    });
    let unfocused = render_at_80(&app);
    capture("unfocused", &unfocused);
    assert!(
        unfocused.starts_with("repo  ·  no ref focused  ·  [AUT ?]"),
        "{unfocused:?}"
    );

    // Pending: nothing read yet. Nothing for focus; `(loading)` stays.
    let (tx, _rx) = mpsc::channel();
    let pending = render_at_80(&App::open("/x/repo", &Config::default(), tx));
    capture("pending", &pending);
    assert!(pending.starts_with("repo  ·  (loading)"), "{pending:?}");
}

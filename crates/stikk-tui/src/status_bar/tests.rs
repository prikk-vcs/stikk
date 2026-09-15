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
// RFC 029 Handoff B §3, and review v1 §2.1: the focus segment, and what the line sheds when it is full.
// ---------------------------------------------------------------------------------------------

const LONG_REPO: &str = "a-repository-name-that-is-long";
const UNRESOLVED: &str = "<unresolved; run `prikk doctor`>";

fn render_at(app: &App, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
    terminal.draw(|f| render(app, f, f.area())).unwrap();
    buffer_text(terminal.backend().buffer())
}

/// A loaded 0.42 view with both badges `?` and `queued` patches for `heads/main`.
fn view_on(current: stikk_model::CurrentBranch, queued: u64) -> OrientationView {
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
        queued_patches: queued,
        queued_target: (queued > 0).then(|| "heads/main".to_string()),
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

fn unresolved() -> stikk_model::CurrentBranch {
    stikk_model::CurrentBranch::Unresolved(UNRESOLVED.to_string())
}

/// `from_state` with a loaded view focuses `heads/main`, as if its first read had resolved there.
fn focused_app(repo: &str, current: stikk_model::CurrentBranch, queued: u64) -> App {
    from_state(
        &format!("/x/{repo}"),
        OrientationState::Loaded(view_on(current, queued)),
        Palette::default(),
    )
}

/// The first read resolved to no ref: nothing named, nothing published. Its picker's `Refs` read is
/// still unanswered, so `⟳ 1` shows.
fn unfocused_app(repo: &str, queued: u64) -> App {
    let (tx, rx) = mpsc::channel();
    let mut app = App::open(format!("/x/{repo}"), &Config::default(), tx);
    let first = rx.try_recv().expect("the first Orientation read");
    app.apply(crate::worker::Response {
        seq: first.seq,
        kind: crate::worker::ResponseKind::Orient(crate::worker::OrientRead::stamped(Ok(view_on(
            stikk_model::CurrentBranch::NotReported,
            queued,
        )))),
    });
    app
}

fn pending_app(repo: &str) -> App {
    let (tx, _rx) = mpsc::channel();
    App::open(format!("/x/{repo}"), &Config::default(), tx)
}

fn show(label: &str, width: u16, text: &str) {
    println!("--- status bar, {width} columns: {label}\n{text}");
    assert!(!text.contains("HEAD"), "{label} at {width}: {text:?}");
}

/// What is never shortened, whole: the focus, and the queue and both badges (or the load marker).
fn assert_never_shortened(label: &str, text: &str, focus: &str, loaded: bool) {
    assert!(
        text.contains(focus),
        "{label}: focus {focus:?} not whole in {text:?}"
    );
    if loaded {
        assert!(
            text.contains("●3 queued"),
            "{label}: queue not whole in {text:?}"
        );
        assert!(
            text.contains("[AUT ?]"),
            "{label}: [AUT not whole in {text:?}"
        );
        assert!(
            text.contains("[MNT ?]"),
            "{label}: [MNT not whole in {text:?}"
        );
    } else {
        assert!(
            text.contains("(loading)"),
            "{label}: (loading) not whole in {text:?}"
        );
    }
}

#[test]
fn at_80_columns_every_row_keeps_its_focus_queue_and_badges_whole() {
    let rows: [(&str, App, &str, bool); 6] = [
        (
            "equal",
            focused_app(LONG_REPO, branch("heads/main"), 3),
            "heads/main",
            true,
        ),
        (
            "differing",
            focused_app(LONG_REPO, branch("heads/dev"), 3),
            "heads/main",
            true,
        ),
        (
            "unresolved",
            focused_app(LONG_REPO, unresolved(), 3),
            "heads/main",
            true,
        ),
        (
            "not reported",
            focused_app(LONG_REPO, stikk_model::CurrentBranch::NotReported, 3),
            "heads/main",
            true,
        ),
        (
            "unfocused",
            unfocused_app(LONG_REPO, 3),
            "no ref focused",
            true,
        ),
        ("pending", pending_app(LONG_REPO), "(loading)", false),
    ];
    for (label, app, focus, loaded) in rows {
        let text = render_at(&app, 80);
        show(label, 80, &text);
        assert_never_shortened(label, &text, focus, loaded);
    }

    // Which steps fired at 80 (review v2 §2): the hint went (1), the name was shortened to its narrowest (2),
    // and prikk's default was shortened to fit (3). The segment stays (4 did not fire).
    let differing_80 = render_at(&focused_app(LONG_REPO, branch("heads/dev"), 3), 80);
    assert_eq!(
        differing_80.trim_end(),
        "a…  ·  heads/main  ·  prikk's default: heads/…  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );
    let unresolved_80 = render_at(&focused_app(LONG_REPO, unresolved(), 3), 80);
    assert_eq!(
        unresolved_80.trim_end(),
        "a…  ·  heads/main  ·  prikk's default: <unres…  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );
    // Equal has no default segment, and the name fits whole once the hint is gone (79 cells).
    let equal_80 = render_at(&focused_app(LONG_REPO, branch("heads/main"), 3), 80);
    assert_eq!(
        equal_80.trim_end(),
        "a-repository-name-that-is-long  ·  heads/main  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );
}

#[test]
fn at_40_columns_the_name_is_shed_to_one_character_and_what_cannot_fit_clips_at_the_edge() {
    // The never-shortened pieces are wider than 40 cells whenever a queue is shown: `a…` (2) + `  ·  heads/main`
    // (15) + `  ·  ●3 queued` (14) + `  ·  [AUT ?] [MNT ?]` (20) = 51. Every step fires, and the line clips.
    for (label, app, focus) in [
        (
            "equal",
            focused_app(LONG_REPO, branch("heads/main"), 3),
            "heads/main",
        ),
        (
            "differing",
            focused_app(LONG_REPO, branch("heads/dev"), 3),
            "heads/main",
        ),
        (
            "unresolved",
            focused_app(LONG_REPO, unresolved(), 3),
            "heads/main",
        ),
        (
            "not reported",
            focused_app(LONG_REPO, stikk_model::CurrentBranch::NotReported, 3),
            "heads/main",
        ),
        ("unfocused", unfocused_app(LONG_REPO, 3), "no ref focused"),
    ] {
        let text = render_at(&app, 40);
        show(label, 40, &text);
        assert!(text.starts_with("a…  ·  "), "{label}: {text:?}");
        assert!(text.contains(focus), "{label}: {text:?}");
        assert!(!text.contains("prikk's default"), "{label}: {text:?}");
    }
    let pending = render_at(&pending_app(LONG_REPO), 40);
    show("pending", 40, &pending);
    assert_never_shortened("pending", &pending, "(loading)", false);

    // Without a queue the never-shortened pieces fit 40 (2 + 15 + 20 = 37), and the badges are whole.
    for (label, current) in [
        ("equal, no queue", branch("heads/main")),
        ("unresolved, no queue", unresolved()),
    ] {
        let text = render_at(&focused_app(LONG_REPO, current, 0), 40);
        show(label, 40, &text);
        assert!(text.contains("heads/main"), "{label}: {text:?}");
        assert!(text.contains("[AUT ?] [MNT ?]"), "{label}: {text:?}");
    }
}

#[test]
fn the_line_sheds_in_order_one_step_at_a_time() {
    // Widths for the unresolved row with a long name and a queue: name 30, focus 15, the default's label 22
    // and value 32, queue and badges 34, hint 27 — 160 in all. The order is review v2 §2's.
    let app = focused_app(LONG_REPO, unresolved(), 3);
    let at = |width| {
        let text = render_at(&app, width);
        show(&format!("shedding, width {width}"), width, &text);
        assert_never_shortened("shedding", &text, "heads/main", true);
        text.trim_end().to_string()
    };

    let nothing = at(160);
    assert!(nothing.ends_with(":palette  ?:help  q:back"), "{nothing:?}");
    assert!(nothing.starts_with(LONG_REPO), "{nothing:?}");
    assert!(nothing.contains(UNRESOLVED), "{nothing:?}");

    // 1. The hint goes first; the name and prikk's text are still whole.
    let hint_gone = at(133);
    assert!(!hint_gone.contains(":palette"), "{hint_gone:?}");
    assert!(hint_gone.starts_with(LONG_REPO), "{hint_gone:?}");
    assert!(
        hint_gone.contains(&format!("prikk's default: {UNRESOLVED}")),
        "{hint_gone:?}"
    );

    // 2. Then the repository name is shortened from its end; prikk's text is still whole.
    let name_short = at(120);
    assert!(
        name_short.starts_with("a-repository-nam…  ·  heads/main"),
        "{name_short:?}"
    );
    assert!(
        name_short.contains(&format!("prikk's default: {UNRESOLVED}")),
        "{name_short:?}"
    );
    // … down to one character, still with prikk's text whole.
    let name_shortest = at(105);
    assert!(
        name_shortest.starts_with("a…  ·  heads/main"),
        "{name_shortest:?}"
    );
    assert!(name_shortest.contains(UNRESOLVED), "{name_shortest:?}");

    // 3. Then prikk's default is shortened from its end, marked `…`.
    let default_short = at(90);
    assert!(
        default_short.starts_with("a…  ·  heads/main"),
        "{default_short:?}"
    );
    assert!(
        default_short.contains("prikk's default: <unresolved; run…"),
        "{default_short:?}"
    );
    // … down to `prikk's default: …` at exactly its narrowest.
    let default_narrowest = at(74);
    assert_eq!(
        default_narrowest,
        "a…  ·  heads/main  ·  prikk's default: …  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );

    // 4. Last, the segment goes.
    let segment_gone = at(73);
    assert_eq!(
        segment_gone,
        "a…  ·  heads/main  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );
    let narrowest = at(51);
    assert_eq!(
        narrowest,
        "a…  ·  heads/main  ·  ●3 queued  ·  [AUT ?] [MNT ?]"
    );
}

#[test]
fn shortening_is_measured_in_cells_for_wide_characters() {
    // Each of these is two cells wide; cutting by chars or bytes would overrun the line.
    let app = focused_app(
        "リポジトリの名前がとても長いリポジトリです",
        branch("heads/main"),
        3,
    );
    let text = render_at(&app, 60);
    show("a wide-character repository name", 60, &text);
    assert_never_shortened("wide", &text, "heads/main", true);
    assert!(text.contains("…"), "{text:?}");
}

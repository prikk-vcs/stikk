//! Render tests for the Orientation view (design TS-01, using `TestBackend`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use stikk_model::{Capability, Readiness, RoleReadiness};

use super::*;
use crate::test_util::buffer_text;

fn view(readiness: Readiness, supported: bool, queued: u64, partial: u64) -> OrientationView {
    OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.27.1".to_string(),
        prikk_supported: supported,
        prikk_validated: supported,
        validated_through: "0.32".to_string(),
        prikk_persists_messages: false,
        prikk_minor: 41, // fixed "prikk 0.27.1" above is well below the 0.32 threshold
        queued_patches: queued,
        queued_target: None,
        trailing_partial_wal_bytes: partial,
        main_ref_state: Some("237d0681".to_string()),
        capability: Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

fn render_to_text(v: &OrientationView) -> String {
    let backend = TestBackend::new(90, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(v, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn shows_version_capability_and_readiness() {
    let r = Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let text = render_to_text(&view(r, true, 0, 0));
    assert!(text.contains("prikk 0.27.1"));
    assert!(text.contains("supported"));
    assert!(text.contains("maintainer"));
    // RFC 026: on this band the author key is present and unverified, and says so — the
    // vocabulary is now the same shape for both roles.
    assert!(text.contains("author present, unverified"));
    assert!(text.contains("Orientation"));
}

#[test]
fn maintainer_unknown_is_spelled_out_never_collapsed_to_ready() {
    // RFC 016 §3/`C-T2c′`: the signing-readiness line must say "unknown", never bare "ready" — the
    // same prohibition the status-bar badge test enforces, applied to Orientation's own text line.
    let r = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::Unknown,
        read_only: false,
    };
    let text = render_to_text(&view(r, true, 0, 0));
    assert!(text.contains("maintainer present, adoption unknown"));
}

#[test]
fn viewer_when_no_readiness() {
    let text = render_to_text(&view(Readiness::none(), true, 0, 0));
    assert!(text.contains("viewer"));
    assert!(text.contains("not ready"));
}

#[test]
fn surfaces_queue_and_torn_tail() {
    let text = render_to_text(&view(Readiness::none(), true, 3, 7));
    assert!(text.contains("queued"));
    assert!(text.contains('3'));
    assert!(text.contains("torn tail"));
}

#[test]
fn unsupported_prikk_is_flagged() {
    let text = render_to_text(&view(Readiness::none(), false, 0, 0));
    assert!(text.contains("outside stikk's validated range"));
}

#[test]
fn a_supported_but_unvalidated_prikk_says_so_without_degrading() {
    // RFC 009 decision 7: above the validated ceiling, stikk still runs but says its shapes have not
    // been checked — never silently asserting a validation it has not done. The notice is long enough
    // to wrap across rows (it is now `Wrap`-enabled — TU-11), so join rows before matching a phrase
    // that could otherwise straddle a wrap point.
    //
    // `validated_through` is read here, never hardcoded (RFC 015: this exact literal drifted once
    // already — the render still said "0.30" after RFC 012 F-e had raised the real ceiling to 31, and
    // no test caught it because this test hardcoded the same stale number the renderer did).
    let mut v = view(Readiness::none(), true, 0, 0);
    v.prikk_validated = false;
    v.validated_through = "0.99".to_string();
    let text = render_to_text(&v).replace('\n', " ");
    assert!(text.contains("validated through 0.99"));
    assert!(text.contains("have not been checked"));
    assert!(!text.contains("outside stikk's validated range")); // still runs, not degraded
}

#[test]
fn queued_target_renders_next_to_the_count() {
    // RFC 009 F1: showing the queue's target ref is strictly more honest than a bare count.
    let mut v = view(Readiness::none(), true, 3, 0);
    v.queued_target = Some("heads/main".to_string());
    let text = render_to_text(&v);
    assert!(text.contains("targeting"));
    assert!(text.contains("heads/main"));
}

#[test]
fn a_hostile_queued_target_is_rendered_inert() {
    let mut v = view(Readiness::none(), true, 1, 0);
    v.queued_target = Some("\u{1b}[2Jheads/main".to_string());
    let text = render_to_text(&v);
    assert!(!text.contains('\u{1b}'), "the ESC must be neutralized");
}

#[test]
fn a_hostile_ref_state_is_rendered_inert() {
    // C-T2a: a control sequence in a repository-sourced string must not survive to the terminal.
    let mut v = view(Readiness::none(), true, 0, 0);
    v.main_ref_state = Some("\u{1b}[2Jhello".to_string());
    let text = render_to_text(&v);
    assert!(!text.contains('\u{1b}'), "the ESC must be neutralized");
}

/// RFC 029 Handoff B review v1 §2.1: prikk's current branch, whole, in a `branch` row directly above
/// `heads/main` — the home for what the status bar may shorten.
#[test]
fn prikks_current_branch_has_a_whole_row_above_heads_main() {
    let with = |current| OrientationView {
        interrupted_materialization: None,
        current_branch: current,
        ..view(Readiness::none(), true, 0, 0)
    };
    let row = |text: &str, needle: &str| {
        text.lines()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} not on screen:\n{text}"))
    };

    let branch = render_to_text(&with(stikk_model::CurrentBranch::Branch(
        stikk_model::RefName::parse("heads/dev").unwrap(),
    )));
    println!("--- Orientation, a branch\n{branch}");
    assert!(
        branch.contains("branch      heads/dev — prikk's current branch"),
        "{branch}"
    );
    assert_eq!(
        row(&branch, "branch      heads/dev") + 1,
        row(&branch, "heads/main")
    );

    // **Unresolved is a sentence, drawn as one** (RFC 057 decision 3): no trailing
    // *"— prikk's current branch"* clause, which would be a second em-dash aside on a row that
    // already has one. Below the JSON band the sentence is prikk's sentinel; at or above it,
    // stikk's own. Both are drawn the same way.
    for said in [
        "<unresolved; run `prikk doctor`>",
        "unresolved — `prikk doctor` can say why",
    ] {
        let unresolved = render_to_text(&with(stikk_model::CurrentBranch::Unresolved(
            said.to_string(),
        )));
        println!("--- Orientation, unresolved: {said}\n{unresolved}");
        assert!(
            unresolved.contains(&format!("branch      {said}")),
            "{unresolved}"
        );
        assert!(
            !unresolved.contains(&format!("{said} — prikk's current branch")),
            "a sentence takes no name's suffix: {unresolved}"
        );
    }

    let below_0_42 = render_to_text(&with(stikk_model::CurrentBranch::NotReported));
    assert!(
        !below_0_42.contains("prikk's current branch"),
        "{below_0_42}"
    );
    assert!(!below_0_42.contains("  branch "), "{below_0_42}");
}

/// RFC 034 §7: a checkout that stopped part-way is a **state of the repository**, not a fault —
/// reads keep working and only commit refuses. **stikk's lead-in, then prikk's routes, each
/// whole** (RFC 057 decision 2): the JSON names the ways out as commands and writes no sentence
/// around them, so the framing is stikk's and the commands are carried exactly.
#[test]
fn rfc034_an_interrupted_materialization_shows_stikks_lead_in_and_prikks_routes() {
    const ROUTES: [&str; 2] = [
        "prikk checkout --patch-materialize --ref heads/main",
        "prikk branch switch heads/main",
    ];
    let mut v = view(Readiness::none(), true, 0, 0);
    v.interrupted_materialization = Some(ROUTES.iter().map(ToString::to_string).collect());
    let text = render_to_text(&v);
    println!("--- RFC 057: a checkout that stopped part-way, 90 columns\n{text}");
    let flat: String = text
        .lines()
        .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(flat.contains("interrupted"), "{text}");
    assert!(
        flat.contains(stikk_core::INTERRUPTED_MATERIALIZATION),
        "stikk's lead-in, whole:\n{text}"
    );
    assert!(
        !flat.contains("prikk reports"),
        "the words are stikk's now:\n{text}"
    );
    for route in ROUTES {
        assert!(flat.contains(route), "prikk's route, whole:\n{text}");
    }

    // Nothing is said when prikk reports nothing.
    let quiet = render_to_text(&view(Readiness::none(), true, 0, 0));
    assert!(!quiet.contains("interrupted"), "{quiet}");
}

/// **RFC 057's captures, at exactly 80 columns**: the two lines stikk now words itself, as a
/// reader meets them on Orientation.
///
/// ```text
/// cargo test -p stikk-tui --lib capture_rfc057 — --nocapture
/// ```
#[test]
fn capture_rfc057_orientation_at_80_columns() {
    let at_80 = |v: &OrientationView| {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render(v, &Palette::default(), f, f.area()))
            .unwrap();
        crate::test_util::buffer_text(terminal.backend().buffer())
    };

    let interrupted = OrientationView {
        interrupted_materialization: Some(vec![
            "prikk checkout --patch-materialize --ref heads/main".to_string(),
            "prikk branch switch heads/main".to_string(),
        ]),
        ..view(Readiness::none(), true, 0, 0)
    };
    let text = at_80(&interrupted);
    println!("--- RFC 057: an interrupted materialization, 80 columns\n{text}");
    assert!(
        text.contains("stopped part-way"),
        "stikk's lead-in, wrapped not clipped:\n{text}"
    );

    let unresolved = OrientationView {
        current_branch: stikk_model::CurrentBranch::Unresolved(
            "unresolved — `prikk doctor` can say why".to_string(),
        ),
        ..view(Readiness::none(), true, 0, 0)
    };
    let text = at_80(&unresolved);
    println!("--- RFC 057: a current branch prikk cannot resolve, 80 columns\n{text}");
    assert!(text.contains("`prikk doctor` can say why"), "{text}");
}

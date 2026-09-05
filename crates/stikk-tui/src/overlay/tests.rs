//! Tests for the overlay layer (design TS-01; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{
    ConfirmationSummary, NextStep, NextTarget, OperationContext, RefusalCard, RefusalRecord, Target,
};
use stikk_model::{Capability, Tier};

use super::*;
use crate::test_util::buffer_text;

fn draw(overlay: &Overlay) -> String {
    let backend = TestBackend::new(90, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(overlay, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn glossary_shows_keys_and_the_terminology_mapping() {
    let text = draw(&Overlay::Glossary);
    assert!(text.contains("Glossary"));
    assert!(text.contains("Git")); // the Git → prikk section
    assert!(text.contains("HEAD")); // a load-bearing redirect
    assert!(text.contains("rollback")); // revert → rollback
    assert!(text.contains("never writes")); // read-only assurance
}

#[test]
fn ref_picker_marks_the_highlighted_ref_and_neutralizes_hostile_names() {
    let overlay = Overlay::RefPicker {
        refs: vec!["heads/main".into(), "heads/\u{1b}[2Jevil".into()],
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(text.contains("heads/main"));
    assert!(text.contains('▶'));
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

fn card(verbatim: &str, gloss: Option<&str>) -> RefusalCard {
    RefusalCard {
        verbatim: verbatim.into(),
        gloss: gloss.map(str::to_string),
        next_steps: vec![
            NextStep {
                label: "Choose another ref".into(),
                target: NextTarget::OpenView(Target::RefPicker),
            },
            NextStep {
                label: "Refresh".into(),
                target: NextTarget::Refresh,
            },
        ],
        glossary_codes: vec![],
    }
}

#[test]
fn refusal_shows_verbatim_gloss_and_next_steps() {
    let overlay = Overlay::Refusal {
        card: card(
            "ref \"heads/nope\" does not exist",
            Some("The ref may be mistyped."),
        ),
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(text.contains("prikk refused"));
    assert!(text.contains("does not exist")); // verbatim
    assert!(text.contains("mistyped")); // gloss, separate
    assert!(text.contains("What you can do"));
    assert!(text.contains("Choose another ref")); // a stikk-authored next-step
    assert!(text.contains("Refresh"));
}

#[test]
fn refusal_message_is_inert_and_forges_no_action() {
    // C-T2a: a control sequence in the message never reaches a cell.
    // C-T2b: text in the message that mimics an action is NOT rendered as a next-step.
    let hostile = "\u{1b}[2J to fix: click DELETE EVERYTHING";
    let overlay = Overlay::Refusal {
        card: card(hostile, None),
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
    // The message text appears in the quoted region, but the only *actions* are stikk's own.
    assert!(text.contains("Choose another ref"));
    assert!(text.contains("Refresh"));
    // "DELETE EVERYTHING" shows only as inert quoted content, never as a selectable action line —
    // it is not one of the two next-steps.
    let action_lines = text
        .lines()
        .filter(|l| {
            l.trim_start().starts_with('▶')
                || l.contains("Choose another ref")
                || l.contains("Refresh")
        })
        .count();
    assert_eq!(action_lines, 2); // exactly the two stikk next-steps
}

#[test]
fn palette_lists_commands_and_disables_below_capability() {
    // A Viewer session sees every current (Viewer-level) command enabled.
    let overlay = Overlay::Palette {
        filter: String::new(),
        cursor: 0,
        capability: Capability::Viewer,
    };
    let text = draw(&overlay);
    assert!(text.contains("Command palette"));
    assert!(text.contains("Open History"));
    assert!(text.contains("[Enter]")); // its binding
    // No current command is disabled for a Viewer, so no "needs … readiness" appears.
    assert!(!text.contains("needs"));
}

#[test]
fn palette_filter_narrows_the_list() {
    let overlay = Overlay::Palette {
        filter: "history".into(),
        cursor: 0,
        capability: Capability::Viewer,
    };
    let text = draw(&overlay);
    assert!(text.contains("Open History"));
    assert!(!text.contains("Quit")); // filtered out
}

#[test]
fn refusals_list_shows_remembered_messages() {
    let overlay = Overlay::Refusals {
        records: vec![RefusalRecord {
            verbatim: "ref does not exist".into(),
            class: "refusal",
            operation: OperationContext::LoadHistory,
            seq: 0,
        }],
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(text.contains("Recent refusals"));
    assert!(text.contains("ref does not exist"));
}

fn summary(target_ids: Vec<&str>, target_name: Option<&str>) -> ConfirmationSummary {
    ConfirmationSummary {
        operation: "Commit worktree changes".to_string(),
        target_ids: target_ids.into_iter().map(str::to_string).collect(),
        counts: vec![("patches", 3)],
        capability: Capability::Author,
        consequence: "Queues patches for the next seal; nothing is sealed yet.".to_string(),
        target_name: target_name.map(str::to_string),
    }
}

#[test]
fn confirmation_restates_operation_targets_counts_and_consequence() {
    let overlay = Overlay::Confirmation {
        summary: summary(vec!["heads/main"], None),
        tier: Tier::Two,
        typed: String::new(),
        error: None,
    };
    let text = draw(&overlay);
    assert!(text.contains("Confirm"));
    assert!(text.contains("Commit worktree changes"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("3 patches"));
    assert!(text.contains("Queues patches for the next seal"));
    assert!(text.contains("Enter to confirm"));
    // Tier 2/3 take a plain yes/no — no typed-input prompt.
    assert!(!text.contains("Type "));
}

#[test]
fn confirmation_tier_three_typed_shows_the_prompt_and_typed_input_so_far() {
    let overlay = Overlay::Confirmation {
        summary: summary(vec!["heads/main"], Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: "heads/ma".to_string(),
        error: None,
    };
    let text = draw(&overlay);
    assert!(text.contains("Type \"heads/main\""));
    assert!(text.contains("heads/ma")); // what has been typed so far
    assert!(!text.contains("Enter to confirm")); // that prompt is tier 2/3's, not this tier's
}

#[test]
fn confirmation_shows_an_inline_declined_error_not_a_separate_popup() {
    let overlay = Overlay::Confirmation {
        summary: summary(vec!["heads/main"], Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: "wrong".to_string(),
        error: Some("test-op was not confirmed as this tier requires".to_string()),
    };
    let text = draw(&overlay);
    assert!(text.contains("was not confirmed"));
    // Still the one overlay — the error is a line inside it, not a second title/overlay.
    assert_eq!(text.matches("Confirm").count(), 1);
}

#[test]
fn confirmation_hostile_target_id_and_target_name_render_inert() {
    // C-T4e: a ConfirmationSummary is built from prikk-authoritative values that a hostile
    // repository could still shape (a ref name, say) — neither may forge chrome nor escape the pane.
    let hostile_id = "heads/\u{1b}[2Jevil";
    let overlay = Overlay::Confirmation {
        summary: summary(vec![hostile_id], Some(hostile_id)),
        tier: Tier::ThreeTyped,
        typed: String::new(),
        error: None,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn confirmation_hostile_typed_input_also_renders_inert() {
    let overlay = Overlay::Confirmation {
        summary: summary(vec!["heads/main"], Some("heads/main")),
        tier: Tier::ThreeTyped,
        typed: "\u{1b}[2Jpasted".to_string(),
        error: None,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
}

//! Tests for the overlay layer (design TS-01; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{
    ConfirmationSummary, NextStep, NextTarget, OperationContext, Presentation, RefusalCard,
    RefusalRecord, Target, present,
};
use stikk_model::{Capability, StikkError, Tier};

use super::*;
use crate::test_util::buffer_text;

fn draw(overlay: &Overlay) -> String {
    // 60 rows: review v1 C1 wraps a long note across two rows instead of clipping it, which needs more
    // vertical room than the flat line-per-term layout this height was originally sized for. The
    // Glossary overlay has no scroll interaction (a named gap, not built here), so a term past whatever
    // height the box gets is simply invisible — this needs to be tall enough to keep every terminology
    // entry visible in this fixed-size render, not merely tall enough for the *previous* layout's needs.
    let backend = TestBackend::new(90, 60);
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
}

/// Review v1, C1: a literal `{:<22}` pad broke the moment a term ran 24 chars ("checkout / switch
/// branch", "merge conflict / resolve"), and with no `.wrap(...)` the notes — the section's entire
/// teaching content — were truncated at the box edge, four of them mid-word. Both defects were visible
/// in the review request's own before/after renders and went unreported because nothing asserted past
/// the first few characters of either the git/prikk pair or a note. Asserting the note's *tail* is the
/// point: a head-only assertion passes on truncated content too, which is exactly why C1 shipped.
#[test]
fn glossary_pads_the_longest_term_and_wraps_a_long_note_to_its_end() {
    let text = draw(&Overlay::Glossary);
    // The git/prikk columns must not run together for a 24-char term (the old fixed pad was 22).
    assert!(text.contains("checkout / switch branch"));
    assert!(!text.contains("branchfocused"));
    // The note's tail, not its head: proof the whole sentence reached a cell, not just its opening.
    assert!(text.contains("plan-first checkout."));
}

/// RFC 018 F1: the panel claimed stikk "never writes your repository" four lines above the `C`/`S`
/// keybindings that commit and seal — the false claim and its own refutation, in one screen. This pins
/// the *shape* of that defect rather than one string: if the key list ever again shows a mutating
/// keybinding, the header must not carry an absolute never-writes claim beside it. A future line could
/// say something else false; this cannot catch that. It can catch this exact defect recurring, and it
/// is the closest thing to pinning a claim this project has (RFC 018's "nothing pins a claim").
#[test]
fn glossary_never_writes_claim_cannot_stand_beside_mutating_keys() {
    let text = draw(&Overlay::Glossary);
    let shows_mutating_keys =
        text.contains("commit worktree changes") && text.contains("seal the active WAL");
    assert!(
        shows_mutating_keys,
        "glossary should list C/S — if it doesn't, this test is vacuous"
    );
    assert!(!text.to_lowercase().contains("never writes"));
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

/// Render `overlay` at a plain 80×24 `TestBackend` — the width review v2 measured the C1 failure at,
/// and the fixed height every "is the next-step still on screen" assertion in this file now uses
/// (review v2, C2: a render test needs at least one assertion about the **end** of the content, not
/// only its middle, since fragment matching alone cannot tell you what fell off the bottom).
fn draw_80x24(overlay: &Overlay) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(overlay, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// The acceptance-critical render test review v2 (C1) asked for: v1's fix was correct in the data
/// model — `present()` already produced prikk's verbatim `detail` and stikk's own separate `gloss` —
/// but nothing checked what actually reached a cell, and `InlineGuidance`'s one-row, non-wrapping
/// banner cut the gloss off entirely at every realistic terminal width. This drives the *real*
/// `present()` (not a hand-built `RefusalCard`) and checks the gloss's key sentence in short fragments
/// rather than as one long contiguous string, since the fix makes it wrap across several rows (and
/// `buffer_text` joins rows with `\n`, so a long contiguous match would fail for the same reason
/// `seal_consent_shows_the_copy_and_the_unacknowledged_mark` checks fragments, not the whole string).
#[test]
fn trust_refusal_gloss_is_reachable_at_80_columns() {
    let err = StikkError::NotReady {
        detail:
            "invalid signature: maintainer signer key id different-maintainer is not trusted by \
                 policy"
                .to_string(),
    };
    let card = match present(&err, OperationContext::Other) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    let text = draw_80x24(&Overlay::Refusal { card, cursor: 0 });

    // prikk's own words, verbatim.
    assert!(text.contains("maintainer signer key id"));
    assert!(text.contains("is not trusted by policy"));
    // The key sentence RFC 016 §9 exists to deliver — reachable, in fragments a wrap point cannot
    // plausibly split (each is a single word or a short, tightly-bound phrase).
    assert!(text.contains("adopted"));
    assert!(text.contains("object trust"));
    assert!(text.contains("cannot verify"));
    assert!(text.contains("adoption"));
    assert!(text.contains("unknown"));
    // Attribution stays distinguishable: prikk's line is quoted, stikk's is separate prose below it —
    // not flattened into one run the way the v1 banner joined them with em-dashes.
    assert!(text.contains("prikk reported"));
    // C2, the acceptance-critical assertion this test was missing: the gloss being reachable is not
    // the same claim as the next-step being reachable — the one thing this card exists to make
    // actionable must also survive being on screen at all, at the very bottom of the content.
    assert!(text.contains("What you can do"));
    assert!(text.contains("Refresh"));
}

/// Review v2 C2's named regression: this exact shape has been shipping since 0.3.0. `SCHEMA_SKEW`'s
/// own real message plus its one next-step (`Upgrade prikk`) — the schema-skew and full-queue fixtures
/// below match `present/tests.rs`'s own captured messages, so this is the same content that crate's
/// tests already exercise for the *model*, now exercised for the *render*.
#[test]
fn schema_skew_next_step_is_reachable_at_80_columns() {
    let err = stikk_model::StikkError::Refusal {
        message:
            "integrity error: format-2 patch does not accept envelope schema 3 (accepted: [1, 2])"
                .to_string(),
    };
    let card = match present(&err, OperationContext::Orient) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    let text = draw_80x24(&Overlay::Refusal { card, cursor: 0 });
    assert!(text.contains("does not accept envelope schema"));
    assert!(text.contains("newer prikk")); // the gloss
    assert!(text.contains("What you can do"));
    assert!(text.contains("Upgrade prikk"));
}

/// Review v2 C2: the full-queue card already had both next-steps visible before this fix (its gloss
/// happens to be short enough that the old `+6` heuristic did not starve it) — kept as the "this was
/// already fine, and the fix must not regress it" control.
#[test]
fn full_queue_next_steps_are_reachable_at_80_columns() {
    let err = stikk_model::StikkError::Refusal {
        message:
            "lock conflict: active WAL has 1 queued patches, at or above the configured limit \
                  (1); run `prikk seal` before committing again"
                .to_string(),
    };
    let card = match present(&err, OperationContext::Commit) {
        Presentation::RefusalOverlay(card) => card,
        other => panic!("expected RefusalOverlay, got {other:?}"),
    };
    let text = draw_80x24(&Overlay::Refusal { card, cursor: 0 });
    assert!(text.contains("at or above the configured limit"));
    assert!(text.contains("Nothing is locked")); // the gloss
    assert!(text.contains("What you can do"));
    assert!(text.contains("Seal the active WAL"));
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
fn stale_names_the_operation_and_never_claims_prikk_reported_it() {
    // design-review C1 (RFC 013 v1): stikk's own words must never render under a "prikk reported"
    // label — this is the regression test for that finding.
    let overlay = Overlay::Stale {
        operation: "commit".into(),
        gloss: "Another writer moved something in this repository between your preview and now."
            .into(),
        next_steps: vec![NextStep {
            label: "Preview again".into(),
            target: NextTarget::Refresh,
        }],
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(text.contains("stikk stopped"));
    assert!(!text.contains("prikk reported"));
    assert!(!text.contains("prikk refused"));
    assert!(text.contains("commit"));
    assert!(text.contains("Preview again"));
}

/// Review v3's "before you push" item: `render_stale` had the same pre-C2 sizing pattern as
/// `render_refusal` (a single `Paragraph` sized by `lines.len() + 6`, a logical-line count, not a
/// wrapped-row count) — it happened to survive at ordinary sizes only because this card's gloss is
/// always a fixed, stikk-authored constant, never lengthened by anything prikk sends. Confirmed
/// clipped at 80×12 before this fix (the review's own measurement); this is the regression test for it,
/// using the same two-region layout and the same fixed-size-backend pattern
/// `trust_refusal_gloss_is_reachable_at_80_columns` established.
#[test]
fn stale_next_step_is_reachable_at_80x12() {
    let overlay = Overlay::Stale {
        operation: "commit".into(),
        gloss: "Another writer moved something in this repository between your preview and now."
            .into(),
        next_steps: vec![NextStep {
            label: "Preview again".into(),
            target: NextTarget::Refresh,
        }],
        cursor: 0,
    };
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(&overlay, &Palette::default(), f, f.area()))
        .unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("stikk stopped"));
    assert!(text.contains("What you can do"));
    assert!(text.contains("Preview again"));
}

#[test]
fn an_ordinary_refusal_still_says_prikk_reported() {
    // The other half of the C1 regression: splitting `Stale` out must not have broken the label an
    // actual prikk refusal still deserves.
    let overlay = Overlay::Refusal {
        card: card("ref \"heads/nope\" does not exist", None),
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(text.contains("prikk reported"));
}

fn viewer_readiness() -> stikk_model::Readiness {
    stikk_model::Readiness {
        author_ready: false,
        maintainer_readiness: stikk_model::MaintainerReadiness::NotReady,
        read_only: false,
    }
}

#[test]
fn palette_lists_every_tier_one_command_enabled_for_a_viewer() {
    let overlay = Overlay::Palette {
        filter: String::new(),
        cursor: 0,
        readiness: viewer_readiness(),
    };
    let text = draw(&overlay);
    assert!(text.contains("Command palette"));
    assert!(text.contains("Open History"));
    assert!(text.contains("[Enter]")); // its binding
    // Every tier-one command is unconditionally free — none of their lines carries a reason.
    for name in ["Open History", "Open Changes", "Glossary & Help", "Quit"] {
        let line = text
            .lines()
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("expected a line for {name}"));
        assert!(
            !line.contains("needs"),
            "{name} should not be disabled: {line:?}"
        );
    }
}

#[test]
fn palette_disables_commit_for_a_viewer_with_the_capability_gate_reason() {
    // RFC 014 §6: the palette and `confirm` must agree — this is the regression test for that.
    let overlay = Overlay::Palette {
        filter: "commit".into(),
        cursor: 0,
        readiness: viewer_readiness(),
    };
    let text = draw(&overlay);
    assert!(text.contains("Commit worktree changes"));
    assert!(text.contains("needs AUTHOR signing readiness"));
}

#[test]
fn palette_disables_commit_under_read_only_even_with_author_keys_present() {
    let overlay = Overlay::Palette {
        filter: "commit".into(),
        cursor: 0,
        readiness: stikk_model::Readiness {
            author_ready: true,
            maintainer_readiness: stikk_model::MaintainerReadiness::NotReady,
            read_only: true,
        },
    };
    let text = draw(&overlay);
    assert!(text.contains("read-only"));
}

#[test]
fn palette_filter_narrows_the_list() {
    let overlay = Overlay::Palette {
        filter: "history".into(),
        cursor: 0,
        readiness: viewer_readiness(),
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

#[test]
fn commit_message_below_0_32_says_the_message_is_not_persisted() {
    let overlay = Overlay::CommitMessage {
        reff: "heads/main".to_string(),
        typed: "fix the thing".to_string(),
        messages_persist: false,
    };
    let text = draw(&overlay);
    assert!(text.contains("Commit message"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("required"));
    assert!(text.contains("does not yet persist"));
    assert!(text.contains("fix the thing"));
}

#[test]
fn commit_message_at_0_32_says_the_message_is_stored() {
    // RFC 015 F2: `UD-01` retires at 0.32 — the copy must not keep asserting the old claim.
    let overlay = Overlay::CommitMessage {
        reff: "heads/main".to_string(),
        typed: "fix the thing".to_string(),
        messages_persist: true,
    };
    let text = draw(&overlay);
    assert!(text.contains("is stored"));
    assert!(!text.contains("does not yet persist"));
}

#[test]
fn commit_message_hostile_ref_and_typed_text_render_inert() {
    let overlay = Overlay::CommitMessage {
        reff: "heads/\u{1b}[2Jevil".to_string(),
        typed: "\u{1b}[2Jpasted".to_string(),
        messages_persist: false,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

fn commit_result(notes: Vec<&str>) -> stikk_prikk::CommitResult {
    stikk_prikk::CommitResult {
        baseline_ref: "heads/main".to_string(),
        patch_id: "a".repeat(64),
        wal_sequence: 1,
        operations: 1,
        referenced_blobs: 1,
        text_edits: 0,
        changes: vec![stikk_prikk::CommitChange {
            operation: "create-file".to_string(),
            path: "a.txt".to_string(),
        }],
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

#[test]
fn commit_result_shows_the_patch_id_counts_changes_and_every_note_verbatim() {
    let overlay = Overlay::CommitResult {
        result: commit_result(vec![
            "note: multi-operation text diff minimization … remain later increments",
            "note: the message is validated but not stored -- it will not appear in `prikk log`",
        ]),
    };
    let text = draw(&overlay);
    assert!(text.contains("Commit recorded"));
    assert!(text.contains(&"a".repeat(64)));
    assert!(text.contains("create-file"));
    assert!(text.contains("a.txt"));
    assert!(text.contains("multi-operation text diff minimization"));
    assert!(text.contains("message is validated but not stored"));
}

#[test]
fn commit_result_with_no_message_fate_note_shows_only_the_note_present() {
    // RFC 014's own found-a-contradiction case: a real prikk 0.32.0 prints only one note.
    let overlay = Overlay::CommitResult {
        result: commit_result(vec!["note: multi-operation text diff minimization"]),
    };
    let text = draw(&overlay);
    assert!(text.contains("multi-operation text diff minimization"));
    assert!(!text.contains("message is validated but not stored"));
}

#[test]
fn commit_result_hostile_patch_id_and_path_render_inert() {
    let mut result = commit_result(vec![]);
    result.patch_id = "\u{1b}[2Jevil".to_string();
    result.changes[0].path = "\u{1b}[2Jpwned.txt".to_string();
    let overlay = Overlay::CommitResult { result };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn seal_consent_shows_the_copy_and_the_unacknowledged_mark() {
    let overlay = Overlay::SealConsent {
        reff: "heads/main".to_string(),
        acknowledged: false,
    };
    let text = draw(&overlay);
    assert!(text.contains("Before you seal"));
    // The full copy wraps across several rows in the fixed-width render, so `buffer_text`'s row-joined
    // output does not contain it as one contiguous substring — checked in fragments instead, the same
    // way other long-text render tests here check distinguishing substrings, not exact whole strings.
    assert!(text.contains("independent audit"));
    assert!(text.contains("cannot be undone"));
    assert!(text.contains("[ ]"));
    assert!(!text.contains("[x]"));
    // RFC 016 §8: Enter must not read as ready to go while unacknowledged.
    assert!(text.contains("Space to acknowledge"));
    assert!(!text.contains("Enter to seal"));
}

#[test]
fn seal_consent_shows_the_acknowledged_mark_and_enter_hint() {
    let overlay = Overlay::SealConsent {
        reff: "heads/main".to_string(),
        acknowledged: true,
    };
    let text = draw(&overlay);
    assert!(text.contains("[x]"));
    assert!(!text.contains("[ ]"));
    assert!(text.contains("Enter to seal"));
}

#[test]
fn seal_consent_hostile_ref_renders_inert() {
    let overlay = Overlay::SealConsent {
        reff: "heads/\u{1b}[2Jevil".to_string(),
        acknowledged: false,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

fn seal_result(notes: Vec<&str>) -> stikk_prikk::SealResult {
    stikk_prikk::SealResult {
        patches: 2,
        block_id: "b".repeat(64),
        reff: "heads/main".to_string(),
        ref_state: "c".repeat(64),
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

#[test]
fn seal_result_shows_the_block_id_ref_state_and_every_note_verbatim() {
    let overlay = Overlay::SealResult {
        result: seal_result(vec!["note: audit plugins remain later PRs"]),
    };
    let text = draw(&overlay);
    assert!(text.contains("Sealed"));
    assert!(text.contains(&"b".repeat(64)));
    assert!(text.contains(&"c".repeat(64)));
    assert!(text.contains("patches 2"));
    assert!(text.contains("audit plugins remain later PRs"));
}

#[test]
fn seal_result_hostile_block_id_and_ref_render_inert() {
    let mut result = seal_result(vec![]);
    result.block_id = "\u{1b}[2Jevil".to_string();
    result.reff = "heads/\u{1b}[2Jpwned".to_string();
    let overlay = Overlay::SealResult { result };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

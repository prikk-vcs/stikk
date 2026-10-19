//! Tests for the Merge plan view (design TS-01; RFC 047 decision 4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_prikk::MergeOutcome;

use super::*;
use crate::test_util::buffer_text;

/// prikk 0.53.0's own plan for scenario 1, from the §1 re-run
/// (`.git-exclude/reports/047-merge/measure047-0.53.txt`), ids shortened for the fixture.
const CONFLUENT: &[&str] = &[
    "merge plan",
    "baseline block: 88f34150e099",
    "left selector: ref heads/main",
    "left target block: 6d17f5f225b7",
    "left operations: 1",
    "right selector: ref heads/feature",
    "right target block: d967f47871e7",
    "right operations: 1",
    "status: ConfluentSubset",
    "evidence outcome: Confluent",
    "reason: proven_confluent",
    "action: review the evidence, then run 'prikk merge' to execute",
    "items: 1 displayed of 1",
    "",
    "report:",
    "  outcome: Confluent",
    "  reason: proven_confluent",
    "  phase: final-state-comparison",
    "note: read-only plan; no merge commit, ref update, WAL write, object write, or worktree \
     change was performed",
];

/// Scenario 3's conflicting plan, from the same run.
const CONFLICTING: &[&str] = &[
    "merge plan",
    "baseline block: 2237a4316d32",
    "left selector: ref heads/main",
    "left target block: 5b59d8a49c98",
    "right selector: ref heads/feature",
    "right target block: 3af8f309a4be",
    "status: BlockedConflict",
    "evidence outcome: Conflict",
    "reason: pair_conflict",
    "action: inspect evidence; conflict resolution is not implemented",
    "items: 1 displayed of 1",
    "",
    "cross:",
    "  left[0] op_seq=1 EditText",
    "  right[0] op_seq=1 EditText",
    "  outcome: Conflict",
    "  reason: pair_conflict",
    "  phase: classification",
    "note: read-only plan; no merge commit, ref update, WAL write, object write, or worktree \
     change was performed",
];

fn view(lines: &[&str], outcome: MergeOutcome) -> MergePlanView {
    MergePlanView {
        from: "heads/feature".into(),
        into: "heads/main".into(),
        baseline: "88f34150".into(),
        lines: lines.iter().map(ToString::to_string).collect(),
        outcome,
    }
}

fn draw_at(view: &MergePlanView, width: u16, height: u16, offset: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(offset);
    terminal
        .draw(|f| render(view, &offset, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// **Capture: a confluent plan at 80×24.**
#[test]
fn capture_a_confluent_plan_at_80x24() {
    let text = draw_at(&view(CONFLUENT, MergeOutcome::Confluent), 80, 24, 0);
    println!("{text}");
    // Both names, with the role each key gave it.
    assert!(text.contains("heads/feature (marked)"), "{text}");
    assert!(text.contains("heads/main (selected)"), "{text}");
    // stikk's baseline, named as stikk's own choice.
    assert!(text.contains("88f34150"), "{text}");
    assert!(text.contains("stikk found"), "{text}");
    // prikk's own words, unchanged.
    assert!(text.contains("evidence outcome: Confluent"), "{text}");
    assert!(text.contains("status: ConfluentSubset"), "{text}");
    // **Nothing about a key that does nothing** -- the merge is Handoff B.
    assert!(!text.contains("Enter"), "{text}");
}

/// **Capture: a conflicting plan at 80×24** -- shown exactly as fully as a confluent one.
#[test]
fn capture_a_conflicting_plan_at_80x24() {
    let text = draw_at(&view(CONFLICTING, MergeOutcome::Conflict), 80, 24, 0);
    println!("{text}");
    assert!(text.contains("evidence outcome: Conflict"), "{text}");
    assert!(text.contains("status: BlockedConflict"), "{text}");
    // **stikk adds no verdict of its own.** prikk's refusal to merge these two is prikk's
    // sentence; a "stikk cannot merge this" line on top would be stikk answering a question
    // prikk already answered, in worse words.
    assert!(!text.to_lowercase().contains("stikk cannot"), "{text}");
}

/// The scroll indicator appears only when there is something below, and the last row of the
/// plan is reachable (RFC 024: wrap, never clip).
#[test]
fn a_long_plan_says_it_scrolls_and_its_last_line_is_reachable() {
    let view = view(CONFLUENT, MergeOutcome::Confluent);
    let first = draw_at(&view, 80, 24, 0);
    assert!(first.contains("lines 1"), "{first}");
    assert!(
        !first.contains("worktree change was performed"),
        "the note is below the fold at 24 rows: {first}"
    );
    let end = draw_at(&view, 80, 24, u16::MAX);
    assert!(end.contains("change was performed"), "{end}");

    // Tall enough for everything: no indicator, nothing cut.
    let whole = draw_at(&view, 80, 40, 0);
    assert!(!whole.contains("↑/↓"), "{whole}");
    assert!(whole.contains("change was performed"), "{whole}");
}

/// **prikk's lines wrap, they do not clip** (RFC 024), and the wrap is a hard one: the plan is
/// laid out in `label: value` rows, so re-flowing it on whitespace would lose prikk's shape.
#[test]
fn a_line_too_long_for_the_panel_wraps_whole() {
    let long = format!("note: {}", "x".repeat(200));
    let view = view(&[&long], MergeOutcome::Confluent);
    let text = draw_at(&view, 80, 24, 0);
    let xs: usize = text.matches('x').count();
    assert_eq!(xs, 200, "every character is drawn somewhere: {text}");
}

/// **A control character in prikk's output is made inert** (`C-T2a`, `T-T4`): the plan is prose
/// stikk shows, and nothing in it may move the cursor or colour the terminal.
#[test]
fn prikks_lines_are_rendered_inert() {
    let view = view(&["reason: \u{1b}[31mred\u{1b}[0m"], MergeOutcome::Conflict);
    let text = draw_at(&view, 80, 24, 0);
    assert!(!text.contains('\u{1b}'), "{text}");
    assert!(text.contains("red"), "{text}");
}

/// A blank line in prikk's plan stays a blank line: it separates the summary from the report,
/// and collapsing it would run two blocks together.
#[test]
fn a_blank_line_in_the_plan_is_kept() {
    let text = draw_at(&view(CONFLUENT, MergeOutcome::Confluent), 80, 40, 0);
    let rows: Vec<&str> = text.lines().collect();
    let items = rows
        .iter()
        .position(|row| row.contains("items: 1 displayed of 1"))
        .expect("the items row");
    let report = rows
        .iter()
        .position(|row| row.contains("report:"))
        .expect("the report row");
    assert_eq!(report, items + 2, "one blank row between them: {text}");
}

/// A ref name that needs escaping is quoted under the identifier rule (RFC 036), so two
/// different branches never look alike in the heading.
#[test]
fn an_awkward_branch_name_is_quoted_in_the_heading() {
    let mut awkward = view(CONFLUENT, MergeOutcome::Confluent);
    awkward.from = "heads/two  spaces".into();
    let text = draw_at(&awkward, 80, 24, 0);
    assert!(text.contains("\\u0020"), "{text}");
}

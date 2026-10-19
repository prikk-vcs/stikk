//! Tests for the baseline finder (RFC 047 decision 3).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{BlockRow, History};

use super::*;

fn block(id: &str, seq: u64, kind: &str, parents: u64) -> BlockRow {
    BlockRow {
        block_id: id.to_string(),
        ref_state_id: format!("rs-{id}"),
        update_seq: seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents,
        patches: 1,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: None,
    }
}

/// Newest-first, the order `prikk log` prints and [`History`] keeps.
fn lineage(reff: &str, blocks: Vec<BlockRow>) -> History {
    History {
        reff: reff.to_string(),
        blocks,
    }
}

/// The two measured lineages of `measure047.sh` scenario 1, newest-first, as 0.53.0 printed them
/// — `main` after the merge, `feature` the branch merged in. The baseline prikk accepted for that
/// merge was `88f34150`, the block both chains share.
fn measured_feature() -> History {
    lineage(
        "heads/feature",
        vec![
            block("01163541", 3, "Merge", 2),
            block("d967f478", 2, "Normal", 1),
            block("88f34150", 1, "Root", 0),
        ],
    )
}

fn measured_main() -> History {
    lineage(
        "heads/main",
        vec![
            block("9edf2f76", 3, "Normal", 1),
            block("2afe7bc4", 2, "Merge", 2),
            block("6ddf9541", 1, "Root", 0),
        ],
    )
}

#[test]
fn the_latest_shared_block_is_the_baseline() {
    let into = lineage(
        "heads/main",
        vec![
            block("mm2", 4, "Normal", 1),
            block("mm1", 3, "Normal", 1),
            block("base", 2, "Normal", 1),
            block("root", 1, "Root", 0),
        ],
    );
    let from = lineage(
        "heads/feature",
        vec![
            block("ff1", 5, "Normal", 1),
            block("base", 2, "Normal", 1),
            block("root", 1, "Root", 0),
        ],
    );
    assert_eq!(
        merge_baseline(&into, &from, 50),
        MergeBaseline::Found("base".into())
    );
}

#[test]
fn the_latest_is_taken_and_not_the_oldest_shared_block() {
    // `root` is shared too, and is the wrong answer: a baseline older than the true merge base
    // turned a confluent pair into a `Conflict` when measured (F2).
    let into = lineage(
        "heads/main",
        vec![
            block("mm1", 3, "Normal", 1),
            block("base", 2, "Normal", 1),
            block("root", 1, "Root", 0),
        ],
    );
    let from = lineage(
        "heads/feature",
        vec![
            block("ff1", 4, "Normal", 1),
            block("base", 2, "Normal", 1),
            block("root", 1, "Root", 0),
        ],
    );
    let MergeBaseline::Found(id) = merge_baseline(&into, &from, 50) else {
        panic!("expected a baseline")
    };
    assert_eq!(id, "base");
    assert_ne!(id, "root");
}

#[test]
fn a_tip_already_in_the_other_lineage_is_itself_the_baseline() {
    // `from` has nothing `into` does not: the latest shared block is `from`'s own tip.
    let into = lineage(
        "heads/main",
        vec![
            block("mm1", 3, "Normal", 1),
            block("ff1", 2, "Normal", 1),
            block("root", 1, "Root", 0),
        ],
    );
    let from = lineage(
        "heads/feature",
        vec![block("ff1", 2, "Normal", 1), block("root", 1, "Root", 0)],
    );
    assert_eq!(
        merge_baseline(&into, &from, 50),
        MergeBaseline::Found("ff1".into())
    );
}

#[test]
fn a_merge_above_the_shared_block_on_either_side_declines() {
    let shared = vec![block("base", 2, "Normal", 1), block("root", 1, "Root", 0)];
    let plain = {
        let mut blocks = vec![block("mm1", 3, "Normal", 1)];
        blocks.extend(shared.clone());
        lineage("heads/main", blocks)
    };
    let merged = {
        let mut blocks = vec![block("ff1", 4, "Merge", 2)];
        blocks.extend(shared.clone());
        lineage("heads/feature", blocks)
    };
    let also_merged = {
        let mut blocks = vec![block("gg1", 5, "Merge", 2)];
        blocks.extend(shared.clone());
        lineage("heads/other", blocks)
    };
    for (into, from, side) in [
        (&plain, &merged, "from"),
        (&merged, &plain, "into"),
        (&merged, &also_merged, "both"),
    ] {
        let MergeBaseline::None(why) = merge_baseline(into, from, 50) else {
            panic!("a merge above the shared block on {side} should decline")
        };
        assert_eq!(why, MERGED_ALREADY);
    }
}

#[test]
fn a_merge_at_or_below_the_shared_block_is_fine() {
    // Everything above the shared block is linear on both sides, which is the whole condition.
    let shared = vec![
        block("base", 3, "Merge", 2),
        block("old", 2, "Normal", 1),
        block("root", 1, "Root", 0),
    ];
    let mut into_blocks = vec![block("mm1", 4, "Normal", 1)];
    into_blocks.extend(shared.clone());
    let mut from_blocks = vec![block("ff1", 5, "Normal", 1)];
    from_blocks.extend(shared);
    assert_eq!(
        merge_baseline(
            &lineage("heads/main", into_blocks),
            &lineage("heads/feature", from_blocks),
            50
        ),
        MergeBaseline::Found("base".into())
    );
}

#[test]
fn disjoint_lineages_read_whole_carry_their_own_reason() {
    let into = lineage(
        "heads/main",
        vec![block("mm1", 2, "Normal", 1), block("mr", 1, "Root", 0)],
    );
    let from = lineage(
        "heads/other",
        vec![block("ff1", 2, "Normal", 1), block("fr", 1, "Root", 0)],
    );
    assert_eq!(
        merge_baseline(&into, &from, 50),
        MergeBaseline::None(NO_SHARED_BLOCK.to_string())
    );
}

#[test]
fn a_truncated_read_is_not_called_disjoint() {
    // The same disjoint pair, with a limit the lineages reached: stikk has not proved anything.
    let into = lineage(
        "heads/main",
        vec![block("mm1", 2, "Normal", 1), block("mr", 1, "Root", 0)],
    );
    let from = lineage(
        "heads/other",
        vec![block("ff1", 2, "Normal", 1), block("fr", 1, "Root", 0)],
    );
    assert_eq!(
        merge_baseline(&into, &from, 2),
        MergeBaseline::None(LINEAGE_TRUNCATED.to_string())
    );
    // One full side is enough to doubt, even when the other is short.
    let short = lineage("heads/other", vec![block("ff1", 2, "Normal", 1)]);
    assert_eq!(
        merge_baseline(&into, &short, 2),
        MergeBaseline::None(LINEAGE_TRUNCATED.to_string())
    );
}

#[test]
fn a_full_read_that_found_a_baseline_is_not_truncated() {
    // Truncation only matters when nothing was found: the shared block was found before the end.
    let into = lineage(
        "heads/main",
        vec![block("mm1", 3, "Normal", 1), block("base", 2, "Normal", 1)],
    );
    let from = lineage(
        "heads/feature",
        vec![block("ff1", 4, "Normal", 1), block("base", 2, "Normal", 1)],
    );
    assert_eq!(
        merge_baseline(&into, &from, 2),
        MergeBaseline::Found("base".into())
    );
}

#[test]
fn an_empty_lineage_shares_nothing() {
    let empty = lineage("heads/new", Vec::new());
    let into = lineage("heads/main", vec![block("mm1", 1, "Root", 0)]);
    assert_eq!(
        merge_baseline(&into, &empty, 50),
        MergeBaseline::None(NO_SHARED_BLOCK.to_string())
    );
    assert_eq!(
        merge_baseline(&empty, &into, 50),
        MergeBaseline::None(NO_SHARED_BLOCK.to_string())
    );
}

#[test]
fn the_two_merge_signals_agree() {
    // Measured at 0.53.0 on a real merged lineage: `kind` and `parent_count` say the same thing.
    // The finder checks both, so this pins the shape the measurement showed rather than the
    // finder's own behaviour.
    for lineage in [measured_feature(), measured_main()] {
        for block in &lineage.blocks {
            assert_eq!(
                block.kind == "Merge",
                block.parents > 1,
                "{} in {}",
                block.block_id,
                lineage.reff
            );
        }
    }
}

#[test]
fn either_signal_alone_declines() {
    // If the two ever disagree, the block is one stikk does not understand -- and that is not a
    // reason to merge across it.
    let shared = vec![block("base", 1, "Root", 0)];
    for odd in [block("x", 2, "Merge", 1), block("x", 2, "Normal", 2)] {
        let mut blocks = vec![odd];
        blocks.extend(shared.clone());
        let MergeBaseline::None(why) = merge_baseline(
            &lineage("heads/main", blocks),
            &lineage("heads/feature", shared.clone()),
            50,
        ) else {
            panic!("a block with one merge signal should decline")
        };
        assert_eq!(why, MERGED_ALREADY);
    }
}

#[test]
fn the_measured_scenario_one_lineages_decline_after_their_merge() {
    // Both measured lineages carry a `Merge` block and share nothing -- scenario 1's repository
    // after its merge. stikk declines, and does so for the reason it can prove.
    assert_eq!(
        merge_baseline(&measured_main(), &measured_feature(), 50),
        MergeBaseline::None(NO_SHARED_BLOCK.to_string())
    );
}

#[test]
fn the_band_reason_names_the_minor_and_the_floor() {
    let why = merge_unavailable(41);
    assert!(why.contains("0.42"), "{why}");
    assert!(why.contains("0.41"), "{why}");
    assert_eq!(MERGE_FROM_MINOR, 42);
}

// ---------------------------------------------------------------------------------------------
// The view (decision 4): the baseline is found first, and prikk is asked only when it is truthful
// ---------------------------------------------------------------------------------------------

use std::path::Path;

use stikk_prikk::{MergeOutcome, NullBackend};

/// Two lineages that share `88f34150` with nothing merged above it -- the shape of scenario 1
/// *before* its merge, which is when a merge is actually offered.
fn sharing_backend() -> NullBackend {
    NullBackend::supported()
        .with_history_of(
            "heads/main",
            lineage(
                "heads/main",
                vec![
                    block("6d17f5f2", 3, "Normal", 1),
                    block("88f34150", 1, "Root", 0),
                ],
            ),
        )
        .with_history_of(
            "heads/feature",
            lineage(
                "heads/feature",
                vec![
                    block("d967f478", 2, "Normal", 1),
                    block("88f34150", 1, "Root", 0),
                ],
            ),
        )
}

const PLAN: &[&str] = &[
    "merge plan",
    "baseline block: 88f34150e099",
    "status: ConfluentSubset",
    "evidence outcome: Confluent",
];

#[test]
fn the_view_carries_both_names_the_baseline_and_the_plan_verbatim() {
    let backend = sharing_backend().with_merge_plan(
        PLAN.iter().map(ToString::to_string).collect(),
        MergeOutcome::Confluent,
    );
    let read =
        merge_plan_view(&backend, Path::new("."), "heads/main", "heads/feature", 50).unwrap();
    let MergePlanRead::Read(view) = read else {
        panic!("expected a plan")
    };
    assert_eq!(view.into, "heads/main");
    assert_eq!(view.from, "heads/feature");
    assert_eq!(view.baseline, "88f34150");
    assert_eq!(view.outcome, MergeOutcome::Confluent);
    assert_eq!(view.lines, PLAN);
}

#[test]
fn a_conflicting_plan_is_read_the_same_way() {
    // Decision 2: the outcome decides only whether a merge is *offered*; the read itself succeeds
    // for all eight words, and the lines are shown either way.
    let backend = sharing_backend().with_merge_plan(
        vec![
            "evidence outcome: Conflict".into(),
            "reason: pair_conflict".into(),
        ],
        MergeOutcome::Conflict,
    );
    let MergePlanRead::Read(view) =
        merge_plan_view(&backend, Path::new("."), "heads/main", "heads/feature", 50).unwrap()
    else {
        panic!("expected a plan")
    };
    assert_eq!(view.outcome, MergeOutcome::Conflict);
    assert!(!view.outcome.is_confluent());
    assert_eq!(view.lines.len(), 2);
}

#[test]
fn no_truthful_baseline_asks_prikk_for_no_plan() {
    let merged = lineage(
        "heads/feature",
        vec![block("ff1", 4, "Merge", 2), block("88f34150", 1, "Root", 0)],
    );
    let backend = sharing_backend()
        .with_history_of("heads/feature", merged)
        // A plan that would succeed if asked for: the decline has to come from the baseline.
        .with_merge_plan(vec!["unreachable".into()], MergeOutcome::Confluent);
    assert_eq!(
        merge_plan_view(&backend, Path::new("."), "heads/main", "heads/feature", 50).unwrap(),
        MergePlanRead::Declined(MERGED_ALREADY.to_string())
    );
}

#[test]
fn below_the_band_the_view_declines_in_stikks_voice() {
    // `NullBackend::supported()` leaves `merge_plan` at `Unavailable`, which is what the seam
    // answers below the band without spawning.
    assert_eq!(
        merge_plan_view(
            &sharing_backend(),
            Path::new("."),
            "heads/main",
            "heads/feature",
            50
        )
        .unwrap(),
        MergePlanRead::Declined(PLAN_UNAVAILABLE.to_string())
    );
}

#[test]
fn prikks_refusal_of_the_plan_travels_verbatim() {
    // `ER-02`: a baseline prikk rejects is prikk's sentence, not stikk's paraphrase.
    let backend = sharing_backend().with_merge_plan_refusal(
        "baseline Block 88f34150e099 is not an ancestor of target Block …",
    );
    let error =
        merge_plan_view(&backend, Path::new("."), "heads/main", "heads/feature", 50).unwrap_err();
    assert!(
        error.to_string().contains("is not an ancestor of target"),
        "{error}"
    );
}

#[test]
fn a_history_refusal_is_not_dressed_as_a_missing_baseline() {
    let backend = NullBackend::supported()
        .with_history_refusal("ref heads/gone does not exist")
        .with_merge_plan(vec!["unreachable".into()], MergeOutcome::Confluent);
    let error =
        merge_plan_view(&backend, Path::new("."), "heads/gone", "heads/feature", 50).unwrap_err();
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn the_lineage_limit_is_the_one_history_lists() {
    assert_eq!(MERGE_LINEAGE_LIMIT, 200);
}

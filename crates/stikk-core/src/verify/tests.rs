//! Tests for the Verify view-model (RFC 040).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{
    ActiveSessions, FailedCondition, NullBackend, StageStatus, Verdict, Verification, VerifyStage,
};

use super::*;
use crate::flatten;

fn stage(name: &str, status: StageStatus) -> VerifyStage {
    VerifyStage {
        stage: name.to_string(),
        status,
    }
}

fn report(ok: bool, conditions: Vec<(&str, &str)>, stages: Vec<VerifyStage>) -> Verification {
    Verification {
        verdict: Verdict {
            ok,
            failed_conditions: conditions
                .into_iter()
                .map(|(id, message)| FailedCondition {
                    id: id.to_string(),
                    message: message.to_string(),
                })
                .collect(),
        },
        active_sessions: Some(ActiveSessions {
            count: 1,
            verified_count: 1,
        }),
        stages,
        raw: r#"{"schema_version": "verify-report-v1"}"#.to_string(),
    }
}

#[test]
fn a_sound_report_says_so_in_stikks_own_words() {
    let view = checked(report(
        true,
        Vec::new(),
        vec![stage("objects", StageStatus::Evaluated)],
    ));
    assert_eq!(view.verdict(), "prikk reports this repository sound");
    assert!(view.conditions.is_empty());
}

#[test]
fn the_verdict_counts_prikks_own_conditions() {
    // One and several. The count comes from the list beneath it, so the two cannot drift.
    let one = checked(report(
        false,
        vec![("item-failure", "see item outcomes above")],
        Vec::new(),
    ));
    assert_eq!(one.verdict(), "prikk reports 1 problem(s)");

    let several = checked(report(
        false,
        vec![
            ("item-failure", "see item outcomes above"),
            ("commit-witness-integrity", "disagrees with the WAL"),
            ("another", "a third"),
        ],
        Vec::new(),
    ));
    assert_eq!(several.verdict(), "prikk reports 3 problem(s)");
    assert_eq!(several.conditions.len(), 3);
}

#[test]
fn a_conditions_id_is_a_name_and_its_message_is_prikks_prose() {
    let view = checked(report(
        false,
        vec![("item-failure", "see item outcomes above")],
        Vec::new(),
    ));
    let pieces = view.conditions[0].row();
    assert_eq!(pieces[0], Piece::name("item-failure"));
    // prikk's sentence arrives verbatim, as its own piece — never folded into the id's.
    assert!(
        pieces
            .iter()
            .any(|p| *p == Piece::said("see item outcomes above")),
        "{pieces:?}"
    );
    assert_eq!(flatten(&pieces), "item-failure  see item outcomes above");
}

#[test]
fn the_standing_sentence_names_the_terminal_and_is_true_either_way() {
    // **Always shown** (decision 3). It is a property of prikk's JSON, not of the verdict, and a
    // sentence that appeared only on failure would read as part of the failure.
    assert!(JSON_OMITS.contains("does not list which objects or refs failed"));
    assert!(JSON_OMITS.contains("count unverifiable signatures"));
    // It names where "above" actually is, which is what makes repeating prikk's pointer honest.
    assert!(JSON_OMITS.contains("Run `prikk verify` in a terminal"));
    // One sentence per claim, and nothing promising stikk will show them later.
    assert!(!JSON_OMITS.contains("yet"), "{JSON_OMITS}");
    assert!(!JSON_OMITS.contains("not yet"), "{JSON_OMITS}");
}

#[test]
fn every_status_prikk_can_report_keeps_its_own_word_and_its_own_field() {
    let view = checked(report(
        false,
        vec![("item-failure", "x")],
        vec![
            stage("objects", StageStatus::Evaluated),
            stage(
                "refs",
                StageStatus::Failed {
                    message: "a ref file did not validate".to_string(),
                },
            ),
            stage(
                "wal-replay",
                StageStatus::NotEvaluated {
                    blocked_by: "refs".to_string(),
                },
            ),
            stage(
                "commit-index",
                StageStatus::Halted {
                    after: "wal-replay".to_string(),
                },
            ),
            stage("brand-new", StageStatus::Other("skipped".to_string())),
        ],
    ));

    let rows: Vec<(&str, &str, Option<&str>)> = view
        .stages
        .iter()
        .map(|s| (s.stage.as_str(), s.status.as_str(), s.detail.as_deref()))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("objects", "evaluated", None),
            ("refs", "failed", Some("a ref file did not validate")),
            ("wal-replay", "not_evaluated", Some("blocked by refs")),
            ("commit-index", "halted", Some("after wal-replay")),
            // **An unmeasured status keeps prikk's word**, and claims no field stikk cannot see.
            ("brand-new", "skipped", None),
        ]
    );
    assert!(view.stages[0].is_evaluated());
    assert!(!view.stages[4].is_evaluated());
}

#[test]
fn a_stage_row_renders_its_name_as_a_name() {
    let view = checked(report(
        true,
        Vec::new(),
        vec![stage(
            "wal-replay",
            StageStatus::NotEvaluated {
                blocked_by: "refs".to_string(),
            },
        )],
    ));
    let pieces = view.stages[0].row();
    assert_eq!(pieces[0], Piece::name("wal-replay"));
    assert_eq!(
        flatten(&pieces),
        "wal-replay  not_evaluated  \u{2014}  blocked by refs"
    );
}

#[test]
fn session_coverage_is_prikks_two_numbers_and_silence_when_it_reported_none() {
    let mut r = report(true, Vec::new(), Vec::new());
    r.active_sessions = Some(ActiveSessions {
        count: 3,
        verified_count: 1,
    });
    assert_eq!(
        checked(r).sessions_sentence().as_deref(),
        Some("prikk verified 1 of 3 active session(s)")
    );

    // **Not reported is silence, never `0 of 0`** (`C-T2c'`), which would claim prikk found no
    // sessions at all.
    let mut none = report(true, Vec::new(), Vec::new());
    none.active_sessions = None;
    assert_eq!(checked(none).sessions_sentence(), None);
}

#[test]
fn the_raw_view_is_bounded_by_the_shared_helper() {
    let mut r = report(true, Vec::new(), Vec::new());
    r.raw = "x".repeat(crate::CONTENT_DISPLAY_BYTES + 500);
    let view = checked(r);
    let last = view.raw.last().expect("the raw view has rows");
    assert!(last.contains("more bytes not shown"), "{last}");
}

#[test]
fn below_the_band_stikk_says_it_did_not_check_rather_than_sound() {
    // **The mistake this guards is the mirror of RFC 040 F2's**: never say *refused* when prikk
    // reports damage, and never say *sound* when stikk never asked.
    let backend = NullBackend::supported().with_version(0, 38, 0);
    let view =
        verify(&backend, std::path::Path::new("/repo")).expect("below the band is not an error");
    assert_eq!(view, VerifyView::Unreported(38));
    assert!(view.checked().is_none());
    let reason = view.unreported_reason().expect("a reason in stikk's voice");
    assert!(reason.contains("from prikk 0.39 on"), "{reason}");
    assert!(reason.contains("this is 0.38"), "{reason}");
    // It never reads as a verdict.
    assert!(!reason.contains("sound"), "{reason}");
}

#[test]
fn a_report_above_the_band_arrives_through_the_seam() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_verification(report(
            false,
            vec![("item-failure", "see item outcomes above")],
            vec![stage("objects", StageStatus::Evaluated)],
        ));
    let view = verify(&backend, std::path::Path::new("/repo")).expect("a report reads");
    let checked = view.checked().expect("a verdict");
    assert_eq!(checked.verdict(), "prikk reports 1 problem(s)");
}

#[test]
fn a_refusal_is_propagated_rather_than_carried() {
    // A Verify screen *is* the report, so there is nothing left to show beside the failure. This
    // is the exit-1-WITHOUT-a-report case; the seam already decided it was a refusal.
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_verify_refusal("error: no prikk repository here");
    let err = verify(&backend, std::path::Path::new("/repo")).expect_err("a refusal propagates");
    assert!(
        err.to_string().contains("no prikk repository here"),
        "{err}"
    );
}

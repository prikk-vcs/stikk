//! Tests for the Verify view (design TS-01; RFC 040).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{VerifyView, checked};
use stikk_prikk::{
    ActiveSessions, FailedCondition, StageStatus, Verdict, Verification, VerifyStage,
};

use super::*;
use crate::test_util::buffer_text;

fn stage(name: &str, status: StageStatus) -> VerifyStage {
    VerifyStage {
        stage: name.to_string(),
        status,
    }
}

/// The 17 stages prikk 0.51 reports, all `evaluated` -- the measured sound shape.
fn all_evaluated() -> Vec<VerifyStage> {
    [
        "objects",
        "refs",
        "ref-update-schema-trust",
        "wal-replay",
        "wal-persistence",
        "rollback-drafts",
        "wal-record-schema",
        "active-wal-metadata",
        "commit-witness",
        "publication-reclassification",
        "commit-index",
        "lifecycle-cache",
        "wal-ordering",
        "received-refs",
        "local-tag-trust",
        "object-connectivity",
        "appended-file-tails",
    ]
    .into_iter()
    .map(|s| stage(s, StageStatus::Evaluated))
    .collect()
}

fn view(ok: bool, conditions: Vec<(&str, &str)>, stages: Vec<VerifyStage>) -> VerifyView {
    VerifyView::Checked(checked(Verification {
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
        raw: "{\n  \"schema_version\": \"verify-report-v1\"\n}".to_string(),
    }))
}

/// **80 columns**, the width every capture in this project is read at.
fn draw(view: &VerifyView, raw: bool) -> String {
    draw_at(view, raw, false, 80, 30)
}

/// The last screenful: the renderer clamps any offset to the end, so `u16::MAX` lands there.
fn draw_scrolled_to_end(view: &VerifyView, raw: bool) -> String {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(u16::MAX);
    terminal
        .draw(|f| render(view, raw, false, &offset, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

fn draw_at(view: &VerifyView, raw: bool, refreshing: bool, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(0);
    terminal
        .draw(|f| {
            render(
                view,
                raw,
                refreshing,
                &offset,
                &Palette::default(),
                f,
                f.area(),
            );
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// The drawn buffer with the borders removed and whitespace collapsed, for asserting on a
/// sentence that wraps.
fn prose(text: &str) -> String {
    text.replace('\u{2502}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// **RFC 040 §6's capture: a sound repository at 80 columns.**
#[test]
fn a_sound_repository_reads_as_sound_with_every_stage() {
    let text = draw(&view(true, Vec::new(), all_evaluated()), false);
    println!("--- RFC 040 §6: Verify, sound, at 80 columns\n{text}");
    assert!(
        prose(&text).contains("prikk reports this repository sound"),
        "{text}"
    );
    // Every stage prikk listed, by prikk's own count.
    assert!(
        prose(&text).contains("stages \u{2014} 17 reported by prikk"),
        "{text}"
    );
    assert!(text.contains("appended-file-tails"), "{text}");
    // prikk's session coverage, both numbers.
    assert!(
        prose(&text).contains("prikk verified 1 of 1 active session(s)"),
        "{text}"
    );
    // Nothing claims a problem.
    assert!(!prose(&text).contains("problem(s)"), "{text}");
}

/// **RFC 040 §6's capture: the `item-failure` case, with the standing sentence directly under it.**
#[test]
fn a_damaged_repository_shows_prikks_condition_with_the_standing_sentence_under_it() {
    let text = draw(
        &view(
            false,
            vec![(
                "item-failure",
                "repository verification found at least one failed object, block, or ref; see \
                 item outcomes above",
            )],
            all_evaluated(),
        ),
        false,
    );
    println!("--- RFC 040 §6: Verify, item-failure, at 80 columns\n{text}");
    let flat = prose(&text);

    // The verdict, in stikk's words, counting prikk's own conditions.
    assert!(flat.contains("prikk reports 1 problem(s)"), "{text}");
    // prikk's id and prikk's sentence, verbatim.
    assert!(flat.contains("item-failure"), "{text}");
    assert!(
        flat.contains("repository verification found at least one failed object, block, or ref; see item outcomes above"),
        "{text}"
    );
    // **And stikk naming where \"above\" actually is**, which is what makes repeating prikk's
    // pointer honest instead of a dead end.
    assert!(
        flat.contains("Run `prikk verify` in a terminal for those"),
        "{text}"
    );
    // **Never \"refused\"**: a damaged repository is a verdict, not a refusal (RFC 040 F2).
    assert!(!flat.contains("refused"), "{text}");
    assert!(!flat.contains("declined"), "{text}");
}

#[test]
fn the_standing_sentence_is_on_a_sound_report_too() {
    // It is a property of prikk's JSON, not of the verdict. A sentence that appeared only on
    // failure would read as part of the failure (RFC 040 decision 3).
    for ok in [true, false] {
        let conditions = if ok {
            Vec::new()
        } else {
            vec![("item-failure", "see item outcomes above")]
        };
        let text = draw(&view(ok, conditions, all_evaluated()), false);
        let flat = prose(&text);
        assert!(
            flat.contains("does not list which objects or refs failed"),
            "ok={ok}: {text}"
        );
        assert!(
            flat.contains("count unverifiable signatures"),
            "ok={ok}: {text}"
        );
    }
}

#[test]
fn several_conditions_are_each_on_their_own_row_in_prikks_order() {
    let text = draw(
        &view(
            false,
            vec![
                ("item-failure", "the first one"),
                ("commit-witness-integrity", "the second one"),
            ],
            Vec::new(),
        ),
        false,
    );
    let flat = prose(&text);
    assert!(flat.contains("prikk reports 2 problem(s)"), "{text}");
    let first = flat.find("item-failure").expect("the first condition");
    let second = flat
        .find("commit-witness-integrity")
        .expect("the second condition");
    assert!(first < second, "prikk's order is kept: {text}");
}

#[test]
fn each_status_prikk_can_report_is_shown_with_its_own_field() {
    let text = draw(
        &view(
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
                // **A status stikk has not measured**, kept as prikk's word rather than shown as
                // `evaluated` -- which would be stikk inventing a clean result.
                stage("brand-new", StageStatus::Other("skipped".to_string())),
            ],
        ),
        false,
    );
    let flat = prose(&text);
    assert!(flat.contains("objects evaluated"), "{text}");
    assert!(
        flat.contains("refs failed \u{2014} a ref file did not validate"),
        "{text}"
    );
    assert!(
        flat.contains("wal-replay not_evaluated \u{2014} blocked by refs"),
        "{text}"
    );
    assert!(
        flat.contains("commit-index halted \u{2014} after wal-replay"),
        "{text}"
    );
    assert!(flat.contains("brand-new skipped"), "{text}");
    assert!(!flat.contains("brand-new evaluated"), "{text}");
}

/// **RFC 040 §6's capture: the raw view.**
#[test]
fn the_raw_view_shows_prikks_own_report() {
    let v = view(
        true,
        Vec::new(),
        vec![stage("objects", StageStatus::Evaluated)],
    );
    let plain = draw(&v, false);
    assert!(!plain.contains("schema_version"), "{plain}");

    let text = draw(&v, true);
    println!("--- RFC 040 §6: Verify, the raw view, at 80 columns\n{text}");
    assert!(text.contains("prikk's own report:"), "{text}");
    assert!(text.contains("schema_version"), "{text}");
    // The verdict is still above it: `v` adds prikk's report, it does not replace the reading.
    assert!(
        prose(&text).contains("prikk reports this repository sound"),
        "{text}"
    );
}

#[test]
fn the_raw_view_is_bounded_and_says_what_it_withheld() {
    // `v` must not be the way around a bound that keeps a large report off the screen.
    let mut report = Verification {
        verdict: Verdict {
            ok: true,
            failed_conditions: Vec::new(),
        },
        active_sessions: None,
        stages: Vec::new(),
        raw: String::new(),
    };
    report.raw = "y".repeat(stikk_core::CONTENT_DISPLAY_BYTES + 400);
    // Scrolled to the bottom, where the withheld count is: the point is that the cut is **said**,
    // not silently made.
    let text = draw_scrolled_to_end(&VerifyView::Checked(checked(report)), true);
    assert!(prose(&text).contains("more bytes not shown"), "{text}");
}

#[test]
fn below_the_band_the_view_says_stikk_did_not_check_never_that_it_is_sound() {
    let text = draw(&VerifyView::Unreported(38), false);
    println!("--- RFC 040: Verify below the band, at 80 columns\n{text}");
    let flat = prose(&text);
    assert!(
        flat.contains("stikk reads prikk's verify report from prikk 0.39 on"),
        "{text}"
    );
    assert!(flat.contains("this is 0.38"), "{text}");
    // **Never a verdict.** A repository nobody checked is not a repository found sound.
    assert!(!flat.contains("sound"), "{text}");
    assert!(!flat.contains("problem(s)"), "{text}");
    // And no stage list invented to fill the space.
    assert!(!flat.contains("stages"), "{text}");
}

#[test]
fn the_heading_says_the_verdict_is_from_this_run() {
    // A verdict is about the moment it was taken (RFC 040 decision 4), and nothing is cached
    // across runs -- so the heading says which run this is, and says when a new one is working.
    let v = view(true, Vec::new(), all_evaluated());
    // **Shortened to fit 80 columns** (RFC 042 review v1 §2); the claim is unchanged -- the
    // verdict on screen is this run's.
    assert!(draw(&v, false).contains("this run"), "{}", draw(&v, false));

    let running = draw_at(&v, false, true, 80, 30);
    assert!(running.contains("re-running"), "{running}");
    assert!(running.contains("the previous verdict"), "{running}");
}

#[test]
fn a_hostile_condition_id_and_stage_name_render_as_identifiers() {
    // prikk's ids and stage names are names the repository's tooling chose, and reach the screen
    // under the identifier rule (RFC 036) -- never raw.
    let text = draw(
        &view(
            false,
            vec![("item\u{202e}failure", "a message")],
            vec![stage("two  spaces", StageStatus::Evaluated)],
        ),
        false,
    );
    assert!(!text.contains('\u{202e}'), "{text}");
    assert!(text.contains("\\u202e"), "{text}");
    // The double space is spelled, so the wrap cannot collapse it into one.
    assert!(!text.contains("two  spaces"), "{text}");
    assert!(text.contains("\\u0020\\u0020"), "{text}");
}

/// **A title must not be cut off at 80 columns** (RFC 042 review v1 §2).
///
/// ratatui truncates a title that will not fit the top border, silently -- so a reader at 80
/// columns sees the view's name and loses the keys at the end of it. The end of a title is the
/// half a reader needs most: what `v` and `r` do, and where in the content they are. This asserts
/// those last characters survive, which is the half that goes first.
fn assert_title_is_whole(screen: &str, tail: &str) {
    let top = screen.lines().next().unwrap_or_default();
    assert!(
        top.contains(tail),
        "the title is cut off at 80 columns -- {tail:?} is missing from:\n{top}"
    );
}

#[test]
fn the_verify_title_is_whole_at_80_columns_in_every_state() {
    // Seventeen stages overflow 24 rows, so every one of these carries the scroll indicator --
    // which is the case that used to cut the title in half.
    let v = view(true, Vec::new(), all_evaluated());
    for (raw, refreshing, tail) in [
        (false, false, "r: re-run"),
        (true, false, "r: re-run"),
        (false, true, "the previous verdict"),
    ] {
        let text = draw_at(&v, raw, refreshing, 80, 24);
        assert!(text.contains("lines 1"), "it must be scrolling: {text}");
        assert_title_is_whole(&text, tail);
        assert_title_is_whole(&text, " of ");
    }
}

//! Tests for the Changes view (design TS-01; RFC 008).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{Authoring, ChangeEntry, ChangeKind, ChangesView, QueuedElsewhere};

use super::*;
use crate::test_util::buffer_text;

fn dirty_view() -> ChangesView {
    ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 2,
        unchanged: 0,
        missing: 1,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        refused: None,
        entries: vec![
            ChangeEntry {
                kind: ChangeKind::Modified,
                path: "readme.txt".into(),
                note: "tracked file bytes differ from the baseline".into(),
                authoring: Authoring::Unreported,
                rename: None,
            },
            ChangeEntry {
                kind: ChangeKind::Missing,
                path: "src/main.rs".into(),
                note: "tracked file is absent from the worktree".into(),
                authoring: Authoring::Unreported,
                rename: None,
            },
            ChangeEntry {
                kind: ChangeKind::Untracked,
                path: "notes.tmp".into(),
                note: "worktree file is not in the baseline".into(),
                authoring: Authoring::Unreported,
                rename: None,
            },
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
    }
}

fn draw(view: &ChangesView, hide_untracked: bool) -> String {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            render(
                view,
                &stikk_core::RefHistory::Published,
                hide_untracked,
                false,
                &Palette::default(),
                f,
                f.area(),
            )
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn renders_the_changed_headline_counts_and_paths() {
    let text = draw(&dirty_view(), false);
    assert!(text.contains("Changes"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("change(s) against baseline"));
    assert!(text.contains("readme.txt"));
    assert!(text.contains("src/main.rs"));
    assert!(text.contains("notes.tmp"));
    // The honesty notes (UD-06 / UD-09) are always present.
    assert!(text.contains("whole-worktree"));
    assert!(text.contains("UD-09"));
}

#[test]
fn a_clean_worktree_says_so() {
    let view = ChangesView {
        reff: "heads/main".into(),
        clean: true,
        tracked: 3,
        unchanged: 3,
        missing: 0,
        modified: 0,
        untracked: 0,
        unsupported: 0,
        refused: None,
        entries: Vec::new(),
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
    };
    let text = draw(&view, false);
    assert!(text.contains("clean against baseline"));
}

#[test]
fn hiding_untracked_removes_the_row_but_keeps_the_caveat() {
    let text = draw(&dirty_view(), true);
    assert!(!text.contains("notes.tmp")); // the untracked row is hidden
    assert!(text.contains("untracked hidden")); // UD-08 caveat present
    assert!(text.contains("still captures them"));
    // Non-untracked rows remain.
    assert!(text.contains("readme.txt"));
}

#[test]
fn a_hostile_path_is_rendered_inert() {
    let view = ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 0,
        unsupported: 0,
        refused: None,
        entries: vec![ChangeEntry {
            kind: ChangeKind::Modified,
            path: "evil\u{1b}[2Jfile.txt".into(),
            note: "bytes differ".into(),
            authoring: Authoring::Unreported,
            rename: None,
        }],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
    };
    let text = draw(&view, false);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn queued_elsewhere_renders_as_a_distinct_verbatim_band() {
    // RFC 009 F4 — the acceptance-critical test.
    let mut view = dirty_view();
    view.queued_elsewhere = Some(QueuedElsewhere::Note(
        "note: the active WAL has queued (unsealed) patches for heads/main, not heads/other"
            .to_string(),
    ));
    let text = draw(&view, false);
    assert!(text.contains("prikk reported"));
    assert!(text.contains("the active WAL has queued (unsealed) patches for heads/main"));
}

#[test]
fn queued_elsewhere_suppresses_the_contradicting_ud08_claim() {
    // The acceptance-critical assertion: with `hide_untracked` set and `queued_elsewhere` present, the
    // string "a commit still captures them" must not appear anywhere in the rendered buffer — it would
    // contradict prikk's own warning that these files may already be committed, queued elsewhere.
    let mut view = dirty_view();
    view.queued_elsewhere = Some(QueuedElsewhere::Note(
        "note: the active WAL has queued (unsealed) patches for heads/main, not heads/other"
            .to_string(),
    ));
    let text = draw(&view, true);
    assert!(!text.contains("still captures them"));
    assert!(text.contains("untracked hidden")); // the UD-08 caveat still appears, reworded
    assert!(text.contains("see prikk's warning above"));
}

#[test]
fn without_queued_elsewhere_the_ud08_claim_is_unchanged() {
    let text = draw(&dirty_view(), true);
    assert!(text.contains("still captures them"));
    assert!(!text.contains("prikk reported"));
}

#[test]
fn a_hostile_queued_elsewhere_note_is_rendered_inert() {
    let mut view = dirty_view();
    view.queued_elsewhere = Some(QueuedElsewhere::Note("\u{1b}[2Jhostile note".to_string()));
    let text = draw(&view, false);
    assert!(!text.contains('\u{1b}'));
}

// RFC 027 Handoff B §5 — every capture at 80 columns, the width Handoff A measured the defects at.

fn draw_80(view: &ChangesView, hide_untracked: bool) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            render(
                view,
                &stikk_core::RefHistory::Published,
                hide_untracked,
                false,
                &Palette::default(),
                f,
                f.area(),
            )
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

const SYMLINK_REASON: &str =
    "precondition not met: link.txt: worktree symlink authoring is out of scope";

/// prikk 0.41's refused-symlink report as the JSON reader maps it (`WORKTREE_SYMLINK_JSON_0_41`).
fn refused_view() -> ChangesView {
    ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 1,
        unchanged: 0,
        missing: 0,
        modified: 1,
        untracked: 1,
        unsupported: 0,
        refused: Some(1),
        entries: vec![
            ChangeEntry {
                kind: ChangeKind::Untracked,
                path: "link.txt".into(),
                note: "worktree file is not in the baseline".into(),
                authoring: Authoring::Refused(SYMLINK_REASON.into()),
                rename: None,
            },
            ChangeEntry {
                kind: ChangeKind::Modified,
                path: "readme.txt".into(),
                note: "tracked file bytes differ from the baseline".into(),
                authoring: Authoring::Authored,
                rename: None,
            },
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
    }
}

/// The screen as one string with each row's border, padding and the queued-work band's `! ` bar
/// stripped, and rows joined by a space — so a wrapped reason or clause can be found whole.
fn joined(screen: &str) -> String {
    screen
        .lines()
        .map(|row| {
            let row = row.trim_matches(|c: char| c == '│' || c.is_whitespace());
            row.strip_prefix("! ").unwrap_or(row)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_refused_entry_reads_as_refused_with_prikks_reason_and_keeps_its_kind() {
    let text = draw_80(&refused_view(), false);
    println!("{text}");
    let link_row = text
        .lines()
        .find(|row| row.contains("link.txt  —"))
        .expect("link.txt's entry row");
    assert!(
        link_row.contains("untracked"),
        "refused is orthogonal to kind: {link_row:?}"
    );
    // A text marker, not colour alone (NFR-A03) — exactly once, on the refused entry only.
    assert_eq!(text.matches("refused — ").count(), 1, "{text}");
    let marker_row_index = text
        .lines()
        .position(|row| row.contains("refused — "))
        .expect("marker row");
    let link_row_index = text.lines().position(|row| row == link_row).expect("row");
    assert_eq!(
        marker_row_index,
        link_row_index + 1,
        "the marker sits under its own entry"
    );
    // prikk's reason, whole — wrapped, never clipped.
    assert!(joined(&text).contains(SYMLINK_REASON), "{text}");
}

#[test]
fn every_count_is_on_screen_at_80_columns() {
    let mut view = refused_view();
    view.unsupported = 2;
    let text = draw_80(&view, false);
    println!("{text}");
    for count in [
        "tracked 1",
        "unchanged 0",
        "modified 1",
        "missing 0",
        "untracked 1",
        "unsupported 2",
        "refused 1",
    ] {
        assert!(text.contains(count), "{count:?} is not on screen:\n{text}");
    }
}

#[test]
fn below_0_39_the_verdict_reads_unreported_never_zero() {
    let text = draw_80(&dirty_view(), false);
    println!("{text}");
    assert!(
        text.contains("refused: not reported by this prikk"),
        "{text}"
    );
    assert!(!text.contains("refused 0"), "{text}");
    assert!(!text.contains("refused — "), "{text}");
}

#[test]
fn the_headline_counts_what_is_listed_and_an_unmodelled_kind_shows_its_word() {
    let mut view = dirty_view();
    view.entries.push(ChangeEntry {
        kind: ChangeKind::Other("typechange".into()),
        path: "link.txt".into(),
        note: "a kind prikk does not print today".into(),
        authoring: Authoring::Unreported,
        rename: None,
    });
    let text = draw_80(&view, false);
    println!("{text}");
    assert!(text.contains("4 change(s) against baseline"), "{text}");
    let row = text
        .lines()
        .find(|row| row.contains("link.txt"))
        .expect("the unmodelled entry is listed");
    assert!(
        row.contains("typechange"),
        "prikk's word in the tag position: {row:?}"
    );
    // On the entry's own row — the counts line's `unchanged 0` contains "changed " too.
    assert!(
        !row.contains("changed"),
        "no stikk paraphrase of the kind: {row:?}"
    );
}

/// The longest repo-relative path an entry row shows whole at 80 columns: 78 inner columns less the
/// 14 the tag spends. **This budget is Handoff A's, and B does not change it** — see the test below.
const PATH_BUDGET_AT_80: usize = 78 - 14;

#[test]
fn a_refused_marker_costs_the_path_row_nothing_at_80_columns() {
    // Handoff A's ask 1: the marker must not cost a refused path its distinguishing tail. **Proved by
    // comparison, not by a single capture**: the same two entries rendered refused and authored must
    // give byte-identical path rows, because the marker lives on its own row beneath. The paths are the
    // longest that fit and differ only in their last characters, so any column the marker took from the
    // path row would cut exactly what tells them apart.
    let long_a = "src/generated/protocol/buffers/nested/module/links/link-alph.txt";
    let long_b = "src/generated/protocol/buffers/nested/module/links/link-omeg.txt";
    assert_eq!(long_a.chars().count(), PATH_BUDGET_AT_80);
    assert_eq!(long_b.chars().count(), PATH_BUDGET_AT_80);

    let with_verdict = |authoring: fn(&str) -> Authoring| {
        let mut view = refused_view();
        view.untracked = 2;
        view.modified = 0;
        view.entries = [long_a, long_b]
            .into_iter()
            .map(|path| ChangeEntry {
                kind: ChangeKind::Untracked,
                path: path.into(),
                note: "worktree file is not in the baseline".into(),
                authoring: authoring(path),
                rename: None,
            })
            .collect();
        view
    };
    let mut refused = with_verdict(|path| {
        Authoring::Refused(format!(
            "precondition not met: {path}: worktree symlink authoring is out of scope"
        ))
    });
    refused.refused = Some(2);
    let mut authored = with_verdict(|_| Authoring::Authored);
    authored.refused = Some(0);

    let refused_text = draw_80(&refused, false);
    let authored_text = draw_80(&authored, false);
    println!("{refused_text}");
    let path_rows = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|row| row.contains("untracked   src/"))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(path_rows(&refused_text).len(), 2, "{refused_text}");
    assert_eq!(
        path_rows(&refused_text),
        path_rows(&authored_text),
        "the refused marker changed a path row"
    );
    for path in [long_a, long_b] {
        assert!(
            refused_text.lines().any(|row| row.contains(path)),
            "{path} is not whole on its row:\n{refused_text}"
        );
    }
}

#[test]
fn a_typed_queued_ref_renders_as_stikks_warning_outside_the_quote_band() {
    // RFC 027 F6: at ≥ 0.39 the words are stikk's, labelled so, and never under "prikk reported —".
    let mut view = dirty_view();
    view.reff = "heads/other".into();
    view.refused = Some(0);
    view.queued_elsewhere = Some(QueuedElsewhere::Ref("heads/main".into()));
    let text = draw_80(&view, false);
    println!("{text}");
    assert!(text.contains("stikk's warning"), "{text}");
    assert!(!text.contains("prikk reported"), "{text}");
    let flat = joined(&text);
    for clause in stikk_core::queued_elsewhere_clauses("heads/main", "heads/other") {
        assert!(
            flat.contains(&clause),
            "clause {clause:?} is not on screen whole:\n{text}"
        );
    }

    // RFC 009 decision 3 still holds at this band.
    let hidden = draw_80(&view, true);
    assert!(!hidden.contains("still captures them"), "{hidden}");
    assert!(
        joined(&hidden).contains("see the queued-work warning above"),
        "{hidden}"
    );
}

#[test]
fn a_hostile_queued_ref_is_rendered_inert() {
    let mut view = dirty_view();
    view.queued_elsewhere = Some(QueuedElsewhere::Ref("\u{1b}[2Jheads/main".into()));
    let text = draw_80(&view, false);
    assert!(!text.contains('\u{1b}'));
}

/// prikk's queued-elsewhere sentence exactly as the **0.28.0** binary printed it on 2026-09-13 (RFC 027
/// Handoff B probe: `Fixture` commit on `heads/main`, repository moved to `/tmp/repo`, then
/// `prikk worktree-status --ref heads/other`). Byte-identical to 0.41.0's prose for the same tree.
const PRIKK_QUEUED_ELSEWHERE_0_28: &str = "note: the active WAL has queued (unsealed) patches for \
     heads/main, not heads/other -- that is real, committed work, not shown above; any \"untracked\" \
     file here may be exactly that work seen from this ref's own baseline, so do not delete based on \
     this report alone (see `prikk status`)";

#[test]
fn prikks_whole_queued_elsewhere_sentence_is_on_screen_at_80_columns() {
    // Before RFC 027 B this band did not wrap, and at 80 columns it clipped after "not hea" — every
    // safety claim after the first was off screen below prikk 0.39.
    let mut view = dirty_view();
    view.reff = "heads/other".into();
    view.queued_elsewhere = Some(QueuedElsewhere::Note(PRIKK_QUEUED_ELSEWHERE_0_28.into()));
    let text = draw_80(&view, false);
    println!("{text}");
    assert!(
        joined(&text).contains(PRIKK_QUEUED_ELSEWHERE_0_28),
        "prikk's sentence is not whole on screen:\n{text}"
    );
    // Every row of it is inside the quote band — prikk's words, and marked as prikk's, row by row.
    let band: Vec<&str> = text
        .lines()
        .skip_while(|row| !row.contains("prikk reported —"))
        .skip(1)
        .take_while(|row| {
            !row.trim_matches(|c: char| c == '│' || c.is_whitespace())
                .is_empty()
        })
        .collect();
    assert!(
        band.len() > 1,
        "the sentence should wrap at 80 columns:\n{text}"
    );
    for row in band {
        assert!(
            row.starts_with("│  │ "),
            "a row of prikk's sentence lost its bar: {row:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// RFC 032 — captures at 80 columns: declared renames, declarations prikk will not author, and a ref with no
// published history.
// ---------------------------------------------------------------------------------------------

fn draw_032(view: &ChangesView, history: &stikk_core::RefHistory, hide_untracked: bool) -> String {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            render(
                view,
                history,
                hide_untracked,
                true,
                &Palette::default(),
                f,
                f.area(),
            )
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

fn change(
    kind: ChangeKind,
    path: &str,
    note: &str,
    rename: Option<stikk_core::RenameHalf>,
) -> ChangeEntry {
    ChangeEntry {
        kind,
        path: path.into(),
        note: note.into(),
        authoring: Authoring::Authored,
        rename,
    }
}

fn declared(
    old: &str,
    new: &str,
    state: stikk_core::DeclarationState,
) -> stikk_core::DeclaredRename {
    stikk_core::DeclaredRename {
        old_path: old.into(),
        new_path: new.into(),
        state,
    }
}

/// Row A beside an ordinary untracked file, with one declaration whose destination is gone and one whose
/// source is back (the destination gone too, so the measured way out is offered).
fn renamed_view() -> ChangesView {
    use stikk_core::{DeclarationState, RenameHalf};
    ChangesView {
        reff: "heads/main".into(),
        clean: false,
        tracked: 4,
        unchanged: 1,
        missing: 2,
        modified: 0,
        untracked: 2,
        unsupported: 0,
        refused: Some(0),
        entries: vec![
            change(
                ChangeKind::Missing,
                "a.txt",
                "tracked file is absent from the worktree",
                Some(RenameHalf::Source {
                    new_path: "b.txt".into(),
                }),
            ),
            change(
                ChangeKind::Untracked,
                "b.txt",
                "worktree file is not in the baseline",
                Some(RenameHalf::Destination {
                    old_path: "a.txt".into(),
                }),
            ),
            change(
                ChangeKind::Missing,
                "c.txt",
                "tracked file is absent from the worktree",
                None,
            ),
            change(
                ChangeKind::Untracked,
                "notes.tmp",
                "worktree file is not in the baseline",
                None,
            ),
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: vec![
            declared("a.txt", "b.txt", DeclarationState::Paired),
            declared("c.txt", "d.txt", DeclarationState::DestinationAbsent),
            declared(
                "e.txt",
                "f.txt",
                DeclarationState::SourcePresent {
                    destination_listed: false,
                },
            ),
        ],
        renames: 1,
    }
}

fn row_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} not on screen:\n{text}"))
}

#[test]
fn rfc032_paired_rows_are_annotated_counted_and_followed_by_the_content_sentence_at_80_columns() {
    let text = draw_032(&renamed_view(), &stikk_core::RefHistory::Published, false);
    println!("--- RFC 032: paired rename and both unmatched declarations, 80 columns\n{text}");
    let flat = joined(&text);
    // prikk's two rows stay, each under prikk's kind, each annotated as half of one declared rename.
    assert!(
        row_of(&text, "a.txt") < row_of(&text, "declared rename → b.txt"),
        "{text}"
    );
    assert!(flat.contains("· declared rename → b.txt"), "{text}");
    assert!(flat.contains("· declared rename ← a.txt"), "{text}");
    assert!(
        text.lines()
            .any(|l| l.contains("missing") && l.contains("a.txt")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|l| l.contains("untracked") && l.contains("b.txt")),
        "{text}"
    );
    // An ordinary entry carries no annotation.
    let notes = text.lines().find(|l| l.contains("notes.tmp")).unwrap();
    assert!(!notes.contains("declared"), "{text}");
    assert!(
        flat.contains("untracked 2 · unsupported 0 · refused 0 · renames 1"),
        "{text}"
    );
    assert!(flat.contains(stikk_core::RENAME_CONTENT_NOTE), "{text}");
    // Under the entries, above the whole-worktree line.
    assert!(
        row_of(&text, "notes.tmp") < row_of(&text, "a declared rename is authored"),
        "{text}"
    );
    assert!(
        row_of(&text, "a declared rename is authored")
            < row_of(&text, "commits are whole-worktree"),
        "{text}"
    );
}

#[test]
fn rfc032_both_unmatched_declaration_sentences_are_on_screen_whole_at_80_columns() {
    let text = draw_032(&renamed_view(), &stikk_core::RefHistory::Published, false);
    let flat = joined(&text);
    assert!(
        flat.contains("declared rename c.txt → d.txt: d.txt is not a file in the worktree, so prikk will not author it as a rename"),
        "{text}"
    );
    assert!(
        flat.contains("declared rename e.txt → f.txt: e.txt is present again, and prikk refuses to commit until the declaration is resolved; in a terminal, prikk mv f.txt e.txt drops it"),
        "{text}"
    );
    assert!(
        row_of(&text, "d.txt is not a file") < row_of(&text, "commits are whole-worktree"),
        "{text}"
    );
}

#[test]
fn rfc032_the_untracked_filter_never_hides_a_paired_destination() {
    let text = draw_032(&renamed_view(), &stikk_core::RefHistory::Published, true);
    println!(
        "--- RFC 032: untracked hidden, the paired destination still shown, 80 columns\n{text}"
    );
    assert!(text.contains("b.txt"), "the destination stays:\n{text}");
    assert!(
        joined(&text).contains("· declared rename ← a.txt"),
        "{text}"
    );
    assert!(
        !text.contains("notes.tmp"),
        "the ordinary untracked file is hidden:\n{text}"
    );
    // Its count says only what it hid.
    assert!(
        joined(&text).contains("1 untracked hidden (display only)"),
        "{text}"
    );
}

#[test]
fn rfc032_a_ref_with_no_published_history_says_so_in_place_of_against_baseline() {
    use stikk_core::{RefHistory, UnpublishedQueue};
    let mut view = dirty_view();
    view.entries = vec![
        change(
            ChangeKind::Untracked,
            "x.txt",
            "worktree file is not in the baseline",
            None,
        ),
        change(
            ChangeKind::Untracked,
            "y.txt",
            "worktree file is not in the baseline",
            None,
        ),
    ];
    let text = draw_032(
        &view,
        &RefHistory::Unpublished(UnpublishedQueue::Empty),
        false,
    );
    println!("--- RFC 032: no published history, no queued patch, 80 columns\n{text}");
    assert!(
        joined(&text).contains("heads/main has no published history — every file is listed as untracked, and a commit would be its first"),
        "{text}"
    );
    assert!(!text.contains("against baseline"), "{text}");

    let text = draw_032(
        &view,
        &RefHistory::Unpublished(UnpublishedQueue::ForThisRef(1)),
        false,
    );
    println!("--- RFC 032: no published history, one queued patch, 80 columns\n{text}");
    assert!(
        joined(&text).contains("heads/main has no published history yet — its 1 queued patch(es) are the baseline here, and nothing is sealed"),
        "{text}"
    );
}

#[test]
fn rfc032_a_refused_destination_keeps_the_annotations_and_drops_the_content_sentence() {
    const REASON: &str = "precondition not met: b.txt: worktree symlink authoring is out of scope";
    let mut view = renamed_view();
    view.refused = Some(1);
    for entry in &mut view.entries {
        if entry.path == "b.txt" {
            entry.authoring = Authoring::Refused(REASON.to_string());
        }
    }
    let text = draw_032(&view, &stikk_core::RefHistory::Published, false);
    println!("--- RFC 032 A7: a paired rename whose destination prikk refuses, 80 columns\n{text}");
    let flat = joined(&text);
    // prikk's report stays whole: both halves annotated, the pair counted, the refusal verbatim.
    assert!(flat.contains("· declared rename → b.txt"), "{text}");
    assert!(flat.contains("· declared rename ← a.txt"), "{text}");
    assert!(flat.contains("renames 1"), "{text}");
    assert!(flat.contains(REASON), "{text}");
    // And nothing promises an outcome prikk would refuse.
    assert!(
        !flat.contains("a declared rename is authored as a rename"),
        "the content sentence must be withheld:\n{text}"
    );

    // Below prikk 0.39 the verdict is unreported, never a zero, and the sentence stays.
    let mut unreported = renamed_view();
    unreported.refused = None;
    assert!(
        joined(&draw_032(
            &unreported,
            &stikk_core::RefHistory::Published,
            false
        ))
        .contains("a declared rename is authored as a rename"),
        "an unreported verdict is not a refusal"
    );
}

//! Tests for the Changes operation (design TS-01/TS-02; RFC 008).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use stikk_prikk::{NullBackend, WorktreeEntry, WorktreeStatus};

use super::*;

fn dirty_status() -> WorktreeStatus {
    WorktreeStatus {
        refused_declarations: None,
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
            WorktreeEntry {
                kind: "modified".into(),
                path: "readme.txt".into(),
                note: "tracked file bytes differ from the baseline".into(),
                authoring: Authoring::Unreported,
            },
            WorktreeEntry {
                kind: "missing".into(),
                path: "src/main.rs".into(),
                note: "tracked file is absent from the worktree".into(),
                authoring: Authoring::Unreported,
            },
            WorktreeEntry {
                kind: "untracked".into(),
                path: "notes.tmp".into(),
                note: "worktree file is not in the baseline".into(),
                authoring: Authoring::Unreported,
            },
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
    }
}

#[test]
fn changes_view_maps_a_dirty_status() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    let view = changes_view(&backend, Path::new("/repo"), "heads/main")
        .expect("changes")
        .view;
    assert!(!view.clean);
    assert_eq!(view.modified, 1);
    assert_eq!(view.entries.len(), 3);
    assert_eq!(view.entries[0].kind, ChangeKind::Modified);
    assert!(view.entries.iter().any(|e| e.kind.is_untracked()));
}

#[test]
fn a_dirty_worktree_is_success_not_a_refusal() {
    // RFC 008 finding 2 / UD-05: the seam has already turned prikk's non-zero dirty exit into an
    // Ok(WorktreeStatus{clean:false}); the operation must carry that through as success.
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    assert!(changes_view(&backend, Path::new("/r"), "heads/main").is_ok());
}

#[test]
fn below_0_28_returns_version_guidance_not_the_command() {
    // RFC 009 raised the default NullBackend to a supported+validated 0.30.0, so the below-the-gate
    // version must now be scripted explicitly rather than relied on as the default.
    let backend = NullBackend::supported()
        .with_version(0, 27, 1)
        .with_worktree_status(dirty_status());
    let err = changes_view(&backend, Path::new("/r"), "heads/main").unwrap_err();
    assert_eq!(err.class(), "not-ready");
    assert!(err.to_string().contains("0.28"));
    assert!(err.to_string().contains("0.27.1")); // states the actual version
}

#[test]
fn a_seam_refusal_propagates() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status_refusal("ref does not exist");
    let err = changes_view(&backend, Path::new("/r"), "heads/nope").unwrap_err();
    assert_eq!(err.class(), "refusal");
}

#[test]
fn an_unknown_kind_is_preserved_as_other() {
    assert_eq!(
        ChangeKind::from_label("typechange"),
        ChangeKind::Other("typechange".into())
    );
}

#[test]
fn prikks_unsupported_path_word_maps_to_unsupported() {
    // RFC 027 F0: `unsupported-path` is what prikk prints, at every tag from 0.28.0 to 0.41.0. The
    // never-printed `unsupported` has no arm of its own any more; if it ever appeared it would render.
    assert_eq!(
        ChangeKind::from_label("unsupported-path"),
        ChangeKind::Unsupported
    );
    assert_eq!(
        ChangeKind::from_label("unsupported"),
        ChangeKind::Other("unsupported".into())
    );
}

/// **An invented kind, through the real reader**: prikk's text → `CliBackend`'s parser →
/// [`from_status`] → `ChangeKind::Other(word)`. The direct `from_label` test above could never catch the
/// defect RFC 027 F0 found, because the parser discarded an unknown word before `from_label` saw it.
/// Unix-only: the stand-in `prikk` is a shell script. **It reports 0.38.0 on purpose**: this is the
/// prose reader's test, and from 0.39 the seam asks for `--format json` (RFC 027 decision 2), which a
/// prose-printing stand-in would not answer.
#[cfg(unix)]
#[test]
fn an_invented_kind_survives_the_parser_and_arrives_as_other() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("stikk-core-invented-kind-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\n\
         case \"$1\" in\n\
         --version)\n\
         printf 'prikk 0.38.0\\n'\n\
         ;;\n\
         worktree-status)\n\
         printf 'ref: heads/main\\n\
         tracked files: 1\\n\
         unchanged files: 0\\n\
         missing files: 0\\n\
         modified files: 1\\n\
         untracked files: 0\\n\
         unsupported paths: 0\\n\
         worktree: changed against baseline\\n\
         \x20 modified readme.txt \\342\\200\\224 tracked file bytes differ from the baseline\\n\
         \x20 typechange link.txt \\342\\200\\224 a kind prikk does not print today\\n\
         live rename declarations: 0\\n'\n\
         exit 1\n\
         ;;\n\
         esac\n",
    )
    .expect("write fake prikk");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let backend = stikk_prikk::CliBackend::with_program(&script);
    // The seam's prose reader into core's view-model, the path this test guards. `changes_view` also reads
    // `refs()` and Orientation since RFC 032, which this stand-in does not script and this test is not about.
    let view = from_status(
        backend
            .worktree_status(&dir, "heads/main")
            .expect("the view builds"),
    );
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(view.entries.len(), 2, "entries were {:?}", view.entries);
    let invented = view
        .entries
        .iter()
        .find(|e| e.path == "link.txt")
        .expect("the invented-kind line is listed, not dropped");
    assert_eq!(invented.kind, ChangeKind::Other("typechange".into()));
    assert_eq!(invented.note, "a kind prikk does not print today");
}

#[test]
fn carries_the_queued_elsewhere_warning_through_unmodified() {
    // RFC 009 F4: the operation layer transports prikk's warning verbatim; it never computes or
    // paraphrases it (ER-02).
    let mut status = dirty_status();
    status.queued_elsewhere = Some(QueuedElsewhere::Note(
        "note: the active WAL has queued patches for heads/main".into(),
    ));
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(status);
    let view = changes_view(&backend, Path::new("/repo"), "heads/main")
        .expect("changes")
        .view;
    assert_eq!(
        view.queued_elsewhere,
        Some(QueuedElsewhere::Note(
            "note: the active WAL has queued patches for heads/main".into()
        ))
    );
}

#[test]
fn queued_elsewhere_is_none_when_prikk_did_not_emit_it() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    let view = changes_view(&backend, Path::new("/repo"), "heads/main")
        .expect("changes")
        .view;
    assert_eq!(view.queued_elsewhere, None);
}

/// prikk's queued-elsewhere sentence exactly as the **0.41.0** binary printed it on 2026-09-13 (RFC 027
/// Handoff B probe: `Fixture` commit on `heads/main`, repository moved to `/tmp/repo`, then
/// `prikk worktree-status --ref heads/other`). Byte-identical to the same probe at 0.28.0.
const PRIKK_QUEUED_ELSEWHERE_SENTENCE: &str = "note: the active WAL has queued (unsealed) patches for \
     heads/main, not heads/other -- that is real, committed work, not shown above; any \"untracked\" \
     file here may be exactly that work seen from this ref's own baseline, so do not delete based on \
     this report alone (see `prikk status`)";

/// **RFC 027 F6, clause by clause.** At prikk ≥ 0.39 stikk words the warning itself; this holds each of
/// its sentences to the claim in prikk's sentence it stands for — one assertion per clause, so an edit
/// that drops or weakens a claim fails by name. Each assertion checks prikk's sentence too, so the test
/// cannot keep passing against a sentence prikk no longer prints.
#[test]
fn stikks_queued_elsewhere_wording_keeps_every_claim_prikks_sentence_makes() {
    let prikk = PRIKK_QUEUED_ELSEWHERE_SENTENCE;
    let [queued, real_work, untracked, do_not_delete] =
        queued_elsewhere_clauses("heads/main", "heads/other");

    // Clause 1 — queued, unsealed patches exist for *that* ref, not for this one.
    assert!(prikk.contains("queued (unsealed) patches for heads/main, not heads/other"));
    assert!(
        queued.contains("unsealed patches for heads/main")
            && queued.contains("queue")
            && queued.contains("not for heads/other"),
        "clause 1 (queued, unsealed, that ref not this one) lost: {queued:?}"
    );

    // Clause 2 — that is real committed work, not shown here.
    assert!(prikk.contains("that is real, committed work, not shown above"));
    assert!(
        real_work.contains("real, committed work") && real_work.contains("not shown here"),
        "clause 2 (real committed work, not shown) lost: {real_work:?}"
    );

    // Clause 3 — an untracked entry may be exactly that work, seen from this ref's baseline.
    assert!(prikk.contains(
        "any \"untracked\" file here may be exactly that work seen from this ref's own baseline"
    ));
    assert!(
        untracked.contains("untracked")
            && untracked.contains("may be exactly that work")
            && untracked.contains("this ref's own baseline"),
        "clause 3 (untracked may be that work, this ref's baseline) lost: {untracked:?}"
    );

    // Clause 4 — do not delete on the strength of this view alone.
    assert!(prikk.contains("do not delete based on this report alone"));
    assert!(
        do_not_delete.contains("Do not delete") && do_not_delete.contains("alone"),
        "clause 4 (do not delete on this alone) lost: {do_not_delete:?}"
    );
}

#[test]
fn stikks_queued_elsewhere_wording_adds_no_claim() {
    // "Adds none": four clauses, each a single sentence. A fifth claim would be a fifth sentence.
    for clause in queued_elsewhere_clauses("heads/main", "heads/other") {
        assert_eq!(
            clause.matches(". ").count(),
            0,
            "one sentence per clause: {clause:?}"
        );
        assert!(clause.ends_with('.'), "{clause:?}");
    }
}

#[test]
fn a_typed_queued_ref_and_the_refused_count_travel_into_the_view() {
    let mut status = dirty_status();
    status.refused = Some(1);
    status.entries[2].authoring = Authoring::Refused("precondition not met: r".into());
    status.queued_elsewhere = Some(QueuedElsewhere::Ref("heads/main".into()));
    let backend = NullBackend::supported()
        .with_version(0, 41, 0)
        .with_worktree_status(status);
    let view = changes_view(&backend, Path::new("/repo"), "heads/other")
        .expect("changes")
        .view;
    assert_eq!(view.refused, Some(1));
    assert_eq!(
        view.entries[2].authoring,
        Authoring::Refused("precondition not met: r".into())
    );
    assert_eq!(
        view.queued_elsewhere,
        Some(QueuedElsewhere::Ref("heads/main".into()))
    );
}

#[test]
fn the_kind_label_is_prikks_word() {
    assert_eq!(ChangeKind::Unsupported.label(), "unsupported-path");
    assert_eq!(ChangeKind::Other("typechange".into()).label(), "typechange");
}

// ---------------------------------------------------------------------------------------------
// RFC 032: declared renames, classified from the report alone, and a ref with no published history.
// Every report below is shaped exactly like a row measured at prikk 0.42.0 (handoff §2; review v1).
// ---------------------------------------------------------------------------------------------

fn listed(kind: &str, path: &str) -> WorktreeEntry {
    WorktreeEntry {
        kind: kind.into(),
        path: path.into(),
        note: format!("prikk's note for {kind}"),
        authoring: Authoring::Authored,
    }
}

fn declared(old: &str, new: &str) -> RenameDeclaration {
    RenameDeclaration {
        resolution: None,
        refusal: None,
        content_changed: None,
        mode_changed: None,
        old_path: old.into(),
        new_path: new.into(),
    }
}

/// A report with `entries` and `declarations`, its counts derived from the entries as prikk derives them.
fn report(entries: Vec<WorktreeEntry>, declarations: Vec<RenameDeclaration>) -> WorktreeStatus {
    let count = |kind: &str| entries.iter().filter(|e| e.kind == kind).count() as u64;
    WorktreeStatus {
        refused_declarations: None,
        reff: "heads/main".into(),
        clean: entries.is_empty(),
        tracked: 2,
        unchanged: 1,
        missing: count("missing"),
        modified: count("modified"),
        untracked: count("untracked"),
        unsupported: 0,
        refused: Some(0),
        entries,
        queued_elsewhere: None,
        declarations,
    }
}

fn state_of(view: &ChangesView) -> Vec<DeclarationState> {
    view.declared_renames
        .iter()
        .map(|d| d.state.clone())
        .collect()
}

fn marks(view: &ChangesView) -> Vec<(String, Option<RenameHalf>)> {
    view.entries
        .iter()
        .map(|e| (e.path.clone(), e.rename.clone()))
        .collect()
}

#[test]
fn rows_a_b_and_c_are_paired_and_mark_both_halves_only() {
    let a = || vec![listed("missing", "a.txt"), listed("untracked", "b.txt")];
    let mut b = a();
    b.push(listed("modified", "keep.txt"));
    // Row C's report is identical to row A's (F3): prikk does not report the edit after the move.
    for (row, entries) in [("A", a()), ("B", b), ("C", a())] {
        let view = from_status(report(entries, vec![declared("a.txt", "b.txt")]));
        assert_eq!(state_of(&view), [DeclarationState::Paired], "row {row}");
        assert_eq!(view.renames, 1, "row {row}");
        let marks = marks(&view);
        assert_eq!(
            marks[0],
            (
                "a.txt".to_string(),
                Some(RenameHalf::Source {
                    new_path: "b.txt".into()
                })
            ),
            "row {row}"
        );
        assert_eq!(
            marks[1],
            (
                "b.txt".to_string(),
                Some(RenameHalf::Destination {
                    old_path: "a.txt".into()
                })
            ),
            "row {row}"
        );
        assert!(marks[2..].iter().all(|(_, m)| m.is_none()), "row {row}");
    }
}

#[test]
fn rows_1_and_3_are_destination_absent_and_mark_nothing() {
    let row_1 = report(
        vec![listed("missing", "a.txt")],
        vec![declared("a.txt", "b.txt")],
    );
    let row_3 = report(
        vec![listed("missing", "a.txt"), listed("untracked", "c.txt")],
        vec![declared("a.txt", "b.txt")],
    );
    // Also measured: the destination replaced by a directory, so prikk lists only a file inside it.
    let directory = report(
        vec![
            listed("missing", "a.txt"),
            listed("untracked", "b.txt/q.txt"),
        ],
        vec![declared("a.txt", "b.txt")],
    );
    for (row, status) in [("1", row_1), ("3", row_3), ("directory", directory)] {
        let view = from_status(status);
        assert_eq!(
            state_of(&view),
            [DeclarationState::DestinationAbsent],
            "row {row}"
        );
        assert_eq!(view.renames, 0, "row {row}");
        assert!(marks(&view).iter().all(|(_, m)| m.is_none()), "row {row}");
    }
}

#[test]
fn rows_2_and_5_are_source_present_again_and_row_5_proves_the_order() {
    let row_2 = from_status(report(
        vec![listed("untracked", "b.txt")],
        vec![declared("a.txt", "b.txt")],
    ));
    assert_eq!(
        state_of(&row_2),
        [DeclarationState::SourcePresent {
            destination_listed: true
        }]
    );
    assert!(marks(&row_2).iter().all(|(_, m)| m.is_none()));

    // Row 5 lists no entries at all, and prikk reports it clean. Checking the destination first would call
    // it "destination absent".
    let row_5 = from_status(report(Vec::new(), vec![declared("a.txt", "b.txt")]));
    assert!(row_5.clean);
    assert_eq!(
        state_of(&row_5),
        [DeclarationState::SourcePresent {
            destination_listed: false
        }]
    );
    assert_eq!(row_5.renames, 0);

    // Measured too: the source recreated with different content lists as `modified`, not `missing`.
    let modified_source = from_status(report(
        vec![listed("modified", "a.txt"), listed("untracked", "b.txt")],
        vec![declared("a.txt", "b.txt")],
    ));
    assert_eq!(
        state_of(&modified_source),
        [DeclarationState::SourcePresent {
            destination_listed: true
        }]
    );
}

#[test]
fn an_unrelated_change_and_two_renames_are_classified_independently() {
    let view = from_status(report(
        vec![
            listed("missing", "a.txt"),
            listed("untracked", "b.txt"),
            listed("untracked", "k2.txt"),
            listed("missing", "keep.txt"),
        ],
        vec![declared("a.txt", "b.txt"), declared("keep.txt", "k2.txt")],
    ));
    assert_eq!(
        state_of(&view),
        [DeclarationState::Paired, DeclarationState::Paired]
    );
    assert_eq!(view.renames, 2);

    let unrelated = from_status(report(
        vec![
            listed("missing", "a.txt"),
            listed("untracked", "b.txt"),
            listed("missing", "keep.txt"),
        ],
        vec![declared("a.txt", "b.txt")],
    ));
    assert_eq!(unrelated.renames, 1);
    assert_eq!(marks(&unrelated)[2], ("keep.txt".to_string(), None));
}

#[test]
fn the_analysis_is_equal_for_equal_reports() {
    // RFC 030's re-read at Enter compares views: the analysis must not make equal reports unequal.
    let status = || {
        report(
            vec![listed("missing", "a.txt"), listed("untracked", "b.txt")],
            vec![declared("a.txt", "b.txt")],
        )
    };
    assert_eq!(from_status(status()), from_status(status()));
}

#[test]
fn every_rename_sentence_is_byte_exact() {
    assert_eq!(
        RenameHalf::Source {
            new_path: "b.txt".into()
        }
        .annotation(),
        "· declared rename → b.txt"
    );
    assert_eq!(
        RenameHalf::Destination {
            old_path: "a.txt".into()
        }
        .annotation(),
        "· declared rename ← a.txt"
    );
    assert_eq!(
        RENAME_CONTENT_NOTE,
        "a declared rename is authored as a rename; prikk does not report whether its content also changed"
    );
    assert_eq!(
        RENAMES_ALSO_COUNTED,
        "each rename is also counted above as one missing and one untracked path"
    );
    let notice = |state| {
        DeclaredRename {
            old_path: "a.txt".into(),
            new_path: "b.txt".into(),
            state,
        }
        .notice()
    };
    assert_eq!(notice(DeclarationState::Paired), None);
    assert_eq!(
        notice(DeclarationState::DestinationAbsent).as_deref(),
        Some(
            "declared rename a.txt → b.txt: b.txt is not a file in the worktree, so prikk will not author it as a rename"
        )
    );
    // Row 2: both copies present — no way out offered.
    assert_eq!(
        notice(DeclarationState::SourcePresent {
            destination_listed: true
        })
        .as_deref(),
        Some(
            "declared rename a.txt → b.txt: a.txt is present again, and prikk refuses to commit until the declaration is resolved"
        )
    );
    // Row 5: the destination gone — prikk mv drops it, as measured at 0.42.0 (A3).
    assert_eq!(
        notice(DeclarationState::SourcePresent {
            destination_listed: false
        })
        .as_deref(),
        Some(
            "declared rename a.txt → b.txt: a.txt is present again, and prikk refuses to commit until the declaration is resolved; in a terminal, prikk mv b.txt a.txt drops it"
        )
    );
}

fn ref_named(name: &str, closed: bool, received: bool) -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: name.into(),
        id: "x".into(),
        closed,
        received,
    }
}

#[test]
fn publication_is_refs_membership_then_the_queue_for_this_ref() {
    let published = [ref_named("heads/main", false, false)];
    assert_eq!(
        RefHistory::from_facts("heads/main", &published, 3, Some("heads/main")),
        RefHistory::Published
    );
    // Closed and received refs are published history too.
    for entry in [
        ref_named("heads/old", true, false),
        ref_named("heads/in", false, true),
    ] {
        let name = entry.name.clone();
        assert_eq!(
            RefHistory::from_facts(&name, &[entry], 0, None),
            RefHistory::Published
        );
    }
    let unpublished = |n, target| RefHistory::from_facts("heads/main", &[], n, target);
    assert_eq!(
        unpublished(0, None),
        RefHistory::Unpublished(UnpublishedQueue::Empty)
    );
    assert_eq!(
        unpublished(2, Some("heads/main")),
        RefHistory::Unpublished(UnpublishedQueue::ForThisRef(2))
    );
    assert_eq!(
        unpublished(1, Some("heads/other")),
        RefHistory::Unpublished(UnpublishedQueue::NotThisRef)
    );
    assert_eq!(
        unpublished(4, None),
        RefHistory::Unpublished(UnpublishedQueue::NotThisRef)
    );
}

#[test]
fn every_no_published_history_sentence_is_byte_exact() {
    let h = |q| RefHistory::Unpublished(q);
    let r = "heads/main";
    assert_eq!(RefHistory::Published.changes_headline(r, false), None);
    assert_eq!(RefHistory::Published.card_line(r), None);
    assert_eq!(
        h(UnpublishedQueue::Empty)
            .changes_headline(r, false)
            .as_deref(),
        Some(
            "heads/main has no published history — every file is listed as untracked, and a commit would be its first"
        )
    );
    assert_eq!(
        h(UnpublishedQueue::Empty)
            .changes_headline(r, true)
            .as_deref(),
        Some("heads/main has no published history, and nothing in the worktree to commit")
    );
    assert_eq!(
        h(UnpublishedQueue::ForThisRef(2))
            .changes_headline(r, false)
            .as_deref(),
        Some(
            "heads/main has no published history yet — its 2 queued patch(es) are the baseline here, and nothing is sealed"
        )
    );
    assert_eq!(
        h(UnpublishedQueue::ForThisRef(2))
            .changes_headline(r, true)
            .as_deref(),
        Some(
            "heads/main has no published history yet — nothing in the worktree beyond its 2 queued patch(es), and nothing is sealed"
        )
    );
    for clean in [false, true] {
        assert_eq!(
            h(UnpublishedQueue::NotThisRef)
                .changes_headline(r, clean)
                .as_deref(),
            Some("heads/main has no published history")
        );
    }
    assert_eq!(
        h(UnpublishedQueue::Empty).card_line(r).as_deref(),
        Some("heads/main has no published history: this would be its first commit")
    );
    assert_eq!(
        h(UnpublishedQueue::ForThisRef(2)).card_line(r).as_deref(),
        Some(
            "heads/main has no published history: this adds to its 2 queued patch(es), and nothing is sealed until the queue is sealed"
        )
    );
    assert_eq!(
        h(UnpublishedQueue::NotThisRef).card_line(r).as_deref(),
        Some("heads/main has no published history")
    );
}

#[test]
fn the_changes_operation_reads_publication_beside_the_view_and_never_guesses() {
    let repo = Path::new("/repo");
    // NullBackend's default refs publish heads/main.
    let read = changes_view(&NullBackend::supported(), repo, "heads/main").expect("reads");
    assert_eq!(read.history, RefHistory::Published);

    let unpublished = NullBackend::supported().with_refs(Vec::new());
    let read = changes_view(&unpublished, repo, "heads/main").expect("reads");
    assert_eq!(
        read.history,
        RefHistory::Unpublished(UnpublishedQueue::Empty)
    );

    let refs_fail = NullBackend::supported().with_refs_refusal("branch list failed");
    assert!(changes_view(&refs_fail, repo, "heads/main").is_err());
    let orientation_fail = NullBackend::supported()
        .with_refs(Vec::new())
        .with_orientation_refusal("status failed");
    assert!(changes_view(&orientation_fail, repo, "heads/main").is_err());
}

/// **prikk 0.38.0's prose drives the same analysis** (handoff §8 test 2). The report is
/// `stikk-prikk`'s `WORKTREE_RENAME_0_38_FIXTURE`, captured from the real 0.38.0 binary, copied here
/// verbatim and printed by a stand-in prikk so the seam's own prose reader parses it.
#[cfg(unix)]
#[test]
fn prikk_0_38_prose_declarations_analyse_as_paired() {
    use std::os::unix::fs::PermissionsExt;

    const WORKTREE_RENAME_0_38_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 1
modified files: 0
untracked files: 1
unsupported paths: 0
worktree: changed against baseline
  missing modified draft.txt — tracked file is absent from the worktree
  untracked renamed.txt — worktree file is not in the baseline
live rename declarations: 1
  modified draft.txt -> renamed.txt
note: each declaration above is authored into the next `prikk commit` as a RenamePath -- run `prikk mv` again to change it, or move the destination back to the source to clear it
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";
    let dir = std::env::temp_dir().join(format!("stikk-core-rename-0-38-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("report.txt"), WORKTREE_RENAME_0_38_FIXTURE).expect("write report");
    let script = dir.join("fake-prikk.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n--version) printf 'prikk 0.38.0\\n' ;;\nworktree-status) cat '{}'; exit 1 ;;\nesac\n",
            dir.join("report.txt").display()
        ),
    )
    .expect("write fake prikk");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let backend = stikk_prikk::CliBackend::with_program(&script);
    let status = backend
        .worktree_status(&dir, "heads/main")
        .expect("0.38 prose parses");
    let _ = std::fs::remove_dir_all(&dir);
    let view = from_status(status);

    assert_eq!(state_of(&view), [DeclarationState::Paired], "{view:?}");
    assert_eq!(view.renames, 1);
    assert_eq!(
        view.entries[0].rename,
        Some(RenameHalf::Source {
            new_path: "renamed.txt".into()
        })
    );
    assert_eq!(
        view.entries[1].rename,
        Some(RenameHalf::Destination {
            old_path: "modified draft.txt".into()
        })
    );
}

/// RFC 032 amendment A7: the content sentence promises an outcome, so it is withheld while prikk reports that
/// `commit` would refuse something — a paired rename whose destination is the refused entry is the measured case.
#[test]
fn the_content_sentence_is_withheld_while_prikk_reports_a_refusal() {
    let paired = |refused| {
        let mut status = report(
            vec![listed("missing", "a.txt"), listed("untracked", "b.txt")],
            vec![declared("a.txt", "b.txt")],
        );
        status.refused = refused;
        from_status(status)
    };
    let refused = paired(Some(1));
    assert_eq!(refused.content_note(), None);
    // The declaration and both halves are prikk's own report, and stay.
    assert_eq!(refused.renames, 1);
    assert_eq!(state_of(&refused), [DeclarationState::Paired]);
    assert!(refused.entries.iter().all(|e| e.rename.is_some()));

    // Nothing refused, and — below prikk 0.39 — a verdict that is unreported, never a zero (`C-T2c′`).
    assert_eq!(paired(Some(0)).content_note(), Some(RENAME_CONTENT_NOTE));
    assert_eq!(paired(None).content_note(), Some(RENAME_CONTENT_NOTE));

    // Nothing paired, nothing to say.
    let unpaired = from_status(report(
        vec![listed("missing", "a.txt")],
        vec![declared("a.txt", "b.txt")],
    ));
    assert_eq!(unpaired.content_note(), None);
}

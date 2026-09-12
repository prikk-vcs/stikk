//! Tests for the Changes operation (design TS-01/TS-02; RFC 008).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use stikk_prikk::{NullBackend, WorktreeEntry, WorktreeStatus};

use super::*;

fn dirty_status() -> WorktreeStatus {
    WorktreeStatus {
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
    }
}

#[test]
fn changes_view_maps_a_dirty_status() {
    let backend = NullBackend::supported()
        .with_version(0, 28, 1)
        .with_worktree_status(dirty_status());
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
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
    let view = changes_view(&backend, &dir, "heads/main").expect("the view builds");
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
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
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
    let view = changes_view(&backend, Path::new("/repo"), "heads/main").expect("changes");
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
    let view = changes_view(&backend, Path::new("/repo"), "heads/other").expect("changes");
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

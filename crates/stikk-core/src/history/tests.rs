//! Tests for the history/block-detail operations (design TS-02), via the scripted backend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use stikk_prikk::{
    BlockRow, History, NullBackend, Orientation, RefEntry, StateFile, StateFiles, UnpublishedBranch,
};

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
        previous_ref_state: if seq > 1 { Some("prev".into()) } else { None },
    }
}

#[test]
fn history_view_composes_lineage_and_queue_count() {
    let backend = NullBackend::supported()
        .with_history(History {
            reff: "heads/main".into(),
            blocks: vec![block("bb", 2, "Normal", 1), block("aa", 1, "Root", 0)],
        })
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 3,
            queued_target: None,
            main_ref_state: Some("rs-bb".into()),
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
        });
    let view = history_view(&backend, Path::new("/repo"), "heads/main", 20).expect("history");
    assert_eq!(view.reff, "heads/main");
    assert_eq!(view.queued, 3);
    assert_eq!(view.blocks.len(), 2);
    assert_eq!(view.blocks[0].update_seq, 2); // tip first
}

/// **An older block lists its own files at prikk ≥ 0.46** (RFC 037 decision 1) — the behaviour this
/// increment exists to add. The test it replaces asserted the opposite,
/// `block_detail_fetches_state_only_for_the_tip`, and was right until prikk's `tree` arrived.
#[test]
fn every_block_lists_its_files_when_the_seam_reports_them() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_state(StateFiles {
            target_block: "bb".into(),
            files: vec![StateFile::file("readme.txt", 0o100_644)],
            total_bytes: 12,
        });
    for (id, is_tip) in [("bb", true), ("aa", false)] {
        let detail = block_detail(
            &backend,
            Path::new("/r"),
            "heads/main",
            block(id, 2, "Normal", 1),
            is_tip,
        )
        .expect("detail");
        assert_eq!(detail.is_tip, is_tip);
        assert_eq!(
            detail.state.listed().map(|s| s.files.len()),
            Some(1),
            "the files are listed whether or not {id} is the tip"
        );
        assert!(detail.state.unreported_reason().is_none());
    }
}

/// **Below prikk 0.46 an older block's files are *not reported*, which is not an empty list**
/// (`C-T2c′`; RFC 037 decision 2). The sentence is stikk's own, and it names both numbers: the band
/// stikk lists from, and the prikk actually running.
#[test]
fn below_the_band_an_older_blocks_files_are_unreported_not_empty() {
    let backend = NullBackend::supported()
        .with_version(0, 45, 0)
        .with_state_unreported();
    let detail = block_detail(
        &backend,
        Path::new("/r"),
        "heads/main",
        block("aa", 1, "Root", 0),
        false,
    )
    .expect("detail");
    assert_eq!(detail.state.listed(), None, "never an empty file list");
    let reason = detail
        .state
        .unreported_reason()
        .expect("a sentence instead of a list");
    assert!(
        reason.contains("prikk replays only to the ref tip"),
        "prikk's limit, where it is still true: {reason}"
    );
    assert!(
        reason.contains("from prikk 0.46 on") && reason.contains("this is 0.45"),
        "both numbers, so the reader knows what would change: {reason}"
    );
}

/// **The sentence about replaying to the tip is never shown at prikk ≥ 0.46**, where it would be
/// false (`C-T2b`; RFC 037 §3). This is the assertion that keeps the two bands from drifting into
/// one: a frontend that printed the old sentence beside a correct file list would be contradicting
/// itself on screen.
#[test]
fn the_tip_only_sentence_is_never_shown_where_it_would_be_false() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_state(StateFiles {
            target_block: "aa".into(),
            files: vec![StateFile::file("old.txt", 0o100_644)],
            total_bytes: 3,
        });
    let detail = block_detail(
        &backend,
        Path::new("/r"),
        "heads/main",
        block("aa", 1, "Root", 0),
        false,
    )
    .expect("detail");
    assert_eq!(detail.state.unreported_reason(), None);
}

/// **A failed file read costs Block detail its file list and nothing else** (RFC 037 decision 5,
/// which is RFC 035 A3's rule applied to the second read in this view). The block's own fields and
/// its patch list were readable before this read existed.
#[test]
fn a_failed_file_read_leaves_the_block_and_its_patches_intact() {
    let mut row = block("bb", 2, "Normal", 1);
    row.patches = 1;
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_state_refusal("precondition not met: block bb is not in this repository");
    let detail = block_detail(&backend, Path::new("/r"), "heads/main", row, true)
        .expect("the block itself still reads");
    assert_eq!(detail.row.block_id, "bb");
    assert_eq!(detail.row.patches, 1);
    assert!(
        !matches!(detail.patches, BlockPatches::Failed(_)),
        "the patches section carries no failure of the *file* read: {:?}",
        detail.patches
    );
    let reason = detail
        .state
        .unreported_reason()
        .expect("the files section says what failed");
    assert!(
        reason.contains("block bb is not in this repository"),
        "prikk's own words, carried (`ER-02`): {reason}"
    );
}

#[test]
fn a_history_refusal_propagates() {
    let backend = NullBackend::supported().with_history_refusal("repository is retired format 3");
    let err = history_view(&backend, Path::new("/r"), "heads/main", 20).unwrap_err();
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("retired format 3"));
}

fn ref_entry(name: &str, id: &str) -> RefEntry {
    RefEntry {
        name: name.to_string(),
        id: id.to_string(),
        closed: false,
        received: false,
    }
}

#[test]
fn list_refs_merges_branches_and_tags() {
    // RFC 012 FR-014 completion: a tag comes from `Prikk::tags`, not (necessarily) `Prikk::refs`.
    let backend = NullBackend::supported()
        .with_refs(vec![ref_entry("heads/main", "x")])
        .with_tags(vec![ref_entry("tags/v1", "y")]);
    let refs = list_refs(&backend, Path::new("/r")).unwrap().refs;
    assert_eq!(refs.len(), 2);
    assert!(!refs[0].is_tag());
    assert!(refs[1].is_tag());
}

#[test]
fn list_refs_passes_through_a_tag_that_leaked_into_refs_when_tags_is_empty() {
    // `Prikk::refs`'s own contract says it does not reliably exclude tags (the real prikk `branch
    // list --all` was found to leak them, RFC 012 FR-014) — if `tags` has nothing to say about a name,
    // whatever `refs` reported for it still shows.
    let backend = NullBackend::supported().with_refs(vec![
        ref_entry("heads/main", "x"),
        ref_entry("tags/v1", "y"),
    ]);
    let refs = list_refs(&backend, Path::new("/r")).unwrap().refs;
    assert_eq!(refs.len(), 2);
    assert!(refs[1].is_tag());
}

#[test]
fn list_refs_deduplicates_a_tag_that_leaked_into_refs_too() {
    // The case the merge exists for: if `refs` ever leaks a tag (as the real `branch list --all` does
    // today) *and* `tags` also reports it, the entry must appear exactly once — the `tags`-sourced one
    // wins, since it is the documented, stable source.
    let leaked_in_refs = RefEntry {
        name: "tags/v1".into(),
        id: "stale-if-this-one-shows".into(),
        closed: false,
        received: false,
    };
    let backend = NullBackend::supported()
        .with_refs(vec![ref_entry("heads/main", "x"), leaked_in_refs])
        .with_tags(vec![ref_entry("tags/v1", "authoritative")]);
    let refs = list_refs(&backend, Path::new("/r")).unwrap().refs;
    assert_eq!(refs.len(), 2);
    let tag = refs.iter().find(|r| r.name == "tags/v1").unwrap();
    assert_eq!(tag.id, "authoritative");
}

#[test]
fn a_tags_refusal_propagates() {
    let backend = NullBackend::supported().with_tags_refusal("prikk refused to list tags");
    let err = list_refs(&backend, Path::new("/r")).unwrap_err();
    assert_eq!(err.class(), "refusal");
    assert!(err.to_string().contains("prikk refused to list tags"));
}

// RFC 028 decision 5 (Handoff A §5): the tier says whose queue it is.

fn view_with_queue(reff: &str, queued: u64, queued_target: Option<&str>) -> HistoryView {
    HistoryView {
        reff: reff.to_string(),
        queued,
        queued_target: queued_target.map(str::to_string),
        blocks: Vec::new(),
    }
}

#[test]
fn the_queued_tier_follows_each_row_of_the_table_exactly() {
    assert_eq!(view_with_queue("heads/main", 0, None).queued_tier(), None);
    assert_eq!(
        view_with_queue("heads/main", 2, Some("heads/main"))
            .queued_tier()
            .as_deref()
            .map(crate::flatten)
            .as_deref(),
        Some("2 patch(es) in the active WAL — not yet sealed · Q: Queue")
    );
    assert_eq!(
        view_with_queue("heads/other", 2, Some("heads/main"))
            .queued_tier()
            .as_deref()
            .map(crate::flatten)
            .as_deref(),
        Some("the active WAL holds 2 patch(es) for heads/main — not this ref's history · Q: Queue")
    );
    assert_eq!(
        view_with_queue("heads/main", 1, None)
            .queued_tier()
            .as_deref()
            .map(crate::flatten)
            .as_deref(),
        Some("the active WAL holds 1 patch(es); prikk reports no target ref · Q: Queue")
    );
}

#[test]
fn history_view_takes_the_count_and_the_target_from_one_orientation_read() {
    let backend = NullBackend::supported()
        .with_history(History {
            reff: "heads/other".into(),
            blocks: Vec::new(),
        })
        .with_orientation(Orientation {
            interrupted_materialization: None,
            queued_patches: 2,
            queued_target: Some("heads/main".into()),
            main_ref_state: None,
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
            current_branch: stikk_model::CurrentBranch::NotReported,
        });
    let view = history_view(&backend, Path::new("/repo"), "heads/other", 20).expect("history");
    assert_eq!(view.queued, 2);
    assert_eq!(view.queued_target.as_deref(), Some("heads/main"));
    assert_eq!(
        view.queued_tier().as_deref().map(crate::flatten).as_deref(),
        Some("the active WAL holds 2 patch(es) for heads/main — not this ref's history · Q: Queue")
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 035 §4.1 — Block detail lists the block's own patches.
// ---------------------------------------------------------------------------------------------

const PATCH_ONE: &str = "4db1669a176b64136838975ef7e2724e2161ad78b89c6dabc4efa24cd6818f66";
const PATCH_TWO: &str = "c23a5d7d4d5d639b3e41e76806158e4bc475f113f3ed85858501010c2d830999";

fn shown(id: &str, operations: usize) -> stikk_prikk::ShownPatch {
    stikk_prikk::ShownPatch {
        patch_id: id.to_string(),
        queued: false,
        operations: (0..operations)
            .map(|_| stikk_prikk::PatchOperation {
                kind: "edit-text".to_string(),
                paths: vec![stikk_prikk::QueuedPath::Path("a.txt".to_string())],
                content: stikk_prikk::OperationContent::TextEdit {
                    old_span: "one".to_string(),
                    replacement: "two".to_string(),
                },
                raw: "{}".to_string(),
            })
            .collect(),
    }
}

/// **The message is joined by `patch_id`, never by position** (RFC 035 §5 trap 4).
///
/// `patch_messages` omits a patch that carries no message — a patch authored below prikk 0.32 — so
/// positions drift. Here the block's **second** patch is the one with an entry, and the first must come
/// out *no message reported*: absence, not an empty message (RFC 015 F4, `C-T2c′`).
#[test]
fn a_blocks_patch_messages_are_joined_by_id_not_by_position() {
    let mut row = block("bb", 2, "Normal", 1);
    row.patches = 2;
    row.messages = vec![stikk_prikk::PatchMessage {
        patch_id: PATCH_TWO.to_string(),
        message: "the second patch".to_string(),
    }];
    let backend =
        NullBackend::supported()
            .with_version(0, 51, 0)
            .with_show(stikk_prikk::ShowReport::Listed(vec![
                shown(PATCH_ONE, 1),
                shown(PATCH_TWO, 2),
            ]));

    let detail = block_detail(&backend, Path::new("/repo"), "heads/main", row, false)
        .expect("block detail reads");
    let rows = detail.patches.rows();
    assert_eq!(rows.len(), 2, "prikk's order, both patches");
    assert_eq!(rows[0].patch.patch_id, PATCH_ONE);
    assert_eq!(
        rows[0].message,
        crate::patch::PatchMessageState::NotReported,
        "no entry for this id — never the other patch's message"
    );
    assert_eq!(
        rows[1].message,
        crate::patch::PatchMessageState::Text("the second patch".to_string())
    );
    assert!(
        crate::flatten(&rows[0].summary()).contains("(no message reported)"),
        "{}",
        crate::flatten(&rows[0].summary())
    );
    assert!(
        crate::flatten(&rows[1].summary()).contains("the second patch")
            && crate::flatten(&rows[1].summary()).contains("2 operation(s)"),
        "{}",
        crate::flatten(&rows[1].summary())
    );
    assert_eq!(detail.patches.unreported_reason(), None);
}

/// Below stikk's JSON floor the section is **unreported**, with stikk's own reason — never an empty
/// list (`C-T2c′`), and never *"prikk does not report it"*, which is false at 0.36–0.38.
#[test]
fn below_the_json_floor_the_patches_section_is_unreported() {
    // The default backend reports prikk 0.30 and an `Unreported` show report.
    let detail = block_detail(
        &NullBackend::supported(),
        Path::new("/repo"),
        "heads/main",
        block("bb", 2, "Normal", 1),
        false,
    )
    .expect("block detail reads");
    assert_eq!(detail.patches, BlockPatches::Unreported(30));
    assert!(detail.patches.rows().is_empty());
    assert_eq!(
        detail.patches.unreported_reason().as_deref(),
        Some("stikk reads a patch's content from prikk 0.39 on; this is 0.30.")
    );
}

/// **Every block's patches are listed, not only the tip's** (RFC 035 §4.1): `show` takes a block id and
/// needs no replay. The tip-only read above it — the replayed state file set — is unchanged.
#[test]
fn an_older_blocks_patches_are_listed_while_its_state_is_not() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_show(stikk_prikk::ShowReport::Listed(vec![shown(PATCH_ONE, 1)]));
    let older = block_detail(
        &backend,
        Path::new("/repo"),
        "heads/main",
        block("aa", 1, "Root", 0),
        false,
    )
    .expect("block detail reads");
    assert_eq!(older.patches.rows().len(), 1, "listed for an older block");
    assert!(
        older.state.listed().is_some(),
        "at 0.51 an older block lists its own files (RFC 037 decision 1)"
    );
}

/// **A failed `show` costs the Patches section and nothing else** (review v1 §2.3).
///
/// The block's metadata and the tip's replayed state file set were readable before this section
/// existed, and a strict reader refusing a report about the *patches* must not take the block away
/// from the reader. The section says what failed, in the error's own words.
#[test]
fn a_failed_show_leaves_the_rest_of_block_detail_intact() {
    let mut row = block("bb", 2, "Normal", 1);
    row.patches = 2;
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_show_refusal(
            "precondition not met: no object bbbb in the object store or the active WAL",
        )
        .with_state(StateFiles {
            target_block: "bb".to_string(),
            files: vec![
                StateFile::file("readme.txt", 0o100_644),
                StateFile::file("src/lib.rs", 0o100_644),
            ],
            total_bytes: 200,
        });

    let detail = block_detail(&backend, Path::new("/repo"), "heads/main", row, true)
        .expect("the block itself still reads");

    // Everything that did not depend on the new read is still there.
    assert_eq!(detail.row.block_id, "bb");
    assert_eq!(detail.row.patches, 2);
    assert_eq!(
        detail.state.listed().map(|state| state.files.len()),
        Some(2),
        "the tip's state file set is unaffected"
    );
    // And the section carries the failure, with prikk's own message inside it (`ER-02`).
    let BlockPatches::Failed(error) = &detail.patches else {
        panic!("expected a carried failure, got {:?}", detail.patches)
    };
    assert!(error.contains("no object bbbb"), "{error}");
    assert!(detail.patches.rows().is_empty());
    let reason = detail
        .patches
        .unreported_reason()
        .expect("a sentence instead of a list");
    assert!(
        reason.starts_with("stikk could not read this block's patches —"),
        "{reason}"
    );
    assert!(reason.contains("no object bbbb"), "{reason}");
}

/// **The unpublished current branch travels beside the merged list, never in it** (RFC 037
/// decision 3). The merge is what RFC 032's *published* test reads, so a branch with no published
/// history entering it would invert that answer (F4).
#[test]
fn list_refs_carries_the_unpublished_current_branch_beside_the_list() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_refs(Vec::new())
        .with_tags(Vec::new())
        .with_unpublished_current(UnpublishedBranch::Branch("heads/main".into()));
    let list = list_refs(&backend, Path::new("/r")).expect("list");
    assert!(list.refs.is_empty(), "nothing is published");
    assert_eq!(list.unpublished_current.name(), Some("heads/main"));
}

/// **Below the band nothing is reported**, and that is not the same as prikk reporting none.
#[test]
fn list_refs_reports_no_branch_below_the_band() {
    let backend = NullBackend::supported().with_refs(Vec::new());
    let list = list_refs(&backend, Path::new("/r")).expect("list");
    assert_eq!(list.unpublished_current, UnpublishedBranch::NotReported);
}

/// **If prikk ever contradicts itself** — naming a branch unpublished *and* listing it — the
/// published entry wins and the extra row is dropped (Handoff B §3). prikk should never do this;
/// the stand-in is how stikk shows what it would do if it did.
#[test]
fn a_branch_that_is_also_published_loses_its_extra_row() {
    let published = RefEntry {
        name: "heads/main".into(),
        id: "0".repeat(64),
        closed: false,
        received: false,
    };
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_refs(vec![published])
        .with_tags(Vec::new())
        .with_unpublished_current(UnpublishedBranch::Branch("heads/main".into()));
    let list = list_refs(&backend, Path::new("/r")).expect("list");
    assert_eq!(list.refs.len(), 1, "the published entry stays");
    assert_eq!(
        list.unpublished_current.name(),
        None,
        "and the row that would have said the opposite is not offered"
    );
}

//! Tests for the history/block-detail operations (design TS-02), via the scripted backend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use stikk_prikk::{BlockRow, History, NullBackend, Orientation, RefEntry, StateFiles};

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

#[test]
fn block_detail_fetches_state_only_for_the_tip() {
    let backend = NullBackend::supported().with_state(StateFiles {
        target_block: "bb".into(),
        files: vec!["readme.txt".into()],
        total_bytes: 12,
    });
    // Tip: state present.
    let tip = block_detail(
        &backend,
        Path::new("/r"),
        "heads/main",
        block("bb", 2, "Normal", 1),
        true,
    )
    .expect("tip detail");
    assert!(tip.is_tip);
    assert_eq!(tip.state.as_ref().map(|s| s.files.len()), Some(1));
    // Non-tip: no state fetched.
    let old = block_detail(
        &backend,
        Path::new("/r"),
        "heads/main",
        block("aa", 1, "Root", 0),
        false,
    )
    .expect("old detail");
    assert!(!old.is_tip);
    assert!(old.state.is_none());
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
    let refs = list_refs(&backend, Path::new("/r")).unwrap();
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
    let refs = list_refs(&backend, Path::new("/r")).unwrap();
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
    let refs = list_refs(&backend, Path::new("/r")).unwrap();
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
            .as_deref(),
        Some("2 patch(es) in the active WAL — not yet sealed · Q: Queue")
    );
    assert_eq!(
        view_with_queue("heads/other", 2, Some("heads/main"))
            .queued_tier()
            .as_deref(),
        Some("the active WAL holds 2 patch(es) for heads/main — not this ref's history · Q: Queue")
    );
    assert_eq!(
        view_with_queue("heads/main", 1, None)
            .queued_tier()
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
        view.queued_tier().as_deref(),
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
        rows[0].summary().contains("(no message reported)"),
        "{}",
        rows[0].summary()
    );
    assert!(
        rows[1].summary().contains("the second patch")
            && rows[1].summary().contains("2 operation(s)"),
        "{}",
        rows[1].summary()
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
    assert!(older.state.is_none(), "prikk replays only to the tip");
}

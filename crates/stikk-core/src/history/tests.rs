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
            queued_patches: 3,
            queued_target: None,
            main_ref_state: Some("rs-bb".into()),
            trailing_partial_wal_bytes: 0,
            active_patch_warning: None,
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

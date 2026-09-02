//! Golden-fixture tests for prikk output parsing (design TS-03).
//!
//! The fixture below is captured verbatim from `prikk status` at the audited revision. A parser
//! change that would misread it fails here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use super::*;

const STATUS_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 0
trailing partial WAL bytes: 0
heads/main RefState: b8586806457aaab97190c7ccfeb3a8c152f9e140613b70cbb09ff2afd8a56c8e
queued patches: 0
status: multi-operation text diff minimization and plugins not yet implemented
";

#[test]
fn parses_a_clean_status() {
    let o = orientation(STATUS_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 0);
    assert_eq!(o.trailing_partial_wal_bytes, 0);
    assert_eq!(
        o.main_ref_state.as_deref(),
        Some("b8586806457aaab97190c7ccfeb3a8c152f9e140613b70cbb09ff2afd8a56c8e")
    );
}

#[test]
fn parses_queued_and_partial_counts() {
    let text = "\
queued patches: 3
trailing partial WAL bytes: 7
heads/main RefState: <none>
";
    let o = orientation(text).expect("parses");
    assert_eq!(o.queued_patches, 3);
    assert_eq!(o.trailing_partial_wal_bytes, 7);
    assert_eq!(o.main_ref_state, None);
}

#[test]
fn refuses_rather_than_guesses_on_a_missing_field() {
    // UD-02: an unrecognized shape is an environment fault, never a fabricated default.
    let text = "some unexpected prikk output with no queued patches line\n";
    let err = orientation(text).expect_err("must refuse");
    assert_eq!(err.class(), "environment");
}

#[test]
fn refuses_a_non_numeric_count() {
    let text = "queued patches: lots\ntrailing partial WAL bytes: 0\n";
    assert_eq!(orientation(text).unwrap_err().class(), "environment");
}

// Captured verbatim from `prikk log` (a two-block repo) at prikk 0.27.1.
const LOG_FIXTURE: &str = "\
history repository: /tmp/repo/.prikk
ref: heads/main
block 76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0
  ref-state: 3f5f846e39f1f1f841faf7d673b98337db9783ca24eba1c210a9f28897330fbf
  update-seq: 2
  kind: Normal
  rollback-block: false
  parents: 1
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  previous-ref-state: 3b0d20d04949092224ed973c08af2777f495121d8b0fcef3bcc5a099c46bf127
block 99ba03d969e4fa6525ff83aa00ea056a4b581011409563d1713ea75651f4e398
  ref-state: 3b0d20d04949092224ed973c08af2777f495121d8b0fcef3bcc5a099c46bf127
  update-seq: 1
  kind: Root
  rollback-block: false
  parents: 0
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  previous-ref-state: <none>
";

#[test]
fn parses_a_two_block_lineage_tip_first() {
    let h = history(LOG_FIXTURE).expect("log parses");
    assert_eq!(h.reff, "heads/main");
    assert_eq!(h.blocks.len(), 2);
    let tip = &h.blocks[0];
    assert_eq!(tip.update_seq, 2);
    assert_eq!(tip.kind, "Normal");
    assert_eq!(tip.parents, 1);
    assert_eq!(tip.patches, 1);
    assert!(!tip.rollback_block);
    assert!(tip.previous_ref_state.is_some());
    let root = &h.blocks[1];
    assert_eq!(root.kind, "Root");
    assert_eq!(root.parents, 0);
    assert_eq!(root.previous_ref_state, None); // <none>
}

#[test]
fn history_refuses_on_a_missing_field() {
    // A truncated block (missing update-seq) is an environment fault, never a fabricated value.
    let broken = "ref: heads/main\nblock abc\n  ref-state: def\n  kind: Root\n";
    assert_eq!(history(broken).unwrap_err().class(), "environment");
}

#[test]
fn history_refuses_when_ref_line_absent() {
    assert_eq!(
        history("some unrelated output\n").unwrap_err().class(),
        "environment"
    );
}

// Captured verbatim from `prikk checkout --patch-plan`.
const PATCH_PLAN_FIXTURE: &str = "\
patch replay plan repository: /tmp/repo/.prikk
ref: heads/main
target block: 76cee1dc985406f931a2bbeb217653e509183c2d5280fdf933b5ebac78f4cbc0
blocks replayed: 2
patches replayed: 2
operations applied: 2
result files: 1
result content bytes: 37
  file: readme.txt
note: this replays CreateFile/DeleteNode/EditText/ReplaceBinary/ChangePerm; renames remain later
";

#[test]
fn parses_the_tip_state_file_set() {
    let s = state_files(PATCH_PLAN_FIXTURE).expect("patch plan parses");
    assert!(s.target_block.starts_with("76cee1dc"));
    assert_eq!(s.total_bytes, 37);
    assert_eq!(s.files, vec!["readme.txt".to_string()]);
}

#[test]
fn state_files_refuses_without_a_target_block() {
    assert_eq!(
        state_files("result content bytes: 0\n")
            .unwrap_err()
            .class(),
        "environment"
    );
}

// Captured verbatim from `prikk branch list --all` (branches, a closed branch, and a tag).
const BRANCH_LIST_FIXTURE: &str = "\
heads/main 7e2434d04c47960d2d039c98d287223561e725e265214f0608be24ff96bcde66
heads/oldwork 75a22546d8acb79663ba2754b5eaeb46aa0363ed54f9e718fad5ed1c620cc241 (closed)
heads/topic 7edf43c937bb63501705b0961c10e7f6f3dc6bf342503a9192de1e65c98058e4
tags/v1 32276e1564f170dfd405b32aab14ffe749c33597e0b2006e553c5fd6141802dd
";

#[test]
fn parses_refs_with_markers_and_kinds() {
    let refs = refs(BRANCH_LIST_FIXTURE).expect("branch list parses");
    assert_eq!(refs.len(), 4);
    assert_eq!(refs[0].name, "heads/main");
    assert!(!refs[0].closed && !refs[0].received && !refs[0].is_tag());
    let oldwork = refs.iter().find(|r| r.name == "heads/oldwork").unwrap();
    assert!(oldwork.closed);
    let tag = refs.iter().find(|r| r.name == "tags/v1").unwrap();
    assert!(tag.is_tag());
}

#[test]
fn refs_skips_blank_lines() {
    assert_eq!(refs("\n\n").expect("empty ok").len(), 0);
}

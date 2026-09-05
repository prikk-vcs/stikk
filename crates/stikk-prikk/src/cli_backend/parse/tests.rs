//! Golden-fixture tests for prikk output parsing (design TS-03; RFC 009 §0; RFC 012 F-e).
//!
//! Every fixture below is captured **verbatim from a real `prikk` run** — reproduced against a live
//! prikk 0.30.0 binary on 2026-09-04 while implementing RFC 009 — never composed by hand or by
//! analogy. **Re-verified against a live prikk 0.31.0 binary on 2026-09-05** (RFC 012 F-e): every
//! fixture below was re-captured from equivalent probe repositories and diffed byte-for-byte against
//! the committed 0.30.0 text; identical in every case (see the review request for the full command
//! transcript). No fixture text changed as a result — only the provenance comments, to record that the
//! shape has now been checked against both versions. `every_fixture_constant_carries_a_provenance_comment`
//! enforces the shape mechanically: a fixture constant with no provenance comment naming a prikk
//! version fails the build.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

// Captured verbatim from `prikk status` on a freshly-`init`ed repository, prikk 0.30.0. Re-verified
// byte-identical against prikk 0.31.0 on 2026-09-05 (RFC 012 F-e).
const STATUS_EMPTY_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 0
trailing partial WAL bytes: 0
heads/main RefState: <not published>
queued patches: 0
status: multi-operation text diff minimization and plugins not yet implemented
";

// Captured verbatim from `prikk status` on a repository with one queued patch (unpublished
// `heads/main`), prikk 0.30.0. Re-verified byte-identical against prikk 0.31.0 on 2026-09-05
// (RFC 012 F-e).
const STATUS_QUEUED_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 1
trailing partial WAL bytes: 0
heads/main RefState: <not published>
queued patches: 1 targeting heads/main
status: multi-operation text diff minimization and plugins not yet implemented
";

// Captured verbatim from `prikk status` on a repository with a sealed `heads/main` and no queued
// work, prikk 0.30.0. Re-verified byte-identical against prikk 0.31.0 on 2026-09-05 (RFC 012 F-e).
const STATUS_CLEAN_PUBLISHED_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 0
trailing partial WAL bytes: 0
heads/main RefState: 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
queued patches: 0
status: multi-operation text diff minimization and plugins not yet implemented
";

// Captured verbatim from `prikk status` with `PRIKK_ACTIVE_PATCH_WARN=1 PRIKK_ACTIVE_PATCH_LIMIT=100`
// on a repository with one queued patch, prikk 0.30.0 — proves the parser tolerates the active-patch
// threshold `warning:` line prikk inserts between `queued patches:` and `status:` (not itself parsed
// this increment; RFC 009 open question, ruled deferred to the commit increment). Re-verified
// byte-identical against prikk 0.31.0 on 2026-09-05 with the same two env vars (RFC 012 F-e) — note a
// *smaller* `PRIKK_ACTIVE_PATCH_LIMIT` produces a differently-worded hard-limit warning instead of this
// one; that shape is likewise not parsed, so it needed no fixture of its own.
const STATUS_QUEUED_WITH_WARNING_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 1
trailing partial WAL bytes: 0
heads/main RefState: 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
queued patches: 1 targeting heads/main
warning: active patches (1) at or above the recommended threshold (1); consider running `prikk seal`
status: multi-operation text diff minimization and plugins not yet implemented
";

#[test]
fn parses_a_clean_empty_status() {
    let o = orientation(STATUS_EMPTY_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 0);
    assert_eq!(o.queued_target, None);
    assert_eq!(o.trailing_partial_wal_bytes, 0);
    // RFC 009 F2: `<not published>` must not become a fabricated object id.
    assert_eq!(o.main_ref_state, None);
}

#[test]
fn parses_a_clean_published_status() {
    let o = orientation(STATUS_CLEAN_PUBLISHED_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 0);
    assert_eq!(o.queued_target, None);
    assert_eq!(
        o.main_ref_state.as_deref(),
        Some("0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27")
    );
}

#[test]
fn parses_queued_patches_and_their_target_ref() {
    // RFC 009 F1: the shipped parser refused this shape outright — the defect that made Orientation
    // fail on any repository anyone had committed to, at every prikk version stikk claimed to support.
    let o = orientation(STATUS_QUEUED_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 1);
    assert_eq!(o.queued_target.as_deref(), Some("heads/main"));
    assert_eq!(o.main_ref_state, None);
}

#[test]
fn tolerates_an_active_patch_threshold_warning_line() {
    let o = orientation(STATUS_QUEUED_WITH_WARNING_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 1);
    assert_eq!(o.queued_target.as_deref(), Some("heads/main"));
}

#[test]
fn refuses_a_control_character_bearing_queued_target() {
    // RFC 012 F-d: the "targeting <ref>" value is validated through `RefName::parse` too.
    let text = "queued patches: 1 targeting heads/ma\x07in\ntrailing partial WAL bytes: 0\n";
    assert_eq!(orientation(text).unwrap_err().class(), "environment");
}

#[test]
fn refuses_an_unrecognized_queued_tail() {
    // UD-02: a trailing shape that is neither bare nor `targeting <ref>` refuses rather than guesses.
    let text = "queued patches: 1 wat heads/main\ntrailing partial WAL bytes: 0\n";
    assert_eq!(orientation(text).unwrap_err().class(), "environment");
}

#[test]
fn queued_target_is_none_for_unreadable_active_ref_metadata() {
    // prikk emits these sentinels (not a ref name) when its own active-ref metadata is unreadable
    // (`prikk-cli/src/main.rs`, `ActiveRefMetadata::Missing`/`Invalid`); mapping either to a fabricated
    // ref name would be worse than showing no target at all.
    for sentinel in ["<missing metadata>", "<malformed metadata>"] {
        let text =
            format!("queued patches: 2 targeting {sentinel}\ntrailing partial WAL bytes: 0\n");
        let o = orientation(&text).expect("parses");
        assert_eq!(o.queued_patches, 2);
        assert_eq!(
            o.queued_target, None,
            "sentinel {sentinel:?} must map to None"
        );
    }
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

#[test]
fn refuses_an_unrecognized_ref_state_sentinel() {
    // RFC 009 F2: stikk assumed exactly one sentinel (`<none>`); a value that is neither a known
    // sentinel nor a valid object id must refuse, not pass through as if it were an identity.
    let text = "\
queued patches: 0
trailing partial WAL bytes: 0
heads/main RefState: <a-future-sentinel-stikk-does-not-know>
";
    assert_eq!(orientation(text).unwrap_err().class(), "environment");
}

// Captured verbatim from `prikk log --ref heads/main` on a repository with two sealed blocks, prikk
// 0.30.0. Re-verified byte-identical against prikk 0.31.0 on 2026-09-05 (RFC 012 F-e).
const LOG_FIXTURE: &str = "\
history repository: /tmp/repo/.prikk
ref: heads/main
block 7c99ec96996ca2722134331eadec281f435f29171a71dcad1885611e6053e60b
  ref-state: 6e94160c800338bece6a8e7722b7eeb0fdefb74078af11969ffda2122396d8ac
  update-seq: 2
  kind: Normal
  rollback-block: false
  parents: 1
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  previous-ref-state: 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
block c87c8a3c5541c1ec07c9c197a33796e96de8564845695e29e1dd476938d6fb60
  ref-state: 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
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
fn history_refuses_a_control_character_bearing_ref_name() {
    // RFC 012 F-d: `history`'s `ref:` field is now validated through `RefName::parse` (INV-9, UD-02) —
    // a shape prikk would never actually emit, but stikk must refuse rather than pass it through.
    let text = "ref: heads/ma\x07in\nblock abc\n  ref-state: def\n  update-seq: 1\n  kind: Root\n  \
                rollback-block: false\n  parents: 0\n  patches: 1\n  rollback-patches: 0\n  \
                required-attestations: 0\n  previous-ref-state: <none>\n";
    assert_eq!(history(text).unwrap_err().class(), "environment");
}

#[test]
fn history_refuses_when_ref_line_absent() {
    assert_eq!(
        history("some unrelated output\n").unwrap_err().class(),
        "environment"
    );
}

#[test]
fn history_refuses_on_an_unrecognized_previous_ref_state() {
    // RFC 009 F2 applies to `log`'s sentinel too: only `<none>` is recognized here.
    let broken = "\
ref: heads/main
block abc
  ref-state: def
  update-seq: 1
  kind: Root
  rollback-block: false
  parents: 0
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  previous-ref-state: <a-future-sentinel>
";
    assert_eq!(history(broken).unwrap_err().class(), "environment");
}

// Captured verbatim from `prikk checkout --patch-plan --ref heads/main` on the same two-block
// repository as `LOG_FIXTURE`, prikk 0.30.0. Re-verified byte-identical against prikk 0.31.0 on
// 2026-09-05 (RFC 012 F-e).
const PATCH_PLAN_FIXTURE: &str = "\
patch replay plan repository: /tmp/repo/.prikk
ref: heads/main
target block: 7c99ec96996ca2722134331eadec281f435f29171a71dcad1885611e6053e60b
blocks replayed: 2
patches replayed: 2
operations applied: 3
result files: 2
result content bytes: 30
  file: main.rs
  file: readme.txt
note: this replays CreateFile/DeleteNode/EditText/ReplaceBinary/ChangePerm; renames, conflicts, and full patch algebra remain later increments
";

#[test]
fn parses_the_tip_state_file_set() {
    let s = state_files(PATCH_PLAN_FIXTURE).expect("patch plan parses");
    assert!(s.target_block.starts_with("7c99ec96"));
    assert_eq!(s.total_bytes, 30);
    assert_eq!(
        s.files,
        vec!["main.rs".to_string(), "readme.txt".to_string()]
    );
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

// Captured verbatim from `prikk branch list --all` on a freshly-`init`ed repository, prikk 0.30.0.
// Re-verified byte-identical against prikk 0.31.0 on 2026-09-05 (RFC 012 F-e).
const BRANCH_LIST_EMPTY_FIXTURE: &str = "no branches\n";

// Captured verbatim from `prikk branch list --all` on a repository with a sealed `heads/main`, a
// second branch created from it and then closed, and a received ref imported from a bundle exported by
// a peer repository, prikk 0.30.0. Re-verified byte-identical (with fresh object ids) against prikk
// 0.31.0 on 2026-09-05 (RFC 012 F-e). RFC 009 F3's claim that "`branch list` cannot emit a tag ... there
// never can be one from this command" is **corrected** by that same re-verification: this repository
// simply had no tag yet, so the absence proved nothing either way. `prikk-cli`'s `branch list` reads
// `RefStore::list_ref_pointers()` (`crates/prikk-cli/src/branch.rs`), which does not filter by ref
// namespace at all — a tag created in the same repository appears in `branch list --all`'s output too,
// confirmed live against both 0.30.0 and 0.31.0 with a tag actually present. This is not documented
// prikk behavior (`tag list` is prikk's own stated, stable way to list tags) and stikk does not rely on
// it — `list_refs` (RFC 012 §7) merges `tag list`'s results in and de-duplicates by name defensively,
// precisely because `branch list --all` cannot be assumed either to include or exclude tags going
// forward. See the review request for the full transcript.
const BRANCH_LIST_FIXTURE: &str = "\
heads/main 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
heads/secondary 7ed0e6312169663946e5edd18aa1e5e1ecc994d43e489c1b77e77c075ddc05ca (closed)
remotes/heads/main 37b1a91bc4bf4c82c35c55f53e13ca65c071fba8c8c62b0a82378d229574a554 (received)
";

#[test]
fn empty_branch_list_yields_an_empty_list_not_a_phantom_ref() {
    // RFC 009 F3's regression test: `refs` used to accept any two-token line, so this became
    // `RefEntry { name: "no", id: "branches" }`. It is the one parser that refused on nothing.
    let refs = refs(BRANCH_LIST_EMPTY_FIXTURE).expect("empty list parses");
    assert!(refs.is_empty());
    assert!(!refs.iter().any(|r| r.name == "no"));
}

#[test]
fn empty_tag_list_line_is_also_recognized() {
    // `tag list` prints `no tags`; `refs` recognizes it too against the day that read is added
    // (RFC 009 F3 §"the tag gap").
    assert!(refs("no tags\n").expect("parses").is_empty());
}

#[test]
fn parses_refs_with_open_closed_and_received_markers() {
    let refs = refs(BRANCH_LIST_FIXTURE).expect("branch list parses");
    assert_eq!(refs.len(), 3);
    let main = refs.iter().find(|r| r.name == "heads/main").unwrap();
    assert!(!main.closed && !main.received && !main.is_tag());
    let secondary = refs.iter().find(|r| r.name == "heads/secondary").unwrap();
    assert!(secondary.closed && !secondary.received);
    let received = refs
        .iter()
        .find(|r| r.name == "remotes/heads/main")
        .unwrap();
    assert!(received.received && !received.closed);
}

#[test]
fn is_tag_is_correct_for_a_tag_shaped_name_though_branch_list_cannot_source_one() {
    // `branch list` never emits a `tags/…` line (RFC 009 F3), so `is_tag()` has no fixture to parse it
    // from — but the predicate itself is still correct, and is exercised directly here rather than via
    // an invented capture.
    let tag = RefEntry {
        name: "tags/v1".to_string(),
        id: "0".repeat(64),
        closed: false,
        received: false,
    };
    assert!(tag.is_tag());
}

#[test]
fn refs_skips_blank_lines() {
    assert_eq!(refs("\n\n").expect("empty ok").len(), 0);
}

#[test]
fn refs_refuses_a_line_whose_second_token_is_not_an_object_id() {
    // The general fix behind F3: any two-token line that is not `no branches`/`no tags` and whose
    // second token is not a 64-hex id refuses, rather than becoming a phantom ref.
    assert_eq!(
        refs("garbage line here\n").unwrap_err().class(),
        "environment"
    );
}

#[test]
fn refs_refuses_a_control_character_bearing_ref_name() {
    // RFC 012 F-d.
    let text = format!("heads/ma\x07in {}\n", "0".repeat(64));
    assert_eq!(refs(&text).unwrap_err().class(), "environment");
}

#[test]
fn refs_refuses_an_unrecognized_marker() {
    let text = format!("heads/main {} (archived)\n", "0".repeat(64));
    assert_eq!(refs(&text).unwrap_err().class(), "environment");
}

// Captured verbatim from `prikk tag list` on a freshly-`init`ed repository, prikk 0.31.0 (RFC 012
// FR-014 — this read is new this increment, so there is no earlier-version baseline to check against).
const TAG_LIST_EMPTY_FIXTURE: &str = "no tags\n";

// Captured verbatim from `prikk tag list` on a repository with two tags pointing at the same block,
// prikk 0.31.0.
const TAG_LIST_FIXTURE: &str = "\
tags/v1 ecf293dfb5953f643fde0dd4ab5cb1e4f8790e44205b2e9328fef418369907a8
tags/v2 ecf293dfb5953f643fde0dd4ab5cb1e4f8790e44205b2e9328fef418369907a8
";

#[test]
fn empty_tag_list_yields_an_empty_list() {
    assert!(
        tags(TAG_LIST_EMPTY_FIXTURE)
            .expect("empty list parses")
            .is_empty()
    );
}

#[test]
fn parses_tags_with_no_markers() {
    let out = tags(TAG_LIST_FIXTURE).expect("tag list parses");
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].name, "tags/v1");
    assert_eq!(out[1].name, "tags/v2");
    for entry in &out {
        assert!(!entry.closed && !entry.received);
        assert!(entry.is_tag());
    }
}

#[test]
fn tags_refuses_a_line_with_a_trailing_marker() {
    // Unlike `branch list`, `tag list` has no marker vocabulary at all (RFC 012 FR-014) — any trailing
    // token refuses rather than guessing at a meaning that does not exist.
    let text = format!("tags/v1 {} (closed)\n", "0".repeat(64));
    assert_eq!(tags(&text).unwrap_err().class(), "environment");
}

#[test]
fn tags_refuses_a_control_character_bearing_name() {
    // RFC 012 F-d.
    let text = format!("tags/v\x071 {}\n", "0".repeat(64));
    assert_eq!(tags(&text).unwrap_err().class(), "environment");
}

#[test]
fn tags_refuses_an_invalid_target_block_id() {
    let text = "tags/v1 not-a-valid-id\n";
    assert_eq!(tags(text).unwrap_err().class(), "environment");
}

#[test]
fn tags_skips_blank_lines() {
    assert_eq!(tags("\n\n").expect("empty ok").len(), 0);
}

// Captured verbatim from `prikk worktree-status --ref heads/main` on a clean two-file repository,
// prikk 0.30.0. Re-verified byte-identical against prikk 0.31.0 on 2026-09-05 (RFC 012 F-e).
const WORKTREE_CLEAN_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 2
unchanged files: 2
missing files: 0
modified files: 0
untracked files: 0
unsupported paths: 0
worktree: clean against baseline
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

// Captured verbatim from `prikk worktree-status --ref heads/main` (stdout; the `error: worktree has
// changes against the baseline` line is on stderr and is *not* part of the report) after modifying,
// deleting, and adding a file, prikk 0.30.0. Re-verified byte-shape-identical against prikk 0.31.0 on
// 2026-09-05 (RFC 012 F-e) — the three entry kinds, their note text, and the headline all matched; only
// the (irrelevant) file-ordering and object ids differed between the two probe repositories.
const WORKTREE_DIRTY_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 2
unchanged files: 0
missing files: 1
modified files: 1
untracked files: 1
unsupported paths: 0
worktree: changed against baseline
  missing main.rs — tracked file is absent from the worktree
  untracked notes.tmp — worktree file is not in the baseline
  modified readme.txt — tracked file bytes differ from the baseline
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

// Captured verbatim from `prikk worktree-status --ref heads/other`, prikk 0.30.0, on a repository whose
// active WAL holds a patch queued for `heads/main` — the F4 reproduction (stdout; the dirty-exit
// `error:` line is on stderr and is not part of the report). Re-verified against prikk 0.31.0 on
// 2026-09-05 (RFC 012 F-e): the queued-elsewhere note text is byte-identical, word for word.
const WORKTREE_QUEUED_ELSEWHERE_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/other
tracked files: 0
unchanged files: 0
missing files: 0
modified files: 0
untracked files: 2
unsupported paths: 0
worktree: changed against baseline
  untracked main.rs — worktree file is not in the baseline
  untracked readme.txt — worktree file is not in the baseline
note: the active WAL has queued (unsealed) patches for heads/main, not heads/other -- that is real, committed work, not shown above; any \"untracked\" file here may be exactly that work seen from this ref's own baseline, so do not delete based on this report alone (see `prikk status`)
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

#[test]
fn parses_a_clean_worktree() {
    let s = worktree_status(WORKTREE_CLEAN_FIXTURE).expect("clean parses");
    assert!(s.clean);
    assert_eq!(s.reff, "heads/main");
    assert_eq!(s.tracked, 2);
    assert_eq!(s.unchanged, 2);
    assert!(s.entries.is_empty());
    assert_eq!(s.queued_elsewhere, None);
}

#[test]
fn parses_a_dirty_worktree_with_all_kinds() {
    let s = worktree_status(WORKTREE_DIRTY_FIXTURE).expect("dirty parses");
    assert!(!s.clean);
    assert_eq!(s.missing, 1);
    assert_eq!(s.modified, 1);
    assert_eq!(s.untracked, 1);
    assert_eq!(s.entries.len(), 3);
    let modified = s.entries.iter().find(|e| e.kind == "modified").unwrap();
    assert_eq!(modified.path, "readme.txt");
    assert!(modified.note.contains("bytes differ"));
    let missing = s.entries.iter().find(|e| e.kind == "missing").unwrap();
    assert_eq!(missing.path, "main.rs");
    // The generic "use `prikk commit`" note is not the queued-elsewhere warning.
    assert_eq!(s.queued_elsewhere, None);
}

#[test]
fn carries_the_queued_elsewhere_warning_verbatim() {
    // RFC 009 F4 — the acceptance-critical fix: this warning exists specifically so a front-end cannot
    // mislead a user into deleting real, committed-but-unsealed work. The note must be transported
    // byte-identical (ER-02), never paraphrased.
    let s = worktree_status(WORKTREE_QUEUED_ELSEWHERE_FIXTURE).expect("parses");
    assert!(!s.clean);
    assert_eq!(s.untracked, 2);
    let note = s.queued_elsewhere.expect("warning must be captured");
    assert!(note.starts_with("note: the active WAL has queued (unsealed) patches for heads/main"));
    assert!(note.contains("do not delete based on this report alone"));
    // Byte-identical to prikk's own line (modulo the leading/trailing whitespace `field`-style readers
    // already trim elsewhere in this module) — no stikk paraphrase anywhere in it.
    assert!(
        WORKTREE_QUEUED_ELSEWHERE_FIXTURE.contains(&note),
        "the captured note must appear verbatim in the fixture"
    );
}

#[test]
fn worktree_entry_preserves_a_path_with_spaces() {
    let text = "\
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 0
modified files: 1
untracked files: 0
unsupported paths: 0
worktree: changed against baseline
  modified my docs/read me.txt — tracked file bytes differ from the baseline
";
    let s = worktree_status(text).expect("parses");
    assert_eq!(s.entries[0].path, "my docs/read me.txt");
}

#[test]
fn worktree_status_refuses_a_control_character_bearing_ref_name() {
    // RFC 012 F-d.
    let text = "ref: heads/ma\x07in\nworktree: clean against baseline\ntracked files: 0\nunchanged \
                files: 0\nmissing files: 0\nmodified files: 0\nuntracked files: 0\nunsupported \
                paths: 0\n";
    assert_eq!(worktree_status(text).unwrap_err().class(), "environment");
}

#[test]
fn worktree_status_refuses_without_the_headline() {
    // UD-02: no `worktree:` headline ⇒ not a worktree-status report ⇒ environment fault (the caller
    // then treats the outcome as a real failure rather than a status).
    let text = "some unrelated prikk output\n";
    assert_eq!(worktree_status(text).unwrap_err().class(), "environment");
}

#[test]
fn worktree_status_refuses_on_a_missing_count() {
    let text = "ref: heads/main\nworktree: clean against baseline\n";
    assert_eq!(worktree_status(text).unwrap_err().class(), "environment");
}

/// RFC 009 §0's rule, enforced mechanically: every fixture constant in this file must carry a
/// provenance comment block (one or more contiguous `//` lines immediately above it) naming a prikk
/// version. A hand-written fixture with no such comment — or, worse, a false one — is exactly the
/// defect class F1 was.
#[test]
fn every_fixture_constant_carries_a_provenance_comment() {
    let source = include_str!("tests.rs");
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("const ") || !trimmed.contains("_FIXTURE") {
            continue;
        }
        // Walk upward over the contiguous block of `//` comment lines directly above this constant
        // (skipping none — a blank line or non-comment line ends the block) and check the block as a
        // whole, since a real provenance note often wraps across several lines.
        let mut block = String::new();
        let mut j = i;
        while j > 0 {
            let candidate = lines[j - 1].trim_start();
            if !candidate.starts_with("//") {
                break;
            }
            block.push_str(candidate);
            block.push(' ');
            j -= 1;
        }
        assert!(
            !block.is_empty() && block.to_ascii_lowercase().contains("prikk"),
            "fixture on line {} has no provenance comment naming prikk: {line:?}",
            i + 1
        );
        assert!(
            block.chars().any(|c| c.is_ascii_digit()),
            "provenance comment above line {} does not name a version: {block:?}",
            i + 1
        );
    }
}

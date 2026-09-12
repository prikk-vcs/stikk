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
//!
//! **Re-verified a third time against a real released prikk 0.32.0 binary on 2026-09-06** (RFC 015,
//! the ceiling raise) — see the review request for the full command transcript. Every shape re-captured
//! byte-identical to the 0.30.0/0.31.0 text **except `log`**, which now emits a `patch <id>: <message>`
//! line for every patch that carries one (RFC 015 F1) — new fixtures below capture that shape, and the
//! pre-existing `LOG_FIXTURE` (still valid: its patches predate the feature) is unchanged. Also captured:
//! the straddling case (RFC 015 F4, one block holding both a pre-0.32 and a 0.32 patch) and the
//! bundle-decode skew refusal (RFC 015 F5).
//!
//! **Re-verified a fourth time against a real released prikk 0.33.0 binary on 2026-09-06** (RFC 017,
//! the classifier-provenance re-baseline — see that review request's C2 for the full transcript and the
//! independent source-diff check backing it). `status`, `log` (all three shapes), `commit`, `branch
//! list`, `tag list`, `worktree-status`, and `checkout --patch-plan` were all re-run against the same
//! probe repositories these fixtures were originally captured from; every shape is **unchanged from
//! 0.32.0**, byte-for-byte. This is not merely "nothing broke": `git diff 0.32.0..0.33.0` touches
//! exactly `prikk-cli`'s `branch.rs` (an internal error-match arm, RFC 017 F1, no output text),
//! `commands.rs`/`main.rs` (registering the new `key`/`setup` subcommands stikk never calls, `C-I1e`),
//! the new `key.rs`/`setup.rs` themselves, `prikk-error` (the `Precondition` variant), and two
//! `prikk-store` sites for messages this classifier already accounts for (RFC 017 F1/F4) — no file
//! implementing `status`, `log`, `branch list`, `tag list`, `worktree-status`, or `checkout
//! --patch-plan`'s own output changed at all between the two tags. Fixture text and provenance comments
//! below remain pinned to their original capture (0.30.0/0.31.0/0.32.0, per fixture) rather than
//! re-stamped to 0.33.0 — re-verified unchanged is a different, and weaker, claim than re-captured, and
//! this paragraph is where that distinction is recorded.
//!
//! **Re-verified a fifth time against a real released prikk 0.38.0 binary on 2026-09-12** (RFC 021, the
//! 0.38 re-baseline — the widest jump this project has made: five prikk releases, 0.34 through 0.38,
//! against a ceiling of 0.33). Every surface below was re-captured from an equivalent probe repository
//! built with `prikk setup`, then `commit`/`seal`/`tag create tags/v2`/`branch create`+`close`:
//!
//! - **Unchanged, byte-for-byte:** `status` (all three shapes — empty, queued, clean-published), `log`
//!   (including the `patch <id>: <message>` line), `commit` (one `note:` line, matching the ≥ 0.32
//!   fixture — the message's-fate note stays gone), `seal`, `branch list --all` (open, closed), `tag
//!   list` (empty and populated), and `checkout --patch-plan`. Letter 005's reply claimed no shape
//!   change 0.33 → 0.38 for these; that claim is now **verified rather than taken**, and these fixtures
//!   keep their original provenance rather than being re-stamped to 0.38.0 — the same distinction the
//!   paragraph above draws.
//! - **Changed:** `worktree-status` alone. 0.38 prints `live rename declarations: N` **unconditionally**
//!   — on a clean worktree too, where it reads `0`. Both new shapes are captured below as their own
//!   fixtures rather than folded into the existing ones, which remain the pre-0.38 regression suite.
//!   The dirty-with-a-rename shape is what made stikk fabricate an entry (RFC 021 F0, fixed in
//!   `parse::worktree_status` before this re-baseline began).
//!
//! Separately re-read at the 0.38.0 tag, and recorded in `classify/tests.rs` rather than here: prikk
//! 0.35 reclassified six `lock conflict:` sites to `precondition not met:` (its own RFC 132 part 2),
//! leaving exactly the four genuine locks stikk's classifier had already narrowed itself to by hand in
//! RFC 017 F4.

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
// on a repository with one queued patch, prikk 0.30.0 — the active-patch threshold `warning:` line
// prikk inserts between `queued patches:` and `status:` (design `C-D2a`; parsed as of RFC 014 F5, the
// commit increment — RFC 009 had deferred it). Re-verified byte-identical against prikk 0.31.0 on
// 2026-09-05 (RFC 012 F-e) and again against a real prikk 0.31.1 binary on 2026-09-06 (RFC 014) with
// the same two env vars.
const STATUS_QUEUED_WITH_WARNING_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 1
trailing partial WAL bytes: 0
heads/main RefState: 0ea4951e3c16277436be45c729885d25d5d92c2073a0c1585e793af60c6d9e27
queued patches: 1 targeting heads/main
warning: active patches (1) at or above the recommended threshold (1); consider running `prikk seal`
status: multi-operation text diff minimization and plugins not yet implemented
";

// Captured verbatim against a real prikk 0.31.1 binary on 2026-09-06 with
// `PRIKK_ACTIVE_PATCH_WARN=1 PRIKK_ACTIVE_PATCH_LIMIT=1` (RFC 014 F5) — the differently-worded
// hard-limit variant `STATUS_QUEUED_WITH_WARNING_FIXTURE`'s own comment predicted but did not capture.
// Distinguished from the warn-threshold wording only by its own text (`configured hard limit` vs
// `recommended threshold`), never by stikk re-deriving the thresholds.
const STATUS_QUEUED_AT_HARD_LIMIT_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 1
trailing partial WAL bytes: 0
heads/main RefState: <not published>
queued patches: 1 targeting heads/main
warning: active patches (1) at or above the configured hard limit (1); commit is blocked until you run `prikk seal`
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
fn captures_the_active_patch_warn_threshold_line_verbatim() {
    // RFC 014 F5 / C-D2a: surfaced in the preview before any refusal, in prikk's own words.
    let o = orientation(STATUS_QUEUED_WITH_WARNING_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 1);
    assert_eq!(o.queued_target.as_deref(), Some("heads/main"));
    let warning = o.active_patch_warning.expect("warning must be captured");
    assert!(
        warning.starts_with("warning: active patches (1) at or above the recommended threshold")
    );
    assert!(STATUS_QUEUED_WITH_WARNING_FIXTURE.contains(&warning));
}

#[test]
fn captures_the_active_patch_hard_limit_line_verbatim_distinguishable_from_the_warn_wording() {
    let o = orientation(STATUS_QUEUED_AT_HARD_LIMIT_FIXTURE).expect("status parses");
    let warning = o.active_patch_warning.expect("warning must be captured");
    assert!(warning.contains("configured hard limit"));
    assert!(!warning.contains("recommended threshold"));
}

#[test]
fn no_active_patch_warning_when_prikk_did_not_print_one() {
    let o = orientation(STATUS_QUEUED_FIXTURE).expect("status parses");
    assert_eq!(o.active_patch_warning, None);
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

// Captured verbatim from `prikk log --ref heads/main` on a repository with one block sealing two
// patches authored by prikk 0.32.0, on 2026-09-06 (RFC 015 F1). Messages deliberately include colons
// ("first message: with a colon", "second: message") to exercise §3's "first `\": \"`" rule.
const LOG_FIXTURE_WITH_MESSAGES: &str = "\
history repository: /tmp/sample/.prikk
ref: heads/main
block 5633ed28be156ea46929e6450764fe290354773f2e7072f0a59489a0e714d86b
  ref-state: c1c6ae60e26dfbb92f957bc75d99310624282eed0cc7bb4802348d18eb7675f3
  update-seq: 1
  kind: Root
  rollback-block: false
  parents: 0
  patches: 2
  rollback-patches: 0
  required-attestations: 0
  patch 2a4dfcf49d9ca967a7cdf462e8d43d09a475a687c1c46c253a00851c12e5ec55: first message: with a colon
  patch 5f20740a94f8e9cc88ef5205838babf643dc19b176e1b1b51b6dd43b4b5112ee: second: message
  previous-ref-state: <none>
";

// Captured verbatim from `prikk log --ref heads/main` on 2026-09-06 (RFC 015 F4, the honesty-critical
// finding) — a repository where one patch was authored by a real prikk 0.31.1 binary (no message
// stored) and a second by a real prikk 0.32.0 binary (message stored), both queued together and sealed
// by 0.32.0 into **one block**. `patches: 2`, but exactly **one** `patch …:` line — the pre-0.32 patch
// contributes no line at all. This is the disagreement Block detail must never render silently.
const LOG_FIXTURE_STRADDLING: &str = "\
history repository: /tmp/sample/.prikk
ref: heads/main
block 14834f8e3c5488f860908700d2ae5cbd596d3b14f2e1b4c1de4ac3ec64d16548
  ref-state: d5dd45f6710c67bfb7e7ba6a8f9a6814b0d4b62a07f05dc8f22c5712dddae592
  update-seq: 1
  kind: Root
  rollback-block: false
  parents: 0
  patches: 2
  rollback-patches: 0
  required-attestations: 0
  patch ed0c918b28c06cada620b7800ab0ba83a845ddd4f13378e61be92aee4ab3ae24: post-0.32 patch, message stored
  previous-ref-state: <none>
";

// Captured verbatim from `prikk log --ref heads/main` on 2026-09-06 (RFC 015 §3) — a message chosen to
// resemble a field label (`kind: Normal, parents: 0`), to prove it cannot forge one: the line still
// starts with the fixed `patch ` prefix, which no field label shares.
const LOG_FIXTURE_LABEL_LIKE_MESSAGE: &str = "\
history repository: /tmp/sample/.prikk
ref: heads/main
block 7c379a2c523028d6cf573ccc044290be9b4d04552db4bffa8aff3e046bf3c8ed
  ref-state: 857bee1bdde8e729f4c8afe7e88fdb40c6c4ab69e7a9091db598b0fcf1b238ca
  update-seq: 1
  kind: Root
  rollback-block: false
  parents: 0
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  patch 13066875f51e72a19afe81fa2eac1c5f2d8d5ecb39083fc8a35e0a9d909a2e68: kind: Normal, parents: 0
  previous-ref-state: <none>
";

#[test]
fn parses_two_messaged_patches_including_colons_in_the_message() {
    let h = history(LOG_FIXTURE_WITH_MESSAGES).expect("log parses");
    let block = &h.blocks[0];
    assert_eq!(block.patches, 2);
    assert_eq!(block.messages.len(), 2);
    assert_eq!(
        block.messages[0].patch_id,
        "2a4dfcf49d9ca967a7cdf462e8d43d09a475a687c1c46c253a00851c12e5ec55"
    );
    assert_eq!(block.messages[0].message, "first message: with a colon");
    assert_eq!(block.messages[1].message, "second: message");
}

#[test]
fn a_pre_0_32_block_has_no_messages_at_all_thats_normal_not_an_error() {
    let h = history(LOG_FIXTURE).expect("log parses");
    assert!(h.blocks.iter().all(|b| b.messages.is_empty()));
}

#[test]
fn a_straddling_block_reports_the_count_and_the_shorter_message_list_disagreeing() {
    // RFC 015 F4: this is the observed reality the acceptance-critical rendering (§4) is built on.
    let h = history(LOG_FIXTURE_STRADDLING).expect("log parses");
    let block = &h.blocks[0];
    assert_eq!(block.patches, 2);
    assert_eq!(block.messages.len(), 1); // the pre-0.32 patch contributes nothing
    assert_eq!(block.messages[0].message, "post-0.32 patch, message stored");
}

#[test]
fn a_message_resembling_a_field_label_parses_intact_and_forges_nothing() {
    let h = history(LOG_FIXTURE_LABEL_LIKE_MESSAGE).expect("log parses");
    assert_eq!(h.blocks[0].messages[0].message, "kind: Normal, parents: 0");
}

#[test]
fn a_malformed_patch_id_refuses() {
    let text = "\
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
  patch not-a-valid-id: some message
  previous-ref-state: <none>
";
    assert_eq!(history(text).unwrap_err().class(), "environment");
}

#[test]
fn a_patch_line_with_no_colon_space_separator_refuses() {
    let text = "\
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
  patch justanid-no-separator-here
  previous-ref-state: <none>
";
    assert_eq!(history(text).unwrap_err().class(), "environment");
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
// 0.31.0 on 2026-09-05 (RFC 012 F-e). RFC 009 F3's claim that a tag could never appear in this output
// is **corrected** by that same re-verification: this repository simply had no tag yet when F3 was
// written, so the absence proved nothing either way. `prikk-cli`'s `branch list` reads
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

// Captured verbatim (stdout; the dirty-exit `error: worktree has changes against the baseline` line is
// on stderr and is not part of the report) from a real prikk **0.38.0** binary on 2026-09-12, RFC 021
// F0. Commands, in order, in a fresh temp directory:
//
//   cargo install prikk --version 0.38.0 --locked --root <dir>
//   prikk setup . --author-seed-out … --maintainer-seed-out …
//   printf 'draft body\n' > "modified draft.txt"
//   prikk commit --from-worktree --ref heads/main -m "add draft"
//   prikk seal --allow-no-audit --ref heads/main
//   prikk mv "modified draft.txt" renamed.txt
//   prikk worktree-status --ref heads/main
//
// **Captured while 0.38.0 was still above the validated ceiling (then 0.33)**, deliberately: a parser
// fixture is a pure-function input and may be captured from any real binary, and F0 was fixed before
// the ceiling moved because it was wrong in a shipped release. RFC 021's Handoff B has since raised the
// ceiling to 0.38 and run the suite there, so this fixture is now *within* the validated range — but it
// was not validation of 0.38 when it was taken, and the distinction is why it was written down.
//
// The last two lines are why F0 exists: `live rename declarations:` is flush-left, and the indented
// line under it begins with `modified`, a change-kind word.
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

// Captured verbatim from the same real prikk 0.38.0 binary, same day and command sequence, with the
// file named **exactly** `modified` and renamed to `untracked` (`prikk mv modified untracked`). The
// narrowest form of the collision prikk's letter predicted: the declaration line is `  modified ->
// untracked`, whose first token is a kind word and whose "path" would be `-> untracked`.
const WORKTREE_RENAME_BARE_KIND_0_38_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 1
modified files: 0
untracked files: 1
unsupported paths: 0
worktree: changed against baseline
  missing modified — tracked file is absent from the worktree
  untracked untracked — worktree file is not in the baseline
live rename declarations: 1
  modified -> untracked
note: each declaration above is authored into the next `prikk commit` as a RenamePath -- run `prikk mv` again to change it, or move the destination back to the source to clear it
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

#[test]
fn a_0_38_rename_declaration_is_not_a_worktree_entry() {
    // RFC 021 F0. prikk reported two changes; stikk reported three, the third a file that does not
    // exist in a state prikk never named (`T-T4` manufactured out of correct prikk output).
    let s = worktree_status(WORKTREE_RENAME_0_38_FIXTURE).expect("parses");
    assert_eq!(
        s.entries.len(),
        2,
        "prikk reported 2 changes; entries were {:?}",
        s.entries
            .iter()
            .map(|e| (&e.kind, &e.path))
            .collect::<Vec<_>>()
    );
    // The count alone would pass a parser that kept the declaration and dropped a real entry.
    assert!(
        !s.entries.iter().any(|e| e.path.contains("->")),
        "a rename declaration was parsed as a change entry: {:?}",
        s.entries
    );
    assert!(
        s.entries
            .iter()
            .any(|e| e.kind == "missing" && e.path == "modified draft.txt")
    );
    assert!(
        s.entries
            .iter()
            .any(|e| e.kind == "untracked" && e.path == "renamed.txt")
    );
    // The counts prikk itself reported are untouched by the scoping.
    assert_eq!(s.missing, 1);
    assert_eq!(s.untracked, 1);
    assert_eq!(s.modified, 0);
}

#[test]
fn a_file_named_exactly_modified_does_not_fabricate_an_entry_when_renamed() {
    // The narrowest form of prikk's prediction: the declaration is `  modified -> untracked`. A `->`
    // reject heuristic would also catch this one — which is why the synthetic-section test below
    // exists, to tell the boundary fix apart from the heuristic.
    let s = worktree_status(WORKTREE_RENAME_BARE_KIND_0_38_FIXTURE).expect("parses");
    assert_eq!(
        s.entries.len(),
        2,
        "entries were {:?}",
        s.entries
            .iter()
            .map(|e| (&e.kind, &e.path))
            .collect::<Vec<_>>()
    );
    assert!(!s.entries.iter().any(|e| e.path.contains("->")));
    assert!(
        s.entries
            .iter()
            .any(|e| e.kind == "missing" && e.path == "modified")
    );
    assert!(
        s.entries
            .iter()
            .any(|e| e.kind == "untracked" && e.path == "untracked")
    );
}

#[test]
fn an_indented_section_prikk_does_not_emit_today_is_still_not_entries() {
    // **The test that proves the fix is the section boundary and not the rename.** A `->` reject list
    // passes the two tests above and fails this one: the fabricated line here contains no `->` at all,
    // and its first token is a change kind. Synthetic on purpose — it is a section prikk does not emit,
    // standing in for whatever it adds next.
    let text = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 0
modified files: 1
untracked files: 0
unsupported paths: 0
worktree: changed against baseline
  modified readme.txt — tracked file bytes differ from the baseline
pending attestation requests: 1
  modified readme.txt requested by someone
note: use `prikk commit -m <message>` to author node-addressed worktree changes
";
    let s = worktree_status(text).expect("parses");
    assert_eq!(
        s.entries.len(),
        1,
        "only the line inside the worktree: region is an entry; got {:?}",
        s.entries
            .iter()
            .map(|e| (&e.kind, &e.path))
            .collect::<Vec<_>>()
    );
    assert_eq!(s.entries[0].path, "readme.txt");
    assert!(s.entries[0].note.contains("bytes differ"));
}

// Captured verbatim (stdout) from a real prikk **0.38.0** binary on 2026-09-12, RFC 021 §4:
// `prikk setup .`, write `a.txt`, `prikk commit --from-worktree --ref heads/main -m "first patch"`,
// `prikk seal --allow-no-audit --ref heads/main`, then `prikk worktree-status --ref heads/main` on the
// now-clean worktree.
//
// **This is the one surface that changed between 0.33 and 0.38.** 0.38 prints `live rename
// declarations: N` unconditionally — here `0`, on a worktree with nothing renamed and nothing dirty.
// The pre-0.38 clean fixture above has no such line and stays as the regression case for 0.28–0.37;
// this one is the same situation at the new ceiling. Both must parse to zero entries.
const WORKTREE_CLEAN_0_38_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 1
missing files: 0
modified files: 0
untracked files: 0
unsupported paths: 0
worktree: clean against baseline
live rename declarations: 0
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

#[test]
fn a_clean_0_38_worktree_reports_no_entries_despite_the_rename_section() {
    // The `live rename declarations: 0` line is flush-left, so it closes an already-empty region
    // rather than opening one — and the count fields are read by label, not by position, so the line
    // between them and the note changes nothing about them either.
    let s = worktree_status(WORKTREE_CLEAN_0_38_FIXTURE).expect("parses");
    assert!(s.clean);
    assert!(
        s.entries.is_empty(),
        "a clean 0.38 worktree has no entries; got {:?}",
        s.entries
    );
    assert_eq!(s.tracked, 1);
    assert_eq!(s.unchanged, 1);
    assert_eq!(s.queued_elsewhere, None);
}

// Captured verbatim from a real prikk **0.38.0** binary on 2026-09-12 (RFC 021 §4, the capture made
// deliberately for the Queue view's own increment to inherit). Sequence, after `prikk setup .`:
// write `doc.txt`, commit, seal; then edit `doc.txt` and commit; then delete `doc.txt` and commit —
// leaving **two unsealed patches** where the first edits a node the second removes.
//
// `status --format json` is a 0.35+ surface and **stikk parses nothing here yet** — no seam method is
// added by RFC 021 (Decision 5: the Queue view is its own increment). This fixture exists so that
// increment starts from a real capture of the awkward case rather than re-deriving the sequence: the
// edit patch's operation carries **`unresolved_node_id` in place of a `path`**, because the node it
// edited no longer resolves to one by the time the queue is read (RFC 021 F6). The delete patch, in
// the same queue, carries an ordinary `path`. A Queue view that assumes every operation has a path
// renders nothing for the first patch, or worse invents one.
const STATUS_JSON_UNRESOLVED_NODE_0_38_FIXTURE: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 2,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": "1a09674ea634b776342f0d6cf92c9f14e1ae74a72f2354921b188fd50f86512c",
  "queue": {
    "count": 2,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "409ad8fca2d4696324ac292ee0ca7a3fcedaa6ff725d957ead07b5530b903829", "operations": [
        {"kind": "edit-text", "paths": [{"unresolved_node_id": "ae6e2234248156cb5eb4a3c1b4d0559222ae793c11fbcad28276a6f4dfb90e9a"}]}
      ]},
      {"patch_id": "6b200644b2444cea25975f1df322c562f7644d966f1f9c307877fb74d95e224d", "operations": [
        {"kind": "delete-node", "paths": [{"path": "doc.txt"}]}
      ]}
    ]
  }
}
"#;

#[test]
fn the_queued_unresolved_node_id_shape_is_pinned_for_the_queue_views_increment() {
    // No parser to exercise — this pins the captured *shape* so that if it drifts before the Queue
    // view is built, it drifts here rather than inside that increment's first hour. Asserted as facts
    // about the text, which is all stikk can honestly claim about a surface it does not yet read.
    let f = STATUS_JSON_UNRESOLVED_NODE_0_38_FIXTURE;
    assert!(f.contains(r#""schema_version": "status-report-v1""#));
    // The queue is enumerated (FR-051's unblocking at 0.35) and carries its own thresholds as data,
    // rather than stikk re-deriving them from environment variables.
    assert!(f.contains(r#""threshold_status": "none""#));
    assert!(f.contains(r#""warn_threshold": 800"#));
    assert!(f.contains(r#""hard_limit": 1000"#));
    // The awkward case itself: one operation identified by node id because no path resolves, beside
    // one identified by path, in the same queue.
    assert!(f.contains(r#""unresolved_node_id""#));
    assert!(f.contains(r#""path": "doc.txt""#));
}

#[test]
fn a_clean_headline_with_an_empty_region_at_end_of_input_parses() {
    // The region may be empty, and may run to end of input with no flush-left line closing it.
    let text = "\
ref: heads/main
tracked files: 2
unchanged files: 2
missing files: 0
modified files: 0
untracked files: 0
unsupported paths: 0
worktree: clean against baseline
";
    let s = worktree_status(text).expect("parses");
    assert!(s.clean);
    assert!(s.entries.is_empty());
}

// Captured verbatim against a real prikk 0.31.1 binary on 2026-09-06 (RFC 014 §2/§9):
// `prikk commit --from-worktree --ref heads/main -m "first patch"` on a freshly-`init`ed repository
// with one new file. Carries **both** `note:` lines a validated-range prikk prints — the perpetual
// diff-minimization note, and the message's-fate note (RFC 014 F4).
const COMMIT_FIXTURE: &str = "\
recorded worktree patch in active WAL
baseline ref: heads/main
patch id: 863f5f1ffbd21e0fe1c4456ab624022d294f664532adc65e3e109a0d7f005207
WAL sequence: 1
operations: 1
referenced blobs: 1
text edits: 0
  create-file a.txt
note: multi-operation text diff minimization, patch algebra, rename detection, and audit plugins remain later increments
note: the message is validated but not stored -- it will not appear in `prikk log`; persisting it is a later increment
";

// Captured verbatim against a real prikk 0.32.0 binary on 2026-09-06 (RFC 014 §9, the "found something
// that contradicts the RFC" item): **only one** `note:` line. prikk 0.32.0 has already shipped RFC 123
// (commit-message-as-evidence) upstream — ahead of stikk's own validated ceiling (0.31) — which stores
// the message and removes the message's-fate note because it would now be false. This fixture is the
// evidence that `CommitResult::notes` must be a `Vec<String>`, never two fixed named fields: the exact
// set of notes a real prikk prints is not stable across versions, even within "supported."
const COMMIT_FIXTURE_NO_MESSAGE_FATE_NOTE: &str = "\
recorded worktree patch in active WAL
baseline ref: heads/main
patch id: 927bed5b80c47fc3ff6db3532951b0b55236450c1e4cf50e3e9f241dfdcf31db
WAL sequence: 1
operations: 1
referenced blobs: 1
text edits: 0
  create-file a.txt
note: multi-operation text diff minimization, patch algebra, rename detection, and audit plugins remain later increments
";

#[test]
fn parses_a_commit_result_with_both_notes() {
    let r = commit(COMMIT_FIXTURE).expect("commit output parses");
    assert_eq!(r.baseline_ref, "heads/main");
    assert_eq!(
        r.patch_id,
        "863f5f1ffbd21e0fe1c4456ab624022d294f664532adc65e3e109a0d7f005207"
    );
    assert_eq!(r.wal_sequence, 1);
    assert_eq!(r.operations, 1);
    assert_eq!(r.referenced_blobs, 1);
    assert_eq!(r.text_edits, 0);
    assert_eq!(r.changes.len(), 1);
    assert_eq!(r.changes[0].operation, "create-file");
    assert_eq!(r.changes[0].path, "a.txt");
    assert_eq!(r.notes.len(), 2);
    // ER-02: verbatim, not paraphrased — both notes must appear byte-identical in the captured fixture.
    for note in &r.notes {
        assert!(COMMIT_FIXTURE.contains(note.as_str()));
    }
    assert!(r.notes[1].contains("message is validated but not stored"));
}

#[test]
fn a_commit_result_with_no_message_fate_note_has_exactly_one_note() {
    // RFC 014's own contradiction, made concrete: a real prikk 0.32.0 no longer prints the second note
    // (RFC 123 shipped), and this parser must not assume it is always there.
    let r = commit(COMMIT_FIXTURE_NO_MESSAGE_FATE_NOTE).expect("commit output parses");
    assert_eq!(r.notes.len(), 1);
    assert!(!r.notes[0].contains("message is validated but not stored"));
}

#[test]
fn commit_refuses_without_the_headline() {
    let text = "some unrelated prikk output\n";
    assert_eq!(commit(text).unwrap_err().class(), "environment");
}

#[test]
fn commit_refuses_an_invalid_patch_id() {
    let text = "recorded worktree patch in active WAL\nbaseline ref: heads/main\npatch id: not-hex\n\
                WAL sequence: 1\noperations: 1\nreferenced blobs: 1\ntext edits: 0\n";
    assert_eq!(commit(text).unwrap_err().class(), "environment");
}

// Captured verbatim against real prikk **0.28.0** and **0.33.0** binaries, both via `prikk seal
// --allow-no-audit --ref heads/main` on a repository with one queued patch (RFC 016 §2). The two are
// byte-identical apart from the object ids — no version gate needed, unlike `log`'s RFC 015 F1 shape
// change. This fixture is the 0.33.0 capture; the 0.28.0 capture is recorded only in the review
// request (identical shape, different hashes), per the handoff's "report whether it differs" ask.
const SEAL_FIXTURE: &str = "\
sealed active WAL into block
patches: 1
block id: 2c41d10327f720346a40d8efae08e130ccf498bfae3a53b988a7403cc07e3ad1
heads/main RefState: b4de76e6570a2f2366cfe2384b94929fb6d88e473e09f56ecd07458e72cd04fe
note: audit plugins remain later PRs
";

// Captured verbatim against a real prikk 0.33.0 binary: `prikk seal --allow-no-audit --ref
// heads/feature` — confirms the `RefState` line's ref half is **not** fixed to `heads/main` the way
// `status`'s is (RFC 016 §5), unlike every other fixture in this file which happens to use main.
const SEAL_FIXTURE_NON_MAIN_REF: &str = "\
sealed active WAL into block
patches: 1
block id: 5c364a6095683687639e2e0307bba207f69c0ed3ec8a624154f069d2928b31bf
heads/feature RefState: d6a87e1ec5cba68a1e5b75e2f77ddf9b23e47c19830c3d5d1f3276981c70e45c
note: audit plugins remain later PRs
";

#[test]
fn parses_a_seal_result() {
    let r = seal(SEAL_FIXTURE).expect("seal output parses");
    assert_eq!(r.patches, 1);
    assert_eq!(
        r.block_id,
        "2c41d10327f720346a40d8efae08e130ccf498bfae3a53b988a7403cc07e3ad1"
    );
    assert_eq!(r.reff, "heads/main");
    assert_eq!(
        r.ref_state,
        "b4de76e6570a2f2366cfe2384b94929fb6d88e473e09f56ecd07458e72cd04fe"
    );
    assert_eq!(r.notes, vec!["note: audit plugins remain later PRs"]);
}

#[test]
fn a_seal_result_names_whichever_ref_was_actually_sealed() {
    let r = seal(SEAL_FIXTURE_NON_MAIN_REF).expect("seal output parses");
    assert_eq!(r.reff, "heads/feature");
}

#[test]
fn seal_refuses_without_the_headline() {
    let text = "some unrelated prikk output\n";
    assert_eq!(seal(text).unwrap_err().class(), "environment");
}

#[test]
fn seal_refuses_an_invalid_block_id() {
    let text = "sealed active WAL into block\npatches: 1\nblock id: not-hex\n\
                heads/main RefState: b4de76e6570a2f2366cfe2384b94929fb6d88e473e09f56ecd07458e72cd04fe\n";
    assert_eq!(seal(text).unwrap_err().class(), "environment");
}

#[test]
fn seal_refuses_a_missing_ref_state_line() {
    let text = "sealed active WAL into block\npatches: 1\n\
                block id: 2c41d10327f720346a40d8efae08e130ccf498bfae3a53b988a7403cc07e3ad1\n";
    assert_eq!(seal(text).unwrap_err().class(), "environment");
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

// Captured verbatim (stdout; the dirty-exit `error: worktree has changes against the baseline` line is
// on stderr and is not part of the report) from a real prikk **0.28.0** binary on 2026-09-13, RFC 027
// F0. The repository is the real-binary harness's own `Fixture` at 0.28 — `readme.txt` committed and
// sealed — moved to the neutral `/tmp/repo` before anything was captured, because prikk prints an
// unsupported entry's path **absolute**, and no fixture may carry anyone's home directory or be edited
// afterwards to remove one. Then, in `/tmp/repo` (written with `std::fs::write`):
//
//   readme.txt            rewritten to `hello again\n`          → one ordinary modification
//   back\slash.txt        a backslash in the name               → unsupported-path
//   bad<0xFF>name.txt     one byte that is not UTF-8            → unsupported-path
//   prikk worktree-status --ref heads/main
//
// Nothing was edited after capture. The two `�` are prikk's own — it prints U+FFFD (`EF BF BD`) for the
// byte it cannot decode — and each `\\` below is one backslash, escaped only as Rust source requires.
//
// **This is the shape stikk has dropped since 0.1.0:** `unsupported paths: 2`, and two lines whose kind
// is `unsupported-path` — a word stikk's reader did not know.
const WORKTREE_UNSUPPORTED_0_28_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 0
modified files: 1
untracked files: 0
unsupported paths: 2
worktree: changed against baseline
  unsupported-path /tmp/repo/back\\slash.txt — worktree path is not representable as a safe Prikk path: invalid name: backslashes are not allowed in repository paths
  unsupported-path /tmp/repo/bad�name.txt — worktree path is not representable as a safe Prikk path: integrity error: worktree path is not UTF-8: bad�name.txt
  modified readme.txt — tracked file bytes differ from the baseline
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

// Captured verbatim (stdout; the dirty-exit `error:` line is on stderr) from a real prikk **0.41.0**
// binary on 2026-09-13, RFC 027 F0 — the same probe, the same three files, the same neutral
// `/tmp/repo`, the harness's `Fixture` at 0.41. Nothing edited after capture.
//
// The entry lines' shape is unchanged from 0.28; what differs at the ceiling is around them —
// `refused paths: 0` (0.39+), `live rename declarations: 0` (0.38+), and the non-UTF-8 entry's own note
// wording, which is prikk's and is carried as-is.
const WORKTREE_UNSUPPORTED_0_41_FIXTURE: &str = "\
worktree-status repository: /tmp/repo/.prikk
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 0
modified files: 1
untracked files: 0
unsupported paths: 2
refused paths: 0
worktree: changed against baseline
  unsupported-path /tmp/repo/back\\slash.txt — worktree path is not representable as a safe Prikk path: invalid name: backslashes are not allowed in repository paths
  unsupported-path /tmp/repo/bad�name.txt — worktree path is not representable as a safe Prikk path: invalid name: worktree path is not valid UTF-8: bad�name.txt
  modified readme.txt — tracked file bytes differ from the baseline
live rename declarations: 0
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
";

#[test]
fn unsupported_path_entries_are_listed_at_both_ends() {
    // RFC 027 F0. prikk counted two and printed two; stikk counted two and listed none.
    for (end, text) in [
        ("0.28", WORKTREE_UNSUPPORTED_0_28_FIXTURE),
        ("0.41", WORKTREE_UNSUPPORTED_0_41_FIXTURE),
    ] {
        let s = worktree_status(text).unwrap_or_else(|e| panic!("{end}: {e:?}"));
        assert_eq!(s.unsupported, 2, "{end}");
        assert_eq!(s.entries.len(), 3, "{end}: entries were {:?}", s.entries);
        assert!(
            s.entries.iter().any(|e| e.kind == "unsupported-path"
                && e.path == "/tmp/repo/back\\slash.txt"
                && e.note.contains("backslashes are not allowed")),
            "{end}: no backslash entry in {:?}",
            s.entries
        );
        assert!(
            s.entries.iter().any(|e| e.kind == "unsupported-path"
                && e.path == "/tmp/repo/bad\u{fffd}name.txt"
                && e.note.contains("UTF-8")),
            "{end}: no non-UTF-8 entry in {:?}",
            s.entries
        );
        assert!(
            s.entries
                .iter()
                .any(|e| e.kind == "modified" && e.path == "readme.txt"),
            "{end}"
        );
    }
}

#[test]
fn an_indented_line_with_an_invented_kind_is_an_entry() {
    // RFC 027 F0: every indented line in the scoped region is an entry, whatever its first word.
    // `typechange` is a kind prikk does not print at any supported version — which is the point:
    // `WorktreeEntry::kind` promises a future kind renders, and a closed list made that unreachable.
    let text = "\
ref: heads/main
tracked files: 1
unchanged files: 0
missing files: 0
modified files: 0
untracked files: 0
unsupported paths: 0
worktree: changed against baseline
  typechange link.txt — a kind prikk does not print today
live rename declarations: 0
";
    let s = worktree_status(text).expect("parses");
    assert_eq!(s.entries.len(), 1, "entries were {:?}", s.entries);
    assert_eq!(s.entries[0].kind, "typechange");
    assert_eq!(s.entries[0].path, "link.txt");
    assert_eq!(s.entries[0].note, "a kind prikk does not print today");
}

#[test]
fn a_refused_suffix_stays_in_the_note() {
    // RFC 027 §3: at 0.39+ a refused entry's line ends ` [refused: <reason>]` (`"  {} {} — {}{}"` in
    // prikk's printer). Handoff A does not parse it; it is prikk's text and rides in the note.
    let text = "\
ref: heads/main
tracked files: 0
unchanged files: 0
missing files: 0
modified files: 0
untracked files: 1
unsupported paths: 0
refused paths: 1
worktree: changed against baseline
  untracked big.bin — worktree file is not in the baseline [refused: some reason]
live rename declarations: 0
";
    let s = worktree_status(text).expect("parses");
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.entries[0].path, "big.bin");
    assert_eq!(
        s.entries[0].note,
        "worktree file is not in the baseline [refused: some reason]"
    );
}

/// Every `worktree-status` fixture constant in this file, by name. Kept complete by
/// [`the_count_invariant_covers_every_worktree_status_fixture`] — a new fixture that is not added here
/// fails that test, so the invariant cannot quietly skip one.
fn every_worktree_status_fixture() -> [(&'static str, &'static str); 8] {
    [
        ("WORKTREE_CLEAN_FIXTURE", WORKTREE_CLEAN_FIXTURE),
        ("WORKTREE_DIRTY_FIXTURE", WORKTREE_DIRTY_FIXTURE),
        (
            "WORKTREE_QUEUED_ELSEWHERE_FIXTURE",
            WORKTREE_QUEUED_ELSEWHERE_FIXTURE,
        ),
        ("WORKTREE_RENAME_0_38_FIXTURE", WORKTREE_RENAME_0_38_FIXTURE),
        (
            "WORKTREE_RENAME_BARE_KIND_0_38_FIXTURE",
            WORKTREE_RENAME_BARE_KIND_0_38_FIXTURE,
        ),
        ("WORKTREE_CLEAN_0_38_FIXTURE", WORKTREE_CLEAN_0_38_FIXTURE),
        (
            "WORKTREE_UNSUPPORTED_0_28_FIXTURE",
            WORKTREE_UNSUPPORTED_0_28_FIXTURE,
        ),
        (
            "WORKTREE_UNSUPPORTED_0_41_FIXTURE",
            WORKTREE_UNSUPPORTED_0_41_FIXTURE,
        ),
    ]
}

#[test]
fn every_worktree_status_fixture_lists_as_many_entries_as_prikk_counts() {
    // RFC 027 §4.2 — **the invariant that would have caught F0.** prikk computes each counter with
    // `count_kind` over the very list it then prints, so for every kind the parsed entry count equals
    // prikk's own number. A reader that drops a kind's lines passes every per-fixture assertion that
    // never looked at that kind; it cannot pass this.
    for (name, text) in every_worktree_status_fixture() {
        let s = worktree_status(text).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let listed = |kind: &str| {
            u64::try_from(s.entries.iter().filter(|e| e.kind == kind).count()).unwrap()
        };
        assert_eq!(listed("modified"), s.modified, "{name}: `modified files:`");
        assert_eq!(listed("missing"), s.missing, "{name}: `missing files:`");
        assert_eq!(
            listed("untracked"),
            s.untracked,
            "{name}: `untracked files:`"
        );
        assert_eq!(
            listed("unsupported-path"),
            s.unsupported,
            "{name}: `unsupported paths:`"
        );
        // And nothing listed beyond what prikk counted: no kind in a captured fixture lacks a counter.
        assert_eq!(
            u64::try_from(s.entries.len()).unwrap(),
            s.modified + s.missing + s.untracked + s.unsupported,
            "{name}: entries {:?}",
            s.entries
        );
    }
}

#[test]
fn the_count_invariant_covers_every_worktree_status_fixture() {
    let listed: Vec<&str> = every_worktree_status_fixture()
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let source = include_str!("tests.rs");
    let declared: Vec<&str> = source
        .lines()
        .filter_map(|line| line.strip_prefix("const WORKTREE_"))
        .filter_map(|rest| rest.split_once(": &str"))
        .map(|(name, _)| name)
        .collect();
    assert!(!declared.is_empty(), "the scan found no worktree fixtures");
    for name in declared {
        let full = format!("WORKTREE_{name}");
        assert!(
            listed.contains(&full.as_str()),
            "{full} is a worktree-status fixture the count invariant does not run over"
        );
    }
}

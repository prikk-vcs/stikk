//! Tests for the three JSON readers.
//!
//! **Every fixture here is captured from a real prikk 0.41.0 binary**, 2026-09-13, against a
//! repository built by `probe41.sh` — `init`, two `key generate --out`, `trust maintainer add`, one
//! `commit --from-worktree`, one `seal --allow-no-audit`, `branch create heads/feature`, and
//! `tag create tags/v1 --target heads/main -m one`. Captured, never written: the rule RFC 009
//! established and every re-baseline since has held to.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

/// Real 64-hex object ids for the hand-written fixtures below.
///
/// **Not placeholders.** `"a"` used to do here, and the JSON readers accepted it — which is precisely
/// how the missing `ObjectId::parse` boundary showed up in this file rather than in the product
/// (review C1). A fixture that could not come out of prikk tests the reader against a world that does
/// not exist.
const BLOCK: &str = "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7";
const STATE: &str = "ab2ee88ddfc741240fdecc423edba4c4d65df60338464902059d34c87101cddb";
const PREV: &str = "7a8cf8d4e1fd0d1f04cbc8d7a590c0e0950d1b319b8ebe7fa45bf3a1f0b5b7f9";

/// `prikk log --ref heads/main --limit 2 --format json` at 0.41.0.
const LOG_0_41: &str = r#"{
  "schema_version": "log-report-v1",
  "repository": "/tmp/probe-cbnR/repo/.prikk",
  "ref": "heads/main",
  "blocks": [
    {
      "block_id": "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7",
      "ref_state_id": "ab2ee88ddfc741240fdecc423edba4c4d65df60338464902059d34c87101cddb",
      "update_seq": 1,
      "kind": "Root",
      "rollback_block": false,
      "parent_count": 0,
      "patch_count": 1,
      "rollback_patch_count": 0,
      "required_attestation_count": 0,
      "patch_messages": [
        {"patch_id": "29042a475dd101cb8deb41232c134b48f07eca9170f0839676917c0a5f9021ac", "message": "first commit"}
      ],
      "previous_ref_state_id": null
    }
  ]
}"#;

/// `prikk branch list --all --format json` at 0.41.0.
const BRANCH_0_41: &str = r#"{
  "schema_version": "branch-list-v1",
  "branches": [
    {"ref_name": "heads/feature", "ref_state_id": "7a8cf8d4e1fd0d1f04cbc8d7a590c0e0950d1b319b8ebe7fa45bf3a1f0b5b7f9", "closed": false},
    {"ref_name": "heads/main", "ref_state_id": "ab2ee88ddfc741240fdecc423edba4c4d65df60338464902059d34c87101cddb", "closed": false}
  ],
  "received": []
}"#;

/// `prikk tag list --format json` at 0.41.0.
const TAG_0_41: &str = r#"{
  "schema_version": "tag-list-v1",
  "tags": [
    {"ref_name": "tags/v1", "target_block_id": "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7"}
  ]
}"#;

#[test]
fn the_captured_log_report_parses_to_the_block_it_describes() {
    let history = history(LOG_0_41).expect("captured 0.41 log report");
    assert_eq!(history.reff, "heads/main");
    assert_eq!(history.blocks.len(), 1);
    let block = &history.blocks[0];
    assert_eq!(block.kind, "Root");
    assert_eq!(block.update_seq, 1);
    assert_eq!(block.patches, 1);
    assert!(!block.rollback_block);
    assert_eq!(block.parents, 0);
    // Genesis: no previous RefState, and `null` is how the schema spells that.
    assert_eq!(block.previous_ref_state, None);
    assert_eq!(block.messages.len(), 1);
    assert_eq!(block.messages[0].message, "first commit");
}

/// A block sealed entirely below prikk 0.32 carries no messages (RFC 015 F4). **Absent and empty must
/// read the same**, and neither is an error — `patch_count` stays the authoritative total.
#[test]
fn a_block_with_no_patch_messages_is_normal_whether_the_field_is_absent_or_empty() {
    for spelling in [r#""patch_messages": [],"#, ""] {
        let text = format!(
            r#"{{"schema_version": "log-report-v1", "ref": "heads/main", "blocks": [
                {{"block_id": "{BLOCK}", "ref_state_id": "{STATE}", "update_seq": 2, "kind": "Normal",
                  "rollback_block": false, "parent_count": 1, "patch_count": 3,
                  "rollback_patch_count": 0, "required_attestation_count": 0, {spelling}
                  "previous_ref_state_id": "{PREV}"}}]}}"#
        );
        let history = history(&text).expect("a message-less block is normal");
        assert_eq!(history.blocks[0].messages.len(), 0);
        assert_eq!(
            history.blocks[0].patches, 3,
            "the count is still authoritative"
        );
        assert_eq!(history.blocks[0].previous_ref_state.as_deref(), Some(PREV));
    }
}

#[test]
fn the_captured_branch_report_parses_both_arrays() {
    let refs = refs(BRANCH_0_41).expect("captured 0.41 branch report");
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].name, "heads/feature");
    assert!(!refs[0].received);
    assert!(!refs[0].closed);
    // **No tags.** prikk 0.39 stopped including tag refs in `branch list --all`; at 0.38 this same
    // command listed `tags/v1` too. stikk's `history::list_refs` de-duplicates by name either way
    // (RFC 012), so the ref picker showed each tag once before and shows it once now.
    assert!(
        !refs.iter().any(|r| r.name.starts_with("tags/")),
        "0.39+ does not list tags here: {refs:?}"
    );
}

/// `received` marks an entry by **which array it came from**, not by a word parsed out of a line.
#[test]
fn a_received_ref_is_flagged_by_its_array() {
    let text = format!(
        r#"{{"schema_version": "branch-list-v1",
        "branches": [{{"ref_name": "heads/main", "ref_state_id": "{BLOCK}", "closed": true}}],
        "received": [{{"ref_name": "heads/theirs", "ref_state_id": "{STATE}"}}]}}"#
    );
    let refs = refs(&text).expect("both arrays");
    assert_eq!(refs.len(), 2);
    assert!(refs[0].closed && !refs[0].received);
    // The received entry carries no `closed` field at all — prikk never emits one there — and stikk
    // does not read for it: `false` is structural, from which array the entry came out of.
    assert!(refs[1].received && !refs[1].closed);
}

/// A repository that has never received a ref omits the array entirely — an empty list spelled by
/// omission, not a schema mismatch.
#[test]
fn an_absent_received_array_is_an_empty_list() {
    let text = format!(
        r#"{{"schema_version": "branch-list-v1",
        "branches": [{{"ref_name": "heads/main", "ref_state_id": "{BLOCK}", "closed": false}}]}}"#
    );
    assert_eq!(refs(&text).map(|r| r.len()).ok(), Some(1));
}

#[test]
fn the_captured_tag_report_parses_to_its_target_block() {
    let tags = tags(TAG_0_41).expect("captured 0.41 tag report");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "tags/v1");
    // A tag points at a block, and that block id is what the prose parser read from the same place.
    assert_eq!(
        tags[0].id,
        "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7"
    );
    assert!(!tags[0].closed && !tags[0].received);
}

/// **The schema name is checked before any field is read.** A `-v2` report is not something to parse
/// optimistically and hope the fields survived — that is RFC 012 F-e's failure shape in JSON.
#[test]
fn an_unknown_schema_version_is_refused_and_names_both_versions() {
    let text = LOG_0_41.replace("log-report-v1", "log-report-v2");
    let err = history(&text).expect_err("stikk does not guess at an unknown schema");
    let message = err.to_string();
    assert!(message.contains("log-report-v2"), "{message}");
    assert!(message.contains("log-report-v1"), "{message}");
}

/// Reading the wrong report's JSON is caught by the same check, rather than producing an empty list.
#[test]
fn a_report_of_the_wrong_kind_is_refused_not_read_as_empty() {
    assert!(refs(TAG_0_41).is_err());
    assert!(tags(BRANCH_0_41).is_err());
    assert!(history(BRANCH_0_41).is_err());
}

/// **A control character in a ref name is refused**, exactly as the prose reader refuses it.
///
/// This test asserted the opposite until review C1: it required the name to be *carried through* to
/// the renderer, on the reasoning that `inert()` neutralizes it at the cell. That reasoning is true
/// and it is not the whole rule — `INV-9`/`UD-02` put the boundary at the parse, because a shape prikk
/// would never emit means stikk misread prikk, and the prose reader has refused it since RFC 012 F-d.
/// Carrying it onward made the JSON path quietly laxer than the prose path it replaced.
#[test]
fn a_control_character_in_a_ref_name_is_refused_the_way_the_prose_reader_refuses_it() {
    let escaped_esc = "\\u001b";
    let text = format!(
        r#"{{"schema_version": "tag-list-v1",
        "tags": [{{"ref_name": "tags/{escaped_esc}[2Jpwned", "target_block_id": "{BLOCK}"}}]}}"#
    );
    let err = tags(&text).expect_err("a control-bearing ref name is not a shape prikk emits");
    let message = err.to_string();
    assert!(message.contains("unrecognized ref name"), "{message}");
    assert!(message.contains("ref_name"), "names the field: {message}");
}

/// One refusal per field family, so the boundary cannot be lost a field at a time.
///
/// Each case swaps exactly one field for a shape prikk never emits and requires an error naming the
/// field. The bad ref name is **empty** and the bad ids are one character: `RefName::parse` refuses
/// empty and control-bearing names (it does not police the rest of the grammar — that is prikk's job,
/// and stikk guessing at it would be the fabrication `C-T2b` forbids), while `ObjectId::parse` wants a
/// full object id.
#[test]
fn every_name_and_id_field_in_the_log_report_is_validated() {
    let log_with = |field: &str, value: &str| {
        let quoted_previous = if field == "previous_ref_state_id" {
            format!("\"{value}\"")
        } else {
            "null".to_string()
        };
        format!(
            r#"{{"schema_version": "log-report-v1", "ref": "{ref_name}", "blocks": [
                {{"block_id": "{block}", "ref_state_id": "{state}", "update_seq": 1,
                  "kind": "Root", "rollback_block": false, "parent_count": 0, "patch_count": 1,
                  "rollback_patch_count": 0, "required_attestation_count": 0,
                  "patch_messages": [{{"patch_id": "{patch}", "message": "m"}}],
                  "previous_ref_state_id": {quoted_previous}}}]}}"#,
            ref_name = if field == "ref" { value } else { "heads/main" },
            block = if field == "block_id" { value } else { BLOCK },
            state = if field == "ref_state_id" {
                value
            } else {
                STATE
            },
            patch = if field == "patch_id" { value } else { PREV },
        )
    };

    // The control: every field well-formed, so a failure below is the swap and not the fixture.
    assert!(
        history(&log_with("none", "")).is_ok(),
        "the control must parse"
    );

    for (field, bad) in [
        ("ref", ""),
        ("block_id", "a"),
        ("ref_state_id", "a"),
        ("patch_id", "a"),
        ("previous_ref_state_id", "a"),
    ] {
        let err = history(&log_with(field, bad))
            .expect_err(&format!("`{field}` = {bad:?} must be refused"));
        let message = err.to_string();
        assert!(
            message.contains(field),
            "the error must name the field it refused: {message}"
        );
        assert!(
            message.contains(&format!("{bad:?}")),
            "the error must show the value prikk sent: {message}"
        );
    }
}

/// The same, for the two ref listings.
#[test]
fn every_name_and_id_field_in_the_ref_listings_is_validated() {
    let branch = |name: &str, id: &str| {
        format!(
            r#"{{"schema_version": "branch-list-v1",
            "branches": [{{"ref_name": "{name}", "ref_state_id": "{id}", "closed": false}}]}}"#
        )
    };
    assert!(
        refs(&branch("heads/main", BLOCK)).is_ok(),
        "the control must parse"
    );
    assert!(refs(&branch("", BLOCK)).is_err(), "an empty ref name");
    assert!(refs(&branch("heads/main", "a")).is_err());

    let tag = |name: &str, id: &str| {
        format!(
            r#"{{"schema_version": "tag-list-v1",
            "tags": [{{"ref_name": "{name}", "target_block_id": "{id}"}}]}}"#
        )
    };
    assert!(
        tags(&tag("tags/v1", BLOCK)).is_ok(),
        "the control must parse"
    );
    assert!(tags(&tag("", BLOCK)).is_err(), "an empty ref name");
    assert!(tags(&tag("tags/v1", "a")).is_err());

    // And a received entry is validated as strictly as a branch one.
    let received = r#"{"schema_version": "branch-list-v1", "branches": [],
        "received": [{"ref_name": "heads/theirs", "ref_state_id": "a"}]}"#;
    assert!(
        refs(received).is_err(),
        "received entries are not a laxer path"
    );
}

/// **`closed` is required on a branch**, because it carries a fact the ref picker shows.
///
/// It read `unwrap_or(false)` until review C2 — the one silent field in three parsers. If prikk
/// renamed or dropped it, every closed branch would have rendered as open and nothing would have said
/// so. prikk emits it on every branch entry and on no received one, deliberately, so which array an
/// entry came from is what decides whether stikk reads for it.
#[test]
fn a_branch_missing_closed_is_a_schema_change_and_says_so() {
    let text = format!(
        r#"{{"schema_version": "branch-list-v1",
        "branches": [{{"ref_name": "heads/main", "ref_state_id": "{BLOCK}"}}]}}"#
    );
    let err = refs(&text).expect_err("a branch without `closed` is a schema change");
    assert!(err.to_string().contains("closed"), "{err}");

    // A wrong type is caught by the same read, rather than quietly meaning `false`.
    let wrong = format!(
        r#"{{"schema_version": "branch-list-v1",
        "branches": [{{"ref_name": "heads/main", "ref_state_id": "{BLOCK}", "closed": "yes"}}]}}"#
    );
    assert!(refs(&wrong).is_err());
}

/// `patch_messages`: **absent is tolerated, a wrong type is not.** Two different rules, and this file
/// used to give one answer to both.
#[test]
fn a_wrong_typed_patch_messages_field_errors_where_an_absent_one_does_not() {
    let with = |messages: &str| {
        format!(
            r#"{{"schema_version": "log-report-v1", "ref": "heads/main", "blocks": [
                {{"block_id": "{BLOCK}", "ref_state_id": "{STATE}", "update_seq": 1,
                  "kind": "Root", "rollback_block": false, "parent_count": 0, "patch_count": 1,
                  "rollback_patch_count": 0, "required_attestation_count": 0, {messages}
                  "previous_ref_state_id": null}}]}}"#
        )
    };
    assert!(
        history(&with("")).is_ok(),
        "absent is how a pre-0.32 block spells it"
    );
    assert!(history(&with(r#""patch_messages": [],"#)).is_ok());
    assert!(
        history(&with(r#""patch_messages": {},"#)).is_err(),
        "an object where an array belongs is a schema change"
    );
}

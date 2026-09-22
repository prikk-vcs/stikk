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

// --- RFC 026: `key-status-v1` -----------------------------------------------------------------
//
// Every constant below is captured from a real prikk 0.41.0, one repository per state, 2026-09-13.
// The states are not hypothetical: each was produced by putting a repository into it and asking.

/// Fresh repository, seed-file override, nothing signed yet — **the default state of every new
/// repository**, and so the first commit every new user makes.
const KS_UNRECORDED: &str = r#"{"schema_version":"key-status-v1","roles":[{"role":"author",
  "source":"seed-file-override","path":"/tmp/ks/a.seed","usable":true,"reason":null,
  "key_id":"author","key_id_source":"default",
  "public_key":"61535ba01d029f09e898dd245b4b96ab68e86c4e1b5c12709f801b124b5aaed2",
  "binding":"unrecorded"}]}"#;

/// After signing: the key binds to what the repository records.
const KS_MATCHES: &str = r#"{"schema_version":"key-status-v1","roles":[{"role":"author",
  "source":"seed-file-override","path":"/tmp/ks/a.seed","usable":true,"reason":null,
  "key_id":"author","key_id_source":"environment",
  "public_key":"946f3fe3409b6881b9cdcc0302417a87dae832bfdc3a2c9ba49e28d8d9c19e66",
  "binding":"matches"}]}"#;

/// A different seed under the same key id — prikk will refuse at signing time.
const KS_MISMATCH: &str = r#"{"schema_version":"key-status-v1","roles":[{"role":"author",
  "source":"seed-file-override","path":"/tmp/ks/other.seed","usable":true,"reason":null,
  "key_id":"author","key_id_source":"environment",
  "public_key":"23f674f45c1619f060344469423d791c3383fb21785cf227ed7811e04d281c55",
  "binding":"mismatch"}]}"#;

/// MAINTAINER whose key the repository's trust policy has not adopted.
const KS_NOT_ADOPTED: &str = r#"{"schema_version":"key-status-v1","roles":[{"role":"maintainer",
  "source":"seed-file-override","path":"/tmp/ks/m.seed","usable":true,"reason":null,
  "key_id":"maintainer","key_id_source":"default",
  "public_key":"3734a7a32300be15304c43d3536bb81a001252803debd81d5b6c3c9aef99e286",
  "binding":"not-adopted"}]}"#;

/// The four unusable shapes prikk produced, one per `reason`. **`public_key` and `binding` are both
/// `null` in every one**, which is why neither may be a required field.
const KS_UNUSABLE: [(&str, &str); 4] = [
    (
        "missing",
        r#"{"schema_version":"key-status-v1","roles":[{"role":"author","source":"key-directory","path":"/tmp/cfg/prikk/author.seed","usable":false,"reason":"missing","key_id":"author","key_id_source":"default","public_key":null,"binding":null}]}"#,
    ),
    (
        "override-missing",
        r#"{"schema_version":"key-status-v1","roles":[{"role":"author","source":"seed-file-override","path":"/tmp/ks/nope.seed","usable":false,"reason":"override-missing","key_id":"author","key_id_source":"default","public_key":null,"binding":null}]}"#,
    ),
    (
        "readable-by-others (mode 0644)",
        r#"{"schema_version":"key-status-v1","roles":[{"role":"author","source":"seed-file-override","path":"/tmp/ks/loose.seed","usable":false,"reason":"readable-by-others (mode 0644)","key_id":"author","key_id_source":"default","public_key":null,"binding":null}]}"#,
    ),
    (
        "undecodable",
        r#"{"schema_version":"key-status-v1","roles":[{"role":"author","source":"seed-file-override","path":"/tmp/ks/bad.seed","usable":false,"reason":"undecodable","key_id":"author","key_id_source":"default","public_key":null,"binding":null}]}"#,
    ),
];

#[test]
fn every_binding_prikk_emits_parses_to_its_own_state() {
    for (text, want) in [
        (KS_UNRECORDED, stikk_model::Binding::Unrecorded),
        (KS_MATCHES, stikk_model::Binding::Matches),
        (KS_MISMATCH, stikk_model::Binding::Mismatch),
        (KS_NOT_ADOPTED, stikk_model::Binding::NotAdopted),
    ] {
        let rows = key_status(text).expect("captured 0.41 key status");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].2.usable);
        assert_eq!(rows[0].2.binding, want, "for {text}");
    }
}

/// **prikk's `reason` travels verbatim** (`ER-02`). Four reasons do not become one "not ready".
#[test]
fn each_unusable_reason_is_carried_word_for_word() {
    for (reason, text) in KS_UNUSABLE {
        let rows = key_status(text).expect("captured 0.41 key status");
        assert!(!rows[0].2.usable, "{reason}");
        assert_eq!(rows[0].1.reason.as_deref(), Some(reason));
        // Both are null whenever the seed is unusable — measured on all four.
        assert_eq!(rows[0].1.public_key, None, "{reason}");
        assert_eq!(rows[0].2.binding, stikk_model::Binding::Absent, "{reason}");
    }
}

/// **prikk always has a key id**, defaulting to the role's own name, and says which.
///
/// This is the half of RFC 026 F4 that made the confirmation card render *nothing*: stikk read the id
/// from `PRIKK_<ROLE>_KEY_ID` and got `None` on the default setup, while prikk would have signed as
/// `author` all along.
#[test]
fn the_key_id_is_always_present_and_says_where_it_came_from() {
    let rows = key_status(KS_UNRECORDED).expect("captured");
    assert_eq!(rows[0].1.key_id.as_deref(), Some("author"));
    assert_eq!(rows[0].1.key_id_source.as_deref(), Some("default"));
    let rows = key_status(KS_MATCHES).expect("captured");
    assert_eq!(rows[0].1.key_id_source.as_deref(), Some("environment"));
}

/// A `binding` prikk has not shipped yet reads as `Absent` — **withholding nothing it should grant,
/// claiming nothing it cannot support**, which is the safe direction for an unknown state.
#[test]
fn an_unrecognized_binding_is_absent_rather_than_a_pass_or_a_parse_error() {
    let text = KS_MATCHES.replace("\"matches\"", "\"rebound-by-policy\"");
    let rows = key_status(&text).expect("an unknown binding is not a parse failure");
    assert_eq!(rows[0].2.binding, stikk_model::Binding::Absent);
}

/// `public_key` is a 64-hex value and crosses the same boundary every other id does (RFC 026 §2).
#[test]
fn a_malformed_public_key_is_refused_at_the_boundary() {
    let text = KS_MATCHES.replace(
        "946f3fe3409b6881b9cdcc0302417a87dae832bfdc3a2c9ba49e28d8d9c19e66",
        "a",
    );
    let err = key_status(&text).expect_err("a one-character public key is not a shape prikk emits");
    assert!(err.to_string().contains("public_key"), "{err}");
}

// ---------------------------------------------------------------------------------------------
// `worktree-status-report-v1` (RFC 027 decision 2).
//
// **Both fixtures are captured from a real prikk 0.41.0 binary**, 2026-09-13, by RFC 027 Handoff B's
// probe: the real-binary harness's own `Fixture`, its repository moved to the neutral `/tmp/repo`
// before any capture, stdout on prikk's dirty exit (1). Never edited after capture. The rule-breaking
// variants below are **derived from these by one textual substitution each**, so every other byte a
// variant carries is still prikk's.
// ---------------------------------------------------------------------------------------------

/// `prikk worktree-status --ref heads/main --format json` at 0.41.0: `readme.txt` committed and sealed,
/// then rewritten, and an untracked symlink `link.txt -> readme.txt` created beside it (RFC 027 F1's
/// measured shape). `prikk commit` on this tree prints `error: ` followed by exactly `link.txt`'s
/// `refusal`.
pub(in crate::cli_backend) const WORKTREE_SYMLINK_JSON_0_41: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "ref": "heads/main",
  "tracked_files": 1,
  "unchanged_files": 0,
  "clean": false,
  "refused_count": 1,
  "queued_elsewhere": null,
  "changes": [
    {"path": "link.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "refused", "refusal": "precondition not met: link.txt: worktree symlink authoring is out of scope"},
    {"path": "readme.txt", "kind": "modified", "detail": "tracked file bytes differ from the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": []
}
"#;

/// `prikk worktree-status --ref heads/other --format json` at 0.41.0, on a repository whose active WAL
/// holds one unsealed patch for `heads/main` (RFC 027 F6's measured shape). The prose report for the same
/// tree carries prikk's warning sentence; this one carries the ref and nothing more.
const WORKTREE_QUEUED_JSON_0_41: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "ref": "heads/other",
  "tracked_files": 0,
  "unchanged_files": 0,
  "clean": false,
  "refused_count": 0,
  "queued_elsewhere": "heads/main",
  "changes": [
    {"path": "readme.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": []
}
"#;

/// `fixture` with `from` replaced once by `to` — and a failure, not a silent no-op, if `from` is not
/// there, so a variant cannot quietly become a copy of the capture.
pub(in crate::cli_backend) fn variant(fixture: &str, from: &str, to: &str) -> String {
    assert!(
        fixture.contains(from),
        "variant source text {from:?} is not in the fixture"
    );
    fixture.replacen(from, to, 1)
}

#[test]
fn a_refused_symlink_reads_with_prikks_reason_verbatim() {
    let s = worktree_status(WORKTREE_SYMLINK_JSON_0_41, 41).expect("parses");
    assert_eq!(s.reff, "heads/main");
    assert!(!s.clean);
    assert_eq!((s.tracked, s.unchanged), (1, 0));
    assert_eq!(s.refused, Some(1));
    assert_eq!(s.queued_elsewhere, None);
    assert_eq!(s.entries.len(), 2, "entries were {:?}", s.entries);

    let link = s
        .entries
        .iter()
        .find(|e| e.path == "link.txt")
        .expect("link.txt");
    assert_eq!(link.kind, "untracked");
    assert_eq!(
        link.authoring,
        Authoring::Refused(
            "precondition not met: link.txt: worktree symlink authoring is out of scope".into()
        ),
        "the reason is prikk's string, byte for byte"
    );
    // `detail`, not the prose line: no `[refused: ...]` suffix rides in the note.
    assert_eq!(link.note, "worktree file is not in the baseline");

    let readme = s
        .entries
        .iter()
        .find(|e| e.path == "readme.txt")
        .expect("readme.txt");
    assert_eq!(readme.kind, "modified");
    assert_eq!(readme.authoring, Authoring::Authored);

    // Per-kind counts, derived from the same list prikk's own counters come from.
    assert_eq!(
        (s.modified, s.missing, s.untracked, s.unsupported),
        (1, 0, 1, 0)
    );
}

#[test]
fn queued_elsewhere_reads_as_the_typed_ref() {
    let s = worktree_status(WORKTREE_QUEUED_JSON_0_41, 41).expect("parses");
    assert_eq!(s.reff, "heads/other");
    assert_eq!(
        s.queued_elsewhere,
        Some(QueuedElsewhere::Ref("heads/main".into())),
        "JSON carries the ref, never a sentence stikk would then have to pretend is prikk's"
    );
    assert_eq!(s.refused, Some(0), "a JSON report of zero is a real zero");
    assert_eq!(s.entries[0].authoring, Authoring::Authored);
}

#[test]
fn the_worktree_report_schema_is_checked_first() {
    let text = variant(
        WORKTREE_SYMLINK_JSON_0_41,
        "worktree-status-report-v1",
        "worktree-status-report-v2",
    );
    let err = worktree_status(&text, 42).unwrap_err();
    assert_eq!(err.class(), "environment");
    assert!(
        err.to_string().contains("worktree-status-report-v2"),
        "{err}"
    );
}

#[test]
fn only_prikks_two_authoring_pairs_are_read() {
    let refused_reason = r#""authoring": "refused", "refusal": "precondition not met: link.txt: worktree symlink authoring is out of scope""#;
    let authored = r#""authoring": "authored", "refusal": null"#;
    let illegal = [
        // "refused" with no reason is not a verdict stikk can show.
        variant(
            WORKTREE_SYMLINK_JSON_0_41,
            refused_reason,
            r#""authoring": "refused", "refusal": null"#,
        ),
        // "authored" with a reason contradicts itself.
        variant(
            WORKTREE_SYMLINK_JSON_0_41,
            authored,
            r#""authoring": "authored", "refusal": "why""#,
        ),
        // A third verdict word.
        variant(
            WORKTREE_SYMLINK_JSON_0_41,
            authored,
            r#""authoring": "deferred", "refusal": null"#,
        ),
        // `refusal` absent rather than null.
        variant(
            WORKTREE_SYMLINK_JSON_0_41,
            authored,
            r#""authoring": "authored""#,
        ),
    ];
    for text in illegal {
        let err = worktree_status(&text, 42).expect_err("an illegal authoring pair must not parse");
        assert_eq!(err.class(), "environment", "{err}");
    }
}

#[test]
fn a_refused_count_that_disagrees_with_the_entries_is_refused() {
    for count in ["0", "2"] {
        let text = variant(
            WORKTREE_SYMLINK_JSON_0_41,
            r#""refused_count": 1"#,
            &format!(r#""refused_count": {count}"#),
        );
        let err = worktree_status(&text, 42).expect_err("no number is picked between the two");
        assert_eq!(err.class(), "environment");
        assert!(err.to_string().contains("refused_count"), "{err}");
    }
}

#[test]
fn a_missing_queued_elsewhere_is_refused_not_read_as_nothing_queued() {
    let text = variant(
        WORKTREE_SYMLINK_JSON_0_41,
        "  \"queued_elsewhere\": null,\n",
        "",
    );
    let err = worktree_status(&text, 42).expect_err("absence is not null");
    assert_eq!(err.class(), "environment");
    assert!(err.to_string().contains("queued_elsewhere"), "{err}");
}

#[test]
fn the_ref_and_the_queued_ref_are_validated_as_ref_names() {
    // The JSON escape `` decodes to BEL — a control character no ref name may carry.
    let bad_ref = variant(
        WORKTREE_SYMLINK_JSON_0_41,
        r#""ref": "heads/main""#,
        r#""ref": "heads/main""#,
    );
    assert_eq!(
        worktree_status(&bad_ref, 42).unwrap_err().class(),
        "environment"
    );

    let bad_queued = variant(
        WORKTREE_QUEUED_JSON_0_41,
        r#""queued_elsewhere": "heads/main""#,
        r#""queued_elsewhere": """#,
    );
    assert_eq!(
        worktree_status(&bad_queued, 42).unwrap_err().class(),
        "environment"
    );
}

#[test]
fn at_0_41_an_unsupported_path_is_carried_as_reported_not_validated() {
    // **A prikk 0.41 fact, kept true of 0.41** (RFC 029 Handoff A §5): 0.41 marked these `authored`
    // with an absolute path. 0.42 marks them refused, relative — see
    // `at_0_42_every_unsupported_path_reads_as_refused_with_prikks_reason`.
    // RFC 027 decision 2: an `unsupported-path` entry's path is by definition not a safe repository
    // path — absolute, and here with a backslash and prikk's U+FFFD (the JSON escapes `\\` and
    // `�`). Validating it would drop F0's entries a second way. `authored`, as prikk 0.41 marks
    // these (F5).
    let text = variant(
        WORKTREE_SYMLINK_JSON_0_41,
        r#"{"path": "link.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "refused", "refusal": "precondition not met: link.txt: worktree symlink authoring is out of scope"}"#,
        r#"{"path": "/tmp/repo/back\\sl�sh.txt", "kind": "unsupported-path", "detail": "worktree path is not representable as a safe Prikk path", "authoring": "authored", "refusal": null}"#,
    );
    let text = variant(&text, r#""refused_count": 1"#, r#""refused_count": 0"#);
    let s = worktree_status(&text, 42).expect("parses");
    let entry = s
        .entries
        .iter()
        .find(|e| e.kind == "unsupported-path")
        .expect("listed");
    assert_eq!(entry.path, "/tmp/repo/back\\sl\u{fffd}sh.txt");
    assert_eq!(s.unsupported, 1);
}

// ---------------------------------------------------------------------------------------------
// prikk 0.42 (RFC 029 Handoff A §4–§5). Captured, never written: a real 0.42.0 binary, the harness's
// own `Fixture`, moved to the neutral `/tmp/repo` before capture, nothing edited after.
// ---------------------------------------------------------------------------------------------

/// `prikk worktree-status --ref heads/main --format json` at **0.42.0**, 2026-09-13: `readme.txt` committed
/// and sealed, then three names prikk cannot represent written beside it — `back\slash.txt`, a name with
/// the byte `0xFF`, and `nested/sub\dir.txt`. Stdout on prikk's dirty exit (1).
///
/// **What moved since 0.41** (RFC 029 F-table): every `unsupported-path` entry is `"refused"` with
/// `commit`'s own text and counted in `refused_count`, and `path` is relative to the worktree. At 0.41
/// the same names were `"authored"` and absolute (`WORKTREE_UNSUPPORTED_0_41_FIXTURE`).
const WORKTREE_UNSUPPORTED_JSON_0_42: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 1,
  "unchanged_files": 1,
  "clean": false,
  "refused_count": 3,
  "queued_elsewhere": null,
  "changes": [
    {"path": "back\\slash.txt", "kind": "unsupported-path", "detail": "worktree path is not representable as a safe Prikk path: invalid name: backslashes are not allowed in repository paths", "authoring": "refused", "refusal": "invalid name: backslashes are not allowed in repository paths"},
    {"path": "bad�name.txt", "kind": "unsupported-path", "detail": "worktree path is not representable as a safe Prikk path: invalid name: worktree path is not valid UTF-8: bad�name.txt", "authoring": "refused", "refusal": "invalid name: worktree path is not valid UTF-8: bad�name.txt"},
    {"path": "nested/sub\\dir.txt", "kind": "unsupported-path", "detail": "worktree path is not representable as a safe Prikk path: invalid name: backslashes are not allowed in repository paths", "authoring": "refused", "refusal": "invalid name: backslashes are not allowed in repository paths"}
  ],
  "declarations": []
}
"#;

/// `prikk log --ref heads/main --format json` at **0.42.0**, 2026-09-13, with `.prikk/current-branch`
/// malformed (`heads/main`, no newline) and `--ref` given explicitly, so prikk answers with
/// `"current_branch": null`. **Kept so that a reader which someday requires the field fails a test, not
/// a user** (RFC 029 Handoff A §4.3).
const LOG_CURRENT_BRANCH_NULL_0_42: &str = r#"{
  "schema_version": "log-report-v1",
  "repository": "/tmp/repo/.prikk",
  "ref": "heads/main",
  "current_branch": null,
  "blocks": [
    {
      "block_id": "a1f061a21cc916a9d85275ad9dba207f798df4b154e7a67c213a37044c5caaf1",
      "ref_state_id": "ca027b8ca6ee25ecf032eb8bda8c259057910d486849f3985ffbafdefe9acbdb",
      "update_seq": 1,
      "kind": "Root",
      "rollback_block": false,
      "parent_count": 0,
      "patch_count": 1,
      "rollback_patch_count": 0,
      "required_attestation_count": 0,
      "patch_messages": [
        {"patch_id": "319961d778c4ef2829dcf619ae2eae08c6c91cbb023c8fa65ca10e81d6b55fdf", "message": "first patch"}
      ],
      "previous_ref_state_id": null
    }
  ]
}
"#;

#[test]
fn at_0_42_every_unsupported_path_reads_as_refused_with_prikks_reason() {
    // RFC 029 Handoff A §5. prikk 0.42 began reporting unrepresentable names as refused — the answer
    // RFC 027's Q1 ruling (b) waited for. Nothing in stikk's reader changed to read it.
    let s = worktree_status(WORKTREE_UNSUPPORTED_JSON_0_42, 42).expect("captured 0.42 report");
    assert_eq!(s.unsupported, 3);
    assert_eq!(s.refused, Some(3), "refused_count counts them");
    let mut paths: Vec<&str> = Vec::new();
    for entry in &s.entries {
        assert_eq!(entry.kind, "unsupported-path", "{entry:?}");
        let Authoring::Refused(reason) = &entry.authoring else {
            panic!("0.42 marks every unsupported-path refused: {entry:?}");
        };
        assert!(
            entry.note.ends_with(reason.as_str()),
            "prikk's detail ends with commit's own reason: {entry:?}"
        );
        assert!(
            !entry.path.starts_with('/'),
            "0.42 reports the path relative to the worktree: {entry:?}"
        );
        paths.push(&entry.path);
    }
    paths.sort_unstable();
    assert_eq!(
        paths,
        [
            "back\\slash.txt",
            "bad\u{fffd}name.txt",
            "nested/sub\\dir.txt"
        ]
    );
    let backslash = s
        .entries
        .iter()
        .find(|e| e.path == "back\\slash.txt")
        .expect("listed");
    assert_eq!(
        backslash.authoring,
        Authoring::Refused("invalid name: backslashes are not allowed in repository paths".into())
    );
}

#[test]
fn a_0_42_json_report_whose_current_branch_is_null_still_reads() {
    // RFC 029 Handoff A §4.3: stikk reads no `current_branch` at A; a `null` there must not fail a read.
    assert!(LOG_CURRENT_BRANCH_NULL_0_42.contains("\"current_branch\": null"));
    let h = history(LOG_CURRENT_BRANCH_NULL_0_42).expect("captured 0.42 log report");
    assert_eq!(h.reff, "heads/main");
    assert_eq!(h.blocks.len(), 1);
}

// ---------------------------------------------------------------------------------------------
// RFC 030 amendment A1 — `declarations`, read and required.
// ---------------------------------------------------------------------------------------------

/// `prikk worktree-status --ref heads/main --format json` at **0.42.0**, 2026-09-15 (RFC 030 handoff v2
/// §5a, M7): the harness's `Fixture` with `a.txt` committed and sealed, moved to the neutral `/tmp/repo`
/// before capture; then `mv a.txt b.txt` in the shell, then `prikk mv a.txt b.txt`, which printed
/// `declared a.txt -> b.txt (already moved on disk; no bytes touched)`. Stdout on prikk's dirty exit (1).
/// Nothing edited after capture.
///
/// **The shape that made declarations part of what commit's confirmation compares**: the same two
/// `changes` a preview taken before `prikk mv` lists, and one declaration that makes `commit` author
/// `rename-path a.txt -> b.txt`.
const WORKTREE_DECLARATION_JSON_0_42: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 2,
  "unchanged_files": 1,
  "clean": false,
  "refused_count": 0,
  "queued_elsewhere": null,
  "changes": [
    {"path": "a.txt", "kind": "missing", "detail": "tracked file is absent from the worktree", "authoring": "authored", "refusal": null},
    {"path": "b.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": [
    {"old_path": "a.txt", "new_path": "b.txt"}
  ]
}
"#;

#[test]
fn a_0_42_declaration_is_read_beside_the_entries_it_leaves_unchanged() {
    let s = worktree_status(WORKTREE_DECLARATION_JSON_0_42, 42).expect("captured 0.42 report");
    assert_eq!(
        s.declarations,
        vec![RenameDeclaration {
            // The 0.42 capture predates prikk's own verdict, and the reader is told so.
            resolution: None,
            refusal: None,
            content_changed: None,
            mode_changed: None,
            // The 0.42 capture predates prikk's own verdict, and the reader is told so.
            old_path: "a.txt".into(),
            new_path: "b.txt".into(),
        }]
    );
    let entries: Vec<(&str, &str)> = s
        .entries
        .iter()
        .map(|e| (e.kind.as_str(), e.path.as_str()))
        .collect();
    assert_eq!(entries, [("missing", "a.txt"), ("untracked", "b.txt")]);
}

#[test]
fn the_other_json_worktree_fixtures_report_no_declarations() {
    for (name, text) in [
        ("WORKTREE_SYMLINK_JSON_0_41", WORKTREE_SYMLINK_JSON_0_41),
        ("WORKTREE_QUEUED_JSON_0_41", WORKTREE_QUEUED_JSON_0_41),
        (
            "WORKTREE_UNSUPPORTED_JSON_0_42",
            WORKTREE_UNSUPPORTED_JSON_0_42,
        ),
    ] {
        let s = worktree_status(text, 42).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(s.declarations.is_empty(), "{name}: {:?}", s.declarations);
    }
}

#[test]
fn a_missing_declarations_array_is_refused_not_read_as_none() {
    // Its absence cannot mean "none": a declaration changes what commit authors without changing `changes`.
    let text = variant(
        WORKTREE_SYMLINK_JSON_0_41,
        r#""declarations": []"#,
        r#""declarations_moved": []"#,
    );
    let err = worktree_status(&text, 42).expect_err("a report without its declarations");
    assert_eq!(err.class(), "environment");
    assert!(err.to_string().contains("declarations"), "{err}");
}

#[test]
fn a_declaration_without_both_paths_is_refused() {
    let text = variant(
        WORKTREE_DECLARATION_JSON_0_42,
        r#""new_path": "b.txt""#,
        r#""destination": "b.txt""#,
    );
    let err = worktree_status(&text, 42).expect_err("a declaration missing new_path");
    assert_eq!(err.class(), "environment");
    assert!(err.to_string().contains("new_path"), "{err}");
}

// ---------------------------------------------------------------------------------------------
// RFC 028 Handoff A: `status --format json`'s queue (`status-report-v1`), prikk >= 0.39.
// ---------------------------------------------------------------------------------------------

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// two patches committed onto a sealed `heads/main`: `add b`, then `prikk mv a.txt c.txt` and `rename a to c`.
// Capture file: `status-queue-two-patch-0.42.json`.
const STATUS_QUEUE_TWO_PATCH_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 2,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": "20444fe9438c81bffa57c9b1ddf7b59f6a1bb57ea18f2d0833d019f7e0a9fb29",
  "current_branch": "heads/main",
  "queue": {
    "count": 2,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a", "message": "add b", "operations": [
        {"kind": "create-file", "paths": [{"path": "b.txt"}]}
      ]},
      {"patch_id": "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478", "message": "rename a to c", "operations": [
        {"kind": "rename-path", "paths": [{"path": "a.txt"},{"path": "c.txt"}], "author_key_id": "author"}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// `doc.txt` committed and sealed; then edited and committed (`edit doc`); then deleted and committed (`delete doc`) — `STATUS_JSON_UNRESOLVED_NODE_0_38_FIXTURE`'s recipe, now with messages.
// Capture file: `status-queue-unresolved-node-0.42.json`.
const STATUS_QUEUE_UNRESOLVED_NODE_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 2,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": "983f0990e322a10333897533deca9d3911d70da14597f240a5335cd85c73d594",
  "current_branch": "heads/main",
  "queue": {
    "count": 2,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "af0fb3f2d7c2b509c7383a23a1c387a2134fdad26f825f3ad446c0c5a7c59029", "message": "edit doc", "operations": [
        {"kind": "edit-text", "paths": [{"unresolved_node_id": "751ae57a14ec0a61402a5118108c41dedd8e5d409c716faaa79a584c2edea6e1"}]}
      ]},
      {"patch_id": "1cab8c21fe1639c54e2f8521ecbfcd53f057bfce9567e0a3991b4a40f222048e", "message": "delete doc", "operations": [
        {"kind": "delete-node", "paths": [{"path": "doc.txt"}]}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.41.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// the same two-patch recipe as `STATUS_QUEUE_TWO_PATCH_0_42`, on prikk 0.41.0: no `message` field, and no `current_branch`.
// Capture file: `status-queue-two-patch-0.41.json`.
const STATUS_QUEUE_TWO_PATCH_0_41: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 2,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": "309e32e147c9c34528917a1a50cedbdce114ac6936d047f6c7cbee5a69b80dbf",
  "queue": {
    "count": 2,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "49d234ae86c48f9126a84acc26d847836b361d8a923081b16188bcc16293e15f", "operations": [
        {"kind": "create-file", "paths": [{"path": "b.txt"}]}
      ]},
      {"patch_id": "80ed9246a501996bfc41b1d9ff0d3f6a7e36d636e47e3d8be08a7c6e73e0ae14", "operations": [
        {"kind": "rename-path", "paths": [{"path": "a.txt"},{"path": "c.txt"}], "author_key_id": "author"}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// one patch committed on a fresh repository, read with `PRIKK_ACTIVE_PATCH_WARN=1` (RFC 028 F3's recipe).
// Capture file: `status-queue-warn-0.42.json`.
const STATUS_QUEUE_WARN_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 1,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": null,
  "current_branch": "heads/main",
  "queue": {
    "count": 1,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "warn",
    "warn_threshold": 1,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "3829ccfb080c07011497119384f429bbcf32395695bfc6030dc3fe20da56f321", "message": "one patch", "operations": [
        {"kind": "create-file", "paths": [{"path": "w.txt"}]}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// a freshly initialised repository with nothing queued.
// Capture file: `status-queue-empty-0.42.json`.
const STATUS_QUEUE_EMPTY_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 0,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": null,
  "current_branch": "heads/main",
  "queue": {
    "count": 0,
    "target_ref": null,
    "target_ref_status": null,
    "threshold_status": null,
    "warn_threshold": null,
    "hard_limit": null,
    "patches": []
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:46Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// the two-patch recipe committed by prikk 0.41.0, then read by prikk 0.42.0. **Both messages are present**: 0.41 already stored them, so this is not a route to a `null` message.
// Capture file: `status-queue-written-0.41-read-0.42.json`.
const STATUS_QUEUE_WRITTEN_0_41_READ_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 2,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": "2d84fce3caa5c24f8e6dadb0988b21a3364ed4934139a740d7b0c51aa6ae3544",
  "current_branch": "heads/main",
  "queue": {
    "count": 2,
    "target_ref": "heads/main",
    "target_ref_status": null,
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "465da48a7dfdb54b26ce003ba2a18f4d8113bc618eb8a9b54f0b479554197714", "message": "add b", "operations": [
        {"kind": "create-file", "paths": [{"path": "b.txt"}]}
      ]},
      {"patch_id": "e212f78fa7622fc41ddf2421e83d45c5eeab41a1395a12479d4fd2b63b9a431c", "message": "rename a to c", "operations": [
        {"kind": "rename-path", "paths": [{"path": "a.txt"},{"path": "c.txt"}], "author_key_id": "author"}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:50Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// one patch committed, then **constructed state**: `.prikk/active/default/ref-name` emptied in a scratch repository, since prikk's CLI cannot produce it. prikk's output is verbatim.
// Capture file: `status-queue-missing-metadata-0.42.json`.
const STATUS_QUEUE_MISSING_METADATA_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 1,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": null,
  "current_branch": "heads/main",
  "queue": {
    "count": 1,
    "target_ref": null,
    "target_ref_status": "missing-metadata",
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "a1fe84d365fd2bdbf237253fccbef77dd850463dd14c60f4d38e516037ce29bf", "message": "queued", "operations": [
        {"kind": "create-file", "paths": [{"path": "q.txt"}]}
      ]}
    ]
  }
}
"#;

// Captured verbatim from a real prikk 0.42.0 binary on 2026-09-15T05:50Z, from a neutral `/tmp/repo` (RFC 028 Handoff A §2):
// one patch committed, then **constructed state**: `.prikk/active/default/ref-name` set to `not a ref` in a scratch repository. prikk's output is verbatim.
// Capture file: `status-queue-malformed-metadata-0.42.json`.
const STATUS_QUEUE_MALFORMED_METADATA_0_42: &str = r#"{
  "schema_version": "status-report-v1",
  "repository": "/tmp/repo/.prikk",
  "active_wal_records": 1,
  "trailing_partial_wal_bytes": 0,
  "heads_main_ref_state": null,
  "current_branch": "heads/main",
  "queue": {
    "count": 1,
    "target_ref": null,
    "target_ref_status": "malformed-metadata",
    "threshold_status": "none",
    "warn_threshold": 800,
    "hard_limit": 1000,
    "patches": [
      {"patch_id": "a1fe84d365fd2bdbf237253fccbef77dd850463dd14c60f4d38e516037ce29bf", "message": "queued", "operations": [
        {"kind": "create-file", "paths": [{"path": "q.txt"}]}
      ]}
    ]
  }
}
"#;

/// **Constructed, not captured.** No honest capture produced a `null` message: prikk 0.42 requires `-m`,
/// and a queue committed by 0.41 carries its messages when 0.42 reads it
/// (`STATUS_QUEUE_WRITTEN_0_41_READ_0_42`). prikk's emitter writes `null` for a patch with no message
/// (`output/status.rs` at the 0.42.0 tag), so this is `STATUS_QUEUE_WARN_0_42` with that one value
/// replaced — the only case in this section tested against a literal stikk wrote.
fn constructed_null_message_0_42() -> String {
    STATUS_QUEUE_WARN_0_42.replace(r#""message": "one patch""#, r#""message": null"#)
}

fn queue_at(text: &str, minor: u32) -> Queue {
    queue(text, minor).unwrap_or_else(|e| panic!("0.{minor}: {e}"))
}

fn queue_rule_error(text: &str, minor: u32) -> String {
    let err = queue(text, minor).expect_err("a broken rule must refuse");
    assert_eq!(err.class(), "environment", "{err}");
    err.to_string()
}

#[test]
fn the_captured_two_patch_queue_parses_to_what_it_shows_at_0_42() {
    let q = queue_at(STATUS_QUEUE_TWO_PATCH_0_42, 42);
    assert_eq!(q.count, 2);
    assert_eq!(q.target, QueueTarget::Ref("heads/main".into()));
    assert_eq!(
        q.threshold,
        Some(QueueThreshold {
            status: ThresholdStatus::None,
            warn: 800,
            hard_limit: 1000
        })
    );
    assert_eq!(
        q.patches,
        vec![
            QueuedPatch {
                patch_id: "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a".into(),
                message: QueuedMessage::Text("add b".into()),
                operations: vec![QueuedOperation {
                    kind: "create-file".into(),
                    paths: vec![QueuedPath::Path("b.txt".into())],
                    author_key_id: None,
                }],
            },
            QueuedPatch {
                patch_id: "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478".into(),
                message: QueuedMessage::Text("rename a to c".into()),
                operations: vec![QueuedOperation {
                    kind: "rename-path".into(),
                    paths: vec![
                        QueuedPath::Path("a.txt".into()),
                        QueuedPath::Path("c.txt".into())
                    ],
                    author_key_id: Some("author".into()),
                }],
            },
        ]
    );
}

#[test]
fn the_captured_unresolved_node_queue_keeps_the_node_apart_from_a_path() {
    let q = queue_at(STATUS_QUEUE_UNRESOLVED_NODE_0_42, 42);
    assert_eq!(q.count, 2);
    assert_eq!(q.patches[0].message, QueuedMessage::Text("edit doc".into()));
    assert_eq!(
        q.patches[0].operations,
        vec![QueuedOperation {
            kind: "edit-text".into(),
            paths: vec![QueuedPath::UnresolvedNode(
                "751ae57a14ec0a61402a5118108c41dedd8e5d409c716faaa79a584c2edea6e1".into()
            )],
            author_key_id: None,
        }]
    );
    assert_eq!(
        q.patches[1].message,
        QueuedMessage::Text("delete doc".into())
    );
    assert_eq!(
        q.patches[1].operations[0].paths,
        vec![QueuedPath::Path("doc.txt".into())]
    );
}

#[test]
fn at_0_41_the_message_is_not_reported_never_none() {
    let q = queue_at(STATUS_QUEUE_TWO_PATCH_0_41, 41);
    assert_eq!(q.count, 2);
    for patch in &q.patches {
        assert_eq!(patch.message, QueuedMessage::NotReported, "{patch:?}");
    }
    assert_eq!(q.patches[1].operations[0].kind, "rename-path");
    assert_eq!(
        q.patches[1].operations[0].author_key_id.as_deref(),
        Some("author")
    );
}

#[test]
fn the_three_message_states_stay_distinct() {
    let text = queue_at(STATUS_QUEUE_WARN_0_42, 42).patches[0]
        .message
        .clone();
    let none = queue_at(&constructed_null_message_0_42(), 42).patches[0]
        .message
        .clone();
    let not_reported = queue_at(STATUS_QUEUE_TWO_PATCH_0_41, 41).patches[0]
        .message
        .clone();
    assert_eq!(text, QueuedMessage::Text("one patch".into()));
    assert_eq!(none, QueuedMessage::None);
    assert_eq!(not_reported, QueuedMessage::NotReported);
    // A queue written by 0.41 and read by 0.42 is not a route to `null`: both messages survive.
    let mixed = queue_at(STATUS_QUEUE_WRITTEN_0_41_READ_0_42, 42);
    assert_eq!(
        mixed.patches[0].message,
        QueuedMessage::Text("add b".into())
    );
    assert_eq!(
        mixed.patches[1].message,
        QueuedMessage::Text("rename a to c".into())
    );
}

#[test]
fn a_warn_threshold_is_carried_with_prikks_numbers() {
    let q = queue_at(STATUS_QUEUE_WARN_0_42, 42);
    assert_eq!(
        q.threshold,
        Some(QueueThreshold {
            status: ThresholdStatus::Warn,
            warn: 1,
            hard_limit: 1000
        })
    );
}

#[test]
fn an_empty_queue_has_no_target_no_threshold_and_no_patches() {
    let q = queue_at(STATUS_QUEUE_EMPTY_0_42, 42);
    assert_eq!(q.count, 0);
    assert_eq!(q.target, QueueTarget::NotReported);
    assert_eq!(q.threshold, None);
    assert!(q.patches.is_empty());
}

#[test]
fn missing_and_malformed_metadata_are_held_apart_from_a_ref_and_from_nothing() {
    let missing = queue_at(STATUS_QUEUE_MISSING_METADATA_0_42, 42);
    assert_eq!(missing.count, 1);
    assert_eq!(missing.target, QueueTarget::MissingMetadata);
    let malformed = queue_at(STATUS_QUEUE_MALFORMED_METADATA_0_42, 42);
    assert_eq!(malformed.target, QueueTarget::MalformedMetadata);
}

// Each §3 rule, broken once. Every one is stikk's environment error, never prikk's refusal.

#[test]
fn an_unknown_status_schema_is_refused() {
    let text = STATUS_QUEUE_TWO_PATCH_0_42.replace("status-report-v1", "status-report-v2");
    let err = queue_rule_error(&text, 42);
    assert!(
        err.contains("status-report-v2") && err.contains("status-report-v1"),
        "{err}"
    );
}

#[test]
fn a_report_without_a_queue_is_refused() {
    let err = queue_rule_error(
        r#"{"schema_version": "status-report-v1", "active_wal_records": 0}"#,
        42,
    );
    assert!(err.contains("`queue`"), "{err}");
}

#[test]
fn a_count_that_disagrees_with_the_patches_listed_is_refused() {
    let text = STATUS_QUEUE_TWO_PATCH_0_42.replace(r#""count": 2"#, r#""count": 3"#);
    let err = queue_rule_error(&text, 42);
    assert!(
        err.contains("`count` is 3 but 2 patch(es) are listed"),
        "{err}"
    );
}

#[test]
fn a_target_ref_that_is_not_a_ref_name_is_refused() {
    let text = STATUS_QUEUE_TWO_PATCH_0_42.replace(
        r#""target_ref": "heads/main""#,
        "\"target_ref\": \"heads/\\u001b[2Jmain\"",
    );
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("target_ref"), "{err}");
}

#[test]
fn a_target_ref_status_outside_its_vocabulary_is_refused() {
    let text = STATUS_QUEUE_MISSING_METADATA_0_42.replace("missing-metadata", "lost-metadata");
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("lost-metadata"), "{err}");
}

#[test]
fn a_target_ref_beside_a_metadata_status_is_refused() {
    let text = STATUS_QUEUE_MISSING_METADATA_0_42
        .replace(r#""target_ref": null"#, r#""target_ref": "heads/main""#);
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("beside"), "{err}");
}

#[test]
fn a_threshold_status_outside_its_vocabulary_is_refused() {
    let text = STATUS_QUEUE_WARN_0_42.replace(
        r#""threshold_status": "warn""#,
        r#""threshold_status": "high""#,
    );
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("\"high\""), "{err}");
}

#[test]
fn a_null_threshold_on_a_non_empty_queue_is_refused() {
    let text = STATUS_QUEUE_WARN_0_42.replace(
        r#""threshold_status": "warn""#,
        r#""threshold_status": null"#,
    );
    let err = queue_rule_error(&text, 42);
    assert!(
        err.contains("null exactly when the queue is empty"),
        "{err}"
    );
}

#[test]
fn a_threshold_on_an_empty_queue_is_refused() {
    let text =
        STATUS_QUEUE_EMPTY_0_42.replace(r#""warn_threshold": null"#, r#""warn_threshold": 800"#);
    let err = queue_rule_error(&text, 42);
    assert!(
        err.contains("null exactly when the queue is empty"),
        "{err}"
    );
}

#[test]
fn a_patch_id_that_is_not_an_object_id_is_refused() {
    let text = STATUS_QUEUE_WARN_0_42.replace(
        "3829ccfb080c07011497119384f429bbcf32395695bfc6030dc3fe20da56f321",
        "3829ccfb",
    );
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("patch_id"), "{err}");
}

#[test]
fn a_path_object_must_carry_exactly_one_of_path_and_unresolved_node_id() {
    let both = STATUS_QUEUE_WARN_0_42.replace(
        r#"{"path": "w.txt"}"#,
        r#"{"path": "w.txt", "unresolved_node_id": "3829ccfb080c07011497119384f429bbcf32395695bfc6030dc3fe20da56f321"}"#,
    );
    let neither = STATUS_QUEUE_WARN_0_42.replace(r#"{"path": "w.txt"}"#, "{}");
    for text in [both, neither] {
        let err = queue_rule_error(&text, 42);
        assert!(
            err.contains("exactly one of `path` and `unresolved_node_id`"),
            "{err}"
        );
    }
}

#[test]
fn a_rename_without_its_author_key_id_is_refused() {
    let text = STATUS_QUEUE_TWO_PATCH_0_42.replace(r#", "author_key_id": "author""#, "");
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("author_key_id"), "{err}");
}

#[test]
fn at_0_42_a_patch_without_a_message_field_is_refused() {
    let err = queue_rule_error(STATUS_QUEUE_TWO_PATCH_0_41, 42);
    assert!(err.contains("no `message` field"), "{err}");
}

#[test]
fn below_0_42_a_patch_with_a_message_field_is_refused() {
    let err = queue_rule_error(STATUS_QUEUE_TWO_PATCH_0_42, 41);
    assert!(err.contains("carries a `message` field"), "{err}");
}

#[test]
fn at_0_42_a_message_of_the_wrong_type_is_refused() {
    let text = STATUS_QUEUE_WARN_0_42.replace(r#""message": "one patch""#, r#""message": 7"#);
    let err = queue_rule_error(&text, 42);
    assert!(err.contains("string or null"), "{err}");
}

// ---------------------------------------------------------------------------------------------
// RFC 034 decision 4: prikk's own verdict on each declaration, read only inside its band (≥ 0.44).
// Every fixture below is captured verbatim from the binary its name gives.
// ---------------------------------------------------------------------------------------------

/// `prikk worktree-status --ref heads/main --format json` at **0.46.0**: `a.txt` sealed, `prikk mv a.txt
/// b.txt`, then `b.txt` edited.
const WORKTREE_RENAME_CONTENT_JSON_0_46: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/probe/rc/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 2,
  "unchanged_files": 1,
  "clean": false,
  "refused_count": 0,
  "refused_declaration_count": 0,
  "queued_elsewhere": null,
  "changes": [
    {"path": "a.txt", "kind": "missing", "detail": "tracked file is absent from the worktree", "authoring": "authored", "refusal": null},
    {"path": "b.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": [
    {"old_path": "a.txt", "new_path": "b.txt", "resolution": "rename", "refusal": null, "content_changed": true, "mode_changed": false}
  ]
}"#;

/// The same command at **0.46.0** after `prikk mv a.txt b.txt` and a shell `mv b.txt a.txt`: prikk
/// reports the worktree **clean** and exits 0, and still refuses the declaration.
const WORKTREE_REFUSED_DECLARATION_JSON_0_46: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/probe/rb/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 2,
  "unchanged_files": 2,
  "clean": true,
  "refused_count": 0,
  "refused_declaration_count": 1,
  "queued_elsewhere": null,
  "changes": [],
  "declarations": [
    {"old_path": "a.txt", "new_path": "b.txt", "resolution": "refused", "refusal": "a.txt -> b.txt: the source is present in the worktree again, so the declared move is not what the worktree holds. Run `prikk mv b.txt a.txt` to drop the declaration, or `prikk mv a.txt b.txt` to make the move again", "content_changed": null, "mode_changed": null}
  ]
}"#;

/// The same command at **0.46.0** with the destination listed in `.prikkignore`.
const WORKTREE_DELETION_IGNORED_JSON_0_46: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/probe/ri/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 2,
  "unchanged_files": 1,
  "clean": false,
  "refused_count": 0,
  "refused_declaration_count": 0,
  "queued_elsewhere": null,
  "changes": [
    {"path": ".prikkignore", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null},
    {"path": "a.txt", "kind": "missing", "detail": "tracked file is absent from the worktree", "authoring": "authored", "refusal": null}
  ],
  "declarations": [
    {"old_path": "a.txt", "new_path": "b.txt", "resolution": "deletion-ignored", "refusal": null, "content_changed": null, "mode_changed": null}
  ]
}"#;

/// The same state as `WORKTREE_RENAME_CONTENT_JSON_0_46`, captured from **0.43.0** — which reports every
/// field and whose classifier stikk does not trust (RFC 034 §1).
const WORKTREE_RENAME_CONTENT_JSON_0_43: &str = r#"{
  "schema_version": "worktree-status-report-v1",
  "repository": "/tmp/probe/rc/.prikk",
  "ref": "heads/main",
  "current_branch": "heads/main",
  "tracked_files": 2,
  "unchanged_files": 1,
  "clean": false,
  "refused_count": 0,
  "refused_declaration_count": 0,
  "queued_elsewhere": null,
  "changes": [
    {"path": "a.txt", "kind": "missing", "detail": "tracked file is absent from the worktree", "authoring": "authored", "refusal": null},
    {"path": "b.txt", "kind": "untracked", "detail": "worktree file is not in the baseline", "authoring": "authored", "refusal": null}
  ],
  "declarations": [
    {"old_path": "a.txt", "new_path": "b.txt", "resolution": "rename", "refusal": null, "content_changed": true, "mode_changed": false}
  ]
}"#;

#[test]
fn prikks_own_resolution_and_its_content_flags_are_read_at_0_44_and_above() {
    let s = worktree_status(WORKTREE_RENAME_CONTENT_JSON_0_46, 46).expect("captured 0.46 report");
    assert_eq!(s.refused_declarations, Some(0));
    let declaration = &s.declarations[0];
    assert_eq!(declaration.old_path, "a.txt");
    assert_eq!(declaration.new_path, "b.txt");
    assert_eq!(
        declaration.resolution,
        Some(DeclarationResolution::Rename),
        "prikk's own word, not stikk's inference"
    );
    assert_eq!(declaration.content_changed, Some(true));
    assert_eq!(declaration.mode_changed, Some(false));
    assert_eq!(declaration.refusal, None);
}

#[test]
fn a_refused_declaration_carries_prikks_refusal_and_is_counted_apart_from_paths() {
    let s =
        worktree_status(WORKTREE_REFUSED_DECLARATION_JSON_0_46, 46).expect("captured 0.46 report");
    // The row that makes the two counts different facts, and the clean check dangerous.
    assert!(s.clean, "prikk reports this worktree clean");
    assert_eq!(s.refused, Some(0), "no path is refused");
    assert_eq!(s.refused_declarations, Some(1), "one declaration is");
    let declaration = &s.declarations[0];
    assert_eq!(declaration.resolution, Some(DeclarationResolution::Refused));
    assert_eq!(
        declaration.refusal.as_deref(),
        Some(
            "a.txt -> b.txt: the source is present in the worktree again, so the declared move is not \
             what the worktree holds. Run `prikk mv b.txt a.txt` to drop the declaration, or `prikk mv \
             a.txt b.txt` to make the move again"
        ),
        "prikk's words, verbatim"
    );
    // Unknown, never "unchanged" (`C-T2c′`).
    assert_eq!(declaration.content_changed, None);
    assert_eq!(declaration.mode_changed, None);
}

#[test]
fn an_ignored_destination_is_prikks_own_word_for_it() {
    let s = worktree_status(WORKTREE_DELETION_IGNORED_JSON_0_46, 46).expect("captured 0.46 report");
    assert_eq!(
        s.declarations[0].resolution,
        Some(DeclarationResolution::DeletionIgnored)
    );
    assert_eq!(
        s.declarations[0]
            .resolution
            .as_ref()
            .map(DeclarationResolution::label),
        Some("deletion-ignored")
    );
}

#[test]
fn below_the_band_prikks_verdict_is_not_read_even_though_0_43_reports_it() {
    // RFC 034 §1: 0.43 carries every field, and stikk ignores them there — its classifier resolved a
    // directory destination as `rename` while `commit` recorded a deletion (measured at 0.43.0).
    let s = worktree_status(WORKTREE_RENAME_CONTENT_JSON_0_43, 43).expect("captured 0.43 report");
    assert_eq!(s.refused_declarations, None, "unreported, never zero");
    let declaration = &s.declarations[0];
    assert_eq!(declaration.old_path, "a.txt");
    assert_eq!(declaration.new_path, "b.txt");
    assert_eq!(declaration.resolution, None);
    assert_eq!(declaration.content_changed, None);
    assert_eq!(declaration.mode_changed, None);

    // The very same bytes, read as the band's first release, are read in full.
    let inside = worktree_status(WORKTREE_RENAME_CONTENT_JSON_0_43, 44).expect("parses");
    assert_eq!(
        inside.declarations[0].resolution,
        Some(DeclarationResolution::Rename)
    );
}

#[test]
fn a_refused_resolution_without_prikks_refusal_is_a_shape_error_not_a_guess() {
    let text = WORKTREE_REFUSED_DECLARATION_JSON_0_46.replace(
        r#""refusal": "a.txt -> b.txt: the source is present in the worktree again, so the declared move is not what the worktree holds. Run `prikk mv b.txt a.txt` to drop the declaration, or `prikk mv a.txt b.txt` to make the move again""#,
        r#""refusal": null"#,
    );
    let err = worktree_status(&text, 46).expect_err("stikk will not invent prikk's refusal");
    assert_eq!(err.class(), "environment");
    assert!(err.to_string().contains("refused"), "{err}");
}

#[test]
fn a_refusal_on_a_resolution_that_is_not_refused_is_a_shape_error() {
    let text = WORKTREE_RENAME_CONTENT_JSON_0_46
        .replace(r#""refusal": null"#, r#""refusal": "something""#);
    let err = worktree_status(&text, 46).expect_err("a refusal belongs only to `refused`");
    assert_eq!(err.class(), "environment");
}

#[test]
fn a_declaration_missing_its_content_flag_is_a_shape_error_not_unknown() {
    let text = WORKTREE_RENAME_CONTENT_JSON_0_46.replace(r#""content_changed": true, "#, "");
    let err = worktree_status(&text, 46).expect_err("absence is not null");
    assert_eq!(err.class(), "environment");
    assert!(err.to_string().contains("content_changed"), "{err}");
}

#[test]
fn the_refused_declaration_count_must_match_the_declarations_listed() {
    let text = WORKTREE_REFUSED_DECLARATION_JSON_0_46.replace(
        r#""refused_declaration_count": 1"#,
        r#""refused_declaration_count": 2"#,
    );
    let err = worktree_status(&text, 46).expect_err("no number is picked between the two");
    assert_eq!(err.class(), "environment");
    assert!(
        err.to_string().contains("refused_declaration_count"),
        "{err}"
    );
}

#[test]
fn an_unmodelled_resolution_is_kept_verbatim_rather_than_dropped() {
    let text = WORKTREE_RENAME_CONTENT_JSON_0_46
        .replace(r#""resolution": "rename""#, r#""resolution": "teleport""#);
    let s = worktree_status(&text, 46).expect("an unknown word is not a broken report");
    assert_eq!(
        s.declarations[0].resolution,
        Some(DeclarationResolution::Other("teleport".to_string()))
    );
    assert_eq!(
        s.declarations[0]
            .resolution
            .as_ref()
            .map(DeclarationResolution::label),
        Some("teleport")
    );
}

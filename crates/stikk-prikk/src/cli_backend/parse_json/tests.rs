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
    let s = worktree_status(WORKTREE_SYMLINK_JSON_0_41).expect("parses");
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
    let s = worktree_status(WORKTREE_QUEUED_JSON_0_41).expect("parses");
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
    let err = worktree_status(&text).unwrap_err();
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
        let err = worktree_status(&text).expect_err("an illegal authoring pair must not parse");
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
        let err = worktree_status(&text).expect_err("no number is picked between the two");
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
    let err = worktree_status(&text).expect_err("absence is not null");
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
        worktree_status(&bad_ref).unwrap_err().class(),
        "environment"
    );

    let bad_queued = variant(
        WORKTREE_QUEUED_JSON_0_41,
        r#""queued_elsewhere": "heads/main""#,
        r#""queued_elsewhere": """#,
    );
    assert_eq!(
        worktree_status(&bad_queued).unwrap_err().class(),
        "environment"
    );
}

#[test]
fn an_unsupported_path_is_carried_as_reported_not_validated() {
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
    let s = worktree_status(&text).expect("parses");
    let entry = s
        .entries
        .iter()
        .find(|e| e.kind == "unsupported-path")
        .expect("listed");
    assert_eq!(entry.path, "/tmp/repo/back\\sl\u{fffd}sh.txt");
    assert_eq!(s.unsupported, 1);
}

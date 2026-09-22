//! The JSON readers for prikk's three machine-readable reports (RFC 026 §6).
//!
//! prikk 0.39 gave `log`, `branch` and `tag` a `--format json` — `log-report-v1`, `branch-list-v1`,
//! `tag-list-v1`. **These three were the entire remainder of stikk's prose parsing**, and each has
//! already cost a re-baseline: `log`'s shape moved at 0.30, 0.32 and 0.35; `branch list --all` quietly
//! included tag refs until 0.39 quietly stopped.
//!
//! # Both paths stay
//!
//! **JSON at ≥ 0.39, prose below.** The floor is 0.28 and the prose parsers are the only thing that
//! works there, so they are not deleted and their fixtures are not retired — a parser that is still
//! reachable is a parser that is still tested. The version gate lives in `cli_backend.rs`, beside the
//! call that chooses the flag, rather than here.
//!
//! # Names and ids are validated here, exactly as the prose readers validate them
//!
//! `parse.rs`'s module doc says **every ref name crossing this boundary goes through
//! [`RefName::parse`], and every id through [`ObjectId::parse`]** (`INV-9`/`UD-02`; RFC 012 F-d,
//! RFC 009 F2). When these readers first landed they did neither, which made that sentence false for
//! the seam as a whole at prikk ≥ 0.39 — that is, on the path carrying all the traffic. A shape prikk
//! would never emit means stikk misread prikk, so it refuses rather than travelling onward as an
//! unvalidated string.
//!
//! **This is not about crashing.** Nothing downstream panics on a short id — `ObjectId::abbreviated`
//! and the History view both slice defensively, and `inert()` neutralizes a control-bearing name at the
//! cell (`C-T2a`). It is the defence-in-depth boundary this project chose twice, applied at the only
//! place it can be: where prikk's bytes become stikk's values.
//!
//! Struct fields stay `String` above this boundary, as they do for the prose readers — the guarantee
//! lives at the parse boundary, not in every downstream type (`parse.rs`'s `required_ref_field`
//! records the measurement behind that call).
//!
//! # The schema version is checked, and checked loudly
//!
//! Every report names its own schema. stikk verifies that name before reading a field: a
//! `log-report-v2` is not something to parse optimistically and hope the fields it wanted survived —
//! that is the RFC 012 F-e failure shape, applied to JSON. An unknown schema is an environment error
//! naming both what was found and what stikk knows, the same way the version gate does.

use stikk_model::{ObjectId, RefName, StikkError};

use crate::json::{self, Json};
use crate::{
    Authoring, BlockRow, DeclarationResolution, History, PatchMessage, Queue, QueueTarget,
    QueueThreshold, QueuedElsewhere, QueuedMessage, QueuedOperation, QueuedPatch, QueuedPath,
    RefEntry, RenameDeclaration, ThresholdStatus, WorktreeEntry, WorktreeStatus,
};

type Result<T> = std::result::Result<T, StikkError>;

/// The schema names stikk knows how to read. Checked rather than assumed (see the module doc).
const LOG_SCHEMA: &str = "log-report-v1";
const BRANCH_SCHEMA: &str = "branch-list-v1";
const TAG_SCHEMA: &str = "tag-list-v1";
const STATUS_SCHEMA: &str = "status-report-v1";

/// The first prikk minor whose queued patches carry `message` (prikk 0.42; RFC 028 as revised).
const QUEUED_MESSAGE_FROM_MINOR: u32 = 42;

/// A required ref-name field, validated at this boundary (`INV-9`; RFC 012 F-d).
///
/// The error names the field **and** the value, the way the prose readers' do: a caller debugging this
/// needs to see the shape prikk sent, and an environment error is the right class because a shape prikk
/// would never emit means stikk misread prikk, not that the user did anything.
fn ref_name_field<'a>(value: &'a Json, key: &str) -> Result<&'a str> {
    let name = value.str_field(key)?;
    RefName::parse(name).map_err(|_| {
        StikkError::environment_msg(format!(
            "prikk's JSON report has an unrecognized ref name in `{key}`: {name:?}"
        ))
    })?;
    Ok(name)
}

/// A required object-id field, validated at this boundary (RFC 009 F2).
fn object_id_field<'a>(value: &'a Json, key: &str) -> Result<&'a str> {
    let id = value.str_field(key)?;
    ObjectId::parse(id).map_err(|_| {
        StikkError::environment_msg(format!(
            "prikk's JSON report has an unrecognized object id in `{key}`: {id:?}"
        ))
    })?;
    Ok(id)
}

/// An optional object-id field: absent or `null` read as `None`, and anything present is validated.
fn opt_object_id_field<'a>(value: &'a Json, key: &str) -> Result<Option<&'a str>> {
    let Some(id) = value.opt_str_field(key)? else {
        return Ok(None);
    };
    ObjectId::parse(id).map_err(|_| {
        StikkError::environment_msg(format!(
            "prikk's JSON report has an unrecognized object id in `{key}`: {id:?}"
        ))
    })?;
    Ok(Some(id))
}

/// Parse `text`, then confirm it announces `want`.
fn report(text: &str, want: &str) -> Result<Json> {
    let value = json::parse(text)?;
    let found = value.str_field("schema_version")?;
    if found != want {
        return Err(StikkError::environment_msg(format!(
            "prikk reported schema `{found}` where stikk reads `{want}`. stikk does not guess at an \
             unknown schema version — upgrade stikk, or run a prikk whose report it knows."
        )));
    }
    Ok(value)
}

/// `prikk log --format json` (`log-report-v1`), prikk ≥ 0.39.
///
/// # Errors
/// [`StikkError::Environment`] if the text is not JSON, announces another schema, or is missing a
/// field this shape requires.
pub(super) fn history(text: &str) -> Result<History> {
    let value = report(text, LOG_SCHEMA)?;
    let reff = ref_name_field(&value, "ref")?.to_string();
    let mut blocks = Vec::new();
    for block in value.array_field("blocks")? {
        blocks.push(BlockRow {
            block_id: object_id_field(block, "block_id")?.to_string(),
            ref_state_id: object_id_field(block, "ref_state_id")?.to_string(),
            update_seq: block.u64_field("update_seq")?,
            kind: block.str_field("kind")?.to_string(),
            rollback_block: block.bool_field("rollback_block")?,
            parents: block.u64_field("parent_count")?,
            patches: block.u64_field("patch_count")?,
            rollback_patches: block.u64_field("rollback_patch_count")?,
            required_attestations: block.u64_field("required_attestation_count")?,
            messages: patch_messages(block)?,
            previous_ref_state: opt_object_id_field(block, "previous_ref_state_id")?
                .map(str::to_string),
        });
    }
    Ok(History { reff, blocks })
}

/// A block's `patch_messages` array. **Absent and empty mean the same thing** and both are normal: a
/// block sealed entirely below prikk 0.32 carries no messages at all (RFC 015 F4), and `patch_count`
/// stays the authoritative total either way.
fn patch_messages(block: &Json) -> Result<Vec<PatchMessage>> {
    // **Absent is tolerated; a wrong type is not.** They are different rules and this file now gives
    // one answer to both questions — absence is how a pre-0.32 block spells "no messages", while a
    // `patch_messages` that is suddenly an object is a schema change and must say so.
    let items = match block.get("patch_messages") {
        None => return Ok(Vec::new()),
        Some(Json::Array(items)) => items,
        // `array_field` produces exactly the error this case needs, naming the field and what it
        // found; calling it is cheaper than a second message that could drift from that one.
        Some(_) => return block.array_field("patch_messages").map(|_| Vec::new()),
    };
    items
        .iter()
        .map(|item| {
            Ok(PatchMessage {
                patch_id: object_id_field(item, "patch_id")?.to_string(),
                message: item.str_field("message")?.to_string(),
            })
        })
        .collect()
}

/// `prikk branch list --all --format json` (`branch-list-v1`), prikk ≥ 0.39.
///
/// **Two arrays, not one.** `branches` and `received` are separate in the schema where the prose form
/// distinguished them by a `received` marker, so the flag stikk carries on [`RefEntry`] comes from
/// *which array an entry was in* rather than from parsing a word out of a line.
///
/// # Errors
/// As [`history`].
pub(super) fn refs(text: &str) -> Result<Vec<RefEntry>> {
    let value = report(text, BRANCH_SCHEMA)?;
    let mut out = Vec::new();
    for (key, received) in [("branches", false), ("received", true)] {
        // **prikk 0.41 always emits both arrays** — a repository that has never received anything
        // reports `"received": []`, measured on a fresh one. The tolerance below is deliberate slack
        // for a prikk that might omit an empty array, not a description of one that does; absence is
        // read as empty, and a wrong type still errors.
        let entries = match value.get(key) {
            Some(Json::Array(items)) => items.as_slice(),
            None => &[],
            Some(_) => value.array_field(key)?,
        };
        for entry in entries {
            out.push(RefEntry {
                name: ref_name_field(entry, "ref_name")?.to_string(),
                id: object_id_field(entry, "ref_state_id")?.to_string(),
                // **Required on `branches`, structural on `received`.** prikk's emitter puts `closed`
                // on every branch and on no received ref, deliberately, so which array an entry came
                // from already answers the question — the same reasoning `received` itself uses. It
                // was `unwrap_or(false)` and that was the one silent field in three parsers, on the
                // one fact the ref picker shows: if prikk renamed it, every closed branch would have
                // quietly rendered as open.
                closed: if received {
                    false
                } else {
                    entry.bool_field("closed")?
                },
                received,
            });
        }
    }
    Ok(out)
}

/// `prikk tag list --format json` (`tag-list-v1`), prikk ≥ 0.39.
///
/// A tag points at a **block**, not a RefState, so `target_block_id` fills [`RefEntry::id`] — the same
/// value the prose parser took from the same place. Tags are never closed and never received.
///
/// # Errors
/// As [`history`].
pub(super) fn tags(text: &str) -> Result<Vec<RefEntry>> {
    let value = report(text, TAG_SCHEMA)?;
    value
        .array_field("tags")?
        .iter()
        .map(|tag| {
            Ok(RefEntry {
                name: ref_name_field(tag, "ref_name")?.to_string(),
                id: object_id_field(tag, "target_block_id")?.to_string(),
                closed: false,
                received: false,
            })
        })
        .collect()
}

#[cfg(test)]
// `pub(super)` so `cli_backend`'s own tests can drive a captured report through `CliBackend` (review
// C1 of RFC 027 B) rather than keep a second copy of the capture.
pub(super) mod tests;

/// The `key-status-v1` schema name (prikk ≥ 0.41).
const KEY_STATUS_SCHEMA: &str = "key-status-v1";

/// `prikk key status --format json` (`key-status-v1`), prikk ≥ 0.41.
///
/// Returns the two roles' rows, keyed by prikk's own `role` value. A role prikk does not report is
/// **absent, not assumed ready** — the caller decides what that means, and today it means the same as
/// unusable.
///
/// **Every state below was measured against a real prikk 0.41.0**, one repository per state
/// (RFC 026 Handoff B): `usable: false` with each of the four reasons, and `binding` at `unrecorded`,
/// `matches`, `not-adopted`, `mismatch` and `null`. `public_key` and `binding` are both `null`
/// whenever the seed is unusable, so neither may be required.
///
/// # Errors
/// As [`history`]. `public_key` is validated as an object id when present — it is a 64-hex value and
/// the same boundary applies (`INV-9`'s sibling, RFC 009 F2).
pub(super) fn key_status(text: &str) -> Result<Vec<(String, crate::RoleDetail, RoleFacts)>> {
    let value = report(text, KEY_STATUS_SCHEMA)?;
    let mut out = Vec::new();
    for role in value.array_field("roles")? {
        let name = role.str_field("role")?.to_string();
        let usable = role.bool_field("usable")?;
        let binding = match role.opt_str_field("binding")? {
            None => stikk_model::Binding::Absent,
            Some("matches") => stikk_model::Binding::Matches,
            Some("unrecorded") => stikk_model::Binding::Unrecorded,
            Some("not-adopted") => stikk_model::Binding::NotAdopted,
            Some("mismatch") => stikk_model::Binding::Mismatch,
            // **An unknown binding is not a parse failure and not a pass.** prikk may add a state;
            // stikk treating it as `Absent` withholds nothing it should grant and claims nothing it
            // cannot support, which is the safe direction. The re-baseline that adds the variant will
            // find it here.
            Some(_) => stikk_model::Binding::Absent,
        };
        let detail = crate::RoleDetail {
            reason: role.opt_str_field("reason")?.map(str::to_string),
            key_id: role.opt_str_field("key_id")?.map(str::to_string),
            key_id_source: role.opt_str_field("key_id_source")?.map(str::to_string),
            public_key: opt_object_id_field(role, "public_key")?.map(str::to_string),
            stale_seed_variable: false,
        };
        out.push((name, detail, RoleFacts { usable, binding }));
    }
    Ok(out)
}

/// The two facts from a `key status` row that decide capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoleFacts {
    pub(super) usable: bool,
    pub(super) binding: stikk_model::Binding,
}

/// The worktree-status report's schema (prikk ≥ 0.38; the verdict fields are additive from 0.39).
const WORKTREE_SCHEMA: &str = "worktree-status-report-v1";

/// `prikk worktree-status --format json` (`worktree-status-report-v1`), prikk ≥ 0.39 (RFC 027
/// decision 2).
///
/// **Held to the shape prikk's emitter writes, and refused otherwise** — this report decides whether
/// commit is offered, so a field read optimistically is a verdict stikk invented:
///
/// - `authoring` is exactly `"authored"` with `refusal: null`, or `"refused"` with a string `refusal`.
///   Any other pair is a schema error.
/// - `refused_count` must equal the number of refused entries. prikk computes both from one list, so a
///   disagreement means stikk misread the report — there is no right number to pick between.
/// - `ref` and a non-null `queued_elsewhere` are ref names, validated at this boundary (`INV-9`).
/// - `queued_elsewhere` must be **present** (as `null` or a name): its absence would read as "nothing
///   queued elsewhere", which is the one warning RFC 009 exists to keep.
///
/// **Paths are not validated as repository paths** (RFC 027 decision 2): an `unsupported-path` entry's
/// path is by definition not one, and is carried as reported, to be rendered inert. Per-kind counts are
/// derived from `changes`, which is the list prikk's own counters are computed from. **`declarations`
/// is read too** (RFC 030 amendment A1), and must be present: a declaration made after a preview changes
/// what `commit` authors without changing `changes`, so its absence cannot be read as "none".
///
/// # Errors
/// [`StikkError::Environment`] if the text is not JSON, announces another schema, or breaks any rule
/// above.
pub(super) fn worktree_status(text: &str, prikk_minor: u32) -> Result<WorktreeStatus> {
    let value = report(text, WORKTREE_SCHEMA)?;
    let reff = ref_name_field(&value, "ref")?.to_string();
    let tracked = value.u64_field("tracked_files")?;
    let unchanged = value.u64_field("unchanged_files")?;
    let clean = value.bool_field("clean")?;
    let refused_count = value.u64_field("refused_count")?;

    let queued_elsewhere = match value.get("queued_elsewhere") {
        None => {
            return Err(StikkError::environment_msg(
                "prikk's JSON report is missing `queued_elsewhere`; stikk does not read its absence \
                 as \"nothing queued elsewhere\"",
            ));
        }
        Some(Json::Null) => None,
        Some(_) => Some(QueuedElsewhere::Ref(
            ref_name_field(&value, "queued_elsewhere")?.to_string(),
        )),
    };

    let mut entries = Vec::new();
    for change in value.array_field("changes")? {
        entries.push(WorktreeEntry {
            kind: change.str_field("kind")?.to_string(),
            path: change.str_field("path")?.to_string(),
            note: change.str_field("detail")?.to_string(),
            authoring: authoring(change)?,
        });
    }

    let refused_listed = entries
        .iter()
        .filter(|entry| matches!(entry.authoring, Authoring::Refused(_)))
        .count();
    if u64::try_from(refused_listed).ok() != Some(refused_count) {
        return Err(StikkError::environment_msg(format!(
            "prikk's JSON report says `refused_count` is {refused_count} but lists {refused_listed} \
             refused entries; stikk does not choose between them"
        )));
    }

    // RFC 034 decision 4: prikk's own verdict on each declaration, and how many it would refuse, read
    // only inside the band (see `DECLARATION_VERDICT_FROM_MINOR`).
    let reads_verdict = prikk_minor >= DECLARATION_VERDICT_FROM_MINOR;
    let declarations = declarations(&value, reads_verdict)?;
    let refused_declarations = if reads_verdict {
        let reported = value.u64_field("refused_declaration_count")?;
        let listed = declarations
            .iter()
            .filter(|declaration| {
                declaration.resolution == Some(crate::DeclarationResolution::Refused)
            })
            .count();
        // The same rule `refused_count` carries: prikk computes both from one list, so a disagreement
        // means stikk misread the report, and there is no right number to pick between.
        if u64::try_from(listed).ok() != Some(reported) {
            return Err(StikkError::environment_msg(format!(
                "prikk's JSON report says `refused_declaration_count` is {reported} but resolves \
                 {listed} declarations \"refused\"; stikk does not choose between them"
            )));
        }
        Some(reported)
    } else {
        None
    };

    let count = |kind: &str| {
        u64::try_from(entries.iter().filter(|entry| entry.kind == kind).count()).unwrap_or(u64::MAX)
    };
    Ok(WorktreeStatus {
        reff,
        clean,
        tracked,
        unchanged,
        missing: count("missing"),
        modified: count("modified"),
        untracked: count("untracked"),
        unsupported: count("unsupported-path"),
        refused: Some(refused_count),
        refused_declarations,
        queued_elsewhere,
        entries,
        declarations,
    })
}

/// The first prikk minor whose declaration verdict stikk reads (RFC 034 §1).
///
/// **The fields exist from 0.43, and stikk skips that release deliberately:** its classifier resolved a
/// directory destination as `rename` while `commit` recorded a deletion (stikk letter 014, fixed in
/// 0.44 — measured here at 0.43.0 against 0.44.0), it is reported to hang on a FIFO destination
/// (prikk's letter 017 §2), and `content_changed`/`mode_changed` changed meaning in 0.44. One band,
/// rather than a matrix for a superseded release.
const DECLARATION_VERDICT_FROM_MINOR: u32 = 44;

/// The report's `declarations` array: required present, each entry a string `old_path` and `new_path`
/// (RFC 030 amendment A1). Paths are carried as reported.
///
/// **From prikk 0.44 each entry also carries prikk's own verdict** (RFC 034 decision 4): `resolution`,
/// `refusal`, `content_changed` and `mode_changed`, all required present — a missing one would read as
/// "unknown" for a prikk that does in fact report it, which is the shape `UD-02` refuses to guess at.
/// Below the band every field is `None`, and the reader does not look.
///
/// **Two rules, both measured:** `resolution: "refused"` comes with a refusal and no other resolution
/// does, and `content_changed`/`mode_changed` are `null` for anything but a resolved rename.
fn declarations(value: &Json, reads_verdict: bool) -> Result<Vec<RenameDeclaration>> {
    value
        .array_field("declarations")?
        .iter()
        .map(|declaration| {
            let old_path = declaration.str_field("old_path")?.to_string();
            let new_path = declaration.str_field("new_path")?.to_string();
            if !reads_verdict {
                return Ok(RenameDeclaration {
                    old_path,
                    new_path,
                    resolution: None,
                    refusal: None,
                    content_changed: None,
                    mode_changed: None,
                });
            }
            let resolution =
                DeclarationResolution::from_label(declaration.str_field("resolution")?);
            let refusal = match (&resolution, declaration.get("refusal")) {
                (DeclarationResolution::Refused, Some(Json::String(reason))) => {
                    Some(reason.clone())
                }
                (DeclarationResolution::Refused, other) => {
                    return Err(StikkError::environment_msg(format!(
                        "prikk's JSON report resolves the declaration {old_path} -> {new_path} \
                         \"refused\" with `refusal` {other:?}; stikk shows prikk's own refusal and \
                         will not invent one"
                    )));
                }
                (_, Some(Json::Null) | None) => None,
                (resolution, Some(_)) => {
                    return Err(StikkError::environment_msg(format!(
                        "prikk's JSON report carries a `refusal` on the declaration {old_path} -> \
                         {new_path}, whose resolution is {:?}; stikk reads a refusal only on \
                         \"refused\"",
                        resolution.label()
                    )));
                }
            };
            Ok(RenameDeclaration {
                old_path,
                new_path,
                resolution: Some(resolution),
                refusal,
                content_changed: opt_bool_field(declaration, "content_changed")?,
                mode_changed: opt_bool_field(declaration, "mode_changed")?,
            })
        })
        .collect()
}

/// A field that is `true`, `false` or `null`, and **must be present**: its absence would be
/// indistinguishable from prikk's own "unknown", and stikk does not read a missing field as a value
/// (`UD-02`). `None` is unknown, never "unchanged" (`C-T2c′`).
fn opt_bool_field(value: &Json, key: &str) -> Result<Option<bool>> {
    match value.get(key) {
        Some(Json::Bool(flag)) => Ok(Some(*flag)),
        Some(Json::Null) => Ok(None),
        Some(_) => Err(StikkError::environment_msg(format!(
            "prikk's JSON report has `{key}` as neither a boolean nor null"
        ))),
        None => Err(StikkError::environment_msg(format!(
            "prikk's JSON report is missing `{key}`; stikk does not read its absence as a value"
        ))),
    }
}

/// One change's verdict: exactly one of prikk's two legal `authoring`/`refusal` pairs.
fn authoring(change: &Json) -> Result<Authoring> {
    let verdict = change.str_field("authoring")?;
    match (verdict, change.get("refusal")) {
        ("authored", Some(Json::Null)) => Ok(Authoring::Authored),
        ("refused", Some(Json::String(reason))) => Ok(Authoring::Refused(reason.clone())),
        (verdict, refusal) => Err(StikkError::environment_msg(format!(
            "prikk's JSON report has an entry with `authoring` {verdict:?} and `refusal` {refusal:?}; \
             stikk reads only \"authored\" with null, or \"refused\" with a reason"
        ))),
    }
}

/// A queue rule broken: stikk's environment error, naming the rule (RFC 027 B, C1).
fn queue_rule(detail: impl std::fmt::Display) -> StikkError {
    StikkError::environment_msg(format!(
        "prikk's status report breaks a queue rule: {detail}"
    ))
}

/// A field that must be present, as a string or `null`. Unlike `Json::opt_str_field`, **absence is an
/// error**: `status-report-v1` writes every queue field, `null` included, so a missing one is a shape
/// change.
fn nullable_str<'a>(value: &'a Json, key: &str) -> Result<Option<&'a str>> {
    match value.get(key) {
        None => Err(queue_rule(format!("missing field `{key}`"))),
        Some(Json::Null) => Ok(None),
        Some(Json::String(s)) => Ok(Some(s)),
        Some(_) => Err(queue_rule(format!("`{key}` should be a string or null"))),
    }
}

/// As [`nullable_str`], for an integer.
fn nullable_u64(value: &Json, key: &str) -> Result<Option<u64>> {
    match value.get(key) {
        None => Err(queue_rule(format!("missing field `{key}`"))),
        Some(Json::Null) => Ok(None),
        Some(Json::Number(n)) => Ok(Some(*n)),
        Some(_) => Err(queue_rule(format!("`{key}` should be a number or null"))),
    }
}

/// `prikk status --format json`'s `queue` (`status-report-v1`), prikk ≥ 0.39 (RFC 028 decision 1).
///
/// **Every rule here is stikk's, and breaking one is an environment error, never prikk's refusal**:
/// - the schema is checked first;
/// - `count` equals the patches listed;
/// - `target_ref` parses as a ref name when present, and `target_ref_status` is `null` beside it; without
///   one, the status is `null`, `missing-metadata` or `malformed-metadata`;
/// - `threshold_status` is `null` exactly when the queue is empty, and `warn_threshold`/`hard_limit` with
///   it; otherwise the status is `none`, `warn` or `hard-limit`;
/// - `patch_id` parses as an object id;
/// - each path object carries exactly one of `path` and `unresolved_node_id`;
/// - `rename-path` carries `author_key_id`;
/// - **the message, by version**: at ≥ 0.42 the field is present, as `null` or a string; below 0.42 it is
///   absent. A message present below 0.42 is a shape stikk has not seen there, and refuses too.
///
/// `prikk_minor` is passed in for the message rule, as `parse::orientation` takes it for `current branch:`.
///
/// # Errors
/// [`StikkError::Environment`] naming the broken rule or missing field.
pub(super) fn queue(text: &str, prikk_minor: u32) -> Result<Queue> {
    let value = report(text, STATUS_SCHEMA)?;
    let queue = match value.get("queue") {
        Some(queue @ Json::Object(_)) => queue,
        Some(_) => return Err(queue_rule("`queue` should be an object")),
        None => return Err(queue_rule("missing field `queue`")),
    };
    let count = queue.u64_field("count")?;
    let listed = queue.array_field("patches")?;
    if u64::try_from(listed.len()).ok() != Some(count) {
        return Err(queue_rule(format!(
            "`count` is {count} but {} patch(es) are listed",
            listed.len()
        )));
    }
    let target = queue_target(queue)?;
    let threshold = queue_threshold(queue, count)?;
    let patches = listed
        .iter()
        .map(|patch| queued_patch(patch, prikk_minor))
        .collect::<Result<Vec<_>>>()?;
    Ok(Queue {
        count,
        target,
        threshold,
        patches,
    })
}

fn queue_target(queue: &Json) -> Result<QueueTarget> {
    let target = nullable_str(queue, "target_ref")?;
    let status = nullable_str(queue, "target_ref_status")?;
    match (target, status) {
        (Some(_), None) => Ok(QueueTarget::Ref(
            ref_name_field(queue, "target_ref")?.to_string(),
        )),
        (None, Some("missing-metadata")) => Ok(QueueTarget::MissingMetadata),
        (None, Some("malformed-metadata")) => Ok(QueueTarget::MalformedMetadata),
        (None, None) => Ok(QueueTarget::NotReported),
        (_, Some(other @ ("missing-metadata" | "malformed-metadata"))) => Err(queue_rule(format!(
            "`target_ref` names a ref beside `target_ref_status: {other:?}`"
        ))),
        (_, Some(other)) => Err(queue_rule(format!(
            "`target_ref_status` is {other:?}, which is not null, \"missing-metadata\" or \
             \"malformed-metadata\""
        ))),
    }
}

fn queue_threshold(queue: &Json, count: u64) -> Result<Option<QueueThreshold>> {
    let status = nullable_str(queue, "threshold_status")?;
    let warn = nullable_u64(queue, "warn_threshold")?;
    let hard_limit = nullable_u64(queue, "hard_limit")?;
    match (count, status, warn, hard_limit) {
        (0, None, None, None) => Ok(None),
        (1.., Some(status), Some(warn), Some(hard_limit)) => {
            let status = match status {
                "none" => ThresholdStatus::None,
                "warn" => ThresholdStatus::Warn,
                "hard-limit" => ThresholdStatus::HardLimit,
                other => {
                    return Err(queue_rule(format!(
                        "`threshold_status` is {other:?}, which is not \"none\", \"warn\" or \
                         \"hard-limit\""
                    )));
                }
            };
            Ok(Some(QueueThreshold {
                status,
                warn,
                hard_limit,
            }))
        }
        _ => Err(queue_rule(format!(
            "`threshold_status`, `warn_threshold` and `hard_limit` must all be null exactly when the \
             queue is empty, but `count` is {count} and they are {status:?}, {warn:?}, {hard_limit:?}"
        ))),
    }
}

fn queued_patch(patch: &Json, prikk_minor: u32) -> Result<QueuedPatch> {
    let patch_id = object_id_field(patch, "patch_id")?.to_string();
    let reports_messages = prikk_minor >= QUEUED_MESSAGE_FROM_MINOR;
    let message = match (patch.get("message"), reports_messages) {
        (None, false) => QueuedMessage::NotReported,
        (Some(Json::Null), true) => QueuedMessage::None,
        (Some(Json::String(text)), true) => QueuedMessage::Text(text.clone()),
        (None, true) => {
            return Err(queue_rule(format!(
                "patch {patch_id} has no `message` field, which prikk \
                 0.{QUEUED_MESSAGE_FROM_MINOR} and later always write (this is 0.{prikk_minor})"
            )));
        }
        (Some(_), false) => {
            return Err(queue_rule(format!(
                "patch {patch_id} carries a `message` field, which prikk 0.{prikk_minor} does not \
                 write — a shape stikk has not seen at this version"
            )));
        }
        (Some(_), true) => {
            return Err(queue_rule(format!(
                "patch {patch_id}'s `message` should be a string or null"
            )));
        }
    };
    let operations = patch
        .array_field("operations")?
        .iter()
        .map(queued_operation)
        .collect::<Result<Vec<_>>>()?;
    Ok(QueuedPatch {
        patch_id,
        message,
        operations,
    })
}

fn queued_operation(operation: &Json) -> Result<QueuedOperation> {
    let kind = operation.str_field("kind")?.to_string();
    let paths = operation
        .array_field("paths")?
        .iter()
        .map(queued_path)
        .collect::<Result<Vec<_>>>()?;
    let author_key_id = operation
        .opt_str_field("author_key_id")?
        .map(str::to_string);
    // prikk cannot construct a rename without its asserting author (RFC 028 F1).
    if kind == "rename-path" && author_key_id.is_none() {
        return Err(queue_rule(
            "a `rename-path` operation carries no `author_key_id`",
        ));
    }
    Ok(QueuedOperation {
        kind,
        paths,
        author_key_id,
    })
}

fn queued_path(path: &Json) -> Result<QueuedPath> {
    match (path.get("path"), path.get("unresolved_node_id")) {
        (Some(Json::String(path)), None) => Ok(QueuedPath::Path(path.clone())),
        (None, Some(Json::String(node))) => Ok(QueuedPath::UnresolvedNode(node.clone())),
        _ => Err(queue_rule(
            "a path object must carry exactly one of `path` and `unresolved_node_id`, as a string",
        )),
    }
}

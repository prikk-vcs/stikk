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
use crate::{BlockRow, History, PatchMessage, RefEntry};

type Result<T> = std::result::Result<T, StikkError>;

/// The schema names stikk knows how to read. Checked rather than assumed (see the module doc).
const LOG_SCHEMA: &str = "log-report-v1";
const BRANCH_SCHEMA: &str = "branch-list-v1";
const TAG_SCHEMA: &str = "tag-list-v1";

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
mod tests;

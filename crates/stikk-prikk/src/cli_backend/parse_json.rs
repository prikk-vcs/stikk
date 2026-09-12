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
//! # The schema version is checked, and checked loudly
//!
//! Every report names its own schema. stikk verifies that name before reading a field: a
//! `log-report-v2` is not something to parse optimistically and hope the fields it wanted survived —
//! that is the RFC 012 F-e failure shape, applied to JSON. An unknown schema is an environment error
//! naming both what was found and what stikk knows, the same way the version gate does.

use stikk_model::StikkError;

use crate::json::{self, Json};
use crate::{BlockRow, History, PatchMessage, RefEntry};

type Result<T> = std::result::Result<T, StikkError>;

/// The schema names stikk knows how to read. Checked rather than assumed (see the module doc).
const LOG_SCHEMA: &str = "log-report-v1";
const BRANCH_SCHEMA: &str = "branch-list-v1";
const TAG_SCHEMA: &str = "tag-list-v1";

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
    let reff = value.str_field("ref")?.to_string();
    let mut blocks = Vec::new();
    for block in value.array_field("blocks")? {
        blocks.push(BlockRow {
            block_id: block.str_field("block_id")?.to_string(),
            ref_state_id: block.str_field("ref_state_id")?.to_string(),
            update_seq: block.u64_field("update_seq")?,
            kind: block.str_field("kind")?.to_string(),
            rollback_block: block.bool_field("rollback_block")?,
            parents: block.u64_field("parent_count")?,
            patches: block.u64_field("patch_count")?,
            rollback_patches: block.u64_field("rollback_patch_count")?,
            required_attestations: block.u64_field("required_attestation_count")?,
            messages: patch_messages(block)?,
            previous_ref_state: block
                .opt_str_field("previous_ref_state_id")?
                .map(str::to_string),
        });
    }
    Ok(History { reff, blocks })
}

/// A block's `patch_messages` array. **Absent and empty mean the same thing** and both are normal: a
/// block sealed entirely below prikk 0.32 carries no messages at all (RFC 015 F4), and `patch_count`
/// stays the authoritative total either way.
fn patch_messages(block: &Json) -> Result<Vec<PatchMessage>> {
    let Some(Json::Array(items)) = block.get("patch_messages") else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            Ok(PatchMessage {
                patch_id: item.str_field("patch_id")?.to_string(),
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
        // `received` is absent on a repository that has never received one; that is not a schema
        // mismatch, it is an empty list spelled by omission.
        let entries = match value.get(key) {
            Some(Json::Array(items)) => items.as_slice(),
            None => &[],
            Some(_) => value.array_field(key)?,
        };
        for entry in entries {
            out.push(RefEntry {
                name: entry.str_field("ref_name")?.to_string(),
                id: entry.str_field("ref_state_id")?.to_string(),
                closed: entry.bool_field("closed").unwrap_or(false),
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
                name: tag.str_field("ref_name")?.to_string(),
                id: tag.str_field("target_block_id")?.to_string(),
                closed: false,
                received: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;

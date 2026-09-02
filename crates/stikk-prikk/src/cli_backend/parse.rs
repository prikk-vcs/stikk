//! Output parsers for prikk commands, confined here per design SEAM-03 (UD-02).
//!
//! Every parser refuses rather than fabricates on an unrecognized shape: a missing expected field is
//! an [`StikkError::Environment`], never a silent default, because encountering it means either
//! stikk misread prikk or the prikk version is outside the validated range. Golden fixtures in
//! `cli_backend/tests.rs` pin the shapes these read.

use stikk_model::{Result, StikkError};

use crate::{BlockRow, History, Orientation, RefEntry, StateFiles};

/// Parse `prikk status` output into an [`Orientation`].
///
/// Expected lines (order-independent), from `prikk status`:
/// ```text
/// prikk repository: <path>/.prikk
/// active WAL records: <n>
/// trailing partial WAL bytes: <n>
/// heads/main RefState: <hex> | <none>
/// queued patches: <n>
/// ```
pub(super) fn orientation(status: &str) -> Result<Orientation> {
    let queued_patches = required_u64(status, "queued patches:")?;
    let trailing_partial_wal_bytes = required_u64(status, "trailing partial WAL bytes:")?;
    let main_ref_state = optional_object_id(status, "heads/main RefState:");
    Ok(Orientation {
        queued_patches,
        main_ref_state,
        trailing_partial_wal_bytes,
    })
}

/// Find the value after a `label` prefix on some line, trimmed. `None` if the label is absent.
fn field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(label).map(str::trim))
}

fn required_u64(text: &str, label: &str) -> Result<u64> {
    let value = field(text, label).ok_or_else(|| {
        StikkError::environment_msg(format!("prikk status output is missing {label:?}"))
    })?;
    value.parse::<u64>().map_err(|_| {
        StikkError::environment_msg(format!("prikk status {label:?} is not a number: {value:?}"))
    })
}

/// Read an object-id field that may be the literal `<none>`.
fn optional_object_id(text: &str, label: &str) -> Option<String> {
    match field(text, label) {
        Some("<none>") | None => None,
        Some(value) => Some(value.to_string()),
    }
}

/// Parse `prikk log` output into a ref's block lineage (RFC 006). Blocks are introduced by a
/// `block <id>` line followed by indented `label: value` fields; a new `block` line, or end of input,
/// ends the current block. Refuses (rather than guesses) on a malformed count or a truncated block.
pub(super) fn history(text: &str) -> Result<History> {
    let reff = required_field(text, "ref:")?;
    let mut blocks = Vec::new();
    // `group[0]` is a block id; the following entries are that block's field lines. A `block <id>`
    // line starts a new group; lines before the first block (the `repository:`/`ref:` header) are
    // skipped because `group` is still empty.
    let mut group: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(id) = trimmed.strip_prefix("block ") {
            if !group.is_empty() {
                blocks.push(decode_block(&group)?);
            }
            group.clear();
            group.push(id.trim());
        } else if !group.is_empty() {
            group.push(trimmed);
        }
    }
    if !group.is_empty() {
        blocks.push(decode_block(&group)?);
    }
    Ok(History { reff, blocks })
}

/// Decode one block's lines. `lines[0]` is the block id; the rest are `label: value` fields.
fn decode_block(lines: &[&str]) -> Result<BlockRow> {
    let block_id = (*lines
        .first()
        .ok_or_else(|| StikkError::environment_msg("empty block group in prikk log"))?)
    .to_string();
    let group = lines.get(1..).unwrap_or(&[]).join("\n");
    Ok(BlockRow {
        block_id,
        ref_state_id: required_field(&group, "ref-state:")?,
        update_seq: required_u64(&group, "update-seq:")?,
        kind: required_field(&group, "kind:")?,
        rollback_block: required_bool(&group, "rollback-block:")?,
        parents: required_u64(&group, "parents:")?,
        patches: required_u64(&group, "patches:")?,
        rollback_patches: required_u64(&group, "rollback-patches:")?,
        required_attestations: required_u64(&group, "required-attestations:")?,
        previous_ref_state: optional_object_id(&group, "previous-ref-state:"),
    })
}

/// Parse `prikk checkout --patch-plan` into the tip state file set (RFC 006). File paths are the
/// indented `file: <path>` lines.
pub(super) fn state_files(text: &str) -> Result<StateFiles> {
    let target_block = required_field(text, "target block:")?;
    let total_bytes = required_u64(text, "result content bytes:")?;
    let files = text
        .lines()
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix("file:")
                .map(|p| p.trim().to_string())
        })
        .collect();
    Ok(StateFiles {
        target_block,
        files,
        total_bytes,
    })
}

/// Parse `prikk branch list --all` into every ref pointer (RFC 006). Each line is
/// `<name> <id> [(closed)|(received)]`.
pub(super) fn refs(text: &str) -> Result<Vec<RefEntry>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| StikkError::environment_msg("empty ref line in prikk branch list"))?;
        let id = parts.next().ok_or_else(|| {
            StikkError::environment_msg(format!("ref line missing an id: {line:?}"))
        })?;
        let rest = line;
        out.push(RefEntry {
            name: name.to_string(),
            id: id.to_string(),
            closed: rest.contains("(closed)"),
            received: rest.contains("(received)"),
        });
    }
    Ok(out)
}

/// A required string field.
fn required_field(text: &str, label: &str) -> Result<String> {
    Ok(field(text, label)
        .ok_or_else(|| StikkError::environment_msg(format!("prikk output is missing {label:?}")))?
        .to_string())
}

/// A required boolean field (`true`/`false`).
fn required_bool(text: &str, label: &str) -> Result<bool> {
    match required_field(text, label)?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(StikkError::environment_msg(format!(
            "prikk field {label:?} is not a bool: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests;

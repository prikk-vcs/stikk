//! Output parsers for prikk commands, confined here per design SEAM-03 (UD-02).
//!
//! Every parser refuses rather than fabricates on an unrecognized shape: a missing expected field is
//! an [`StikkError::Environment`], never a silent default, because encountering it means either
//! stikk misread prikk or the prikk version is outside the validated range. Golden fixtures in
//! `parse/tests.rs` pin the shapes these read — **captured from a real `prikk` run, never composed by
//! hand or by analogy**, each with a provenance comment naming the command and prikk version it came
//! from (RFC 009 §0). A hand-written fixture is a defect, not a shortcut: RFC 009's F1 was exactly
//! that — a `queued patches: 3` fixture prikk has never emitted, which made Orientation's parser pass
//! its test while refusing on every real repository with queued work.
//!
//! **Every ref name is validated through [`stikk_model::RefName::parse`] at this boundary** (`INV-9`;
//! RFC 012 F-d), the same role [`ObjectId::parse`] already played for ids here (RFC 009 F2): an empty
//! or control-character-bearing name is a shape prikk would never emit, so it refuses rather than
//! travelling further as an unvalidated string. Struct fields (`History.reff`, `RefEntry.name`,
//! `WorktreeStatus.reff`, `Orientation.queued_target`) stay plain `String` above this boundary —
//! `RefName`'s guarantee lives here, at the parse boundary, not in every downstream type — matching
//! `ObjectId`'s own precedent rather than the alternative of changing every field's type (see the RFC
//! 012 review request for the diff-size measurement behind that call).

use stikk_model::{ObjectId, RefName, Result, StikkError};

use crate::{BlockRow, History, Orientation, RefEntry, StateFiles, WorktreeEntry, WorktreeStatus};

/// The per-path change kinds `worktree-status` emits; used to tell an indented entry line from a
/// flush-left count line that happens to share a first word (`modified files:` vs `  modified …`).
const WORKTREE_KINDS: [&str; 4] = ["modified", "missing", "untracked", "unsupported"];

/// Parse `prikk status` output into an [`Orientation`].
///
/// Expected lines (order-independent), from `prikk status`:
/// ```text
/// prikk repository: <path>/.prikk
/// active WAL records: <n>
/// trailing partial WAL bytes: <n>
/// heads/main RefState: <hex> | <not published>
/// queued patches: <n> [targeting <ref>]
/// [warning: active patches …]
/// status: …
/// ```
/// The `queued patches:` value is a bare count only when the queue is empty; otherwise prikk appends
/// ` targeting <ref>` (RFC 009 F1 — this is not new drift, `git log -S` dates it to prikk 0.18.0).
/// prikk may also print an active-patch threshold `warning:` line before the closing `status:` line;
/// this parser does not read it (deferred to the commit increment), but tolerates its presence by
/// anchoring on the `queued patches:` line rather than on where the report ends.
pub(super) fn orientation(status: &str) -> Result<Orientation> {
    let (queued_patches, queued_target) =
        parse_queued(&required_field(status, "queued patches:")?)?;
    let trailing_partial_wal_bytes = required_u64(status, "trailing partial WAL bytes:")?;
    let main_ref_state = optional_object_id(status, "heads/main RefState:")?;
    Ok(Orientation {
        queued_patches,
        queued_target,
        main_ref_state,
        trailing_partial_wal_bytes,
    })
}

/// Parse the `queued patches:` field's value into a count and, when the queue targets a specific ref, a
/// look at that ref (RFC 009 F1/F2). A bare integer means an empty queue; `<n> targeting <ref>` means
/// `<ref>` is the queue's target — unless `<ref>` is one of prikk's own sentinel forms for unreadable
/// active-ref metadata (`<missing metadata>`, `<malformed metadata>`), which map to `None` rather than
/// a fabricated ref name. Any other trailing shape refuses (UD-02) rather than guesses.
fn parse_queued(value: &str) -> Result<(u64, Option<String>)> {
    let mut parts = value.splitn(2, ' ');
    let count_str = parts.next().unwrap_or_default();
    let count = count_str.parse::<u64>().map_err(|_| {
        StikkError::environment_msg(format!(
            "prikk field \"queued patches:\" is not a number: {value:?}"
        ))
    })?;
    let rest = parts.next().map(str::trim).unwrap_or("");
    if rest.is_empty() {
        return Ok((count, None));
    }
    let Some(target) = rest.strip_prefix("targeting ") else {
        return Err(StikkError::environment_msg(format!(
            "prikk field \"queued patches:\" has an unrecognized tail: {value:?}"
        )));
    };
    let queued_target = match target.trim() {
        "<missing metadata>" | "<malformed metadata>" => None,
        other => {
            // RFC 012 F-d: a target that is neither sentinel nor a shape prikk could ever emit as a
            // ref name refuses (UD-02) rather than travelling as an unvalidated string.
            RefName::parse(other).map_err(|_| {
                StikkError::environment_msg(format!(
                    "prikk field \"queued patches:\" targets an invalid ref name: {other:?}"
                ))
            })?;
            Some(other.to_string())
        }
    };
    Ok((count, queued_target))
}

/// Find the value after a `label` prefix on some line, trimmed. `None` if the label is absent.
fn field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(label).map(str::trim))
}

fn required_u64(text: &str, label: &str) -> Result<u64> {
    let value = field(text, label)
        .ok_or_else(|| StikkError::environment_msg(format!("prikk output is missing {label:?}")))?;
    value.parse::<u64>().map_err(|_| {
        StikkError::environment_msg(format!("prikk field {label:?} is not a number: {value:?}"))
    })
}

/// prikk's sentinel forms for "no object id here" (RFC 009 F2): an absent chain link (`log`'s
/// `<none>`), an unpublished ref (`status`'s `<not published>`), and unreadable active-ref metadata
/// (`status`'s `<missing metadata>` / `<malformed metadata>`, the same forms [`parse_queued`] handles).
/// All map to `None`; stikk assumed only `<none>` existed, which turned `<not published>` into a
/// fabricated object id (F2's defect).
const OBJECT_ID_SENTINELS: [&str; 4] = [
    "<none>",
    "<not published>",
    "<missing metadata>",
    "<malformed metadata>",
];

/// Read an object-id field: a known sentinel maps to `None`, a valid 64-hex id maps to `Some`, and
/// anything else — a sentinel this parser does not yet know, or plain corruption — refuses (RFC 009
/// F2, UD-02) rather than passing an unrecognized value through as if it were an identity.
fn optional_object_id(text: &str, label: &str) -> Result<Option<String>> {
    match field(text, label) {
        None => Ok(None),
        Some(value) if OBJECT_ID_SENTINELS.contains(&value) => Ok(None),
        Some(value) => {
            ObjectId::parse(value).map_err(|_| {
                StikkError::environment_msg(format!(
                    "prikk field {label:?} is neither a known sentinel nor a valid object id: \
                     {value:?}"
                ))
            })?;
            Ok(Some(value.to_string()))
        }
    }
}

/// Parse `prikk log` output into a ref's block lineage (RFC 006). Blocks are introduced by a
/// `block <id>` line followed by indented `label: value` fields; a new `block` line, or end of input,
/// ends the current block. Refuses (rather than guesses) on a malformed count or a truncated block.
pub(super) fn history(text: &str) -> Result<History> {
    let reff = required_ref_field(text, "ref:")?;
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
        previous_ref_state: optional_object_id(&group, "previous-ref-state:")?,
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

/// Parse `prikk branch list --all` into every ref pointer (RFC 006; corrected RFC 009 F3). Named for
/// branches (open, closed, received), but does **not** reliably exclude tags — prikk's ref-pointer
/// index carries no namespace filter, so a tag can appear here too (RFC 012 FR-014). `tags` (`prikk tag
/// list`) is the documented source for tags; `stikk_core::list_refs` merges both, deduplicated, so
/// stikk's own behavior does not depend on whether this parser's output happens to include one. prikk
/// prints the literal line `no branches` (or `tag list`'s `no tags`) when there are none; otherwise each
/// line is `<name> <64-hex-id> [(closed)|(received)]`.
///
/// Anchored on the id's shape, unlike before: a line with two tokens whose second is not a 64-hex id
/// (the literal `no branches` included) previously became a phantom `RefEntry { name: "no", id:
/// "branches" }` (RFC 009 F3's defect) because this was the one parser that refused on nothing. It now
/// refuses on anything but the recognized empty-list line or the exact ref-line shape.
pub(super) fn refs(text: &str) -> Result<Vec<RefEntry>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "no branches" || line == "no tags" {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| StikkError::environment_msg("empty ref line in prikk branch list"))?;
        let id = parts.next().ok_or_else(|| {
            StikkError::environment_msg(format!("ref line missing an id: {line:?}"))
        })?;
        // RFC 012 F-d: validated at this parse boundary (UD-02), same as the id below.
        RefName::parse(name).map_err(|_| {
            StikkError::environment_msg(format!(
                "prikk branch list line has an unrecognized ref name: {line:?}"
            ))
        })?;
        ObjectId::parse(id).map_err(|_| {
            StikkError::environment_msg(format!(
                "prikk branch list line has an unrecognized id: {line:?}"
            ))
        })?;
        let (closed, received) = match parts.next() {
            None => (false, false),
            Some("(closed)") => (true, false),
            Some("(received)") => (false, true),
            Some(other) => {
                return Err(StikkError::environment_msg(format!(
                    "prikk branch list line has an unrecognized marker {other:?}: {line:?}"
                )));
            }
        };
        if parts.next().is_some() {
            return Err(StikkError::environment_msg(format!(
                "prikk branch list line has unexpected trailing content: {line:?}"
            )));
        }
        out.push(RefEntry {
            name: name.to_string(),
            id: id.to_string(),
            closed,
            received,
        });
    }
    Ok(out)
}

/// Parse `prikk tag list` into every tag pointer (RFC 012 FR-014 completion). Unlike `branch list`,
/// prikk's own `--help` documents this command's shape as "name, target block" with no marker at all
/// — tags are neither closeable nor received — so any trailing token past the id refuses rather than
/// guessing at a meaning that does not exist. `no tags` is the recognized empty-list line, the same
/// shape `refs` already tolerates for it (RFC 009 F3).
pub(super) fn tags(text: &str) -> Result<Vec<RefEntry>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "no tags" {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| StikkError::environment_msg("empty tag line in prikk tag list"))?;
        let id = parts.next().ok_or_else(|| {
            StikkError::environment_msg(format!("tag line missing a target block id: {line:?}"))
        })?;
        RefName::parse(name).map_err(|_| {
            StikkError::environment_msg(format!(
                "prikk tag list line has an unrecognized ref name: {line:?}"
            ))
        })?;
        ObjectId::parse(id).map_err(|_| {
            StikkError::environment_msg(format!(
                "prikk tag list line has an unrecognized target block id: {line:?}"
            ))
        })?;
        if parts.next().is_some() {
            return Err(StikkError::environment_msg(format!(
                "prikk tag list line has unexpected trailing content: {line:?}"
            )));
        }
        out.push(RefEntry {
            name: name.to_string(),
            id: id.to_string(),
            closed: false,
            received: false,
        });
    }
    Ok(out)
}

/// The distinguishing prefix of prikk's queued-elsewhere warning (RFC 009 F4): present only when the
/// active WAL holds queued patches for a **different** ref than the one `worktree-status` was asked
/// about. Matched by prefix, not the full line, because the note names the specific refs involved.
const QUEUED_ELSEWHERE_PREFIX: &str = "note: the active WAL has queued";

/// Parse `prikk worktree-status` into a [`WorktreeStatus`] (design FR-034; RFC 008; RFC 009 F4).
///
/// The `worktree: clean|changed against baseline` headline is the shape anchor — its absence means
/// this is not a worktree-status report (the caller then treats the outcome as a real failure). Count
/// lines are flush-left (`modified files: N`); per-path entries are indented (`  modified <path> —
/// <note>`), which is how the two are told apart despite sharing a first word. A flush-left `note:`
/// line is prose, not shape: only the queued-elsewhere warning is captured (verbatim, never
/// paraphrased — ER-02), and any other `note:` line — including prikk's generic "use `prikk commit`"
/// hint — passes through unread rather than causing a refusal. Refusing on an unrecognized `note:`
/// would make every future prikk note a user-visible outage; only a malformed *count* or a missing
/// *headline* refuses.
pub(super) fn worktree_status(text: &str) -> Result<WorktreeStatus> {
    let headline = field(text, "worktree:").ok_or_else(|| {
        StikkError::environment_msg(
            "prikk worktree-status output is missing the worktree: headline",
        )
    })?;
    let clean = headline.starts_with("clean");
    let reff = required_ref_field(text, "ref:")?;
    let entries = text
        .lines()
        .filter(|line| line.starts_with(' ') || line.starts_with('\t'))
        .filter_map(parse_worktree_entry)
        .collect();
    let queued_elsewhere = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(QUEUED_ELSEWHERE_PREFIX))
        .map(str::to_string);
    Ok(WorktreeStatus {
        reff,
        clean,
        tracked: required_u64(text, "tracked files:")?,
        unchanged: required_u64(text, "unchanged files:")?,
        missing: required_u64(text, "missing files:")?,
        modified: required_u64(text, "modified files:")?,
        untracked: required_u64(text, "untracked files:")?,
        unsupported: required_u64(text, "unsupported paths:")?,
        entries,
        queued_elsewhere,
    })
}

/// Decode one indented entry line `  <kind> <path> — <note>`. Returns `None` for an indented line
/// that is not an entry (its first word is not a change kind), e.g. a wrapped note.
fn parse_worktree_entry(line: &str) -> Option<WorktreeEntry> {
    let trimmed = line.trim_start();
    let (kind, rest) = trimmed.split_once(' ')?;
    if !WORKTREE_KINDS.contains(&kind) {
        return None;
    }
    // The path runs up to the " — " separator; the note is the remainder. A path may contain spaces,
    // so split on the separator rather than on whitespace.
    let (path, note) = match rest.split_once(" — ") {
        Some((path, note)) => (path.trim(), note.trim()),
        None => (rest.trim(), ""),
    };
    Some(WorktreeEntry {
        kind: kind.to_string(),
        path: path.to_string(),
        note: note.to_string(),
    })
}

/// A required string field.
fn required_field(text: &str, label: &str) -> Result<String> {
    Ok(field(text, label)
        .ok_or_else(|| StikkError::environment_msg(format!("prikk output is missing {label:?}")))?
        .to_string())
}

/// A required ref-name field: fetched like [`required_field`], then validated through
/// [`RefName::parse`] (RFC 012 F-d, `INV-9`) — empty or control-character-bearing is a shape prikk
/// would never emit, so it refuses (UD-02) rather than passing the raw text through. The validated
/// value is still returned as a plain `String`: `RefName` exists to guard this parse boundary, the
/// same role `ObjectId::parse` already plays for ids here (RFC 009 F2) — neither newtype's adoption
/// changed the struct field types it validates, and this stays consistent with that precedent (see the
/// review request for the fuller rationale, including the diff-size measurement behind it).
fn required_ref_field(text: &str, label: &str) -> Result<String> {
    let value = required_field(text, label)?;
    RefName::parse(&value).map_err(|_| {
        StikkError::environment_msg(format!(
            "prikk field {label:?} is not a valid ref name: {value:?}"
        ))
    })?;
    Ok(value)
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

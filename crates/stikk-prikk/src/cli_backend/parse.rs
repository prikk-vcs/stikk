//! Output parsers for prikk commands, confined here per design SEAM-03 (UD-02).
//!
//! Every parser refuses rather than fabricates on an unrecognized shape: a missing expected field is
//! an [`StikkError::Environment`], never a silent default, because encountering it means either
//! stikk misread prikk or the prikk version is outside the validated range. Golden fixtures in
//! `cli_backend/tests.rs` pin the shapes these read.

use stikk_model::{Result, StikkError};

use crate::Orientation;

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

#[cfg(test)]
mod tests;

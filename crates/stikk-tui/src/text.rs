//! The inert-text primitive (threat model C-T2a, handoff §5).
//!
//! Repository-sourced strings — ref names, tag messages, patch operation text, and even the version
//! line prikk prints — can carry terminal control sequences. A front-end that passed them to the
//! terminal verbatim could let hostile content corrupt the display or spoof stikk's own chrome
//! (threat T-T2). Every string that originates outside stikk is routed through [`inert`] before it
//! reaches a cell.
//!
//! Orientation (this increment) shows only a handful of such strings, but the primitive is built and
//! tested here because the next increment (History, Patch detail) renders untrusted content in bulk,
//! and this is the cheapest place to establish the control.

/// Return a display-safe copy of `input`: every control character (C0, DEL, and any other Unicode
/// control) is replaced with the Unicode replacement character `U+FFFD`, so nothing reaching the
/// terminal can be an escape/control sequence. Ordinary text is returned unchanged.
#[must_use]
pub fn inert(input: &str) -> String {
    if input.chars().any(char::is_control) {
        input
            .chars()
            .map(|ch| if ch.is_control() { '\u{FFFD}' } else { ch })
            .collect()
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests;

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

/// Word-wrap `text` to `width` columns, prefixing every produced line with `indent`.
///
/// **Why stikk wraps rather than `Paragraph::wrap`** (RFC 023 F2): the Glossary needs to *scroll*, and a
/// scroll clamp that cannot overshoot needs the **exact** number of rendered lines. `Wrap { trim: false }`
/// wraps inside the widget, after the line count is out of reach — ratatui exposes it only behind the
/// unstable `rendered-line-info` feature — so the clamp would have to guess. Wrapping here makes the
/// count a fact: `wrap_indented(...).len()` is precisely what will be drawn. RFC 018 shipped wrap without
/// scroll and had to revert it; this is what lets the two land together.
///
/// A word longer than the available width is hard-split rather than allowed to overflow, because the
/// paragraph is no longer wrapping and would clip it. Wrapping is by `char` count, which is what the
/// terminal grid measures for the Latin text and box characters stikk renders.
///
/// Returns at least one line, even for empty input, so a blank line stays a blank line.
#[must_use]
pub fn wrap_indented(text: &str, width: usize, indent: &str) -> Vec<String> {
    let available = width.saturating_sub(indent.chars().count()).max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word;
        // Hard-split anything that cannot fit on a line of its own.
        while word.chars().count() > available {
            if !current.is_empty() {
                lines.push(format!("{indent}{current}"));
                current.clear();
            }
            let cut = word
                .char_indices()
                .nth(available)
                .map_or(word.len(), |(byte, _)| byte);
            lines.push(format!("{indent}{}", &word[..cut]));
            word = &word[cut..];
        }
        let would_be = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };
        if would_be > available && !current.is_empty() {
            lines.push(format!("{indent}{current}"));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(format!("{indent}{current}"));
    }
    lines
}

#[cfg(test)]
mod tests;

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

/// Wrap `text` to `width` columns **without changing a single byte of it**, prefixing every produced
/// line with `indent`.
///
/// **This is the wrap repository *content* needs** (RFC 035 review v1 §2.1). [`wrap_indented`]
/// word-wraps, which means it rebuilds each line from `split_whitespace()` — and that is exactly right
/// for a sentence stikk wrote and exactly wrong for a file's bytes: a re-indent, a tab turned into
/// spaces, a double space collapsed to one, or a trailing newline removed would all render as *no
/// change at all*, which is `T-T4`'s confident-but-wrong picture. Spans are where whitespace edits
/// live, so that is the common case, not an edge case.
///
/// So this one never collapses, never trims, and never looks for a word boundary: it hard-splits at the
/// available width, by `char`, which is what the terminal grid measures. A row that fits is returned
/// unchanged. Returns at least one line, even for empty input, so a blank line stays a blank line.
#[must_use]
pub fn wrap_verbatim(text: &str, width: usize, indent: &str) -> Vec<String> {
    let available = width.saturating_sub(indent.chars().count()).max(1);
    let mut lines = Vec::new();
    let mut rest = text;
    while rest.chars().count() > available {
        let cut = rest
            .char_indices()
            .nth(available)
            .map_or(rest.len(), |(byte, _)| byte);
        match (rest.get(..cut), rest.get(cut..)) {
            (Some(head), Some(tail)) => {
                lines.push(format!("{indent}{head}"));
                rest = tail;
            }
            // Unreachable: `cut` is a `char` boundary at or below `rest.len()`. Emitting the remainder
            // whole is the one fallback that cannot drop bytes; the row may overflow and the paragraph
            // clips it, which is visible, where silently losing content would not be.
            _ => break,
        }
    }
    lines.push(format!("{indent}{rest}"));
    lines
}

#[cfg(test)]
mod tests;

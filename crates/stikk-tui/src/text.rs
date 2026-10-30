//! The two rules for text stikk did not write (threat model C-T2a, `T-T2`; RFC 036).
//!
//! Repository- and prikk-sourced strings — ref names, paths, messages, and the version line prikk
//! prints — can carry characters that do not show as themselves. A front-end that passed them to the
//! terminal verbatim could let hostile content reorder what is read, hide itself, or make one name
//! look like another. **Every such string goes through exactly one of two functions here**, and
//! stikk's own words go through neither.
//!
//! - **[`inert`] — prose.** Messages, refusals, notes, reasons, routes. Every character that does not
//!   show as itself is written as its escape, in place; `"` and `\` are left alone and nothing is
//!   quoted. The promise is `T-T2` only: nothing invisible or reordering reaches the terminal. Prose
//!   may still reflow its whitespace in layout, as it always has.
//! - **[`identifier`] — paths, ref names, key ids, prikk's kind words as tags.** The same promise,
//!   **plus**: two different identifiers never look alike. Plain when nothing needs escaping and the
//!   whitespace is unremarkable; otherwise quoted, with every whitespace character inside the quotes
//!   written as an escape — so no word-wrap can collapse, trim or break inside one.
//!
//! **Neither decides what is invisible.** Both ask [`stikk_prikk::escape`], the one table RFC 035
//! built for Patch detail's content. A second table is how two spellings drift apart.
//!
//! **What [`inert`] used to do, and why it stopped.** Until RFC 036 it replaced every
//! [`char::is_control`] character with `U+FFFD` and passed everything else through. `is_control` is
//! Unicode category `Cc` alone, so a bidi override, a zero-width character and a non-breaking space
//! all reached the terminal — measured at prikk 0.51, which reports all three in a worktree path and
//! accepts the first two in a branch name. And `U+FFFD` said *something was here* without saying
//! what. **`U+FFFD` is stikk's no longer**: prikk substitutes it for filename bytes it cannot show,
//! and `stikk-core`'s commit guidance keys a next step on seeing one, so one on screen now means
//! prikk put it there.

/// **The prose rule** (RFC 036 decision 2): a display-safe copy of `input` in which every character
/// that does not show as itself is written as its escape — `\n`, `\t`, `\r`, `\b`, `\f`, or
/// `\uXXXX`, with a surrogate pair above `U+FFFF`.
///
/// `"` and `\` are **left exactly as they are**, and nothing is quoted: a refusal prikk wrote with a
/// quotation mark in it must read as prikk wrote it (`ER-02`). Visible text — `é`, `日本`, an emoji —
/// is returned unchanged.
///
/// **This is not [`identifier`].** Prose makes one promise, `T-T2`: nothing invisible or reordering
/// reaches the terminal. It does **not** promise that two different messages look different, because
/// wrapping may reflow their whitespace and that has always been so. A string where that second
/// promise matters is an identifier, and there is a function for it.
#[must_use]
pub fn inert(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    stikk_prikk::escape::escape_invisible_into(&mut out, input);
    out
}

/// **The identifier rule** (RFC 036 decision 3): a path, ref name, key id or kind word rendered so
/// that **two different identifiers never look alike**, and so that no layout pass can change one.
///
/// **Plain** — returned as it is — when the value is non-empty, nothing in it needs an escape, it
/// holds no `"` and no `\`, it neither starts nor ends with whitespace, and it has no run of two or
/// more whitespace characters. That is nearly every identifier a user will ever see, so the common
/// case costs nothing visually.
///
/// **Otherwise quoted**: `"`, the value escaped as it would be inside JSON (so `"` and `\` are
/// escaped too), **with every whitespace character written as an escape** — a tab as `\t`, a
/// space as `\u0020` — then `"`. The quoted form therefore contains **no whitespace at all**, which is what makes it
/// survive [`wrap_indented`]: there is nothing inside it for `split_whitespace` to collapse, trim or
/// break on. A quoted identifier longer than a row is hard-split by the wrapper and loses nothing.
///
/// The empty identifier renders as `""`, because nothing is not the same fact as a name that is
/// absent.
///
/// **Why a space must be escaped inside the quotes.** prikk's own path validator accepts `a  b.txt`,
/// ` lead.txt` and `say"hi.txt` — measured at 0.51, where all three are reported as ordinary
/// untracked files that a commit would author. Quoting alone would keep them distinct on the page
/// but not through [`wrap_indented`], which rebuilds every row from `split_whitespace`.
#[must_use]
pub fn identifier(value: &str) -> String {
    if is_plain(value) {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    let mut buffer = [0u8; 4];
    for ch in value.chars() {
        if ch == ' ' {
            // **The one character the shared table lets through that this rule may not.** Inside the
            // quotes nothing may be whitespace, because that is what makes the quoted form survive a
            // word-wrap unchanged — and `U+0020` is the only whitespace `escape_into` writes as
            // itself (every other one answers `needs_unicode_escape`, and `\n`, `\r` and `\t` take
            // their short escapes there). Spelling it `\u0020` here keeps one notation across the
            // product rather than inventing a second for identifiers.
            out.push_str("\\u0020");
        } else {
            stikk_prikk::escape::escape_into(&mut out, ch.encode_utf8(&mut buffer));
        }
    }
    out.push('"');
    out
}

/// Whether `value` can be shown as itself: the conditions [`identifier`] documents.
fn is_plain(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let mut previous_was_whitespace = false;
    for (at, ch) in value.chars().enumerate() {
        if ch == '"' || ch == '\\' || stikk_prikk::escape::needs_unicode_escape(ch) {
            return false;
        }
        if ch.is_whitespace() {
            // A leading space, or two in a row. The trailing case is caught after the loop.
            if at == 0 || previous_was_whitespace {
                return false;
            }
            previous_was_whitespace = true;
        } else {
            previous_was_whitespace = false;
        }
    }
    !previous_was_whitespace
}

/// **Render a sentence core returned as pieces** (RFC 036 review v1 §2.3): each [`stikk_core::Piece::Said`]
/// under the prose rule, each [`stikk_core::Piece::Name`] under the identifier rule, joined in order.
///
/// This is the one place the two rules meet. Core decides every word and which pieces are names;
/// this decides how a name looks on a terminal, which is the frontend's to decide and a GUI's to
/// decide differently.
#[must_use]
pub fn compose(pieces: &[stikk_core::Piece]) -> String {
    pieces
        .iter()
        .map(|piece| match piece {
            stikk_core::Piece::Said(text) => inert(text),
            stikk_core::Piece::Name(value) => identifier(value),
        })
        .collect()
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
        // **Hard-split anything that cannot fit on a line of its own, evenly** ([`even_rows`],
        // RFC 056 Handoff B §1). The last row is left in `word` to carry on wrapping with
        // whatever follows it, as it always did.
        if word.chars().count() > available {
            if !current.is_empty() {
                lines.push(format!("{indent}{current}"));
                current.clear();
            }
            let rows = even_rows(word, available);
            let kept = rows.last().map(String::len).unwrap_or_default();
            for row in rows.iter().take(rows.len().saturating_sub(1)) {
                lines.push(format!("{indent}{row}"));
            }
            word = word.get(word.len() - kept..).unwrap_or(word);
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

/// Split one value into the fewest rows that fit `available`, **as evenly as possible**
/// (RFC 056 Handoff B §1).
///
/// **Why even.** Filling each row and leaving the remainder on the last splits a 64-character
/// key as 63 + 1, which reads as a defect and is hard to compare against another key. The same
/// value as 32 + 32 reads as one value on two rows. The row count is unchanged — this is the
/// same number of rows, differently filled — so nothing that fits on one row is ever split.
///
/// Rows differ by at most one character, and the longer ones come first. Measured by `char`,
/// which is what the terminal grid measures, and what the two callers already count by.
fn even_rows(text: &str, available: usize) -> Vec<String> {
    let available = available.max(1);
    let units = split_units(text);
    let total = units.len();
    if total <= available {
        return vec![text.to_string()];
    }
    // The fewest rows that can hold it, which is what filling each row would have used.
    let rows = total.div_ceil(available);
    let base = total / rows;
    // The first `remainder` rows take one unit more, so the rows differ by at most one.
    let remainder = total % rows;
    let mut units = units.into_iter();
    (0..rows)
        .map(|row| {
            let take = base + usize::from(row < remainder);
            units.by_ref().take(take).collect()
        })
        .collect()
}

/// The pieces [`even_rows`] may split between: one ordinary `char`, or **one whole escape**.
///
/// **Why an escape is one piece** (RFC 056 Handoff B §4 trap 2). Everything these helpers draw
/// has already been through `inert`/`identifier`, which write a character that does not show as
/// itself as `\uXXXX` — and as a surrogate pair, `\uXXXX\uXXXX`, above `U+FFFF`
/// (`stikk_prikk::escape`). Splitting inside one would put `\u20` at the end of a row and `2E`
/// at the start of the next, which reads as neither the escape nor the character: the one
/// notation stikk invented to make a hidden character visible would itself become unreadable.
///
/// A piece is **not** a column: a `\uXXXX` occupies six of them. Rows of equal *pieces* can
/// therefore differ in width, which is the right trade — nothing overflows, because a row's
/// pieces are counted against the available width only when they are all single characters,
/// and an escaped value is drawn at whatever width its pieces come to, which is never more than
/// the unescaped count would have been.
fn split_units(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut units: Vec<String> = Vec::new();
    let mut at = 0;
    while let Some(rest) = chars.get(at..) {
        if rest.is_empty() {
            break;
        }
        let take = escape_len(rest).unwrap_or(1).min(rest.len());
        units.push(rest.iter().take(take).collect());
        at += take;
    }
    units
}

/// The length in `char`s of the escape starting here, or `None` where no escape starts here.
///
/// `\uXXXX` is six; a surrogate pair is twelve, kept together because the two halves spell one
/// character and neither half means anything alone.
fn escape_len(rest: &[char]) -> Option<usize> {
    let one = |from: usize| -> bool {
        rest.get(from) == Some(&'\\')
            && rest.get(from + 1) == Some(&'u')
            && (from + 2..from + 6).all(|at| rest.get(at).is_some_and(char::is_ascii_hexdigit))
    };
    if !one(0) {
        return None;
    }
    // A high surrogate is `\ud800`..=`\udbff`; the pair is written only by `escape`'s own
    // astral path, so a following `\uXXXX` after one is its low half.
    let high = matches!(rest.get(2), Some('d' | 'D'))
        && matches!(rest.get(3), Some('8'..='9' | 'a' | 'b' | 'A' | 'B'));
    if high && one(6) {
        return Some(12);
    }
    Some(6)
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
///
/// **The split is even** (RFC 056 Handoff B §1): the same rows a fill-and-remainder split would
/// have used, filled equally, so a 64-hex id reads as 32 + 32 rather than 63 + 1. An escape
/// (`\uXXXX`, or a surrogate pair) is one piece and is never cut across rows.
#[must_use]
pub fn wrap_verbatim(text: &str, width: usize, indent: &str) -> Vec<String> {
    let available = width.saturating_sub(indent.chars().count()).max(1);
    even_rows(text, available)
        .into_iter()
        .map(|row| format!("{indent}{row}"))
        .collect()
}

#[cfg(test)]
mod tests;

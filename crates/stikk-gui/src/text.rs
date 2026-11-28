//! **The two rules for text stikk did not write** (threat model C-T2a, `T-T2`; RFC 036), for a
//! window.
//!
//! Repository- and prikk-sourced strings — ref names, paths, messages, and the version line prikk
//! prints — can carry characters that do not show as themselves. **Every such string goes through
//! exactly one of these two functions**, and stikk's own words go through neither.
//!
//! - **[`inert`] — prose.** Messages, refusals, notes, reasons, routes.
//! - **[`identifier`] — paths, ref names, block ids, key ids, prikk's kind words as tags.** The
//!   same promise, **plus**: two different identifiers never look alike.
//!
//! **Neither decides what is invisible.** Both ask `stikk_prikk::escape`, the one table RFC 035
//! built — the same table the terminal's own two functions ask. A second table is how two
//! spellings drift apart.
//!
//! **Why this is not `stikk_tui::text`.** RFC 036's own words: how a name looks is *"the
//! frontend's to decide, and a GUI's to decide differently"*. The terminal's identifier rule exists
//! to survive `split_whitespace` word-wrapping, which a window does not do. What is shared is the
//! table and the two promises; what differs is the rendering — and in this app the rendering also
//! wraps every value in rule 12's isolates, through the design crate. `stikk-gui` cannot depend on
//! `stikk-tui` in any case: `deny.toml` keeps ratatui inside that one crate (RFC 067 decision 6).

/// **The prose rule** (RFC 036 decision 2): a display-safe copy of `input` in which every character
/// that does not show as itself is written as its escape.
///
/// `"` and `\` are **left exactly as they are**, and nothing is quoted: a refusal prikk wrote with
/// a quotation mark in it must read as prikk wrote it (`ER-02`). Visible text — `é`, `日本`, an
/// emoji — is returned unchanged.
#[must_use]
pub fn inert(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    stikk_prikk::escape::escape_invisible_into(&mut out, input);
    out
}

/// **The identifier rule** (RFC 036 decision 3): a path, ref name, block id, key id or kind word
/// rendered so that **two different identifiers never look alike**.
///
/// **Plain** when the value is non-empty, nothing in it needs an escape, it holds no `"` and no
/// `\`, it neither starts nor ends with whitespace, and it has no run of two or more whitespace
/// characters. That is nearly every identifier a user will ever see, so the common case costs
/// nothing visually.
///
/// **Otherwise quoted**, with every whitespace character inside the quotes written as an escape, so
/// the quoted form holds no whitespace at all and no layout pass can change it.
///
/// The empty identifier renders as `""`, because nothing is not the same fact as a name that is
/// absent.
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
            // The one character the shared table lets through that this rule may not: inside the
            // quotes nothing may be whitespace. One notation across the product, as the terminal
            // spells it.
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

/// **A sentence core returned as pieces**, rendered: each `Said` under the prose rule, each `Name`
/// under the identifier rule, joined in order.
///
/// The one place the two rules meet. Core decides every word and which pieces are names.
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

#[cfg(test)]
mod tests;

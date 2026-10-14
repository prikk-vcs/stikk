//! One spelling for repository text, shared by every view that shows it verbatim (RFC 035 review v2
//! §2).
//!
//! # Why this is not `is_control()`
//!
//! Patch detail quotes a file's content so that **two spans differing in any byte cannot render
//! identically** (review v1 §2.1). The first implementation escaped `"`, `\`, `\n`, `\r`, `\t` and
//! [`char::is_control`] — and `is_control` is Unicode general category `Cc` alone, 65 code points.
//! Everything below passed through as itself, and none of it *is* itself on a terminal:
//!
//! - **Bidi controls** (`U+202A`–`U+202E`, `U+2066`–`U+2069`, `U+200E`, `U+200F`, `U+061C`) reorder the
//!   displayed text. That is "Trojan Source" ([CVE-2021-42574]): source that reads one way and runs
//!   another. **stikk is a history browser for code**, and Patch detail is the first view that renders
//!   whole files of it, so it is exactly the view that has to resist this (`T-T2`, `T-T4`).
//! - **Zero-width characters** (`U+200B`–`U+200D`, `U+2060`, `U+FEFF`, `U+00AD`) display nothing, so
//!   `"x"` and `"x\u{200b}"` looked alike.
//! - **Whitespace that is not `U+0020`** (`U+00A0`, `U+2000`–`U+200A`, `U+202F`, `U+3000`, …) looks
//!   like a space, and `U+2028`/`U+2029` break the row.
//! - **Tag characters** (`U+E0000`–`U+E007F`) display nothing.
//!
//! So the rule is not a category but a question: **does this character show as itself?** Ordinary
//! visible non-ASCII — `é`, `日本`, an emoji — does, and is written as itself. What does not is written
//! as an escape.
//!
//! # The list, and where it comes from
//!
//! [`needs_unicode_escape`] is the union of three sets, named so the table can be re-derived rather
//! than trusted:
//!
//! 1. [`char::is_control`] — general category `Cc`.
//! 2. [`char::is_whitespace`] — the Unicode `White_Space` property — **except `U+0020`**, the one that
//!    does show as itself (and except `\n`, `\r`, `\t`, which keep their shorter escapes).
//! 3. `IGNORABLE`, the table below: the union of Unicode's **`Default_Ignorable_Code_Point`** property and general
//!    category **`Cf`** (format characters), from Unicode 15.1. Every bidi control in the list above is
//!    in both; `Cf` adds the ones that are not default-ignorable, such as the interlinear annotation
//!    anchors `U+FFF9`–`U+FFFB`.
//!
//! The table is written out rather than computed because stikk's dependency tree is one crate deep
//! (`json.rs` says why), and because a list that can be read is a list that can be checked.
//!
//! # Why `stikk-prikk`, and why one function
//!
//! **Two views render repository content, and they must not drift**: Patch detail's quoted rows
//! (`stikk-core`'s `patch` module) and its raw view of prikk's own JSON (this crate's `json` module).
//! They had two copies of the escape table, agreeing on the day they were written — the shape
//! `version.rs` records as how
//! `0.30` survived in a second copy after the ceiling moved twice. One function, in the crate core
//! already depends on, is the fix.
//!
//! **What this is not.** It is not `stikk-core`'s `inert`, which every *other* repository string goes
//! through — paths, messages, ref names. That has the same `Cc`-only rule and the same gap, it predates
//! this increment, it spans every view, and widening it is its own increment (review v2 §2). This
//! predicate is what that increment reuses.
//!
//! [CVE-2021-42574]: https://www.cve.org/CVERecord?id=CVE-2021-42574

/// Code points that are not themselves on screen: Unicode's `Default_Ignorable_Code_Point` property
/// unioned with general category `Cf`, as of Unicode 15.1. Inclusive ranges, sorted and disjoint, so
/// [`needs_unicode_escape`] can binary-search them.
///
/// `U+2028` and `U+2029` are absent on purpose: they are `White_Space`, and the whitespace rule above
/// already catches them.
static IGNORABLE: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),   // SOFT HYPHEN
    (0x034F, 0x034F),   // COMBINING GRAPHEME JOINER
    (0x0600, 0x0605),   // Arabic number signs
    (0x061C, 0x061C),   // ARABIC LETTER MARK (bidi)
    (0x06DD, 0x06DD),   // ARABIC END OF AYAH
    (0x070F, 0x070F),   // SYRIAC ABBREVIATION MARK
    (0x0890, 0x0891),   // Arabic pound/piastre marks
    (0x08E2, 0x08E2),   // ARABIC DISPUTED END OF AYAH
    (0x115F, 0x1160),   // Hangul choseong/jungseong fillers
    (0x17B4, 0x17B5),   // Khmer inherent vowels
    (0x180B, 0x180F),   // Mongolian variation selectors, vowel separator
    (0x200B, 0x200F),   // ZWSP, ZWNJ, ZWJ, LRM, RLM (bidi)
    (0x202A, 0x202E),   // bidi embeddings and overrides
    (0x2060, 0x206F),   // word joiner, invisible operators, bidi isolates, deprecated formats
    (0x3164, 0x3164),   // HANGUL FILLER
    (0xFE00, 0xFE0F),   // variation selectors
    (0xFEFF, 0xFEFF),   // ZERO WIDTH NO-BREAK SPACE (BOM)
    (0xFFA0, 0xFFA0),   // HALFWIDTH HANGUL FILLER
    (0xFFF0, 0xFFFB),   // reserved default-ignorables, interlinear annotation anchors
    (0x110BD, 0x110BD), // KAITHI NUMBER SIGN
    (0x110CD, 0x110CD), // KAITHI NUMBER SIGN ABOVE
    (0x13430, 0x1343F), // Egyptian hieroglyph format controls
    (0x1BCA0, 0x1BCA3), // shorthand format controls
    (0x1D173, 0x1D17A), // musical format controls
    (0xE0000, 0xE0FFF), // tag characters and variation selectors supplement
];

/// Whether `ch` must be written as a `\uXXXX` escape because it does not show as itself.
///
/// The three sets are named in this module's documentation. `\n`, `\r`, `\t`, `\u{8}` and `\u{c}`
/// answer `true` here and are still written with their shorter escapes by [`escape_into`] — this
/// predicate says *that* a character is escaped, not *how*.
#[must_use]
pub fn needs_unicode_escape(ch: char) -> bool {
    if ch == ' ' {
        return false;
    }
    if ch.is_control() || ch.is_whitespace() {
        return true;
    }
    let code = u32::from(ch);
    IGNORABLE
        .binary_search_by(|&(low, high)| {
            if code < low {
                std::cmp::Ordering::Greater
            } else if code > high {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Append `text` to `out` as the **body** of a JSON string literal — no surrounding quotes, so the
/// caller decides whether this is a quoted content row or a value inside rendered JSON.
///
/// The spelling is JSON's, because prikk's own reports are JSON: a reader who sees `\t` in a quoted
/// content row and `\t` in the raw view is looking at the same convention, not two of stikk's.
pub fn escape_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ch if needs_unicode_escape(ch) => push_unicode_escape(out, ch),
            ch => out.push(ch),
        }
    }
}

/// `text` as a complete quoted JSON string literal — [`escape_into`] with the quotes.
#[must_use]
pub fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    escape_into(&mut out, text);
    out.push('"');
    out
}

/// Append `text` to `out` with **only what does not show as itself** escaped: every character for
/// which [`needs_unicode_escape`] holds becomes its escape, and `"` and `\` are left exactly as they
/// are.
///
/// **This is the prose rule** (RFC 036 decision 2), and it is the difference between the two rules
/// this workspace has. [`escape_into`] additionally escapes `"` and `\`, because its output goes
/// inside quotes; prose is not quoted, so a refusal that contains a quotation mark must read as prikk
/// wrote it. Both call the same table, which is the point: a character is invisible here exactly when
/// it is invisible there.
pub fn escape_invisible_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ch if needs_unicode_escape(ch) => push_unicode_escape(out, ch),
            ch => out.push(ch),
        }
    }
}

/// Write `ch` as `\uXXXX`, **as a surrogate pair above `U+FFFF`**, which is how JSON spells an astral
/// character.
///
/// `\u{:04x}` alone is not injective up there: `U+E0001` would print `1`, which also reads as
/// `U+E000` followed by `1`. Writing `󠀁` instead keeps the escape unambiguous *and* keeps
/// the raw view's output parseable as the JSON it claims to be.
fn push_unicode_escape(out: &mut String, ch: char) {
    let code = u32::from(ch);
    match code.checked_sub(0x1_0000) {
        Some(above) => {
            let high = 0xD800 + (above >> 10);
            let low = 0xDC00 + (above & 0x3FF);
            out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
        }
        None => out.push_str(&format!("\\u{code:04x}")),
    }
}

#[cfg(test)]
mod tests;

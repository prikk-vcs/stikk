//! The top-level views (design TU-01): [`orientation`], [`history`], [`block`] (block detail),
//! [`changes`], [`queue`], [`patch`] (patch detail), [`compare`], [`verify`] and [`merge`] (the
//! merge plan). Each renders a `stikk-core` view-model into a body region and computes nothing
//! itself.
//!
//! [`wrap_spans`] is the one piece of rendering shared between them: a **span-aware** hard split,
//! for a row that is already a sequence of differently-styled fields and so cannot be wrapped as a
//! string.

pub mod block;
pub mod changes;
pub mod compare;
pub mod history;
pub mod merge;
pub mod orientation;
pub mod patch;
pub mod queue;
pub mod verify;

/// Word-wrap `text` to `width`, opening at `indent` and hanging the rest at `hang`, and return
/// the rows **already prefixed**.
///
/// **Each row gets the room its own indent leaves**, which is not one number when the two indents
/// differ. Wrapping the whole thing to either one is wrong in a different direction: to the first
/// row's room and a continuation is laid out wider than it has, so it **clips at the border**; to
/// the hanging room and the first row stops short of the border for no reason a reader can see.
/// Both faults have been found by measurement in this codebase rather than reasoned about — the
/// clip twice, in two different views.
///
/// Re-joining the leftover rows with single spaces loses nothing: [`crate::text::wrap_indented`]
/// has already collapsed the text's whitespace, and this is for stikk's own prose and for strings
/// composed through the two rules, where an identifier that needed escaping is quoted with every
/// space spelled (RFC 036 decision 3).
pub(crate) fn wrap_hanging(text: &str, width: usize, indent: &str, hang: &str) -> Vec<String> {
    use crate::text::wrap_indented;

    let mut first = wrap_indented(text, width, indent).into_iter();
    let Some(head) = first.next() else {
        return Vec::new();
    };
    let mut rows = vec![head];
    // What the first row could not take, laid out again to the hanging indent's own room.
    // **The whole opening indent comes off, not just its whitespace.** `wrap_indented` prefixes
    // every row it returns, so a continuation carries the first row's marker too -- and an indent
    // like `    ! ` ends in a character a trim would leave behind, repeating the marker down the
    // side of the paragraph (measured on the first run).
    let leftover = first
        .map(|row| {
            row.strip_prefix(indent)
                .unwrap_or(row.as_str())
                .trim_start()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ");
    if !leftover.is_empty() {
        rows.extend(wrap_indented(&leftover, width, hang));
    }
    rows
}

/// Hard-split a row of styled spans to `width`, continuing each further row under `indent`.
///
/// The string wraps in [`crate::text`] take a string, so a row assembled from several spans — a
/// History block row, whose id, update-seq, kind and markers each carry their own style — has no
/// way to use them without flattening the styles away. This splits the **span sequence** instead,
/// so every character keeps the style it was given.
///
/// It is a hard split ([`crate::text::wrap_verbatim`]'s rule), not a word wrap, for the reason the
/// patch summary row in [`block`] is: the row's own separators are runs of spaces that place its
/// columns, and a word wrap rebuilds a line from its whitespace-separated words, which would
/// collapse them. Nothing is added, removed or reordered — concatenating the output rows, each
/// without its indent, gives back exactly the input text.
pub(crate) fn wrap_spans<'a>(
    spans: Vec<ratatui::text::Span<'a>>,
    width: usize,
    indent: &str,
) -> Vec<ratatui::text::Line<'a>> {
    use ratatui::text::{Line, Span};

    let first_room = width.max(1);
    // A continuation row spends `indent` before any content, so it has that much less room. `max(1)`
    // keeps progress possible at a width narrower than the indent itself.
    let continued_room = width.saturating_sub(indent.chars().count()).max(1);

    let mut rows: Vec<Line<'a>> = Vec::new();
    let mut current: Vec<Span<'a>> = Vec::new();
    let mut free = first_room;

    for span in spans {
        let style = span.style;
        // By characters, not bytes: splitting a row mid-character would render a replacement
        // glyph, which is a changed string (C-T2a).
        let mut chars = span.content.chars().peekable();
        while chars.peek().is_some() {
            if free == 0 {
                rows.push(Line::from(std::mem::take(&mut current)));
                current.push(Span::raw(indent.to_string()));
                free = continued_room;
            }
            // `free` is at least 1 here, so each pass places at least one character and the loop
            // terminates. It also means the indent just pushed is never left as a row of its own.
            let chunk: String = chars.by_ref().take(free).collect();
            free -= chunk.chars().count();
            current.push(Span::styled(chunk, style));
        }
    }

    if !current.is_empty() {
        rows.push(Line::from(current));
    }
    rows
}

#[cfg(test)]
mod tests;

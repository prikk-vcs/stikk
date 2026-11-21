//! **The key line** (design `TU-05`; RFC 061 decision 5): one row, above the status bar, saying
//! which keys this screen has.
//!
//! # Why it exists
//!
//! The keys used to be named in view titles, in the status bar's hint and in a help overlay, and a
//! reader on History saw three of the nineteen they had (RFC 061 F5). The line names them where they
//! are used, from the catalog, so a key cannot appear here that nothing runs.
//!
//! # What goes when it does not fit
//!
//! **Whole entries, lowest [`rank`](stikk_core::Action::rank) first.** An entry is a key and its
//! word, and half of either says nothing: `b bran` is not a shorter way of saying *branches*. `?
//! help` and `q back` rank 255 and never drop — the one key that explains the others, and the way
//! out — and *"Enter actions"* ranks 254, so it goes last of the rest.

use ratatui::text::{Line, Span};
use stikk_core::Action;

use crate::theme::Palette;

/// The separator between entries.
const SEPARATOR: &str = " \u{00b7} ";

/// The rank that never drops, and sits at the end of the line: `? help` and `q back`.
const NEVER: u8 = 255;

/// **The line's order: by rank, highest first, with the two that never drop at the end** (RFC 061
/// decision 5's own example).
///
/// Rank, not catalog order, because the key a reader most needs is the one this screen or this row
/// has — `u` on Changes, `m` on a block — and those outrank the keys every screen carries. (The
/// catalog's own order is what the palette and the action lists read, where a row's *open* action
/// must come first.)
fn ordered(actions: &[&'static Action]) -> Vec<&'static Action> {
    let mut rest: Vec<(usize, &'static Action)> = actions
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, action)| action.hint.is_some() && action.rank < NEVER)
        .collect();
    rest.sort_by_key(|(index, action)| (std::cmp::Reverse(action.rank), *index));
    let pinned = actions
        .iter()
        .copied()
        .filter(|action| action.hint.is_some() && action.rank == NEVER);
    rest.into_iter()
        .map(|(_, action)| action)
        .chain(pinned)
        .collect()
}

/// **The line, fitted to `width`** — the keys `actions` lists, in that order, with the lowest-ranked
/// dropped until what remains fits.
#[must_use]
pub(crate) fn line(actions: &[&'static Action], palette: &Palette, width: usize) -> Line<'static> {
    let mut kept = ordered(actions);
    while drawn_width(&kept) > width && kept.len() > 1 {
        // The lowest rank goes, and `? help` and `q back` never do — so at the narrowest the line
        // is those two, and below even that it is them and nothing else to give up.
        let Some(at) = kept
            .iter()
            .enumerate()
            .filter(|(_, action)| action.rank < NEVER)
            .min_by_key(|(index, action)| (action.rank, std::cmp::Reverse(*index)))
            .map(|(index, _)| index)
        else {
            break;
        };
        kept.remove(at);
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    for action in &kept {
        if !spans.is_empty() {
            spans.push(Span::styled(
                SEPARATOR,
                ratatui::style::Style::default().fg(palette.dim),
            ));
        }
        let Some(key) = action.key else { continue };
        let Some(hint) = action.hint else { continue };
        // **The key in the body colour, its word dim** (decision 5): the key is what a reader
        // presses, and the word is what it does.
        spans.push(Span::styled(
            key.label(),
            ratatui::style::Style::default().fg(palette.fg),
        ));
        spans.push(Span::styled(
            format!(" {hint}"),
            ratatui::style::Style::default().fg(palette.dim),
        ));
    }
    let line = Line::from(spans);
    crate::text::record_overflow("key line", std::slice::from_ref(&line), width);
    line
}

/// How wide these entries draw, separators included.
fn drawn_width(actions: &[&'static Action]) -> usize {
    let entries: usize = actions
        .iter()
        .map(|action| {
            let key = action.key.map_or(0, |key| key.label().chars().count());
            let hint = action.hint.map_or(0, |hint| hint.chars().count());
            key + 1 + hint
        })
        .sum();
    let separators = actions.len().saturating_sub(1) * SEPARATOR.chars().count();
    entries + separators
}

#[cfg(test)]
mod tests;

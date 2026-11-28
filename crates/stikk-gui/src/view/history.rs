//! **The History screen** (design VW-03, FR-010/011; RFC 006), drawn to RFC 069's rules.
//!
//! A ref's block lineage, newest first, preceded by the **queued tier** — the patches in the active
//! WAL that are *not yet history* (FR-010), in core's own words. prikk's history is
//! block-granular, so a row is one sealed block.
//!
//! **Lines, not panels** (rule 4): the tier is separated from the lineage by one line, and the rows
//! by nothing at all, because a list of rows at `body`'s line height reads as a list without
//! being boxed.

use iced::Element;
use iced::widget::Column;
use stikk_core::HistoryView;
use stikk_gui_design as design;
use stikk_gui_design::Tokens;

use crate::words::Say;
use crate::{Message, Stikk, text};

/// How many characters of a block id a row shows: the same twelve the terminal shows.
const SHORT_ID: usize = 12;

/// The screen.
pub fn view<'a>(
    view: &'a HistoryView,
    cursor: usize,
    _stikk: &'a Stikk,
    tokens: &Tokens,
) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(tokens.spacing.sm);
    rows = rows.push(design::heading(
        tokens,
        Say::HistoryHeading.fill(&[("ref", &design::isolate(&text::identifier(&view.reff)))]),
    ));
    // The "not yet history" tier, in core's words: whose queue it is, and **no line at all when
    // nothing is queued** (RFC 028 decision 5). Core composes a ref name into its own sentence, so
    // this is a string of both kinds and takes the composing rule.
    if let Some(tier) = view.queued_tier() {
        rows = rows.push(design::body(tokens, text::compose(&tier)));
    }
    rows = rows.push(design::line(tokens));
    if view.blocks.is_empty() {
        rows = rows.push(design::body(tokens, Say::HistoryEmpty.template()));
        return rows.into();
    }
    for (at, block) in view.blocks.iter().enumerate() {
        let id: String = block.block_id.chars().take(SHORT_ID).collect();
        let row = Say::HistoryRow.fill(&[
            ("id", &design::isolate(&text::identifier(&id))),
            ("seq", &block.update_seq.to_string()),
            ("kind", &design::isolate(&text::identifier(&block.kind))),
            ("patches", &block.patches.to_string()),
        ]);
        // **The selection is the program's**, and it is shown by weight and never by colour alone
        // (rule 3's *colour never alone*): the selected row is bold, every row is the same size,
        // and nothing moves.
        rows = rows.push(if at == cursor {
            design::strong(tokens, row)
        } else {
            design::body(tokens, row)
        });
    }
    rows.into()
}

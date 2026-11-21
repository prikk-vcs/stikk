//! The Block-detail view (design FR-031/032 at block granularity; RFC 006; RFC 015).
//!
//! Renders one block's metadata and — for the ref tip only — the replayed state file set. prikk
//! exposes no per-block state for older blocks, and `UD-09` narrows rather than retires (RFC 015 F3):
//! per-patch *content* (operations, preimages, `show`/`diff`) and `log --format json` are still
//! absent, but a patch authored by prikk ≥ 0.32 carries an id and a message, and this view names both.
//!
//! **Since RFC 035 it also lists the block's patches** (`FR-030`), from the block's own
//! `show <block-id>` report: one row per patch in prikk's order, with the message joined **by id** from
//! the `log` row, the operation count, and a cursor whose `Enter` opens Patch detail for that patch —
//! out of the report already read, so drilling in spawns nothing. Below stikk's JSON floor the section
//! says, in stikk's own voice, that stikk reads a patch's content from prikk 0.39 on.
//!
//! **The patch count and the message list are never shown one without the other** (RFC 015 decision
//! 2): a block can legitimately report more patches than it lists messages for — a patch authored
//! below prikk 0.32 carries none, which is absence, not an empty message (RFC 015 F4) — and rendering
//! the list alone would tell a user a block contains fewer patches than it does (`T-T4`). Every
//! repository-sourced string (a message included) is routed through [`crate::text::inert`]
//! (`C-T2a`) and is a display string only — never a next-step, never a glossary trigger (`C-T2b`).
//!
//! **Every row is wrapped here, not by `Paragraph`** (RFC 037's carried item). ratatui's own wrap
//! continues a long line at **column 0**, so a 64-hex id and the below-0.46 state sentence used to
//! spill under the labels: nothing was cut, but a reader could not tell which field a continuation
//! belonged to. Rows are now wrapped to **their own value column**, which also makes the line count
//! exact — and it has to be, because the patch cursor and the scroll are computed from it (RFC 023
//! F2 is why this project wraps by hand). **Ids and paths are identifiers and are hard-split**
//! ([`wrap_verbatim`]); stikk's own sentences word-wrap ([`wrap_indented`]).

use std::cell::Cell;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::BlockDetailView;

use crate::text::{compose, identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;
use crate::title::{Position, Title};

/// Render the Block-detail view for `detail` into `area`, with `cursor` selecting one of the block's
/// patches or files.
///
/// **It scrolls** (RFC 060 Handoff C §1), the Queue's way and for the Queue's reason: this view is a
/// header and a list, and at 60 × 15 it drew eleven rows of twenty-two, with no position, no foot
/// line and no way to reach the rest. Where a row is selected the window **follows the cursor**, by
/// the least it can, so the selected row is never off screen and the header rows above it scroll
/// back into view as `↑` walks up past the first one.
pub fn render(
    detail: &BlockDetailView,
    cursor: usize,
    offset: &Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    // Inside the border. Every row below is wrapped to this, so the count is exact.
    let width = usize::from(area.width.saturating_sub(2));
    let (lines, cursor_rows) = block_lines(detail, cursor, palette, width);

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let mut scroll = offset.get().min(max_offset);
    // **The window follows the cursor, by the least it can** — the Queue's own rule (RFC 028 A review
    // v1 §2.1): the selected row's own rows are brought on screen and nothing else moves.
    // `cursor_rows` is where each selectable row's block of rows starts and ends in the wrapped
    // output, which only `block_lines` knows.
    if let Some((from, to)) = cursor_rows.get(cursor).copied() {
        let from = u16::try_from(from).unwrap_or(u16::MAX);
        let to = u16::try_from(to).unwrap_or(u16::MAX);
        if from < scroll {
            scroll = from;
        } else if viewport > 0 && to >= scroll.saturating_add(viewport) {
            scroll = to.saturating_sub(viewport).saturating_add(1);
        }
        scroll = scroll.min(max_offset);
    }
    offset.set(scroll);

    // One title builder, for every frame (RFC 060 review v1 C1), and the position says what is
    // hidden (RFC 024 §4) — `record_cut` below refuses a frame that hides rows without it.
    let position = Position::of(scroll, viewport, lines.len());
    let title = Title::of(if detail.is_tip {
        "Block \u{00b7} tip"
    } else {
        "Block"
    })
    .position(position)
    .render(area.width);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    // **No `Wrap`**: the rows arrive already wrapped, to their own value column. See the module doc.
    // **No row may be wider than the area it is drawn in** (RFC 060 decision 2). This view
    // wraps its own rows to a width taken from `area`, and draws them through a `Paragraph`
    // with no `Wrap`: a row too wide would be cut, so it is recorded in tests instead.
    crate::text::record_overflow(
        "Block detail",
        &lines,
        usize::from(area.width.saturating_sub(2)),
    );
    // **And no row is left below the area in silence** (RFC 060 Handoff C §2).
    crate::text::record_cut(
        "Block detail",
        lines.len(),
        usize::from(viewport),
        position.is_some(),
    );
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// Every row of the view, already wrapped to `width`, so the scroll clamp is exact — and, beside it,
/// **the first and last wrapped row of each selectable row**, in the *cursor's* order, so the caller
/// can scroll to the selected one (RFC 060 Handoff C §1).
///
/// The cursor's order is patches, then files (`stikk_core::block_cursor_row`), while the rendered
/// order is files, then patches. That is RFC 053's, not this handoff's, and it is why these spans are
/// collected separately and joined in the cursor's order rather than in the order they were drawn.
fn block_lines<'a>(
    detail: &BlockDetailView,
    cursor: usize,
    palette: &Palette,
    width: usize,
) -> (Vec<Line<'a>>, Vec<(usize, usize)>) {
    let row = &detail.row;
    let mut lines: Vec<Line> = Vec::new();
    // Where each selectable row's own rows are, by section; joined in the cursor's order at the end.
    let mut patch_rows: Vec<(usize, usize)> = Vec::new();
    let mut file_rows: Vec<(usize, usize)> = Vec::new();
    field_id(&mut lines, palette, width, "block", &row.block_id);
    field_id(&mut lines, palette, width, "ref-state", &row.ref_state_id);
    field_said(
        &mut lines,
        palette,
        width,
        "update-seq",
        &row.update_seq.to_string(),
    );
    field_id(&mut lines, palette, width, "kind", &row.kind);
    field_said(
        &mut lines,
        palette,
        width,
        "parents",
        &row.parents.to_string(),
    );
    field_said(
        &mut lines,
        palette,
        width,
        "patches",
        &patches_summary(row.patches, row.messages.len()),
    );
    if row.rollback_patches > 0 || row.rollback_block {
        field_said(
            &mut lines,
            palette,
            width,
            "rollback",
            &format!(
                "{}{} rollback patch(es)",
                if row.rollback_block {
                    "rollback block · "
                } else {
                    ""
                },
                row.rollback_patches
            ),
        );
    }
    if row.required_attestations > 0 {
        field_said(
            &mut lines,
            palette,
            width,
            "attestations",
            &format!("{} required", row.required_attestations),
        );
    }
    match &row.previous_ref_state {
        Some(id) => field_id(&mut lines, palette, width, "prev-ref-state", id),
        // stikk's own words, not an id.
        None => field_said(
            &mut lines,
            palette,
            width,
            "prev-ref-state",
            "<none — root>",
        ),
    }

    // The message list always travels with the count above it (RFC 015 decision 2) — never rendered
    // on its own, since on its own it would understate the block the moment any patch predates 0.32.
    //
    // **Shown only where the Patches section is not** (RFC 035 §4.1): where the patches are listed,
    // each row already carries its own message, joined by id, beside the operation count — and this
    // list would say the same thing a second time, by a weaker join. Below stikk's JSON floor the
    // Patches section has nothing to list, and this is the only place a message appears at all.
    if detail.patches.unreported_reason().is_some() && !row.messages.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  messages:",
            Style::default().fg(palette.dim),
        )));
        for patch in &row.messages {
            // The id and message are separate lines, not one — a 64-hex id plus a message routinely
            // exceeds a normal terminal's width, and wrapping one long line can split a message's own
            // text across two rows (caught while testing this exact rendering).
            for wrapped in wrap_verbatim(&identifier(&patch.patch_id), width, "    ") {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.accent),
                )));
            }
            for wrapped in wrap_indented(&inert(&patch.message), width, "      ") {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.fg),
                )));
            }
        }
    }

    lines.push(Line::from(""));

    // The files at this block (RFC 037 decision 1). **The heading says which block**, because from
    // prikk 0.46 this is no longer the tip's file set reached through the tip's name: `tree` answers
    // for the block the user opened, and a heading still saying "at tip" over an older block's files
    // would be stikk mislabelling a correct answer.
    match detail.state.unreported_reason() {
        Some(reason) => {
            // **The row RFC 037's review found spilling to column 0.** Its continuations now sit
            // under the sentence, not under the left edge.
            said(
                &mut lines,
                palette,
                width,
                &format!("state files: {reason}"),
                "  ",
                "               ",
            );
        }
        None => {
            let files = detail.state.listed().map_or(&[][..], |state| &state.files);
            let total = detail.state.listed().map_or(0, |state| state.total_bytes);
            said(
                &mut lines,
                palette,
                width,
                &format!(
                    "state {} — {} file(s), {} byte(s):",
                    if detail.is_tip {
                        "at tip"
                    } else {
                        "at this block"
                    },
                    files.len(),
                    total
                ),
                "  ",
                "    ",
            );
            // **The keys get a row of their own** (RFC 053 Handoff B §2). On one row with the
            // state line they wrapped at 80 columns, splitting *"A saves / all"* across two
            // rows and reading as a broken sentence rather than a hint.
            said(&mut lines, palette, width, SAVE_ACTIONS, "  ", "    ");
            if files.is_empty() {
                lines.push(Line::from(Span::styled(
                    "    (empty state)",
                    Style::default().fg(palette.dim),
                )));
            } else {
                // **The cursor runs on from the patches** (RFC 053 decision 3), so `↓` reaches a
                // file and `F` knows which one. The marker is the one History and the patch list
                // above already use.
                let first_file = detail.patches.rows().len();
                for (at, file) in files.iter().enumerate() {
                    let selected = cursor == first_file + at;
                    let mut spans = vec![
                        Span::styled(
                            format!("  {}", crate::theme::marker(selected)),
                            Style::default().fg(palette.accent),
                        ),
                        // The mode first, in the fixed-width octal Patch detail already shows
                        // (`100644`), so the paths below it line up whatever their length. Not
                        // reported is said, never shown as `0`, which would read as `000000` — a
                        // mode prikk never reported and no file ever has.
                        Span::styled(
                            match file.mode {
                                Some(mode) => format!("{mode:06o}  "),
                                None => "  (no mode)  ".to_string(),
                            },
                            Style::default().fg(palette.dim),
                        ),
                        Span::styled(identifier(&file.path), Style::default().fg(palette.fg)),
                    ];
                    // prikk's own word for what the path is, shown **only when it is not `file`**
                    // (RFC 037 §2). Every entry prikk has ever emitted is a `file`, so labelling
                    // each one would be noise; a kind stikk has not measured is exactly the thing a
                    // reader must not be shown as an ordinary file.
                    if let Some(kind) = file.kind.as_deref()
                        && kind != "file"
                    {
                        spans.push(Span::styled(
                            format!("  ({})", identifier(kind)),
                            Style::default().fg(palette.warn),
                        ));
                    }
                    lines.push(Line::from(spans));
                    let row = lines.len() - 1;
                    file_rows.push((row, row));
                    if selected {
                        // The whole row, to the view's edge (RFC 061 decision 4).
                        if let Some(rows) = lines.get_mut(row..) {
                            crate::view::select_rows(rows, width);
                        }
                    }
                }
            }
        }
    }

    // The block's own patches (RFC 035 §4.1). This replaces the line that said per-patch content
    // "still awaits prikk support (UD-09)" — it was true when written, retired at prikk 0.36, and
    // built here.
    lines.push(Line::from(""));
    match detail.patches.unreported_reason() {
        Some(reason) => {
            // **"Patches not shown: …", in the label colour** (RFC 060 decision 7). It was italic,
            // and *"patches: <why there are none>"* read as a list's label with a strange value.
            for wrapped in wrap_indented(&format!("Patches not shown: {reason}"), width, "  ") {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().fg(palette.dim),
                )));
            }
        }
        None => {
            let rows = detail.patches.rows();
            said(
                &mut lines,
                palette,
                width,
                &format!("patches — {} listed · Enter opens one:", rows.len()),
                "  ",
                "    ",
            );
            for (at, row) in rows.iter().enumerate() {
                // The selected row is marked the way History's own cursor marks one.
                let selected = at == cursor;
                let marker = crate::theme::marker(selected);
                let style = if selected {
                    crate::theme::selected(palette)
                } else {
                    Style::default().fg(palette.fg)
                };
                // Core returns this as pieces (RFC 036 review v1 §2.3): the short id is a name,
                // the message is prose, the count is stikk's. `compose` renders each under its own
                // rule.
                //
                // **Hard-split, not word-wrapped.** The row's own separators are double spaces
                // (`id␣␣message␣␣·␣␣N operation(s)`), and a word wrap rebuilds a line from its
                // whitespace-separated words — which collapsed them and moved the columns. Caught
                // by this section's own test on the first run.
                let summary = compose(&row.summary());
                let room = width.saturating_sub(6).max(1);
                let mut wrapped = wrap_verbatim(&summary, room, "").into_iter();
                let head = wrapped.next().unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(format!("    {marker}"), Style::default()),
                    Span::styled(head, style),
                ]));
                let entry = lines.len() - 1;
                for rest in wrapped {
                    // Under the summary, not under the marker.
                    lines.push(Line::from(Span::styled(format!("      {rest}"), style)));
                }
                patch_rows.push((entry, lines.len() - 1));
                if selected {
                    // Every row of the selected entry, to the view's edge (RFC 061 decision 4).
                    if let Some(rows) = lines.get_mut(entry..) {
                        crate::view::select_rows(rows, width);
                    }
                }
            }
        }
    }

    // **The cursor's order**: patches first, then files, as `block_cursor_row` indexes them.
    let cursor_rows = patch_rows.into_iter().chain(file_rows).collect();
    (lines, cursor_rows)
}

/// The `patches` field's value: the count alone when it agrees with the message list, or the count
/// plus an explanation when it does not (RFC 015 F4/decision 2, the acceptance-critical behaviour) —
/// a patch authored below prikk 0.32 carries no message, which is absence, not an empty one.
fn patches_summary(patches: u64, message_count: usize) -> String {
    let message_count = message_count as u64;
    if patches == message_count {
        patches.to_string()
    } else {
        format!(
            "{patches} · {message_count} with a message — patches written before prikk 0.32 carry \
             none, which is absence, not an empty message"
        )
    }
}

/// The column a field's value starts at: two spaces of margin plus the 15-wide label.
const VALUE_COLUMN: usize = 17;

/// A `label: value` field whose value is an **identifier** — an object id, a ref name, prikk's kind
/// word. Hard-split, never word-wrapped (RFC 036), and continued **at the value's own column** so a
/// reader can see which field a continuation belongs to.
fn field_id<'a>(
    lines: &mut Vec<Line<'a>>,
    palette: &Palette,
    width: usize,
    label: &str,
    value: &str,
) {
    field_rows(lines, palette, width, label, &identifier(value), false);
}

/// A `label: value` field whose value is **stikk's own words** — a count, a sentence. Word-wrapped,
/// and continued at the value's own column.
fn field_said<'a>(
    lines: &mut Vec<Line<'a>>,
    palette: &Palette,
    width: usize,
    label: &str,
    value: &str,
) {
    field_rows(lines, palette, width, label, value, true);
}

fn field_rows<'a>(
    lines: &mut Vec<Line<'a>>,
    palette: &Palette,
    width: usize,
    label: &str,
    value: &str,
    prose: bool,
) {
    let room = width.saturating_sub(VALUE_COLUMN).max(1);
    let rows = if prose {
        wrap_indented(value, room, "")
    } else {
        wrap_verbatim(value, room, "")
    };
    let mut rows = rows.into_iter();
    let head = rows.next().unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled(format!("  {label:<15}"), Style::default().fg(palette.dim)),
        Span::styled(head, Style::default().fg(palette.fg)),
    ]));
    for rest in rows {
        lines.push(Line::from(vec![
            Span::styled(" ".repeat(VALUE_COLUMN), Style::default()),
            Span::styled(rest, Style::default().fg(palette.fg)),
        ]));
    }
}

/// One of stikk's own sentences, word-wrapped and styled: the first row at `indent`, the rest at
/// `hang`.
///
/// **The wrapping is [`crate::view::wrap_hanging`]'s**, not this module's. Giving the opening row
/// and the hanging rows each the room its own indent leaves used to be written out here, and the
/// same reasoning had been written twice more elsewhere -- which is how it came to be a shared
/// helper (RFC 040) after the third copy. What is left here is the part that really is this
/// view's: the style the rows are drawn in.
/// **What the state list can do**, on its own row beneath it — named as actions, not as keys (RFC 061
/// decision 2, and the sweep in its §8.1 that this line escaped: it said `F` and `A`, both of which
/// were withdrawn, so a reader who followed it pressed two letters that do nothing).
const SAVE_ACTIONS: &str = "Enter on a file saves it, or saves every file at this block.";

fn said<'a>(
    lines: &mut Vec<Line<'a>>,
    palette: &Palette,
    width: usize,
    text: &str,
    indent: &str,
    hang: &str,
) {
    for row in crate::view::wrap_hanging(text, width, indent, hang) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
}

#[cfg(test)]
mod tests;

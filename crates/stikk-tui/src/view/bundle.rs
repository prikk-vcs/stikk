//! The Bundle view: what one bundle file **is**, and what importing it would **change**
//! (`FR-091`; RFC 050 decision 3).
//!
//! # What this view is careful to say
//!
//! - **prikk's verification is prikk's, word for word** — including the caveat that the check is
//!   structural only and *"a verified bundle is not yet a trusted one"*. That sentence is why
//!   the view shows the verification at all rather than a green tick.
//! - **"Sealed by" is never a trust claim.** prikk's key ids are drawn with prikk's own
//!   continuity note beside them, because the ids alone read as an assurance nobody gave.
//! - **The sentence is stikk's and the names are prikk's**, drawn in different spans: *"your
//!   branch"* is stikk's chrome, `heads/main` is repository text through `identifier`, and a
//!   path is repository text through `inert` (`C-T2a`, RFC 036).
//! - **Nothing here acts.** Importing is RFC 050's Handoff B, so the view advertises no key
//!   that would do nothing.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use stikk_core::{BUNDLE_EFFECTS_SHOWN, BundleOpened, BundleView, unshown_effects_line};
use stikk_prikk::BundleEffect;

use crate::text::{identifier, inert, wrap_indented, wrap_verbatim};
use crate::theme::Palette;

/// Indents: the heading rows, and prikk's own lines and the lists under them.
const TOP: &str = "  ";
const QUOTED: &str = "    ";

/// Render `opened` into `area`, scrolled to `offset` (clamped here and written back).
pub fn render(
    opened: &BundleOpened,
    offset: &std::cell::Cell<u16>,
    palette: &Palette,
    frame: &mut Frame,
    area: Rect,
) {
    let width = usize::from(area.width.saturating_sub(3));
    let lines = match opened {
        BundleOpened::Read(view) => read_lines(view, palette, width),
        BundleOpened::Refused { path, message } => refused_lines(path, message, palette, width),
        BundleOpened::Unavailable(message) => stikk_said(message, palette, width),
    };

    let viewport = area.height.saturating_sub(2);
    let max_offset = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_sub(viewport);
    let scroll = offset.get().min(max_offset);
    offset.set(scroll);

    // **"Bundle"**, not "Import": nothing here imports, and a title that implied one would be
    // the view promising what the keys do not do.
    let mut title = " Bundle \u{00b7} read only ".to_string();
    if max_offset > 0 {
        let first = usize::from(scroll) + 1;
        let last = (usize::from(scroll) + usize::from(viewport)).min(lines.len());
        title.push_str(&format!(
            "\u{00b7} \u{2191}/\u{2193} \u{00b7} lines {first}\u{2013}{last} of {} ",
            lines.len()
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(palette.fg));
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

/// A band: **stikk's own sentence**, and nothing else. There is no file to name beyond the one
/// the reader just typed and no prikk to quote, because nothing was spawned.
fn stikk_said<'a>(message: &str, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for row in wrap_indented(message, width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    lines
}

/// A file prikk would not read: **the path, a label naming prikk, and prikk's words** — and no
/// preview, because `open_bundle` never asked for one.
///
/// **The path is here because prikk's refusal often is not** (review v1 C1): *"malformed
/// persisted data: …"* names no file, so without this a mistyped path and a damaged download
/// read the same. **The label is here because the words are prikk's**: the read screen puts
/// prikk's lines under *"prikk's verification, as prikk printed it:"*, and bare prose in the
/// same panel would read as stikk's own.
fn refused_lines<'a>(path: &str, message: &str, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    for row in wrap_verbatim(&inert(path), width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(""));
    lines.push(dim("prikk could not read it:", palette, width));
    // **Wrapped as prikk's lines are on the read screen**, not as stikk's prose: hard-wrapped
    // at the quoted indent, so a long refusal keeps the shape prikk gave it (RFC 024).
    for line in message.lines() {
        if line.trim().is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        for row in wrap_verbatim(&inert(line), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.warn),
            )));
        }
    }
    lines
}

/// Every row, already wrapped to `width`, so the caller's scroll clamp is exact (RFC 023 F2).
fn read_lines<'a>(view: &BundleView, palette: &Palette, width: usize) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // The path, resolved, as the reader must be able to check it: it is what prikk read.
    for row in wrap_verbatim(&inert(&view.path), width, TOP) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // ---- prikk's verification, verbatim, under prikk's own bar.
    lines.push(Line::from(""));
    lines.push(dim(
        "prikk's verification, as prikk printed it:",
        palette,
        width,
    ));
    for line in &view.verified {
        if line.trim().is_empty() {
            lines.push(Line::from(""));
            continue;
        }
        for row in wrap_verbatim(&inert(line), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.fg),
            )));
        }
    }

    // ---- what it would change, against which branch.
    lines.push(Line::from(""));
    // **The bundle's own ref, alone.** The local side used to be here too, as
    // `<bundle> -> your <local>`; the label below now says it, and saying it twice in two rows
    // made the sentence under them read as a statement about both (review v1 C1).
    let heading = format!(
        "This bundle carries {}",
        identifier(&view.preview.bundle_ref)
    );
    for row in crate::view::wrap_hanging(&heading, width, TOP, "      ") {
        lines.push(Line::from(Span::styled(
            row,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }
    // **The same label the import card carries** (review v1 C1): beside an advertised `I`, a
    // sentence like *"it would bring it forward"* reads as what the key does. It is what prikk's
    // preview measured about a branch an import leaves exactly as it is.
    for row in crate::view::wrap_hanging(
        &format!(
            "{} {} {}",
            stikk_core::COMPARED_WITH_BEFORE,
            identifier(&view.preview.local_ref),
            stikk_core::COMPARED_WITH_AFTER
        ),
        width,
        TOP,
        "    ",
    ) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.dim),
        )));
    }
    for row in wrap_indented(view.sentence, width, QUOTED) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.fg),
        )));
    }
    // **prikk's reason, verbatim, under stikk's sentence** (`ER-02`) -- why it conflicts is
    // prikk's to say, and stikk's sentence above says only that it does.
    if let Some(detail) = view.conflict_detail() {
        lines.push(Line::from(""));
        for row in wrap_verbatim(&inert(detail), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.warn),
            )));
        }
    }

    // ---- the effects.
    lines.push(Line::from(""));
    let total = view.preview.effects.len();
    lines.push(dim(
        &match total {
            0 => "No files would change.".to_string(),
            1 => "One file would change:".to_string(),
            n => format!("{n} files would change:"),
        },
        palette,
        width,
    ));
    for effect in view.preview.effects.iter().take(BUNDLE_EFFECTS_SHOWN) {
        for row in wrap_verbatim(&effect_row(effect), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.fg),
            )));
        }
    }
    // **Never a silent cut** (§4c): what is not drawn is counted, with the route to the rest.
    if total > BUNDLE_EFFECTS_SHOWN {
        for row in wrap_indented(
            &unshown_effects_line(total - BUNDLE_EFFECTS_SHOWN, total),
            width,
            QUOTED,
        ) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.dim),
            )));
        }
    }
    if total > 0 {
        // prikk's rename note, verbatim -- a delete and a create in this list may be one move,
        // and prikk says so itself rather than stikk paraphrasing the limit.
        for row in wrap_indented(&inert(&view.preview.rename_note), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.dim),
            )));
        }
    }

    // ---- what can be done here, and only where it can.
    //
    // **The view advertises `I` exactly when the key would act** (RFC 050 Handoff B §3b): the
    // same `received_branch()` answers both, so a bundle whose tip line stikk could not read
    // shows no key rather than a key whose preview would only block.
    lines.push(Line::from(""));
    if let Some(received) = view.received_branch() {
        for row in crate::view::wrap_hanging(
            &format!(
                "I imports this as {}, after a confirmation.",
                identifier(&received)
            ),
            width,
            TOP,
            "    ",
        ) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.accent),
            )));
        }
    } else {
        // **Why there is no key**, rather than its silent absence: prikk's verification is on
        // screen above, and a reader comparing the two should not have to infer the link.
        for row in wrap_indented(
            "stikk could not read a tip block from prikk's verification above, so it cannot              tell whether this bundle is older than work you have already received — and does              not offer to import it.",
            width,
            TOP,
        ) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.warn),
            )));
        }
    }

    // ---- who sealed it, and prikk's note that this is not trust.
    lines.push(Line::from(""));
    if view.preview.sealed_by.is_empty() {
        lines.push(dim("No author key material is recorded.", palette, width));
    } else {
        let ids: Vec<String> = view
            .preview
            .sealed_by
            .iter()
            .map(|id| identifier(id))
            .collect();
        for row in
            crate::view::wrap_hanging(&format!("Sealed by {}", ids.join(", ")), width, TOP, "    ")
        {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.fg),
            )));
        }
        // **prikk's own words, and they are the point of the row.** The ids above read as an
        // assurance without this; prikk said it, so prikk says it here.
        for row in wrap_indented(&inert(&view.preview.sealed_by_note), width, QUOTED) {
            lines.push(Line::from(Span::styled(
                row,
                Style::default().fg(palette.dim),
            )));
        }
    }

    lines
}

/// One effect: prikk's kind, the path, and the sizes it reported.
///
/// **The path is repository text and the rest is stikk's chrome**, so the row is built with the
/// path already neutralized and never reflowed afterwards.
fn effect_row(effect: &BundleEffect) -> String {
    let sizes = match (effect.current_bytes, effect.after_bytes) {
        (Some(before), Some(after)) => format!(" ({before} \u{2192} {after} bytes)"),
        (None, Some(after)) => format!(" ({after} bytes)"),
        (Some(before), None) => format!(" (was {before} bytes)"),
        (None, None) => String::new(),
    };
    format!("{:<19}{}{sizes}", effect.kind.name(), inert(&effect.path))
}

fn dim<'a>(text: &str, palette: &Palette, width: usize) -> Line<'a> {
    let row = wrap_indented(text, width, TOP)
        .into_iter()
        .next()
        .unwrap_or_default();
    Line::from(Span::styled(row, Style::default().fg(palette.dim)))
}

#[cfg(test)]
mod tests;

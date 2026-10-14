//! Compare: what changed between two points, in prikk's own hunks (design `FR-033`; RFC 038).
//!
//! **prikk computes the difference and its hunks; this module shapes them for a frontend to draw,
//! and computes no part of the comparison** (RFC 038 §7 trap 1). Nothing here renumbers a hunk,
//! re-hunks a file, infers a rename from a delete beside an add, or decides that two entries "look
//! the same". Every one of those would be stikk asserting a difference prikk did not report
//! (`T-T4`).
//!
//! # What this module does decide
//!
//! - **How a hunk becomes rows.** prikk gives each hunk as one string of unified-diff text. A
//!   frontend cannot wrap that safely without knowing which part is prikk's `@@` header, which is a
//!   marker, and which is content that must survive byte for byte. So it is split here, once:
//!   `split_inclusive('\n')`, the first line is the header, and each body line is its first
//!   character as a marker plus the rest as content.
//! - **Content is quoted and escaped** with RFC 035 A3's one table (`stikk_prikk::escape`), **with
//!   the line's terminator inside the quotes**. A diff is where whitespace edits live — a
//!   re-indent, a tab for spaces, a stripped trailing space, a CRLF — and a renderer that collapsed
//!   or dropped them would show the most common kind of edit as *no edit at all*. prikk's own `@@`
//!   header and its `\ No newline at end of file` line are **prikk's words, not content**, so they
//!   are not quoted (§7 trap 4).
//! - **The bound.** One entry's hunks can be the whole file: a 20 000-line reversal measures at
//!   457 KB in a single hunk. [`crate::CONTENT_DISPLAY_BYTES`] bounds each entry, and the withheld
//!   byte count is said out loud, never silently cut.
//! - **Structure, not composed text** (RFC 036 decision 8): a status word and a path come back as
//!   [`Piece`]s, so the frontend renders each under its own rule.

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{
    Comparison, Diff, DiffEntry, DiffFile, DiffReport, DiffSide, DiffTarget, Prikk, QueuedOnSide,
    UnsupportedPath,
};

use crate::patch::CONTENT_DISPLAY_BYTES;
use crate::piece::Piece;

/// The first prikk minor that compares, re-exported so a view says the same number the seam gates
/// on — the one-constant rule [`crate::CONTENT_FROM_MINOR`] follows.
pub use stikk_prikk::DIFF_FROM_MINOR as COMPARE_FROM_MINOR;

/// One comparison, ready to render (RFC 038 decision 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareView {
    /// prikk compared the two points.
    Listed(Compared),
    /// **Below prikk 0.46**, carrying that prikk's minor. stikk's own voice (`C-T4d`): it compares
    /// from 0.46 on. **Never an empty comparison**, which would say the two points are the same.
    Unreported(u32),
}

impl CompareView {
    /// The comparison, or `None` where there is none to show.
    #[must_use]
    pub fn listed(&self) -> Option<&Compared> {
        match self {
            Self::Listed(compared) => Some(compared),
            Self::Unreported(_) => None,
        }
    }

    /// The sentence to show instead of a comparison, or `None` when there is one.
    #[must_use]
    pub fn unreported_reason(&self) -> Option<String> {
        match self {
            Self::Listed(_) => None,
            Self::Unreported(minor) => Some(format!(
                "stikk compares from prikk 0.{COMPARE_FROM_MINOR} on; this is 0.{minor}."
            )),
        }
    }
}

/// A comparison prikk reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compared {
    /// The left side.
    pub from: ComparedSide,
    /// The right side.
    pub to: ComparedSide,
    /// One section per differing path, in prikk's order.
    pub entries: Vec<ComparedEntry>,
    /// Paths `commit` would refuse, with prikk's refusal verbatim (worktree form only).
    pub unsupported: Vec<UnsupportedPath>,
}

impl Compared {
    /// **True only when prikk listed no entries *and* no unsupported paths** (Handoff A §3). A
    /// comparison with an unsupported path is not "no differences": prikk could not look at that
    /// path, which is a different fact from having looked and found nothing (`C-T2c′`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.unsupported.is_empty()
    }

    /// The heading, as pieces: both sides, left first.
    #[must_use]
    pub fn heading(&self) -> Vec<Piece> {
        let mut pieces = self.from.label();
        pieces.push(Piece::said("  \u{2192}  "));
        pieces.extend(self.to.label());
        pieces
    }
}

/// One side of a comparison, as the heading names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparedSide {
    /// `point`, as it was given: a block id, a ref name, or the word `worktree`.
    pub point: String,
    /// What it resolved to.
    pub target: DiffTarget,
    /// The queue folded into this side.
    pub queued: QueuedOnSide,
}

impl ComparedSide {
    /// How to name this side (RFC 038 decision 5): a short block id, a ref as an identifier, or
    /// *the worktree* — and *the tip plus N queued commits not yet sealed* where prikk reports a
    /// queue, because that state has no block id to name it by.
    #[must_use]
    pub fn label(&self) -> Vec<Piece> {
        if matches!(self.target, DiffTarget::Worktree) {
            return vec![Piece::said("the worktree")];
        }
        // The point is what the caller asked with. A block id is shortened the way every other
        // view shortens one; a ref name is a name the repository chose, so it stays whole.
        let named = if is_object_id(&self.point) {
            Piece::name(short_id(&self.point))
        } else {
            Piece::name(self.point.clone())
        };
        match self.queued {
            QueuedOnSide::Queued(0) | QueuedOnSide::NotApplicable => vec![named],
            QueuedOnSide::Queued(n) => vec![
                named,
                Piece::said(format!(" plus {n} queued commit(s) not yet sealed")),
            ],
        }
    }
}

/// One differing path, as a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparedEntry {
    /// prikk's status word, verbatim.
    pub status: String,
    /// The section's title: the status word and the path(s), each as the kind of thing it is.
    pub title: Vec<Piece>,
    /// Facts about the two sides that are not hunks: a `mode` entry's two modes, a `binary`
    /// entry's sizes and content ids. Each is one line, already in the order to show them.
    pub facts: Vec<Vec<Piece>>,
    /// prikk's note where the line diff did not reach the shortest edit script.
    pub minimal_note: Option<String>,
    /// The hunks, as rows.
    pub rows: Vec<HunkRow>,
    /// prikk's own fields for this entry, for the raw view (`v`).
    pub raw: String,
}

/// One row of a rendered hunk (RFC 038 decision 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkRow {
    /// prikk's `@@ -1,3 +1,3 @@` line, **as prikk wrote it** — prose, not content, so it is not
    /// quoted (§7 trap 4).
    Header(String),
    /// One body line: its marker, and its content **quoted** with the line's own terminator inside
    /// the quotes. The marker is not content and is not inside the quotes.
    Line {
        /// `-` removed, `+` added, ` ` context — prikk's first character, never stikk's guess.
        marker: char,
        /// The rest of the line, quoted and escaped.
        content: String,
    },
    /// prikk's `\ No newline at end of file`, in prikk's words (`ER-02`).
    Note(String),
    /// What [`crate::CONTENT_DISPLAY_BYTES`] withheld from this entry, said out loud.
    Withheld(String),
}

/// Whether `value` looks like one of prikk's object ids (64 lowercase hex).
fn is_object_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn short_id(id: &str) -> String {
    id.chars().take(crate::SHORT_ID_CHARS).collect()
}

/// Read a comparison and shape it for a frontend.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises. **A refusal is propagated, not
/// carried** (RFC 038 decision 6): unlike Block detail's file list, a Compare screen *is* the
/// comparison, so there is nothing left to show beside the failure — it goes through the usual
/// refusal path with prikk's words.
pub fn compare(prikk: &impl Prikk, repo: &Path, comparison: &Comparison) -> Result<CompareView> {
    match prikk.diff(repo, comparison)? {
        DiffReport::Unreported => Ok(CompareView::Unreported(prikk.handshake()?.version.minor)),
        DiffReport::Listed(diff) => Ok(CompareView::Listed(compared(diff))),
    }
}

/// Shape one report. Separate from [`compare`] so a test can drive it without a backend.
#[must_use]
pub fn compared(diff: Diff) -> Compared {
    let Diff {
        from,
        to,
        entries,
        unsupported_paths,
    } = diff;
    Compared {
        from: side(from),
        to: side(to),
        entries: entries.into_iter().map(entry).collect(),
        unsupported: unsupported_paths,
    }
}

fn side(side: DiffSide) -> ComparedSide {
    ComparedSide {
        point: side.point,
        target: side.target,
        queued: side.queued,
    }
}

fn entry(entry: DiffEntry) -> ComparedEntry {
    let DiffEntry {
        path,
        status,
        from_path,
        from,
        to,
        minimal,
        hunks,
        raw,
    } = entry;

    // The status word is prikk's; the path is a name the repository chose. Both come back as
    // pieces so the frontend renders each under its own rule (RFC 036).
    let mut title = vec![Piece::name(status.clone()), Piece::said("  ")];
    match &from_path {
        Some(old) => {
            title.push(Piece::name(old.clone()));
            title.push(Piece::said("  \u{2192}  "));
            title.push(Piece::name(path.clone()));
        }
        None => title.push(Piece::name(path.clone())),
    }

    ComparedEntry {
        facts: facts(&status, from.as_ref(), to.as_ref()),
        // prikk's own words for what `minimal: false` means, as prikk's guide puts it.
        minimal_note: (!minimal).then(|| {
            "not the shortest edit script \u{2014} the hunks are larger than they need to be, and \
             still correct"
                .to_string()
        }),
        rows: rows(&hunks),
        status,
        title,
        raw,
    }
}

/// The non-hunk facts for an entry, by status. A `mode` entry has no hunks at all, and a `binary`
/// entry's content is never shown — so for those two, this is the whole section.
fn facts(status: &str, from: Option<&DiffFile>, to: Option<&DiffFile>) -> Vec<Vec<Piece>> {
    let mut out = Vec::new();
    match status {
        "mode" => {
            if let (Some(from), Some(to)) = (from, to) {
                out.push(vec![Piece::said(format!(
                    "mode {:06o} \u{2192} {:06o}",
                    from.mode, to.mode
                ))]);
            }
        }
        "binary" => {
            for (which, file) in [("from", from), ("to", to)] {
                if let Some(file) = file {
                    let mut line = vec![Piece::said(format!("{which}  {} byte(s)", file.size))];
                    // **The blob's identity, never its bytes** (`FR-030`).
                    if let Some(id) = &file.content_id {
                        line.push(Piece::said("  \u{b7}  "));
                        line.push(Piece::name(short_id(id)));
                    }
                    out.push(line);
                }
            }
        }
        _ => {
            // A mode change that rides along with a content change is still worth saying: prikk
            // reports both sides for every status that has them.
            if let (Some(from), Some(to)) = (from, to)
                && from.mode != to.mode
            {
                out.push(vec![Piece::said(format!(
                    "mode {:06o} \u{2192} {:06o}",
                    from.mode, to.mode
                ))]);
            }
        }
    }
    out
}

/// prikk's hunks as rows, bounded by [`crate::CONTENT_DISPLAY_BYTES`] across the whole entry.
///
/// **The split is the only transformation.** `split_inclusive('\n')` keeps each line's terminator
/// with it, so a missing final newline is a line without one — visible, because the terminator
/// lives inside the quotes.
fn rows(hunks: &[String]) -> Vec<HunkRow> {
    const NO_NEWLINE: &str = "\\ No newline at end of file";
    let mut out = Vec::new();
    let mut spent = 0usize;
    let mut withheld = 0usize;
    for hunk in hunks {
        for (at, line) in hunk.split_inclusive('\n').enumerate() {
            let trimmed = line.strip_suffix('\n').unwrap_or(line);
            if withheld > 0 {
                withheld += line.len();
                continue;
            }
            if spent + line.len() > CONTENT_DISPLAY_BYTES {
                withheld += line.len();
                continue;
            }
            spent += line.len();
            // The first line of a hunk is prikk's header; prikk's no-newline note can appear
            // anywhere after it. Neither is content, so neither is quoted (§7 trap 4).
            if at == 0 {
                out.push(HunkRow::Header(trimmed.to_string()));
            } else if trimmed == NO_NEWLINE {
                out.push(HunkRow::Note(trimmed.to_string()));
            } else {
                let mut chars = line.chars();
                // **prikk's first character is the marker.** An empty line in a unified diff still
                // carries its space; treating a genuinely empty line as context is the only
                // reading that does not invent one.
                let marker = chars.next().unwrap_or(' ');
                let content = chars.as_str();
                out.push(HunkRow::Line {
                    marker,
                    content: stikk_prikk::escape::quoted(content),
                });
            }
        }
    }
    if withheld > 0 {
        out.push(HunkRow::Withheld(format!(
            "{withheld} more bytes not shown"
        )));
    }
    out
}

#[cfg(test)]
mod tests;

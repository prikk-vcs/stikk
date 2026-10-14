//! Patch detail: what one patch changed, in prikk's own terms (design `FR-030`; RFC 035 decisions 2,
//! 3, 3a, 5 and 7).
//!
//! **`FR-030` has been unbuilt since 0.1.0** — first on `UD-09`, whose content half retired at prikk
//! 0.36, then on its own increment. prikk's `show --format json` says, per operation, exactly what it
//! asks for, and this module turns that report into the words both frontends show.
//!
//! **Every word here is stikk's and every value is prikk's** (`ER-02`). The kind is prikk's text,
//! verbatim; the spans, modes, blob ids, sizes and key ids are prikk's values, unrounded and
//! unsummarised; and where prikk resolves no path, stikk says exactly that and **never guesses which
//! file it is** (decision 3) — not from `diff`, not from the Changes view, not from a later patch's
//! rename in the same report.
//!
//! # Three rules this module exists to hold
//!
//! - **An unresolved path gets one sentence, true at every version, with no reason attached.** stikk
//!   cannot know which of three reasons applies (RFC 035 A1 F7): a bare sealed patch id resolves
//!   nothing at any measured version, a queued patch resolved nothing below prikk 0.47, and a sealed
//!   block resolves against its own end — where a file a later patch deleted has no path left to name.
//! - **In a block of more than one patch, a node-addressed path is labelled as prikk's resolution
//!   point** (decision 3a): *as at the end of this block*. The count comes from the block's own report.
//!   stikk **never** corrects the name from a later rename it can see in that same report — rendering
//!   prikk's value and labelling it truthfully is the honest option, and inventing the patch-time name
//!   would be stikk asserting a state prikk did not report (`T-T4`).
//! - **Content is bounded before it is rendered, and the cut is said out loud** (decision 7,
//!   `C-T2c′`). A create or a preimage carries whole file content. [`CONTENT_DISPLAY_BYTES`] is the one
//!   bound, in one place, and a silent truncation is the defect the rule exists to prevent.
//! - **Content is rendered byte-faithfully: two spans that differ in any byte never render the same**
//!   (review v1 §2.1). Spans are where whitespace edits live — a re-indent, a tab turned into spaces, a
//!   trailing newline removed — so a renderer that collapsed whitespace would show the most common kind
//!   of edit as *no edit at all*, which is `T-T4`'s confident-but-wrong picture in its purest form.
//!   Every content row is therefore **quoted and escaped**, one row per source line, with the line's own
//!   terminator inside the quotes: `"  a\n"`. The escaping is the same one prikk's JSON uses, so the
//!   rendered view and the raw view spell content the same way, and it is injective — the quotes delimit,
//!   and `\t`, `\n`, `\r`, `\"`, `\\` and `\u00XX` are each one byte spelled one way. No marker is
//!   invented, so there is no new vocabulary for the Glossary to carry.
//!
//!   Rows carry which kind they are ([`OperationRow`]), because only the frontend knows the width to
//!   wrap at, and a content row must be hard-split rather than word-wrapped.

use std::path::Path;

use crate::piece::Piece;
use stikk_model::Result;
use stikk_prikk::{
    BinaryBlob, FileContent, OperationContent, PatchOperation, Prikk, QueuedMessage, QueuedPath,
    ShowReport, ShownPatch,
};

/// The most bytes of any one content field Patch detail renders — a span, a created file, a preimage,
/// or prikk's raw JSON for one operation (RFC 035 decision 7).
///
/// **One constant, bounding every content field shown.** A preimage can be a whole file, and a view
/// that rendered it would be a view that hung. 4 KiB is roughly fifty terminal rows: enough that an
/// ordinary edit, create or delete is shown whole, and small enough that a large one cannot flood the
/// screen before the reader can read anything.
///
/// **The bound is stikk's own display limit, and it is stated where it bites** — never a silent cut.
/// Where it applies, the view says *N more bytes not shown*, counting the UTF-8 bytes withheld.
pub const CONTENT_DISPLAY_BYTES: usize = 4096;

/// The sentence for a path prikk did not resolve (RFC 035 decision 3, as amended).
///
/// **Exactly this, at every version, with no reason.** The old wording named one — *"for a patch with
/// no block yet"* — and A1 F7 withdrew it, because the same value arrives in a sealed block too.
pub const NO_PATH_REPORTED: &str = "prikk names no path for this operation";

/// The label a node-addressed path carries in a block of more than one patch (RFC 035 decision 3a).
pub const AS_AT_BLOCK_END: &str = "as at the end of this block";

/// The first prikk minor whose patch content stikk reads — **stikk's JSON floor**, not
/// `show-report-v1`'s own (RFC 035 A1, decision 1 as amended): the schema exists from prikk 0.36, but
/// stikk reads no JSON from prikk at all between 0.36 and 0.38.
///
/// **Re-exported, not re-declared** (review v1 §2.2). It was a second `39` here until the review
/// caught it, which is exactly how RFC 015's stale `0.30` survived: two copies that agree are two
/// copies, and only one of them moves.
pub use stikk_prikk::JSON_REPORTS_FROM_MINOR as CONTENT_FROM_MINOR;

/// One patch's detail, as rows in stikk's words (RFC 035 §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDetailView {
    /// The patch id, in full.
    pub patch_id: String,
    /// The rows under it: prikk's `queued`/sealed word with the block it is in, then the message.
    pub heading: Vec<String>,
    /// Each operation, in prikk's order. **Empty only when prikk listed none, or when
    /// [`Self::unreported`] is set** — never as a stand-in for either (`C-T2c′`).
    pub operations: Vec<OperationView>,
    /// Set when this prikk does not report a patch's content at all: stikk asked nothing, so there is
    /// nothing to list. [`Self::foot`] carries the reason, in stikk's own voice.
    pub unreported: bool,
    /// The one line at the foot, when one applies.
    pub foot: Option<String>,
}

/// One row of an operation's detail, and **which kind of text it is** (review v1 §2.1).
///
/// The distinction is the frontend's to honour: stikk's own words wrap on whitespace like any prose,
/// and repository content must not — a content row is hard-split at the width, with every space,
/// tab and newline kept exactly as prikk reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationRow {
    /// **stikk's own words**: a label, a path, a sentence, an id. Wraps like prose.
    Said(String),
    /// **Repository content**, already quoted and escaped, one row per source line, the line's own
    /// terminator inside the quotes. Rendered verbatim: never re-wrapped on whitespace, never trimmed.
    Content(String),
    /// **A row that mixes stikk's words with a name the repository chose** (RFC 036 review v1 §2.3)
    /// — a path, or prikk's node id. The frontend renders each [`Piece`] under its own rule and
    /// joins them; it wraps like prose once composed.
    Named(Vec<Piece>),
}

impl OperationRow {
    /// The row's text where it is one string. **`None` for a [`Self::Named`] row**, whose pieces
    /// must be rendered separately — a caller that flattens one has lost the distinction the row
    /// exists to carry.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Said(text) | Self::Content(text) => Some(text),
            Self::Named(_) => None,
        }
    }

    /// Whether this row is repository content, and so must be rendered verbatim.
    #[must_use]
    pub fn is_content(&self) -> bool {
        matches!(self, Self::Content(_))
    }
}

/// One operation of a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationView {
    /// The row naming the operation: **prikk's kind word, verbatim**, and its path where prikk gives
    /// one — with decision 3a's label where it applies. **As pieces, not as one string** (RFC 036
    /// review v1 §2.3): the kind word and each path are names the repository chose, and the
    /// frontend renders them under its identifier rule.
    pub title: Vec<Piece>,
    /// stikk's rendering of what the operation did, as rows: spans, content, modes, blobs, the key
    /// that asserted a rename, or the one sentence for a path prikk did not resolve.
    pub rows: Vec<OperationRow>,
    /// **prikk's own fields for this operation**, as rows, for the raw view (`v`; decision 5): node
    /// ids, blob ids, modes as prikk's integers, spans as text. Bounded by the same
    /// [`CONTENT_DISPLAY_BYTES`], because `v` must not be the way around a bound that exists to keep a
    /// whole file off the screen.
    pub raw: Vec<String>,
}

/// A patch's message, in the three states that never collapse into each other (`C-T2c′`; RFC 015 F4,
/// RFC 028 decision 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchMessageState {
    /// prikk reported a message. Repository content: the frontend renders it inert (`C-T2a`).
    Text(String),
    /// prikk reported that the patch carries none — a queued `message: null`.
    Absent,
    /// This prikk does not report it: no entry in the block's `patch_messages` (a patch authored below
    /// prikk 0.32 carries no message at all), or a queued patch below prikk 0.42. **Never "no
    /// message"**.
    NotReported,
}

impl PatchMessageState {
    /// The queued form's three states, as prikk reports them (RFC 028 decision 2).
    #[must_use]
    pub fn from_queued(message: &QueuedMessage) -> Self {
        match message {
            QueuedMessage::NotReported => Self::NotReported,
            QueuedMessage::None => Self::Absent,
            QueuedMessage::Text(text) => Self::Text(text.clone()),
        }
    }

    /// The sealed form: a block's `patch_messages` entry, joined **by id**, or its absence.
    #[must_use]
    pub fn from_block_join(message: Option<&str>) -> Self {
        match message {
            Some(text) => Self::Text(text.to_string()),
            None => Self::NotReported,
        }
    }

    fn row(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Absent => "(no message)".to_string(),
            Self::NotReported => "(no message reported)".to_string(),
        }
    }
}

/// Read and render one **queued** patch's detail — the Queue view's drill-in (RFC 035 decision 6).
///
/// **One `show <patch-id>` read, and nothing else.** A queued patch has no block to ask about (F2), so
/// it is never labelled with decision 3a's block-end label and never names a block.
///
/// `message` is the one the Queue already read, passed in rather than re-read: prikk reports a queued
/// patch's message in `status`, not in `show`.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn patch_detail(
    prikk: &impl Prikk,
    repo: &Path,
    patch_id: &str,
    message: &PatchMessageState,
) -> Result<PatchDetailView> {
    let prikk_minor = prikk.handshake()?.version.minor;
    match prikk.show(repo, patch_id)? {
        ShowReport::Unreported => Ok(unreported(patch_id, prikk_minor)),
        ShowReport::Listed(patches) => {
            // The report is asked about one patch id, and answers with that patch. Matched by id
            // rather than taken by position: if prikk ever answers a patch id with more than the patch
            // asked for, stikk shows the one the user opened, not whichever came first.
            let Some(patch) = patches.iter().find(|patch| patch.patch_id == patch_id) else {
                return Ok(PatchDetailView {
                    patch_id: patch_id.to_string(),
                    heading: Vec::new(),
                    operations: Vec::new(),
                    unreported: false,
                    foot: Some(format!(
                        "prikk's report for this id does not list it ({} patch(es) listed).",
                        patches.len()
                    )),
                });
            };
            Ok(render(patch, None, message, 1))
        }
    }
}

/// Render one patch of a block's own `show <block-id>` report — Block detail's drill-in (RFC 035
/// decision 6, narrowed).
///
/// **This spawns nothing** (§5 trap 7): Block detail has already read the block's report, and Patch
/// detail renders one patch from it.
///
/// `patches_in_block` is the number of patches in **that report**, which is what decides decision 3a's
/// label — not whether a later patch renames anything (§5 trap 3).
#[must_use]
pub fn patch_detail_in_block(
    patch: &ShownPatch,
    block_id: &str,
    message: &PatchMessageState,
    patches_in_block: usize,
) -> PatchDetailView {
    render(patch, Some(block_id), message, patches_in_block)
}

/// The view below stikk's JSON floor: stikk asked prikk nothing, and says so in **its own** voice
/// (`C-T4d`).
///
/// **Not *"prikk does not report it"***, which would be false at prikk 0.36–0.38: `show-report-v1`
/// exists there and stikk does not read it (RFC 035 A1, decision 1 as amended).
fn unreported(patch_id: &str, prikk_minor: u32) -> PatchDetailView {
    PatchDetailView {
        patch_id: patch_id.to_string(),
        heading: Vec::new(),
        operations: Vec::new(),
        unreported: true,
        foot: Some(format!(
            "stikk reads a patch's content from prikk 0.{CONTENT_FROM_MINOR} on; this is \
             0.{prikk_minor}."
        )),
    }
}

fn render(
    patch: &ShownPatch,
    block_id: Option<&str>,
    message: &PatchMessageState,
    patches_in_block: usize,
) -> PatchDetailView {
    // prikk's own word for where the patch is, never an inference from the id stikk asked with
    // (§5 trap 5).
    let state = if patch.queued { "queued" } else { "sealed" };
    let mut heading = vec![match (patch.queued, block_id) {
        (false, Some(block)) => format!("{state} · in block {block}"),
        _ => state.to_string(),
    }];
    heading.push(message.row());
    // Decision 3a: the label is about the block's patch count, and never applies to a queued patch or
    // to a block of one — where the block's end *is* that patch's end, and the path is simply right.
    let label_paths = !patch.queued && patches_in_block > 1;
    PatchDetailView {
        patch_id: patch.patch_id.clone(),
        heading,
        operations: patch
            .operations
            .iter()
            .map(|operation| operation_view(operation, label_paths))
            .collect(),
        unreported: false,
        foot: None,
    }
}

/// Whether decision 3a's label applies to this kind: the three node-addressed ones. The other three
/// carry their own paths in the payload (RFC 035 F0), so there is nothing to resolve and nothing to
/// label.
fn node_addressed(kind: &str) -> bool {
    matches!(kind, "edit-text" | "replace-binary" | "change-perm")
}

fn operation_view(operation: &PatchOperation, label_paths: bool) -> OperationView {
    let labelled = label_paths && node_addressed(&operation.kind);
    let mut rows = Vec::new();
    // **The title and any path-bearing row come back as pieces, not as one string** (RFC 036 review
    // v1 §2.3): a path is a name the repository chose, and only the frontend knows how to render one
    // so that two different paths cannot look alike. The words are still core's.
    let kind = || Piece::name(operation.kind.clone());
    let title = match operation.paths.as_slice() {
        // A rename names both of its own paths, in the title, as the Queue view does.
        [QueuedPath::Path(from), QueuedPath::Path(to)] if operation.kind == "rename-path" => {
            vec![
                kind(),
                Piece::said(" "),
                Piece::name(from.clone()),
                Piece::said(" → "),
                Piece::name(to.clone()),
            ]
        }
        [QueuedPath::Path(path)] if labelled => vec![
            kind(),
            Piece::said(" "),
            Piece::name(path.clone()),
            Piece::said(format!(" — {AS_AT_BLOCK_END}")),
        ],
        [QueuedPath::Path(path)] => vec![kind(), Piece::said(" "), Piece::name(path.clone())],
        // One sentence, no reason (decision 3), and prikk's node id beside it — which the raw view
        // shows too, and is the only honest thing stikk has to name the node by (F6).
        [QueuedPath::UnresolvedNode(node)] => {
            rows.push(OperationRow::Said(NO_PATH_REPORTED.to_string()));
            rows.push(OperationRow::Named(vec![
                Piece::said("node "),
                Piece::name(node.clone()),
            ]));
            vec![kind()]
        }
        // Any other shape — several paths, a mix, none at all — is listed rather than reshaped, with
        // each unresolved element carrying the same sentence.
        paths => {
            for path in paths {
                match path {
                    QueuedPath::Path(path) if labelled => {
                        rows.push(OperationRow::Named(vec![
                            Piece::name(path.clone()),
                            Piece::said(format!(" — {AS_AT_BLOCK_END}")),
                        ]));
                    }
                    QueuedPath::Path(path) => {
                        rows.push(OperationRow::Named(vec![Piece::name(path.clone())]));
                    }
                    QueuedPath::UnresolvedNode(node) => {
                        rows.push(OperationRow::Said(NO_PATH_REPORTED.to_string()));
                        rows.push(OperationRow::Named(vec![
                            Piece::said("node "),
                            Piece::name(node.clone()),
                        ]));
                    }
                }
            }
            vec![kind()]
        }
    };
    rows.extend(content_rows(&operation.content));
    OperationView {
        title,
        rows,
        raw: bounded_rows(&operation.raw),
    }
}

fn content_rows(content: &OperationContent) -> Vec<OperationRow> {
    match content {
        // Spans, labelled as spans: no line numbers, no context, no line-diff, at any prikk version
        // (RFC 021 F2, `T-T4`). prikk reports the text it replaced and the text it wrote, and that is
        // what stikk shows — **byte for byte**, because a span edit is very often a whitespace edit.
        OperationContent::TextEdit {
            old_span,
            replacement,
        } => {
            let mut rows = block_rows("span replaced", old_span);
            rows.extend(block_rows("replacement", replacement));
            rows
        }
        OperationContent::FileCreated { content, mode } => {
            let mut rows = vec![OperationRow::Said(format!("mode {}", octal(*mode)))];
            rows.extend(file_content_rows("content", content));
            rows
        }
        OperationContent::NodeDeleted { node_kind, content } => {
            let mut rows = Vec::new();
            // `file` at every measured state; anything else is named rather than assumed away.
            if node_kind != "file" {
                rows.push(OperationRow::Said(format!(
                    "prikk reports the removed node as {node_kind}"
                )));
            }
            rows.extend(file_content_rows("removed content", content));
            rows
        }
        // Blob ids and sizes. **Never bytes** (`FR-030`): stikk has no way to render them honestly and
        // no reason to try.
        OperationContent::BinaryReplaced { old, new } => {
            let mut rows = blob_rows("old blob", old);
            rows.extend(blob_rows("new blob", new));
            rows
        }
        OperationContent::PermChanged { old_mode, new_mode } => vec![OperationRow::Said(format!(
            "mode {} → {}",
            octal(*old_mode),
            octal(*new_mode)
        ))],
        OperationContent::PathRenamed {
            author_key_id,
            author_key_ids,
        } => {
            let mut rows = vec![OperationRow::Said(format!(
                "asserted by (AUTHOR key id) {author_key_id}"
            ))];
            // Carried only when prikk names more than the one: a list repeating the id above it would
            // be noise, and dropping a second asserting key would be a fact withheld.
            let others: Vec<&String> = author_key_ids
                .iter()
                .filter(|id| *id != author_key_id)
                .collect();
            if !others.is_empty() {
                rows.push(OperationRow::Said(format!(
                    "prikk also lists {}",
                    others
                        .iter()
                        .map(|id| id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            rows
        }
        // prikk's word is on the title row and its fields are in the raw view; what stikk adds is the
        // admission that it does not render this kind (RFC 035 §3 rule 3). **Never an empty operation.**
        OperationContent::Unrendered => vec![OperationRow::Said(
            "stikk does not render this operation kind — v shows prikk's own fields".to_string(),
        )],
    }
}

fn file_content_rows(label: &str, content: &FileContent) -> Vec<OperationRow> {
    match content {
        FileContent::Text(text) => block_rows(label, text),
        // Bytes, so the same two facts `replace-binary` carries and nothing more — identity and size,
        // **never the bytes** (`FR-030`; review v1 §2.4). This arrived as "no content to show" until
        // the review pointed out that is every image in a repository.
        FileContent::Binary(blob) => blob_rows(label, blob),
        // prikk names the blob it does not hold, which is **by design**, not a fault: a text node's
        // pre-edit blob is deliberately absent. The id is the one fact there is, so it is shown.
        FileContent::Unavailable(blob_id) => vec![
            OperationRow::Said(format!("{label}: prikk reports the content as unavailable")),
            OperationRow::Said(format!("blob {blob_id}")),
        ],
        // A kind beyond prikk's three: named with prikk's own word, neither dropped nor shown as empty
        // (§3 rule 5). **It does not claim the kind carries no content** (review v1 §3) — stikk does
        // not know that, and saying so would be a claim about prikk's data (`C-T2b`).
        FileContent::Unrendered(kind) => vec![OperationRow::Said(format!(
            "{label}: prikk reports it as {kind}, which stikk does not render; v shows prikk's own \
             fields"
        ))],
    }
}

fn blob_rows(label: &str, blob: &BinaryBlob) -> Vec<OperationRow> {
    vec![
        OperationRow::Said(format!("{label} — {} byte(s)", blob.size)),
        OperationRow::Said(blob.blob_id.clone()),
    ]
}

/// A labelled block of repository text: one row for an empty value, saying it is **empty** rather than
/// rendering a blank row that reads as a line of nothing (RFC 035 decision 2); otherwise one
/// **quoted** row per source line, bounded.
///
/// **The quotes are what make the rendering faithful** (review v1 §2.1). Each row holds one line of
/// prikk's text with its own terminator inside the quotes, so a re-indent, a tab against spaces, a
/// double space, trailing whitespace and a removed trailing newline are all visible, and two spans
/// that differ in any byte cannot render identically.
///
/// **The quotes alone were not enough for that last claim** (review v2 §2): a bidi override, a
/// zero-width character or a non-breaking space showed as nothing or as something else until
/// [`quoted`] escaped them too. See [`stikk_prikk::escape`] for what is escaped and why.
fn block_rows(label: &str, text: &str) -> Vec<OperationRow> {
    if text.is_empty() {
        return vec![OperationRow::Said(format!("{label}: (empty)"))];
    }
    let mut rows = vec![OperationRow::Said(format!("{label}:"))];
    let (shown, withheld) = bounded(text);
    // `split_inclusive` keeps each line's own `\n`, which is the only way a trailing newline present
    // and absent can render differently — `lines()` drops it, and that was the defect.
    rows.extend(
        shown
            .split_inclusive('\n')
            .map(|line| OperationRow::Content(quoted(line))),
    );
    if withheld > 0 {
        rows.push(OperationRow::Said(format!(
            "… {withheld} more bytes not shown"
        )));
    }
    rows
}

/// `line` as a quoted, escaped string: the spelling prikk's own JSON uses, which is injective — the
/// quotes delimit, and every character that does not show as itself is written one way.
///
/// **The table is [`stikk_prikk::escape`]'s, not this module's** (review v2 §2). This function held a
/// second copy once, and both copies escaped [`char::is_control`] — Unicode category `Cc` — so a bidi
/// override, a zero-width character or a non-breaking space passed through as itself and the
/// invariant above was not true. One table, shared with the raw view, is what keeps the two spellings
/// from drifting apart again.
fn quoted(line: &str) -> String {
    stikk_prikk::escape::quoted(line)
}

/// prikk's raw JSON for one operation, as rows, bounded by the same [`CONTENT_DISPLAY_BYTES`] — `v`
/// must not be the way around a bound that exists to keep a whole file off the screen.
///
/// Rendered verbatim by the frontend, like a content row: prikk's JSON nesting and any whitespace
/// inside its string values are part of what the raw view is for.
fn bounded_rows(text: &str) -> Vec<String> {
    let (shown, withheld) = bounded(text);
    let mut rows: Vec<String> = shown.lines().map(str::to_string).collect();
    if rows.is_empty() {
        rows.push(String::new());
    }
    if withheld > 0 {
        rows.push(format!("… {withheld} more bytes not shown"));
    }
    rows
}

/// The bounded prefix of `text` and the number of UTF-8 bytes withheld.
fn bounded(text: &str) -> (&str, usize) {
    if text.len() <= CONTENT_DISPLAY_BYTES {
        return (text, 0);
    }
    // The last `char` boundary at or below the bound. `floor_char_boundary` is unstable, so this walks
    // back from the bound itself — at most three bytes.
    let mut cut = CONTENT_DISPLAY_BYTES;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    match text.get(..cut) {
        Some(shown) => (shown, text.len() - cut),
        // Unreachable: `cut` is a boundary at or below `text.len()`. Degrading to the whole text is
        // the one answer that cannot show a lie — it shows more, never a wrong count.
        None => (text, 0),
    }
}

/// A mode as the octal a reader knows it by (`33188` → `100644`).
///
/// **Beside prikk's integers, never instead of them** (RFC 035 §4.3): the raw view shows `33188`,
/// because that is the field prikk wrote.
fn octal(mode: u64) -> String {
    format!("{mode:o}")
}

#[cfg(test)]
mod tests;

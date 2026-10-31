//! Getting files back: one file as it was at a block, or every file at it, written **outside the
//! repository** (RFC 053).
//!
//! # What this is for
//!
//! A reader who broke a file needs the version from before. stikk already shows every block's
//! files and every patch's content; until now it could hand none of it back, and the answer was
//! *go to the CLI*. prikk 0.54 writes one file's bytes at any point (`cat --output`) and every
//! file at a point into a new directory (`extract`), touching neither the repository nor the
//! worktree. This offers both, previewed.
//!
//! It is also the honest answer while stikk cannot take a change back: get the old version, look
//! at it, and copy what you want into your worktree yourself.
//!
//! # Two rules, and both are stikk's
//!
//! prikk writes a copy **inside** the repository happily at 0.54 — the copy then sits in the
//! worktree as untracked files, and the next commit captures it — and 0.55 refuses that for
//! `extract` but still not for `cat --output` (RFC 053 F2, RFC 055 F2). And neither command's
//! plan checks whether the destination is already taken: `extract --plan-only` answers the counts
//! and exits 0 into an existing directory, and only the real run refuses (F6).
//!
//! So stikk decides both before prikk is asked at all, and **spawns nothing** when it declines.
//! A missing *parent* directory is deliberately not pre-checked: prikk refuses it and names it,
//! and a second opinion about the same filesystem would be stikk guessing at a race.
//!
//! # The point is a block id
//!
//! Every read and every write here takes the **block** the reader opened, never a ref name. A
//! ref can move between the card being drawn and the moment it is confirmed; the block cannot.

use std::path::{Path, PathBuf};

use stikk_model::{RequestCategory, Result};
use stikk_prikk::{Extracted, FileMeta, FileMetaRead, FileSaved, Prikk};

use crate::confirm::{self, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken};

/// The first prikk minor that offers this, re-exported so a view says the number the seam gates
/// on — the one-constant rule every other band follows.
pub use stikk_prikk::FILES_FROM_MINOR;

/// stikk's own short name for saving one file, used in stale and declined messages.
pub const SAVE_FILE_OPERATION: &str = "save a file";
/// stikk's own short name for saving every file at a block.
pub const SAVE_ALL_OPERATION: &str = "save a block's files";

/// **Why stikk will not write a copy inside the repository** (decision 4, `C-E2`).
pub const INSIDE_THE_REPOSITORY: &str = "That path is inside this repository. The copy would sit in your worktree as untracked \
     files, and the next commit would capture them. Choose a path outside the repository.";

/// **Why stikk will not write over what is already there** (decision 4).
///
/// prikk refuses this too, in its own words — but only at the write, and `--plan-only` does not
/// check it at all (F6), so a card could be drawn for a destination that was never free.
pub const DESTINATION_EXISTS: &str =
    "Something is already there; stikk does not overwrite it — choose another name.";

/// The consequence both cards carry, in stikk's words and naming nothing (RFC 044 C1).
pub const COPY_CONSEQUENCE: &str =
    "A copy is written there; nothing in the repository or your worktree changes.";

/// **The one thing `cat --output` does not carry** (F3, measured): prikk writes the bytes and
/// sets no permissions, so a script comes back without its execute bit.
pub const LOSES_THE_EXECUTE_BIT: &str = "prikk saves the file's bytes, not its execute permission — the copy will not be \
     executable. Saving all files keeps it.";

/// **What `provisional` means**, in stikk's words (F4).
///
/// prikk's own reading: the files match that history, and *nothing has checked the history
/// itself* — neither its integrity nor whether its sealer is trusted here.
pub const PROVISIONAL_REPLAY: &str = "No trusted checkpoint is in reach, so prikk replays every file from the start of history. \
     The copy will match that history, but nothing has checked the history itself, and it may be \
     signed by a key this repository does not trust. Verify (`V`) checks it.";

/// **stikk's own reason, below the band**, for both keys.
#[must_use]
pub fn files_unavailable(minor: u32) -> String {
    format!(
        "Saving a file from a block needs prikk ≥ 0.{FILES_FROM_MINOR} — this prikk is 0.{minor}. \
         The rest of stikk works."
    )
}

/// The destination stikk suggests for one file at `block`: the repository's parent, the file's
/// own stem, and the block's short id (decision 4).
///
/// **Outside the repository by construction**, so a reader who accepts the suggestion never meets
/// [`INSIDE_THE_REPOSITORY`]. The path's directories are dropped — a copy of `dir/sub/d.txt` lands
/// beside the repository as `d-<short>.txt`, not under a `dir/sub` nobody asked to create.
///
/// **A leading-dot name has no extension**: `.env` suggests `.env-<short>`, because `env` is the
/// name and not a suffix.
#[must_use]
pub fn suggested_file_destination(repo: &Path, path: &str, block: &str) -> PathBuf {
    let leaf = path.rsplit('/').next().unwrap_or(path);
    let short = crate::bundle::short_id(block);
    let parent = repo.parent().unwrap_or(repo);
    // `rfind` rather than `split_once`, so `a.tar.gz` keeps `.gz` as the extension and
    // `a.tar-<short>.gz` as the suggestion — the shape every other tool uses.
    match leaf.rfind('.').filter(|dot| *dot > 0) {
        Some(dot) => parent.join(format!("{}-{short}{}", &leaf[..dot], &leaf[dot..])),
        None => parent.join(format!("{leaf}-{short}")),
    }
}

/// The directory stikk suggests for every file at `block`: the repository's parent, the
/// repository's own directory name, and the block's short id (decision 4).
#[must_use]
pub fn suggested_all_destination(repo: &Path, block: &str) -> PathBuf {
    let name = repo
        .file_name()
        .map_or_else(|| "repository".to_string(), |n| n.to_string_lossy().into());
    let parent = repo.parent().unwrap_or(repo);
    parent.join(format!("{name}-{}", crate::bundle::short_id(block)))
}

/// A block id, shortened the way every card and view shortens one — for a frontend that needs
/// the same spelling in a heading.
#[must_use]
pub fn block_short(block: &str) -> String {
    crate::bundle::short_id(block)
}

/// What Block detail's cursor is on (RFC 053 decision 3).
///
/// **One list, two sections.** The cursor ranges over the block's patches and then its files, so
/// a reader reaches a file with the same `↓` that walks the patches — and the keys can say which
/// of the two they act on without a second cursor to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockCursorRow {
    /// The patch at this index. `Enter` opens Patch detail for it.
    Patch(usize),
    /// The file at this index in the block's listed state. `Enter` does nothing; `F` saves it.
    File(usize),
}

/// How many rows Block detail's cursor ranges over: its patches, then its files.
///
/// Files count only where prikk **listed** them — below the band, or after a failed read, there
/// is a sentence where the list would be and nothing to put a cursor on.
#[must_use]
pub fn block_cursor_len(view: &crate::BlockDetailView) -> usize {
    view.patches.rows().len() + listed_files(view).len()
}

/// What `cursor` is on, or `None` past the end.
#[must_use]
pub fn block_cursor_row(view: &crate::BlockDetailView, cursor: usize) -> Option<BlockCursorRow> {
    let patches = view.patches.rows().len();
    if cursor < patches {
        return Some(BlockCursorRow::Patch(cursor));
    }
    let at = cursor - patches;
    (at < listed_files(view).len()).then_some(BlockCursorRow::File(at))
}

/// The file the cursor is on, if it is on one.
#[must_use]
pub fn block_cursor_file(view: &crate::BlockDetailView, cursor: usize) -> Option<&str> {
    match block_cursor_row(view, cursor)? {
        BlockCursorRow::Patch(_) => None,
        BlockCursorRow::File(at) => listed_files(view).get(at).map(|f| f.path.as_str()),
    }
}

fn listed_files(view: &crate::BlockDetailView) -> &[stikk_prikk::StateFile] {
    view.state.listed().map_or(&[][..], |state| &state.files)
}

/// **Why `F` has nothing to save here**, when Block detail shows a sentence instead of a file
/// list — below [`FILES_FROM_MINOR`] is said by [`files_unavailable`] instead.
///
/// It carries prikk's own reason for the missing list rather than restating it: that sentence
/// already says whether the band or a failed read is why.
#[must_use]
pub fn no_files_to_save(reason: &str) -> String {
    format!("There are no files to save at this block: {reason}")
}

/// **What `F` says when the cursor is on a patch** (decision 3): `F` saves a *file*, and the
/// files are listed below the patches.
pub const F_SAVES_A_FILE: &str = "`F` saves one of this block's files — move down to the files below the patches and press it \
     there. `A` saves all of them into a new folder.";

/// What a preview decided, before it becomes a token or a sentence.
enum ReadView {
    Blocked(String),
    Ready,
}

/// Whether saving one file is offered, and the token that arms it.
#[derive(Debug)]
pub enum SaveFilePreview {
    /// stikk will not: its own reason, or prikk's refusal verbatim.
    Blocked(String),
    /// Armed. The card is [`PreviewToken::summary`].
    Ready {
        /// The token this save is confirmed with.
        token: Box<PreviewToken>,
    },
}

/// Whether saving every file at a block is offered, and the token that arms it.
#[derive(Debug)]
pub enum SaveAllPreview {
    /// stikk will not: its own reason, or prikk's refusal verbatim.
    Blocked(String),
    /// Armed. The card is [`PreviewToken::summary`].
    Ready {
        /// The token this save is confirmed with.
        token: Box<PreviewToken>,
    },
}

/// What one confirmed file save writes — carried beside the token, as every write's target is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFileTarget {
    /// The block the file is read at.
    pub block: String,
    /// The repository path, as prikk names it.
    pub path: String,
    /// The file to write — resolved, and what the card showed.
    pub output: PathBuf,
}

/// What one confirmed whole-block save writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAllTarget {
    /// The block the files are read at.
    pub block: String,
    /// The directory to create — resolved, and what the card showed.
    pub dir: PathBuf,
}

/// Preview saving `path` as it was at `block`, to `output`.
///
/// **Both of stikk's rules are checked before prikk is asked anything** (the module doc), and
/// both are re-read at confirmation because the preview runs inside [`confirm::preview`].
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the metadata read raises; prikk's refusal of a
/// path absent at that block becomes [`SaveFilePreview::Blocked`] with prikk's words.
pub fn save_file_preview(
    prikk: &impl Prikk,
    repo: &Path,
    block: &str,
    path: &str,
    output: &Path,
) -> Result<SaveFilePreview> {
    let (block, path) = (block.to_string(), path.to_string());
    let output = output.to_path_buf();
    let intent = Intent {
        category: RequestCategory::Exchange,
        operation: SAVE_FILE_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        if let Some(reason) = destination_refused(repo, &output) {
            return Ok(blocked(&reason, "Save file"));
        }
        let meta = match prikk.file_meta(repo, &block, &path)? {
            FileMetaRead::Read(meta) => meta,
            FileMetaRead::Unavailable => {
                let minor = prikk.handshake()?.version.minor;
                return Ok(blocked(&files_unavailable(minor), "Save file"));
            }
        };
        Ok(file_card(&block, &path, &output, &meta))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => SaveFilePreview::Blocked(reason),
        ReadView::Ready => SaveFilePreview::Ready {
            token: Box::new(token),
        },
    })
}

/// Preview saving every file at `block` into `dir`.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the plan raises; prikk's refusal becomes
/// [`SaveAllPreview::Blocked`] with prikk's words.
pub fn save_all_preview(
    prikk: &impl Prikk,
    repo: &Path,
    block: &str,
    dir: &Path,
) -> Result<SaveAllPreview> {
    let block = block.to_string();
    let dir = dir.to_path_buf();
    let intent = Intent {
        category: RequestCategory::Exchange,
        operation: SAVE_ALL_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        if let Some(reason) = destination_refused(repo, &dir) {
            return Ok(blocked(&reason, "Save files"));
        }
        let plan = match prikk.extract_plan(repo, &block, &dir)? {
            Extracted::Read(plan) => plan,
            Extracted::Unavailable => {
                let minor = prikk.handshake()?.version.minor;
                return Ok(blocked(&files_unavailable(minor), "Save files"));
            }
        };
        Ok(all_card(&block, &dir, &plan))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => SaveAllPreview::Blocked(reason),
        ReadView::Ready => SaveAllPreview::Ready {
            token: Box::new(token),
        },
    })
}

/// **stikk's two destination rules, in order** — and neither spawns prikk (§7 trap 5).
fn destination_refused(repo: &Path, destination: &Path) -> Option<String> {
    // **`C-E2`'s own check, resolved** — the same control that keeps stikk's state files out of
    // every repository, applied to what stikk asks prikk to write.
    if stikk_state::paths::ensure_outside_repository(destination, Some(repo)).is_err() {
        return Some(INSIDE_THE_REPOSITORY.to_string());
    }
    // **Existence, and nothing more.** Whether the *parent* is there is prikk's to say.
    if already_there(destination) {
        return Some(DESTINATION_EXISTS.to_string());
    }
    None
}

/// Whether something is already at `path` — **prikk's own rule, so stikk's sentence answers
/// first.**
///
/// `Path::exists` follows a symlink, so a **dangling** one reads as nothing there; prikk's
/// `destination_exists` is `symlink_metadata(..).is_ok()`, which sees the link itself. Asking
/// the easier question let stikk arm a card that prikk then refused — nothing was written
/// either way, but the refusal came from the wrong program, in the wrong words.
///
/// Used by the two saves and by the bundle export, so the three cannot drift.
pub(crate) fn already_there(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// One file's card (decision 5).
fn file_card(
    block: &str,
    path: &str,
    output: &Path,
    meta: &FileMeta,
) -> (ReadView, ConfirmationSummary) {
    let mut summary = summary("Save file", COPY_CONSEQUENCE);
    summary.target_ids = vec![
        confirm::TargetId::named("block", crate::bundle::short_id(block)),
        confirm::TargetId::named("file", path.to_string()),
        confirm::TargetId::named("size", format!("{} bytes", meta.size)),
        confirm::TargetId::named("to", output.display().to_string()),
    ];
    // **The one notice this card can carry** (F3): `cat --output` writes the bytes and no mode.
    if meta.mode & 0o111 != 0 {
        summary.notices = vec![LOSES_THE_EXECUTE_BIT.to_string()];
    }
    (ReadView::Ready, summary)
}

/// Every file's card (decision 5).
fn all_card(
    block: &str,
    dir: &Path,
    plan: &stikk_prikk::ExtractReport,
) -> (ReadView, ConfirmationSummary) {
    let mut summary = summary("Save files", COPY_CONSEQUENCE);
    summary.target_ids = vec![
        confirm::TargetId::named("block", crate::bundle::short_id(block)),
        confirm::TargetId::named("files", plan.file_count.to_string()),
        confirm::TargetId::named("bytes", plan.total_bytes.to_string()),
        confirm::TargetId::named("to", dir.display().to_string()),
    ];
    // **The one notice this card can carry** (F4), and only when prikk says so.
    if plan.provisional {
        summary.notices = vec![PROVISIONAL_REPLAY.to_string()];
    }
    (ReadView::Ready, summary)
}

/// A card with everything that does not apply here left empty. **No key, Tier Two** (decision 2):
/// a copy outside the repository signs nothing.
fn summary(operation: &str, consequence: &str) -> ConfirmationSummary {
    ConfirmationSummary {
        operation: operation.to_string(),
        target_ids: Vec::new(),
        counts: Vec::new(),
        capability: None,
        consequence: consequence.to_string(),
        target_name: None,
        signing_key_id: None,
        signing_key_claim: confirm::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        notices: Vec::new(),
        message_note: None,
        materializes: None,
        merge_plan: None,
        preamble: None,
        undo_plan: None,
        asides: Vec::new(),
    }
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: &str, operation: &str) -> (ReadView, ConfirmationSummary) {
    (
        ReadView::Blocked(reason.to_string()),
        summary(operation, ""),
    )
}

/// Save the file this token was previewed for.
///
/// **Nothing is re-read afterwards**: nothing in the repository changed (decision 6).
///
/// # Errors
/// As [`save_file_preview`], plus every refusal prikk raises — a missing parent directory among
/// them, in prikk's own words.
pub fn save_file_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &SaveFileTarget,
) -> Result<Outcome<FileSaved>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.save_file(repo, &target.block, &target.path, &target.output)
    })
}

/// Save every file at the block this token was previewed for.
///
/// # Errors
/// As [`save_all_preview`], plus every refusal prikk raises.
pub fn save_all_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &SaveAllTarget,
) -> Result<Outcome<Extracted>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.extract(repo, &target.block, &target.dir)
    })
}

/// What stikk says after one file is written (decision 6).
#[must_use]
pub fn saved_file_line(target: &SaveFileTarget) -> String {
    format!(
        "Saved {} as it was at {} to {}.",
        target.path,
        crate::bundle::short_id(&target.block),
        target.output.display()
    )
}

/// What stikk says after a block's files are written, from prikk's own report (decision 6).
///
/// The provisional notice is repeated here when the report says so: the reader confirmed on a
/// plan, and the run is what actually replayed.
#[must_use]
pub fn saved_all_lines(target: &SaveAllTarget, report: &stikk_prikk::ExtractReport) -> Vec<String> {
    let mut lines = vec![format!(
        "Wrote {} files ({} bytes) to {}.",
        report.file_count,
        report.total_bytes,
        target.dir.display()
    )];
    if report.provisional {
        lines.push(PROVISIONAL_REPLAY.to_string());
    }
    lines
}

#[cfg(test)]
mod tests;

//! What stikk remembers about a repository between launches (design `DM-04`, `LC-3`/`LC-5`/`LC-6`;
//! RFC 046).
//!
//! **A convenience, never authority.** A record holds the **name** of the ref that was focused, which
//! of four top-level views was open, and whether the untracked filter was on — nothing else. No
//! repository content, no object ids, no prose (`INV-5`, `INV-9`). Deleting `<state_dir>/sessions/`
//! costs a reader their place and nothing more, and the guide says so.
//!
//! **Nothing here is believed.** [`load`] hands back a record; the caller re-validates every name
//! against prikk before showing any of it (`LC-6`, `T-T4`). A record that cannot be decoded is
//! ignored **whole** — never partly applied — and so is one whose stored path is not this
//! repository's, which is how a hash collision becomes a miss rather than another repository's state.
//!
//! **This is `C-E2`'s first real caller.** [`save`] runs
//! [`crate::paths::ensure_outside_repository`] against the file it is about to write, with the open
//! repository's root, before it creates anything. A refused or failed save is the caller's to report
//! once and carry on from (RFC 046 decision 3); it never stops a session.

use std::path::{Path, PathBuf};

use stikk_model::{Result, StikkError};

/// The format this module writes. A record naming any other version is ignored whole: a future
/// stikk that adds a field bumps this, rather than teaching every older stikk to skip a key it
/// does not know.
const FORMAT_VERSION: &str = "1";

/// At most this many records are kept (RFC 046 decision 7). When a write would exceed it, the
/// least recently written are removed — never the one just written.
pub const MAX_RECORDS: usize = 200;

/// The directory under [`crate::paths::state_dir`] that holds them.
const SESSIONS_DIR: &str = "sessions";

/// Which top-level view was open (RFC 046 F2's four, and only those).
///
/// **Block detail, Patch detail and Compare are stored as [`Self::History`]**: they need object
/// ids and, for a comparison, a second point, which `DM-04`'s id anchors are a later increment
/// for. **Verify is stored as whatever was beneath it**, because reopening it would run a full
/// verification at launch, which `NFR-P05` forbids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastView {
    /// The Orientation view — also the fallback for anything not in this list.
    Orientation,
    /// A ref's block history.
    History,
    /// The worktree-vs-baseline Changes view.
    Changes,
    /// The Queue view.
    Queue,
}

impl LastView {
    /// The word written to the file. Stable across releases: it is on disk.
    const fn name(self) -> &'static str {
        match self {
            Self::Orientation => "orientation",
            Self::History => "history",
            Self::Changes => "changes",
            Self::Queue => "queue",
        }
    }

    fn parse(word: &[u8]) -> Option<Self> {
        match word {
            b"orientation" => Some(Self::Orientation),
            b"history" => Some(Self::History),
            b"changes" => Some(Self::Changes),
            b"queue" => Some(Self::Queue),
            _ => None,
        }
    }
}

/// One repository's remembered session (`DM-04`, narrowed to RFC 046 F2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    /// The canonical repository root this record belongs to.
    ///
    /// **Stored so a hash collision is a miss**, not another repository's state. [`load`] compares
    /// the file's bytes with the root it was asked about and answers `None` on any difference; the
    /// `PathBuf` here is always the caller's own, never rebuilt from the file — a path is an
    /// `OsStr`, and rebuilding one from bytes needs `unsafe`, which this crate forbids.
    pub root: PathBuf,
    /// The ref that was focused, by name, or `None` where none was.
    pub focus: Option<String>,
    /// Which of the four top-level views was open.
    pub view: LastView,
    /// Whether the Changes view's untracked filter was hiding untracked entries.
    pub hide_untracked: bool,
}

/// The file name for `canonical_root`: sixteen lowercase hex digits of [`fnv1a64`].
///
/// **Written out rather than hashed with `DefaultHasher`**, whose output Rust does not promise to
/// keep stable: a toolchain upgrade would rename every file and silently lose every session.
#[must_use]
pub fn file_name(canonical_root: &Path) -> String {
    format!(
        "{:016x}",
        fnv1a64(canonical_root.as_os_str().as_encoded_bytes())
    )
}

/// FNV-1a, 64-bit, over `bytes` — the algorithm spelled out, with its two constants, so this file
/// name cannot change under us.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// The record for `canonical_root`, or `None`.
///
/// **`None` is every failure**: no file, an unreadable one, a version this stikk does not know,
/// any decode failure at all, or a stored path that is not this root. Never an error, never a
/// panic — a session that cannot be remembered is a launch that starts fresh, which is exactly
/// what every launch did before this existed.
#[must_use]
pub fn load(state_dir: &Path, canonical_root: &Path) -> Option<SessionRecord> {
    let bytes = std::fs::read(record_path(state_dir, canonical_root)).ok()?;
    decode(&bytes, canonical_root)
}

/// Write `record` for `canonical_root`, atomically and outside every repository.
///
/// The order is the point: [`crate::paths::ensure_outside_repository`] **first**, with
/// `Some(canonical_root)` — before `sessions/` is created, before a temporary file exists. Then a
/// temporary file in the same directory and a `rename` over the target, so a reader never sees a
/// half-written record and a crash leaves either the old one or the new one. Then the bound.
///
/// # Errors
/// [`StikkError::Internal`] when the target is repository-internal (`C-E2`), and
/// [`StikkError::Environment`] when the write itself fails. Both are the caller's to report once
/// and carry on from (RFC 046 decision 3).
pub fn save(state_dir: &Path, canonical_root: &Path, record: &SessionRecord) -> Result<()> {
    let target = record_path(state_dir, canonical_root);
    crate::paths::ensure_outside_repository(&target, Some(canonical_root))?;
    let dir = state_dir.join(SESSIONS_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, &e))?;
    // The temporary file sits in the same directory, so the rename is within one filesystem and
    // so atomic. Its name carries the process id: two stikks on one repository would otherwise
    // race on one temporary name, and the loser would rename a file the winner had removed.
    let temporary = dir.join(format!(
        "{}.{}.tmp",
        file_name(canonical_root),
        std::process::id()
    ));
    std::fs::write(&temporary, encode(record, canonical_root))
        .map_err(|e| write_failed(&temporary, &e))?;
    if let Err(error) = std::fs::rename(&temporary, &target) {
        // Leave nothing behind: a stray `.tmp` in the state directory is stikk's litter.
        let _ = std::fs::remove_file(&temporary);
        return Err(write_failed(&target, &error));
    }
    prune(&dir, &target);
    Ok(())
}

/// Keep at most [`MAX_RECORDS`], oldest first by modification time, **never the one just
/// written**.
///
/// Best-effort throughout: a directory that cannot be listed, or a file that cannot be removed,
/// leaves the bound unenforced for now and costs nothing. Failing a save over housekeeping would
/// be the wrong trade.
fn prune(dir: &Path, just_written: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut records: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path != just_written && is_record_name(path))
        .filter_map(|path| {
            let modified = path.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();
    // The file just written is excluded above and still counts against the bound, so the list
    // may hold one fewer than the limit.
    if records.len() < MAX_RECORDS {
        return;
    }
    records.sort();
    let excess = records.len() + 1 - MAX_RECORDS;
    for (_, path) in records.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
}

/// Whether a path in `sessions/` is a record rather than a temporary file: sixteen lowercase hex
/// digits and nothing else.
fn is_record_name(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        name.as_encoded_bytes().len() == 16
            && name
                .as_encoded_bytes()
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    })
}

fn record_path(state_dir: &Path, canonical_root: &Path) -> PathBuf {
    state_dir.join(SESSIONS_DIR).join(file_name(canonical_root))
}

fn write_failed(path: &Path, error: &std::io::Error) -> StikkError {
    StikkError::environment(
        format!(
            "could not write stikk's session record at {}",
            path.display()
        ),
        std::io::Error::new(error.kind(), error.to_string()),
    )
}

// ---------------------------------------------------------------------------------------------
// The encoding. **Lines of `key=value`, over bytes rather than text**, for one reason: a record
// carries a filesystem path, and a path is an `OsStr` that need not be UTF-8 on any platform
// stikk supports. Reading and writing bytes keeps the comparison exact without `unsafe`.
//
// Only three bytes are escaped — `\` itself, and the two that would otherwise end a line. A `=`
// needs none, because a line splits at its **first** one. Everything else, a space and every
// non-ASCII byte included, is written as itself: RFC 046 decision 2 asks that every ref name
// prikk accepts round-trips, and prikk accepts a great many.

const ESCAPE: u8 = b'\\';
const NEWLINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

fn escape_into(out: &mut Vec<u8>, bytes: &[u8]) {
    for byte in bytes {
        match *byte {
            ESCAPE => out.extend_from_slice(br"\\"),
            NEWLINE => out.extend_from_slice(br"\n"),
            CARRIAGE_RETURN => out.extend_from_slice(br"\r"),
            other => out.push(other),
        }
    }
}

/// `None` on any escape this module does not write — an unknown escape is a record stikk did not
/// produce, and a record it does not understand is ignored whole.
fn unescape(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes.iter().copied();
    while let Some(byte) = rest.next() {
        if byte != ESCAPE {
            out.push(byte);
            continue;
        }
        match rest.next()? {
            ESCAPE => out.push(ESCAPE),
            b'n' => out.push(NEWLINE),
            b'r' => out.push(CARRIAGE_RETURN),
            _ => return None,
        }
    }
    Some(out)
}

fn encode(record: &SessionRecord, canonical_root: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"version=");
    out.extend_from_slice(FORMAT_VERSION.as_bytes());
    out.push(NEWLINE);
    out.extend_from_slice(b"path=");
    escape_into(&mut out, canonical_root.as_os_str().as_encoded_bytes());
    out.push(NEWLINE);
    if let Some(focus) = &record.focus {
        out.extend_from_slice(b"focus=");
        escape_into(&mut out, focus.as_bytes());
        out.push(NEWLINE);
    }
    out.extend_from_slice(b"view=");
    out.extend_from_slice(record.view.name().as_bytes());
    out.push(NEWLINE);
    out.extend_from_slice(b"hide-untracked=");
    out.extend_from_slice(if record.hide_untracked {
        b"yes".as_slice()
    } else {
        b"no".as_slice()
    });
    out.push(NEWLINE);
    out
}

/// `None` on anything at all that is not a record this module wrote for this root.
fn decode(bytes: &[u8], canonical_root: &Path) -> Option<SessionRecord> {
    let (mut version, mut path, mut focus, mut view, mut hide) = (false, false, None, None, None);
    for line in bytes.split(|b| *b == NEWLINE) {
        if line.is_empty() {
            continue;
        }
        let at = line.iter().position(|b| *b == b'=')?;
        let (key, value) = line.split_at(at);
        let value = unescape(value.get(1..)?)?;
        match key {
            b"version" => {
                if value != FORMAT_VERSION.as_bytes() {
                    return None;
                }
                version = true;
            }
            // **Compared, never rebuilt.** A differing path means the hash collided, and this
            // record belongs to another repository.
            b"path" => {
                if value != canonical_root.as_os_str().as_encoded_bytes() {
                    return None;
                }
                path = true;
            }
            b"focus" => focus = Some(String::from_utf8(value).ok()?),
            b"view" => view = Some(LastView::parse(&value)?),
            b"hide-untracked" => {
                hide = Some(match value.as_slice() {
                    b"yes" => true,
                    b"no" => false,
                    _ => return None,
                });
            }
            // A key this version does not know means a format it does not know, whatever the
            // version line claimed.
            _ => return None,
        }
    }
    if !version || !path {
        return None;
    }
    Some(SessionRecord {
        root: canonical_root.to_path_buf(),
        focus,
        view: view?,
        hide_untracked: hide?,
    })
}

#[cfg(test)]
mod tests;

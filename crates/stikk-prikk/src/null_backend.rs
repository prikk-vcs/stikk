//! A scripted [`Prikk`] implementation for tests and offline demos (design MOD-02 `null_backend`).
//!
//! It returns canned responses with no repository and no `prikk` binary, so the operation layer and
//! the frontends can be exercised deterministically — including refusal branches — without a real
//! prikk. It is the seam the layers above test against (design TS-02).

use std::path::Path;

use stikk_model::{ChangeToken, Result, StikkError};

use crate::version::Version;
use crate::{Handshake, History, Orientation, Prikk, RefEntry, StateFiles, WorktreeStatus};

type Scripted<T> = std::result::Result<T, String>;

/// A [`Prikk`] whose answers are set up front.
#[derive(Debug, Clone)]
pub struct NullBackend {
    handshake: Handshake,
    orientation: Scripted<Orientation>,
    history: Scripted<History>,
    state: Scripted<StateFiles>,
    refs: Scripted<Vec<RefEntry>>,
    tags: Scripted<Vec<RefEntry>>,
    worktree: Scripted<WorktreeStatus>,
    change_token: Scripted<ChangeToken>,
}

impl NullBackend {
    /// A backend reporting a supported, validated prikk (RFC 009: the floor is 0.28; the version
    /// reported here, 0.30.0, is left as-is by RFC 012 F-e's ceiling raise to 0.31 — many render tests
    /// assert this exact string, and moving it is unrelated churn this increment does not need) and a
    /// clean, empty repository — the common happy path.
    #[must_use]
    pub fn supported() -> Self {
        Self {
            handshake: Handshake {
                version: Version {
                    major: 0,
                    minor: 30,
                    patch: 0,
                },
                raw_version: "prikk 0.30.0".to_string(),
                supported: true,
                validated: true,
            },
            orientation: Ok(Orientation {
                queued_patches: 0,
                queued_target: None,
                main_ref_state: None,
                trailing_partial_wal_bytes: 0,
            }),
            history: Ok(History {
                reff: "heads/main".to_string(),
                blocks: Vec::new(),
            }),
            state: Ok(StateFiles {
                target_block: String::new(),
                files: Vec::new(),
                total_bytes: 0,
            }),
            refs: Ok(vec![RefEntry {
                name: "heads/main".to_string(),
                id: "0".repeat(64),
                closed: false,
                received: false,
            }]),
            tags: Ok(Vec::new()),
            worktree: Ok(WorktreeStatus {
                reff: "heads/main".to_string(),
                clean: true,
                tracked: 0,
                unchanged: 0,
                missing: 0,
                modified: 0,
                untracked: 0,
                unsupported: 0,
                entries: Vec::new(),
                queued_elsewhere: None,
            }),
            // Matches the default `refs`/`orientation` above, so an un-scripted backend's token is
            // internally consistent rather than an arbitrary placeholder.
            change_token: Ok(ChangeToken::compose(
                [("heads/main", "0".repeat(64).as_str())],
                0,
                None,
            )),
        }
    }

    /// Set the reported prikk version (for testing the version floor/ceiling — e.g. Changes needs
    /// ≥ 0.28, or a version above 0.30 to exercise the unvalidated-but-running case, RFC 009 decision
    /// 7). Recomputes both `supported` and `validated`.
    #[must_use]
    pub fn with_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.handshake.version = Version {
            major,
            minor,
            patch,
        };
        self.handshake.raw_version = format!("prikk {major}.{minor}.{patch}");
        self.handshake.supported = self.handshake.version.is_supported();
        self.handshake.validated = self.handshake.version.is_validated();
        self
    }

    /// Replace the worktree status this backend returns.
    #[must_use]
    pub fn with_worktree_status(mut self, status: WorktreeStatus) -> Self {
        self.worktree = Ok(status);
        self
    }

    /// Set the `queued_elsewhere` warning on the worktree status this backend returns, leaving
    /// everything else as previously set (RFC 009 F4) — the state that caused the defect, made
    /// drivable with no prikk and no repository.
    #[must_use]
    pub fn with_queued_elsewhere(mut self, note: impl Into<String>) -> Self {
        if let Ok(status) = &mut self.worktree {
            status.queued_elsewhere = Some(note.into());
        }
        self
    }

    /// Make the worktree-status call fail with a refusal carrying `message`.
    #[must_use]
    pub fn with_worktree_status_refusal(mut self, message: impl Into<String>) -> Self {
        self.worktree = Err(message.into());
        self
    }

    /// Replace the orientation this backend returns.
    #[must_use]
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Ok(orientation);
        self
    }

    /// Make the orientation call fail with a refusal carrying `message` (for testing refusal flows).
    #[must_use]
    pub fn with_orientation_refusal(mut self, message: impl Into<String>) -> Self {
        self.orientation = Err(message.into());
        self
    }

    /// Replace the block lineage this backend returns.
    #[must_use]
    pub fn with_history(mut self, history: History) -> Self {
        self.history = Ok(history);
        self
    }

    /// Make the history call fail with a refusal carrying `message`.
    #[must_use]
    pub fn with_history_refusal(mut self, message: impl Into<String>) -> Self {
        self.history = Err(message.into());
        self
    }

    /// Replace the tip state file set this backend returns.
    #[must_use]
    pub fn with_state(mut self, state: StateFiles) -> Self {
        self.state = Ok(state);
        self
    }

    /// Replace the branch ref list this backend returns from [`Prikk::refs`].
    #[must_use]
    pub fn with_refs(mut self, refs: Vec<RefEntry>) -> Self {
        self.refs = Ok(refs);
        self
    }

    /// Replace the tag list this backend returns from [`Prikk::tags`] (RFC 012 FR-014).
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<RefEntry>) -> Self {
        self.tags = Ok(tags);
        self
    }

    /// Make the tag-list call fail with a refusal carrying `message`.
    #[must_use]
    pub fn with_tags_refusal(mut self, message: impl Into<String>) -> Self {
        self.tags = Err(message.into());
        self
    }

    /// Replace the change token this backend returns from [`Prikk::change_token`] (RFC 003), so the
    /// layers above can script staleness deterministically — set two backends (or a session's before
    /// and after) to different tokens to exercise "the repository changed", or to the same one to
    /// exercise "nothing changed". Independent of this backend's own `refs`/`orientation` fields: a
    /// real `CliBackend` composes its token from those reads, but a script may want an arbitrary token
    /// with no matching ref/orientation state, since only equality between two tokens is ever
    /// load-bearing — never their contents.
    #[must_use]
    pub fn with_change_token(mut self, token: ChangeToken) -> Self {
        self.change_token = Ok(token);
        self
    }

    /// Make the change-token call fail with a refusal carrying `message`.
    #[must_use]
    pub fn with_change_token_refusal(mut self, message: impl Into<String>) -> Self {
        self.change_token = Err(message.into());
        self
    }

    /// Mark the reported prikk version as unsupported (for testing the version-skew path).
    #[must_use]
    pub fn unsupported(mut self) -> Self {
        self.handshake.supported = false;
        self
    }
}

fn deliver<T: Clone>(scripted: &Scripted<T>) -> Result<T> {
    scripted
        .clone()
        .map_err(|message| StikkError::Refusal { message })
}

impl Prikk for NullBackend {
    fn handshake(&self) -> Result<Handshake> {
        Ok(self.handshake.clone())
    }

    fn orientation(&self, _repo: &Path) -> Result<Orientation> {
        deliver(&self.orientation)
    }

    fn history(&self, _repo: &Path, _reff: &str, _limit: usize) -> Result<History> {
        deliver(&self.history)
    }

    fn block_state(&self, _repo: &Path, _reff: &str) -> Result<StateFiles> {
        deliver(&self.state)
    }

    fn refs(&self, _repo: &Path) -> Result<Vec<RefEntry>> {
        deliver(&self.refs)
    }

    fn tags(&self, _repo: &Path) -> Result<Vec<RefEntry>> {
        deliver(&self.tags)
    }

    fn worktree_status(&self, _repo: &Path, _reff: &str) -> Result<WorktreeStatus> {
        deliver(&self.worktree)
    }

    fn change_token(&self, _repo: &Path) -> Result<ChangeToken> {
        deliver(&self.change_token)
    }
}

#[cfg(test)]
mod tests;

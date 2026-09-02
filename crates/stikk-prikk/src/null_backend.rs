//! A scripted [`Prikk`] implementation for tests and offline demos (design MOD-02 `null_backend`).
//!
//! It returns canned responses with no repository and no `prikk` binary, so the operation layer and
//! the frontends can be exercised deterministically — including refusal branches — without a real
//! prikk. It is the seam the layers above test against (design TS-02).

use std::path::Path;

use stikk_model::{Result, StikkError};

use crate::version::Version;
use crate::{Handshake, History, Orientation, Prikk, RefEntry, StateFiles};

type Scripted<T> = std::result::Result<T, String>;

/// A [`Prikk`] whose answers are set up front.
#[derive(Debug, Clone)]
pub struct NullBackend {
    handshake: Handshake,
    orientation: Scripted<Orientation>,
    history: Scripted<History>,
    state: Scripted<StateFiles>,
    refs: Scripted<Vec<RefEntry>>,
}

impl NullBackend {
    /// A backend reporting a supported prikk and a clean, empty repository — the common happy path.
    #[must_use]
    pub fn supported() -> Self {
        Self {
            handshake: Handshake {
                version: Version {
                    major: 0,
                    minor: 27,
                    patch: 1,
                },
                raw_version: "prikk 0.27.1".to_string(),
                supported: true,
            },
            orientation: Ok(Orientation {
                queued_patches: 0,
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
        }
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

    /// Replace the ref list this backend returns.
    #[must_use]
    pub fn with_refs(mut self, refs: Vec<RefEntry>) -> Self {
        self.refs = Ok(refs);
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
}

#[cfg(test)]
mod tests;

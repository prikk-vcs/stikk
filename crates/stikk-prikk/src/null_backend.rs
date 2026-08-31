//! A scripted [`Prikk`] implementation for tests and offline demos (design MOD-02 `null_backend`).
//!
//! It returns canned responses with no repository and no `prikk` binary, so the operation layer and
//! the frontends can be exercised deterministically — including refusal branches — without a real
//! prikk. It is the seam the layers above test against (design TS-02).

use std::path::Path;

use stikk_model::{Result, StikkError};

use crate::version::Version;
use crate::{Handshake, Orientation, Prikk};

/// A [`Prikk`] whose answers are set up front.
#[derive(Debug, Clone)]
pub struct NullBackend {
    handshake: Handshake,
    orientation: std::result::Result<Orientation, String>,
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

    /// Mark the reported prikk version as unsupported (for testing the version-skew path).
    #[must_use]
    pub fn unsupported(mut self) -> Self {
        self.handshake.supported = false;
        self
    }
}

impl Prikk for NullBackend {
    fn handshake(&self) -> Result<Handshake> {
        Ok(self.handshake.clone())
    }

    fn orientation(&self, _repo: &Path) -> Result<Orientation> {
        self.orientation
            .clone()
            .map_err(|message| StikkError::Refusal { message })
    }
}

#[cfg(test)]
mod tests;

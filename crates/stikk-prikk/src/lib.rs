//! The prikk seam — the only code in stikk that talks to prikk (design `stikk-04` AR-02, MOD-02).
//!
//! Everything above this crate depends on the [`Prikk`] trait, never on *how* prikk is reached. This
//! localizes four things to one place: the constraint that stikk drives prikk only through its public
//! surface (CON-1), the pre-1.0-library risk and machine-output gap (UD-02), the EPIPE guard (UD-04),
//! and version-skew handling (NFR-R03). The v1 implementor is [`cli_backend::CliBackend`], which
//! drives the `prikk` binary; a linked-library backend is deferred behind the same trait.
//!
//! Two security properties live here and nowhere else:
//! - [`env`] reads signing-key **presence only, never values** (threat model C-I1, data model LC-13).
//! - No key material ever crosses the seam: prikk reads its own environment when it signs; stikk
//!   hands it nothing, because it holds nothing (design SEAM-06).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cli_backend;
pub mod env;
pub mod null_backend;
pub mod version;

pub use cli_backend::CliBackend;
pub use null_backend::NullBackend;
pub use version::Version;

use std::path::Path;

use stikk_model::Result;

/// The result of the version-and-capability probe stikk performs when it first reaches prikk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    /// The parsed prikk version.
    pub version: Version,
    /// prikk's raw `--version` line, preserved verbatim for display and diagnostics (NFR-I03).
    pub raw_version: String,
    /// Whether this prikk version is within the range stikk was validated against (NFR-R03). When
    /// false, the operation layer degrades to read-only rather than misrender an unknown format.
    pub supported: bool,
}

/// A minimal read-only orientation of a repository — the summary the launcher and the (future) TUI
/// Orientation view show (design VW-01, FR-002). Deliberately small in this foundation increment;
/// more fields land as the read surfaces are built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orientation {
    /// Number of patches queued in the active WAL, not yet sealed.
    pub queued_patches: u64,
    /// The current RefState object id of `heads/main`, if the ref is published.
    pub main_ref_state: Option<String>,
    /// Trailing partial WAL bytes, if any — a torn tail an interrupted commit left behind.
    pub trailing_partial_wal_bytes: u64,
}

/// The entire prikk contract stikk depends on. Every method returns [`stikk_model::StikkError`] on
/// failure, classified into the presentation taxonomy the operation layer consumes.
///
/// This foundation increment defines the two read methods the launcher needs; the full nine-category
/// surface (design CT-03) grows against this trait without disturbing callers.
pub trait Prikk {
    /// Probe prikk's version and whether it is supported (design SEAM-05).
    ///
    /// # Errors
    /// [`stikk_model::StikkError::Environment`] when `prikk` cannot be launched or its version line
    /// cannot be parsed.
    fn handshake(&self) -> Result<Handshake>;

    /// Read a minimal read-only orientation of the repository rooted at `repo`.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified: a refusal, an environment fault (unparseable output,
    /// missing binary), or a lock conflict, per the seam's classification rules.
    fn orientation(&self, repo: &Path) -> Result<Orientation>;
}

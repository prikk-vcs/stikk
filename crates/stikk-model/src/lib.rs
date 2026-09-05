//! Shared kernel for stikk (internal design `stikk-04` MOD-01).
//!
//! stikk is a TUI/GUI front-end for the prikk version control system. Its founding property, from
//! which the whole design follows, is that **stikk owns no repository authority and no secrets**:
//! every repository fact is re-derived from prikk through the seam, and prikk — never stikk — reads
//! signing key material. This crate holds the vocabulary the layers above share and carries no I/O
//! of its own, so it can be depended on freely without pulling a transport or a filesystem.
//!
//! - [`error`] — the single [`StikkError`] type with seven presentation classes, preserving prikk's
//!   verbatim message (requirement NFR-I03, FR-110).
//! - [`id`] — validated object-id and ref-name newtypes; stikk never fabricates an identifier.
//! - [`category`] — the request-category vocabulary the seam and operation layer share, carrying
//!   each category's mutation/cancellability/lock policy as data (design CT-03).
//! - [`capability`] — the derived capability levels (Viewer/Author/Maintainer/Operator) that gate
//!   what a session may do, computed from prikk-side signing readiness (design AC-01…04).
//! - [`change_token`] — the repository-change detection primitive (design `LC-4`; RFC 003), composed
//!   from prikk-observable state; a preview↔execute binding stamps and re-checks it, never a lock.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod capability;
pub mod category;
pub mod change_token;
pub mod error;
pub mod id;

pub use capability::{Capability, Readiness};
pub use category::RequestCategory;
pub use change_token::ChangeToken;
pub use error::{Result, StikkError};
pub use id::{ObjectId, RefName};

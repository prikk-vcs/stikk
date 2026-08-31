//! stikk's own durable data (design `stikk-04` MOD-03, data model `stikk-05`).
//!
//! Everything this crate stores is a *convenience* — a preference, a pointer, a cache — that can be
//! deleted with no effect on any repository (data model INV-1). Nothing here is ever read back as
//! authority about repository state; on every use, repository facts are re-derived from prikk. Two
//! rules make that safe and are enforced here in code:
//!
//! - [`paths`] resolves stikk's stores to **user-scope locations only** and **refuses any
//!   repository-internal path** before a write (data model INV-2, threat model C-E2). Because prikk
//!   has no general foreign-file scan of `.prikk/`, this refusal is the *primary* control against
//!   stikk ever writing into a repository, not defence-in-depth.
//! - [`config`] preserves unknown keys and falls back to defaults on a malformed file, so a user's
//!   newer key survives and a corrupt file never blocks launch (data model LC-1/LC-2, INV-4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod handle;
pub mod paths;

pub use config::Config;
pub use handle::RepositoryHandle;

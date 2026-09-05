//! The repository-change token (design `LC-4`; RFC 003).
//!
//! stikk holds no repository authority (`INV-1`): every derived view and armed preview is valid only
//! as long as the repository has not changed underneath it. [`ChangeToken`] is the cheap, comparable
//! signal that answers "has anything changed since we last looked?" — composed from prikk-observable
//! state the seam already reads (ref pointers, queue extent), never from `.prikk/` bytes stikk is
//! forbidden to read (`CON-1`).
//!
//! **It is a detection primitive, not a lock** (`CT-05`/`NFR-R02`). Between reading a token and acting
//! on it, the repository can change again — prikk's own locking is the real guard against a mutation
//! racing another writer. What the token gives is that a preview cannot be *executed* against a
//! repository that has demonstrably moved since the preview was computed; it does not make execution
//! atomic, and nothing in the UI may imply that it does.
//!
//! **It is not an identity.** [`ChangeToken`] changes with ordinary history growth, so it cannot serve
//! as a stable fingerprint for "is this the same repository" — that question is answered by canonical
//! path plus `INV-5`'s re-resolution-on-load discipline (RFC 003 decision 4), not by this type.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// An opaque, cheap-to-store staleness marker (design `LC-4`). Two tokens composed from an unchanged
/// signal set are equal; that is the only guarantee — nothing above the seam may branch on, decode, or
/// otherwise interpret a token's contents (RFC 003 decision 3). Its `Debug` prints the digest only, on
/// purpose: there is no structure in it worth reading.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChangeToken(u64);

impl ChangeToken {
    /// Compose a token from the RFC 003 signal set: every ref's `(name, `RefState`/pointer id)`, plus
    /// the queued-patch count and its target ref (or `None` for an empty or unpublished queue).
    ///
    /// `refs` need not be pre-sorted — this always sorts by name internally before hashing, so a
    /// caller (or a future prikk version) returning the same refs in a different order can never look
    /// like a repository change. This is deliberate, not incidental: relying on a sort prikk merely
    /// happens to do today would make an upstream ordering change indistinguishable from a real one.
    #[must_use]
    pub fn compose<'a>(
        refs: impl IntoIterator<Item = (&'a str, &'a str)>,
        queued_patches: u64,
        queued_target: Option<&str>,
    ) -> Self {
        let mut sorted: Vec<(&str, &str)> = refs.into_iter().collect();
        sorted.sort_unstable_by_key(|(name, _)| *name);

        let mut hasher = DefaultHasher::new();
        sorted.len().hash(&mut hasher);
        for (name, id) in &sorted {
            name.hash(&mut hasher);
            id.hash(&mut hasher);
        }
        queued_patches.hash(&mut hasher);
        queued_target.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl std::fmt::Debug for ChangeToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChangeToken({:016x})", self.0)
    }
}

#[cfg(test)]
mod tests;

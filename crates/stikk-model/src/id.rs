//! Validated object-id and ref-name newtypes.
//!
//! Design `stikk-04` MOD-01. stikk never fabricates an identifier (data model INV-9): an object id
//! or ref name is either parsed from prikk's own output or rejected. These newtypes keep an
//! unvalidated string from being mistaken for a real identity, and provide the abbreviation the UI
//! shows with full-copy on focus (FR-124). They are deliberately permissive about *what* prikk's id
//! space is (stikk is not prikk's authority on identity) and strict only about the shape prikk
//! documents: a 64-character lowercase-hex object id, and a namespace-prefixed ref name.

use std::fmt;

use crate::error::StikkError;

/// A prikk object identifier: 64 lowercase hexadecimal characters (a SHA-256 digest in hex).
///
/// Validated on construction so a malformed string never travels the UI as if it were an identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(String);

impl ObjectId {
    /// Parse a 64-character lowercase-hex object id, rejecting any other shape.
    ///
    /// # Errors
    /// Returns [`StikkError::Environment`] when the string is not exactly 64 lowercase hex
    /// characters — a shape prikk would never emit, so encountering it means stikk misread prikk's
    /// output, which is an environment fault, not a user refusal.
    pub fn parse(value: &str) -> crate::error::Result<Self> {
        if value.len() != 64 {
            return Err(StikkError::environment_msg(format!(
                "object id must be 64 hex characters, got {}",
                value.len()
            )));
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(StikkError::environment_msg(
                "object id must be lowercase hexadecimal".to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// The full 64-character id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A short prefix for display; the UI pairs this with full-copy on focus (FR-124). Returns the
    /// whole id if it is somehow shorter than `len` (it never is after construction).
    #[must_use]
    pub fn abbreviated(&self, len: usize) -> &str {
        self.0.get(..len).unwrap_or(&self.0)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A prikk reference name, such as `heads/main`, `tags/v1`, or `remotes/heads/main`.
///
/// stikk keeps a *client-side focused ref* (design FR-055) as one of these — a preference, never a
/// HEAD. Validation here is deliberately light: prikk is the authority on whether a ref exists and
/// what it points to, so this only rejects the shapes that could not be a ref name at all (empty,
/// control characters), not the namespace policy prikk itself enforces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RefName(String);

impl RefName {
    /// Parse a ref name, rejecting only empty strings and control characters.
    ///
    /// # Errors
    /// Returns [`StikkError::Environment`] for an empty or control-character-bearing string.
    pub fn parse(value: &str) -> crate::error::Result<Self> {
        if value.is_empty() {
            return Err(StikkError::environment_msg("ref name must not be empty"));
        }
        if value.chars().any(char::is_control) {
            return Err(StikkError::environment_msg(
                "ref name must not contain control characters",
            ));
        }
        Ok(Self(value.to_string()))
    }

    /// The full ref name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when this is a received (`remotes/…`) ref — read-only in prikk until adopted, and
    /// labelled distinctly by the UI (FR-014).
    #[must_use]
    pub fn is_received(&self) -> bool {
        self.0.starts_with("remotes/")
    }
}

impl fmt::Display for RefName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;

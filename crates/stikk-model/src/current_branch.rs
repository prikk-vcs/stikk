//! prikk's current branch, as prikk's own `status` report names it (RFC 030 decision 1).
//!
//! From prikk 0.42, `.prikk/current-branch` names a default for `--ref` — *"a default, never an
//! authority"* in prikk's words, and not a HEAD. stikk never reads that file (`CON-1`); it reads the
//! `current branch:` line prikk prints in `status`, which the seam already parses.

use crate::RefName;

/// What prikk's `status` report says about its current branch.
///
/// **Three states, kept apart in the type** rather than folded into an `Option<String>` (`C-T2c′`):
/// "this prikk has no pointer", "prikk has a pointer it cannot resolve" and "prikk names this branch" are
/// three different facts, and the change token hashes each distinctly so no two of them can compose the
/// same token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CurrentBranch {
    /// Below prikk 0.42, which prints no `current branch:` line: there is no pointer to report.
    NotReported,
    /// prikk has a pointer it cannot resolve — malformed, or naming a branch that is missing or closed.
    /// Carries prikk's text after the label **verbatim** (`ER-02`), e.g.
    /// ``<unresolved; run `prikk doctor`>`` at prikk 0.42.0.
    Unresolved(String),
    /// The branch prikk's `--ref` default names, validated as a ref name at the parse boundary.
    Branch(RefName),
}

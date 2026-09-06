//! prikk version parsing and the validated-range gate (design SEAM-05, NFR-R03; RFC 009 decisions 6–7;
//! RFC 012 F-e).
//!
//! stikk targets prikk `>= 0.28`, validated through `0.33.0` (RFC 017, re-verified 2026-09-06 against a
//! real, **released** prikk 0.33.0 binary — every `cli_backend/parse/tests.rs` fixture shape re-run
//! against the same probe repositories and confirmed byte-identical to 0.32.0, not merely assumed
//! unchanged because RFC 017's stated scope did not name `parse.rs`; see that module's own doc for the
//! fourth re-verification paragraph). Unlike RFC 009/012/015's re-baselines, 0.33 changed no output
//! *shape* `cli_backend/parse/tests.rs` parses — its two message rewordings (`lock conflict:` →
//! `precondition not met:` on both messages our own letter reported) were already absorbed by design,
//! because the classifier matches the stable semantic clause, never the class prefix (RFC 017 F1). What
//! 0.33 actually cost was the classifier's own provenance: reading prikk's complete error taxonomy for
//! the first time (RFC 017 F0) found five matching arms keyed on text prikk has never emitted at any
//! validated version, dead since 0.1.0, plus one arm firing on a real precondition today and rendering
//! a gloss that contradicted prikk's own verbatim words beside it (F4, fixed in `stikk-core::present`).
//! The range has two ends that behave differently: below the floor stikk degrades to read-only, because
//! prikk's `worktree-status` is the UD-03 defect there and stikk already refuses to run it. **Above the
//! validated ceiling stikk still runs** — refusing every prikk newer than the last stikk release would
//! break users the day prikk ships a minor — but it says the range is unvalidated rather than silently
//! asserting knowledge it does not have. An unbounded upper range is exactly how RFC 009's F1–F4 defects
//! went unnoticed for three releases, and how RFC 017's five dead classifier arms went unnoticed for
//! four years: no version signal ever said "this has not actually been checked."

use stikk_model::{Result, StikkError};

/// The lowest prikk minor version stikk requires (`0.MIN.x`). Below it, `worktree-status` is the UD-03
/// defect and stikk already refuses to run it (RFC 008); 0.27.x is dropped rather than promising a
/// surface stikk cannot serve (RFC 009 decision 6).
const SUPPORTED_MAJOR: u32 = 0;
const SUPPORTED_MIN_MINOR: u32 = 28;
/// The highest prikk minor version stikk has actually validated against (RFC 009 decision 7; raised to
/// 31 by RFC 012 F-e, to 32 by RFC 015 §2/§8, then to 33 by RFC 017 §8 — each only after empirical
/// re-verification against a real released binary, never a changelog). RFC 017's re-verification found
/// no output-shape drift; it found the classifier's own provenance gap instead (`classify.rs`'s module
/// doc). A prikk above this still runs; [`Version::is_validated`] tells the caller to say so.
const VALIDATED_MAX_MINOR: u32 = 33;

/// A parsed semantic version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major component.
    pub major: u32,
    /// Minor component.
    pub minor: u32,
    /// Patch component.
    pub patch: u32,
}

impl Version {
    /// Parse a `prikk --version` line such as `prikk 0.27.1`. Tolerant of surrounding whitespace and
    /// a leading program name; strict about the `major.minor.patch` shape.
    ///
    /// # Errors
    /// [`StikkError::Environment`] when no `major.minor.patch` triple can be found — encountering
    /// that means stikk misread prikk's version output.
    pub fn parse_version_line(line: &str) -> Result<Self> {
        // Take the last whitespace-separated token that looks like a version triple.
        let candidate = line
            .split_whitespace()
            .rev()
            .find(|tok| tok.split('.').count() == 3)
            .ok_or_else(|| {
                StikkError::environment_msg(format!(
                    "could not find a version in prikk output: {line:?}"
                ))
            })?;
        let mut parts = candidate.split('.');
        let mut next = |what: &str| -> Result<u32> {
            let raw = parts.next().ok_or_else(|| {
                StikkError::environment_msg(format!("prikk version missing {what} component"))
            })?;
            // Strip any trailing non-digit suffix (e.g. a pre-release tag) from the patch.
            let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().map_err(|_| {
                StikkError::environment_msg(format!(
                    "prikk version {what} is not a number: {raw:?}"
                ))
            })
        };
        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// Whether this version is at or above the floor stikk requires (design NFR-R03; RFC 009 decision
    /// 6). `false` degrades the operation layer to read-only rather than misrender an unknown format.
    #[must_use]
    pub fn is_supported(self) -> bool {
        self.major == SUPPORTED_MAJOR && self.minor >= SUPPORTED_MIN_MINOR
    }

    /// Whether this version is within the range stikk has actually checked its output shapes against —
    /// at or above the floor **and** at or below the validated ceiling (RFC 009 decision 7). A
    /// `is_supported` prikk that is not `is_validated` still runs; the caller says its shapes are
    /// unverified rather than claiming a validation stikk has not done.
    #[must_use]
    pub fn is_validated(self) -> bool {
        self.is_supported() && self.minor <= VALIDATED_MAX_MINOR
    }
}

/// The validated ceiling as a display string (`"0.33"`), for UI copy that says what range stikk has
/// actually checked its shapes against. **Nothing else may hardcode this number**: a renderer that
/// copies it as a string literal instead of calling this drifts the moment the ceiling moves again —
/// exactly what RFC 015 found in `stikk-tui`'s own Orientation view, still reading "0.30" after RFC 012
/// F-e had already raised the ceiling to 31 and no one had touched the copy.
#[must_use]
pub fn validated_ceiling_display() -> String {
    format!("{SUPPORTED_MAJOR}.{VALIDATED_MAX_MINOR}")
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests;

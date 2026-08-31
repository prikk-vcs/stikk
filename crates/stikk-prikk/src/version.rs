//! prikk version parsing and the validated-range gate (design SEAM-05, NFR-R03).
//!
//! stikk v0.1 targets prikk `>= 0.27.x` behaviour, as established by the 2026 audit (requirement
//! ASM-2). A prikk outside that range is not refused outright — read surfaces where safe still work —
//! but mutation degrades and the version is shown, so stikk never silently misrenders an unknown
//! format.

use stikk_model::{Result, StikkError};

/// The lowest prikk minor version stikk v0.1 was validated against (`0.MIN.x`).
const SUPPORTED_MAJOR: u32 = 0;
const SUPPORTED_MIN_MINOR: u32 = 27;

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

    /// Whether this version is within the range stikk v0.1 was validated against (design NFR-R03).
    #[must_use]
    pub fn is_supported(self) -> bool {
        self.major == SUPPORTED_MAJOR && self.minor >= SUPPORTED_MIN_MINOR
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests;

//! The CLI backend — drives the `prikk` binary (design SEAM-01/SEAM-03, MOD-02).
//!
//! This is the v1 implementor of [`crate::Prikk`]. It honours the constraints the seam exists to
//! localize:
//! - **CON-1** — it uses only the public `prikk` command surface; it never reads `.prikk/` directly.
//! - **UD-04 (EPIPE guard)** — it reads prikk's stdout and stderr fully to end (via
//!   [`std::process::Command::output`]) before inspecting the exit, so prikk never receives EPIPE
//!   writing to a closed pipe.
//! - **UD-02 (parse containment)** — output parsing is confined here and refuses rather than
//!   fabricates on an unrecognized shape; a stray field is an environment fault, never a guess.
//! - **SEAM-06** — it passes prikk no key material; prikk reads its own environment when it signs.
//!
//! The program invoked defaults to `prikk` on `PATH`, overridable with `STIKK_PRIKK_BIN` for testing
//! against a specific build.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

use stikk_model::{Result, StikkError};

use crate::version::Version;
use crate::{Handshake, History, Orientation, Prikk, RefEntry, StateFiles};

mod parse;

/// A [`Prikk`] implementation that shells out to the `prikk` binary.
#[derive(Debug, Clone)]
pub struct CliBackend {
    program: OsString,
}

impl Default for CliBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CliBackend {
    /// Construct a backend using `prikk` on `PATH`, or the `STIKK_PRIKK_BIN` override if set.
    #[must_use]
    pub fn new() -> Self {
        let program =
            std::env::var_os("STIKK_PRIKK_BIN").unwrap_or_else(|| OsString::from("prikk"));
        Self { program }
    }

    /// Construct a backend that invokes an explicit program path (used in tests).
    #[must_use]
    pub fn with_program(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// Run `prikk` with `args`, optionally in `cwd`, draining both streams fully before classifying
    /// the exit. On a non-zero exit the combined message is classified into the error taxonomy.
    fn run<I, S>(&self, cwd: Option<&Path>, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.program);
        command.args(args);
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        // `output()` reads stdout and stderr to EOF, which is exactly the drain the EPIPE guard needs
        // (UD-04): stikk never closes prikk's stdout early, so prikk never panics writing to us.
        let output = command.output().map_err(|error| {
            StikkError::environment(
                format!(
                    "could not launch prikk ({}): is it installed and on PATH?",
                    self.program.to_string_lossy()
                ),
                error,
            )
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            return Ok(stdout);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(classify_failure(&stdout, &stderr))
    }
}

/// Classify a non-zero prikk exit into the presentation taxonomy (design UD-05: prikk collapses
/// distinct outcomes onto exit 1, so stikk classifies by message text and defaults to a refusal —
/// the safest class, since it never triggers a retry). The verbatim message is always preserved.
fn classify_failure(stdout: &str, stderr: &str) -> StikkError {
    let message = pick_message(stdout, stderr);
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("lock") && (lowered.contains("conflict") || lowered.contains("already")) {
        return StikkError::LockConflict { message };
    }
    if lowered.contains("could not") && lowered.contains("prikk") {
        return StikkError::environment_msg(message);
    }
    // Default: a semantic refusal. Preserving the verbatim message is mandatory (NFR-I03); the
    // operation layer adds a localized gloss beside it, never in place of it.
    StikkError::Refusal { message }
}

/// prikk writes errors to stderr as `error: <msg>`; prefer that, else fall back to stdout.
fn pick_message(stdout: &str, stderr: &str) -> String {
    let trimmed = stderr.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    stdout.trim().to_string()
}

impl Prikk for CliBackend {
    fn handshake(&self) -> Result<Handshake> {
        let raw = self.run(None, ["--version"])?;
        let raw_version = raw.trim().to_string();
        let version = Version::parse_version_line(&raw_version)?;
        Ok(Handshake {
            supported: version.is_supported(),
            version,
            raw_version,
        })
    }

    fn orientation(&self, repo: &Path) -> Result<Orientation> {
        let status = self.run(Some(repo), ["status"])?;
        parse::orientation(&status)
    }

    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<History> {
        let limit = limit.to_string();
        let out = self.run(
            Some(repo),
            ["log", "--ref", reff, "--limit", limit.as_str()],
        )?;
        parse::history(&out)
    }

    fn block_state(&self, repo: &Path, reff: &str) -> Result<StateFiles> {
        let out = self.run(Some(repo), ["checkout", "--patch-plan", "--ref", reff])?;
        parse::state_files(&out)
    }

    fn refs(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        let out = self.run(Some(repo), ["branch", "list", "--all"])?;
        parse::refs(&out)
    }
}

#[cfg(test)]
mod tests;

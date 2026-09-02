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

use stikk_model::{RequestCategory, Result, StikkError};

use crate::version::Version;
use crate::{Handshake, History, Orientation, Prikk, RefEntry, StateFiles};

mod classify;
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
    /// the exit. On a non-zero exit the combined message is classified into the error taxonomy, using
    /// `category` to disambiguate where the message text alone is ambiguous (design UD-05).
    fn run<I, S>(&self, cwd: Option<&Path>, category: RequestCategory, args: I) -> Result<String>
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
        Err(classify::classify(&stdout, &stderr, category))
    }
}

impl Prikk for CliBackend {
    fn handshake(&self) -> Result<Handshake> {
        let raw = self.run(None, RequestCategory::ReadHistory, ["--version"])?;
        let raw_version = raw.trim().to_string();
        let version = Version::parse_version_line(&raw_version)?;
        Ok(Handshake {
            supported: version.is_supported(),
            version,
            raw_version,
        })
    }

    fn orientation(&self, repo: &Path) -> Result<Orientation> {
        let status = self.run(Some(repo), RequestCategory::ReadHistory, ["status"])?;
        parse::orientation(&status)
    }

    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<History> {
        let limit = limit.to_string();
        let out = self.run(
            Some(repo),
            RequestCategory::ReadHistory,
            ["log", "--ref", reff, "--limit", limit.as_str()],
        )?;
        parse::history(&out)
    }

    fn block_state(&self, repo: &Path, reff: &str) -> Result<StateFiles> {
        let out = self.run(
            Some(repo),
            RequestCategory::ReadState,
            ["checkout", "--patch-plan", "--ref", reff],
        )?;
        parse::state_files(&out)
    }

    fn refs(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        let out = self.run(
            Some(repo),
            RequestCategory::ReadHistory,
            ["branch", "list", "--all"],
        )?;
        parse::refs(&out)
    }
}

#[cfg(test)]
mod tests;

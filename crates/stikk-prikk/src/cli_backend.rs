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
//! - **RFC 009 F6** — prikk 0.28 split its exit contract into `0` success / `1` operational failure /
//!   `2` usage error (a bad argument list, detected before any repository work). Exit `2` means *stikk*
//!   built a bad command, not that prikk refused something; it is never classified as prikk's voice.
//! - **SEAM-05 (handshake caching, RFC 010)** — the version probe runs at most once per backend
//!   instance, cached in a [`OnceLock`](std::sync::OnceLock), matching the design's "recorded at open"
//!   semantics: a session does not notice prikk being upgraded underneath it, which is correct.
//!
//! The program invoked defaults to `prikk` on `PATH`, overridable with `STIKK_PRIKK_BIN` for testing
//! against a specific build.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use stikk_model::{RequestCategory, Result, StikkError};

use crate::version::Version;
use crate::{Handshake, History, Orientation, Prikk, RefEntry, StateFiles, WorktreeStatus};

mod classify;
mod parse;

/// The exit code prikk uses for a usage error detected before any repository work — an unknown,
/// duplicate, or malformed argument (prikk 0.28+; RFC 009 F6). This is a bug in the command *stikk*
/// assembled, never prikk's semantic refusal, so it is routed to [`StikkError::Internal`] and must
/// never reach [`classify::classify`].
const USAGE_ERROR_EXIT: i32 = 2;

/// A [`Prikk`] implementation that shells out to the `prikk` binary.
#[derive(Debug, Clone)]
pub struct CliBackend {
    program: OsString,
    /// The version probe, run at most once per instance (SEAM-05; RFC 010). `OnceLock` is `Send + Sync`
    /// whenever its contents are, so this does not disturb the trait's `Send + Sync` bound.
    handshake_cache: OnceLock<Handshake>,
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
        Self {
            program,
            handshake_cache: OnceLock::new(),
        }
    }

    /// Construct a backend that invokes an explicit program path (used in tests).
    #[must_use]
    pub fn with_program(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            handshake_cache: OnceLock::new(),
        }
    }

    /// Run `prikk` with `args`, optionally in `cwd`, draining both streams fully before classifying
    /// the exit. On a non-zero exit the combined message is classified into the error taxonomy, using
    /// `category` to disambiguate where the message text alone is ambiguous (design UD-05) — unless
    /// the exit is prikk's usage-error code, which is a stikk bug, not prikk's voice (RFC 009 F6).
    fn run<I, S>(&self, cwd: Option<&Path>, category: RequestCategory, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|a| a.as_ref().to_os_string())
            .collect();
        let mut command = Command::new(&self.program);
        command.args(&args);
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
        if output.status.code() == Some(USAGE_ERROR_EXIT) {
            return Err(self.usage_error(&args, &stdout, &stderr));
        }
        Err(classify::classify(&stdout, &stderr, category))
    }

    /// Run `prikk`, draining both streams, and return `(stdout, stderr, success)` **without**
    /// classifying a non-zero exit. Used where a non-zero exit is a normal outcome the caller must
    /// interpret itself — `worktree-status` exits 1 for a *dirty* tree (RFC 008 finding 2 / UD-05). A
    /// usage-error exit (`2`) is still handled here, since it is never a normal outcome for any caller
    /// (RFC 009 F6).
    fn run_capturing<I, S>(&self, cwd: Option<&Path>, args: I) -> Result<(String, String, bool)>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|a| a.as_ref().to_os_string())
            .collect();
        let mut command = Command::new(&self.program);
        command.args(&args);
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
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
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if output.status.code() == Some(USAGE_ERROR_EXIT) {
            return Err(self.usage_error(&args, &stdout, &stderr));
        }
        Ok((stdout, stderr, output.status.success()))
    }

    /// Build the [`StikkError::Internal`] for a usage-error exit (RFC 009 F6): the repository was not
    /// touched (no repository work runs before prikk validates its arguments), so the fault-screen
    /// framing `present()` already gives `Internal` ("the repository was not touched") is accurate
    /// here. prikk's own message is kept — it names the bad argument — but the class is stikk's, never
    /// prikk's refusal voice.
    fn usage_error(&self, args: &[OsString], stdout: &str, stderr: &str) -> StikkError {
        let command = args
            .iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        StikkError::Internal {
            detail: format!(
                "stikk ran `{} {command}`, which prikk rejected as a usage error — a stikk bug, not \
                 prikk's refusal: {}",
                self.program.to_string_lossy(),
                classify::pick_message(stdout, stderr),
            ),
        }
    }
}

impl Prikk for CliBackend {
    fn handshake(&self) -> Result<Handshake> {
        if let Some(cached) = self.handshake_cache.get() {
            return Ok(cached.clone());
        }
        let raw = self.run(None, RequestCategory::ReadHistory, ["--version"])?;
        let raw_version = raw.trim().to_string();
        let version = Version::parse_version_line(&raw_version)?;
        let handshake = Handshake {
            supported: version.is_supported(),
            validated: version.is_validated(),
            version,
            raw_version,
        };
        // `OnceLock::set` losing a race is fine: both sides computed the same probe of the same
        // process, so whichever value ends up cached is equivalent. Ignore the "already set" case.
        let _ = self.handshake_cache.set(handshake.clone());
        Ok(handshake)
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

    fn worktree_status(&self, repo: &Path, reff: &str) -> Result<WorktreeStatus> {
        // `worktree-status` exits 1 for a *dirty* tree and 0 for a clean one, writing the report to
        // stdout either way (RFC 008 finding 2). Capture without classifying, and parse stdout
        // regardless of exit; only when stdout carries no report is the outcome a real failure.
        let (stdout, stderr, _success) =
            self.run_capturing(Some(repo), ["worktree-status", "--ref", reff])?;
        match parse::worktree_status(&stdout) {
            Ok(status) => Ok(status),
            Err(_shape) => Err(classify::classify(
                &stdout,
                &stderr,
                RequestCategory::WorktreeAnalysis,
            )),
        }
    }
}

#[cfg(test)]
mod tests;

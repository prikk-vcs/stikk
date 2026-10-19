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
//!   instance, cached in a [`std::sync::OnceLock`], matching the design's "recorded at open"
//!   semantics: a session does not notice prikk being upgraded underneath it, which is correct.
//!
//! The program invoked defaults to `prikk` on `PATH`, overridable with `STIKK_PRIKK_BIN` for testing
//! against a specific build.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use stikk_model::{Capability, ChangeToken, RequestCategory, Result, StikkError};

use crate::env;
use crate::version::Version;
use crate::{
    CommitResult, Handshake, History, Orientation, Prikk, RefEntry, SealResult, ShowReport,
    StateReport, WorktreeStatus,
};

mod classify;
mod parse;
mod parse_json;

/// The exit code prikk uses for a usage error detected before any repository work — an unknown,
/// duplicate, or malformed argument (prikk 0.28+; RFC 009 F6). This is a bug in the command *stikk*
/// assembled, never prikk's semantic refusal, so it is routed to [`StikkError::Internal`] and must
/// never reach [`classify::classify`].
const USAGE_ERROR_EXIT: i32 = 2;

/// prikk's own clause when it refuses a ref it treats as absent — the one string the RFC 034 repair
/// keys on.
///
/// **prikk 0.45.0 made `worktree-status --ref R` and `log --ref R` refuse when `R` has no published
/// history** (`precondition not met: ref R does not exist in this repository`, exit 1, measured at
/// 0.45.0 and 0.46.0; 0.44.0 and below report normally). That is **every** ref until a repository's
/// first `seal`, so the Changes view, History and commit's preview all refused on a new repository —
/// the path RFC 032's own increment shipped.
///
/// prikk changed this deliberately, for a ref that is genuinely absent. **An unpublished ref is not an
/// absent ref**, and prikk's own default path still agrees: **with no `--ref` at all both commands
/// report**, and the report names the ref it used (`"ref": "heads/main"`). So stikk retries without
/// `--ref` and **verifies from the report itself** that it got the ref it asked for
/// ([`CliBackend::worktree_status_without_ref`], [`CliBackend::history_without_ref`]).
///
/// **Matched as prikk's semantic clause, never on the `precondition not met:` prefix** — `classify.rs`'s
/// standing rule, and the reason prikk's class-word churn has never broken a match here.
///
/// **When prikk restores the behaviour** (stikk letter 015 asks it to), this constant, the two retry
/// methods and their two call sites retire together; nothing else depends on them. The repair stays as
/// long as stikk supports a prikk in the 0.45–0.46 band.
const ABSENT_REF_CLAUSE: &str = "does not exist in this repository";

/// Whether prikk's output carries [`ABSENT_REF_CLAUSE`]. prikk writes this refusal to stderr with an
/// empty stdout (measured at 0.45.0 and 0.46.0); both streams are read so the match does not depend on
/// which one a future prikk picks.
fn refuses_absent_ref(stdout: &str, stderr: &str) -> bool {
    let carries = |text: &str| text.to_ascii_lowercase().contains(ABSENT_REF_CLAUSE);
    carries(stdout) || carries(stderr)
}

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
    /// Whether this session's prikk offers `--format json` for `log`, `branch` and `tag` (RFC 026 §6).
    ///
    /// Reads the cached handshake, so the gate costs nothing after the first call. **The number lives
    /// in one place**, [`crate::JSON_REPORTS_FROM_MINOR`], which this gate and every view built on a
    /// JSON-only report read: a second copy in any call site is how `0.30` survived in `stikk-tui`'s
    /// Orientation copy after the ceiling had moved twice (RFC 015), and RFC 035 review v1 §2.2 caught
    /// this very constant being copied into `stikk-core` three weeks after it was written down here.
    ///
    /// # Errors
    /// Propagates the handshake's own error — a version stikk cannot read is not a version it can
    /// choose a flag for.
    fn reads_json(&self) -> Result<bool> {
        Ok(Prikk::handshake(self)?.version.minor >= crate::JSON_REPORTS_FROM_MINOR)
    }

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

    /// Run `prikk`, draining both streams, and return `(stdout, stderr, code)` — the exit **code**,
    /// not just whether it was zero.
    ///
    /// **For [`Prikk::verify`] and nothing else.** `verify` is the one command whose non-zero exit
    /// can carry a complete, valid report: a damaged repository exits 1 *and* prints the whole
    /// `verify-report-v1` (RFC 040 F2, measured). Telling that apart from a refusal needs the code
    /// itself — [`Self::run_capturing`] gives only a boolean, which cannot distinguish prikk's
    /// exit 1 from any other failure, and that distinction is the whole rule.
    ///
    /// **No other method may use this.** Every other command on this trait means one thing by a
    /// non-zero exit: prikk refused, or stikk has a bug. Reaching for the raw code elsewhere would
    /// be a caller deciding for itself which failures to believe, which is exactly what
    /// [`Self::run`]'s classification exists to prevent (UD-05). A usage-error exit (`2`) is still
    /// handled here, since it is never a normal outcome for any caller (RFC 009 F6).
    ///
    /// `code` is `None` only where the platform reports no code (a signal on Unix), which is not a
    /// refusal and not a report — the caller treats it as it treats any other unexpected exit.
    fn run_with_status<I, S>(
        &self,
        cwd: Option<&Path>,
        args: I,
    ) -> Result<(String, String, Option<i32>)>
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
        Ok((stdout, stderr, output.status.code()))
    }

    /// Run one ref-writing command, and keep what it printed (RFC 044 decision 1).
    ///
    /// The three operations differ only in their argument vector, and share everything that
    /// matters: the band, the readiness check, the category, and that **prikk's output is kept
    /// verbatim and never parsed for state** (§6 trap 2). One helper, so the three cannot drift
    /// in any of it.
    ///
    /// **Readiness is checked before prikk runs** (decision 2), the same fold [`Prikk::seal`]
    /// applies and for the same reason: adoption can change between orientation and this call.
    /// prikk's own signature refusal is the fallback, never the first thing a reader sees.
    ///
    /// `what` names the operation in that readiness message, in stikk's words.
    fn write_ref(&self, repo: &Path, args: &[&str], what: &str) -> Result<crate::RefWrite> {
        if Prikk::handshake(self)?.version.minor < crate::CURRENT_BRANCH_FROM_MINOR {
            return Ok(crate::RefWrite::Unavailable);
        }
        let readiness = Prikk::readiness(self, repo)?.readiness;
        if !Capability::derive(readiness).may_publish() {
            return Err(StikkError::NotReady {
                detail: format!("{what} needs MAINTAINER signing readiness"),
            });
        }
        let out = self.run(Some(repo), RequestCategory::Publication, args)?;
        Ok(crate::RefWrite::Done(printed_lines(&out)))
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
    /// The RFC 034 retry for [`Prikk::worktree_status`]: **the same command with the `--ref <reff>` pair
    /// removed**, and everything else identical. The report is accepted **only when it names `reff`**.
    ///
    /// `None` on every other outcome — the retry refuses, its report does not parse, or it names another
    /// ref — and the caller then returns prikk's original refusal. **Showing a report for a ref the user
    /// did not ask about is the failure this must never have** (`T-T4`), which is why the report's own
    /// `ref` is the check: it costs no extra call and it cannot race. **Nothing consults `refs()` or
    /// `current_branch`** to decide this (RFC 034 Handoff A §3), and the retry is never cached.
    fn worktree_status_without_ref(
        &self,
        repo: &Path,
        reff: &str,
        prikk_minor: u32,
    ) -> Option<WorktreeStatus> {
        let (stdout, _stderr, _success) = self
            .run_capturing(Some(repo), ["worktree-status", "--format", "json"])
            .ok()?;
        let status = parse_json::worktree_status(&stdout, prikk_minor).ok()?;
        (status.reff == reff).then_some(status)
    }

    /// The RFC 034 retry for [`Prikk::history`]. See [`Self::worktree_status_without_ref`] for why the
    /// report's own ref is the only thing that may accept it.
    fn history_without_ref(&self, repo: &Path, reff: &str, limit: &str) -> Option<History> {
        let out = self
            .run(
                Some(repo),
                RequestCategory::ReadHistory,
                ["log", "--limit", limit, "--format", "json"],
            )
            .ok()?;
        let history = parse_json::history(&out).ok()?;
        (history.reff == reff).then_some(history)
    }
}

impl Prikk for CliBackend {
    fn readiness(&self, repo: &Path) -> Result<crate::ReadinessReport> {
        // The three bands (RFC 026 §4), chosen from the cached handshake. **One number, one place** —
        // the `0.30`-in-two-places lesson (RFC 015) is why this is not repeated at any call site.
        const KEY_STATUS_FROM: u32 = 41;
        const KEY_DIRECTORY_FROM: u32 = 40;

        let minor = Prikk::handshake(self)?.version.minor;
        let read_only = env::read_only_override();
        // Established by presence, on every band — it is the answer at ≤ 0.39 and the stale-variable
        // warning above it (`key-status-v1` has no `legacy_variable_set`; measured, not read).
        let stale = env::stale_seed_variables();

        if minor < KEY_DIRECTORY_FROM {
            let readiness = env::read_readiness(read_only);
            return Ok(crate::ReadinessReport {
                readiness,
                // On this band the seed variables are the mechanism, so being set is not stale.
                author: crate::RoleDetail::default(),
                maintainer: crate::RoleDetail::default(),
            });
        }

        if minor < KEY_STATUS_FROM {
            // **0.40 exactly** (Q1, ruled (b)): seeds moved to a key directory and nothing shipped to
            // ask about them until 0.41. stikk reports unknown, names the cause and names the fix,
            // rather than probing — the probe would need its own parse and its own fixtures, re-verified
            // at every re-baseline from now on, to serve one version that was current for a day.
            return Ok(crate::ReadinessReport {
                readiness: stikk_model::Readiness {
                    author: stikk_model::RoleReadiness::Unverifiable,
                    maintainer: stikk_model::RoleReadiness::Unverifiable,
                    read_only,
                },
                author: crate::RoleDetail {
                    stale_seed_variable: stale.author,
                    ..crate::RoleDetail::default()
                },
                maintainer: crate::RoleDetail {
                    stale_seed_variable: stale.maintainer,
                    ..crate::RoleDetail::default()
                },
            });
        }

        // ≥ 0.41: prikk answers. One spawn covers both roles — `key status` without `--role` reports
        // every role in one `roles` array, measured on a real 0.41.0.
        let out = self.run(
            Some(repo),
            RequestCategory::ReadState,
            ["key", "status", "--format", "json"],
        )?;
        let rows = parse_json::key_status(&out)?;
        let role = |want: &str| rows.iter().find(|(name, _, _)| name == want);
        let build = |want: &str, stale_set: bool| {
            role(want).map_or_else(
                || {
                    // A role prikk did not report is absent, never assumed ready.
                    (
                        stikk_model::RoleReadiness::NotReady,
                        crate::RoleDetail {
                            stale_seed_variable: stale_set,
                            ..crate::RoleDetail::default()
                        },
                    )
                },
                |(_, detail, facts)| {
                    let state = if facts.usable {
                        stikk_model::RoleReadiness::Known(facts.binding)
                    } else {
                        stikk_model::RoleReadiness::NotReady
                    };
                    let mut detail = detail.clone();
                    detail.stale_seed_variable = stale_set;
                    (state, detail)
                },
            )
        };
        let (author_state, author_detail) = build("author", stale.author);
        let (maintainer_state, maintainer_detail) = build("maintainer", stale.maintainer);
        Ok(crate::ReadinessReport {
            readiness: stikk_model::Readiness {
                author: author_state,
                maintainer: maintainer_state,
                read_only,
            },
            author: author_detail,
            maintainer: maintainer_detail,
        })
    }

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
        // RFC 030 decision 1: only the version tells a prikk with no current-branch pointer (< 0.42)
        // from a report that lost its `current branch:` line (≥ 0.42) — the handshake is cached.
        let prikk_minor = Prikk::handshake(self)?.version.minor;
        parse::orientation(&status, prikk_minor)
    }

    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<History> {
        let limit = limit.to_string();
        // RFC 026 §6: JSON where prikk offers it, prose at the floor. The gate is here, beside the
        // flag it adds, rather than inside either parser — a parser that had to know the version
        // would be two parsers in one function.
        if self.reads_json()? {
            let refused = match self.run(
                Some(repo),
                RequestCategory::ReadHistory,
                [
                    "log",
                    "--ref",
                    reff,
                    "--limit",
                    limit.as_str(),
                    "--format",
                    "json",
                ],
            ) {
                Ok(out) => return parse_json::history(&out),
                Err(refused) => refused,
            };
            // RFC 034 decision 1, exactly as in `worktree_status` above. `run` has already classified
            // this; prikk's absent-ref refusal matches no arm and so degrades to `Refusal` carrying
            // prikk's verbatim message (`classify.rs`'s documented default), which is where the clause
            // is read from.
            if let StikkError::Refusal { message } = &refused
                && refuses_absent_ref("", message)
                && let Some(history) = self.history_without_ref(repo, reff, limit.as_str())
            {
                return Ok(history);
            }
            return Err(refused);
        }
        // The prose path needs no RFC 034 repair, for the reason `worktree_status` gives: prose is read
        // only below prikk 0.39, and the refusal starts at 0.45.
        let out = self.run(
            Some(repo),
            RequestCategory::ReadHistory,
            ["log", "--ref", reff, "--limit", limit.as_str()],
        )?;
        parse::history(&out)
    }

    fn block_state(
        &self,
        repo: &Path,
        reff: &str,
        block_id: &str,
        is_tip: bool,
    ) -> Result<StateReport> {
        // RFC 037 decision 1. The gate is here, beside the command it chooses, exactly as
        // `history` and `queue` put theirs — a parser that had to know the version would be two
        // parsers in one function.
        if Prikk::handshake(self)?.version.minor >= crate::TREE_FROM_MINOR {
            // **The block's own id, never `reff`** (§7 trap 1). A ref moves between the moment the
            // user opened a block and this read; the block does not. Asking with `reff` would
            // answer about the tip for every row in the lineage, which is the defect this whole
            // increment exists to remove — and it would do it silently, since the report would
            // parse.
            let out = self.run(
                Some(repo),
                RequestCategory::ReadState,
                ["tree", "--ref", block_id, "--format", "json"],
            )?;
            return parse_json::tree(&out, block_id).map(StateReport::Listed);
        }
        // Below the band prikk can replay only to a tip, so that is the only block with an answer.
        if !is_tip {
            return Ok(StateReport::Unreported);
        }
        let out = self.run(
            Some(repo),
            RequestCategory::ReadState,
            ["checkout", "--patch-plan", "--ref", reff],
        )?;
        parse::state_files(&out).map(StateReport::Listed)
    }

    fn diff(&self, repo: &Path, comparison: &crate::Comparison) -> Result<crate::DiffReport> {
        // RFC 038 decision 1. The gate is here, beside the command it chooses, as every other
        // banded read in this file puts it.
        if Prikk::handshake(self)?.version.minor < crate::DIFF_FROM_MINOR {
            return Ok(crate::DiffReport::Unreported);
        }
        // **The bare form is the bare command.** `diff` with no `--from` and no `--to` is the one
        // way to ask for "the worktree against what `commit` would author against" — the sealed tip
        // plus the queue, a state no point names, so there is nothing to spell instead.
        let args: Vec<&str> = match comparison {
            crate::Comparison::WorktreeAgainstBaseline => vec!["diff", "--format", "json"],
            crate::Comparison::Points { from, to } => {
                vec!["diff", "--from", from, "--to", to, "--format", "json"]
            }
        };
        let out = self.run(Some(repo), RequestCategory::ReadState, args)?;
        parse_json::diff(&out, comparison).map(crate::DiffReport::Listed)
    }

    fn verify(&self, repo: &Path) -> Result<crate::VerifyReport> {
        // RFC 040 decision 1. The gate sits beside the command it chooses, as every other banded
        // read in this file puts it.
        if !self.reads_json()? {
            return Ok(crate::VerifyReport::Unreported);
        }
        let (stdout, stderr, code) =
            self.run_with_status(Some(repo), ["verify", "--format", "json"])?;

        // **The one place a non-zero exit can be success** (RFC 040 F2, measured at 0.51 on a
        // flipped container byte and a torn WAL record: exit 1, `ok: false`, and every stage still
        // reported). The rule is deliberately narrow -- only exit 1, and only when stdout really is
        // a `verify-report-v1`.
        match code {
            Some(0) => parse_json::verify(&stdout).map(crate::VerifyReport::Listed),
            Some(1) => match parse_json::verify(&stdout) {
                // A report. stderr only repeats the failing condition, so it is not read.
                Ok(report) => Ok(crate::VerifyReport::Listed(report)),
                // **prikk printed a report and stikk could not read it** (review v1 §2). That is
                // stikk's environment error, exactly as it is at exit 0 -- never a refusal. A
                // future prikk could add a stage status this reader rejects, or print an `ok`
                // disagreeing with its own conditions, on a repository that really is damaged;
                // calling that *prikk refused* would be the one mistake this whole method exists
                // to prevent, and it would do it while prikk's condition sat unread in stderr.
                //
                // The question asked is only whether prikk *printed* a report, never whether
                // stikk can read it -- the reader has already answered that, and its error is the
                // one that names the broken rule.
                Err(error) if parse_json::announces_verify_report(&stdout) => Err(error),
                // **Not a report at all, so it is a refusal** -- an absent or locked repository,
                // say -- and it classifies exactly as every other refusal does, in prikk's own
                // words. The parse error is dropped on purpose here: reporting "stikk could not
                // read the report" would bury prikk's actual reason for a reader who needs it.
                Err(_) => Err(classify::classify(
                    &stdout,
                    &stderr,
                    RequestCategory::ReadState,
                )),
            },
            // Exit 2 never reaches here (`run_with_status` turns it into stikk's bug), and any
            // other code is as unexpected for `verify` as for anything else.
            _ => Err(classify::classify(
                &stdout,
                &stderr,
                RequestCategory::ReadState,
            )),
        }
    }

    fn refs(&self, repo: &Path) -> Result<crate::BranchList> {
        if self.reads_json()? {
            let out = self.run(
                Some(repo),
                RequestCategory::ReadHistory,
                ["branch", "list", "--all", "--format", "json"],
            )?;
            // The minor goes to the reader because the band, not the field's absence, decides
            // whether prikk can name an unpublished current branch (RFC 037 decision 3).
            return parse_json::refs(&out, Prikk::handshake(self)?.version.minor);
        }
        let out = self.run(
            Some(repo),
            RequestCategory::ReadHistory,
            ["branch", "list", "--all"],
        )?;
        // **The prose band is below 0.39, and the field arrived at 0.47**, so nothing here could
        // report it even if this parser looked: not reported is the only honest answer.
        Ok(crate::BranchList {
            refs: parse::refs(&out)?,
            unpublished_current: crate::UnpublishedBranch::NotReported,
        })
    }

    fn tags(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        if self.reads_json()? {
            let out = self.run(
                Some(repo),
                RequestCategory::ReadHistory,
                ["tag", "list", "--format", "json"],
            )?;
            return parse_json::tags(&out);
        }
        let out = self.run(Some(repo), RequestCategory::ReadHistory, ["tag", "list"])?;
        parse::tags(&out)
    }

    fn worktree_status(&self, repo: &Path, reff: &str) -> Result<WorktreeStatus> {
        // `worktree-status` exits 1 for a *dirty* tree and 0 for a clean one, writing the report to
        // stdout either way (RFC 008 finding 2). Capture without classifying, and parse stdout
        // regardless of exit; only when stdout carries no report is the outcome a real failure.
        //
        // RFC 027 decision 2: JSON at ≥ 0.39, where each entry carries commit's own verdict. The exit
        // handling is the same on both paths, deliberately — the JSON report is written on a dirty
        // exit exactly as the prose one is (measured at 0.41).
        if self.reads_json()? {
            // RFC 034 decision 4: the declaration verdict is read only inside its band, and the reader
            // is told which prikk produced the report — the handshake is cached.
            let prikk_minor = Prikk::handshake(self)?.version.minor;
            let (stdout, stderr, _success) = self.run_capturing(
                Some(repo),
                ["worktree-status", "--ref", reff, "--format", "json"],
            )?;
            // **A document that parses is a report** (RFC 027 B, review C1). When stdout is JSON and the
            // reader rejects it — a rule broken, an unknown schema — that is stikk refusing prikk's
            // report, a version-skew condition (`UD-02`), and it surfaces as the reader's own
            // environment error naming the rule. Sent to `classify` instead, it fell to a `Refusal`
            // that prefers stderr: prikk's `worktree has changes against the baseline` on a dirty tree,
            // the raw report on a clean one — the validation invisible exactly when it fired. Only
            // when stdout carries no JSON document at all is the outcome classified, as before.
            return match parse_json::worktree_status(&stdout, prikk_minor) {
                Ok(status) => Ok(status),
                Err(rejected) if crate::json::parse(&stdout).is_ok() => Err(rejected),
                Err(_no_report) => {
                    // RFC 034 decision 1: on prikk ≥ 0.45 this is where a ref with no published history
                    // lands, since prikk refuses it outright ([`ABSENT_REF_CLAUSE`]). Retry only on
                    // prikk's own clause, and keep prikk's original refusal unless the retry's report
                    // names the ref that was asked for — an absent ref (a typo) refuses either way.
                    if refuses_absent_ref(&stdout, &stderr)
                        && let Some(status) =
                            self.worktree_status_without_ref(repo, reff, prikk_minor)
                    {
                        return Ok(status);
                    }
                    Err(classify::classify(
                        &stdout,
                        &stderr,
                        RequestCategory::WorktreeAnalysis,
                    ))
                }
            };
        }
        // The prose path needs no RFC 034 repair: stikk reads JSON from prikk 0.39 (`reads_json`), and
        // the refusal it repairs starts at 0.45, so a prose-era prikk never emits it.
        let (stdout, stderr, _success) =
            self.run_capturing(Some(repo), ["worktree-status", "--ref", reff])?;
        // RFC 030: only the version tells "no rename declarations exist" (< 0.38) from a report that
        // lost its `live rename declarations:` section (≥ 0.38) — the handshake is cached.
        let prikk_minor = Prikk::handshake(self)?.version.minor;
        match parse::worktree_status(&stdout, prikk_minor) {
            Ok(status) => Ok(status),
            Err(_shape) => Err(classify::classify(
                &stdout,
                &stderr,
                RequestCategory::WorktreeAnalysis,
            )),
        }
    }

    fn queue(&self, repo: &Path) -> Result<crate::QueueReport> {
        // RFC 028 decision 1: the enumeration at ≥ 0.39, the JSON band stikk already reads and the suite's
        // ceiling exercises. `status-report-v1` exists from 0.35, but 0.35–0.38 is a band nobody runs.
        if self.reads_json()? {
            let out = self.run(
                Some(repo),
                RequestCategory::ReadHistory,
                ["status", "--format", "json"],
            )?;
            // The message rule is version-banded; the handshake is cached.
            let prikk_minor = Prikk::handshake(self)?.version.minor;
            return Ok(crate::QueueReport::Listed(parse_json::queue(
                &out,
                prikk_minor,
            )?));
        }
        // Below 0.39 there is no enumeration to ask for: the same prose `status` read Orientation makes,
        // and nothing else. The list is unreported, never empty.
        let orientation = self.orientation(repo)?;
        Ok(crate::QueueReport::Unreported {
            count: orientation.queued_patches,
            target: orientation.queued_target,
        })
    }

    fn show(&self, repo: &Path, id: &str) -> Result<ShowReport> {
        // RFC 035 decision 1, in `queue`'s pattern: the JSON band stikk already reads, and the one
        // place 0.39 is written down. `show-report-v1` exists from 0.36, but 0.36–0.38 is a band
        // stikk reads no JSON in at all, and the owner ruled it unoccupied.
        if !self.reads_json()? {
            // Nothing is spawned: there is no enumeration to ask for, and the view says in stikk's own
            // voice that it reads a patch's content from 0.39 on (`C-T4d`).
            return Ok(ShowReport::Unreported);
        }
        let out = self.run(
            Some(repo),
            RequestCategory::ReadHistory,
            ["show", id, "--format", "json"],
        )?;
        Ok(ShowReport::Listed(parse_json::show(&out)?))
    }

    fn commit(&self, repo: &Path, reff: &str, message: &str) -> Result<CommitResult> {
        // OPL-04's seam-side half (handoff §7). **It now catches what it was written for.** Until
        // RFC 026 this re-check read environment presence, which cannot change underneath a running
        // process — the comment here said so and apologised for it. Against `key status` the answer is
        // genuinely live: a key rebound, revoked, or a seed file removed between the preview and this
        // call changes it, and prikk is the one computing it. One spawn on a human-confirmed action is
        // the price of the check meaning something.
        //
        // `Capability::derive` rather than a bare readiness read, so `STIKK_READ_ONLY=1` is honoured
        // here too — the same fold `confirm::capability_gate` applies.
        let readiness = Prikk::readiness(self, repo)?.readiness;
        if !Capability::derive(readiness).may_author() {
            return Err(StikkError::NotReady {
                detail: "commit needs AUTHOR signing readiness".to_string(),
            });
        }
        // No `--text-edits`: prikk's own source documents it as a no-op kept for compatibility (RFC
        // 014 F1) — offering a control that changes nothing would be a small dishonesty.
        let out = self.run(
            Some(repo),
            RequestCategory::QueueMutation,
            ["commit", "--from-worktree", "--ref", reff, "-m", message],
        )?;
        parse::commit(&out)
    }

    fn change_token(&self, repo: &Path) -> Result<ChangeToken> {
        // RFC 003 decision 3, corrected by review C1: the ref half is `refs()` **and** `tags()`,
        // deduplicated by name — not `refs()` alone. RFC 012 F3 established that `prikk branch list
        // --all`'s tag coverage is unspecified (its ref-pointer index carries no namespace filter, so a
        // tag may or may not appear there depending on prikk's incidental behaviour); composing from
        // `refs()` alone would make this token's own coverage of the tag namespace nondeterministic —
        // worse than deliberately narrow, for the primitive every 0.4.0 mutation preview will be gated
        // on. Deduplicating (the same merge `stikk_core::list_refs` performs) is what makes the token
        // the same regardless of which way prikk's unspecified behaviour goes: a tag counted once via
        // `refs()`'s leak or once via `tags()` composes identically either way.
        //
        // **prikk's current branch is in** (RFC 030 decision 2), from the `status` read this already
        // makes — no new spawn. A terminal `prikk branch switch` at ≥ 0.42 moves no ref, tag or queue, so
        // nothing else here would see it.
        //
        // **The worktree marker stays excluded from the token** (RFC 003): it would cost a
        // `worktree-status` spawn per token, and a view built from `worktree-status` re-reads on refresh.
        // That premise holds for views and not for confirmation, where nothing re-read — so RFC 030 moved
        // the worktree check to **commit's confirmation** (`stikk_core::commit`), one spawn per confirmed
        // commit rather than one per token.
        // **The unpublished current branch is deliberately not in the token** (RFC 037). The one
        // transition it has — a branch becoming published — adds that branch to the ref list in
        // the same report, which already moves the token; and prikk refuses `branch switch` to a
        // branch that does not exist (measured at 0.51.0), so the field cannot change under a
        // session without the list changing too. A second source for a fact already covered would
        // cost a spawn and detect nothing.
        let branches = self.refs(repo)?.refs;
        let tags = self.tags(repo)?;
        let orientation = self.orientation(repo)?;
        let merged = branches
            .iter()
            .filter(|entry| !tags.iter().any(|tag| tag.name == entry.name))
            .chain(tags.iter());
        Ok(ChangeToken::compose(
            merged.map(|entry| (entry.name.as_str(), entry.id.as_str())),
            orientation.queued_patches,
            orientation.queued_target.as_deref(),
            &orientation.current_branch,
        ))
    }

    fn seal(&self, repo: &Path, reff: &str) -> Result<SealResult> {
        // OPL-04's seam-side half, the same fold `commit` applies (RFC 014 handoff §7) — and, since
        // RFC 026, against prikk's own `key status` rather than environment presence. Adoption is
        // exactly the thing that can change out from under a running session: a maintainer key adopted
        // or revoked between orientation and this call now changes the answer, which is what this
        // re-check was written for and could not do until prikk shipped the surface (RFC 016 F3).
        let readiness = Prikk::readiness(self, repo)?.readiness;
        if !Capability::derive(readiness).may_publish() {
            return Err(StikkError::NotReady {
                detail: "seal needs MAINTAINER signing readiness".to_string(),
            });
        }
        // `--allow-no-audit` unconditionally, every time (RFC 016 F1) — there is no configuration for
        // this and no code path that omits it: prikk refuses without it as a usage error (exit 2),
        // which per RFC 014 F6 is a stikk bug, never a user-facing refusal.
        let out = self.run(
            Some(repo),
            RequestCategory::Publication,
            ["seal", "--allow-no-audit", "--ref", reff],
        )?;
        parse::seal(&out)
    }

    fn create_branch(&self, repo: &Path, name: &str, from_ref: &str) -> Result<crate::RefWrite> {
        // **`--from`, always** (RFC 044 §6 trap 1): the user chose this ref, and prikk's default
        // is the current branch, which is a different question.
        self.write_ref(
            repo,
            &["branch", "create", name, "--from", from_ref],
            "create a branch",
        )
    }

    fn create_tag(
        &self,
        repo: &Path,
        name: &str,
        target: &str,
        message: &str,
    ) -> Result<crate::RefWrite> {
        let mut args = vec!["tag", "create", name, "--target", target];
        // **No `-m` at all for an empty or blank message.** This is prikk's rule, not stikk's:
        // `tag create -m "   "` is `error: tag message must not be empty`, exit 2 — a *usage*
        // error, which would reach a reader as stikk's Internal fault framing (measured at
        // 0.52.0; `prikk-cli/src/tag.rs:280`). A tag with no message is prikk's own shape for a
        // tag that carries none, and `trim` is what keeps a blank one from becoming an exit 2.
        if !message.trim().is_empty() {
            args.push("-m");
            args.push(message);
        }
        self.write_ref(repo, &args, "create a tag")
    }

    fn merge_plan(
        &self,
        repo: &Path,
        baseline: &str,
        left: &str,
        right: &str,
    ) -> Result<crate::MergePlan> {
        if Prikk::handshake(self)?.version.minor < crate::CURRENT_BRANCH_FROM_MINOR {
            return Ok(crate::MergePlan::Unavailable);
        }
        let out = self.run(
            Some(repo),
            RequestCategory::ReadState,
            [
                "merge-plan",
                "--baseline-block",
                baseline,
                "--left-ref",
                left,
                "--right-ref",
                right,
            ],
        )?;
        let printed = printed_lines(&out);
        Ok(crate::MergePlan::Read {
            outcome: evidence_outcome(&printed)?,
            printed,
        })
    }

    fn merge(
        &self,
        repo: &Path,
        baseline: &str,
        into: &str,
        from: &str,
    ) -> Result<crate::MergeResult> {
        // **The band, the MAINTAINER check and the `Publication` spawn are `write_ref`'s, the
        // same code and not a copy of it.** A merge writes a ref, at the same tier, needing the
        // same key; two gates for one requirement is two things that can drift apart, and the
        // one that drifts is always the one with fewer tests. Only the type differs, because a
        // merge is not one of RFC 044's three ref writes to the layer above.
        let printed = self.write_ref(
            repo,
            &[
                "merge",
                // **Exactly as seal passes it** (decision 1). Without it prikk exits 2, a usage
                // error -- stikk's own bug class, never prikk's refusal voice.
                "--allow-no-audit",
                "--baseline-block",
                baseline,
                "--into",
                into,
                "--from",
                from,
            ],
            "merging",
        )?;
        Ok(match printed {
            crate::RefWrite::Done(lines) => crate::MergeResult::Done(lines),
            crate::RefWrite::Unavailable => crate::MergeResult::Unavailable,
        })
    }

    fn switch_branch(&self, repo: &Path, name: &str) -> Result<crate::SwitchResult> {
        // **Banded before anything else** (RFC 045 decision 4): below the band nothing is
        // spawned, not even the readiness read.
        if Prikk::handshake(self)?.version.minor < crate::DIFF_FROM_MINOR {
            return Ok(crate::SwitchResult::Unavailable);
        }
        // **Read-write, and no key** (decision 2). prikk needs none (F1, measured), so stikk asks
        // for none — the requirement is `Materialization`'s, read from the category itself.
        if !Prikk::readiness(self, repo)?.readiness.may_operate() {
            return Err(StikkError::NotReady {
                detail: "switching branch is not available in read-only mode".to_string(),
            });
        }
        // **Both streams.** At exit 0 prikk can name a snapshot fallback on stderr, and that line
        // belongs on the result card beside the counts, not merged into them.
        let (stdout, stderr, ok) = self.run_capturing(Some(repo), ["branch", "switch", name])?;
        if !ok {
            return Err(classify::classify(
                &stdout,
                &stderr,
                RequestCategory::Materialization,
            ));
        }
        Ok(crate::SwitchResult::Done {
            printed: printed_lines(&stdout),
            noted: printed_lines(&stderr),
        })
    }

    fn close_branch(&self, repo: &Path, name: &str) -> Result<crate::RefWrite> {
        self.write_ref(repo, &["branch", "close", name], "close a branch")
    }
}

/// The one line a merge plan is read for (RFC 047 decision 2): `evidence outcome: <word>`.
///
/// **Exactly one, and a word stikk knows.** Missing, repeated, or unknown is an environment
/// error, never a guess (`UD-02`) — this line decides whether stikk offers the merge at all, so
/// reading it wrong would offer one prikk refuses, or hide one it would allow. Every other line
/// of the plan is shown verbatim and parsed by nothing.
///
/// **Repeated is an error, not "take the first".** prikk prints it once today; two would mean
/// the plan's shape has changed under stikk, and picking one of them would be stikk deciding
/// which verdict to believe.
fn evidence_outcome(printed: &[String]) -> Result<crate::MergeOutcome> {
    const LABEL: &str = "evidence outcome:";
    let mut found = printed
        .iter()
        .filter_map(|line| line.trim().strip_prefix(LABEL))
        .map(str::trim);
    let Some(word) = found.next() else {
        return Err(StikkError::environment_msg(
            "prikk's merge plan carries no `evidence outcome:` line; stikk will not guess at the \
             verdict",
        ));
    };
    if found.next().is_some() {
        return Err(StikkError::environment_msg(
            "prikk's merge plan carries more than one `evidence outcome:` line; stikk will not \
             choose which verdict to believe",
        ));
    }
    crate::MergeOutcome::parse(word).ok_or_else(|| {
        StikkError::environment_msg(format!(
            "prikk's merge plan reports an outcome stikk does not know: {}",
            {
                let mut escaped = String::new();
                crate::escape::escape_into(&mut escaped, word);
                escaped
            }
        ))
    })
}

/// Every non-empty line of one of prikk's streams, in order, trailing whitespace trimmed.
///
/// **Blank lines are the shell's, not prikk's**, and a result card should not show them. Nothing
/// else is done to the text: these lines are prikk's own words, kept for a reader and never
/// parsed for state (RFC 044 §6 trap 2; RFC 045 §6 trap 3).
fn printed_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests;

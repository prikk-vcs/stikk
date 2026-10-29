//! prikk version parsing and the validated-range gate (design SEAM-05, NFR-R03; RFC 009 decisions 6–7;
//! RFC 012 F-e).
//!
//! stikk targets prikk `>= 0.28`, validated through `0.54.1` — **RFC 052**, the 0.54 re-baseline:
//! prikk 0.54 fixes a read stikk shows (`show <block-id>` no longer refuses after consecutive
//! text edits of one file, so Block and Patch detail open where 0.53 showed prikk's refusal
//! verbatim) and adds a `verify` count line and an `extract` command stikk does not call; the
//! suite was green at 0.28.0 and 0.54.1 with only this constant moved and no leg changed.
//! **0.54.1, not 0.54.0**: prikk withdrew 0.54.0's Windows installer and asks people not to use
//! it, so 0.54.1 is the binary measured and the version every live statement names.
//!
//! Before it **RFC 049**, the 0.53 re-baseline:
//! prikk 0.53's one output change is to `checkout`'s refusing modes, and stikk calls `checkout`
//! only below [`TREE_FROM_MINOR`]; the suite was green at 0.28.0 and 0.53.0 with only this
//! constant moved and no leg changed.
//!
//! Before it **RFC 043**, the 0.52 re-baseline:
//! prikk 0.52.0 fixed the defect stikk reported in letter 020 — `show <block-id>` resolves each
//! sealed patch's node-addressed path **at that patch** now, not at the block's end, so the case
//! that reported `z.txt` reports `a.txt` and the case that reported no path reports `d.txt`
//! (measured at 0.52.0). The raise came first and alone, and the suite was red on exactly the one
//! leg that pinned 0.51's answer. [`PATCH_PATH_AT_PATCH_FROM_MINOR`] bands Patch detail's
//! *as at the end of this block* label below 0.52, where it is still true.
//!
//! Before it **RFC 035**, the 0.51 re-baseline, which
//! exists to build Patch detail against prikk's `show --format json`: the raise came first, and on its
//! own it cost nothing — the suite passed 37 of 37 at 0.28.0 and 0.51.0 with only this constant moved.
//! What 0.51 changed for stikk is elsewhere and was read deliberately: `show <block-id>` resolved each
//! sealed patch's node-addressed path **at the block's end**, not at that patch (RFC 035 A1 F7), so a
//! two-patch block could name a path the first patch never saw. Across 0.47–0.51 the queued form
//! gained the paths it used to withhold (0.47, `### Changed`: a `path` appears where one was absent),
//! which is why no reader here may key on a field's absence. The previous ceiling was `0.46.0`
//! (**RFC 034**, the 0.46 re-baseline:
//! prikk 0.45.0 made `worktree-status --ref R` and `log --ref R` refuse for a ref with no published
//! history — every ref until a repository's first seal — and `cli_backend` repairs that by retrying
//! without `--ref` and verifying the report names the ref asked for. **This raise came first**, as the
//! rule below requires: it is what let the suite measure the regression it repairs. The previous
//! ceiling was `0.42.0` (**RFC 029**, the 0.42 re-baseline: the
//! real-binary suite passed unchanged at 0.28 and 0.42 on every platform, and the fixtures were
//! re-verified against the 0.42 binary **except those that record a state 0.42 cannot produce**:
//! `LOG_FIXTURE`, whose 0.30.0 history carries no patch messages, which 0.42 writes on every commit;
//! `LOG_FIXTURE_STRADDLING`, which needs patches authored by 0.31.1 and 0.32.0 in one block; and the
//! seven RFC 027 captures (`WORKTREE_UNSUPPORTED_*`, `WORKTREE_SYMLINK_*`, `WORKTREE_QUEUED_JSON_0_41`),
//! which record 0.28 and 0.41 by design. The one prose surface stikk reads that 0.42 changed is
//! `status`, which gained a `current branch:` line — read into `Orientation` since RFC 030. See
//! `cli_backend/parse/tests.rs` for what each version changed).)
//!
//! The previous ceiling was `0.41.0` (**RFC 026**, the 0.41 re-baseline — raised from 0.38 across three
//! releases, 0.39 through 0.41, one of which changed how prikk is configured at all: 0.40 moved signing
//! seeds out of the environment and into a key directory, which the real-binary suite's own fixture
//! builder had to be rebuilt around before it could measure anything).
//!
//! Before it, `0.38.0` (**RFC 021**, raised from 0.33 across five releases at once and verified by RFC
//! 019's real-binary suite rather than by hand).
//!
//! The previous ceiling, for the history the raises form: `0.33.0` (RFC 017, re-verified 2026-09-06
//! against a real, **released** prikk 0.33.0 binary — every `cli_backend/parse/tests.rs` fixture shape
//! re-run against the same probe repositories and confirmed byte-identical to 0.32.0, not merely assumed
//! unchanged because RFC 017's stated scope did not name `parse.rs`). Unlike RFC 009/012/015's re-baselines, 0.33 changed no output
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
/// 31 by RFC 012 F-e, to 32 by RFC 015 §2/§8, to 33 by RFC 017 §8, to 38 by RFC 021, to 41 by RFC 026,
/// to 42 by RFC 029, to 46 by RFC 034, to 51 by RFC 035, to 52 by RFC 043, to 53 by RFC 049, to 54
/// by RFC 052, then to **55 by RFC 055** — each only after empirical re-verification against a
/// real released binary, never a changelog). RFC 017's re-verification found no output-shape
/// drift; it found the classifier's own provenance gap instead (`classify.rs`'s module doc). A
/// prikk above this still runs; [`Version::is_validated`] tells the caller to say so.
///
/// **The raise comes early in a re-baseline, not last.** The real-binary suite (RFC 019, `TS-07`)
/// refuses to run against a binary this constant does not name, so raising it is what lets the suite
/// report what the raise cost; re-capturing fixtures first would leave it nothing to find. RFC 021
/// established that order across five releases at once; **RFC 026 had to put one step before it** —
/// the suite's fixture builder configures prikk through the environment, and prikk 0.40 stopped
/// reading it, so the harness had to work before the raise could be measured at all. RFC 035 is the
/// first raise made **to build a view on the new version's report** rather than to absorb drift, and
/// the order held for the same reason: the suite would not have run `show` at 0.51 at all until this
/// constant named it.
///
/// **55 means prikk 0.55.0.** Two of 0.55's changes come anywhere near stikk, and neither cost a
/// leg (RFC 055 F2). `extract` now refuses a destination **inside** the repository: stikk does not
/// call `extract` at all yet, and the increment that will refuses that destination in its own words
/// before prikk is asked. A damaged bundle's refusal is worded differently — *"unexpected end of
/// bundle: failed to fill whole buffer"* where 0.54 said *"unexpected end of record"* — which stikk
/// shows verbatim and parses none of; the suite's truncated-bundle leg asserts only *"malformed
/// persisted data"*, which both versions say. Everything else 0.55 changed is a command or a
/// library surface stikk does not touch. **The suite was green at 0.28.0 and 0.55.0 with only this
/// constant moved and no leg changed.**
///
/// The band before it, for the history the raises form: **54 meant prikk 0.54.1, and the patch was
/// the measured one** — prikk withdrew 0.54.0's Windows installer, which refused some ordinary x64
/// machines, so only 0.54.1 was ever run (RFC 052 F3). What 0.54 changed that stikk shows is prikk's
/// own fix: `show <block-id>` no longer refuses after consecutive text edits of one file, so Block
/// and Patch detail open where 0.53 showed prikk's refusal verbatim.
const VALIDATED_MAX_MINOR: u32 = 55;

/// The first prikk minor whose reports stikk reads as **JSON** (prikk 0.39, where `log`, `branch` and
/// `tag` gained `--format json`; RFC 026 §6).
///
/// **The number lives here and nowhere else.** `CliBackend::reads_json` gates every JSON read on it,
/// and it is also the floor of every *view* built on a JSON-only report — Patch detail's content
/// (`FR-030`, RFC 035) among them. A second copy is how `0.30` survived in `stikk-tui`'s Orientation
/// copy after the ceiling had moved twice (RFC 015), and how this constant's own two copies agreed
/// right up until one of them would have moved (RFC 035 review v1 §2.2).
///
/// **It is not every JSON report's own floor**, and must not be read as one: `status --format json`
/// exists from prikk 0.35 and `show --format json` from 0.36. What this names is the floor *stikk*
/// reads at, which the project owner has ruled is the only band that matters — no user is on
/// 0.36–0.38, and stikk reads no JSON from prikk there at all.
pub const JSON_REPORTS_FROM_MINOR: u32 = 39;

/// The first prikk minor that lists **any block's** files: prikk 0.46, where `tree [--ref
/// <ref|block-id>] [--format json]` arrived with the `tree-listing-v1` schema (RFC 037 F1).
///
/// **Measured, not read from a changelog**: 0.45.0 answers `error: unknown command: tree`, and 0.46.0
/// and 0.51.0 both report the same shape for the same two-block lineage. Below this, prikk replays
/// only to a ref's tip, so an older block's files are **not reported** — never an empty list
/// (`C-T2c′`).
///
/// **The number lives here and nowhere else**, for the reason [`JSON_REPORTS_FROM_MINOR`] gives: the
/// seam gates its command on it, and the one view built on that report says the same number in the
/// sentence it shows below the band.
pub const TREE_FROM_MINOR: u32 = 46;

/// The first prikk minor that compares two points: prikk 0.46, where `diff [--from <a>] [--to <b>]
/// [--format json]` arrived with the `diff-report-v1` schema (RFC 038 decision 1).
///
/// **The same release as [`TREE_FROM_MINOR`], and a separate constant on purpose**: they are two
/// surfaces that happen to have shipped together, and folding them into one number would make a
/// later divergence silent. Measured at 0.46.0 and 0.51.0, **identical in every case once ids are
/// normalised**.
///
/// Below it, Compare is unavailable and stikk says so in its own voice (`C-T4d`) — **no prose
/// `diff` is parsed, at any version**.
pub const DIFF_FROM_MINOR: u32 = 46;

/// The first prikk minor that names an **unpublished current branch**: prikk 0.47, where
/// `branch-list-v1` gained the nullable top-level `unpublished_current_branch` field (RFC 037 F3).
///
/// **Measured**: at 0.46.0 the field is absent from the report entirely; at 0.51.0 it is
/// `"heads/main"` in a fresh repository and still after a commit is queued, and `null` once the
/// first seal publishes the branch into `branches[]`.
///
/// Below this band the field is **not reported**, which is not the same fact as `null` — one says
/// this prikk cannot answer, the other says prikk answered "none" (`C-T2c′`). A reader that keyed on
/// the field's absence would read the two as one, which is why the band decides and absence never
/// does.
pub const UNPUBLISHED_BRANCH_FROM_MINOR: u32 = 47;

/// The first prikk minor that resolves a **sealed patch's node-addressed path at that patch**
/// rather than at its block's end: prikk 0.52, which fixed the defect stikk reported in letter 020
/// (RFC 035 A1 F7, A2; RFC 043 F2).
///
/// **Measured at 0.52.0**, against the two cases RFC 035 found. In a sealed block of two patches
/// where patch 2 renames the file patch 1 edited, `show <block-id>` reports patch 1's path as
/// `a.txt` -- the name that patch saw -- where 0.51 reported `z.txt`, the name a later patch gave
/// it. Where patch 2 *deletes* the file, patch 1 now reports `d.txt` where 0.51 reported no path
/// at all. The queued and sealed forms agree from here on.
///
/// Below this band the path prikk reports is the one at the block's end, which can be a name that
/// patch never saw -- so Patch detail labels it *as at the end of this block*
/// ([`stikk_core`'s `AS_AT_BLOCK_END`]). **At and above it the label would be false**, and stikk
/// shows the path unlabelled. The label is banded rather than deleted because 0.39-0.51 are still
/// in the supported range and the sentence stays true there.
///
/// [`stikk_core`'s `AS_AT_BLOCK_END`]: https://docs.rs/stikk-core
pub const PATCH_PATH_AT_PATCH_FROM_MINOR: u32 = 52;

/// The first prikk minor that keeps a **current branch** pointer: prikk 0.42, whose `status`
/// prints `current branch:` (RFC 030 decision 1).
///
/// **It bands the ref-writing operations too** (RFC 044 decision 3). Creating a branch or a tag
/// and closing a branch exist further back than this — prikk 0.28 has all three — but closing
/// one safely does not: prikk lets you close **the branch you are on**, and the repository is
/// then on a closed branch, where a default-source `branch create` refuses (RFC 044 F3, measured
/// at 0.52.0). stikk declines to offer that, and it can only know which branch you are on from
/// 0.42. **One constant for both facts**, rather than a second literal that could drift from it.
pub const CURRENT_BRANCH_FROM_MINOR: u32 = 42;

/// The first prikk minor stikk reads **bundles** at: prikk 0.53, the only version measured
/// (RFC 050 decision 6).
///
/// `bundle export`, `verify`, `preview` and `import` exist further back, but **nothing below
/// 0.53 has been measured**, and `bundle preview --format json` -- the one of the four that
/// speaks JSON, and the only one stikk reads rather than shows -- is what stikk's reading of a
/// bundle is built on. No user is on an older band, so the band is set where the evidence is
/// rather than guessed backwards. Below it stikk spawns nothing and answers *unavailable*, as
/// [`CURRENT_BRANCH_FROM_MINOR`] does for the ref writes.
pub const EXCHANGE_FROM_MINOR: u32 = 53;

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

/// The validated ceiling as a display string (`"0.42"` at the time of writing), for UI copy that says
/// what range stikk has actually checked its shapes against. **Nothing else may hardcode this number**:
/// a renderer that copies it as a string literal instead of calling this drifts the moment the ceiling
/// moves again — exactly what RFC 015 found in `stikk-tui`'s own Orientation view, still reading "0.30"
/// after RFC 012 F-e had already raised the ceiling to 31 and no one had touched the copy.
///
/// *(This doc comment's own example said `"0.33"` until RFC 021's sweep caught it — the function whose
/// whole purpose is to stop the number being copied had a stale copy of it in its own documentation.
/// Illustrative text drifts the same way renderer text does; it is just harder to notice, because
/// nothing renders it.)*
#[must_use]
pub fn validated_ceiling_display() -> String {
    format!("{SUPPORTED_MAJOR}.{VALIDATED_MAX_MINOR}")
}

/// The supported/validated minor-version bounds themselves (`SUPPORTED_MIN_MINOR`,
/// `VALIDATED_MAX_MINOR`), for tooling that needs the numbers rather than a yes/no answer or a display
/// string. Everything *inside* stikk keeps using [`Version::is_supported`]/[`Version::is_validated`]/
/// [`validated_ceiling_display`] — this exists for what is outside stikk: the real-binary integration
/// suite's CI workflow reads it (via `examples/print_version_matrix.rs`) to decide which two prikk
/// versions to install, so raising or lowering either bound here widens or narrows what that suite
/// exercises without anyone having to remember a second place to update (`TS-07`; RFC 019 §6).
#[must_use]
pub fn supported_minor_range() -> (u32, u32) {
    (SUPPORTED_MIN_MINOR, VALIDATED_MAX_MINOR)
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests;

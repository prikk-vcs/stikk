# Handoff A — RFC 072: checks follow their causes, and the release steps are written once (v1)

**Companion to:** [RFC 072](../../accepted/072-checks-follow-their-causes.md), **accepted 2026-11-25**. **Read the
RFC first**: F1–F3 and decisions 1–4. **Covers:** all of it. **0.23.0's increments 4 and 5.** **Not a release.**

> **What this increment is.** The supply-chain check starts running on the change that can make it fail. The three
> release steps that are copied (the installer fill, the Unix archive packaging, and the workspace-version read) are
> written once, as scripts that both the release and its tests call.
>
> **Nothing the release produces changes,** and the trial's artifacts prove it byte for byte.
>
> **Sanctioned:**
> - `.github/workflows/`: `supply-chain.yml`, `release.yml`, `release-trial.yml`, `build-release.yml` and
>   `installers.yml`;
> - a new `.github/scripts/`;
> - `docs/src/contributing/releasing.md` and `CHANGELOG.md`.
>
> Anything else: stop and report. **No branch.**

---

## 0. Evidence plan

| Question | Level | Evidence in the request, or in the delivery report |
|---|---|---|
| Do the scripts do exactly what the copies did? | local | each script run on this machine beside the step it replaces, on the same inputs, with the outputs compared by `sha256sum` (§2.4) |
| Are the release's outputs unchanged? | CI, **after approval** | `release-trial.yml` at the pushed SHA, against **the 0.22.0 tag's trial run `45512394810`**, whose artifacts are kept until 2026-12-01: the eight installer files equal by `sha256sum`; for `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`, the archive listings equal and the `.sha256` line shapes equal |
| Does the check follow the dependencies? | CI, after the push | the push itself touches `supply-chain.yml`, so it starts the check: show the run. The next push that touches no dependency file starts no check: show `gh run list --workflow supply-chain.yml` |
| Do the installers still install? | CI | `installers.yml` at the pushed SHA (the push starts it via `.github/scripts/**`): every leg |
| Does the guard still read the version? | local | `.github/scripts/workspace-version.sh` prints `0.22.0`. Its next real test is 0.23.0's tag |
| Gates | local | **all eight**, and the local suite |

**Before approval the workflows can only be linted.** Run `actionlint` if it is available. If not, say so, as at RFC
066.

## 1. Commit 1: the supply-chain check follows the dependencies

`supply-chain.yml`:
1. **`on:`**
   - `push`, with `branches: [main]` and `paths:` `Cargo.lock`, `**/Cargo.toml`, `deny.toml` and
     `.github/workflows/supply-chain.yml`;
   - `workflow_dispatch`.

   **No `schedule:`, no `pull_request`.**
2. **`permissions: contents: read`** at the top.
3. **The header comment is rewritten.** Keep it short and true:
   - why a dependency change is the trigger: a red run then always names its cause;
   - that every prep still dispatches it at the SHA to be tagged, as the release gate;
   - that new advisories between releases come from GitHub's Dependabot vulnerability alerts;
   - that **RFC 020 Q1 was answered by the owner on 2026-11-25: no schedule** (RFC 072).

## 2. Commit 2: the release steps, written once

1. **`.github/scripts/workspace-version.sh`:** prints the `stikk` package's version. The body is the guard's
   `cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="stikk") | .version'`.
   - `release.yml`'s guard calls it, and **its comparison and its outputs stay exactly as they are**;
   - `release-trial.yml`'s first job calls it.
2. **`.github/scripts/fill-installers.sh <out-dir> <stikk-version> <prikk-floor> <prikk-version> <file>...`:** for each
   `scripts/<file>`:
   - writes `<out-dir>/<file>` with the three `sed` substitutions as they are written today;
   - `chmod +x` for the `.sh` files;
   - **fails, naming the file, if any placeholder survives.**

   `build-release.yml`'s installer job calls it, and keeps its own `.sha256` step after it. `installers.yml`'s three
   fill steps call it too: the shell scripts, the PowerShell scripts (in `bash`, as today) and the glibc leg.
3. **`.github/scripts/package-unix.sh <version> <target> <binary>`:**
   - makes `stikk-<version>-<target>/` holding the binary, README.md, LICENSE and CHANGELOG.md;
   - makes the `.tar.gz`, and its `.sha256` with `sha256sum`, or `shasum -a 256` where that is missing;
   - prints the two file names, one per line.

   `build-release.yml`'s *Package (unix)* step calls it, and sets `ASSET` and `ASSET_SHA` from what it prints.
   `installers.yml`'s glibc leg calls it **with the published version's name, as its comment explains** (its two
   versions). **Keep that comment.**
4. **Prove each script against the step it replaces, before any workflow uses it.** On this machine, run the old
   `sed` loop and the new script on the same inputs, then the old packaging lines and the new script on one built
   binary. Compare the filled files and the archives' listings by `sha256sum`, and show the outputs.
5. **Scripts are POSIX `sh`,** with `set -eu`, and no Bash-only syntax. The Windows legs run them through `bash`, as
   they do today.
6. **The Windows *Package (windows)* step stays where it is,** written once in PowerShell. `installers.yml` downloads
   the published zip and never packages one (RFC 072 decision 2).

## 3. Commit 3: the triggers and the text

1. **`release-trial.yml`'s and `installers.yml`'s `paths:`** gain `.github/scripts/**`.
2. **Each comment that says *"the way build-release.yml does"* or *"exactly as …"* (`installers.yml`)** now names the
   script it calls.
3. **`releasing.md`:** where it lists what a prep runs, say that the supply-chain check also runs on every dependency
   change, and still gates the release.
4. **`CHANGELOG.md`, `## Unreleased`, `### Changed`:** the RFC's line.

## 4. The traps

1. **No change to what is produced.** If a filled installer's `sha256sum` differs from the 0.22.0 trial's, stop and
   report, unless the only difference is the version (it is still 0.22.0, so there should be none).
2. **The guard's behaviour does not change.** It compares the tag with the version and stops the release on a
   mismatch, exactly as now.
3. **No placeholder check is lost.** `build-release.yml` refused a surviving placeholder, and so did one of
   `installers.yml`'s legs. The script refuses it for every caller.
4. **`.github/scripts/`, not `scripts/`.** `scripts/` holds the shipped installer templates, and helpers do not belong
   beside them.

## 5. Submit (no branch)

**Run:**
- all eight gates (none exempted);
- the local suite at 0.28.0 + 0.58.0;
- `actionlint` if available;
- §2.4's comparisons.

**Package it to** `.git-exclude/review-request/072-checks-follow-their-causes-a/review-request-v1.md`.

**After approval:**
1. Push.
2. Show the supply-chain run and the `installers.yml` run that the push starts.
3. Dispatch `release-trial.yml` at the pushed SHA, and compare it with `45512394810` as in §0.
4. Report every run and leg.

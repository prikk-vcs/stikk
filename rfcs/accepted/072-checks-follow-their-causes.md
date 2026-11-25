# RFC 072 — Checks follow their causes, and the release steps are written once

**Status.** **Accepted by the project owner 2026-11-25**; [Handoff A](../handoffs/072-checks-follow-their-causes/a-checks-and-shared-steps-handoff-v1.md). **Proposed 2026-11-25** by the architect, as 0.23.0's increments 4 and 5
(`.git-exclude/release/0-23-0-scope.md`, authorized by the project owner 2026-11-25). **No open question.**
**Measured** on `main` at `baed649` (the 0.22.0 tag).
**Tracks.**
- RFC 020, whose Q1 (a schedule for the supply-chain check) **the owner answered on 2026-11-25: no schedule, a trigger
  that follows the cause**;
- RFC 066, whose F3 (copies drift) left two copies behind;
- RFC 068.

**Touches.**
- `.github/workflows/`: `supply-chain.yml`, `release.yml`, `release-trial.yml`, `build-release.yml` and
  `installers.yml`;
- a new `.github/scripts/`;
- `docs/src/contributing/releasing.md`.

## Summary

**Two checks are triggered by the wrong thing, or by nothing, and two release steps are written more than once.**
1. **The supply-chain check runs only when dispatched.** A dependency change can bring in an advisory, and nothing
   would say so until the next release prep.
2. **The installer fill and the archive packaging** are written in `build-release.yml` and copied in
   `installers.yml`, which tests the installers with them. **The workspace-version command** is written in
   `release.yml`'s guard and copied in `release-trial.yml`. A copy tests itself, not the release (RFC 066 F3).

This RFC runs the supply-chain check on the change that can cause a finding, and keeps it as the prep's gate. It
writes each release step once, as a script both workflows call.

## Findings

### F1 — what can make the supply-chain check fail, and when

| Cause | When it happens | Covered today by |
|---|---|---|
| **A dependency change brings in a flagged crate** | only when `Cargo.lock`, a `Cargo.toml` or `deny.toml` changes | the next prep's dispatched run: **late, and blamed on nothing** |
| **A new advisory against a crate already in the tree** | any day | **every prep's run, which refuses the release** (RFC 068); between releases, **GitHub's Dependabot vulnerability alerts**, enabled on the repository (`vulnerability-alerts`: 204, 2026-11-25) |

**The owner's ruling, 2026-11-25:** no weekly schedule. Its run would be blamed on a commit that did not cause its
failure, and it would mail the owner with nothing to act on, while no users are waiting on a fix.

### F2 — what `supply-chain.yml` says about itself

- **The header justifies dispatch only:** *"a gate that fails for an upstream reason overnight … must not redden an
  unrelated PR."* With a `paths:` trigger the run is never unrelated, because the push changed the dependencies.
- **It names Q1 as open,** which it no longer is.
- **It declares no `permissions:`.** It runs with the repository's default token, measured as `read`. That is safe
  today, but it comes from a setting, not from the file.

### F3 — the copies

| Step | Written in | Copied in |
|---|---|---|
| Fill the installers' three placeholders, and refuse any that survive | `build-release.yml:222–227` | `installers.yml:80`, `:306`, `:581` |
| Package a Unix archive: the directory, binary, README, LICENSE, CHANGELOG, `tar`, `.sha256` | `build-release.yml`'s *Package (unix)* | `installers.yml:593–615` |
| Read the workspace version | `release.yml`'s guard | `release-trial.yml`'s first job |

The copies agree today; RFC 066's review checked them by eye. **Nothing keeps them agreeing.**

## Decisions

1. **`supply-chain.yml` runs on a push to `main` that touches `Cargo.lock`, `**/Cargo.toml`, `deny.toml` or the
   workflow itself, and on dispatch.**
   - No schedule, no pull-request trigger.
   - It keeps its place as a prep requirement: every prep dispatches it at the SHA to be tagged.
   - Its header is rewritten to say this, and that RFC 020 Q1 is answered (RFC 072).
   - **`permissions: contents: read`** at the top.
2. **The release steps are written once, in `.github/scripts/`,** small POSIX shell scripts, each called by every
   workflow that needs the step:
   - **`workspace-version.sh`** prints the `stikk` package's version (the guard's `cargo metadata | jq` command).
     `release.yml`'s guard and `release-trial.yml` call it;
   - **`fill-installers.sh <out-dir> <stikk-version> <prikk-floor> <prikk-version> <file>...`** fills, sets the mode,
     and **fails if a placeholder survives**. `build-release.yml` and `installers.yml` call it, for both the shell
     scripts and the PowerShell ones;
   - **`package-unix.sh <version> <target> <binary>`** makes the archive and its `.sha256`, with `sha256sum` or
     `shasum -a 256`, as today, and prints the two file names. `build-release.yml` and `installers.yml` call it.

   **The Windows packaging step stays in PowerShell in `build-release.yml`,** where it is written once already.
   `installers.yml` does not package a zip; the handoff checks that.
3. **The triggers that watch these files gain `.github/scripts/**`:** `release-trial.yml` (RFC 066 decision 2) and
   `installers.yml`. A change to a shared step is then tried by both of its users.
4. **What the scripts produce is unchanged byte for byte,** proved by the release trial's artifacts before and after:
   - the four filled installers and their `.sha256` files are identical;
   - an archive's listing is identical. An archive's own hash may differ, because of timestamps (RFC 066's comparison).

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does a dependency change start the check, and does nothing else? | CI, after the push | the push that adds the trigger touches the workflow and starts it. A later documentation-only push does not, shown by `gh run list` |
| Are the filled installers unchanged? | CI | `release-trial.yml` dispatched at the base and at the head: the eight installer files' `sha256sum` equal, file by file |
| Are the archives unchanged in shape? | CI | the two runs' `.tar.gz` listings and `.sha256` line shapes equal, for one Linux and one macOS target |
| Do the installers still install? | CI | `installers.yml` dispatched at the head: every leg |
| Does the guard still guard? | local | `workspace-version.sh` prints `0.22.0` at `baed649`. The guard's comparison is unchanged; its next real test is 0.23.0's tag |
| Gates | local | all eight, and the local suite |

## Delivery

**One handoff, after acceptance.**

## Changelog

`### Changed` (for contributors): *the supply-chain check runs whenever the dependencies change, and still gates
every release; the installer fill, the archive packaging and the version read are each written once, as a script
the release and its tests share.*

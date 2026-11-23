# RFC 066 — Every release build is tried before a tag

**Status.** **Done (implemented) 2026-11-24** at `fc2fd95`, shipping in 0.22.0; see *Delivered* below. **Accepted by the project owner 2026-11-24**; [Handoff A](../handoffs/066-every-release-build-is-tried-first/a-release-trial-handoff-v1.md). **Proposed 2026-11-24** by the architect, as 0.22.0's increment 1 (`.git-exclude/release/0-22-0-scope.md`,
authorized by the project owner 2026-11-24). **No open question.**
**Measured** from `release.yml` at `e214d53` (the 0.21.1 tag) and from runs `45103941837` (0.21.0, cancelled) and
`45177444241` (0.21.1, green).
**Tracks.** RFC 011 (release prep); RFC 039 and RFC 048 (the installers and the binary matrix); the 0.21.1 prep's
runner-label check, which this RFC replaces.
**Touches.** `.github/workflows/`: `release.yml`, one new reusable workflow, and one new dispatchable workflow. The
release-prep handoff's checklist.

## Summary

**A release's build machines are exercised only by its tag.** 0.21.0 found that out at the worst time: GitHub had
retired the `macos-14` image, both macOS jobs died before their first step, and the installers and the AUR push never
ran (run `45103941837`). 0.21.1's prep added a manual check, a recent green run for every label. **A check that runs
is better than one that is remembered.**

This RFC moves `release.yml`'s build into one reusable workflow, and adds a **trial run** that calls the very same
build with publishing off. Each release prep runs it, and it also runs once a week by itself. A retired machine, a
broken packaging step or a toolchain change is then found days before a tag, not during one.

## Findings

### F1 — what the tag run does, and which steps publish

`release.yml` has seven jobs:
- **`guard`:** the tag must equal the workspace version.
- **`verify`:** fmt, clippy and test.
- **`release`:** creates the GitHub Release. **It publishes.**
- **`crates`:** **publishes to crates.io.**
- **`binaries`:** a 7-target matrix that builds, packages, **attests** and **uploads**.
- **`installers`:** fills the four scripts' placeholders, checksums them, and **uploads**.
- **`aur`:** calls `aur.yml`, which **pushes**.

**What can be tried without publishing anything:** the build and packaging steps of `binaries`, and the filling and
checksumming steps of `installers`. Attesting, uploading, crates.io and the AUR push cannot be.

### F2 — the build machines, and how each is exercised today

| Label | Release targets | Exercised outside a tag? |
|---|---|---|
| `ubuntu-latest` | x86_64 gnu, x86_64 musl; guard, verify, release, crates, installers | yes: CI on every push |
| `ubuntu-24.04-arm` | aarch64 gnu, aarch64 musl | **no**: only `release.yml` names it |
| `macos-latest` | x86_64 and aarch64 darwin (since 0.21.1) | yes: the real-binary suite and installers, when dispatched |
| `windows-latest` | x86_64 msvc | yes: the real-binary suite and installers, when dispatched |

**Even where a label is exercised elsewhere, the release's own steps are not**: its targets, its musl tools, its
packaging and its archive layout are run only by a tag.

### F3 — copies drift

A trial that **copies** the build steps would test the copy. A second job definition is the kind of second source the
owner's philosophy rules out, and it is how 0.21.0's pinned label would have survived a copy that said `macos-latest`.

## Decisions

1. **One build, defined once.** `release.yml`'s `binaries` and `installers` jobs move into a new reusable workflow,
   **`.github/workflows/build-release.yml`** (`on: workflow_call`). It takes two inputs:
   - **`version`**, a string, the version the archives carry;
   - **`publish`**, a boolean.

   **With `publish: true`** it does exactly what the two jobs do today: build, package, attest, upload to the tag's
   GitHub Release, then fill, checksum and upload the installers. **With `publish: false`** it builds, packages,
   fills and checksums, attests nothing, and uploads the results **as workflow artifacts** only. That is enough for
   someone to check that the archive layout and the checksum lines are right.

   `release.yml` calls it with `publish: true` and the guarded version. **`release.yml`'s other jobs do not change.**
2. **The trial run: `.github/workflows/release-trial.yml`.** It calls `build-release.yml` with `publish: false` and
   the workspace version, read from `Cargo.toml` as `guard` reads it, with no tag. It runs:
   - **on `workflow_dispatch`**, which every release prep uses;
   - **weekly on a schedule**, so a retired image is found within a week whether or not a release is near. GitHub
     Actions minutes are free for a public repository;
   - **on a push that touches `build-release.yml`, `release.yml`, `release-trial.yml`, or `scripts/**`**, the
     installer templates.

   It has **no** write permission, no `id-token`, and no access to any secret.
3. **The trial and the release cannot disagree about a target:** the matrix lives only in `build-release.yml`. A
   target added for a release is tried the next time the trial runs, with no second edit.
4. **Every release prep dispatches the trial on `main`, and reports its run** with every leg, at the SHA to be tagged.
   **This replaces the 0.21.1 prep's manual label table.** A red leg stops the cut, as a red suite leg does.
5. **The workflows' permissions stay least-privilege.**
   - `build-release.yml` asks for `contents: write`, `id-token: write` and `attestations: write` only on its publish
     path. A reusable workflow takes its permissions from the caller's job, so `release.yml` grants them and
     `release-trial.yml` grants none.
   - The trial's artifacts are kept for 7 days.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does the trial build every target on its real runner? | CI | one dispatched `release-trial.yml` run on `main`: every leg green, and each target's archive and `.sha256` present as an artifact |
| Are the trial's archives the ones a release would publish? | CI, by inspection | download two artifacts (one `.tar.gz`, the Windows `.zip`): the same layout as `0.21.1`'s published archives (file names, contents), and the checksum line's shape |
| Does a release still publish exactly as before? | the next tag | 0.22.0's own release run: 22 files, the checksums, the attestations, the installers, the AUR. **Verified by the architect after the tag, as for every release** |
| Can the trial publish anything? | review | the trial's job grants no write permission, and the publish steps are gated on `publish` |

**Not tried, and why:** a planted retired label. The trial and the release share one job definition (decision 3), so a
label that fails one fails the other. Proving it would mean pushing a broken workflow to `main`.

## Delivery

**One handoff, after acceptance.** It touches only `.github/workflows/`, and the release-prep checklist line in the
next prep handoff.

## Changelog

`### Changed` (for contributors): every release build is tried before a tag, and weekly. This is the lesson of 0.21.0,
whose release run could not build for macOS.

## Delivered

**On `main` at `fc2fd95`, 2026-11-24**, in four commits: `3ec1ac3`, `f8ff32d`, `e4ead73` and `fc2fd95`. Handoff A was
reviewed by the architect and approved with two text fixes.

**What was delivered:**
- **The build is defined once, in `build-release.yml`.** `release.yml`'s `binaries` and `installers` jobs moved there
  verbatim. Its seven edits: `needs` and `permissions` moved to the caller; the version and the tag come from
  `inputs.version`; three publish steps are gated on `inputs.publish`; two artifact steps run when publishing is off.
  `release.yml` calls the file through one `build` job that holds the three write permissions, and `aur` now needs
  `build`.
- **`release-trial.yml`** runs on a dispatch, every Monday at 04:17 UTC, and on a push that touches the release
  workflows or `scripts/**`. It has `contents: read` only, and no secrets.
- **The text:**
  - `CHANGELOG.md`'s `### Changed` line;
  - `releasing.md`: every prep runs the trial, a red leg stops the cut, and from 0.22.0 the attestations are signed by
    `build-release.yml`;
  - `installers.yml`'s seven references now point at `build-release.yml`.

**Evidence, checked by the architect against GitHub:**
- **Release trial `45285201965`**, started by the push on `fc2fd95`, is green in all nine jobs: the version, seven
  targets on their real runners (`ubuntu-24.04-arm` included, for the first time outside a tag) and the installers.
  It left eight artifacts.
- **Nothing was published:**
  - 0.21.1 still holds 22 assets, the newest updated 12:50:25Z, before the run, although the trial built archives
    with the same names;
  - no new Release and no new tag;
  - the trial's archive has no attestation (the dev team's `gh attestation verify` returned 404).
- **The dev team's comparison with 0.21.1's published archives:**
  - the names and the files inside, in order, match;
  - `CHANGELOG.md` differs only by this increment's own lines;
  - the `.sha256` lines match in every byte of shape: length, two-space separator, LF, lowercase.
- CI, Docs and Installers were green on `fc2fd95`.

**Not yet seen:** a tag through the new `build` job. **0.22.0's release run is its proof**, and the architect checks
it after the tag, together with `gh attestation verify --repo` on archives signed by `build-release.yml`.

**Carried** (`project_stikk_carried_followups`):
- the trial repeats `guard`'s version command;
- `installers.yml` keeps its own copies of the fill and the packaging.

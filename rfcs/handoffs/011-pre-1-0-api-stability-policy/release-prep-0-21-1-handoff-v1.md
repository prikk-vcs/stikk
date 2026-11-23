# Handoff — 0.21.1 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md).
**Issued** 2026-11-23. **The project owner authorized 0.21.1** the same day, and the dev team prepares it by standing
permission. The tag waits for the owner after review. **Urgent but small.**

**Follows** [0.21.0's prep handoff](./release-prep-0-21-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Why there is a 0.21.1.** The 0.21.0 tag's release run (`45103941837`) was **cancelled**. Its two macOS binary jobs
> ended one second after they started, with no steps run, and GitHub's annotation says *"The configuration
> 'macos-14-arm64-us-default' is not supported"*. **GitHub retired the `macos-14` image** that `release.yml`'s matrix
> names, at `:194` and `:196`. The jobs that wait on the binaries (installers, AUR) were skipped.
>
> So **0.21.0 is published on crates.io, all six crates**, and its GitHub Release holds **10 of its 22 files**: no
> macOS archives, and no installers. **`releases/latest/download/install.sh` and `install.ps1` return 404 now.** A
> re-run cannot help: it runs the workflow as it was at the tagged commit. Moving the tag is not clean either, since
> crates.io already holds 0.21.0 from that commit. **0.21.1 is the same stikk, released by a working workflow.**
>
> **Also in this release, at the owner's decision:** a GitHub Release's title is the bare version, as prikk's are.
> The owner has already renamed all 22 existing releases to their bare version.
>
> **Sanctioned:** `release.yml`, the version bump, `Cargo.lock`'s workspace entries, and text. Anything else: stop and
> report. **No branch.**

---

## 1. `release.yml`: two fixes

1. **The macOS runner: `macos-14` → `macos-latest`**, in both matrix entries (`:194`, `:196`).
   - `macos-latest` is the label the real-binary suite has run on green every cycle, including today. **So the release
     builds on what was tested**, and it does not retire under us the way a pinned image did.
   - Update the comment above the two entries. Both slices still build on an Apple-Silicon runner, x86_64 by
     cross-compiling. Add one sentence: the pinned `macos-14` was retired mid-release (0.21.0), and that is why the
     label is `macos-latest`.
2. **The title: `name: stikk ${{ needs.guard.outputs.version }}` → `name: ${{ needs.guard.outputs.version }}`**
   (`:104`). Nothing reads a release's title: the installers read `tagName`, and the upload steps pass only the tag.

**Change nothing else in the workflows.** The other pinned labels (`ubuntu-24.04-arm`, `ubuntu-22.04-arm`) ran green
today.

## 2. The bump

`0.21.0` → `0.21.1` in `[workspace.package]` and the five pins. `cargo update --workspace`: **only `stikk*` lines
may move.**

## 3. `CHANGELOG.md`

Add `## 0.21.1 — <the date you push>` above 0.21.0. It should be short:
- **A bold summary:** *"The same stikk as 0.21.0, released in full. 0.21.0's release run could not build for macOS,
  so its GitHub Release has no macOS archives and no installers; use 0.21.1."*
- **`### Fixed`, two lines:**
  - the release workflow builds macOS on a supported runner;
  - a release's title is its version alone.
- **No `### Breaking`:** `cargo public-api` shows nothing between 0.21.0 and 0.21.1. Confirm it with
  `cargo public-api -p <crate> diff 0.21.0`, and say so.

**0.21.0's section is not changed.** It is what 0.21.0 is, on crates.io.

## 4. Text and stamps

- **Status texts** say *v0.21.x*, so they stay. Check them.
- **The install guide's *"What you should see"* transcript:** 0.21.0 → **0.21.1** (the carried rule).
- **`ROADMAP.md`:** in 0.21.0's *Shipped* section, add one line: *"Released as 0.21.1: 0.21.0's release run could not
  build for macOS (a retired runner image), so 0.21.1 is the same stikk, released in full."*
- **RFC 011's row:** add this handoff.

## 5. A check this release adds to every prep, from now on

A runner label in `release.yml` is only ever exercised by a tag. **In the prep request, list every `runs-on` label and
matrix `os:` in `release.yml`**, and for each, give the most recent green run you can name that used it. A label with
no recent green run is a stop-and-report. For this prep: `macos-latest` (today's suite), `ubuntu-latest`,
`ubuntu-24.04-arm` and `windows-latest`.

## 6. Submit and runs (no branch)

**Before submitting:** run the eight gates, the **local real-binary suite at 0.28.0 + 0.58.0**, and `release.yml`'s
`awk` for `0.21.1`.

**Package it** to `.git-exclude/review-request/011-release-0-21-1-preparation/review-request-v1.md`.

**After approval:**

1. Push.
2. On `main`, dispatch:
   - the full-matrix suite;
   - supply chain;
   - `installers.yml`;
   - `aur.yml` with `push` unchecked.
3. Report every run and leg at **the SHA to be tagged**.

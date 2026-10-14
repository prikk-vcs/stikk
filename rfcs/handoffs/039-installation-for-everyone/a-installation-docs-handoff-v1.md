# Handoff A — RFC 039: installation for everyone, the documentation (v1)

**Companion to:** [RFC 039](../../proposed/039-installation-for-everyone.md). **This handoff's part (decision 1) was
accepted by the project owner on 2026-10-14**, before the rest of the RFC, which is why it is issued while the RFC is
still proposed. **Do not start Handoff B's work here.**
**Covers:** decision 1, for what exists **today**. 0.11.0's first increment.
**Not a release.**

> **Who this is for:** someone who has never used Rust, Git or a terminal installer, and wants to look at a prikk
> repository. **Today the first thing they read is `cargo build --release`.** After this handoff, the first thing
> they read is how to get a working `stikk` and `prikk` on their machine without a compiler.
>
> **Text only.** No `.rs`, no workflow, no manifest. **No command may be documented that does not work today**: the
> one-line installer is Handoff B's, and it goes to the top of this same Quick start when it exists.

---

## 1. Measure the path before you write it

On a clean scratch directory, **follow the steps you are about to document, as written**, on Linux, and on macOS
or Windows if you can reach one. Record each command and its output in the request:
- **prikk**: prikk's own installer (`curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh`)
  into a scratch prefix (`--prefix`), then `prikk --version`. **Check that it installs a prikk within stikk's
  validated range** (≥ 0.28, through 0.51.0); if the latest prikk is newer, say so, and document how to pin
  (`--version 0.51.0`).
- **stikk**: download `stikk-<ver>-<target>.tar.gz` and its `.sha256` from the 0.10.0 release, verify
  (`sha256sum -c`, or `shasum -a 256 -c` on macOS; on Windows, `Get-FileHash` against the `.sha256`), extract, and
  put `stikk` on `PATH`. Then `stikk --version`.
- **The first run:** `prikk init` in an empty directory, then `stikk` there. The orientation must open.

## 2. `README.md`

- **`## Install`** comes first, before anything about building:
  1. **Install prikk** in one command (prikk's installer), with the pin for stikk's validated version, and a link to
     prikk's install page for Windows and Intel Macs, which prikk's installer does not cover.
  2. **Install stikk**: a short table with **one row per platform** and the exact archive name, then the three
     steps (download the archive and its `.sha256`, verify, extract into a directory on `PATH`). Give the commands
     for Linux, macOS and Windows separately; **do not make a user translate**.
  3. **Optional:** `gh attestation verify <archive> -R prikk-vcs/stikk`, for the cautious, saying what it proves (who
     built it) beside what the checksum proves (that it was not altered).
  4. **For Rust users:** `cargo install stikk`.
- **`## Quick start`**: three lines. `prikk init` (or `cd` into an existing prikk repository), `stikk`, and `?` for
  help inside it.
- **Building from source** moves to `CONTRIBUTING.md` (or a *Building from source* section under Contributing in
  the README), with the existing `cargo build` text and `STIKK_PRIKK_BIN`.
- Keep the launcher utilities (`stikk config path` and the rest), after the Quick start.

## 3. The guide and the crates.io page

- **`docs/src/guide/getting-started.md`**: replace `## Build` with **`## Install`**, the same paths as the README (or
  a link to a new `docs/src/guide/install.md`, with the details and the platform table, if the page grows; your
  choice, say which). `## Point stikk at prikk` stays, but it is now the exception, not the first step.
- **`crates/stikk/README.md`**: crates.io readers have Rust, so `cargo install stikk` stays first. **Add one line**
  pointing to the prebuilt binaries for people who do not.

## 4. Write for the user who has never done this

- **Say what each step is for, in one clause.** *"Verify the download, so you know it was not altered on the way."*
- **No jargon without its plain meaning** the first time it appears (`PATH`, `checksum`, `terminal`).
- **One command per code block where a user copies it**, and no `$` prompts inside the blocks.
- **Show what success looks like**: `stikk --version` prints `stikk 0.10.0`.

## 5. The changelog

Under `## Unreleased`, `### Changed`: the README and the guide lead with installing prebuilt binaries, not building
from source.

## 6. Gates and submit (no branch)

The eight gates (gate 7 builds the book), the local suite, and the **pre-submit grep** (before committing too).
Package to `.git-exclude/review-request/039-a-installation-docs/review-request-v1.md`, leading with §1's recorded run
on each platform you reached, then the README's new `## Install` and `## Quick start` **as the file holds them**.

**After approval:** push to `main`. Docs deploys on push; confirm the guide's new page is live.

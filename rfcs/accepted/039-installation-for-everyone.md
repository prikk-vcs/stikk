# RFC 039 — Installation for everyone: one command, prikk included

**Status.** **Accepted by the project owner 2026-10-14**, decision 4 included. **Proposed 2026-10-14** by the architect, after the project owner's direction the same day: *"Much
easier installation for unskilled users is required in the near future."* Its first handoff, the documentation, was
already accepted by the owner in conversation that day. **No open question**, but decision 4 reaches into another
project, and accepting this RFC accepts it.
Grounded in stikk's 0.10.0 release (12 assets: six targets, each with a `.sha256`, all attested) and prikk's own
install guide and release tooling in the clone (`docs/src/guide/install.md`, `crates/prikk-cli/Cargo.toml`).
**Tracks.** `NFR-R03`, the scope of 0.11.0 (`.git-exclude/release/0-11-0-scope.md`), RFC 007's release workflow,
RFC 020.
**Touches.** `README.md`, `crates/stikk/README.md`, the guide, `crates/stikk/Cargo.toml` (`binstall` metadata),
`.github/workflows/release.yml` (installer assets), and two new scripts.

## Summary

**A new user has to build stikk from source**, because that is what `README.md`'s *Quick start* says first, and the
guide's *Getting started* opens under the heading *Build*. **Every release already ships prebuilt binaries for six
targets**, checksummed and attested, and nothing tells the user. Worse, stikk needs **prikk** too, and our README
only says *"install a compatible `prikk`"*.

This RFC makes installation **one command on each platform**, and that command installs prikk as well when it is
missing. A user who has never used Rust, Git or a terminal installer before needs no other step.

## Findings

### F1 — what the user is told today

- `README.md`, *Quick start*: `cargo build --release`, then `./target/release/stikk`. **This needs a Rust toolchain
  and a clone.**
- `docs/src/guide/getting-started.md`: opens with `## Build` and `cargo build --release`.
- `crates/stikk/README.md` (crates.io): `cargo install stikk`. This still needs Rust, and it compiles.
- **None of them mentions the release binaries.**

### F2 — what we already publish

Per release: `stikk-<ver>-<target>.tar.gz` (`.zip` on Windows) and `.sha256`, for `x86_64-unknown-linux-gnu`,
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` and
`x86_64-pc-windows-msvc`, each with a build-provenance attestation (`gh attestation verify` passed for 0.10.0).
**Tags have no `v` prefix** (`0.10.0`).

### F3 — how prikk does it, which a stikk user also meets

- `curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh`. It detects OS and
  architecture, downloads the archive and its `.sha256`, **installs nothing if the checksum fails**, puts the binary
  in `~/.local/bin`, adds one marked `PATH` block to the shell's startup file, and has a matching `uninstall.sh`.
  Re-running is safe. The version can be pinned and the directory overridden.
- **Its limits:** Linux x86_64/aarch64 and macOS Apple Silicon only. **No Windows installer** (it refuses and points
  at the `.zip` or `cargo install`), and **no Intel-Mac binary**.
- prikk publishes `[package.metadata.binstall]`, so `cargo binstall prikk` downloads instead of compiling.

## Decisions

1. **The README leads with one command per platform, and building from source moves to Contributing.** The Quick
   start becomes: run the installer, then `stikk` inside a prikk repository, with the one prikk command a new user
   needs to make a repository (`prikk init`). The manual download (with checksum, and `gh attestation verify` for the
   cautious) and `cargo install`/`cargo binstall` follow as alternatives. The same applies to the guide and the
   crates.io page. **Delivered first, as Handoff A**: it was already accepted, and needs no new code.

2. **Two installers, published as release assets:**
   - **`install.sh`** (Linux, macOS): `curl -fsSL …/releases/latest/download/install.sh | sh`;
   - **`install.ps1`** (Windows): `irm …/releases/latest/download/install.ps1 | iex`. **Windows gets one even though
     prikk has none.**
   Both detect the platform, download the matching archive **and its `.sha256` from the same release**, **install
   nothing if verification fails**, install to the user's own directory (`~/.local/bin`, and
   `%LOCALAPPDATA%\Programs\stikk` on Windows), add stikk to `PATH` once and say so, and print the next command to
   run. Each is **re-runnable**, takes a **version pin** and an **install-directory override**, and has a matching
   **uninstaller** that removes exactly what it added. **Each is one self-contained file**, so a cautious user can
   read it before running it, and each says what a checksum does and does not prove, as prikk's does.

3. **The scripts live in the repository and are attached by `release.yml`**, versioned with the release they
   install. They are tested in CI by **installing the release they ship with into a scratch directory** on Linux,
   macOS and Windows.

4. **When prikk is missing, the installer offers to install it, using prikk's own installer.** This is the decision
   that reaches into another project, and it is shaped to stay honest:
   - stikk **never re-hosts or re-packages prikk**. On Linux and macOS (Apple Silicon) it runs **prikk's published
     `install.sh`**, which verifies prikk's own checksum, and pins **the version stikk's release validated**, so a
     new prikk cannot arrive untested.
   - It **asks first**, and stays non-interactive when told to (`--with-prikk` or `--no-prikk`), so CI can use it.
   - **Where prikk has no installer** (Windows, Intel Macs), it does not improvise. It says so plainly and prints
     the exact alternative: the `.zip` from prikk's release page, or `cargo install prikk --version <validated>`.
   - **When prikk is already installed**, it reports the version and whether stikk validated it, and changes nothing.

5. **`cargo binstall stikk` works**, through `[package.metadata.binstall]` in `crates/stikk/Cargo.toml` that names
   our archive pattern. It is measured against a real release before it is documented.

6. **Not in this RFC:** package managers (Homebrew, Scoop, winget, distribution packages). Each means accounts and
   publishing outside this repository, so each is the owner's decision and its own increment. **Decided after the
   installers have shipped and been used.**

## Delivery

- [**Handoff A, the documentation (decision 1)**](../handoffs/039-installation-for-everyone/a-installation-docs-handoff-v1.md), issued with this proposal, since the owner already accepted it.
  **It documents what exists today**: the prebuilt download per platform (with its checksum) as the first path,
  prikk's own installer for prikk, and `cargo install` for Rust users. **Handoff B then puts the one-line installer
  at the top of the same Quick start.** A never documents a command that does not exist yet.
- **Handoff B, the installers and `binstall` (decisions 2–5)**, issued on acceptance: [Handoff B](../handoffs/039-installation-for-everyone/b-installers-handoff-v1.md). It touches `release.yml`, so the next
  release run is the installers' first real publication. B's CI test installs **the previous release** until then.

## Changelog

`### Added`: one-command installation on Linux, macOS and Windows, with prikk installed when missing; `cargo
binstall stikk`. `### Changed`: the README and the guide lead with installing, not building.

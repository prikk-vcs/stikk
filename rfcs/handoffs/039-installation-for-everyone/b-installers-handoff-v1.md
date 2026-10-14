# Handoff B — RFC 039: one-command installers, prikk included, and `cargo binstall` (v1)

**Companion to:** [RFC 039](../../accepted/039-installation-for-everyone.md), **accepted 2026-10-14, decision 4
included.** **Read the RFC first.**
**Covers:** decisions 2–5. **Start after [Handoff A](./a-installation-docs-handoff-v1.md) has landed**: B puts its
one-line commands at the top of the Quick start A writes.
**Not a release.** The next release run is the installers' first real publication.

> **The user this is for has never installed a developer tool.** They copy one line from the README into a terminal
> and expect `stikk` to work, with prikk. **Every message the scripts print is for that person**: what happened, what
> to type next, and what failed and why, in plain words.
>
> **Sanctioned:** two new scripts and their uninstallers, `crates/stikk/Cargo.toml`'s `binstall` metadata,
> `release.yml` (attaching the scripts, and the Windows checksum format, §5), one new workflow (testing them), and the README and guide lines that name
> them. **No Rust source.** Anything else: stop and report.

---

## 1. Measured facts to build on (re-check before relying on them)

- **stikk's archives**: `stikk-<ver>-<target>.tar.gz` (`.zip` on Windows), each beside a `.sha256`, under
  `releases/download/<ver>/`, **with no `v` in the tag**. Inside: `stikk-<ver>-<target>/stikk` (with `README.md`,
  `LICENSE`, `CHANGELOG.md`). Confirm the Windows `.zip`'s layout yourself.
- **prikk attaches `install.sh` and `install.sh.sha256` to every release** (tags without `v`, e.g. `0.51.0`), and
  `uninstall.sh` likewise. Its installer takes `--version` and `--prefix`, refuses Windows, and has no Intel-Mac
  binary.
- `release.yml` uploads assets with `softprops/action-gh-release` (`files:`); the version comes from the tag.

## 2. The scripts

**Where:** `scripts/install.sh`, `scripts/uninstall.sh`, `scripts/install.ps1`, `scripts/uninstall.ps1`, as
**templates** with two placeholders that `release.yml` fills at release time:
- **the stikk version they install** (the release's own);
- **the prikk version to pin**: stikk's validated ceiling, **derived from `crates/stikk-prikk/src/version.rs`** the way
  the real-binary workflow already derives its matrix. **Never a second literal.**

Then each published script installs exactly the release it was attached to, and `…/releases/latest/download/install.sh`
always means "the latest stikk, with the prikk it was validated against".

**What each installer does, in order:**
1. **Detects the platform** and picks the archive. **An unsupported platform stops with a sentence naming the
   manual download and `cargo install stikk`.** Never guess.
2. **Downloads the archive and its `.sha256` into a temporary directory**, verifies (`sha256sum -c`, `shasum -a 256
   -c` on macOS, `Get-FileHash` on Windows), and **installs nothing if verification fails**, saying so.
3. **Installs** into `~/.local/bin` (Linux, macOS) or `%LOCALAPPDATA%\Programs\stikk` (Windows). It overwrites a
   previous stikk there, which is what makes re-running safe.
4. **`PATH`**: if the directory is not already on it, add **one marked block** to the shell's startup file (as prikk
   does; say which file, and the macOS login-shell caveat prikk documents), or the **user** `Path` on Windows. Never
   the system `Path`, never twice.
5. **prikk** (RFC 039 decision 4):
   - **already on `PATH`**: print its version and whether stikk validated it (`≥ 0.28` and `≤` the pinned version),
     and change nothing;
   - **missing, Linux or Apple-Silicon macOS**: **ask** (default yes). On yes, download **prikk's `install.sh` and
     its `.sha256` from prikk's release for the pinned version**, verify that file, then run it with `--version
     <pinned>`. **A verification failure installs nothing**;
   - **missing, Windows or Intel macOS**: prikk has no installer. Say so, and print the exact alternatives: prikk's
     release page `.zip` for Windows, and `cargo install prikk --version <pinned>`;
   - **`--with-prikk` / `--no-prikk`** (`-WithPrikk` / `-NoPrikk`) skip the question, so CI and scripts never block.
     **With no terminal to ask on** (`curl … | sh` in CI), the default is **no**, and the script says so.
6. **Ends with what to type next**: `prikk init` (if no repository), then `stikk`, and how to open a new terminal if
   `PATH` changed.

**Options**, the same on every script: a version pin (`--version`, and an environment variable), an install
directory (`--prefix`, and an environment variable), `--with-prikk`/`--no-prikk`, and **`--base-url`** for a mirror.
`--base-url` is also what §4's tests use to serve a tampered file. Document it as being for mirrors.

**The uninstallers** remove the `stikk` binary and the marked `PATH` block, **and nothing else**. **They never remove
prikk**: prikk has its own uninstaller, and the message says how to run it if the user wants to.

**Each script is one self-contained file**, readable before running. Each ends by saying what the checksum proves
(not altered in transit) and what it does not (who built it), and names `gh attestation verify` for the latter.

## 3. `cargo binstall stikk`

`[package.metadata.binstall]` in `crates/stikk/Cargo.toml`: the `pkg-url` for the archive pattern above (no `v`),
`bin-dir` for the `stikk-<ver>-<target>/` directory, and the Windows `.zip` override. **Measure it against the 0.10.0
release**: `cargo binstall stikk --version 0.10.0 --root <scratch>` must **download, not compile**. Record its output.
If binstall cannot reach 0.10.0 without the metadata being in a published crate (it reads crates.io), say so, and
measure with `--manifest-path` or the equivalent override instead.

## 4. Tests: a new workflow, `installers.yml`

- **Triggers:** pushes that touch `scripts/**` or the workflow itself, and `workflow_dispatch`.
- **Matrix:** ubuntu, macos, windows.
- **Each leg installs the latest published release** with the scripts **from the commit under test**, the
  placeholders filled the way `release.yml` fills them, into a scratch directory with `--no-prikk`. Then it checks:
  `stikk --version` prints that version; a **re-run** is clean; the **uninstaller** leaves the directory and the
  startup file as they were before the install.
- **The tamper case, on every leg:** serve the archive with a **wrong `.sha256`** through `--base-url` (a local
  directory or `file://` URL). The installer must **exit non-zero and install nothing**.
- **On Linux and macOS:** one leg with `--with-prikk` installs prikk through prikk's verified installer into the
  scratch prefix. `prikk --version` must equal the pinned version.
- **The first real publication** is the next release run. Its review must show the four scripts attached, with their
  `.sha256` files.

## 5. `release.yml`

Fill the placeholders, attach the four scripts **and a `.sha256` for each** (users who download a script to read it
can verify it too), in the job that already attaches assets.

**Amended 2026-10-14, after Handoff A's review: one more change, and only one.** The Windows archive's checksum is
written in a **different format** from the other five (`release.yml:238`: `(Get-FileHash …).Hash | Out-File`). It
holds an uppercase hash alone, with no filename and a CRLF, where the others hold `sha256sum`'s `<lowercase hash>␣␣<file>`.
**Write it in the same format as the others**: lowercase, two spaces, the file name, LF. Then one shape serves every
platform, `install.ps1` parses one format, and Git Bash's `sha256sum -c` works on Windows too.
- `install.ps1` must parse **the new format** (the hash is the first field); it only ever installs releases from the
  one it ships with, so it never meets the old one.
- **Update the README's and guide's Windows check** in §6 to read the first field (for example
  `(Get-Content f.sha256).Split(' ')[0]`), and **keep the 0.10.0 example working**: either take that one instruction
  through the next release (the old file's first field is the whole hash, so `.Split(' ')[0]` still works on it), or
  say which release the format changed in. **Measure both formats** with your PowerShell snippet before committing.
- Changelog `### Changed`: every release's checksum files now share one format.

**Change nothing else in the workflow.**

## 6. The README and the guide

**Put the one-line installers at the very top of Handoff A's `## Install`**: one line for Linux and macOS, one for
Windows. The manual download (A's text) follows as *"or, by hand"*. Add what the prikk question means, in one
sentence, and the uninstall line. Mirror this in the guide's install page. **`cargo binstall stikk`** goes with
`cargo install` for Rust users. **Keep everything A wrote that is still true.**

## 7. The changelog

Under `## Unreleased`, `### Added`: one-command installation on Linux, macOS and Windows, checksum-verified, with
uninstallers, and prikk installed through prikk's own verified installer when it is missing (where prikk has one);
and `cargo binstall stikk`.

## 8. The traps

1. **Never install anything unverified**, stikk's archive or prikk's installer.
2. **Never pin prikk with a literal**: derive it from `version.rs`.
3. **Never block on a question without a terminal.**
4. **Never touch the system `PATH`, and never remove prikk.**
5. **Windows is first-class**: test it, don't describe it.
6. **Grep before committing** for decoded escapes; shell and PowerShell quoting are where they hide.

## 9. Gates, runs, submit (no branch)

The eight gates, the local suite, and `shellcheck` on the `.sh` files (and PowerShell's `PSScriptAnalyzer` if you can
run it). Package to `.git-exclude/review-request/039-b-installers/review-request-v1.md`, leading with **a real
install transcript** on each platform you reached (including the prikk question, answered both ways), then the tamper
case, then binstall's output.

**After approval:** push to `main`, then dispatch `installers.yml` (if the push did not trigger it), the full-matrix
suite and supply chain on `main`, and report every run and leg.

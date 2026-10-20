# Handoff A — RFC 048: the installers install stikk only, and a page for newcomers (v1)

**Companion to:** [RFC 048](../../accepted/048-delivery-for-new-users.md), **accepted 2026-10-20**. **Read the RFC
first**, F6 and decisions 2–4. **Covers:** decisions 2, 3 and 4. 0.15.0's increment 4.
**Not a release.** The tap (decision 5) is Handoff B, later; signing (decision 7) is not code.

> **What this increment is.** The owner's principle, which prikk adopted in its letter 026: **each program installs
> itself.** stikk's installers install stikk, **check** for prikk, and print **prikk's own** install command when it is
> missing or too old. They stop downloading, verifying, pinning or running anything of prikk's. And the install page
> is rewritten for someone who has never opened a terminal.
>
> **No branch.** Local gates before submitting; after approval, push to `main`, and `installers.yml` runs on the push
> (it triggers on script changes). Report its run with the SHA.

---

## 1. Commit 1: the four scripts

**a. Remove prikk installation entirely**, on every platform:
- `scripts/install.sh:186-228`: the `/dev/tty` prompt, the download of prikk's installer and its checksum, the run
  with `--version`/`--prefix`, and the per-platform fallbacks (`cargo install`, the release tag URL).
- `scripts/install.ps1:167-181`: the manual zip instructions and the `-WithPrikk` explanation.
- **The flags go**: `--with-prikk`, `--no-prikk` (`install.sh:18-19, 38, 48-49` and the usage text at `:13`),
  `-WithPrikk`, `-NoPrikk` (`install.ps1:34-36`). An unknown flag is already an error in `install.sh`; keep it so. The
  header comments at `install.sh:6-7` and in `install.ps1` stop saying the script can install prikk.

**b. The check that replaces it** — the same on both, after stikk is installed:

| prikk on `PATH`? | Version | What the installer says |
|---|---|---|
| no | — | stikk reads prikk repositories, so you need prikk too. **prikk installs itself:** then prikk's command (below) |
| yes | below the floor | that this prikk is **too old for this stikk** (it needs 0.`<floor>` or newer), then prikk's command to update |
| yes | floor … tested ceiling | that it is inside the range this stikk was tested against |
| yes | above the ceiling | that it is newer than this stikk was tested against; **it still runs, and stikk will say so** |

Keep the newer-than-tested wording **calm**: this is the normal state for a few days after every prikk release, and a
newcomer should not read it as a fault. *"Yours still runs"* must **not** be said below the floor (today's `else`
branch at `install.sh:182` says it for both).

**prikk's command**, printed exactly:
- Linux and macOS: `curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh`, and
  prikk's install guide URL on the next line. **Print it on every Unix platform**, Intel Macs included: prikk's
  `install.sh` decides what it supports and says what to do otherwise (RFC 048 decision 2). stikk no longer encodes
  prikk's platform list.
- Windows: **if prikk 0.54.0 is published with `install.ps1` when you submit**, `irm
  https://github.com/prikk-vcs/prikk/releases/latest/download/install.ps1 | iex` — measure that the URL resolves first.
  **Otherwise** prikk's release page (`https://github.com/prikk-vcs/prikk/releases/latest`) and its install guide,
  with one line saying a Windows installer is coming from prikk. Do not write prikk's manual steps out; link to them.
- The guide URL: `https://github.com/prikk-vcs/prikk/blob/main/docs/src/guide/install.md`, unless prikk publishes its
  guide somewhere you can confirm is more stable — say which you used and why.

**Exit 0 in every row.** stikk is installed; a missing prikk is reported, not an error. The final *"Next"* list keeps
`prikk init` and `stikk`, and when prikk is missing its first step is *install prikk (above)*.

**c. Floor and ceiling are derived, never written.** Today only the ceiling is filled (`@PRIKK_VERSION@`, from
`print_version_matrix`), and the floor `28` is hard-coded in both scripts (`install.sh:179`, `install.ps1:160`).
**Fill the floor the same way** (e.g. `@PRIKK_FLOOR@`), in `release.yml`'s installer job (`:287-308`) and in
`installers.yml`'s *"Fill the placeholders"* steps, and extend the unfilled-template refusal to it. Rename
`PRIKK_PINNED_VERSION`/`$PrikkPinnedVersion` to say what it now is — **the tested ceiling**, not a pin.

**d. Windows: the current session's `$env:Path`** (decision 3). When the installer adds stikk's folder to the user
`PATH`, also prepend it to `$env:Path` in the running session (only if absent), so `stikk` works in the same window.
The *"open a new PowerShell window (PATH changed)"* step (`install.ps1:188`) goes. **Unix keeps its step**: the script
runs in a child shell and cannot change the parent's.

**e. The uninstallers** already never touch prikk. Make their line say that prikk is removed by **prikk's own**
uninstaller, printing prikk's command where prikk publishes one (`uninstall.sh` already does; `uninstall.ps1:71-72`
says *"Remove prikk.exe from wherever you put it"* — replace with prikk's guide, or prikk's `uninstall.ps1` line if
0.54.0 is out).

## 2. Commit 2: `installers.yml`

**Remove** the legs that test prikk installation: *"--with-prikk installs the pinned prikk"* (`:143`), *"curl | sh at a
terminal asks about prikk"* (`:154`), and the prikk half of *"With no controlling terminal"* (`:199`). Every remaining
`--no-prikk` argument goes with the flag.

**Add**, on the Unix legs (Linux and macOS) and on Windows:
1. **prikk missing**: with no `prikk` on `PATH` (prove it before running), install exits 0, stikk runs, the log
   contains prikk's command **exactly**, and **nothing of prikk's was downloaded or written**: no `prikk` anywhere under
   the prefix or `$HOME/.local/bin`, and (Unix) no request for prikk's assets — run with a `PATH` whose `curl` is a
   wrapper that logs its URLs, and assert none contains `prikk-vcs/prikk/releases/download`.
2. **prikk too old**: a fake `prikk` on `PATH` printing `prikk 0.<floor-1>.0` → the too-old wording and prikk's command;
   *"still runs"* absent.
3. **prikk inside the range**: a fake printing the ceiling → the inside-range line.
4. **prikk newer**: a fake printing `0.<ceiling+1>.0` → the newer-than-tested line.
5. **The removed flags**: `--with-prikk` is refused as an unknown option on Unix, and `-WithPrikk` as an unknown
   parameter on Windows — so a stale copy-pasted command fails loudly rather than appearing to work.
6. **Windows only**: in the **same** PowerShell session that ran the installer, `stikk --version` resolves (no new
   session). Run the installer the way the README does (`… | iex` of the filled script's content), not as a file.
7. **Linux aarch64 on `ubuntu-22.04-arm`** (glibc 2.35): install, then `stikk --version`. **Record stderr** in the job
   log and in the review request: whether the weak-symbol loader warning (RFC 048 F4) prints. Do not fail on it; the
   point is to learn.

The existing legs (template refusal, install, re-run, uninstall restores the startup file, the tamper case failing for
the checksum reason) stay, minus their `--no-prikk`.

## 3. Commit 3: the page and the README

`docs/src/guide/install.md`, in RFC 048 decision 4's order:
1. **Open a terminal** — one or two lines per platform (macOS: Spotlight → *Terminal*; Windows: Start → *PowerShell*;
   Linux: the *Terminal* app).
2. **Step 1, prikk** — prikk's one-line command per platform, **linked to prikk's install guide as the authority**;
   Windows before prikk 0.54.0 is a link, never a copy of prikk's steps.
3. **Step 2, stikk** — our one line, then **what you should see** (paste the installer's real last lines from a run,
   as the file holds them), then `prikk init` and `stikk`.
4. **Updating** (run each line again) and **removing** (each program's uninstall line).
5. **By hand (advanced)** — today's *"Or, by hand"*, minus its prikk part (link to prikk's guide instead), opened by
   decision 4's warning, with the mitigations: macOS *Open Anyway* (Privacy & Security) or `xattr -d
   com.apple.quarantine`; Windows **unblock the zip before extracting** (Properties → *Unblock*, or `Unblock-File`),
   and start `stikk` from PowerShell. **Never** tell a user to download `install.ps1` and run it as a file.
6. **If Smart App Control blocks stikk or prikk** — one plain paragraph: what it is, why (both are unsigned), and
   Microsoft's page (`https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003`).
   **Do not tell the reader to turn it off.**
7. *If you already use Rust* and *Next* stay.

`README.md`'s `## Install` (`:41-236`) **shrinks** to the two steps (prikk's line, ours) and a link to the page; the
manual tables live only on the page. **Fix the pin example** (`--version 0.51.0` beside *"tested through 0.53.0"*);
better, derive nothing in prose that a release changes. `crates/stikk/README.md` (the crates.io page): check it against
the new page and say what you changed. `CONTRIBUTING.md` unchanged unless it mentions the removed flags.

**Changelog** `## Unreleased`: `### Changed` and `### Removed` as RFC 048's Changelog section words them.

## 4. The traps

1. **stikk runs nothing of prikk's.** Not its installer, not a probe of its assets beyond the one Windows URL check in
   §1b (done at submission, by you, not by the script). The curl-wrapper leg proves it.
2. **No prikk version anywhere as a pin.** The ceiling is only for wording; nothing passes it to prikk.
3. **Below the floor is not "still runs".**
4. **Exit 0 when prikk is missing.**
5. **The page links prikk's steps; it never copies them** beyond the one-line command prikk publishes.
6. **Never instruct turning off a security feature.**
7. **Paste captures and installer output as the file holds them; grep before committing.**

## 5. Submit (no branch)

The eight gates (the Rust gates are unaffected, but run them), `shellcheck` on both shell scripts if available, and
`installers.yml`'s logic run locally where it can be (the Unix legs in a scratch `HOME`). Package to
`.git-exclude/review-request/048-delivery-a/review-request-v1.md`, leading with **the four installer outputs**
(missing, too old, inside, newer) on Linux as they print, and saying whether prikk 0.54.0 was out at submission.

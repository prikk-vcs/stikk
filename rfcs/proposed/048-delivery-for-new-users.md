# RFC 048 — Delivery for users new to all of this

**Status.** **Proposed 2026-10-20** by the architect, as 0.15.0's increment 4 (`.git-exclude/release/0-15-0-scope.md`,
authorized), on the project owner's direction of 2026-10-18: *"we will have to consider and design carefully about
delivery this time and in the future. It should be easy enough for unmatured users as possible and reasonable."*
**Four questions only the owner can answer** (§ *For the owner*); the rest is ready to hand off on acceptance.
Measured on Linux against the published stikk 0.14.0 and prikk 0.53.0 assets:
`.git-exclude/reports/048-delivery/measure048.txt`. Channel facts are sourced, **not measured on macOS or Windows**:
`.git-exclude/reports/048-delivery/channels-research.md`. prikk's side read in the clone (`docs/src/guide/install.md`,
`.github/workflows/release.yml`, `crates/prikk-cli/Cargo.toml`, `ROADMAP.md`).
**Tracks.** RFC 039 (the installers; **decision 4 is revised here for Windows**), `NFR-R03`, `CON-5`.
**Touches.** `scripts/install.ps1`, `scripts/uninstall.ps1`, `.github/workflows/installers.yml`, the guide's install
page, `README.md`'s Install section; later, if the owner chooses it, `release.yml` and a tap repository.

## Summary

RFC 039 gave every platform one command. **Measured against a user who has never installed a program from a
terminal, that command works end to end on Linux and on Apple-Silicon Macs, and stops halfway on Windows**: stikk is
installed, prikk is not, and the user is told to download a zip and *"put prikk.exe in a folder on your PATH"* — the
step this whole effort exists to remove. On Intel Macs the same happens, with `cargo install` as the way out.

This RFC fixes the Windows half in stikk's own installer, writes the first page for someone who has never opened a
terminal, and settles **which channels are worth adding and which are not, and why**, so the question is not reopened
at every release. **It spends no money and creates no account by default.** One channel — a Homebrew tap — waits only
on the owner.

## Findings

### F1 — where a new user stops today

| Platform | stikk | prikk | Where a novice stops |
|---|---|---|---|
| Linux x86_64 / aarch64 | one line (`curl … \| sh`) | offered, via prikk's own `install.sh` | nowhere |
| macOS, Apple Silicon | one line | offered, the same way | nowhere |
| macOS, Intel | one line | **prikk publishes no binary** | `cargo install prikk` — needs Rust, compiles |
| Windows x86_64 (ARM64 by emulation) | one line (`irm … \| iex`) | **prikk publishes no installer** | *"download the zip … put prikk.exe in a folder on your PATH"* (`scripts/install.ps1:167-181`) |

**Windows is where most people new to terminals are likely to be, and it is the one that stops.** prikk *does* publish a
Windows build — `prikk-x86_64-pc-windows-msvc.zip` (`prikk.exe` + `LICENSE`) with its `.sha256` (M1) — only no
installer for it (prikk's guide, `install.md:36-38`: *"A PowerShell equivalent is a separate, later increment"*; no
plan for it in prikk's roadmap or RFCs). RFC 039 decision 4 chose to *"not improvise"* there.

Two smaller things a novice meets:
- **The guide never says how to open a terminal.** It defines one (*"the window where you type commands"*) and moves on.
- **On Windows the installer ends with "open a new PowerShell window"**, because it changes the user `PATH`. But
  `irm … | iex` runs **in the user's own session**, so the installer can update that session's `$env:Path` too and the
  step disappears.

### F2 — the security prompts, and which paths meet them

| Path | macOS Gatekeeper | Windows SmartScreen |
|---|---|---|
| The one-line installers | **none**: curl does not set the quarantine attribute | **none**: PowerShell's web cmdlets do not write Mark-of-the-Web, and stikk is started from a console |
| A Homebrew formula | none (fetched by curl; inferred, not stated by Homebrew) | — |
| An archive downloaded **in a browser** | quarantined. Since Sequoia, no Control-click bypass: System Settings → Privacy & Security → *Open Anyway*, within about an hour, then a password | marked; double-clicking `stikk.exe` shows SmartScreen. Run from a console it is not checked |

**Sourced, not measured** (channels-research §macOS, §Windows). So **signing helps exactly one path — a browser
download — and that is the path the guide can steer a novice away from.** The manual section today mentions the
quarantine only in a macOS aside and SmartScreen not at all.

### F3 — the channels, by what each asks of a novice and of us

| Channel | What a novice must do first | What it costs us | Eligible now? |
|---|---|---|---|
| **One-line installers** (have) | open a terminal, paste one line | nothing new | yes |
| **Homebrew tap** (`brew install prikk-vcs/tap/stikk`) | have Homebrew (itself an admin password and Apple's command-line tools) | a public repo, a token scoped to it, one CI step per release | yes |
| homebrew-core | have Homebrew | build from source; review | **no**: ≥ 75 stars (225 self-submitted), repo ≥ 30 days |
| **winget** (built into Windows 10/11) | nothing | **one human-reviewed PR per version** (observed 1–4 days) against weekly releases; automation needs a **classic** token (`public_repo`, `workflow`); and it helps a novice only if prikk is in winget too | yes, at that cost |
| Scoop, own bucket | **install Scoop**, add the bucket | a repo of JSON manifests | yes, but no gain over `irm \| iex` |
| Scoop main | install Scoop | — | **no**: 500 stars / 150 forks |
| .deb / .rpm on releases, apt/rpm repository, AUR, nixpkgs | Linux users — already one line | per-format builds, a repository, accounts | some; nixpkgs/Debian/Fedora **no** (vendoring, audience) |
| Snap / Flatpak | — | — | **no**: confinement blocks running `prikk` and reading hidden files; Flathub refuses console programs |
| cargo-dist | — | replaces our `release.yml` and installers, which already exist and are tested | — |

### F4 — the Linux builds run on older systems than their symbols suggest

Both `-gnu` builds reference `GLIBC_2.39`, but only through **weak** symbols (`pidfd_spawnp`, `pidfd_getpid`); the hard
floor is `GLIBC_2.34`. Run under **Debian 12's glibc 2.36** (its own loader), stikk 0.14.0 x86_64-gnu prints a one-line
loader warning and runs; prikk 0.53.0 runs clean (M2, M3). x86_64 installs the static musl build anyway, so this
matters for **aarch64** — Raspberry Pi OS bookworm is glibc 2.36. **Not measured on a real aarch64 system**, and
whether the warning prints outside an explicitly invoked loader is not known.

### F5 — prikk's side

- **No Windows installer, no Intel-Mac binary** (prikk's `release.yml:39-43`: x86_64-apple-darwin *"has never actually
  been built or run"* in its CI). No package-manager work scheduled; its roadmap names Homebrew, Scoop, the AUR and
  nixpkgs as *"worth doing alongside"* (`ROADMAP.md:62`).
- **`cargo binstall prikk` finds nothing on Windows** (M4): its `[package.metadata.binstall]` names a `.tar.gz` for every
  target, and the Windows asset is a `.zip`. stikk's own override finds its zip.
- Nothing of prikk's is signed (`install.md:130`).

## Decisions

1. **The one-line installers stay the first path on every platform, and every other channel is in addition.** They
   ask for no account, no tool installed first and no administrator password; neither meets Gatekeeper or SmartScreen
   (F2); running the same line again upgrades.

2. **On Windows, stikk's installer installs prikk itself, from prikk's own release** — revising RFC 039 decision 4 for
   Windows only, where prikk has a build and no installer:
   - **Asked first**, as on Linux and macOS: `[Y/n]` when interactive, *no* when not; `-WithPrikk` / `-NoPrikk` decide
     in advance.
   - Downloads `prikk-x86_64-pc-windows-msvc.zip` **and its `.sha256` from prikk's release at the pinned, validated
     version** (the same pin `install.sh` passes to prikk's installer); **installs nothing if the checksum fails.**
   - Puts **only `prikk.exe`** in `%LOCALAPPDATA%\Programs\prikk` — prikk's own folder, never stikk's — and adds that
     folder to the **user** `PATH` once, as it does for stikk.
   - **An existing prikk is left alone**, reported with its version against the validated range, as today.
   - `uninstall.ps1` **still never removes prikk**; it prints the two steps that do (the folder, the `PATH` entry).
   - **stikk never rebuilds, re-hosts or alters prikk**: it fetches prikk's published file from prikk's release and
     checks it against prikk's published checksum. **When prikk publishes its own `install.ps1`, stikk switches to
     running it**, exactly as on Linux and macOS, and this decision is retired.

   Why the revision is reasonable now: RFC 039 declined to own prikk's install layout. The layout is one folder named
   for prikk, and the alternative is the manual step on the platform where it costs most.

3. **The Windows installer updates the current session's `$env:Path`** as well as the user `PATH`, so `stikk` and
   `prikk` work in the same window; the *"open a new PowerShell window"* step goes. (On Linux and macOS the installer
   runs in a child shell and cannot; that step stays.)

4. **The install page is written for someone who has never opened a terminal.** In order:
   - **How to open one**, per platform, in one or two lines each (macOS: Spotlight, *Terminal*; Windows: Start,
     *PowerShell*; Linux: the *Terminal* app).
   - Paste the one line; **what you should see** when it worked (the installer's last lines), and what to type next.
   - **Updating** is running the same line again; **removing** is the uninstall line.
   - **By hand (advanced)**: the archives, as now, opened by one plain warning — *a file downloaded in a browser is
     checked by macOS or Windows, which will warn because stikk is not signed; the one-line command avoids that* — and
     what to do if it happened (macOS: *Open Anyway* in Privacy & Security; Windows: run it from PowerShell rather than
     double-clicking).
   - `README.md`'s Install keeps only the two lines and links the page. The stale pin example (`--version 0.51.0`
     beside *"tested through 0.53.0"*) is fixed while there.

5. **A Homebrew tap is the one package manager worth adding now — if the owner creates it** (O2). A **formula, not a
   cask** (casks are quarantined and unsigned ones are deprecated), shipping the prebuilt archives for macOS and Linux,
   **pushed by `release.yml` after the assets are published**, with a fine-grained token that can write only the tap.
   **It is not a novice's first path** — installing Homebrew is harder than our one line — its value is `brew upgrade`
   for people who already use Homebrew. Whether it may also carry **prikk** is O3; if not, the formula's caveats say how
   to install prikk. **Homebrew 6's tap trust** (a fully qualified `brew install` trusts that formula; whether it also
   trusts a dependency in the same tap is unconfirmed) **is measured before anything is documented.**

6. **Not now, each with the condition that would change it:**
   - **winget**: per-version human review against a weekly cadence, a classic token, and no novice gain unless prikk
     is there too. **Revisit at 1.0 or a slower cadence, together with prikk.**
   - **Scoop**: its own bucket asks a novice to install Scoop first, a step more than `irm | iex`; main is out of reach.
   - **homebrew-core, nixpkgs, Debian, Fedora**: not eligible (F3). **Revisit when the project meets the thresholds.**
   - **.deb/.rpm, an apt repository, the AUR**: Linux already has one line. Revisit on user demand.
   - **Snap, Flatpak**: ruled out by confinement and policy, not by effort.
   - **cargo-dist**: would replace what exists and is tested.

7. **No signing spend now.** Signing helps only browser downloads (F2), which decision 4 steers away from. **It becomes
   worth paying for when stikk ships something a user double-clicks** — the GUI, an `.app`, an `.msi` — or users
   report browser-download trouble. The costs recorded for that day: Apple Developer Program **$99/year** (individual
   enrolment possible; notarization from Linux CI via rcodesign); Windows: **SignPath Foundation, free** for OSS with
   per-release approval and a code-signing-policy page, or **Azure Artifact Signing ≈ $9.99/month** (individuals: US
   and Canada only), or an OV certificate $150–300/year on a hardware token.

8. **Three items for prikk are held, not sent** (`.git-exclude/upstream/prikk/hold-for-next-letter.md`): an
   `install.ps1` (which retires decision 2), an x86_64-apple-darwin build its installer accepts (the Intel-Mac gap
   stikk cannot close), and binstall's Windows override (M4, with the one-line fix). None blocks this RFC.

## For the owner

| | Question | Recommendation |
|---|---|---|
| **O1** | Accept decision 2: stikk's Windows installer installs prikk's own published build, verified, until prikk has an installer. | **Yes.** It removes the manual step on the platform where it costs most. |
| **O2** | Homebrew tap: create the public, empty repository **`prikk-vcs/homebrew-tap`**, and in `prikk-vcs/stikk` a secret **`HOMEBREW_TAP_TOKEN`**: a fine-grained token with *Contents: read and write* on that one repository and nothing else. | **Yes, when convenient.** Nothing waits on it; Handoff B starts when it exists. |
| **O3** | May that tap also carry a **prikk** formula (prikk's own release archives and checksums; on Intel Macs it could build prikk from source, which would close that gap for Homebrew users — **unmeasured**, since prikk has never built that target)? It is packaging another team's program. | **Your call** — as owner of both, it needs no letter; I would tell prikk in the next one. |
| **O4** | Signing: confirm no spend until decision 7's condition. | **Confirm.** |

## Changelog

`### Changed`: on Windows, the installer offers to install prikk too, from prikk's own release, and stikk and prikk
work in the same window afterwards. The install page starts from opening a terminal. (Handoff B, if O2: `### Added`,
`brew install prikk-vcs/tap/stikk`.)

## Delivery

- **Handoff A** (decisions 2–4), on acceptance with O1, **for 0.15.0**. `install.ps1` and `uninstall.ps1`;
  `installers.yml`'s Windows leg gains: prikk installed from prikk's **real** release on yes and not on no; **a bad prikk
  checksum installs no prikk** (failing for the checksum reason); an existing prikk untouched; **`stikk --version` and
  `prikk --version` resolve in the same session**; uninstall leaves prikk. **A Linux leg on `ubuntu-22.04-arm`**
  (glibc 2.35) installs the aarch64 build and runs it, settling F4 on real hardware and recording whether the loader
  warning prints. The install page and README.
- **Handoff B** (decision 5), **once O2 exists** — 0.15.0 if it arrives before the cut, otherwise 0.16.0; never a
  reason to hold the cut. A formula template filled by `release.yml` like the installers; `brew install` measured on
  macOS and Linux runners, including tap trust; prikk's formula only if O3.

## What this RFC does not do

- **No package manager besides the tap**, and the tap only on O2 (decision 6 says why, per channel).
- **No signing, no notarization, no new account** unless the owner chooses one.
- **No Intel-Mac prikk**: stikk cannot build or vouch for a prikk binary prikk has never built (held for prikk).
- **No re-hosting or modifying prikk** anywhere.
- **No GUI installer** (`.dmg`, `.msi`): there is nothing to double-click until there is a GUI.

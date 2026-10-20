# RFC 048 — Delivery for users new to all of this

**Status.** **Done 2026-10-20**: Handoffs A (`9ea8487`..`9f91ee9`) and B (`cba431f`, `bbdca15`) delivered on `main`; shipped in **0.15.0**. **Deferred, as roadmap decision points for 0.16.0's scoping:** the Homebrew tap (Handoff C, on the owner's word), SignPath signing (O5), and prikk's Windows installer line at the prikk 0.54.0 re-baseline. **Accepted by the project owner 2026-10-20**, O5 included; [Handoff A](../handoffs/048-delivery-for-new-users/a-installers-and-install-page-handoff-v1.md) delivered on `main` (`9ea8487`..`9f91ee9`); [Handoff B](../handoffs/048-delivery-for-new-users/b-aarch64-static-build-handoff-v1.md) issued for F4, corrected. **Proposed 2026-10-20** by the architect, as 0.15.0's increment 4 (`.git-exclude/release/0-15-0-scope.md`,
authorized), on the project owner's direction of 2026-10-18: *"we will have to consider and design carefully about
delivery this time and in the future. It should be easy enough for unmatured users as possible and reasonable."*
**Revised the same day on the owner's principle** that *"the responsibility for installing core software should not be
placed on peripheral convenient tools"*, which prikk adopted in **letter 026**: each program installs itself. The
first draft's decision 2 (stikk's Windows installer installing prikk's build) is withdrawn, and with it the Linux and
macOS half of RFC 039 decision 4. **No open question.**
Measured on Linux against the published stikk 0.14.0 and prikk 0.53.0 assets:
`.git-exclude/reports/048-delivery/measure048.txt`. Channel facts are sourced, **not measured on macOS or Windows**:
`.git-exclude/reports/048-delivery/channels-research.md`. prikk's side read in the clone (`docs/src/guide/install.md`,
`.github/workflows/release.yml`, `crates/prikk-cli/Cargo.toml`, `ROADMAP.md`) and in letters 025 and 026.
**Tracks.** RFC 039 (the installers; **decision 4 is replaced here, on every platform**), `NFR-R03`, `CON-5`.
**Touches.** `scripts/install.sh`, `scripts/install.ps1`, both uninstallers, `.github/workflows/installers.yml`, the
guide's install page, `README.md`'s Install section; later, if the owner chooses it, `release.yml` and a tap repository.

## Summary

RFC 039 gave every platform one command, and made stikk's installer install prikk too where prikk had an installer to
run. **Measured against a user who has never installed a program from a terminal**, that works end to end on Linux
and Apple-Silicon Macs and stops halfway on Windows, where prikk has a build and no installer.

The first draft of this RFC closed the Windows gap by having stikk's installer install prikk's build. **The owner
asked whose responsibility that is, and the answer is prikk's**: an installer decides where a program lives, how it is
verified and what that claims, how it changes `PATH`, upgrades and uninstalls, and which version a user gets. Two
installers for one program drift apart, and every tool built on prikk would carry the same copy and the same version
pin, changed at every prikk release. **prikk agreed (letter 026) and ships its own Windows installer in 0.54.0.**

So: **each program installs itself.** stikk's installers install stikk, **check** for prikk, and when it is missing or
too old **print prikk's own install command**. The install page starts from opening a terminal and has two steps,
prikk's then stikk's. The RFC also settles **which channels are worth adding and which are not, and why**, and spends
no money.

## Findings

### F1 — where a new user stops today

| Platform | stikk | prikk | Where a novice stops |
|---|---|---|---|
| Linux x86_64 / aarch64 | one line (`curl … \| sh`) | offered, by running prikk's own `install.sh` | nowhere |
| macOS, Apple Silicon | one line | offered, the same way | nowhere |
| macOS, Intel | one line | **prikk publishes no binary** | `cargo install prikk` — needs Rust, compiles |
| Windows x86_64 (ARM64 by emulation) | one line (`irm … \| iex`) | **prikk publishes no installer** (until 0.54.0) | *"download the zip … put prikk.exe in a folder on your PATH"* (`scripts/install.ps1:167-181`) |

Two smaller things a novice meets:
- **The guide never says how to open a terminal.** It defines one (*"the window where you type commands"*) and moves on.
- **On Windows the installer ends with "open a new PowerShell window"**, because it changes the user `PATH`. But
  `irm … | iex` runs **in the user's own session**, so the installer can update that session's `$env:Path` too and the
  step disappears.

### F2 — the security prompts, and which paths meet them

| Path | macOS Gatekeeper | Windows: Mark-of-the-Web, SmartScreen |
|---|---|---|
| The one-line installers | **none**: curl does not set the quarantine attribute | **none**: PowerShell's web cmdlets do not write Mark-of-the-Web, and stikk is started from a console |
| A Homebrew formula | none (fetched by curl; inferred, not stated by Homebrew) | — |
| An archive downloaded **in a browser** | quarantined. Since Sequoia, no Control-click bypass: System Settings → Privacy & Security → *Open Anyway*, within about an hour, then a password | marked, and **Explorer's *Extract all* carries the mark onto `stikk.exe`**; double-clicking shows SmartScreen. Run from a console it is not checked. **Unblocking the zip before extracting** (Properties → *Unblock*, or `Unblock-File`) removes the mark |

**One Windows gate no path avoids: Smart App Control** (the owner raised Mark-of-the-Web, 2026-10-20; looking into
it found this). On Windows 11 where it is **enforcing**, *"if the app is unsigned or its signature is invalid, it is
treated as untrusted and blocked"* when Microsoft's cloud has no confident verdict, and *"there is currently no way to
bypass Smart App Control protection for individual apps"* — only turning it off
(https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003). It is
**not tied to Mark-of-the-Web**: an unsigned, unknown console program started from a command prompt is refused with a
text error (https://textslashplain.com/2026/04/28/smart-app-control/). It starts in evaluation mode and **turns itself
off for developer-type use** — so the user it does stop is exactly the newcomer this RFC is for. **It applies to
`prikk.exe` as much as to `stikk.exe`**: both are unsigned.

**Sourced, not measured** (channels-research §macOS, §Windows; no Windows 11 machine with it enforcing is available to
us, and GitHub's Windows Server runners do not have it). So: **on macOS, signing helps only a browser download, which
the guide can steer away from. On Windows, Mark-of-the-Web is the same, but Smart App Control is answered only by
signing**, or by a reputation stikk cannot earn on a weekly cadence. The manual section today mentions the quarantine
only in a macOS aside and Windows not at all.

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

### F4 — the 64-bit ARM Linux build does not start below glibc 2.39 (corrected 2026-10-20)

**As first written, this finding was wrong for aarch64.** It reasoned that the `GLIBC_2.39` references are weak
symbols (`pidfd_spawnp`, `pidfd_getpid`), so the hard floor is `GLIBC_2.34`, and it measured that under Debian 12's glibc
2.36 — by invoking that glibc's loader explicitly, on the **x86_64** build only.

**Handoff A's `ubuntu-22.04-arm` job (glibc 2.35) measured the aarch64 build on real hardware: it does not start.**
*"version `GLIBC_2.39' not found"*, stdout empty. The loader checks the **version-requirement entry**
(`.gnu.version_r`), not the symbols' binding, and skips only an entry flagged weak: in the same 0.14.0 release the
x86_64 build's `GLIBC_2.39` entry is flagged `WEAK` and the aarch64 build's is **`Flags: none`** — visible in M2's own
`readelf -V` output, and misread then. (`.git-exclude/reports/048-delivery/measure048-f4-aarch64.txt`.)

So **every aarch64 Linux below glibc 2.39** — Debian 12 and Raspberry Pi OS bookworm (2.36), Ubuntu 22.04 (2.35) — got
a stikk that would not start, because `install.sh` installs the glibc build there (no aarch64 musl build exists). The
requirement comes from building on `ubuntu-24.04-arm`. **x86_64 is unaffected**: the installer takes the static musl
build. **prikk is unaffected**: its aarch64 build requires only `GLIBC_2.34` (measured, prikk 0.53.0; it does not link
`pidfd_spawnp`). Decision 9 fixes it.

### F5 — prikk's side

- **No Windows installer today; one is scheduled for prikk 0.54.0** (letter 026): `install.ps1` and `uninstall.ps1`,
  into `%LOCALAPPDATA%\Programs\prikk`, verifying the checksum and saying what it proves.
- **No Intel-Mac binary**, and none planned until a runner proves it (`release.yml:39-43`; letter 025). prikk's
  `install.sh` refuses that platform itself and says what to do instead.
- **`cargo binstall prikk` finds nothing on Windows** (M4); prikk fixes it in 0.54.0 (letter 025).
- Nothing of prikk's is signed (`install.md:130`); signing is prikk's owner's decision, not scheduled (letter 025).

### F6 — why one installer per program (owner, 2026-10-20; prikk, letter 026)

What RFC 039 decision 4 had stikk's installer do on Linux and macOS was already more than *offering*: it downloaded
prikk's installer, checked prikk's checksum, and ran it **with stikk's version pin and stikk's install folder**
(`scripts/install.sh:186-220`). Each of those is prikk's decision, and each is copied into every tool that does the
same:
- **where prikk lives, how it is verified and what that claims, how it upgrades and uninstalls** — prikk's release
  design, which may change (for example with signing) and must change in one place;
- **which version a user runs** — two tools pinning different prikk versions would move it back and forth on every
  install. stikk already handles a prikk newer than it has checked: it runs and says so (`NFR-R03`);
- **a stikk change at every prikk release**, and the same for every other tool built on prikk.

## Decisions

1. **The one-line installers stay the first path for stikk on every platform, and every other channel is in
   addition.** They ask for no account, no tool installed first and no administrator password; neither meets
   Gatekeeper or Mark-of-the-Web (F2); running the same line again upgrades. (Smart App Control meets every path
   alike: decision 7.)

2. **Each program installs itself: stikk's installers install nothing of prikk's, on every platform** — replacing RFC
   039 decision 4 (F6). They:
   - **check** whether `prikk` is on `PATH`, and report its version against the range this stikk was tested with
     (*inside*, *newer than tested — it still runs and stikk will say so*, or **too old**: below the floor);
   - when prikk is **missing or too old**, say that stikk needs prikk and **print prikk's own install command** for
     that system: `curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh` on Linux and
     macOS; on Windows, `irm https://github.com/prikk-vcs/prikk/releases/latest/download/install.ps1 | iex` **once prikk
     0.54.0 publishes it**, and until then prikk's release page and install guide (prikk's steps, not a copy of them);
   - **never download or run anything of prikk's**, and carry **no prikk version pin**: prikk's installer installs what
     prikk publishes. The tested ceiling is used only to word the version report;
   - leave **which platforms prikk supports** to prikk's installer, which says so itself (an Intel Mac is refused by
     prikk's `install.sh` with prikk's own advice);
   - finish with stikk installed and exit 0 either way: prikk missing is reported, not an error.

   **`--with-prikk` / `--no-prikk` and `-WithPrikk` / `-NoPrikk` are removed** (no user depends on them). The
   uninstallers never touched prikk and still do not; they say that prikk is removed by prikk's own uninstaller.

3. **The Windows installer updates the current session's `$env:Path`** as well as the user `PATH`, so `stikk` works in
   the same window; the *"open a new PowerShell window"* step goes. (On Linux and macOS the installer runs in a child
   shell and cannot; that step stays.)

4. **The install page is written for someone who has never opened a terminal.** In order:
   - **How to open one**, per platform, in one or two lines each (macOS: Spotlight, *Terminal*; Windows: Start,
     *PowerShell*; Linux: the *Terminal* app).
   - **Step 1, prikk**: prikk's one-line command for that system, **linked to prikk's install guide as the authority**;
     anything longer (Windows before prikk 0.54.0, an Intel Mac) is a link to prikk's guide, never a copy of it.
   - **Step 2, stikk**: our one line, and **what you should see** when it worked (the installer's last lines), then
     what to type next.
   - **Updating** is running each program's line again; **removing** is each program's uninstall line.
   - **By hand (advanced)**: the archives, as now, opened by one plain warning — *a file downloaded in a browser is
     marked by macOS or Windows, which will warn because stikk is not signed; the one-line command avoids that* — and
     the mitigations, each one step: **macOS**, *Open Anyway* in Privacy & Security (or `xattr -d
     com.apple.quarantine <file>`); **Windows**, **unblock the zip before extracting it** (right-click → Properties →
     *Unblock*, or `Unblock-File <zip>`), and start `stikk` from PowerShell, not by double-clicking.
   - **The page never tells a user to download `install.ps1` and run it as a file**: a downloaded script is marked,
     and PowerShell's execution policy then refuses it. `irm … | iex` is not a file and is not refused.
   - **If Windows says Smart App Control blocked stikk or prikk**: one plain paragraph saying what it is and why it
     happens (both programs are unsigned), with Microsoft's own page. **stikk does not tell a newcomer to turn a
     security feature off**; the choice and its consequences are on Microsoft's page.
   - `README.md`'s Install keeps the two steps and links the page. The stale pin example (`--version 0.51.0` beside
     *"tested through 0.53.0"*) is fixed while there.

5. **A Homebrew tap is the one package manager worth adding — when the owner is ready** (O2). A **formula, not a cask**
   (casks are quarantined and unsigned ones are deprecated), shipping the prebuilt archives for macOS and Linux,
   **pushed by `release.yml` after the assets are published**, with a fine-grained token that can write only the tap.
   **It is not a novice's first path** — installing Homebrew is harder than our one line — its value is `brew upgrade`
   for people who already use Homebrew. **A prikk formula is prikk's to publish** (decision 2's principle); stikk's
   formula depends on it only if prikk publishes one, and otherwise its caveats print prikk's install command.
   **Homebrew 6's tap trust is measured before anything is documented.**

6. **Not now, each with the condition that would change it:**
   - **winget**: per-version human review against a weekly cadence, a classic token, and no novice gain unless prikk
     is there too. **Revisit at 1.0 or a slower cadence, together with prikk.**
   - **Scoop**: its own bucket asks a novice to install Scoop first, a step more than `irm | iex`; main is out of reach.
   - **homebrew-core, nixpkgs, Debian, Fedora**: not eligible (F3). **Revisit when the project meets the thresholds.**
   - **.deb/.rpm, an apt repository, the AUR**: Linux already has one line. Revisit on user demand.
   - **Snap, Flatpak**: ruled out by confinement and policy, not by effort.
   - **cargo-dist**: would replace what exists and is tested.

7. **No paid signing** (owner, 2026-10-20: no capacity now).
   - **macOS**: no signing until stikk ships something a user double-clicks — the GUI, an `.app`. Then the Apple
     Developer Program, **$99/year** (individual enrolment; notarization from Linux CI via rcodesign).
   - **Windows**: Mark-of-the-Web is mitigated by decision 4. **Smart App Control is not, and the free route is the
     SignPath Foundation**: free for OSS; it asks for an OSI licence, a released and maintained project, builds from
     this repository in CI, MFA, **a team member approving each signing request**, and a code-signing-policy page
     (https://signpath.org/terms). **Implement or drop is decided when 0.16.0 is scoped** (O5). Paid alternatives, recorded only: Azure Artifact Signing
     ≈ $9.99/month (individuals: US and Canada only), an OV certificate $150–300/year on a hardware token.
   - **Signing stikk alone does not finish the path**: `prikk.exe` meets the same gate, and signing it is prikk's
     owner's decision (letter 025). prikk asked to hear what we decide.

8. **prikk's part, as agreed** (letters 022, 025, 026): its own Windows installer and the `cargo binstall` fix in
   **0.54.0**, in the folder already named. **stikk's re-baseline to prikk 0.54.0 switches the Windows message** to
   prikk's `irm` line — or Handoff A does, if 0.54.0 is published before it is submitted.

9. **64-bit ARM Linux gets a static musl build, and the installer takes it** (F4, corrected), as x86_64 already
   does. `release.yml` publishes `aarch64-unknown-linux-musl` beside the glibc archive, which stays. The
   `ubuntu-22.04-arm` installer job builds and installs that archive at every commit and requires that stikk starts
   with nothing on stderr. Added 2026-10-20 by the architect, within this RFC's scope; Handoff B.

## For the owner

| | Question | Answer (owner, 2026-10-20) |
|---|---|---|
| **O1** | stikk's Windows installer installs prikk's build. | **Withdrawn on the owner's principle**: each program installs itself. prikk agreed (letter 026) and ships its own Windows installer in 0.54.0. The same principle removes stikk's running of prikk's installer on Linux and macOS (decision 2). |
| **O2** | Homebrew tap: the repository `prikk-vcs/homebrew-tap` and a secret `HOMEBREW_TAP_TOKEN` (fine-grained, *Contents: read and write* on that one repository). | **Considered, not ready.** Handoff C waits on it; not in 0.15.0. |
| **O3** | May that tap carry a **prikk** formula? | **It is prikk's to publish** (decision 2's principle). |
| **O4** | Signing spend. | **No capacity to pay now.** Mark-of-the-Web is an issue, with ways to mitigate it (decision 4). |
| **O5** | Apply to the SignPath Foundation for free Windows signing (decision 7) — now, or later? Each release then needs one approval of its signing request. | **After 0.15.0, managed as a milestone: implement or drop, decided when 0.16.0 is scoped** (`ROADMAP.md`). Tell prikk what we decide, as they asked. |

## Changelog

`### Changed`: the installers install stikk only. They check for prikk and, when it is missing or too old, print prikk's
own install command; prikk's installer decides prikk's version and location. On Windows, stikk works in the same window
after installing. The install page starts from opening a terminal. `### Removed`: the installers' `--with-prikk` /
`--no-prikk` (`-WithPrikk` / `-NoPrikk`). (Handoff C, if O2: `### Added`, `brew install prikk-vcs/tap/stikk`.)

## Delivery

- **Handoff A** (decisions 2–4), **on acceptance, for 0.15.0.** All four scripts. `installers.yml` on Linux, macOS and
  Windows: **prikk missing** → stikk installed, prikk's command printed, exit 0, **and nothing of prikk's downloaded or
  written** (no prikk on `PATH` afterwards, no prikk folder); **prikk present** inside the range, and **below the
  floor** (a fake `prikk` printing an old version) → the matching report; the removed flags rejected or ignored as the
  handoff says; uninstall leaves prikk; on Windows, **`stikk --version` resolves in the same session**. **A Linux leg on
  `ubuntu-22.04-arm`** (glibc 2.35) installs the aarch64 build and runs it, settling F4 on real hardware and recording
  whether the loader warning prints. The install page and README.
- **Handoff B** (decision 9), **for 0.15.0**: the aarch64 static build, the installer's target, and the
  `ubuntu-22.04-arm` job made a hard requirement.
- **Handoff C** (decision 5), **once O2 exists**; not in 0.15.0. A formula template filled by `release.yml` like the
  installers; `brew install` measured on macOS and Linux runners, including tap trust.

## What this RFC does not do

- **No installing, pinning, re-hosting or modifying prikk**, anywhere: prikk installs itself.
- **No package manager besides the tap**, and the tap only on O2 (decision 6 says why, per channel).
- **No paid signing, no notarization, no new account** unless the owner chooses one (O5 is free, and still the
  owner's).
- **No GUI installer** (`.dmg`, `.msi`): there is nothing to double-click until there is a GUI.

## Delivered

**Two handoffs, seven commits, green on `main` at `bbdca15`**: CI, Docs, and `installers.yml` on Linux, macOS,
Windows and `ubuntu-22.04-arm`.

- **A, the installers and the page** (`9ea8487`, `152f761`, `7665fbd`, `8e65a8d`, `9f91ee9`). Both installers install
  stikk only and **report prikk in six rows**: missing, too old, unreadable, inside the tested range, newer, another
  major. Each row says what stikk will really do with that prikk (measured: read-only below the floor and for another
  major; refusal for an unreadable version), and prints **prikk's own** command where prikk is the fix. Nothing of
  prikk's is downloaded, verified, pinned or run — a `curl` wrapper leg proves no prikk asset is requested. The floor and
  ceiling are derived; the four prikk flags are gone and fail loudly. Windows puts stikk on the current session's
  `PATH`. The install page starts from opening a terminal, takes prikk then stikk, and carries the browser-download
  mitigations and a Smart App Control paragraph that never advises turning it off. **Caught in review**: prikk's guide
  URL redirecting to plain `http`, a version row promising stikk "still runs" when it refuses, a prikk 1.x called "too
  old", a Windows assertion left on the old URL, and a promise of a stikk that may not exist.
- **B, the static ARM build** (`cba431f`, `bbdca15`). **F4 was wrong for aarch64** and A's ARM job found it: the
  glibc build does not start below glibc 2.39. `release.yml` adds `aarch64-unknown-linux-musl` (the glibc archive stays),
  the installer takes it, and the ARM job builds, installs and runs it on glibc 2.35 at every change, requiring no
  interpreter, no shared library, no glibc version requirement and **an empty stderr**. Measured green on real hardware.
- **Not code**: the channel survey (decision 6), no paid signing (decision 7), and prikk's agreement that each program
  installs itself (letters 022, 025, 026), with prikk's Windows installer and binstall fix due in prikk 0.54.0.

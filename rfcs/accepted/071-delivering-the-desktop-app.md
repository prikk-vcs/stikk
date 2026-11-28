# RFC 071 — Delivering the desktop app

**Status.** **Accepted by the project owner 2026-11-28** (*"Accepted."*); [Handoff A](../handoffs/071-delivering-the-desktop-app/a-the-delivery-measurements-handoff-v1.md). **Proposed 2026-11-28** by the architect, as 0.23.0's increment 3 (`.git-exclude/release/0-23-0-scope.md`,
revision 2, authorized by the project owner 2026-11-25: *"design only … built in the next release"*).
**The owner's answers, 2026-11-28:**
- **Q1:** agreed, with a careful comparison against the other candidate pairs, given under Q1;
- **Q2:** agreed, Flathub included. The owner proposed **the Microsoft Store**, and asked what winget and Homebrew are.
  Both are answered under Q2;
- **Q3:** ruled. **No paid signing and no notarization.**

**Measured** on `main` at `45340b2`: the release workflows, the installers, the AUR packages, and RFC 065's M8.
**Tracks.**
- RFC 065's M8 (packaging), and its outcome;
- RFC 048, which made the Linux installer take a static musl binary because a glibc build carries its runner's glibc;
- RFC 070, which builds the desktop app (`stikk-gui`, unpublished in 0.23.0);
- the carried decision points in `ROADMAP.md`: SignPath signing, tied to prikk's own; the Homebrew tap.

**Touches.** Nothing in 0.23.0. This RFC designs, and its handoff measures. The release that publishes the desktop
app builds it:
- `.github/workflows/build-release.yml` and `installers.yml`;
- `scripts/install.sh` and `install.ps1`;
- `packaging/aur/`;
- a new `packaging/desktop/` (the menu entry and the icons);
- `docs/src/guide/install.md`.

## Summary

**The desktop app is for people who would never type `curl … | sh`.** Getting it should be as ordinary as getting any
other desktop program: from a menu entry, with an icon, from a source the person already trusts. It has to start the
first time, on the systems they actually have. Three things stand between today's release and that:
1. **A Linux desktop app can't be the static build stikk ships today.** RFC 065 measured that a static musl GUI binary
   can't open a window, because winit loads the Wayland and X11 libraries at run time. So it must be a glibc build,
   and a glibc build needs a system at least as new as the one it was built on. On ARM that already excludes Debian
   12 (RFC 048).
2. **A desktop app needs a menu entry and an icon** on each system. stikk ships neither today.
3. **Windows and macOS warn about unsigned programs,** and a warning is where a novice stops.

This RFC decides how each is met, and what's measured before the release that publishes the app. **The publishing
release builds it.**

## Findings

### F1 — what stikk ships today (measured in the workflows)

- **Release archives** for seven targets: Linux x86_64 and aarch64, each as gnu and musl; macOS aarch64 and x86_64;
  Windows x86_64 (msvc). All are built with `-p stikk`.
- **The one-line installers:** `install.sh` (Linux, macOS) installs the **musl** binary on Linux, on both
  architectures (RFC 048). `install.ps1` installs into the user's own directory. **Neither creates a menu entry.**
- **The AUR:** `stikk` (source) and `stikk-bin`. Pushed by every release.
- **prikk is installed by prikk's own installer.** stikk's installers check for it and print prikk's command.

### F2 — what RFC 065 measured about the desktop app's binary (M8)

- **Linux gnu:** opens its window. **Its glibc floor is the build machine's:** 2.44 on the owner's machine, set by
  `cosh` and `sinh`. The code needs nothing newer.
- **Linux musl:** builds, and **can't open a window** (`NoWaylandLib`). That holds for every winit toolkit.
- **Run-time libraries loaded by name, not linked:** libwayland-client, libxkbcommon, libEGL/libGL/libvulkan,
  libX11/libxcb/libXcursor/libXi. A desktop system has them. A minimal or server system may not.
- **Windows:** builds as a **console-subsystem** `.exe`, so it opens a console window beside its own.
- **aarch64 Linux** wasn't measured: there's no cross linker here.

### F3 — the glibc floor depends on how and where the build runs

- **GitHub's `ubuntu-latest` is moving to 26.04** (RFC 069 Handoff A found the change). A gnu build there carries a
  floor newer than most of the systems users have.
- **The choices for a lower floor, unmeasured for this binary:**
  - an older runner image (`ubuntu-22.04`, glibc 2.35), which GitHub will retire as it retired `macos-14` mid-release;
  - a pinned container image on any runner (for example Debian 12, glibc 2.36);
  - a build tool that links against an older glibc's symbol set without a container.

**The floor each yields, and whether the result starts on a real Debian 12, are this RFC's first measurement.**

### F4 — what a person sees on first start

- **Windows:** an unsigned `.exe` downloaded from the internet gets SmartScreen's *"Windows protected your PC"*. The
  way past it is *More info*, then *Run anyway*. The owner met the same with prikk's kit.
- **macOS:** an unsigned, un-notarized app gets Gatekeeper's refusal. The way past it is in System Settings. Apple's
  notarization needs a paid developer account.
- **Linux:** no such gate. But a program that isn't in the menu effectively doesn't exist for a novice.

## Decisions

### 1. The Linux build: glibc, at a measured, pinned floor

- **The desktop app ships as a glibc build only,** on both architectures. Static musl can't open a window (F2).
- **The build method is the one with the lowest floor that starts on a real Debian 12,** chosen from F3's three by
  Handoff A's measurement.
  - **It's pinned:** an image by digest, or a tool by exact version. A runner retirement then can't change it.
- **The floor is printed in the install guide and checked by the installer,** which says so in plain words and stops
  before installing a binary that wouldn't start.
- **The terminal binary is unchanged:** the installer keeps taking musl for `stikk` (RFC 048).

### 2. Run-time libraries are checked, not assumed

- **The installer checks** that the libraries F2 lists can be found (`ldconfig -p`, or the platform's equivalent). If
  one is missing, it names it, with the package that provides it on the common distributions, and doesn't install
  the desktop app.
- **The AUR packages declare them as dependencies.**

### 3. Windows: a GUI-subsystem binary, and a Start menu entry

- **`stikk-gui` builds as a GUI-subsystem binary** (`#![windows_subsystem = "windows"]`), so no console opens beside
  it. The terminal `stikk.exe` stays a console program.
- **`install.ps1` adds a per-user Start menu shortcut** with the app's icon, and the uninstaller removes it. There's no
  machine-wide install and no administrator prompt, as today.
- **The icon is embedded in the `.exe`.**

### 4. macOS: an app bundle

- **The desktop app ships as an `.app` bundle** (`Info.plist`, the icon as `.icns`), in a zip archive, for aarch64
  and x86_64.
- **It isn't notarized** (Q3). **It's ad-hoc signed** in CI (`codesign --sign -`, which is free), and checked with
  `codesign --verify`, because Apple Silicon won't run arm64 code without at least that.
- **The install guide says plainly what Gatekeeper will say,** and how to open it. Since macOS 15 the way is
  System Settings → Privacy & Security → *Open Anyway*, after one refused start. Control-click → *Open* no longer
  works.
- **`install.sh` also installs the `.app`** into `~/Applications`. A file fetched by `curl` carries no quarantine
  mark, so Gatekeeper doesn't stop it. That's offered in the guide beside the download, as one line to paste.
- **Unverified on a Mac:** there's no Mac here. The bundle is built and checked for structure on GitHub's macOS
  runner, and the guide says it hasn't been tried on a real Mac.

### 5. Linux: a menu entry, and icons

- **A `.desktop` file** (freedesktop's Desktop Entry spec) and **icons in the hicolor theme's sizes,** plus a scalable
  SVG.
- **One app ID everywhere,** from the first release: the `.desktop` file's name, the window's Wayland `app_id`, and
  later the stores' (Q1). A Wayland desktop finds a window's icon by matching the two, so they can't differ. Changing
  the ID later breaks the launchers people have pinned.
- `install.sh` puts them in the user's own data directory (`~/.local/share/applications`, `…/icons/hicolor`). The
  uninstaller removes them. The AUR packages install them system-wide.
- **Validated with `desktop-file-validate`** in CI.

### 6. prikk stays prikk's to install

The desktop app needs prikk as the terminal does. **The installers check for it and print prikk's own command,** as
today (F1). If prikk is missing when the desktop app starts, the app says so in its first screen, in plain words,
with that command.

### 7. What Handoff A measures (the design is final only after it)

| Question | How |
|---|---|
| The glibc floor of the desktop app's binary, per build method (F3), on x86_64 and aarch64 | `objdump -T`, the highest `GLIBC_` version, and the symbols that set it |
| Does the chosen build start on a real Debian 12? | a pinned `debian:12` container on the runner, with a virtual display (`Xvfb`), the probe of RFC 070 Handoff A §1: it opens, reports its viewport, exits 0 |
| Which run-time libraries a minimal Debian 12, Ubuntu 22.04 and Fedora lack | the same containers, minimal, before and after installing the desktop packages |
| The GUI-subsystem `.exe` | built on `windows-latest`; `file` or `dumpbin` shows the subsystem; the probe starts it |
| The `.app` bundle | built on `macos-latest`; its structure checked; ad-hoc signed, and `codesign --verify` clean; the probe started from inside it |
| The `.desktop` file | `desktop-file-validate`, clean |

## The owner's questions, answered

### Q1 — What's the desktop app called? (agreed, compared)

**Agreed:**
- **"stikk"** on the menu, in the window title, and on the Windows and macOS shortcut and bundle names;
- **`stikk-desktop`** as the executable;
- the crate stays `stikk-gui`, which is internal.

**Where a person meets the executable's name.** Not on the menu: the menu and the window say "stikk" whatever the
file is called. Windows' Task Manager shows the `.exe`'s description, which is also "stikk". A person meets it:
- in the download's file name;
- in the AUR package's name;
- in the guide;
- in a terminal, if they use one.

**The pairs compared** (terminal, desktop):

| Pair | For | Against |
|---|---|---|
| **`stikk` + `stikk-desktop`** | the typed command keeps the short name; "desktop" is a word people know; the `-desktop` suffix is the established convention (`signal-desktop`, `github-desktop`, `element-desktop`); `stikk-<Tab>` keeps them together | a longer name, but it's typed rarely: the menu starts the app |
| `stikk` + `stikk-gui` | short; developers know "GUI" | **jargon:** "gui" is the word the download page would show a novice. The convention is older tools (`git-gui`) |
| `stikk-tui` + `stikk-gui` | symmetric | **renames the command every terminal user types,** along with the crates.io package's binary, both AUR packages, the installers and every page of the docs. And "tui" is jargon too. The command typed most deserves the shortest name |
| one `stikk`, with `stikk desktop` | one file | **the terminal's binary would become the desktop's:** glibc-bound, so no musl (decision 1, RFC 048), many times larger, and needing display libraries on servers. A Windows `.exe` can't be a console and a GUI program at once (decision 3) |
| `stikk` + `stikk-app` | short | **`stikk-app` is already an internal crate,** so contributors would confuse the two. And "app" doesn't tell the two apart, since the terminal is an app too |
| `stikk` + `Stikk` | the brand | **the same file on Windows and macOS,** whose file systems ignore case by default |
| `stikk` + `stikkw` | Windows' convention for a windowed twin (`pythonw`) | obscure, and only Windows knows it |

**The app ID** (decision 5): **`io.github.prikk_vcs.stikk`**:
- the reverse-DNS form freedesktop and Flathub require;
- derived from the repository stikk's own `homepage` names;
- with `-` written as `_`, which Flathub and D-Bus need.

`org.prikk.stikk` would need the consent of whoever holds `prikk.org`, which isn't stikk's. **The stores' RFC
confirms the ID before the publishing release.**

### Q2 — Which channels? (agreed; the Microsoft Store added)

**Agreed, for the publishing release:**
- the release archives and the one-line installers, extended as decisions 1–5 say;
- the AUR packages, named `stikk-desktop` and `stikk-desktop-bin` (Q1);
- the install guide, written for a person who has never used a terminal.

**The stores, in one RFC after this one** (RFC 075). Flathub and the Microsoft Store raise the same three questions,
so one design answers them for both:
- **Containment.** A store app runs contained: Flatpak's sandbox, or MSIX's virtualized `AppData`. stikk must run
  prikk and read repositories anywhere in the home folder. **On Windows, a program started by a packaged app may
  or may not see the same `AppData` as the app does.** Reports differ by Windows build and install path, and
  prikk's keys and settings live there. That's measured, not assumed.
- **prikk.** A person who got stikk from a store still needs prikk (decision 6). The store page and the app's first
  screen must make that one plain step.
- **The app ID** (Q1).

**The Microsoft Store, measured for the owner's proposal:**
- **Registration is free for individual developers.** Microsoft dropped the fee from June 2025. It asks for a
  Microsoft account, and an identity check by ID and selfie, in the owner's name. Microsoft's own FAQ says the old
  fee may still apply in some markets or through some sign-up pages, so the owner sees the price before agreeing.
- **The Store signs an MSIX package itself.** A Store install then meets no SmartScreen warning, with no certificate
  bought: that's Q3's Windows answer at no cost. An `.exe` or `.msi` submission would need stikk's own paid
  certificate, so the Store route is MSIX.

**What winget and Homebrew are, and why they're dropped from this RFC:**
- **winget** is Windows' own command-line installer: `winget install <name>`, typed in a terminal. It's free, and
  listed by a pull request to Microsoft's community repository.
- **Homebrew** is the Mac's most used command-line installer, run by volunteers. Its official repository now accepts
  only apps that are signed and notarized by Apple, and it has removed the way to skip that check. **After Q3, the
  desktop app can't be listed there.**
- **Both are typed commands,** so they serve terminal users, not the novice this app is for. **Neither is planned
  for the desktop app.** The terminal's Homebrew tap stays carried, as before.

### Q3 — Signing (ruled)

**Ruled by the owner, 2026-11-28: no paid signing and no notarization.** What follows from it:
- **macOS:** the `.app` ships ad-hoc signed and **not notarized,** for good (decision 4):
  - the download page and the guide say so plainly;
  - the guide shows *Open Anyway* in System Settings, with a picture, and offers `install.sh`'s one line, which
    Gatekeeper doesn't stop.
- **Windows, a direct download:** unsigned. **The guide shows SmartScreen's two clicks,** *More info* then *Run
  anyway*, with a picture.
- **Windows, the Store:** signed by the Store, for free (Q2).
- **SignPath stays carried,** with prikk's own signing decision. Its programme for open-source projects is free, so
  the ruling doesn't close it.

## What this RFC does not do

- **No build, no installer change, no release change in 0.23.0.** The publishing release builds it.
- **No store design** (Q2): Flathub and the Microsoft Store are RFC 075's.
- **No icon artwork.** The icon is drawn in the publishing release's handoff, offered to the owner as two or three
  candidates, and chosen by the owner.

## Evidence plan

Decision 7's table, in **Handoff A**, issued on acceptance:
- **Containers and CI runners only.** Nothing on the owner's desktop, and no Windows session of the owner's.
- Each measurement is a script whose output is the record.

## Delivery

**Handoff A (measurements) after acceptance.** Then the decisions are amended from what it measured, and the
publishing release's own handoff builds them.

## Changelog

None in 0.23.0.

# Handoff A — RFC 071: the delivery measurements (v1)

**Companion to:** [RFC 071](../../accepted/071-delivering-the-desktop-app.md).
**Issued** 2026-11-28. **0.23.0 increment 3: measurements only.** Nothing is built for users, and nothing ships.

> **What this is.** RFC 071 decision 7's table, measured on GitHub's runners and in containers on them. The answers
> settle the decisions, which the architect then amends. The release that publishes the desktop app builds them.
>
> **Starts when RFC 070 Handoff A's probe** (`crates/stikk-gui/examples/window_probe.rs`, its §1) **is on `main`.**
> Every measurement here is taken on that binary, the real path, not on RFC 065's prototype.
>
> **Sanctioned:**
> - a new `.github/workflows/delivery-probe.yml`, **`workflow_dispatch` only**, which builds and measures and
>   publishes nothing;
> - a new `.git-exclude/reports/071-delivering-the-desktop-app/`.
>
> **Not sanctioned:**
> - any crate, `ci.yml`, the release workflows, the installers, `packaging/`, the docs;
> - any release, tag or upload.
>
> **Nothing runs on the owner's desktop:** no nested compositor, no window there, no Windows session of the
> owner's.

## 0. Read first

- **RFC 071, whole,** with the owner's answers: the executable `stikk-desktop`, the app ID
  `io.github.prikk_vcs.stikk`, and no paid signing.
- **RFC 065's report, M8** (`.git-exclude/reports/065-gui-toolkit/snora/report.md`): the musl failure, the 2.44
  floor set by `cosh` and `sinh`, and the libraries loaded by name.
- **RFC 048,** for why the terminal's Linux installer takes musl, and its aarch64 lesson: **a measurement covers only
  its architecture, its loader and its entry point.**
- **RFC 070 Handoff A §1,** the probe's contract: it opens, it reports its viewport, and it exits 0 or 1.

**The record rule:** each measurement is a workflow step whose log is the record. **The report quotes the run's
log and its URL,** and adds nothing typed by hand. A case that's missing is added to the workflow and run again.

**The repeat rule:** a runner that fails is reported, not retried in a loop. One re-run is allowed for a failure
outside stikk (a download, a runner fault). Say so when you use it.

## 1. The glibc floor, per build method (decision 1, F3)

Build the probe in **release** mode by each method, on **x86_64 and aarch64**:

| Method | x86_64 | aarch64 |
|---|---|---|
| an older runner | `ubuntu-22.04` | `ubuntu-22.04-arm` |
| a container pinned by digest | `debian:12` on `ubuntu-latest` | `debian:12` on `ubuntu-24.04-arm` |
| a tool linking an older symbol set | `cargo zigbuild` to `x86_64-unknown-linux-gnu.2.31` | the same to `aarch64-unknown-linux-gnu.2.31` |

For each one, record:
- **the floor:** `objdump -T`, the highest `GLIBC_` version, and the symbols that need it;
- **the pin:** the image's digest, or the tool's exact version and Zig's;
- **whether it built at all,** and the error if it didn't. A failed method is a finding, not a gap.

**The run that fails it:** the same probe built on `ubuntu-latest` itself. Its floor must come out newer than
Debian 12's 2.36, or F3's premise is wrong. Say which.

## 2. Does it start on Debian 12? (decision 1)

For **each build that §1 shows has a floor of 2.36 or lower,** on its own architecture:
- **a `debian:12` container,** pinned by digest, with the libraries a desktop has (the list §3 finds), and `Xvfb`;
- **the probe, started under `DISPLAY` with no keys sent.** Record its whole log, its exit code, and **which renderer
  drew** (wgpu on a software adapter, or iced's software fallback), from the log;
- **the run that fails it:** §1's `ubuntu-latest` build in the same container. It must refuse to start with glibc's
  own `version 'GLIBC_2.xx' not found`. Quote it.

**Wayland isn't measured in a container.** Say so in the report. RFC 065 measured it on a real desktop.

## 3. The libraries a minimal system lacks (decision 2)

**Containers,** each pinned by digest, **x86_64:**
- `debian:12`;
- `ubuntu:22.04`;
- the current Fedora release (name it);
- `archlinux:base`, for the AUR packages' dependencies.

**In each one:**
1. **Minimal, as pulled:**
   - run decision 2's check, `ldconfig -p`, for each library F2 lists, and record found or missing;
   - then start the probe, and record how it fails. **This is the run that fails decision 2's check:** the check
     must report missing for each library the probe then fails on.
2. **Add packages one at a time** until the probe starts under `Xvfb`. Record each package's name and version, and
   the check's answer after each.
3. **The result is each distribution's smallest package list.** It's what the installer names when a library is
   missing, and what the AUR packages declare.

**Also say** whether `ldconfig` is on `PATH` for an ordinary user in each one, or only as `/sbin/ldconfig`.

## 4. Windows: the GUI-subsystem `.exe`, and a Start menu entry (decision 3)

On `windows-latest`:
- **The subsystem,** with no code change: build the probe with
  `RUSTFLAGS="-C link-arg=/SUBSYSTEM:WINDOWS -C link-arg=/ENTRY:mainCRTStartup"`.
  - **Record `dumpbin /headers`'** subsystem line for it, and for `stikk.exe` built as today. **That's the run that
    fails it:** the terminal must stay `Windows CUI`.
  - **Start it with its output redirected to a file,** and record whether the log arrives. A GUI-subsystem program
    has no console of its own, so this is a finding either way.
- **The Start menu shortcut,** by PowerShell's own means (`WScript.Shell`'s `CreateShortcut`):
  - create `stikk.lnk` under `$env:APPDATA\Microsoft\Windows\Start Menu\Programs`, pointing at the probe;
  - read it back, and record its target;
  - then remove it, and show it's gone.

  **No administrator rights.** Record the user the step ran as.
- **Not measured here:** the embedded icon, which needs the artwork (RFC 071 *What this RFC does not do*).

## 5. macOS: the `.app` bundle (decision 4)

On `macos-latest`:
- **Assemble `stikk.app`:**
  - `Contents/MacOS/stikk-desktop`, the probe;
  - `Contents/Info.plist`, with `CFBundleIdentifier` = `io.github.prikk_vcs.stikk`, `CFBundleExecutable` =
    `stikk-desktop`, `CFBundleName` = `stikk`, and `CFBundlePackageType` = `APPL`.

  The workflow writes the plist. The report quotes it.
- **Checks:**
  - `plutil -lint`;
  - `codesign --sign - --force` on the bundle, then `codesign --verify --strict --verbose=2`;
  - **the run that fails it:** the same verify on a bundle whose binary was changed after signing.
- **Starting it:**
  - once from `Contents/MacOS/` directly;
  - once through `open -W stikk.app`, which goes through LaunchServices as a double-click does.

  Record both logs and their exit codes.
- **x86_64:** build for `x86_64-apple-darwin` on the same runner. Run it if Rosetta is there. If it isn't, record
  "built, not run".
- **Gatekeeper isn't measured.** A runner's file carries no quarantine mark. Say so: that stays unverified, as
  RFC 071 says.

## 6. The `.desktop` file (decision 5)

- **Write the draft in the workflow:**
  - the file is `io.github.prikk_vcs.stikk.desktop`;
  - `Name=stikk`, `Exec=stikk-desktop`, `Icon=io.github.prikk_vcs.stikk`, `Type=Application`;
  - `Categories=Development;RevisionControl;`, and a `Comment`.

  Quote it in the report.
- **`desktop-file-validate`, clean.** **The run that fails it:** the same file with `Type` removed must be refused.
- **The window's `app_id`:** record, from `stikk-gui`'s source, what the probe sets as `application_id`. **Don't
  change it,** because that's RFC 070's. If it isn't the app ID, say so, and it becomes an item for RFC 070's next
  handoff.

## 7. Submit

`.git-exclude/review-request/071-delivering-the-desktop-app-a/review-request-v1.md`:
- **§1's table filled in:** each method's floor, its symbols, and its pin. Then **the method you'd choose** and why,
  for the architect to rule on;
- §2's start logs, and the failing run;
- §3's per-distribution package lists, with the check's answers;
- §4's and §5's records;
- §6's file and validator output;
- **each run's URL,** and what wasn't measured, and why.

**The workflow is committed, pushed, and run from `main`** before you submit. If your session refuses a push, say so,
and don't commit on top of it.

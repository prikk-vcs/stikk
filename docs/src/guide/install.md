# Install

stikk needs two programs: **prikk** (the version control system itself) and **stikk** (this
program, which shows you what is in a prikk repository). Neither needs a compiler — both ship as
ready-to-run files, and **each one installs itself**: prikk's installer installs prikk, stikk's
installs stikk.

Two commands, in order. If you have never used a terminal, start here.

## Opening a terminal

A *terminal* is a window where you type commands instead of clicking. Every command on this page
is typed there, one line at a time, each followed by Enter.

- **macOS** — press ⌘ Space, type `Terminal`, press Enter.
- **Windows** — press the Start button, type `PowerShell`, and open *Windows PowerShell*.
- **Linux** — open your *Terminal* application (on GNOME: press the Super/Windows key and type
  `Terminal`).

## Step 1 — install prikk

prikk is the version control system; stikk reads the repositories it makes. prikk publishes its
own installer, and [prikk's install guide](https://prikk.org/docs/guide/install.html)
is the authority on it.

**Linux and macOS**:

```sh
curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh
```

**Windows**: prikk does not publish a Windows installer yet — one is coming. Until it does, follow
[prikk's install guide](https://prikk.org/docs/guide/install.html), which has the
steps for Windows; the files are on [prikk's latest release](https://github.com/prikk-vcs/prikk/releases/latest).

Check it worked:

```sh
prikk --version
```

stikk works with **prikk 0.28 and above**. With an older prikk it opens your repository read-only
and says so, rather than changing anything it has not been checked against. A prikk *newer* than
the one this stikk was tested against works normally; stikk just tells you its output has not been
checked rather than assuming it has.

## Step 2 — install stikk

**Linux and macOS**:

```sh
curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh
```

**Windows**, in PowerShell:

```powershell
irm https://github.com/prikk-vcs/stikk/releases/latest/download/install.ps1 | iex
```

It downloads stikk, checks it against the checksum published beside it, installs nothing if that
check fails, and puts stikk somewhere your terminal can find it. It **installs only stikk** — it
checks whether prikk is there and new enough, and tells you prikk's own command if it is not.

### What you should see

```text
stikk-install: downloading stikk-0.14.0-x86_64-unknown-linux-musl.tar.gz (0.14.0, x86_64-unknown-linux-musl)
stikk-install: checking the download
stikk-install: extracting
stikk-install: installed to ~/.local/bin/stikk
stikk-install: added ~/.local/bin to PATH in ~/.bashrc
stikk-install: open a new terminal, or run: . ~/.bashrc
stikk-install: prikk is already installed: prikk 0.53.0
stikk-install: that is inside the range this stikk was tested against (0.28 through 0.53.0)

stikk-install: done. Next:
stikk-install:   1. open a new terminal (PATH changed)
stikk-install:   2. make a repository:  prikk init
stikk-install:   3. open it:            stikk
```

The exact versions will differ. If it says `PATH changed`, open a new terminal before carrying on:
your terminal looks for programs in a fixed list of folders, and it reads that list when it
starts. **On Windows there is no such step** — stikk works in the same window straight away.

Then make a repository and open it:

```sh
prikk init
```

```sh
stikk
```

Press `?` inside stikk for help, and `q` to leave.

## Updating

Run each program's install line again. Both installers overwrite what is there, so running them
twice is safe.

## Removing

Each program removes itself, the same way it installed itself:

```sh
curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/uninstall.sh | sh
```

```powershell
irm https://github.com/prikk-vcs/stikk/releases/latest/download/uninstall.ps1 | iex
```

That removes stikk and the `PATH` entry it added, and nothing else — not your repositories, which
stikk never writes to. To remove prikk, use
[prikk's own uninstaller](https://prikk.org/docs/guide/install.html).

## If Windows says Smart App Control blocked stikk or prikk

Smart App Control is a Windows 11 feature that blocks programs it cannot confidently recognise.
Neither stikk nor prikk is signed with a paid code-signing certificate, so on a machine where
Smart App Control is switched on, either of them may be refused.

There is no per-program exception for it. Microsoft's own page explains what the feature does and
what your options are:
[What is Smart App Control?](https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003)
That decision is yours and Microsoft's; this guide does not advise turning a security feature off.

## By hand (advanced)

You do not need this section if the commands above worked.

> **A file you download in a browser is marked by macOS and by Windows**, which will then warn
> about it or refuse to run it, because stikk is not signed. The one-line commands above do not
> leave that mark, which is why they are the easy path. If you download by hand anyway, the two
> notes below clear it.

The archives are on the [latest release](https://github.com/prikk-vcs/stikk/releases/latest).
Each has a second file beside it ending in `.sha256` — a *checksum*, a short fingerprint you can
use to tell the file arrived exactly as published.

| Your computer | File |
|---|---|
| Linux, Intel/AMD | `stikk-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, Intel/AMD (older systems, or Alpine) | `stikk-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM | `stikk-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| Mac, Apple Silicon (M1 and later) | `stikk-<version>-aarch64-apple-darwin.tar.gz` |
| Mac, Intel | `stikk-<version>-x86_64-apple-darwin.tar.gz` |
| Windows | `stikk-<version>-x86_64-pc-windows-msvc.zip` |

Replace `<version>` with the version on that release page, and `<file>` below with the name from
the table.

**Linux and macOS.** Download both files, check the one against the other, unpack, and install:

```sh
curl -fLO https://github.com/prikk-vcs/stikk/releases/latest/download/<file>
```

```sh
curl -fLO https://github.com/prikk-vcs/stikk/releases/latest/download/<file>.sha256
```

```sh
sha256sum -c <file>.sha256
```

On macOS that last command is `shasum -a 256 -c <file>.sha256` instead. Either prints
`<file>: OK`. **If it says `FAILED`, stop and download again — do not run the file.** Then:

```sh
tar -xzf <file>
```

```sh
mkdir -p ~/.local/bin
```

```sh
install -m755 stikk-<version>-<target>/stikk ~/.local/bin/stikk
```

If `~/.local/bin` is not already on your `PATH`, add `export PATH="$HOME/.local/bin:$PATH"` to your
shell's startup file and open a new terminal.

> **macOS, if you downloaded in a browser.** macOS marks browser downloads and refuses to run an
> unsigned program that carries the mark. Either open System Settings → Privacy & Security and
> press *Open Anyway* after the first refusal, or clear the mark yourself:
>
> ```sh
> xattr -d com.apple.quarantine ~/.local/bin/stikk
> ```

**Windows**, in PowerShell. There is no `sha256sum` here, so you compare the fingerprint yourself:

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/latest/download/<file> -OutFile stikk.zip
```

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/latest/download/<file>.sha256 -OutFile stikk.zip.sha256
```

```powershell
if ((Get-FileHash stikk.zip -Algorithm SHA256).Hash -eq (((Get-Content stikk.zip.sha256 -Raw).Trim() -split '\s+')[0])) { "OK" } else { "FAILED - do not use this file" }
```

```powershell
Expand-Archive stikk.zip -DestinationPath .
```

Move the `stikk.exe` that leaves behind into a folder on your `PATH`.

> **Windows, if you downloaded the zip in a browser: unblock it *before* extracting.** The mark
> travels from the zip onto `stikk.exe` when Explorer extracts it, and then Windows warns every
> time you start the program. Right-click the zip → Properties → tick *Unblock* → OK, or:
>
> ```powershell
> Unblock-File stikk.zip
> ```
>
> Start `stikk` from PowerShell rather than by double-clicking it: it is a terminal program and
> has nothing to show in a window of its own.

**Do not download `install.ps1` and run it as a file.** A downloaded script is marked too, and
PowerShell's execution policy then refuses to run it. The `irm … | iex` line above is not a file
and is not refused.

### Optional: check who built it

The checksum proves the file was not altered on the way to you. It does **not** prove who built it.
If you want that too, and you have [GitHub CLI](https://cli.github.com/):

```sh
gh attestation verify <file> -R prikk-vcs/stikk
```

It exits successfully when the file really was built by this project's release workflow, and fails
otherwise.

## If you already use Rust

```sh
cargo install stikk
```

This compiles stikk from source, so it takes a few minutes and needs a Rust toolchain
(2024 edition, MSRV 1.88). To download the same prebuilt binary the installer uses, without
compiling:

```sh
cargo binstall stikk
```

The one-line installers above need neither.

## Next

[Getting started](./getting-started.md) opens a repository and walks through what stikk shows you.

# Install

stikk needs two programs: **prikk** (the version control system itself) and **stikk** (this
program, which shows you what is in a prikk repository). Neither needs a compiler — both ship as
ready-to-run files. A *terminal* is the window where you type commands; everything below is typed
there.

**One command.** On **Linux and macOS**:

```sh
curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh
```

On **Windows**, in PowerShell:

```powershell
irm https://github.com/prikk-vcs/stikk/releases/latest/download/install.ps1 | iex
```

It downloads stikk, checks it against the checksum published beside it, installs nothing if that
check fails, and puts stikk somewhere your terminal can find it. **If prikk is missing it offers to
install it too**, using prikk's own installer and the prikk version this stikk was tested against —
answer `n` to skip, or pass `--with-prikk` / `--no-prikk` to decide in advance. Run it again any
time to upgrade. On Windows, prikk has no installer of its own, so the script tells you where to
get it rather than guessing.

To remove stikk later:

```sh
curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/uninstall.sh | sh
```

It removes stikk and the `PATH` entry it added, and nothing else. It never removes prikk.

The script is one file and is meant to be read before it is run — open the URL in a browser first
if you would rather see it. The rest of this section is the same thing by hand.

## Or, by hand

### 1. Install prikk

prikk publishes its own installer. One line:

```sh
curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh
```

It downloads prikk, checks the download against the checksum prikk published, installs it into
`~/.local/bin`, and adds that folder to your `PATH` — the list of places your terminal looks for
programs. **If it changed your `PATH`, open a new terminal before carrying on.** Then:

```sh
prikk --version
```

stikk works with prikk **0.28 and above, and is tested through 0.51.0** — which is what the command
above installs today. To ask for exactly that version rather than the newest one:

```sh
curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh -s -- --version 0.51.0
```

**prikk's installer covers Linux and Apple-Silicon Macs only.** On Windows, or on an Intel Mac, use
prikk's own [install guide](https://github.com/prikk-vcs/prikk/blob/main/docs/src/guide/install.md).

### 2. Install stikk

Find the row for your computer — the commands below download that file for you, from the
[latest release](https://github.com/prikk-vcs/stikk/releases/latest):

| Your computer | File to download |
|---|---|
| Linux, Intel/AMD | `stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, Intel/AMD (older systems) | `stikk-0.10.0-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM | `stikk-0.10.0-aarch64-unknown-linux-gnu.tar.gz` |
| Mac, Apple Silicon (M1 and later) | `stikk-0.10.0-aarch64-apple-darwin.tar.gz` |
| Mac, Intel | `stikk-0.10.0-x86_64-apple-darwin.tar.gz` |
| Windows | `stikk-0.10.0-x86_64-pc-windows-msvc.zip` |

Each file has a second file beside it ending in `.sha256`. That is a *checksum*: a short fingerprint
of the download, so you can tell the file arrived exactly as published. The commands below fetch
both. **Use them rather than downloading in a browser** — on macOS a browser download is treated as
untrusted and the program will be refused the first time you run it (see the macOS note below).

**Linux**

```sh
curl -fLO https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz
```

```sh
curl -fLO https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Check the download, so you know it was not altered on the way:

```sh
sha256sum -c stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

That prints `stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz: OK`. **If it says `FAILED`, stop and
download again — do not run the file.** Then unpack it and put it somewhere your terminal looks:

```sh
tar -xzf stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz
```

Make the folder you are going to keep it in, in case it does not exist yet:

```sh
mkdir -p ~/.local/bin
```

```sh
install -m755 stikk-0.10.0-x86_64-unknown-linux-gnu/stikk ~/.local/bin/stikk
```

**macOS** is the same — `curl`, check, `tar -xzf`, `mkdir -p ~/.local/bin`, `install` — with the Mac
file name and one different check command (replace `aarch64-apple-darwin` with
`x86_64-apple-darwin` on an Intel Mac):

```sh
shasum -a 256 -c stikk-0.10.0-aarch64-apple-darwin.tar.gz.sha256
```

**Download with the `curl` commands above, not with a browser.** macOS marks anything a browser
downloads as quarantined, and refuses to run a quarantined program that has not been notarized —
stikk is not — with a warning there is no obvious way past. Files fetched with `curl` are not
marked. If you already used a browser and macOS refuses to open stikk, clear the mark:

```sh
xattr -d com.apple.quarantine ~/.local/bin/stikk
```

**Windows**, in PowerShell. There is no `sha256sum` here, so you compare the fingerprint yourself:

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-pc-windows-msvc.zip -OutFile stikk.zip
```

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-pc-windows-msvc.zip.sha256 -OutFile stikk.zip.sha256
```

```powershell
if ((Get-FileHash stikk.zip -Algorithm SHA256).Hash -eq (((Get-Content stikk.zip.sha256 -Raw).Trim() -split '\s+')[0])) { "OK" } else { "FAILED - do not use this file" }
```

```powershell
Expand-Archive stikk.zip -DestinationPath .
```

That leaves `stikk-0.10.0-x86_64-pc-windows-msvc\stikk.exe`. Move it into a folder on your `PATH`.

If `~/.local/bin` (or the folder you chose) is not already on your `PATH`, add it — on Linux and
macOS, `export PATH="$HOME/.local/bin:$PATH"` in your shell's startup file, then open a new
terminal.

Check it worked:

```sh
stikk --version
```

It prints `stikk 0.10.0`.

### Optional: check who built it

The checksum proves the file was not altered on the way to you. It does **not** prove who built it.
If you want that too, and you have [GitHub CLI](https://cli.github.com/):

```sh
gh attestation verify stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz -R prikk-vcs/stikk
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

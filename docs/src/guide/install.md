# Install

stikk needs two programs: **prikk** (the version control system itself) and **stikk** (this
program, which shows you what is in a prikk repository). Neither needs a compiler — both ship as
ready-to-run files. A *terminal* is the window where you type commands; everything below is typed
there.

## 1. Install prikk

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

## 2. Install stikk

Pick the file for your computer from the [latest release](https://github.com/prikk-vcs/stikk/releases/latest):

| Your computer | File to download |
|---|---|
| Linux, Intel/AMD | `stikk-0.10.0-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, Intel/AMD (older systems) | `stikk-0.10.0-x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM | `stikk-0.10.0-aarch64-unknown-linux-gnu.tar.gz` |
| Mac, Apple Silicon (M1 and later) | `stikk-0.10.0-aarch64-apple-darwin.tar.gz` |
| Mac, Intel | `stikk-0.10.0-x86_64-apple-darwin.tar.gz` |
| Windows | `stikk-0.10.0-x86_64-pc-windows-msvc.zip` |

Each file has a second file beside it ending in `.sha256`. That is a *checksum*: a short fingerprint
of the download, so you can tell the file arrived exactly as published. Download both.

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

```sh
install -m755 stikk-0.10.0-x86_64-unknown-linux-gnu/stikk ~/.local/bin/stikk
```

**macOS** is the same, with the Mac file name and one different check command (replace
`aarch64-apple-darwin` with `x86_64-apple-darwin` on an Intel Mac):

```sh
shasum -a 256 -c stikk-0.10.0-aarch64-apple-darwin.tar.gz.sha256
```

**Windows**, in PowerShell. Windows publishes its checksum as the fingerprint alone, so you compare
it yourself rather than running a check command:

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-pc-windows-msvc.zip -OutFile stikk.zip
```

```powershell
Invoke-WebRequest -Uri https://github.com/prikk-vcs/stikk/releases/download/0.10.0/stikk-0.10.0-x86_64-pc-windows-msvc.zip.sha256 -OutFile stikk.zip.sha256
```

```powershell
if ((Get-FileHash stikk.zip -Algorithm SHA256).Hash -eq (Get-Content stikk.zip.sha256).Trim()) { "OK" } else { "FAILED - do not use this file" }
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

## Optional: check who built it

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
(2024 edition, MSRV 1.88). The downloads above need neither.

## Next

[Getting started](./getting-started.md) opens a repository and walks through what stikk shows you.

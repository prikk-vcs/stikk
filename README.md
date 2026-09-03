# stikk

[![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/stikk/)
[![license](https://img.shields.io/crates/l/stikk.svg)](LICENSE)
[![CI](https://github.com/prikk-vcs/stikk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/prikk-vcs/stikk/actions/workflows/ci.yml)    
[Report a vulnerability](SECURITY.md)

**stikk is a history browser and workbench for the [prikk](https://github.com/nabbisen/prikk)
version control system** — a terminal (TUI) and graphical (GUI) front-end over one shared operation
layer.

## Overview

prikk is a distributed VCS built around block-oriented patch theory. stikk sits on top of it to
navigate, inspect, and operate on prikk history. Its founding property — from which the whole design
follows — is that **stikk owns no repository authority and no secrets**: every repository fact is
re-derived from prikk, and prikk (never stikk) reads signing key material. Its stance mirrors
prikk's own: *where prikk refuses, stikk explains.*

The name is Norwegian for *to set a course, to take a bearing.*

## Why / when

Reach for stikk when you want to read and work with prikk history without memorizing the CLI, and
when you want prikk's refusals — non-confluent merges, trust conflicts, integrity findings —
explained rather than dumped. stikk never guesses about history and never presents imported or
unverifiable authorship as sound: it makes prikk's own truth legible.

It is **not** a second prikk CLI (VCS verbs stay prikk's), not a network tool (prikk moves no bytes),
and not a place your signing keys live.

## Quick start

```sh
# Build (Rust 2024 edition, MSRV 1.85).
cargo build --release

# Point at a prikk build if it is not on PATH.
export STIKK_PRIKK_BIN=/path/to/prikk

# Open a repository and see where you stand.
./target/release/stikk /path/to/repo

# Launcher utilities.
stikk --version
stikk config path          # where stikk's config and state live (never inside a repository)
stikk config check         # validate the config file
```

Opening a repository on a terminal launches the interactive **TUI** — an Orientation view showing
prikk version and support, queue depth, signing readiness, and your derived capability, inside a shell
with a status bar and a Help overlay (`?`). Run it piped or in CI and you get the same orientation as a
one-shot print instead. The TUI is built on `ratatui` (RFC 001); more views (History, Patch detail)
follow. Try it with no repository: `cargo run -p stikk-tui --example orientation_demo`.

stikk reads `PRIKK_*_SEED` **presence only, never their values**; set `STIKK_READ_ONLY=1` to force a
read-only session.

## Design notes

- **The prikk seam is the only door to prikk.** One crate (`stikk-prikk`) talks to prikk; a CLI
  backend ships first (output parsing confined and version-gated, with an EPIPE guard), a linked
  library backend is deferred behind the same trait. Nothing above the seam knows *how* prikk is
  reached.
- **stikk owns no repository truth.** Every stikk-owned datum is a deletable convenience re-resolved
  against prikk on use; cutting all of stikk's state leaves every repository byte-identical. stikk's
  files live in user scope and a path resolver refuses any repository-internal target before every
  write — the *primary* control, since prikk has no foreign-file backstop.
- **No key material ever enters stikk.** It reads signing-key *presence* only; prikk reads seeds
  itself. This is enforced by test, not just convention.
- **One operation layer, two frontends.** The TUI and GUI drive the same operations, so parity is
  mechanical, not maintained by hand.

## Project Status

### Crates

| Crate | Purpose | Version | Docs | Dependencies |
|---|---|---|---|---|
| [`stikk`](https://crates.io/crates/stikk) | (todo) | [![crates.io](https://img.shields.io/crates/v/stikk.svg?label=%20)](https://crates.io/crates/stikk) | [![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/stikk/) | [![Dependency Status](https://deps.rs/crate/stikk/latest/status.svg)](https://deps.rs/crate/stikk) |
| [`stikk-core`](https://crates.io/crates/stikk-core) | (todo) | [![crates.io](https://img.shields.io/crates/v/stikk-core.svg?label=%20)](https://crates.io/crates/stikk-core) | [![docs.rs](https://img.shields.io/docsrs/stikk-core?version=latest&label=%20)](https://docs.rs/stikk-core) | [![Dependency Status](https://deps.rs/crate/stikk-core/latest/status.svg)](https://deps.rs/crate/stikk-core) |

(todo)

### Project Structure

(todo)

## More detail

The full design set lives in [`docs/src`](docs/src) (mdBook), organized by reader:

- **New users** — the guide (getting started, orientation).
- **Reference** — the design documents: requirements, external design, internal design, data model,
  and the security threat model.
- **Contributors** — [CONTRIBUTING.md](CONTRIBUTING.md), the workflow, and the testing discipline.

Where stikk is headed: see [ROADMAP.md](ROADMAP.md). Security: see [SECURITY.md](SECURITY.md).
Changes: see [CHANGELOG.md](CHANGELOG.md).

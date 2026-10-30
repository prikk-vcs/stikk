# stikk

[![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/stikk/)
[![license](https://img.shields.io/crates/l/stikk.svg)](LICENSE)
[![CI](https://github.com/prikk-vcs/stikk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/prikk-vcs/stikk/actions/workflows/ci.yml)    
[Report a vulnerability](SECURITY.md)

**stikk is a history browser and workbench for the [prikk](https://github.com/nabbisen/prikk)
version control system** — a terminal (TUI) front-end, with a graphical (GUI) one planned, over one shared
operation layer.

![stikk's History view: a ref's block lineage, newest first, with the queued tier above it, the tip marked, and a rollback block and one needing attestations called out](docs/assets/readme/history.svg)

![stikk's Compare view: what changed between two blocks, in prikk's own diff hunks, every line quoted byte for byte so a trailing space is visible](docs/assets/readme/compare.svg)

![stikk's Verify view: prikk's verdict that the repository is sound, every stage prikk ran, and a sentence naming what prikk's JSON report leaves out](docs/assets/readme/verify.svg)

*Drawn by stikk's own views from invented data, not photographed — see
[RFC 042](rfcs/accepted/042-readme-key-visuals.md).*

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

## Install

stikk needs two programs: **prikk** (the version control system itself) and **stikk** (this
program, which shows you what is in a prikk repository). Neither needs a compiler, and **each one
installs itself**. Two commands, in order.

**1. prikk** — Linux and macOS:

```sh
curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh
```

Windows, in PowerShell:

```powershell
irm https://github.com/prikk-vcs/prikk/releases/latest/download/install.ps1 | iex
```

[prikk's install guide](https://prikk.org/docs/guide/install.html) is the authority on that line.

**2. stikk** — Linux and macOS:

```sh
curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh
```

Windows, in PowerShell:

```powershell
irm https://github.com/prikk-vcs/stikk/releases/latest/download/install.ps1 | iex
```

Each installer checks its download against the checksum published beside it and installs nothing
if that check fails. stikk's installs stikk only: it checks whether prikk is there and new enough,
and prints prikk's own command if it is not. Running a line again upgrades that program.

stikk works with **prikk 0.28 and above**; a newer prikk runs, and stikk says its output has not
been checked rather than assuming it has.

**[The install guide](https://prikk-vcs.github.io/stikk/guide/install.html)** has the rest: how to
open a terminal if you have not before, what a successful run looks like, updating, removing, the
archives by hand, and what to do if Windows blocks an unsigned program.

## Quick start

Make a repository, or go into one you already have:

```sh
prikk init
```

Open it:

```sh
stikk
```

Press `?` for help and the full key reference. `q` or `Esc` leaves.

Launcher utilities:

```sh
stikk --version
stikk config path          # where stikk's config and state live (never inside a repository)
stikk config check         # validate the config file
```

If prikk is installed somewhere your terminal does not look, point stikk at it:

```sh
export STIKK_PRIKK_BIN=/path/to/prikk
```

stikk requires prikk **≥ 0.28** and is validated through **0.55.0**; a newer prikk still runs, but
stikk says its output shapes have not been checked against it, rather than assuming they have.

Opening a repository on a terminal launches the interactive **TUI** — an Orientation view showing
prikk version and support, queue depth, signing readiness, and your derived capability, inside a shell
with a status bar and a Help overlay (`?`). Run it piped or in CI and you get the same orientation as a
one-shot print instead. The TUI is built on `ratatui` (RFC 001), and from there you can open History and
Block detail, worktree Changes and the Queue view, **Patch detail** — what one patch changed, from
prikk's own report, with content shown byte for byte — **Compare**, what changed between two points
in prikk's own `diff` hunks (prikk ≥ 0.46), including **any two refs** marked and selected in the
ref picker, **Verify** (`V`) — prikk's verdict on whether the repository is sound, with its failing
conditions and every stage it ran (prikk ≥ 0.39) — and commit or seal behind preview-first
confirmation, and from the ref picker **create a branch** (`n`), **create a tag** (`t`, or at any
block from History), **close a branch** (`x`) and **switch to a branch** (`s`, previewed as the
exact files it will write and delete) — prikk ≥ 0.42, and ≥ 0.46 for a switch. stikk also
**merges one branch into another** — mark the branch to merge from (`m`), select the one it goes
into, `M` for prikk's own plan, `Enter` to run it — only where prikk's plan proves the two sides
confluent, previewed like every other write and never into the branch you are on (prikk ≥ 0.42).
stikk **shares work as a bundle file** — `i` opens one and shows prikk's own verification of it
and what prikk says importing it would change, `I` imports it as a received branch beside your
own, and `E` writes a branch out to a file outside the repository — each previewed, and nothing
written until you confirm (prikk ≥ 0.53).
stikk **gets an earlier version of your files back** — in Block detail, `F` saves one file as it
was at that block and `A` saves every file at it into a new folder, always outside the repository
and never over anything already there, with nothing in the repository or your worktree changed
(prikk ≥ 0.54).
stikk **chooses which keys this repository trusts** — `T` lists the adopted maintainer keys with
your own marked, and a bundle card says which of them, if any, each recorded sealer's id names
(Verify after an import is what checks the signature). **`a` adopts a key and `x` removes one**,
each previewed and confirmed by typing the key id; a removal warns first that it can make blocks
that key sealed fail Verify until the key is adopted again, and an id already trusted under a
different key is shown as the security event it is (prikk ≥ 0.54).
stikk **reopens each repository where you left it** — the focused ref, the view and the untracked filter,
re-checked against prikk first and stored outside every repository; `STIKK_PRIVATE=1` remembers
nothing.
To see it with no repository at all, from a clone of
this repository: `cargo run -p stikk-tui --example orientation_demo`.

stikk never reads a seed's value. On prikk ≥ 0.41 it asks prikk for signing readiness (`prikk key
status`); below that it checks only whether `PRIKK_*_KEY_ID` and `PRIKK_*_SEED` are set. Set
`STIKK_READ_ONLY=1` to force a read-only session.

## Design notes

- **The prikk seam is the only door to prikk.** One crate (`stikk-prikk`) talks to prikk; a CLI
  backend ships first (output parsing confined and version-gated, with an EPIPE guard), a linked
  library backend is deferred behind the same trait. Nothing above the seam knows *how* prikk is
  reached.
- **stikk owns no repository truth.** Every stikk-owned datum is a deletable convenience re-resolved
  against prikk on use; cutting all of stikk's state leaves every repository byte-identical. stikk's
  files live in user scope and a path resolver refuses any repository-internal target before every
  write — the *primary* control, since prikk has no foreign-file backstop.
- **No key material ever enters stikk.** It asks prikk for signing readiness on prikk ≥ 0.41 and reads
  signing-key *presence* only below that; prikk reads seeds itself. This is enforced by test, not just
  convention.
- **One operation layer, two frontends.** The TUI and GUI drive the same operations, so parity is
  mechanical, not maintained by hand.

## Project Status

### Crates

| Crate | Purpose | Version | Docs | Dependencies |
|---|---|---|---|---|
| [`stikk`](https://crates.io/crates/stikk) | A user-facing history browser and workbench for the prikk version control system (the launcher binary). | [![crates.io](https://img.shields.io/crates/v/stikk.svg?label=%20)](https://crates.io/crates/stikk) | [![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/stikk/) | [![Dependency Status](https://deps.rs/crate/stikk/latest/status.svg)](https://deps.rs/crate/stikk) |
| [`stikk-core`](https://crates.io/crates/stikk-core) | The operation layer: one shared operation set both frontends drive. Owns no I/O and no widgets; orchestrates the seam, state, and view-models. | [![crates.io](https://img.shields.io/crates/v/stikk-core.svg?label=%20)](https://crates.io/crates/stikk-core) | [![docs.rs](https://img.shields.io/docsrs/stikk-core?version=latest&label=%20)](https://docs.rs/stikk-core) | [![Dependency Status](https://deps.rs/crate/stikk-core/latest/status.svg)](https://deps.rs/crate/stikk-core) |
| [`stikk-model`](https://crates.io/crates/stikk-model) | Shared kernel for stikk: error taxonomy, object/ref identity, request categories, and capabilities. No I/O. | [![crates.io](https://img.shields.io/crates/v/stikk-model.svg?label=%20)](https://crates.io/crates/stikk-model) | [![docs.rs](https://img.shields.io/docsrs/stikk-model?version=latest&label=%20)](https://docs.rs/stikk-model) | [![Dependency Status](https://deps.rs/crate/stikk-model/latest/status.svg)](https://deps.rs/crate/stikk-model) |
| [`stikk-prikk`](https://crates.io/crates/stikk-prikk) | The prikk seam: the only code in stikk that talks to prikk. CLI backend, version handshake, and signing readiness — from prikk itself where it reports it, and presence only otherwise. | [![crates.io](https://img.shields.io/crates/v/stikk-prikk.svg?label=%20)](https://crates.io/crates/stikk-prikk) | [![docs.rs](https://img.shields.io/docsrs/stikk-prikk?version=latest&label=%20)](https://docs.rs/stikk-prikk) | [![Dependency Status](https://deps.rs/crate/stikk-prikk/latest/status.svg)](https://deps.rs/crate/stikk-prikk) |
| [`stikk-state`](https://crates.io/crates/stikk-state) | stikk's own durable data: config, sessions, recents. Lives in user scope, never inside a repository, and is never authority. | [![crates.io](https://img.shields.io/crates/v/stikk-state.svg?label=%20)](https://crates.io/crates/stikk-state) | [![docs.rs](https://img.shields.io/docsrs/stikk-state?version=latest&label=%20)](https://docs.rs/stikk-state) | [![Dependency Status](https://deps.rs/crate/stikk-state/latest/status.svg)](https://deps.rs/crate/stikk-state) |
| [`stikk-tui`](https://crates.io/crates/stikk-tui) | The terminal (TUI) frontend for stikk. Renders the operation layer's view-models; computes nothing about the repository. | [![crates.io](https://img.shields.io/crates/v/stikk-tui.svg?label=%20)](https://crates.io/crates/stikk-tui) | [![docs.rs](https://img.shields.io/docsrs/stikk-tui?version=latest&label=%20)](https://docs.rs/stikk-tui) | [![Dependency Status](https://deps.rs/crate/stikk-tui/latest/status.svg)](https://deps.rs/crate/stikk-tui) |

### Project Structure

stikk is a five-layer workspace, dependencies pointing strictly downward: `stikk-model` (shared kernel,
no I/O) ← `stikk-prikk` (the seam — the only code that talks to prikk) and `stikk-state` (user-scope
config/session) ← `stikk-core` (the one operation layer both frontends drive) ← the frontends
(`stikk-tui` today) and the `stikk` launcher. See
[`docs/src/contributing/development.md`](docs/src/contributing/development.md) for the full picture.

## More detail

The full design set lives in [`docs/src`](docs/src) (mdBook), organized by reader:

- **New users** — the guide (getting started, orientation).
- **Reference** — the design documents: requirements, external design, internal design, data model,
  and the security threat model.
- **Contributors** — [CONTRIBUTING.md](CONTRIBUTING.md), the workflow, and the testing discipline.

Where stikk is headed: see [ROADMAP.md](ROADMAP.md). Security: see [SECURITY.md](SECURITY.md).
Changes: see [CHANGELOG.md](CHANGELOG.md).

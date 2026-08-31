# Getting started

stikk is a front-end for the prikk version control system. It does not replace prikk's own command
line — VCS verbs stay prikk's — it helps you *read and work with* prikk history.

## Build

stikk builds with a standard Rust toolchain (2024 edition, MSRV 1.85):

```sh
cargo build --release
```

## Point stikk at prikk

stikk drives the `prikk` binary. If prikk is on your `PATH`, nothing is needed. Otherwise, point at a
specific build:

```sh
export STIKK_PRIKK_BIN=/path/to/prikk
```

## Open a repository

```sh
stikk /path/to/repo
```

This prints a one-shot **orientation**: the prikk version and whether stikk supports it, the queue
depth, your signing readiness, and the capability that readiness gives you. Run it inside a repository
with no argument and stikk discovers the repository root by walking upward for a `.prikk` directory,
the same way prikk does.

The interactive TUI — which turns this orientation into a live, navigable interface — is the next
increment.

## Capability and signing

What you can do is *derived* from which signing keys are ready in your environment, not from any
account stikk keeps:

- No keys → **Viewer** (every read surface).
- `PRIKK_AUTHOR_KEY_ID` + `PRIKK_AUTHOR_SEED` present → **Author** (can queue commits and rollback
  drafts).
- `PRIKK_MAINTAINER_KEY_ID` + `PRIKK_MAINTAINER_SEED` present → **Maintainer** (can seal, merge,
  publish refs and tags).
- `STIKK_READ_ONLY=1` forces Viewer regardless of keys.

stikk reads only the **presence** of these variables. It never reads a seed's value — prikk reads
seeds itself when it signs. Your keys never enter stikk.

## Launcher utilities

```sh
stikk --version           # print the stikk version
stikk config path         # where stikk's config and state live (never inside a repository)
stikk config check [file] # validate the config file; exits non-zero on a notice, for CI
```

stikk's own files live in user scope (following the XDG convention: `~/.config/stikk`,
`~/.local/state/stikk`), never inside a repository — so a repository stays byte-identical whether or
not stikk ever opened it.

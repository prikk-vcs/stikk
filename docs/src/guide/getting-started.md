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

stikk requires prikk **≥ 0.28** and is validated through **0.31.0**. A prikk below the floor degrades
to read-only where it can; a prikk above the validated ceiling still runs, but Orientation says its
output shapes have not actually been checked against it, rather than silently assuming they have.

## Open a repository

```sh
stikk /path/to/repo
```

On a terminal this launches the interactive **TUI**: an Orientation view — the prikk version and
whether stikk supports it, the queue depth, your signing readiness, and the capability that readiness
gives you — inside a shell with a status bar and a Help overlay. Run it inside a repository with no
argument and stikk discovers the repository root by walking upward for a `.prikk` directory, the same
way prikk does.

Key reference: `Enter` opens History and drills into a block; `b` chooses which ref to view; `w` opens
Changes (worktree vs. baseline), `u` toggles its display-only untracked filter; `:` opens the command
palette; `R` shows the session's recent refusals; `o` lists background operations still in flight or
recently finished (a listing only — there is no cancel action); `?` opens the glossary and full key
reference; `r` refreshes the current view from prikk; `Esc`/`q` steps back, and quits at the root.

Run `stikk` piped or in CI (no terminal) and you get the same orientation as a one-shot print instead.
To see the TUI with no repository at all: `cargo run -p stikk-tui --example orientation_demo` (also see
`history_demo`, `explanation_demo`, and `changes_demo` for the other views, all scripted — no prikk
binary or repository needed).

**Patch detail** is deferred behind `UD-09` (prikk exposes no per-patch content yet); **Compare** is
deferred behind the same ceiling, with a recorded future route (RFC 008). Both are named gaps, not
upcoming work.

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

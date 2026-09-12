# Getting started

stikk is a front-end for the prikk version control system. It does not replace prikk's own command
line — VCS verbs stay prikk's — it helps you *read and work with* prikk history.

## Build

stikk builds with a standard Rust toolchain (2024 edition, MSRV 1.88):

```sh
cargo build --release
```

## Point stikk at prikk

stikk drives the `prikk` binary. If prikk is on your `PATH`, nothing is needed. Otherwise, point at a
specific build:

```sh
export STIKK_PRIKK_BIN=/path/to/prikk
```

stikk requires prikk **≥ 0.28** and is validated through **0.41.0**. A prikk below the floor degrades
to read-only where it can; a prikk above the validated ceiling still runs, but Orientation says its
output shapes have not actually been checked against it, rather than silently assuming they have.

**One known upstream limitation at the floor, on Windows only:** prikk **0.28** cannot commit a file
that lives in a subdirectory when running on Windows — its worktree scan builds the repository path
with the platform separator and its own path validator then refuses the backslash
(`invalid name: backslashes are not allowed in repository paths`). This is prikk's, not stikk's, and
prikk fixed it in **0.29.0**; on Windows, use prikk ≥ 0.29. Every other supported version is
unaffected, and so is 0.28 on Linux and macOS. stikk's real-binary suite covers this combination
explicitly rather than skipping it quietly.

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

**Where stikk gets the answer depends on your prikk**, because prikk changed how signing keys are
configured at 0.40:

| your prikk | stikk asks |
|---|---|
| **≥ 0.41** | prikk itself, with `prikk key status` — including whether the key matches what this repository already records |
| **0.40 exactly** | nothing it can trust. 0.40 moved keys to a key directory and shipped no way to ask about them, so stikk reports signing readiness as **unknown**, says why, and points at 0.41 |
| **≤ 0.39** | the environment, as presence only: `PRIKK_<ROLE>_KEY_ID` **and** `PRIKK_<ROLE>_SEED` both set |

What that readiness grants is the same on every version:

- No keys → **Viewer** (every read surface).
- AUTHOR ready → **Author** (can queue commits and rollback drafts).
- MAINTAINER ready → **Maintainer** (can seal, merge, publish refs and tags).
- `STIKK_READ_ONLY=1` forces Viewer regardless of keys.

**On prikk ≥ 0.40, a `PRIKK_*_SEED` you exported previously is no longer what signs** — 0.40 refuses
it and 0.41 ignores it in favour of the key directory. stikk says so when it sees one still set,
because it is easy to believe otherwise.

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

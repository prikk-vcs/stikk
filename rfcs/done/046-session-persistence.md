# RFC 046 — Session persistence

**Status.** **Done 2026-10-18**: Handoff A delivered on `main` (`c27068a`, `dfec75a`, `0a350c4`); **released in 0.14.0**. **Accepted by the project owner 2026-10-18**; [Handoff A](../handoffs/046-session-persistence/a-session-persistence-handoff-v1.md) issued. **Proposed 2026-10-18** by the architect, as 0.14.0's increment 3 (`.git-exclude/release/0-14-0-scope.md`,
authorized). **No open question.**
**Measures nothing in prikk**: every prikk read it uses already exists. It is design against the data model's own
entities (`DM-02`, `DM-04`) and lifecycle (`LC-3`, `LC-5`, `LC-6`, `LC-8`), which were written before the code and are
followed here rather than re-derived.
**Tracks.** `FR-122`, `UC-19`, `FR-055`, `CON-4`, `NFR-R01`, `INV-2`, `INV-5`, `INV-9`, `C-E2`, `TS-04`, `T-T4`.
**Touches.** `stikk-state` (a session store), `stikk` (the launcher), `stikk-tui` (restore and flush), the guide, the
changelog.

## Summary

Every launch starts from scratch: Orientation, focus on prikk's current branch, untracked files shown. **This RFC makes
stikk reopen each repository where the user left it** — the focused ref, the last view, and the untracked filter —
stored **outside the repository**, and **re-checked against prikk before any of it is shown**. It is also **the first
production caller of `C-E2`**, the refusal that keeps stikk's files out of every repository, which today guards no real
write.

## Findings

### F1 — what exists

- `stikk-state::paths::state_dir()` resolves the per-platform state directory (`STIKK_STATE_DIR` overrides), and
  `ensure_outside_repository(target, repo_root)` refuses a `.prikk` component or a target inside the open worktree.
  **Nothing calls it for a real write today.**
- `RepositoryHandle` names a repository by its root. The data model keys session state by **canonical path** (`LC-9`:
  prikk has no repository identity, by design, and a fingerprint was deferred because `INV-5` already covers a reused
  path).
- `DM-04` already defines the record (focused ref by name, last view, filters, id-only anchors) and `LC-5` its flush
  rule (atomic, on every meaningful transition, so `kill -9` loses at most the last one).

### F2 — what can be restored safely today

| State | Restorable | Why |
|---|---|---|
| **Focused ref** | yes, by name | re-validated against `branch list`/`tag list` (`LC-6`) |
| **Top-level view**: Orientation, History, Changes, Queue | yes | each is re-read from prikk on open |
| **Untracked filter** (`u`) | yes | a display preference |
| Block detail, Patch detail, Compare | **no** (reopens their parent, History) | they need object ids and a second point; `DM-04`'s id anchors are a later increment |
| **Verify** | **no** | reopening it would run a full verification at launch, which `NFR-P05` forbids |

## Decisions

1. **One small file per repository** under `<state_dir>/sessions/`, named by a **stable, dependency-free 64-bit hash**
   of the canonical root path (FNV-1a over the path's bytes; `stikk-state` keeps its one edge, `stikk-model`). **The
   file also holds the full canonical path**, and a file whose path does not match is ignored, so a collision is a
   miss, never another repository's state. Per-repository files mean two stikk instances on two repositories never
   race on one file.
2. **What a record holds** (`DM-04`, narrowed to F2): a format version, the canonical path, the focused ref **name**,
   the last top-level view, and the untracked filter. **Names and flags only — no repository content, no ids, no
   prose** (`INV-5`, `INV-9`). The encoding is the handoff's to choose, with two requirements: it **round-trips every
   ref name prikk accepts** (spaces and non-ASCII included), and **any record that fails to decode is ignored whole**.
3. **Every write is atomic and goes through `ensure_outside_repository(target, Some(root))`** first: a temporary file
   in the same directory, then a rename. **A refused or failed write never stops the session**: stikk says once, in a
   banner, that the session will not be remembered, and carries on.
4. **Flush on each meaningful transition** (`LC-5`): a focus change, a top-level view change, the untracked toggle.
   Not on a timer, not only at exit.
5. **Restore is re-validation first, display second** (`LC-3`, `LC-6`). At launch, after the first successful reads:
   - the stored focus is used **only if prikk still lists it and it is not closed**; otherwise today's rule applies
     (prikk's current branch at ≥ 0.42, `FR-055` as amended), and **a one-line note says the remembered ref is gone**;
   - the stored view opens only if it is one of F2's four; otherwise Orientation;
   - the stored focus **wins over prikk's current branch** when both are valid — that is what *resume* means — and
     the status bar's existing *"prikk's default: <branch>"* already shows the difference (RFC 029 B).
   **Nothing stored is ever shown as a fact before prikk has confirmed it** (`T-T4`).
6. **A private session** (`LC-8`): `STIKK_PRIVATE=1` reads no record and writes none, for that launch. It sits beside
   `STIKK_READ_ONLY=1`; `--help`'s notes (which name only `STIKK_PRIKK_BIN` today) gain both, and the guide lists
   them. **Read-only mode still remembers**: it forbids
   repository mutations, and the session record is stikk's own file outside the repository.
7. **Bounded**: at most **200** records; when a write would exceed it, the least recently written are removed.
   **Deleting `<state_dir>/sessions/` is always safe**, and the guide says so.
8. **Only the TUI reads or writes a record.** The one-shot orientation print (piped, CI) touches no session state.

## Changelog

`### Added`: stikk reopens each repository on the ref, view and untracked filter you left it on, re-checked against
prikk first; `STIKK_PRIVATE=1` for a session that remembers nothing.

## Delivery

**One handoff, issued on acceptance, in two commits**: the store in `stikk-state` with its tests (hash, round-trip of
hostile ref names, corrupt and foreign records ignored, the `C-E2` refusal on a state directory inside the repository,
atomic replace, the bound), then restore and flush in the TUI and launcher. **Suite legs**, at 0.52 with
`STIKK_STATE_DIR` in a scratch directory: focus a branch and open Changes, relaunch, and both are restored; close that
branch with prikk, relaunch, and focus falls back with the note; `STIKK_PRIVATE=1` leaves the sessions directory
untouched; **a state directory inside the repository is refused and the repository is byte-identical afterwards**.

## What this RFC does not do

- **No deep restore** (Block detail, Patch detail, Compare) and **no scroll or selection anchors**: `DM-04`'s id anchors
  are a later increment.
- **No progressive disclosure** (`TU-12`), **no recents list** (`DM-03`), no settings UI.
- **No repository fingerprint** (`LC-9`, unchanged).

## Delivered

**One handoff, three commits, green on `main` at `0a350c4`**: CI, Docs, the real-binary suite (58 cases at 0.28.0 and
0.52.0) and supply chain.

- **The store** (`stikk-state::session`): one record per repository, named by FNV-1a 64 over the canonical path with
  the constants pinned; a byte-level `key=value` encoding, so a non-UTF-8 path is exact without `unsafe`; any decode
  failure, unknown version or foreign path is a miss; atomic replace; at most 200 records.
- **`C-E2` was lexical, and is not any more** (review v1 C1). The first real write through it showed that a relative
  state directory, a `..` leading back in, and a symlink into the repository all got past it
  (`.git-exclude/reports/046-session-persistence/c2e-probe.txt`). `ensure_outside_repository` now resolves both sides
  before comparing, refuses an unresolvable `..`, and checks `.prikk` on the resolved path. No released stikk had ever
  written a file through it, so no release was exposed. The suite proves six inside-the-repository state directories
  refused against a real repository, byte-identical afterwards.
- **Restore** is a pure `stikk_core::restore`, driven in the suite by prikk's real lists; nothing stored is drawn before
  both reads land, and no extra read happens without a record. The view is recorded by walking the screen stack, so
  Verify never reopens.
- **Beyond the RFC, accepted in review**: the untracked filter also persists across closing and reopening Changes
  within one session.

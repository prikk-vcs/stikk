# RFC 053 — Getting files back: an earlier version, saved outside the repository

**Status.** **Done 2026-10-29**: Handoff A delivered on `main` (`e304e9a`..`6b14260`, and `b280057` for Windows); 0.17.0 candidate. **Accepted by the project owner 2026-10-29**; [Handoff A](../handoffs/053-get-files-back/a-get-files-back-handoff-v1.md). **Proposed 2026-10-29** by the architect, as 0.17.0's increment 1 (`.git-exclude/release/0-17-0-scope.md`,
authorized 2026-10-29). **No open question.**
Measured against a real prikk **0.54.1**: `.git-exclude/reports/053-get-files-back/` (`measure053.sh` →
`measure053-0.54.txt`, one fresh repository and a receiver). prikk's source read in the clone at the `0.54.1` tag
(`crates/prikk-cli/src/extract.rs`, `crates/prikk-cli/src/cat.rs`, `crates/prikk-store/src/extract.rs`,
`crates/prikk-store/src/anchor_trust.rs`).
**Tracks.** `FR-053` (its non-destructive half), `FR-031/032` (Block detail), `TU-06`, `C-E2`, `C-T2a`, `UD-02`;
RFC 013 (confirmation), RFC 037 (the files at a block), RFC 050 decision 5 (a destination outside the repository).
**Touches.** `stikk-prikk` (four seam methods, one band constant), `stikk-core` (two previews and two writes),
`stikk-tui` (a cursor over Block detail's files, `F` and `A`, the path prompt), `stikk-real-binary`, `requirements.md`,
the guide, the changelog.

## Summary

A newcomer who broke a file needs **the version from before**. stikk shows every block's files (RFC 037) and every
patch's content (RFC 035), but cannot hand any of it back: the user leaves for the CLI. prikk 0.54 writes **one file
as it was at any block** (`cat --output`) and **every file at a block into a new directory** (`extract`), touching
neither the repository nor the worktree. stikk offers both from Block detail, previewed, **outside the repository and
never over an existing path**. This is also the honest answer while stikk cannot take a change back (rollback,
withdrawn in 0.16.0): get the old version, look at it, and copy what you want into your worktree yourself.

## Findings (measured at 0.54.1)

### F1 — two commands, one with JSON

- **`cat --path P --ref <block> --output FILE`** writes that file's bytes exactly (text and binary measured) and
  **prints nothing** on success. **Refuses an existing file** unless `--force`: *"refusing to overwrite existing file
  …; pass --force to replace it (nothing was written)"*, exit 1. A path absent at that block is refused by name. **`cat
  --path P --ref <block> --format json`** (`path-content-v1`) reports `kind`, `encoding`, `mode` and `size`, writing
  nothing.
- **`extract --ref <block> <dir> [--plan-only] --format json`** (`extract-report-v1`): `point`, `target_block_id`,
  `provisional`, `written`, `file_count`, `total_bytes`, `largest_file_bytes`. `--plan-only` writes nothing.
  Subdirectories and the execute bit are kept. **Refuses an existing destination** (*"refusing to extract into an
  existing path: … (nothing was written)"*). It writes into a temporary sibling (`<dir>.prikk-extract-tmp`) and renames
  it into place once, with no replace.
- **Refusals under `--format json` are a document of their own**: `extract` prints **`refusal-v1`** on stdout —
  `{"schema_version": "refusal-v1", "command": "extract", "refusal": <id>, "reason": <id|null>, "message": …}` — with
  the same message on stderr, exit 1. Thirteen ids, among them `destination-exists`, `io-failure`, `over-max-files`,
  `no-checkpoint-in-reach` (with a `reason`), `case-collision`, `point-not-found`.
- **`extract` exists from 0.54** (absent at 0.53.0 and 0.52.0); `cat --output` is older.

### F2 — neither refuses a destination inside the repository

Measured, both write there; prikk's source says so (*"Nothing here refuses a destination that happens to sit inside
this repository's own tree"*). The copy then sits in the worktree as **untracked files** (`worktree-status`:
`untracked inside-out/a.txt …`), and the next commit would capture it. **The same defect class RFC 050 decision 5
closed for export.**

**At prikk 0.55.0** (re-measured 2026-10-29, `measure053-0.55.txt`): **`extract` now refuses it**, as `refusal-v1`
`destination-inside-repository` (*"refusing to extract into inside-out, inside this repository's own root … (nothing
was written)"*), and the worktree stays clean. **`cat --output` still writes there.** Every other row of the
measurement is unchanged. stikk's own check (decision 4) stays: it covers `cat`, and 0.54.

### F3 — `cat --output` drops the execute bit; `extract` keeps it

`run.sh` with `mode: 33261` (`0o100755`) at the block came back `-rw-r--r--` from `cat --output`, and
`-rwxr-xr-x` from `extract`. prikk's `cat.rs` sets no permissions. **Non-blocking**: the bytes are exact, and a
reader restoring a script gets a file that needs `chmod +x`.

### F4 — a received branch replays from the start, and says so

`extract` from a received branch whose signer this repository has not adopted reported **`provisional: true`**.
prikk's own wording (prose mode only): *"no trusted checkpoint was in reach: every file was replayed from the start of
history. The files match that history. `prikk verify` checks the history …"*, and its source: provisional means
**nothing has checked that history's own integrity or its publication trust**. Past **63 blocks** from an admitted
anchor (`MAX_ANCHOR_DISTANCE`, a checkpoint every 64) with none in reach, `extract` refuses as
`no-checkpoint-in-reach` unless `--allow-full-replay`.

### F5 — a missing parent directory is prikk's refusal

Both refuse: `cat` as *"failed to create a temporary file beside the destination at …/a.txt.tmp-<pid>-<n>: No such
file or directory"*, `extract` as `io-failure` naming its temporary sibling. Nothing is written.

### F6 — `--plan-only` does not check the destination

`extract --plan-only` into an **existing** directory answered `written: false` with the counts, exit 0; only the real
run refused. The plan says what the point is, not whether the destination is free.

## Decisions

1. **Four seam methods**, all at a **block id** (the point the user opened, never a ref name that can move):
   `file_meta(repo, block, path) -> FileMeta` (`cat --format json`, `path-content-v1`), `save_file(repo, block, path,
   output)` (`cat --output`), `extract_plan(repo, block, dir) -> ExtractReport` (`--plan-only --format json`), and
   `extract(repo, block, dir) -> ExtractReport`. **Never `--force`, `--allow-full-replay` or `--max-*`.** A
   **`refusal-v1`** document on stdout is recognized by its `schema_version` and becomes a refusal carrying prikk's
   `message` verbatim — never parsed as a report, never matched on its prose. A missing or unknown field is a shape
   error (`UD-02`).

2. **The Exchange category, as export**: Tier Two, no key. Saving a copy outside the repository is export's act —
   one confirmation that names the destination, nothing in the repository changes, nothing is signed — and a new
   category would add ceremony with nothing to decide. Read-only mode declines it, as it declines export.

3. **Where: Block detail.** It already lists the files at the block (RFC 037). **The cursor ranges over the block's
   patches, then its files**; `Enter` on a patch opens Patch detail as today, and on a file does nothing.
   - **`F` on a file — save this file** as it was at this block.
   - **`A` anywhere in Block detail — save all files** at this block into a new folder.
   Patch detail is not a starting point: it shows what a patch did, not the files at a point, and a "save" there
   would have to say which point. Where Block detail says *state files* are unreported (below RFC 037's band) or
   failed, `F` and `A` answer that, spawning nothing.

4. **The destination, by stikk's rules** (RFC 050 decision 5, applied again). The path prompt suggests, **outside the
   repository by construction**: for one file, `<the repository's parent>/<file stem>-<short block>.<ext>`; for all
   files, `<the repository's parent>/<repository name>-<short block>`. Resolved against the directory stikk was started
   in and **shown resolved**. Declined in stikk's words before anything is spawned:
   - **inside the repository** (`C-E2`'s resolved check, F2): *the copy would sit in your worktree as untracked files,
     and the next commit would capture them*;
   - **an existing path** (F1, F6): *something is already there; stikk does not overwrite it — choose another name*.
   **A missing parent directory is prikk's refusal**, shown verbatim (F5), as for export.

5. **The card** (Tier Two). For **one file**: the block (short id), the file's path through `identifier`, its size
   from `file_meta`, the resolved destination. For **all files**: the block, **prikk's plan** — *N files, B bytes* — and
   the resolved directory. The consequence, in stikk's words: *a copy is written there; nothing in the repository or
   your worktree changes*. Two notices, each only when it applies:
   - **the file is executable** (`mode & 0o111`, from `file_meta`; F3): *prikk saves the file's bytes, not its execute
     permission — the copy will not be executable. Saving all files keeps it.*
   - **the plan is provisional** (F4): *no trusted checkpoint is in reach, so prikk replays every file from the start
     of history. The copy will match that history, but nothing has checked the history itself, and it may be signed
     by a key this repository does not trust. Verify (`V`) checks it.*
   Both previews re-read what they need at confirmation (RFC 013): the destination rules, and for all files the plan.

6. **The result.** One file: *Saved `<path>` as it was at `<block>` to `<destination>`.* All files: *Wrote N files (B
   bytes) to `<directory>`.*, from the report, with the provisional notice repeated when the report says so. A refusal
   is prikk's `message`, verbatim. Nothing is re-read: nothing in the repository changed.

7. **Band: prikk ≥ 0.54** (`FILES_FROM_MINOR`), one band for both keys — `extract` starts there, and no user is on an
   older band. Below it, `F` and `A` answer *unavailable*, spawning nothing.

8. **For prikk, held for the next necessary letter** (non-blocking): F3 (should `cat --output` keep the mode, as
   `extract` does?), and a `refusal-v1` document for `cat` as `extract` has.

## Changelog

`### Added`: save a file as it was at any block, or all of a block's files into a new folder, from Block detail —
outside the repository, never over an existing path, at prikk ≥ 0.54. `### Breaking`: whatever `cargo public-api`
shows (the `Prikk` trait gains four required methods).

## Delivery

**One handoff.** It re-runs `measure053.sh` at 0.54.1 first. **Suite legs at 0.54**: one file saved and its bytes
compared with `cat`'s; all files extracted and the tree compared with `tree --format json`; **a destination inside the
repository declined with no prikk write spawned, and `worktree-status` unchanged**; an existing destination declined;
an executable file's card carries the notice; a received branch's plan carries the provisional notice. **At 0.28**,
unavailable.

## What this RFC does not do

- **Write into the worktree** (`FR-053`'s materialize half, `FR-054`): the riskiest write prikk offers; this RFC covers
  the need without it.
- **Overwrite** anything, **force** a full replay, or **bound** a point by size.
- **Save from Patch detail, History or Compare** (decision 3).

## Delivered

**One handoff, six commits, green on `main` at `18b4223`**: CI, Docs, supply chain, and the real-binary suite on the full
platform matrix (80 cases at 0.28.0 and 0.55.0 on Linux and macOS; 79 on Windows, where the execute-bit leg is
`#[cfg(unix)]`).

- **A, getting files back** (`e304e9a`..`6b14260`). Four seam methods (`file_meta`, `save_file`, `extract_plan`,
  `extract`), each refusing a relative path before anything is spawned, with `refusal-v1` recognised by
  `schema_version` and its `message` shown verbatim. **Both destination rules are stikk's, decided before prikk is
  asked**: never inside the repository, never over anything already there. `F` and `A` in Block detail, with the card
  naming the block, the size and the resolved destination; the execute-bit and full-replay notices sit above the
  consequence (`ConfirmationSummary::notices`, a Breaking row). Suite leg 3 proves the point of the rule: at 0.55
  `cat --output` still writes inside the repository.
- **Found after the push**: the first full-matrix run was red on Windows, in the test only: the test compared
  `Path::display()` with prikk's forward-slash paths. Fixed in `b280057`, reviewed with RFC 054, green on Windows at
  `18b4223`.
- **Accepted deviations**: a refusal in both profiles rather than a `debug_assert!`; one 12-character short id; four
  wrapper types.
- **Carried to the next time those files are touched**: "already there" as `symlink_metadata(..).is_ok()` (prikk's
  meaning, which a dangling symlink satisfies) in `files.rs` and `bundle.rs`; Block detail's key hint on its own row at
  80 columns; `version.rs:113` rewrapped.

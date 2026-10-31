# RFC 058 — Undo one block: putting back what a block changed, as a new change

**Status.** **Accepted by the project owner 2026-10-31**; [Handoff A](../handoffs/058-undo-one-block/a-undo-one-block-handoff-v1.md). **Clarified at acceptance:** the parent's mode comes from the diff entry's `from.mode`, so no `tree` read is needed (F2, decision 2); and guard 1 is defined exactly, so an untracked file elsewhere does not block an undo (decision 4). **Proposed 2026-10-31** by the architect, as 0.20.0's increment 1 (`.git-exclude/release/0-20-0-scope.md`,
authorized 2026-10-31 with its two recommendations: the word *"Undo this block"*, and *any single-parent block whose
paths no later block changed*). **No open question.**
**Measured** against a real prikk **0.55.0**: `.git-exclude/reports/058-scope-0-20/` (`measure-undo.sh` →
`measure-undo-0.55.txt`). It undoes the newest block, an older block, and an older block whose path changed since, and
it reads a merge's history. Seeds went to `--out` files.
**Tracks.** A new requirement, `FR-085`, and a new use case, `UC-21`, both added by this RFC. Also `FR-120` (preview
first), `FR-121`, `CON-3` (history is never rewritten), `C-T4a`..`C-T4e`, `ER-02`; RFC 013 (tiers), RFC 014 (commit),
RFC 045 (materialization), RFC 053 (`save_file`). prikk's letter 029 §4.6: *"not on our plans, so build it if you need
it"*.
**Touches.** `stikk-prikk` (one seam method), `stikk-core` (a new `undo` module: preview, guards, execute),
`stikk-tui` (one key in History, one card), `stikk-real-binary`, `requirements.md`, the guide, the changelog.

## Summary

A user who sealed a mistake needs the obvious next step: **undo it.** prikk's own rollback (`FR-083`) undoes a whole
branch back to empty, and prikk does not plan a single-block one. stikk builds it from what prikk already gives:
- **what the block changed**, from prikk's `diff` between the block and its parent;
- **the earlier files**, from prikk's `cat`;
- **a rename back**, from prikk's `mv`;
- **the record**, an ordinary `commit`.

**Nothing is erased.** The undo is a new change after the block, so history shows both. In History, `U` on a block
opens a card that lists what goes back, file by file, and asks once. stikk refuses, and writes nothing, wherever the
undo could overwrite work or would not come out exact.

## Findings (measured at 0.55.0)

### F1 — prikk's `diff` names every change a block made

`diff --from <parent> --to <block> --format json` (`diff-report-v1`) gives one entry per path, with a `status`:
`added`, `deleted`, `modified`, `binary`, `mode`, `renamed` (with `from_path`), or `symlink`. Each entry carries the
mode on each side. `unsupported_paths` lists what prikk could not compare. prikk's `diff.rs` says `symlink` is
*"unreachable today"*.

### F2 — the inverse of each entry, done in the worktree, gives back the parent exactly

| Entry | The inverse |
|---|---|
| `added` | remove the file |
| `deleted`, `modified`, `binary`, `mode` | write the parent's bytes with `cat --path P --ref <parent> --output P --force`, then apply the parent's mode |
| `renamed` | `prikk mv <path> <from_path>`; if the content changed too, then also write the parent's bytes |

A directory that the inverse empties is removed, since a commit cannot record an empty one.

**Measured.** One block changed a text file, deleted a file, replaced a binary, added a file, deleted a nested
folder, created one, changed a mode, renamed a file, and renamed and edited another. stikk-style inversion, then
`commit --from-worktree` and `seal`, gives a tip **identical to the parent: every path, every mode, every byte**.
`verify` passes. The undo block's operations are ordinary ones (`delete-node`, `create-file`, `change-perm`,
`replace-binary`, `edit-text`, `rename-path`), so a rename is undone as a rename.

`cat --output` does not keep a file's mode at 0.55 (letter 023 §3.5; prikk applies it from 0.57.0). So stikk applies
the mode itself. The value is the diff entry's own `from.mode`: the measurement read it from `tree`, which gives the
same value, and stikk needs no second read. Applying it is harmless to keep once prikk applies it too.

### F3 — an older block can be undone when no later block touched its paths, and stikk can tell when one did

- **Blocks `one`, `two`, `three`, where `three` changed only `x.txt`:** undoing `two` (`a.txt`, `n.txt`) gives
  `a.txt` = `one`'s, `n.txt` absent, and `x.txt` = `three`'s. `three` is kept.
- **Where `three` changed `a.txt` again:** `diff --from two --to three` names `a.txt`. A plain inverse would overwrite
  `three`'s change, so stikk refuses this case.

### F4 — a block's parent, today, is the next-older entry in `log`

`log --format json` gives each block `kind` (`Root`, `Normal`, `Merge`, …) and `parent_count`, but no parent id, and
`show` gives none either. A single-parent block's parent is the entry below it in its branch's own `log`.
- **A merge** has two parents, and the order of `log` names only one of them.
- **A fast-forward merge** still seals a new merge block (`reports/047-merge/measure047-0.53.txt`, scenario 6), so it
  moves no branch onto another branch's block.
- **The first block in a branch's `log`** has no entry below it. That is a root, or the block the branch was created
  from.

prikk 0.57.0 adds `parent_block_ids` to `log` (029 §4.3). A later re-baseline reads the parent from it.

### F5 — on Windows, prikk cannot record a mode from the worktree

prikk's `platform-support.md` says worktree authoring on Windows *"never derives a node's recorded mode"*. An
existing node keeps its recorded mode, and a new file is created non-executable. So on Windows two inverses would not
come out exact:
- a `mode` entry;
- putting back a deleted file whose recorded mode is executable.

## Decisions

1. **The requirement.** `FR-085` *[S]* **Undo one block** (`UC-21`): preview what goes back, confirm once, and record
   the undo as an ordinary commit. History is never rewritten (`CON-3`). It sits beside `FR-083`, which stays prikk's
   whole-branch rollback. **stikk never calls this a rollback.**

2. **One seam method**: `rename_path(repo, from, to) -> Renamed`, which runs `prikk mv`. Category **`QueueMutation`**:
   the declaration is authored into the next commit. `Renamed` carries prikk's stdout **verbatim**, and stikk parses
   none of it. Everything else is a method stikk already has:
   - `history`, for the block's place and its parent (F4);
   - `diff`, for both comparisons (F1, F3), and for the parent's modes (`from.mode`);
   - `save_file`, for the bytes;
   - `orientation` and `worktree_status`, for the guards;
   - `commit`.

   **Band:** `FILES_FROM_MINOR` (0.54), where `cat --output` begins. Below it, `U` is unavailable, with that reason.

3. **The undo is offered on a block of the current branch only.** That means History focused on the current branch
   (RFC 029's `.prikk/current-branch`). On any other ref, `U` is unavailable: *"Undo works on the branch you are on.
   Switch to it first."*

4. **The guards, inside `confirm::preview`, so they are taken again at confirmation.** Each one declines with
   **nothing written**, as disabled-with-reason (`C-T4d`), in stikk's words:
   1. **The worktree has changes**: any `missing` or `modified` file, any entry other than an untracked file, or any
      refused declaration. **An untracked file elsewhere does not count**, because an editor's temporary file must
      not block an undo. One that is in the way is guard 6's case. *"Undo needs a clean worktree, so it cannot
      overwrite your work. Commit or remove your changes first."*
   2. **Work is queued** (Orientation's queued patches > 0): *"Work is waiting to be sealed. Seal it first, so the
      undo is recorded after it."*
   3. **The block is a merge** (`kind: Merge`, or `parent_count` ≠ 1): *"stikk cannot undo a merge yet."*
   4. **The block is the first in this branch's history** (F4): *"This is the first block on this branch, so there is
      nothing before it to go back to."*
   5. **A later block changed one of its paths** (F3). The set compared is every entry's `path` plus every rename's
      `from_path`, against every path in `diff --from <block> --to <tip>`: *"A later block changed `a.txt` too.
      Undoing this one would overwrite that change."* It names every such path.
   6. **A file stikk would write is already in the worktree, untracked:** *"`b.txt` is in the way: it is not part of
      this branch, and the undo would replace it."*
   7. **An entry stikk cannot invert exactly:** a status it does not know (`symlink` included), any
      `unsupported_paths`, or on Windows a case from F5: *"stikk cannot undo this block exactly: <the reason, naming
      the path>."*

5. **The card: `U` in History, on the selected block.** Tier **Three**, like a switch: it writes the worktree from
   history, and every refusal writes nothing. The signing requirement is **AUTHOR**, because the undo ends in a
   commit (`FR-050`).
   - **First, the message**, as the commit flow asks for one (`FL-05`). It is prefilled with `Undo: <the block's first
     patch message>` and is editable.
   - **The card**, under the heading *"Undo this block"*, says:
     - *"This adds a new change that puts back what this block changed. History keeps both."*;
     - the block's id and its messages;
     - **the files, one per line, in stikk's words**: *put back* (`modified`, `binary`, `deleted`), *remove* (`added`),
       *rename back* `new → old` (`renamed`), *put back the permissions* (`mode`);
     - the consequence: *"After this, seal as usual to make the undo part of the history."*
   - The preview is **stikk's derivation from prikk's `diff`, labelled so**, as the commit preview is (RFC 014),
     because prikk has no plan for this operation.

6. **Execution, in an order that touches the worktree last.** After the preview's change token is re-checked
   (`C-T4b`):
   1. **Fetch every earlier file first** (`save_file`) into a fresh temporary directory **outside the repository**.
      A failure here has written nothing to the worktree.
   2. **Apply**, in this order:
      - renames back (`rename_path`);
      - removals;
      - every fetched file copied into place with its parent's mode (on Unix);
      - every directory the undo emptied, removed.

      Then delete the temporary directory.
   3. **Commit**, with the message (`commit`).

   **A failure in step 2 or 3 stops at once.** stikk commits nothing, and its result says which paths it changed and
   *"Changes (`w`) shows them."* prikk's refusal text, if there is one, is shown verbatim (`ER-02`). **Never retried**
   (`NFR-S04`).

   **Every path stikk writes comes from prikk's own report.** Before any write, stikk joins each one to the repository
   root and checks the result is inside the worktree and outside `.prikk/`. A path that fails the check refuses the
   whole undo before step 1.

7. **This is the first time stikk writes worktree files itself.** It does so only in decision 6, only at paths
   prikk's report names, and only after guard 1 has confirmed nothing uncommitted is there to lose. It writes only
   bytes prikk's `cat` gave it. It writes no state file and nothing under `.prikk/`. The threat model's `C-E2`
   concerns stikk's own state, and is unchanged. The RFC adds `C-T4f` beside it: *"stikk writes worktree files only to
   undo a block, at paths prikk names, after a clean-worktree check."*

8. **The guide gets *"Undoing a block"*.** It says what `U` does, that history keeps both blocks, that sealing comes
   next, and when stikk refuses and why. It links *"Going back to an earlier block"*, which is a different thing: that
   switches to the older files without recording anything.

9. **Not in this RFC:** undoing a merge, undoing several blocks at once (a series of single undos does it), undoing a
   block on another branch, and sealing automatically. Seal stays the user's own step, with its own card.

## Changelog

- `### Added`: undo one block from History (`U`). stikk puts back what the block changed and records it as a new
  commit, so history keeps both. It refuses when the worktree has changes, work is queued, the block is a merge or the
  first on its branch, or a later block changed the same files. At prikk ≥ 0.54.
- `### Breaking`: whatever `cargo public-api` shows. The `Prikk` trait gains one required method.

## Delivery

**One handoff.** It must land before 0.20.0's increment 2 (the prikk 0.56.0 re-baseline), which re-measures `commit`.

1. **It re-runs `measure-undo.sh` at 0.54.1 and 0.55.0 first**; the band rests on both.
2. **Unit tests** cover every guard, the inverse table, the order of execution, a failure in each of steps 1–3, and the
   path check.
3. **Suite legs at the ceiling**, each checked through `tree` and `cat` against the expected point:
   1. undo the newest block, with every status F2 lists → **identical to the parent**; `verify` passes;
   2. undo an older block whose paths no later block changed → that block's changes gone, the later one kept;
   3. a later block changed the same path → guard 5, **zero writes**;
   4. a dirty worktree → guard 1, zero writes;
   5. work queued → guard 2, zero writes;
   6. a merge block, and the first block of a branch → guards 3 and 4;
   7. an untracked file in the way → guard 6;
   8. **on Windows:** a block with a `mode` entry → guard 7; a block with no mode change → leg 1's result.

   **At 0.28:** unavailable, with nothing spawned.
4. **Captures at 80 columns:** the card; each refusal; the result after a failure in step 2.

## What this RFC does not do

- **Erase or rewrite history.** The undo is always a new change.
- **Call itself a rollback**, or use prikk's `rollback-*` commands.
- **Write anywhere** except the worktree paths prikk's own report names.
- **Seal.**

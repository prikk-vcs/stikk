# Handoff A — RFC 058: Undo one block (v1)

**Companion to:** [RFC 058](../../done/058-undo-one-block.md), **accepted 2026-10-31**. **Read the RFC first**:
F1–F5 and decisions 1–9, including the two clarifications made at acceptance. **Covers:** all of it. **0.20.0's
increment 1.**
**Land it before 0.20.0's increment 2** (the prikk 0.56.0 re-baseline), which re-measures `commit`.
**Not a release.**

> **What this increment is.** `U` in History undoes one block. It inverts each entry of prikk's `diff --from <parent>
> --to <block>` in the worktree, then makes an ordinary commit. **stikk writes worktree files itself here for the first
> time** (decision 7), so the guards, the path check and the order of execution are the review's centre.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite with `full_platform_matrix: true` and supply chain there.

---

## 1. Measure first

Run `.git-exclude/reports/058-scope-0-20/measure-undo.sh <prikk> <empty scratch dir>` at **0.54.1** and **0.55.0**.
Save each output beside the script as `measure-undo-<version>.txt`; 0.55.0's is already there, so diff against it.
Ignore ids and paths. **The band rests on 0.54.1**: if its run differs from 0.55.0's in anything but ids, stop and
report.

**Two things the script does not do, which you must not copy from it:**
- It removes **every** empty directory. stikk removes only the directories the undo itself emptied (§3.4).
- It writes with `cat --output … --force` straight into the worktree. stikk fetches into a temporary directory with
  `save_file`, which never passes `--force` (RFC 053 decision 1), and copies from there (§3.3).

## 2. Commit 1: the seam (`stikk-prikk`)

1. **`rename_path(repo, from, to) -> Result<Renamed>`** runs `prikk mv <from> <to>`. Its category is
   **`QueueMutation`**.
   - `Renamed` carries prikk's stdout lines **verbatim**, and nothing parses them.
   - Below `FILES_FROM_MINOR` it spawns nothing and returns an `Unavailable` variant, as `save_file` does.
   - Add it to the null backend too, with the recorder.
2. **Nothing else in the seam changes.** The diff entry's `from.mode` is the parent's mode (RFC 058 F2, as
   clarified), so no `tree` read is added.

**Tests:** the argv; `Unavailable` below the band with **zero spawns**; stdout carried verbatim.

## 3. Commit 2: the operation (`stikk-core`, a new `undo` module)

Model it on `switch.rs` (Tier Three, a preview that lists files) and `commit.rs` (a message step before the card).
**`UNDO_OPERATION = "undo-block"`** is its operation id.

### 3.1 The preview: `undo_preview(prikk, repo, block_id, message) -> Result<UndoPreviewOutcome>`

Take every read inside `confirm::preview`'s `compute`, so the token and the guards describe one instant.

1. **Orientation** gives the current branch and the queued count. **History of the current branch** gives the block's
   row and the row below it, which is the parent (F4). A block not in that history is decision 3's unavailability.
2. **`diff --from <parent> --to <block>`** gives the entries. **`diff --from <block> --to <current tip>`** gives the
   later paths; when the block *is* the tip, skip this read.
3. **`worktree_status`** of the current branch.
4. **The guards, in RFC 058 decision 4's order.** The first that applies is the outcome, `Blocked(String)` with the
   RFC's sentence, and **no `PreviewToken` exists for it**. Exact rules:
   - **Guard 1:** blocked by `missing > 0`, `modified > 0`, a refused declaration, or any entry whose kind is not
     *untracked*. **An untracked file alone does not block.**
   - **Guard 3:** blocked by `kind == "Merge"` or `parents != 1`.
   - **Guard 4:** blocked when no row lies below the block in the current branch's history.
   - **Guard 5:** compare the set of every entry's `path` and every rename's `from_path` against every `path` and
     `from_path` of the later diff. Name **every** overlapping path, sorted, each through `inert`.
   - **Guard 6:** blocked when any path the undo would *create* (a `deleted` entry's `path`, a rename's `from_path`) is
     an untracked file in `worktree_status`'s entries.
   - **Guard 7:** blocked by a status not in F2's table (`symlink` included), any `unsupported_paths`, or on
     **Windows** (`cfg!(windows)`) a `mode` entry, or a `deleted` entry whose `from.mode & 0o111 != 0`.
5. **The path check** (decision 6) runs here too, before any token exists. Every path stikk would touch must be
   relative, with no `..`, no root and no prefix component. Joined to the repository root, it must stay inside the
   root, and its first component must not be `.prikk`. **A failure blocks the whole undo**, naming the path.
6. **`UndoPreview`** holds the block id, its messages, the plan (one step per entry: *put back*, *remove*,
   *rename back*, *put back the permissions*), and the message. Its `summary()` gives the card's lines (§4).

### 3.2 The tier and the gate

Tier **Three**. The signing requirement is **AUTHOR**, because the operation ends in `commit`. Use the gate as commit
does. **Do not add a new `RequestCategory`**: each seam call keeps its own category.

### 3.3 Execution: `undo_confirm_and_execute`, in decision 6's order

After the token re-check:

1. **Fetch.** Create a fresh directory under `std::env::temp_dir()`, with a unique name and `create_dir`, never
   `create_dir_all` on an existing path. Check that it is **outside the repository root**. Run `save_file(repo,
   parent, path, <tmp>/<n>)` for each path to put back, with `n` = 0, 1, 2… **so no nested directory is needed.**
   A failure here deletes the temporary directory and returns. **The worktree has not been touched.**
2. **Apply**, in this order:
   - each rename back (`rename_path(block_path, from_path)`);
   - each removal (`std::fs::remove_file`);
   - each fetched file copied into place, creating parent directories first. **On Unix, set the mode to
     `from.mode & 0o777`.** On Windows, set nothing: guard 7 has already refused every case where that matters;
   - the directories this undo emptied (§3.4).

   Record every path changed, in order.
3. **Delete** the temporary directory, whatever happened.
4. **Commit** with the message.

**A failure in steps 2 or 4 stops at once**, with nothing committed. It returns a result carrying the changed paths
and prikk's text verbatim, if there is one. **No retry anywhere.**

### 3.4 Empty directories

Only the parent directories of a path this undo **removed** or **renamed away**: walk each one upward toward the
root, removing it only if it is now empty, and stop at the first one that is not. **Never touch any other
directory.**

**Tests:**
- every guard, its sentence and zero writes;
- untracked elsewhere allowed (guard 1), and untracked in the way blocked (guard 6);
- the inverse table, entry by entry;
- the order of steps 1–4, asserted through the null backend's recorder and a scratch worktree;
- a failure injected at each of steps 1, 2 and 4: after step 1's, the worktree is byte-identical; after step 2's or
  4's, the result names exactly the paths changed;
- the path check, with `../x`, `/x`, `.prikk/x`, and `a/../../x`;
- §3.4's rule, with a directory the undo emptied and an unrelated empty directory, which stays.

## 4. Commit 3: the interface (`stikk-tui`)

1. **`U` → `Action::UndoBlock`**, in `keys.rs` (`u` is taken by `ToggleUntracked`). It is live in History on the
   selected block, and in Block detail. Add it to the palette and to the help line where History lists its keys.
   Outside the band, or on a ref that is not the current branch, it is unavailable **with the reason** (`C-T4d`).
2. **The message prompt** comes first, prefilled with `Undo: <the block's first patch message>`, through `inert`.
   Pressing Enter with an empty message keeps the prompt open, as commit's does.
3. **The card**, under *"Undo this block"*, has decision 5's lines in this order:
   - the two sentences;
   - the block id and messages;
   - the plan, one line per path with its verb, paths through `inert`, wrapped and never clipped;
   - the consequence.

   It confirms like a switch (Tier Three).
4. **The result.** On success: prikk's commit lines as commit shows them, then *"Seal as usual (`S`) to make the undo
   part of the history."* After a failure in step 2 or 4: the paths changed, then *"Nothing was committed. Changes
   (`w`) shows them."*, then prikk's text.

## 5. Commit 4: the suite (`stikk-real-binary`)

RFC 058's delivery legs 1–8, at the ceiling. **Leg 1's fixture holds every status in F2's table**, and its check is
`tree` + `cat` of the undo's tip against the parent's, every path, mode and byte, plus `verify`.
- Every refusal leg asserts **zero mutating spawns** and an unchanged worktree.
- **Leg 8** runs only `#[cfg(windows)]`, and the mode-entry fixture is `#[cfg(unix)]`, built the way the existing
  exec-bit leg builds its file. Say in the request how each platform's count changes.
- **At 0.28:** `U` is unavailable, with nothing spawned.

## 6. Text

- **`docs/src/guide/`:** *"Undoing a block"* (decision 8), beside *"Going back to an earlier block"*, which it links
  and which links back.
- **`requirements.md`:** add `FR-085` under §2.6 after `FR-084`, with the RFC's words and *"Delivered …, RFC 058"*.
  Add `UC-21` to the use-case table and to the traceability table. Point `FR-083` at `FR-085` in one clause: *"undoing
  a single block is `FR-085`"*.
- **`threat-model.md`:** add `C-T4f` under `T-T4`, in decision 7's sentence.
- **`CHANGELOG.md`, `## Unreleased`:** the RFC's two entries.

## 7. The traps

1. **Never write the worktree before step 1 has fetched everything.** That ordering is the whole safety argument for
   a failed fetch.
2. **Never pass `--force`**, and never write into the repository except the plan's paths.
3. **The parent comes from History's row below**, never from `previous_ref_state`, which is a RefState and not a
   block.
4. **Guard 5 compares `from_path` too.** A rename back onto a path that a later block re-created would otherwise slip
   through.
5. **Every path in every sentence goes through `inert`.** These are repository strings (`C-T2a`).
6. **Paste captures as the file holds them, and grep before committing.**

## 8. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/058-undo-one-block-a/review-request-v1.md`. Lead with:
- §1's two diffs;
- the guard table: each guard, its test and its suite leg;
- leg 1's identity check.

Include 80-column captures of the card, each refusal, and the result after a failure in step 2.

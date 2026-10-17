# RFC 045 — Switch branch

**Status.** **Done 2026-10-17**: Handoff A delivered on `main` (`c652405`, `86c4c85`, `0c7606e`); **released in 0.13.0**. **Accepted by the project owner 2026-10-17**, decision 3 (Tier Three rather than the scope's typed name)
with it; [Handoff A](../handoffs/045-switch-branch/a-switch-branch-handoff-v1.md) issued. **Proposed 2026-10-17** by the
architect, as 0.13.0's increment 3 (`.git-exclude/release/0-13-0-scope.md`, authorized). **No open question.**
Measured against a real prikk **0.52.0**. Evidence: `.git-exclude/reports/045-switch-branch/` (`measure045.sh`,
`measure045-0.52.txt`; `measure045-queued-target.sh`, `measure045-queued-target-0.52.txt`). prikk's source was read in
the clone (`prikk-store/src/branch_switch.rs`, `prikk-cli/src/branch.rs`).
**Tracks.** `FR-053` (plan-first materialization), `FR-055` (focus), `FR-120/121` (preview-first, tiers), `TU-09`,
`C-T4d`, `ER-02`, `C-T2a`; RFC 013 (confirmation), RFC 036 (names), RFC 038/041 (Compare), RFC 044 (the ref writes
this completes).
**Touches.** `stikk-model` (one request category), `stikk-prikk` (one seam method), `stikk-core` (a preview and an
execution), `stikk-tui` (the ref picker, the card, the result), `stikk-real-binary`, `requirements.md`, the changelog.

## Summary

RFC 044 made stikk write refs. **This RFC adds the one branch operation left: switch**, which moves prikk's current
branch **and rewrites the worktree** to the target's files. It is the first stikk operation that changes files on disk.
**The preview lists exactly which files will be written and deleted, from stikk's own Compare**, which measured as a
precise prediction of what prikk does. One case prikk allows leaves the worktree out of step with the queue, and stikk
does not offer it.

## Findings (measured at 0.52)

### F1 — the command, and what it prints

`branch switch heads/<name>`, **no flags at all**: any argument starting with `-` is a usage error (exit 2), so **there
is no plan or dry-run mode**. On success, exit 0:

```
switched from heads/main to heads/feature
files written: 2
files unchanged: 0
files deleted: 1
```

Switching to the branch already current prints `already on heads/main` and does nothing. **No key is needed**: the same
switch succeeds with every key variable unset. Prose only; the new current branch is read back from `status`.

### F2 — what it refuses, verbatim, and every refusal writes nothing

prikk checks all of these before the first write (`branch_switch.rs` module doc, and measured):

- **Missing**: *"heads/nope does not exist; run `prikk branch create heads/nope` first"*.
- **No prefix**: *"invalid name: ref feature is not a local branch ref; expected heads/<name>"*.
- **Closed**: *"heads/old is closed; switch to an open branch (`prikk branch list` shows them)"*.
- **Work queued on the branch being left**: *"the active WAL holds unsealed work for heads/main; seal it (`prikk seal
  --allow-no-audit --ref heads/main`) before switching to heads/feature"*.
- **A tracked file changed or missing**: *"the worktree is not clean against heads/main: a.txt (modified); commit it,
  or restore those files, before switching (`prikk worktree-status` lists them)"*; `(missing)` for a deleted one.
- **An untracked file where the target writes**: *"refusing to switch to heads/feature: 1 in the way of its files: c.txt
  (a file that is not part of the branch being left); move them aside and run the switch again (nothing was
  written)"*.

**An untracked file the target does not need is left alone**, and the switch succeeds around it.

### F3 — it loses nothing, by construction

prikk replaces a tracked file only after refusing a dirty worktree, deletes an old file **only if its bytes still equal
the old branch's**, never reads or deletes an untracked file, and writes the pointer last, inside the worktree-dirty
marker, so an interrupted switch is completed by running it again. **Switching back restores every byte**: everything a
switch replaces is clean, so it is in history.

### F4 — stikk's Compare predicts it exactly

`diff --from heads/main --to heads/feature --format json`, which stikk's Compare already reads (RFC 041), listed
`a.txt modified`, `b.txt deleted`, `c.txt added`. The switch then reported **written 2** (added + modified) and
**deleted 1** (deleted), and the worktree held exactly the target's files. Every other successful switch in the script
reported counts equal to the files its two branches differ by.

### F5 — work queued *for the target* is allowed, and is not written

prikk refuses work queued on the branch being left, but **allows a switch while the queue's work belongs to the
target**. It writes the target's **sealed** files only. After the switch the queued files are absent from the worktree,
`worktree-status` reports them **missing**, and a `diff` against the worktree lists them as **deleted** — so the next
`commit --from-worktree` would queue their deletion. Nothing is lost yet (the work is still in the queue), but the
picture is wrong, and stikk can reach this case itself: it commits to its focused ref, which may differ from prikk's
current branch (`FR-055`).

### F6 — the floor

`branch switch` arrived in prikk **0.42**, with the current-branch pointer. The preview needs `diff --from --to`, which
stikk reads from **0.46** (`DIFF_FROM_MINOR`).

## Decisions

1. **One seam method, one prikk call**: `switch_branch(repo, name)` runs `branch switch <name>`, no `--format json`.
   Exit 0 keeps prikk's lines **verbatim, stdout and any stderr lines** (prikk names a snapshot fallback on stderr at
   exit 0). **Never parsed for state**: the new current branch is read back from `status`. Every other exit is
   classified as today.
2. **A new request category, `Materialization`**: *writes the worktree from history* (switch now; checkout if it ever
   comes). It mutates, is never auto-retried, and is **gated on `Readiness::may_operate`** (any human at the machine,
   not in read-only mode): F1 needs no key, so stikk asks for none.
3. **Tier Three: restate, then an explicit yes.** **This departs from the scope's wording**, which promised "the
   highest confirmation tier" (typed name, `ThreeTyped`) before the command was measured. Typing the name back is for
   actions that are irreversible or that change trust. F2 and F3 show a switch is neither: it refuses rather than
   overwrite anything not in history, every refusal writes nothing, and switching back restores every byte. What the
   user needs is **to see which files will change**, and decision 5 gives exactly that. `Materialization`'s tier is
   the category's, so a later checkout inherits the same decision.
4. **Banded at prikk ≥ 0.46, reusing `DIFF_FROM_MINOR`.** The command exists from 0.42, but a switch without its file
   list is not offered (`FR-053`: the user confirms the plan, not the idea). Below 0.46, stikk's reason (`C-T4d`).
5. **The preview is the file plan**, from `diff --from <current branch> --to <target>` read **inside the preview**:
   - the two branch names and the target's short block id, in `target_ids` (rendered through `identifier`, RFC 036);
   - **counts**: files written (added + modified) and files deleted;
   - **the paths**, each marked written or deleted, rendered inert (`C-T2a`), as many as the card holds, then *"and N
     more"* — never a silent cut;
   - the consequence, in stikk's words with **no name in it** (RFC 044 review C1): the worktree will be rewritten to the
     target's files; untracked files are not touched; prikk refuses, writing nothing, if a tracked file has changes or
     an untracked file is in the way.
   A target with **no differences** says so: the switch moves the pointer and writes nothing.
6. **What stikk does not offer, each with its own reason and without running prikk:**
   - **the current branch** (*"already the current branch"*), and **a closed branch**, from the list's own flags;
   - **while work is queued for the target** (F5): *"work is queued for this branch; seal it first, or the worktree
     would not show it"*. prikk does not police this one; stikk does, as RFC 044 did for F3;
   - **when the current branch cannot be compared** (unpublished or unresolved, or the plan read fails): there is no
     file plan, so no switch is offered, and the reason names `prikk branch switch` as the way out. In that state it is
     prikk's repair route, and it writes only files that are absent or already identical.
   **Work queued on the branch being left, and a dirty worktree, are left to prikk's own refusals** (F2), which already
   name the way out.
7. **The key is `s` in the ref picker**: switch to the selected branch. `Enter` keeps its meaning (focus stikk on a
   ref, touching nothing; `FR-055`), and the card says the difference in one line. Listed in the Glossary. Only in the
   picker: a switch acts on a branch, not a block.
8. **After success**: the result shows prikk's lines verbatim; orientation, the ref lists and Changes are re-read; and
   **stikk's focus moves to the switched-to branch**. `FR-055` says a later *read* never moves focus; this is the
   user's own action, and leaving History on the old branch would show a worktree that is no longer its files.
9. **RFC 044's current-branch refusal** stops naming `prikk branch switch` and says *"Switch to another branch first
   (`s` in the ref picker)"*.
10. **F5 goes to prikk in the next necessary letter**, not a letter of its own: is leaving target-queued work
    unwritten intended? stikk's decision 6 is right either way.

## Changelog

`### Added`: switch branch, from the ref picker, previewed as the exact files it will write and delete, at prikk ≥ 0.46.
`### Breaking`: the `Prikk` trait's new method; `RequestCategory::ALL` grows to ten.

## Delivery

**One handoff, issued on acceptance, in two commits**: the category and seam method with their tests, then the
preview, card, key and result. It re-runs `measure045.sh` and `measure045-queued-target.sh` at 0.52 first. Suite legs
at 0.52: a switch whose written and deleted paths equal the preview's, with the worktree's files checked after; a dirty
worktree refused verbatim with nothing written; an untracked file left alone; the target-queued case not offered
**with no `branch switch` spawned**; switching back restores the original bytes. At 0.28, the below-band reason. **0.46
is not measured** (no user is on that band); every refusal is prikk's and the plan is re-read at preview, so an older
prikk can produce an error, never a wrong picture.

## What this RFC does not do

- **No checkout** of a block or a path, and **no switch to a block**: `branch switch` takes a branch.
- **No "switch and carry my changes"**: prikk refuses a dirty worktree, and stikk does not stash on its behalf.
- **No creating and switching in one step**: `n` then `s`.

## Delivered

**One handoff, three commits, green on `main` at `0c7606e`**: CI, Docs, the real-binary suite (55 cases at 0.28.0 and
0.52.0), supply chain and `installers.yml`. Both measurement scripts reproduced F1–F5 before any code moved.

- **`RequestCategory::Materialization`**, appended to keep the derived order stable, at Tier Three. **The signing
  requirement is now the category's, not the tier's** (`RequestCategory::signing_requirement`): left tier-keyed, a
  switch would have demanded a maintainer key it does not use. A test holds every other category to its old
  requirement and asserts `Materialization` still differs.
- **The seam** runs `branch switch <name>` banded at `DIFF_FROM_MINOR`, checks read-write only, and keeps stdout and
  stderr apart, verbatim.
- **The plan** is classified by prikk's own `DiffStatus` vocabulary; `symlink`, any unmeasured status, and any
  unsupported path block it as incomplete. Only a comparison **prikk refused** (or `Unreported`) means *no file plan*;
  a lock or environment fault travels as itself (review v1 C1). The overflow row names RFC 041's route to the full list
  (review v1 C2).
- **F5 is blocked in stikk's words** with no `branch switch` spawned, proved in the suite by prikk's own `status`; the
  suite also proves the preview's paths equal what the switch did, and that switching back restores the bytes.
- **After success** focus moves to the new branch and Changes is re-read against it; RFC 044's refusal now names `s`.
- **Decision 10**: F5 is held for the next necessary letter to prikk.

# RFC 008 — Worktree Changes via a now-fixed `worktree-status`; Compare deferred behind the content ceiling

**Status.** Implemented (0.1.0) — **Changes** (worktree-vs-baseline) shipped on prikk's
`worktree-status`, verified **fixed as of prikk 0.28** (UD-03 was a 0.27.x defect).
**Deferred, carried forward (not built by this RFC):** **Compare (FR-033)** — prikk exposes no command
that can compute a two-tree difference honestly, so a Compare built now would mislead (T-T4);
re-verified against prikk 0.30 on 2026-09-04 (`checkout` is still ref-tip-only, there is still no
`show`/`diff`/`compare`), with the materialize-to-temp route recorded below. Also deferred: **per-file
content diffs** (UD-09), the **`C` commit action**, and the **status-bar worktree marker** (TU-03).
**Superseded in part by [RFC 009](./009-prikk-0-30-rebaseline-and-parser-fidelity.md)**, which corrects
this increment's untracked-filter copy and adds the `queued_elsewhere` warning this RFC's fixtures
missed. Handoff:
[`../handoffs/008-worktree-changes-and-the-compare-ceiling/changes-view-handoff-v1.md`](../handoffs/008-worktree-changes-and-the-compare-ceiling/changes-view-handoff-v1.md).
**Tracks.** The roadmap's "Next" increment 5 — Compare (`FR-033`) and Changes / worktree-vs-baseline
(`FR-034`, via the `UD-03` route).
**Touches.** `stikk-prikk` (a `worktree_status` seam method + confined parser, with a special
dirty-exit rule), `stikk-core` (a `changes_view` read operation, version-gated), `stikk-tui` (a Changes
view + navigation). Nothing mutates; nothing below the seam's contract changes.

## Summary

Increment 5 was scoped as *Compare + Changes*. As RFC 006 did for history, this RFC records what a
check of the **real prikk binary** (0.28.0, at `/…/prikk-git`, verified — not assumed) allows each of
the two to actually be, because the finding changes the increment: **Changes is now a first-class,
correct surface** (the command the design's UD-03 workaround pointed at is fixed), and **Compare cannot
be built without misleading the user** and is deferred with a concrete future route.

## The findings that scope this increment

Verified empirically against prikk 0.28.0 (`worktree-status`, `checkout --help`, a scratch repo with a
committed baseline and a dirtied worktree):

1. **`prikk worktree-status [path] [--ref REF]` works** and is the honest source for Changes. Its
   report (verbatim):
   ```
   worktree-status repository: <path>/.prikk
   ref: heads/main
   tracked files: N
   unchanged files: N
   missing files: N
   modified files: N
   untracked files: N
   unsupported paths: N
   worktree: clean against baseline        (or: changed against baseline)
     modified <path> — tracked file bytes differ from the baseline
     missing <path> — tracked file is absent from the worktree
     untracked <path> — worktree file is not in the baseline
   note: use `prikk commit …`
   ```
   It computes against the **replay baseline including the committed (queued) WAL** — no seal is
   required. **UD-03 is resolved as of 0.28** (the audit's 1A-High-1 was 0.27.x); this is verified, not
   assumed.
2. **A dirty tree exits non-zero.** `worktree-status` prints the report to **stdout** and, when the
   tree differs, an `error: worktree has changes against the baseline` line to **stderr** with **exit
   code 1**; a clean tree exits **0**. The non-zero exit is a *normal status result*, not a semantic
   refusal — a corollary of UD-05 (prikk overloads exit 1). stikk must parse stdout regardless of exit
   and must **not** route the dirty case through the failure classifier.
3. **worktree-status is path-level, not content-level.** `modified` means "tracked file **bytes
   differ** from the baseline"; there is no per-file line diff, no old/new content, no per-file size.
   Per-file **content** diffs (part of FR-034) are the same ceiling RFC 006 recorded as UD-09.
4. **Compare (FR-033) has no honest command.** There is **no `prikk diff`, `compare`, or `show`**;
   `checkout` replays only a **ref tip** (`[path] [--ref REF]`), never an arbitrary block; and the plan
   output carries a file *list* + an *aggregate* byte total, **no per-file content and no per-file
   size**. So a Compare between two blocks could, at most, set-difference two sealed ref tips' file
   lists into added/removed — and would be **unable to detect a content change**, reporting two
   differing files as identical. That is precisely the "confident-but-wrong picture" the threat model
   forbids (**T-T4**). A misleading Compare is worse than none.

## Decisions

1. **Ship Changes on `worktree-status`, version-gated** (design FR-034/UD-03). Add a seam method
   `worktree_status(repo, ref)`; the operation layer's `changes_view` requires prikk **≥ 0.28** (where
   the command is fixed) and, below it, returns stikk-authored guidance ("worktree review needs prikk
   ≥ 0.28; before that `worktree-status` is unreliable — UD-03") **instead of invoking the broken
   command** — satisfying FR-034's "must not present the broken command's error to users" by not
   running it. The imagined replay/plan workaround is **superseded**: it was never actually feasible
   (it needs per-file baseline content prikk does not expose, and would require stikk to read worktree
   bytes directly, against CON-1).
2. **Parse the report regardless of exit** (finding 2). The `worktree_status` seam method reads the
   report from **stdout** whether prikk exits 0 (clean) or 1 (dirty); only when stdout does **not**
   carry the report shape is the outcome treated as a real failure (classified per UD-05). Golden
   fixtures pin the clean and dirty shapes; the parser **refuses on an unrecognized shape** (UD-02).
3. **Changes is path-level and honest about the content ceiling.** It shows the counts and the
   modified/missing/untracked/unsupported paths, and names where a per-file **content diff** would open
   that per-file content awaits prikk support (UD-09). No faked diffs — the RFC 006 rule.
4. **Whole-worktree and untracked honesty in the UI** (UD-06/UD-08). Changes states plainly that
   commits are whole-worktree (no staging), and offers a **display-only** untracked filter that, when
   active, keeps a banner saying a commit would still capture those files. Changes ships **read-only**
   this increment; the `C` commit action is the mutation increment.
5. **Defer Compare (FR-033)**, splitting it out like RFC 006's patch detail. Record the concrete
   future route (below) rather than a bare "wait for upstream."

## Upstream dependency

**Extends UD-09** (RFC 006: prikk exposes no per-patch / per-file content surface). Two stikk features
wait on it: Changes' **per-file content diff**, and **Compare** in full (added/removed **and**
content-/mode-changed, each expandable to a content diff). The clean upstream asks, in priority order:

- a **content surface** — `prikk show`/`diff`, or per-file content in the plan/`--format json` output —
  which unblocks Changes' per-file diff, block/patch detail (RFC 006 3b), *and* Compare; and
- a **two-tree compare** (`prikk compare <block|ref> <block|ref>`), or block-addressable checkout
  (`checkout --block ID`) so stikk can replay two arbitrary blocks, not only ref tips.

**A concrete interim route for Compare (recorded, not built here):** `checkout --patch-materialize
<dir> --ref REF` writes a ref tip's replayed files; stikk could materialize two ref tips to temporary
directories and compute a true content diff from prikk-produced bytes. It is correct but write-heavy
and is its own increment (temp orchestration, large-tree limits per NFR-P01, cleanup); it is the
intended path once Compare is picked up, ahead of any upstream change.

## Open questions

- **Ship a thin added/removed-only Compare now?** *Ruled no:* without content-change detection it would
  label differing files "unchanged" (T-T4). The honest minimum for Compare needs content, so it waits
  for the content surface or the materialize route (above).
- **A worktree marker in the status bar (TU-03 clean/dirty/unknown)?** *Ruled deferred:* it needs a
  `worktree-status` call at open (an extra spawn, version-gated) and belongs with the commit increment
  where worktree state is central. Changes is the surface for now; the status bar shows `unknown`.
- **Gate by version or by behaviour?** *Ruled by version (≥ 0.28):* it is the only way to honor "never
  present the broken command's error" — the 0.27.x defect is not a clean error to detect after the
  fact, so stikk does not run the command there at all.

## Consequences

- `stikk-tui` gains a Changes view (path-level, grouped, with the UD-08 filter and the UD-06/UD-09
  honesty notes); the view stack and inert-text primitive extend to it. `stikk-core` gains one
  version-gated read operation; the seam gains one method and one parser with the dirty-exit rule.
- **UD-03 is recorded resolved (verified at 0.28)** and stikk adopts the real command — an update to
  the design's standing assumption grounded in the binary, not a guess.
- Compare is explicitly deferred with a concrete route, so the gap is a stated property, not a
  surprise, and the next person has a plan rather than a blocker.

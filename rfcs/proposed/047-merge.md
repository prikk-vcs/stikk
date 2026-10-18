# RFC 047 — Merge

**Status.** **Proposed 2026-10-18** by the architect, as 0.15.0's increments 2 and 3
(`.git-exclude/release/0-15-0-scope.md`, authorized). **No open question.**
Measured against a real prikk **0.52.0**, one fresh repository per scenario. Evidence: `.git-exclude/reports/047-merge/`
(`measure047.sh`, `measure047-0.52.txt`; `measure047-0.52-contaminated.txt` is the first run, kept because what
contaminated it is F3). prikk's merge guide and source were read in the clone (`docs/src/guide/merge.md`,
`prikk-store/src/merge/execute.rs`).
**Tracks.** `FR-080/081/082`, `FR-120/121`, `TU-09`, `C-T4d`, `ER-02`, `C-T2a`, `UD-02`; RFC 013 (confirmation), RFC 016
(seal, the same tier), RFC 036 (names), RFC 041 (marking two refs), RFC 044/045 (refusals stikk adds where prikk does
not police).
**Touches.** `stikk-prikk` (two seam methods), `stikk-core` (a baseline finder, a plan read, a merge preview and
execution), `stikk-tui` (the ref picker, a Merge plan view, the card), `stikk-real-binary`, `requirements.md`, the
changelog.

## Summary

stikk can create, switch and close branches, but not bring one into another. prikk can, **only when it has proven the
two sides confluent**, and refuses everything else without writing anything. **This RFC shows prikk's merge plan for
two branches, and then runs the merge it describes.** Two things prikk leaves to its caller decide the design: **the
baseline block must be named**, and prikk exposes no parents to find it from; and **a merge into the branch you are on
leaves the worktree behind its new tip**. stikk offers merge only where it can name the baseline truthfully, and never
into the current branch.

## Findings (measured at 0.52)

### F1 — three commands, prose only

- `merge-evidence --baseline-block B --left-ref L --right-ref R` and `merge-plan` (same arguments): read-only, exit 0
  for any outcome. `--format json` is **a usage error** (exit 2). The plan prints the evidence's lines and adds
  `status:` (`ConfluentSubset`, `BlockedConflict`) and `action:`, so **the plan is a superset of the evidence**.
- One line carries the verdict: **`evidence outcome: Confluent`** (or `Conflict`, …; prikk's guide lists `Deferred`,
  `NotConfluent`, `Unsupported`, `OrderedDependency`, `EvidenceFailure`, `InvalidCandidate`).
- `merge --allow-no-audit --baseline-block B --into I --from F`: exit 0 prints `merged F into I`, the baseline, parent
  and adopted blocks, the adopted patch ids, the new block id and RefState. Without `--allow-no-audit`: exit 2. **No
  MAINTAINER key**: *"invalid signature: maintainer signer public key does not match trusted key maintainer"*. A
  non-confluent pair: *"merge refused: F is not confluent with I from baseline B (outcome: Conflict, reason:
  pair_conflict)"*, and **refusal writes nothing** (prikk's guide, and measured).
- **The identical change on both sides is a `Conflict`** (scenario 4) — prikk's documented rule, not stikk's to soften.

### F2 — the baseline is the caller's, and prikk gives no parents to find it from

- prikk refuses a baseline that is not an ancestor of both tips (*"baseline Block … is not an ancestor of target Block
  …"*) or does not exist (*"block … is not in this repository"*). **Automatic merge-base discovery is deferred** in
  prikk's own guide.
- `log --format json` lists **the mainline only**, with `kind` (`Root`, `Normal`, `Merge`) and `parent_count`, but
  **no parent ids**; `show --format json` of a merge block carries none either.
- **A wrong baseline is not harmless.** With a baseline older than the true merge base, scenario 1's re-merge became a
  `Conflict` that the true base (main's tip) proves `Confluent`. And prikk records the baseline on the merge block,
  where **`verify` re-derives the true merge base and reports disagreement** (prikk's guide).
- **Where neither lineage holds a `Merge` block above their latest shared block, that block *is* the merge base**: every
  block on both chains has one parent, so the latest common one is the only candidate.

### F3 — a merge into the current branch leaves the worktree behind

Scenario 2: `merge --into heads/main` while `heads/main` is current **succeeds and writes no file**. The worktree then
lacks the adopted `c.txt`; `worktree-status` reports it **missing**; the next `commit --from-worktree` would queue its
deletion; and **`branch switch` refuses** (*"the worktree is not clean against heads/main: c.txt (missing)"*), so the
way out is not even a switch. Merging into a **non-current** branch (scenario 1) leaves the worktree clean, and a later
switch to it writes the adopted file.

### F4 — work queued on the branch merged into

Scenario 5: with unsealed work queued for `heads/main`, `merge-plan` judges main's **sealed** tip, and `merge --into
heads/main` succeeds; the queued work then seals on top of the merge block. prikk allows it; whether the queued patch
still means what it did against the old tip is not something stikk can see.

## Decisions

1. **Two seam methods.** `merge_plan(repo, baseline, left, right) -> Vec<String>` runs `merge-plan`, keeping prikk's
   lines verbatim; `merge(repo, baseline, into, from) -> Vec<String>` runs `merge --allow-no-audit` at `Publication`
   (Tier Three, MAINTAINER readiness checked before prikk runs, as seal). `--allow-no-audit` is passed exactly as stikk's
   seal already passes it. Neither has JSON (F1); every other exit classifies as today.
2. **One line is read from the plan, and only one**: `evidence outcome: <word>`, matched exactly, the word one of F1's
   nine. It decides **only whether stikk offers the merge**. A missing line, a second one, or an unknown word is a shape
   error (`UD-02`: refuse, never guess), and no merge is offered. Everything else prikk printed is shown **verbatim**
   and never parsed.
3. **stikk names the baseline only when it can name the true one** (F2): the latest block both mainline lineages share,
   **provided neither lineage has a `Merge` block above it**. Otherwise the plan is not requested, and stikk says why
   in its own words: after an earlier merge, prikk does not expose what stikk would need to find the merge base. The
   baseline is shown (short id) with the two tips, so the reader can check it.
4. **Where it lives**: in the ref picker, **mark a branch (`m`), select another, press `M`**: the **Merge plan** view,
   for *merging the marked branch into the selected one* — marked is `--from`, as marked is *from* in Compare
   (RFC 041). Both must be branches (`heads/`). It shows the two names, the baseline, and prikk's plan verbatim.
5. **What stikk does not offer, with its own reason and without running prikk:**
   - **merging into the current branch** (F3): *"merging into the branch you are on would leave the worktree without
     the merged files, and prikk would then refuse a switch. Switch to another branch first."*;
   - **merging into a branch with work queued** (F4), as switch declines F5;
   - either side **closed**, or **unpublished** (RFC 037 B), or **the same branch twice**;
   - **no truthful baseline** (decision 3).
6. **The merge, from the plan view** (`Enter`): offered only when the plan's outcome is `Confluent`. The preview
   **re-reads the plan inside the preview**, so what is confirmed is current, and restates: *from*, *into*, the
   baseline, the adopted side's tip — names in `target_ids` through `identifier`, no name in the consequence (RFC 044
   C1) — and prikk's plan verbatim. **After success**: prikk's lines verbatim, the ref lists and History re-read, and
   one stikk line: switch to the branch merged into (`s`) to have its files in the worktree.
7. **Banded at prikk ≥ 0.42**, reusing `CURRENT_BRANCH_FROM_MINOR`: decision 5's first guard needs the current branch,
   and decision 3 needs `log`'s `kind` (0.39). Below, stikk's reason (`C-T4d`).
8. **Three items go to prikk in the next necessary letter**, none blocking: parents (or a merge base) in `log`'s JSON;
   F3 (should a merge into the current branch write the worktree, or refuse?); and `--format json` for the plan. Each
   would let stikk widen decisions 2, 3 and 5; none is needed for this RFC.

## Changelog

`### Added`: the Merge plan view (prikk's own plan for two branches, with the baseline stikk found), and merging one
branch into another when prikk proves them confluent, preview-first at seal's tier, at prikk ≥ 0.42.

## Delivery

**Two handoffs, issued on acceptance.** **A, read-only** (increment 2): the `merge_plan` seam method, the baseline
finder, the one-line read, the view and the guards. **B, execution** (increment 3): the `merge` seam method, the
preview, the card, the result. Each re-runs `measure047.sh` at 0.52 first. **Suite legs at 0.52**: the baseline found
for scenario 1 equals the block prikk accepts, and a lineage with a `Merge` block is declined; a confluent plan offers
the merge and a conflicting one does not; **the merge into a non-current branch lands, and a later switch writes its
files**; **into the current branch is declined with no `merge` spawned**. **0.42 is not measured** (no user is on that
band); every refusal is prikk's, verbatim.

## What this RFC does not do

- **No conflict resolution**: prikk refuses it by design, and stikk does not imitate it.
- **No merging into the current branch**, and no switch-merge-switch done on the user's behalf.
- **No merging a tag, a block id, or more than two sides.**
- **No merge-base guess** where decision 3 cannot name the true one.

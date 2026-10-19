# RFC 049 — The prikk 0.53 re-baseline

**Status.** **Accepted by the project owner 2026-10-19**; [Handoff A](../handoffs/049-prikk-0-53-rebaseline/a-rebaseline-handoff-v1.md) issued. **Proposed 2026-10-19** by the architect, as 0.15.0's conditional increment
(`.git-exclude/release/0-15-0-scope.md`: *"if prikk 0.53.0 publishes before the cut, a re-baseline joins"*). prikk
0.53.0 published 2026-10-18 (crates.io; prikk's letter 024). **No open question.**
Measured against real prikk **0.53.0** and **0.28.0** binaries. Evidence:
`.git-exclude/reports/049-prikk-0-53-rebaseline/suite-0.28-0.53-ceiling-raised.txt`.
**Tracks.** `ASM-2`, `NFR-R03`; RFC 043 (the last re-baseline, and its shape), RFC 034 A1 (raise the ceiling first),
RFC 039 (the installers' prikk pin).
**Touches.** `stikk-prikk/src/version.rs`, `ASM-2`, the READMEs, the guide, the docs landing page, and the changelog.

## Summary

**prikk 0.53.0 changes nothing stikk reads.** Its one output change is to `checkout`, which stikk calls only below
prikk 0.46. **So this re-baseline raises the validated ceiling to 53 and says so**, and nothing else moves.

## Findings

### F1 — the suite, with only the ceiling raised

On a scratch copy of `main` at `6916824` with `VALIDATED_MAX_MINOR` set to 53 and nothing else, at prikk 0.28.0 and
0.53.0: **58 passed, 0 failed**; the workspace's own tests, **1132 passed, 0 failed**. Unlike RFC 043's F1, **no leg
moved.** (Against `main` unchanged, every leg is refused by the harness by design: it requires the ceiling binary to
equal the validated ceiling, so the matrix can never silently run against an unvalidated prikk.)

### F2 — what 0.53.0 changed, and why none of it reaches stikk

- **`checkout`'s six refusing modes** now say a never-sealed branch *"has no sealed block yet (N commits queued)"*
  instead of *"does not exist"*, and `--plan-only`'s note is reworded (prikk's changelog; letter 024 §1). **stikk calls
  `checkout --patch-plan` only below `TREE_FROM_MINOR` (0.46)** (`cli_backend.rs:530`); from 0.46 it reads `tree`.
- **`ABSENT_REF_CLAUSE`** (*"does not exist in this repository"*) is matched on `worktree-status` and `log`, which did
  not change; a ref that was never created still says it (letter 024 §1).
- **No JSON output changed** (letter 024 §2; prikk's changelog lists none). `trust maintainer check`, whose route
  changed, is never called by stikk.
- **A cold `commit` is cheaper**, and below a checkpoint no longer refuses over a damaged stored file that the change
  does not need. stikk shows commit's refusals verbatim and claims nothing about this case.

## Decisions

1. **The validated ceiling is 53**, raised in one commit. The suite at 0.28.0 + 0.53.0 at that commit is F1's: green.
2. **`ASM-2`, both READMEs, the guide and the docs landing page say *validated through 0.53.0*.** The installers need
   no change: they derive prikk's pin from `version.rs` (RFC 039 B), so the next release pins 0.53.0 by itself.
3. **The suite's pinned ceiling moves to 0.53.0** wherever the suite's binaries are installed (CI derives it; local runs
   install 0.53.0).
4. **No letter.** Letter 024 needs no reply; its subject cannot reach stikk.

## Changelog

`### Changed`: validated through prikk 0.53.0.

## Delivery

**One handoff, issued on acceptance, one commit**: the ceiling and the docs together (no band moves, so RFC 034 A1's
"ceiling first and alone" has nothing to separate it from). The suite at 0.28.0 + 0.53.0 before submitting; after
approval, the full-matrix suite, supply chain **and `installers.yml`**, which must derive and install 0.53.0.

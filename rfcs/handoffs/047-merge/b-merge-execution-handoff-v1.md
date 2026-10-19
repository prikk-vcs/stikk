# Handoff B — RFC 047: the merge (v1)

**Companion to:** [RFC 047](../../done/047-merge.md), **accepted 2026-10-19**. **Covers:** decisions 1 (the `merge`
method), 6 and 8's stikk side. 0.15.0's increment 3. **Starts only after [Handoff A](./a-merge-plan-handoff-v1.md) is
approved and pushed.** **Not a release.** **No branch.**

---

## 1. Commit 1: the seam

**`merge(repo, baseline, into, from)`** at **`Publication`** (Tier Three, MAINTAINER readiness checked **before** prikk
runs, as `seal` and RFC 044's writes): `merge --allow-no-audit --baseline-block <b> --into <i> --from <f>`, one argument
each. Exit 0 keeps prikk's lines verbatim, **never parsed for state**; every other exit classifies as today. Banded at
`CURRENT_BRANCH_FROM_MINOR`. **Tests**: the argument vector; F1's success lines; the non-confluent and no-key refusals
verbatim; the usage exit without `--allow-no-audit` is stikk's own bug class (it must never happen); nothing spawned
below the band.

## 2. Commit 2: preview, card, result

- **`Enter` in the Merge plan view**, offered **only** when the view's outcome is `Confluent`; otherwise the view's own
  line says why there is nothing to run.
- **The preview re-runs the baseline finder and `merge_plan` inside `confirm::preview`**, so what is confirmed is
  current; if the outcome is no longer `Confluent`, or the baseline changed, it is blocked in stikk's words. Re-check
  **every Handoff A guard** there too (the current branch may have changed since the view opened).
- **The card**: `target_ids` = from, into, the baseline short id, the adopted tip's short id (all through
  `identifier`); **no name in the consequence** (RFC 044 C1); prikk's plan below, inert, fitted whole-lines-first
  (`frozen_lines`'s rule) with a count for what does not fit.
- **After success**: prikk's lines verbatim on the result card; refs and any open History re-read; one stikk line —
  switch to the branch merged into (`s`) to have its files in the worktree.

## 3. Tests and the suite

- **Core/TUI**: the readiness stop without a MAINTAINER key, in stikk's words; a preview whose plan turned non-confluent
  is blocked; each guard re-checked at preview; Tier Three; the result re-reads.
- **Suite at the ceiling**: **scenario 1 end to end** — preview, confirm, merge lands on the non-current branch, prikk's
  own `log` shows a `Merge` block on it, and **a switch to it writes the adopted file**; a conflicting pair is never
  armed; **into the current branch, nothing is spawned** (prikk's `log` of that branch unchanged). At 0.28, unavailable.
- **Captures at 80×24**: the preview, an overflowing plan on the card, the result.

## 4. Text

`FR-082` delivery note; `## Unreleased` `### Added`: merging one branch into another, preview-first, when prikk proves
it confluent. `### Breaking` for the trait method and anything `cargo public-api` shows.

## 5. Submit

The eight gates, the suite at 0.28.0 + 0.53.0. Package `.git-exclude/review-request/047-merge-b/review-request-v1.md`.
**After approval**: push, then the suite and supply chain on `main`.

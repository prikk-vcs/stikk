# Handoff A — RFC 047: the merge plan, read-only (v1)

**Companion to:** [RFC 047](../../done/047-merge.md), **accepted 2026-10-19**, no open question. **Read the RFC
first**, F1–F3 especially. **Covers:** decisions 2, 3, 4, 5, 7 (the read-only half). 0.15.0's increment 2.
[Handoff B](./b-merge-execution-handoff-v1.md) is the merge itself and comes after this one lands. **Not a release.**
**No branch.** **Start after RFC 049's Handoff A**, so you measure at the 0.53 ceiling.

---

## 1. Measure first

Re-run `.git-exclude/reports/047-merge/measure047.sh` at the ceiling (0.53.0) and confirm F1–F4 row for row. **F2's
scenario 1 and F3's scenario 2 especially.** If any row differs, stop and report.

## 2. Commit 1: the seam and the plan read

- **`merge_plan(repo, baseline, left, right)`** at `ReadHistory` (or `ReadState`; your call, Tier One either way): runs
  `merge-plan --baseline-block <b> --left-ref <l> --right-ref <r>`, **no `--format json`** (exit 2, F1). Exit 0 keeps
  prikk's lines verbatim; banded at `CURRENT_BRANCH_FROM_MINOR`, nothing spawned below.
- **The one line** (decision 2): `evidence outcome: <word>`, **exactly once**, the word one of F1's nine (`Confluent`,
  `Conflict`, `Deferred`, `NotConfluent`, `Unsupported`, `OrderedDependency`, `EvidenceFailure`, `InvalidCandidate`
  — take the list from prikk's `docs/src/guide/merge.md` in the clone and cite it). Missing, repeated or unknown →
  a shape error (`UD-02`). Return the lines and the outcome **apart**; nothing else is parsed.

**Tests**: the argument vector; F1's confluent and conflicting outputs as fixtures (from your §1 run, pasted as the
file holds them); each refusal verbatim; the one-line read's three failure shapes; nothing spawned below the band.

## 3. Commit 2: the baseline, the view, the guards

- **The baseline finder** (decision 3), in `stikk-core`, a pure function over two `History`s (`BlockRow` already has
  `block_id`, `kind`, `parents`): the latest block id in both lineages, **offered only if no block above it on either
  side has `kind == "Merge"`** (or `parents > 1`; check both agree on your fixtures). Read **whole** lineages: if a
  read is truncated before a shared block is found, there is no baseline. Its "none" carries stikk's reason.
- **The view.** In the ref picker: `m` marks (exists), select another, **`M`** opens **Merge plan** for *marked into
  selected* (marked = `--from`, RFC 041's convention). It shows **from**, **into** (through `identifier`), the baseline
  short id, and prikk's plan **verbatim, inert, scrollable**. `M` with nothing marked says so, as `d` does.
- **The guards** (decision 5), each in stikk's words, **without running prikk**: into is the current branch; work
  queued for into; either side closed or the unpublished row; the same branch twice; a tag on either side; no
  truthful baseline; below the band. **The into-current guard is the one F3 measured** — word it as the RFC does.
- **Keys/Glossary**: `M` listed. Nothing in this handoff merges: the view says, in one line, that the merge is
  `Enter` **once Handoff B lands** — until then, show nothing about `Enter`.

## 4. Tests and the suite

- **Core**: the finder on a straight pair, a fast-forward shape (one side unchanged), a lineage with a `Merge` block
  above the shared one (declined), disjoint lineages (none), and a truncated read (none).
- **TUI**: `M` opens the view; each guard declines without dispatching; the plan renders inert and scrolls; captures at
  80×24 of a confluent plan, a conflicting plan and the into-current refusal.
- **Suite at the ceiling**: scenario 1's baseline equals the block prikk accepted, and the plan's outcome reads
  `Confluent`; scenario 3's reads `Conflict`; after scenario 1's merge, a further plan is **declined** for the `Merge`
  block; the into-current guard spawns no `merge-plan`. **At 0.28**, the below-band reason.

## 5. Text

`FR-080/081` delivery notes; `## Unreleased` `### Added`: the Merge plan view. **No changelog line for merging yet.**

## 6. Submit

The eight gates, the suite at 0.28.0 + 0.53.0. Package `.git-exclude/review-request/047-merge-a/review-request-v1.md`,
leading with §1. **Fill the base from `git rev-parse <commit>^` at submission time.**

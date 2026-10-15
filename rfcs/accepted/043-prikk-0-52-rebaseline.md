# RFC 043 — The prikk 0.52 re-baseline: a sealed patch now names its own path

**Status.** **Accepted by the project owner 2026-10-15.** **Proposed 2026-10-15** by the architect, the day prikk 0.52.0 was published (the owner also updated
their own machine to it). It delivers the item carried since 0.10.0: *"when prikk 0.52.0 publishes, validate it, and
the block-end label goes or is banded"* (RFC 035 A2). **No open question.**
Measured against real prikk **0.52.0** and **0.28.0** binaries. Evidence: `.git-exclude/reports/043-prikk-0-52-rebaseline/`.
**Tracks.** `ASM-2`, `NFR-R03`, `FR-030`; RFC 035 (A1 F7, A2, decision 3a), RFC 040, RFC 039 (the installers' prikk
pin), RFC 034 A1 (raise the ceiling first).
**Touches.** `stikk-prikk/src/version.rs`, Patch detail's label in `stikk-core`, one suite leg, `ASM-2`, the READMEs,
and the changelog.

## Summary

**prikk 0.52.0 fixed the defect stikk reported in letter 020**: in a block of several patches, `show <block-id>` now
resolves each patch's path **at that patch**, not at the block's end. Everything else stikk reads is unchanged. **So
this re-baseline raises the ceiling to 0.52 and removes Patch detail's *"as at the end of this block"* label where
prikk no longer needs it.**

## Findings

### F1 — the suite, with only the ceiling raised

On a scratch copy of `main` at `8908ae2`, with `VALIDATED_MAX_MINOR` set to 52, at prikk 0.28.0 and 0.52.0:
**50 passed, 1 failed** (`suite-0.28-0.52-ceiling-raised.txt`). The one failure is
`rfc035_a_sealed_blocks_patch_is_resolved_at_the_blocks_end_at_both_ends`, which pins 0.51's answer: *"left: "a.txt",
right: "z.txt""*. **prikk now returns the right name, and the test asserted the wrong one.** No other leg moved.

### F2 — the fix, measured

`measure035-at-0.52.txt` re-runs RFC 035's two cases:

| Patch 1 in a two-patch sealed block | 0.51 (`show <block-id>`) | **0.52** |
|---|---|---|
| edits `a.txt`, then patch 2 renames it to `z.txt` | `z.txt` | **`a.txt`** |
| edits `d.txt`, then patch 2 deletes it | `unresolved_node_id` | **`d.txt`** |

The queued and sealed forms now agree, as prikk's reply 022 promised. **No schema, field or key changed.**

### F3 — the rest of what stikk reads is unchanged

- **Verify** (`measure040-at-0.52.txt`, `measure040b-at-0.52.txt`): the same `verify-report-v1`, 17 stages, and exit 1
  with a complete report for both damage cases, with the same condition ids.
- **prikk 0.52.0's changelog** touches no JSON report stikk reads, apart from `show-report-v1`'s paths (F2). Its other
  changes are to the archive, to bundles, to `trust maintainer check`'s exit code (which stikk never calls; letter 021),
  and to `verify`'s prose, which stikk does not read.

## Decisions

1. **The validated ceiling is 52**, raised **first and alone** (RFC 034 A1), with the suite red at that commit on F1's
   one leg, reported rather than hidden.
2. **The block-end label is banded, not deleted**: shown at prikk **< 0.52**, where prikk resolves at the block's end,
   and **not shown at ≥ 0.52**, where the path is that patch's own. **One constant** for the band, beside the others.
   Below 0.52 the label stays true, and the label's code already exists, so banding costs one comparison.
3. **F1's test becomes version-aware**: at < 0.52 it asserts the label and prikk's block-end name; at ≥ 0.52 it asserts
   **no label** and the patch's own name (`a.txt`).
4. **`ASM-2`, both READMEs, the guide and the docs landing page** say *validated through 0.52.0*. **The installers need
   no change**: they derive prikk's pin from `version.rs` (RFC 039 B), so the next release's installer pins 0.52.0 by
   itself.
5. **The held letter to prikk is not triggered by this.** Its items remain held (`.git-exclude/upstream/prikk/hold-for-next-letter.md`);
   item 1's lapse is now complete, and item 3 (Verify's item outcomes) is unchanged at 0.52.

## Changelog

`### Changed`: validated through prikk 0.52.0. Patch detail's *"as at the end of this block"* label shows only below
prikk 0.52, which now names each patch's own path.

## Delivery

**One handoff, issued on acceptance** ([Handoff A](../handoffs/043-prikk-0-52-rebaseline/a-rebaseline-handoff-v1.md)), **in two commits**: the ceiling raise alone (suite red on F1's leg, reported), then the band,
the test and the docs. It re-runs F2 at 0.52 before building.

## What this RFC does not do

- **No other behaviour change.** prikk 0.52's other changes do not reach stikk.
- **No letter to prikk** (decision 5).

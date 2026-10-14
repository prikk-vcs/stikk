# Handoff A — RFC 037: every block's files, from `tree` (v1)

**Companion to:** [RFC 037](../../accepted/037-read-prikk-json-where-it-now-answers.md), **accepted 2026-10-14**, no
open question. **Read the RFC first.**
**Covers:** decisions 1, 2 and 5, plus **one carried item from RFC 036**: History's queued tier clips (§5).
**Independent of [Handoff B](./b-unpublished-current-branch-handoff-v1.md)**; either may land first.
**Not a release.**

> **The workflow changed (owner, 2026-10-14): no `prep/` branch.** Submit on the local gates and the local
> real-binary suite. After approval, push to `main`, then dispatch the full-matrix suite and the supply-chain gate
> **on `main`** and report them (§8).

---

## 1. Measure first, at 0.51

Re-run `.git-exclude/reports/037-read-prikk-json/measure037.sh` (it needs a prikk binary and a scratch directory)
and confirm F1: `tree --ref <block-id> --format json` gives each block's own state, with `target_block_id` equal to
the id asked for. **Add one case it lacks: a symlink in the tree**, and record the entry's `kind` and `encoding`.
`file` is the only kind measured so far. **If anything differs from F1, stop and report.**

## 2. The seam: a block's files, banded behind one method

A new `Prikk` method, or a change to `block_state`; the shape is yours. **The rules are fixed:**
- **At prikk ≥ 0.46** it runs **`tree --ref <block-id> --format json`**, with the block's own id, never a ref name.
  Put the version in **one** constant beside the other band constants, not a literal.
- **Below 0.46** it spawns nothing for a non-tip block and answers *unreported*, never an empty list. The tip keeps
  today's prose `checkout --patch-plan` read.
- **The reader is strict** (RFC 037 decision 1): `schema_version` must be `tree-listing-v1`, `target_block_id`
  must equal the id asked for, each entry carries `path`, `kind`, `mode` and `size`, and a path is not empty.
  **Anything else is stikk's environment error.**
- **An entry `kind` stikk has not measured is kept as prikk's word**, not refused (§1 decides whether `symlink` is
  measured).
- Total bytes is the sum of `size`. **Each file carries its mode.** The prose band has no mode, so the type says
  *not reported* there, never `0`.

## 3. Core and the view

- `stikk_core::block_detail` reads the files for **every** block at ≥ 0.46, and for the tip only below.
- **The heading stops saying "at tip"** wherever it is not the tip. It says *state at this block*, with the count
  and bytes as today.
- **Each file row shows its mode** as the octal Patch detail already uses (`100644`), beside the path. The path is an
  identifier (RFC 036).
- **The sentence *"prikk replays only to the ref tip — not shown for an older block"* is shown only below 0.46**,
  where it is true. At ≥ 0.46 it would be false (`C-T2b`).
- **A failed `tree` read costs Block detail its file list only** (decision 5, RFC 035 A3's rule): the block's
  fields and its patch list stay.

## 4. Tests

- Reader: a captured 0.51 report for each block of §1's lineage; negatives (wrong schema, a mismatched
  `target_block_id`, an entry missing `mode`, an empty path); an unknown `kind` kept.
- Core, with a stand-in: an older block lists its files at ≥ 0.46; *unreported* below; a failed read keeps the
  block and its patches.
- **A suite leg at 0.51:** two blocks; Block detail for the **older** one lists that block's files, not the tip's.
  At 0.28 it shows the below-0.46 sentence.
- **Captures at exactly 80 columns** (RFC 036 v2's note): an older block's files with modes, and the tip.

## 5. Carried from RFC 036: History's queued tier must not clip

`view/history.rs`: the tier is one `Line`, `"  queued   "` plus the composed sentence, never wrapped, so at 80
columns it is cut mid-word (*"…not this ref's hist"*), and **a long ref name in it would be cut too**. **Wrap it**:
the sentence as prose, the ref as an identifier piece (`text::compose`). Continuation rows go at the tier's indent,
and the header-row count that follows stays exact; it feeds the cursor arithmetic below it. **Test at 80 columns with
a 60-character ref name**: every character of the name appears once, in order.

## 6. The changelog

Under `## Unreleased`:
- **`### Changed`**: Block detail lists **any** block's files, with modes, at prikk ≥ 0.46, where it listed only
  the tip's.
- **`### Fixed`**: History's queued tier no longer clips.

## 7. The traps

1. **Ask with the block id, never the ref.** A ref moves; the block the user opened does not.
2. **"Not reported" is not 0**, for a mode or for the list.
3. **The tip has two sources by version.** At ≥ 0.46 it uses `tree` like every other block; the prose path serves
   below 0.46 only.
4. **Paste captures as the file holds them**, and grep the request for decoded escapes before submitting.

## 8. Gates, runs, submit (no branch)

**Before submitting:** the eight gates (1–5, 7, 8 on the MSRV; 6 on stable with a fresh `CARGO_TARGET_DIR`), and
the real-binary suite **locally** at prikk 0.28.0 and 0.51.0. Package to
`.git-exclude/review-request/037-a-every-blocks-files/review-request-v1.md`, leading with §1's measurement.

**After approval:** push to `main`. CI runs on push. Then dispatch on `main`:

```sh
gh workflow run real-binary.yml  --ref main -f full_platform_matrix=true
gh workflow run supply-chain.yml --ref main
```

Read every job and leg, and report the run ids and results. **A failure on macOS or Windows is fixed by a follow-up
commit on `main`**, reported the same way.

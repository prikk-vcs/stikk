# Handoff A — RFC 041: compare two refs, from the ref picker (v1)

**Companion to:** [RFC 041](../../accepted/041-compare-two-refs.md), **accepted 2026-10-15**, no open question.
**Read the RFC first.**
**Covers:** decisions 1–6. 0.11.0's last increment.
**Not a release.**

> **What this increment is.** The comparison a branch user asks first: *what is different between this branch and
> that one?* prikk answers it, and stikk's seam already sends it (`Comparison::Points`). **This is a picker change and
> a heading change; the seam does not change.** If you find you need to change it, stop and report.
>
> **No branch.** Local gates and suite before submitting; after approval, push to `main` and dispatch the suite and
> supply chain there.

---

## 1. Measure first, at 0.51

Before building, with `diff --from <a> --to <b> --format json` on real repositories:
- **two diverged branches, in both directions.** Record that the entries swap sides (`added` ↔ `deleted`) and that
  each side's `target_block_id` is the ref's tip;
- **a tag against a branch**;
- **a received ref (`remotes/…`) against a branch**, if the suite can make one cheaply (`bundle` export and import,
  or `sync`). **If not, say so**, and test that case with a stand-in;
- **the unpublished current branch** (a fresh repository's `heads/main`) **as `--from` and as `--to`**, against a
  published branch in a second repository state, or whatever prikk allows. **Record prikk's exact words and exit
  code.** RFC 041 decision 4 refuses this pair in stikk's own words; the measurement shows what prikk would say.

**If any report's shape differs from RFC 038's, stop and report.**

## 2. The picker

The ref picker is `Overlay::RefPicker` (`overlay.rs`).
- **`m`** marks the selected ref (`●` on its row, as in History). **`m` on the marked row unmarks it.** The title says
  a mark is set.
- **`d`** compares **the marked ref (`from`) with the selected one (`to`)** through `Comparison::Points` with the two
  ref names, and opens the Compare view. **`d` with no mark** says *mark a ref with `m` first*, in the picker, and
  does nothing else.
- **The unpublished current branch's row** (RFC 037 B): `m` and `d` on it say *this branch has no published history to
  compare yet*, and point at Changes' `d`. **Neither side may be that branch.**
- **The mark's lifetime:** **it lasts only while the picker is open, and until the next ref change**. RFC 041
  decision 1 says *"until the next ref change"*. Choose whether reopening the picker keeps it, **say which**, and
  test it. A stale pair is worse than an empty one.
- **A ref mark and History's block mark are different things.** Keep them apart in `App`, so `d` in History never
  uses a ref mark, and the reverse.

## 3. The Compare view's heading

RFC 038's view, unchanged except the heading for this source:
- **`<marked ref> (marked) → <selected ref> (selected)`**, both refs as identifiers (RFC 036), **and beside each, the
  short block id prikk resolved it to** (`target_block_id`), because a ref can move and the block cannot;
- **`r` re-runs against wherever the refs now point**, and the heading's block ids update. Never cache a pair across
  runs.
- **Two refs at the same block** give *no differences*, computed by prikk.

## 4. `requirements.md` and the changelog

- **`FR-033` delivered in full** at prikk ≥ 0.46: drop *"two-ref comparison not built"*, and name the four
  comparisons.
- **The guide**: the Compare section gains the picker's `m`/`d`. **`README.md` and `crates/stikk/README.md`**: remove
  *"Comparing two different refs … not built yet"*.
- **`## Unreleased`**: `### Added`, compare two refs from the ref picker. `### Breaking` **only if** `cargo public-api`
  shows a public change (an `Overlay::RefPicker` field, say).

## 5. Tests

- **Picker:** mark, unmark, `d` with and without a mark, the unpublished row refusing both keys, and the mark's
  lifetime as you chose it.
- **Separation:** a ref mark is never used by History's `d`, and a block mark never by the picker's.
- **Heading:** both sides' names and block ids; `r` after one ref moves shows the new block id.
- **Suite leg at 0.51:** two diverged branches compared through the picker, entries as prikk reported them; and
  §1's tag case. **At 0.28:** the below-0.46 sentence.
- **Captures at exactly 80 columns:** the picker with a mark set; the Compare heading for two refs; the unpublished
  row's sentence.

## 6. Submit (no branch)

The eight gates, the local suite, and the grep before committing. Package to
`.git-exclude/review-request/041-compare-two-refs/review-request-v1.md`, leading with §1's measurements.

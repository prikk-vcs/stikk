# Handoff A — RFC 043: the prikk 0.52 re-baseline (v1)

**Companion to:** [RFC 043](../../accepted/043-prikk-0-52-rebaseline.md), **accepted 2026-10-15**, no open question.
**Read the RFC first.**
**Covers:** decisions 1–4.
**Not a release.**

> **Small, and shaped by RFC 034 A1: the ceiling raise comes first and alone.** Then one band, one test and the docs.
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main` and dispatch the
> suite and supply chain there.

---

## 1. Measure first, at 0.52

Re-run `.git-exclude/reports/035-patch-detail/measure035-queued.sh` and `measure035-edit-then-delete.sh` at prikk
**0.52.0** and confirm RFC 043 F2: the sealed block names `a.txt` and `d.txt`, the same as the queued form. **If it
does not, stop and report.**

## 2. Commit 1: the ceiling, alone

`VALIDATED_MAX_MINOR` 51 → **52**. **Run the suite at 0.28.0 + 0.52.0 at this commit and keep its output.** It should
be **50 passed, 1 failed**: `rfc035_a_sealed_blocks_patch_is_resolved_at_the_blocks_end_at_both_ends`, asserting 0.51's
`z.txt` (RFC 043 F1). **If anything else fails, stop and report.**

## 3. Commit 2: the band, the test, the docs

- **The block-end label** (`stikk-core/src/patch.rs`, decision 3a's `label_paths`): shown only when prikk's minor is
  **below 0.52**. **One constant** for 0.52, beside the other band constants in `version.rs`, read by core. Core needs
  the minor; take it the way core already receives it for other bands. **Not a second literal.**
- **The suite leg** becomes version-aware (RFC 043 decision 3):
  - **< 0.52**: the label and prikk's block-end name, as today;
  - **≥ 0.52**: **no label**, and the patch's own name (`a.txt`).

  Add the delete case (`d.txt` at ≥ 0.52, *prikk names no path* below) if the leg does not already cover it.
- **Core tests** with a stand-in at minor 51 (labelled) and 52 (not).
- **Docs:** *validated through 0.52.0* in `ASM-2`, `README.md`, `crates/stikk/README.md`, `docs/src/index.md`, the guide,
  and `ROADMAP.md`'s versioning paragraph. **Grep for `0.51.0` and `validated through`, and read each hit.** The
  manual-install section's archive names are **stikk's** versions, not prikk's, and stay as they are.
- **The installers:** no change (they derive prikk's pin from `version.rs`). **Confirm** that the installers test now
  derives `0.52.0` and installs it on the Unix `--with-prikk` legs.
- **`## Unreleased`**, `### Changed`: validated through prikk 0.52.0. Patch detail's block-end label shows only below
  prikk 0.52, which now names each patch's own path.

## 4. Submit (no branch)

The eight gates, the local suite at 0.28.0 + 0.52.0, and the grep before committing. Package to
`.git-exclude/review-request/043-prikk-0-52-rebaseline/review-request-v1.md`, leading with §1's measurement and the
suite at commit 1 (red on the one leg), then at commit 2 (green).

**After approval:** push, then dispatch the full-matrix suite, the supply-chain gate **and `installers.yml`** on `main`,
and report every run.

# Handoff A — RFC 055: the prikk 0.55 re-baseline (v1)

**Companion to:** [RFC 055](../../done/055-prikk-0-55-rebaseline.md), **accepted 2026-10-29**. **Read the RFC
first**, F1–F3 and decisions 1–4. **Covers:** all of it. 0.17.0's increment 3.
**Best landed before RFC 053's [Handoff A](../053-get-files-back/a-get-files-back-handoff-v1.md) commit 1**, so 053's
and 054's legs run at 0.55 from their first run. If 053 is already under way, land this next; nothing breaks either
way.
**Not a release.**

> **What this increment is.** The validated ceiling to 55, and the range said as `0.55.0` wherever stikk states it.
> No leg changes.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main`, and dispatch the
> suite with `full_platform_matrix: true` and supply chain there; `installers.yml` runs on the push.

---

## 1. Measure first

Install the ceiling: `cargo install prikk --version 0.55.0 --locked --root <your pinned dir>`. Run the suite at 0.28.0
and 0.55.0 **with only the constant moved** (commit 1 alone). Expect **69 passed**, 40 at 0.28 and 29 at 0.55, as
`.git-exclude/reports/053-get-files-back/suite-0.28-0.55.0-ceiling-raised.txt` records. If any leg fails, stop and
report.

## 2. Commit 1: the constant

`crates/stikk-prikk/src/version.rs`: **`VALIDATED_MAX_MINOR = 55`**. Rewrite its doc comment's last paragraph in its
own shape (RFC 052's paragraph is the model): **55 means prikk 0.55.0**; what 0.55 changed that stikk meets —
`extract` refuses a destination inside the repository, which stikk does not call yet; a damaged bundle's refusal is
worded differently, which stikk shows verbatim and parses none of — and that the suite was green with only the
constant moved and no leg changed. Keep RFC 052's sentence on 0.54.0's withdrawal only if it still reads as history.

## 3. Commit 2: the range, where stikk states it

`0.54.1` → **`0.55.0`** in exactly these places (RFC 055 decision 2):

| File | Where |
|---|---|
| `README.md` | the requirements sentence (*"validated through"*) |
| `crates/stikk/README.md` | the requirements sentence, and the Status line (*"against prikk 0.54.1"*) |
| `docs/src/index.md` | the Status line (*"against prikk 0.54.1"*) and *"validated through"* |
| `docs/src/guide/getting-started.md` | *"validated through"* |
| `docs/src/guide/install.md` | the sample output: `prikk 0.55.0`, *(0.28 through 0.55)* |
| `docs/src/reference/requirements.md` | `ASM-2`: *validated through 0.55.0*, and its history gains **to 0.55.0 by RFC 055, 2026-10-29**, in the shape of the 0.54.1 entry (one clause on what 0.55 changed for stikk) |
| `ROADMAP.md` | *Releases and versioning* only |

**Not touched:** the roadmap's 0.16.0 section and `CHANGELOG.md`'s 0.16.0 section (history); `crates/stikk/README.md`'s
*"…and is on the AUR"* (carried to 0.17.0's release prep, not this handoff); the unit-test fixtures quoting 0.54's
*"unexpected end of record"* (fixtures of 0.54's wording, still true of 0.54).

`CHANGELOG.md` `## Unreleased`: `### Changed` — validated through prikk 0.55.0.

## 4. The traps

1. **Only the constant moves in commit 1**; the suite result is the evidence, so take it before commit 2.
2. **Sweep with grep**, `0\.54\.1` and `through 0\.54` over the repository, and account for every hit left.
3. **Paste captures as the file holds them; grep before committing.**

## 5. Submit (no branch)

The eight gates and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/055-prikk-0-55-rebaseline/review-request-v1.md`, leading with §1's suite result and §4's
sweep.

# Handoff B — RFC 037: the ref picker offers the current branch before its first seal (v1)

**Companion to:** [RFC 037](../../accepted/037-read-prikk-json-where-it-now-answers.md), **accepted 2026-10-14**, no
open question. **Read the RFC first, especially F4.**
**Covers:** decisions 3 and 4.
**Independent of [Handoff A](./a-every-blocks-files-handoff-v1.md)**; either may land first.
**Not a release.**

> **The workflow changed (owner, 2026-10-14): no `prep/` branch.** Submit on the local gates and the local
> real-binary suite. After approval, push to `main`, then dispatch the full-matrix suite and the supply-chain gate
> **on `main`** and report them (§6).

---

## 1. Measure first, at 0.51

Re-run `.git-exclude/reports/037-read-prikk-json/measure037.sh` and confirm F3:
- `unpublished_current_branch` is `"heads/main"` in a fresh repository, and still after a queued commit;
- it is `null` after the first seal;
- `branch create` publishes at once.

**Add one case:** a repository whose current branch is **not** `heads/main` and is unpublished, if prikk can reach
that state (for example, `init` with another default branch, if prikk offers one). Record what the field says.
**If anything differs from F3, stop and report.**

## 2. The reader: the branch beside the refs, never among them

**`refs()`'s list does not change**, and **RFC 032's *published* test (membership in that list) must not change**.
That is RFC 037 F4, and the reason prikk shaped the field as it did.

The branch-list reader also returns the unpublished current branch, in **three states, never keyed on absence**:
- **not reported:** prikk < 0.47, the field's floor (prikk's changelog: 0.47.0). Decide this **by the version**,
  with **one** constant beside the other band constants. The prose band (< 0.39) is also *not reported*.
- **reported, none:** `null`;
- **reported, this branch:** a string, validated through `RefName::parse` like every other ref name (`INV-9`).

**At ≥ 0.47 the field is required**: missing, or neither `null` nor a string, is stikk's environment error.

The shape of the API is yours: a struct around the list, or a new return type. **It is a Breaking row** in
`## Unreleased`, since `Prikk::refs` and `list_refs` change.

## 3. The picker

- **When the branch is reported, it gets its own row**, above or below the published refs (your choice; say which).
  It is marked in stikk's words as **the branch you are on, not yet published**. The name is an **identifier**
  (RFC 036), and the mark is stikk's prose.
- **Selecting it focuses that ref like any other.** History then shows its queued tier and no sealed blocks, and
  Changes works as today, through RFC 034's retry on prikk 0.45–0.46 and directly from 0.47.
- **When it is not reported, the picker is exactly as today.** Nothing says "none".
- **If the reported branch also appears in `branches[]`**, which prikk should never do, the published entry wins and
  the extra row is not drawn. Test it with a stand-in.

## 4. Tests

- Reader: captured 0.51 reports for fresh, queued and sealed; 0.46's report with the field absent → *not reported*;
  ≥ 0.47 with the field missing → environment error; a non-string value → environment error.
- **RFC 032's test stays unchanged**: a test asserting that Changes on a fresh repository still says *no published
  history* with the field present.
- Picker, with a stand-in: the row appears, is selectable, focuses the ref; it is absent when not reported; the
  duplicate case.
- **A suite leg at 0.51:** a fresh repository's picker offers `heads/main`, marked. Selecting it opens History with
  no sealed blocks. **At 0.28, the picker has no such row.**
- **Captures at exactly 80 columns**: the picker on a fresh repository, and after the first seal.

## 5. The changelog

Under `## Unreleased`: **`### Added`**: the ref picker offers the branch you are on before its first seal (prikk
≥ 0.47). **`### Breaking`**: the reader's return shape.

## 6. Gates, runs, submit (no branch)

**Before submitting:** the eight gates, and the real-binary suite **locally** at prikk 0.28.0 and 0.51.0. Package to
`.git-exclude/review-request/037-b-unpublished-current-branch/review-request-v1.md`, leading with §1's measurement.
**Paste captures as the file holds them**, and grep for decoded escapes before submitting.

**After approval:** push to `main`, let CI run, then dispatch the full-matrix suite and the supply-chain gate on
`main` (`gh workflow run … --ref main`), and report every run id and result. A macOS- or Windows-only failure is fixed
by a follow-up commit on `main`.

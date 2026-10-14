# Handoff A — RFC 040: Verify (v1)

**Companion to:** [RFC 040](../../accepted/040-verify.md), **accepted 2026-10-15**, no open question. **Read the RFC
first**, F2 especially.
**Covers:** decisions 1–5. 0.11.0's increment 4.
**Not a release.**

> **What this increment is.** stikk shows prikk's verdict on whether the repository is sound. **The one thing it must
> never do is say *prikk refused* when prikk is reporting damage**: a damaged repository makes `verify` exit 1 *with*
> a complete report (F2). Everything else is a familiar shape: a strict reader, prikk's words verbatim, a raw view, and
> a sentence where prikk's JSON is silent.
>
> **No branch.** Local gates and suite before submitting; after approval, push to `main` and dispatch the suite and
> supply chain there.

---

## 1. Measure first

Re-run `.git-exclude/reports/040-verify/measure040.sh` and `measure040b.sh` at **0.51** (they need a prikk binary
and a scratch directory; `measure040b.sh` takes the sound repository `measure040.sh` leaves). Confirm F1 and F2.
**Then run `measure040.sh` at 0.39**, the band floor, and record the stage list and keys there. **If the shape differs
in anything the reader relies on (§2), stop and report.**

## 2. The seam

**`Prikk::verify(repo)`**, banded at `JSON_REPORTS_FROM_MINOR` like the other JSON views. **Below it, spawn nothing
and answer *unreported*.**

**The exit-code rule (decision 1).** It is the one place the seam reads a non-zero exit as success, so build it
narrowly:
- **exit 0, or exit 1, with stdout that parses as `verify-report-v1`** → a report, whatever stderr says (stderr
  repeats the condition);
- **exit 1 with stdout that does not parse** → prikk's refusal, verbatim, classified as every other refusal is;
- **exit 2** → stikk's bug, as today;
- **any other code** → as today.
The existing `run` helper may hide the status from you. **Add the narrowest helper that returns stdout and status
together, for this method only**, and say in its doc comment why no other method may use it. **A test for each row of
the list above.**

**The reader (decision 2):**
- the schema;
- `verdict.ok` a boolean **equal to** `failed_conditions` being empty, with a disagreement as stikk's environment
  error;
- each condition `{id, message}`, both strings;
- each stage a `stage` string and a `status`. `failed` carries `message`, `not_evaluated` carries `blocked_by`,
  `halted` carries `after`; **an unmeasured status is kept as prikk's word**, with its fields in the raw view;
- `active_sessions` optional, `{count, verified_count}` integers when present;
- **no stage count and no stage name assumed.**

## 3. Core and the view

- **Opened on demand with `V`** (capital, like `Q` and `R` for repository-wide views; lowercase `v` is the raw toggle),
  from anywhere the shell is, and from the palette. **It never runs on its own.** It runs on the worker, and shows that
  it is running.
- **The verdict first, in stikk's words**: *prikk reports this repository sound*, or *prikk reports N problem(s)*.
- **Each failed condition**: prikk's `id` and `message`, verbatim, under the prose rule (RFC 036).
- **stikk's standing sentence, always, beneath the conditions** (decision 3): *prikk's JSON report does not list which
  objects or refs failed, or count unverifiable signatures. Run `prikk verify` in a terminal for those.* This is what
  makes *"see item outcomes above"* honest. **Show it on a sound report too**: it is true there as well, and a sentence
  that appears only on failure reads as part of the failure.
- **The stages**, in prikk's order, each with its status word and any `message`, `blocked_by` or `after`. Stage names
  are prikk's words and render as identifiers.
- **Active sessions**: *prikk verified N of M active sessions*, when `active_sessions` is present.
- **`v`**: the raw JSON, through the shared `bounded_rows`. **`r`** re-runs it. **The heading says the verdict is from
  this run**, never cached across runs.
- **Below 0.39**: stikk's sentence that it reads prikk's verify report from 0.39 on.

**Verify changes nothing** (decision 4): no repair, no lock clearing, no automatic or background run.

## 4. `requirements.md`

**`FR-100`, partly delivered**, with RFC 040 F4's table: which clauses are met, and which wait on prikk's JSON. Correct
*"all 14 stages"* to *every stage prikk reports*.

## 5. The changelog

Under `## Unreleased`, `### Added`: Verify (`V`), prikk's verdict, failing conditions and stages (prikk ≥ 0.39), and
the sentence on what prikk's JSON leaves out. **`### Breaking`**: the new `Prikk` method, with no default, and any
`match`-breaking variant you add to a public enum. Check visibility with `cargo public-api`, as the 0.10.0 prep did.

## 6. Tests

- **Seam:** the four exit-code rows, from captured 0.51 reports (sound, `item-failure`, `commit-witness-integrity`);
  the reader's negatives (schema, `ok` disagreeing with its conditions, a condition missing `message`, a stage missing
  `status`); an unmeasured status kept.
- **Core and view:** the verdict sentence for zero and for several conditions; the standing sentence on both a sound
  and a damaged report; stages with each status; the raw view bounded.
- **A suite leg at 0.51:** a real repository, sound, then with a byte flipped in a blob container (as
  `measure040b.sh` does). **Verify reports the problem, never a refusal.** At 0.28: the below-floor sentence.
- **Captures at exactly 80 columns:** sound; damaged (the `item-failure` case, with the standing sentence directly
  under it); the raw view.

## 7. The traps

1. **exit 1 is not a refusal here, and only here.**
2. **`ok` and the conditions must agree**; never pick one when they don't.
3. **No stage list is hard-coded.** 0.51 has 17; `FR-100` once said 14.
4. **Never parse the prose**, not even to fill F3's gap.
5. **Paste captures as the file holds them; grep before committing.**

## 8. Submit (no branch)

The eight gates and the local suite. Package to `.git-exclude/review-request/040-verify/review-request-v1.md`, leading
with §1's measurement (0.51 and 0.39), then the exit-code table's tests, then the captures.

# RFC 040 — Verify: whether the repository is sound, in prikk's own verdict

**Status.** **Done 2026-10-15**: Handoff A delivered on `main` (`763f10b`, `95ffea5`); released in **0.11.0**. **Accepted by the project owner 2026-10-15.** **Proposed 2026-10-14** by the architect, as 0.11.0's increment 4 (`.git-exclude/release/0-11-0-scope.md`,
authorized). **No open question.**
Measured against a real prikk **0.51.0**: a sound repository, one with a queued commit, one with a byte flipped inside
an object container, and one with a queued commit's WAL record torn. Evidence: `.git-exclude/reports/040-verify/`
(`measure040.sh`, `measure040b.sh`, and their `-0.51.txt` outputs), and prikk's own emitter in the clone
(`prikk-cli/src/output/verification.rs`).
**Tracks.** `FR-100` (partly; see F4), `C-T2b`, `C-T2c′`, `ER-02`, `T-T4`; RFC 036 (rendering), RFC 035 (the raw view).
**Touches.** `stikk-prikk` (a `verify` seam method and its reader), `stikk-core` (a verify view-model), `stikk-tui`
(the Verify view and its entry), `stikk-real-binary`, `requirements.md`, and the changelog.

## Summary

**stikk has never run `prikk verify`.** A reader can browse every block and patch and still not know whether the
repository underneath is sound. prikk answers that in JSON (`verify-report-v1`): a **verdict**, **every currently
failing condition**, and **one entry per verification stage**. Verify shows that report **on demand**, in prikk's words,
and is honest about the one thing the JSON leaves out: **which objects or refs failed**.

## Findings

### F1 — the report, measured at 0.51

`verify --format json` → `{schema_version: "verify-report-v1", verdict: {ok, failed_conditions: [{id, message}]},
active_sessions: {count, verified_count}, stages: [{stage, status, …}]}`.
- **17 stages** at 0.51, in prikk's declared order (`objects`, `refs`, … `object-connectivity`,
  `appended-file-tails`). **`FR-100` says 14**, written when there were 14. Each stage's `status` is `evaluated`,
  `failed` (+`message`), `not_evaluated` (+`blocked_by`), or `halted` (+`after`) (prikk's emitter).
- **`verdict.ok` is `failed_conditions` being empty**, and the conditions come from *"the same declaration the exit
  code reads, so they cannot disagree"* (prikk's source).
- `active_sessions.verified_count` is a fixed `1`: only the default session is covered (*"see `prikk doctor` for the
  rest"*, in the prose).
- **The JSON has existed since prikk 0.25, and `active_sessions` since 0.27** (prikk's changelog), both below stikk's
  floor.

### F2 — a damaged repository exits 1 and still prints a complete report

| State | exit | `verdict` | stages not `evaluated` |
|---|---|---|---|
| sound (sealed, or with a commit queued) | 0 | `ok: true` | none |
| one byte flipped in `containers/blob/a.container` | **1** | `ok: false`, `item-failure`: *"…found at least one failed object, block, or ref; see item outcomes above"* | **none** |
| a queued commit's WAL record torn | **1** | `ok: false`, `commit-witness-integrity`: *"a queued commit you were told had succeeded disagrees with the WAL…"* | **none** |

**Two consequences.**
- **stikk's seam treats every non-zero exit as a refusal.** Unchanged, Verify would say *prikk refused* exactly when
  the repository is damaged: the most wrong thing it could say (`T-T4`).
- **The verdict carries the signal; the stages mostly confirm coverage.** In both damage cases every stage still read
  `evaluated`.

### F3 — the JSON points at information it does not carry

The `item-failure` message says *"see item outcomes above"*. **The item outcomes exist only in the prose report**:
*"object items: 9 scanned, 2 failed"*, then each failed object with its container, offset and reason. prikk's emitter
says why: *"Counts and item-level findings are deliberately out of v1 scope."* A JSON consumer receives a pointer to
nothing. The prose also carries counts the JSON omits, including *"unverifiable author signatures: N"*, the
three-valued signature outcome `FR-100` asks stikk to explain.

### F4 — what that means for `FR-100`

`FR-100` asks for every stage with its status (**met**), per-item findings grouped by object and ref (**not
available**, F3), warnings distinguished from errors (**not available**: the JSON has no warnings), three-valued
author-signature outcomes (**not available**, F3), and a machine-readable export through `verify --format json`
(**met**, by the raw view). **stikk does not parse prose for this.** It never has for a new surface, and the prose is
*"current CLI vocabulary, not a stable machine-readable schema"* (prikk's integrity reference).

## Decisions

1. **A `verify` seam method** runs `verify --format json` at prikk ≥ 0.39 (`JSON_REPORTS_FROM_MINOR`, the one
   constant). Below that it spawns nothing, and the view gives stikk's reason (`C-T4d`). **Exit 0 or exit 1 with a
   valid `verify-report-v1` on stdout is a report, not a refusal**, whatever stderr says (stderr repeats the
   condition). **Any other outcome is classified as today:** exit 1 without a parseable report is prikk's refusal,
   verbatim, and exit 2 is stikk's bug. **This is the one place the seam reads a non-zero exit as success, and it
   says so in a comment and a test.**

2. **The reader is strict where stikk relies on the shape:** the schema; `verdict.ok` a boolean that **equals
   `failed_conditions` being empty** (prikk guarantees it, so disagreement is stikk's environment error, never a
   guess); each condition an `{id, message}` pair of strings; each stage a `stage` string and a `status`. **An
   unmeasured `status` is kept as prikk's word.** `active_sessions` is optional. **No stage count or stage name is
   assumed**: stikk shows what prikk lists, in prikk's order.

3. **The view**, opened **on demand** (a key from Orientation, and the palette). It **never runs on its own**: it
   scans the whole repository. It runs on the worker and shows that it is running.
   - **The verdict first, in stikk's words**: *prikk reports this repository sound*, or *prikk reports N
     problem(s)*. Then **each failed condition** as prikk's id and message, verbatim (prose rule, RFC 036).
   - **Then stikk's standing sentence, always shown and always true:** *prikk's JSON report does not list which
     objects or refs failed, or count unverifiable signatures. Run `prikk verify` in a terminal for those.* This is
     what makes F3's *"see item outcomes above"* honest: stikk names where "above" actually is.
   - **Then the stages**, each with its status and, where prikk gives one, its message, `blocked_by` or `after`.
   - **Active sessions**: *prikk verified N of M active sessions*, from `active_sessions`.
   - **`v`**: prikk's raw JSON, bounded, as in Patch detail and Compare.
   - **`r`** re-runs it.

4. **Verify changes nothing.** No repair, no lock clearing, no automatic run, and no cached verdict. A verdict is
   about the moment it was taken, and the view says when that was, in stikk's own session terms ("just now",
   refreshed by `r`). stikk has no clock policy beyond its session.

5. **`requirements.md`:** `FR-100` is **partly delivered**, with F4's table. The stage count in `FR-100`'s text is
   corrected to *"every stage prikk reports"*.

6. **prikk is told about F3**, as a measured, defined point, **with the next necessary letter** (the held list), not
   on its own: stikk's design does not wait on it. The question to prikk: *would item outcomes (or at least their
   counts and the unverifiable-signature count) fit `verify-report-v1` additively, as `active_sessions` did?*

## Changelog

`### Added`: Verify, prikk's verdict on whether the repository is sound, with its failing conditions and stages
(prikk ≥ 0.39), and where to find what prikk's JSON leaves out.

## Delivery

**One handoff, issued on acceptance:** [Handoff A](../handoffs/040-verify/a-verify-handoff-v1.md). It re-measures F1 and F2 at 0.51 first, including the exit-1-with-report case, and at
0.39 (the floor) for the shape. **It stops if the shape differs.**

## What this RFC does not do

- **No doctor view and no lock inspector** (`FR-101/102`). `doctor` has no JSON.
- **No per-item findings**, unless prikk adds them to the JSON.
- **No verify of any session but the default**, which is prikk's limit too.

## Delivered

**One handoff, two commits, green on `main` at `95ffea5`**: CI, Docs, the real-binary suite on the full matrix (49 cases
at prikk 0.28.0 and 0.51.0) and supply chain.

- **Measured at both ends:** 0.51 reports **17** stages; **0.39, the floor, reports 14**, the three later stages absent
  rather than renamed. Both are pinned as fixtures. Nothing the reader relies on differs.
- **The exit-code rule**, in one `match` over a helper used by `verify` alone. Exit 0, or exit 1 with a readable
  report, is a report. **Exit 1 with a `verify-report-v1` stikk cannot read is stikk's environment error** (review v1
  §2; without it, a damaged repository would again have read as a refusal). Exit 1 with no report is prikk's refusal.
  Exit 2 is stikk's bug. Any other code is a refusal even carrying a report. One test per row.
- **The reader** checks `ok` against its conditions in both directions, keeps an unmeasured status as prikk's word, and
  assumes no stage list.
- **The view (`V`, Tier One)**: the verdict in stikk's words, conditions verbatim, **the standing sentence on what
  prikk's JSON omits, shown on a sound report too**, the stages, active sessions, `v` bounded, and `r` re-runs. **A
  damaged report never says "refused"**, and a report stikk did not check never says "sound", both asserted.
- **A suite leg flips a byte in a real blob container**: prikk exits 1, and stikk returns a verdict.
- **`view::wrap_hanging`**, shared, after this view reproduced Handoff C's fault and its own test caught it.

## Carried forward

- **Per-item findings, warnings and the unverifiable-signature count** (`FR-100`'s other clauses) wait on prikk's JSON.
  The question is held for the next necessary letter (`.git-exclude/upstream/prikk/hold-for-next-letter.md`, item 3).
- **`block.rs`'s `said` duplicates `wrap_hanging`**: fold it in when Block detail is next opened.

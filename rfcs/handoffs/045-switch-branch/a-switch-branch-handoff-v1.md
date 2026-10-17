# Handoff A — RFC 045: switch branch (v1)

**Companion to:** [RFC 045](../../accepted/045-switch-branch.md), **accepted 2026-10-17**, no open question; decision 3
(**Tier Three**, not a typed name) accepted with it. **Read the RFC first**, F2, F4 and F5 especially.
**Covers:** decisions 1–9 (10 is the architect's). 0.13.0's increment 3.
**Not a release.**

> **What this increment is.** stikk's first write that changes **files on disk**. prikk makes the switch itself safe
> (F2, F3); stikk's job is that **the user sees exactly which files change before saying yes**, and that stikk never
> offers the one case prikk allows and gets wrong (F5).
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main` and dispatch the
> suite, supply chain and installers there.

---

## 1. Measure first, at 0.52

Re-run `.git-exclude/reports/045-switch-branch/measure045.sh` and `measure045-queued-target.sh` against the pinned
0.52.0 and confirm F1–F5, **F4 and F5 especially**. **If any row differs, stop and report.** (Both scripts take the
binary and a scratch directory; seeds stay in 0600 files and are never printed.)

## 2. Commit 1: the category, the gate, the seam

**a. `RequestCategory::Materialization`** (`stikk-model/src/category.rs`): *writes the worktree from history*.
`mutates() == true`, `tier() == Tier::Three`, `name() == "materialization"`, in `ALL` (now ten). Fix the module and
type docs that say *nine*.

**b. The gate must be keyed on the category, not the tier.** This is the one thing the RFC did not spell out, and it
is load-bearing: `stikk-core/src/confirm.rs:346`'s `capability_gate(operation, tier, readiness)` maps **Tier Three →
MAINTAINER readiness**. Left as it is, a switch would demand a maintainer key it does not use (F1) — exactly the false
requirement decision 2 rules out. Make the requirement a property of the **category** beside its tier:

| Category | Requirement |
|---|---|
| `QueueMutation` | AUTHOR |
| `Publication`, `Exchange` | MAINTAINER |
| `Trust`, `Recovery`, **`Materialization`** | read-write only (`may_operate`) |

Every existing category must keep **exactly** its current requirement — prove it with a test over `ALL`. `Recovery`/
`Trust` already pass with no key today (they are `ThreeTyped`, which falls through), so only the *keying* changes. Change
`capability_gate`'s signature (and `confirm`'s use of it) as you see fit; it is public, so it goes in `### Breaking`.

**c. The seam**: `switch_branch(repo, name) -> Result<SwitchResult>` at `Materialization`:
- runs `branch switch <name>`, one argument, **no `--format json`** (there is none, F1);
- **banded at `DIFF_FROM_MINOR`** (decision 4): below it, spawn nothing — not even a readiness read — and answer
  *unavailable*, as RFC 044's `RefWrite::Unavailable`;
- checks `may_operate` **before prikk runs**;
- exit 0 keeps **stdout's lines and stderr's lines, both verbatim**, separately (prikk names a snapshot fallback on
  stderr at exit 0, `branch.rs:353`). **Never parsed for state**;
- every other exit classifies as today.

**Tests:** the argument vector (one argument, a space intact); a success with stderr lines kept apart; each F2 refusal
verbatim; nothing spawned below the band; no key required (a recording fake that never sees a readiness refusal).

## 3. Commit 2: preview, card, key, result

**a. The preview** (`stikk-core`, a `switch` module modelled on `refs.rs`; reuse its shape, not its `Publication`
helper). Inside `confirm::preview`, read **orientation**, then — only if offered — **`diff` with
`Comparison::Points { from: <current branch>, to: <target> }`**, and the target's tip with `history(…, 1)` as RFC 044
does. **Blocked, in stikk's words, without the diff read** (decision 6):
- the target is the current branch → *"already the current branch"*;
- the target is closed → from the list's flag; the frontend may decline it before previewing, as `x` does;
- **work queued for the target**: `orientation.queued_patches > 0` and `queued_target` is the target → the F5 reason.
  Queued work for **another** ref is **not** stikk's to block: prikk refuses it, verbatim (F2);
- the current branch is `CurrentBranch::{NotReported, Unresolved}` or unpublished, or the diff read fails or answers
  `Unreported` → *no file plan*, naming `prikk branch switch` as the way out.

**b. Classifying the plan.** `DiffEntry::status` is prikk's string. `added`/`modified` → **written**; `deleted` →
**deleted**; a rename (`from_path` present) → written at `path` **and** deleted at `from_path`. **Any status you have
not measured, or any `unsupported_paths`, means the plan is incomplete: blocked, with that reason.** A plan the user
confirms must be the whole plan (`FR-053`).

**c. The card** (decision 5). `target_ids`: the current branch, the target, the target's short block id (through
`identifier`). Counts: written, deleted. **The paths**, each marked written or deleted, through `inert`, as many as fit,
then *"and N more"* — the card's existing height rules decide how many; **never a silent cut**. Consequence: **no name
in it** (RFC 044 C1). **No "Consumes: MAINTAINER" row**: say no signing key is used, in whatever shape keeps
`ConfirmationSummary` honest (an `Option`, a variant — your call; it is Breaking either way). An empty plan says the
switch moves the pointer and writes nothing. One line distinguishes this from `Enter` in the picker (focus only).

**d. The key: `s` in the ref picker** only (decision 7). Declined, without dispatching: a tag row, the current
branch's row, a closed row, the unpublished row (RFC 037 B), and below the band (stikk's reason, `C-T4d`). Glossary.

**e. After success** (decision 8): the result card quotes stdout under prikk's bar and stderr beneath it, labelled as
prikk's; then orientation, the ref lists and Changes are re-read; **focus moves to the switched-to branch** (an explicit
user action, not a read — say so in `FR-055`'s delivery note).

**f. RFC 044's refusal** (decision 9): `CLOSING_THE_CURRENT_BRANCH` now says *"Switch to another branch first (`s` in
the ref picker)."*; update its tests and capture.

## 4. Tests and the suite

- **Core:** each blocked case with its reason, and **the diff not read** for the first three; the plan's
  classification, including a rename and an unknown status; the gate table over `ALL`; Tier Three; no key needed.
- **TUI:** `s` declines each row kind without dispatching; a ready preview arms Tier Three; the card's overflow row;
  success re-reads and moves focus; a refusal hands back to the picker.
- **Suite legs at 0.52:** **the preview's written and deleted paths equal what the switch did**, with the worktree's
  file list checked after; switching back restores the original bytes (compare file contents); a dirty worktree
  refused verbatim, nothing written; an untracked file left alone; **the target-queued case blocked with no `branch
  switch` spawned** (prove it: the current branch is unchanged by prikk's own `status`). **At 0.28**, unavailable.
- **Captures at exactly 80 columns:** the preview with a few paths, the preview with overflow, an empty plan, the
  target-queued refusal, the result card, and RFC 044's reworded refusal.

## 5. `requirements.md` and the changelog

`FR-053`: delivered for branch switch at prikk ≥ 0.46; checkout of a block or path is not. `FR-055`: the focus note
from §3e. `## Unreleased`: `### Added` (switch branch), `### Changed` (RFC 044's refusal wording), `### Breaking`
(the new trait method, `RequestCategory::ALL`, the gate's signature, and whatever `cargo public-api` shows).

## 6. The traps

1. **The gate is per category.** A switch that asks for a maintainer key is the bug this handoff exists to prevent.
2. **The plan is the whole plan.** An unmeasured status or an unsupported path blocks; it is never dropped.
3. **Never parse prikk's counts for state.** They go on the result card; the truth is re-read.
4. **F5 is stikk's to block; F2's queued-elsewhere is prikk's.** Do not widen the guard.
5. **One argument per name.** A space is legal to prikk.
6. **Paste captures as the file holds them; grep before committing**, including for repeated sections.

## 7. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.52.0. Package to
`.git-exclude/review-request/045-switch-branch/review-request-v1.md`, leading with §1's measurement.

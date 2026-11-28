# Handoff A — RFC 074: the prikk 0.59 re-baseline (v1)

**Companion to:** [RFC 074](../../accepted/074-prikk-0-59-rebaseline.md).
**Issued** 2026-11-28. **0.23.0's conditional increment.** It's worked beside RFC 070's handoffs and RFC 072, and the
0.23.0 cut doesn't wait for it.

> **What this is.** RFC 074's seven decisions, in its order: the raise first, so the suite reports what it costs.
>
> **Sanctioned:**
> - `crates/stikk-prikk`, `crates/stikk-core`, `crates/stikk-app` (only for decision 4's offer), `crates/stikk-tui`
>   (only to draw decision 4's offer), `crates/stikk-real-binary`;
> - `CHANGELOG.md`;
> - the suite's binaries under `~/.local/share/stikk-test-prikk/`;
> - a new `.git-exclude/reports/074-prikk-0-59-rebaseline/`.
>
> **Not sanctioned:** the floor (0.28), the release workflows, and the GUI crates. RFC 070 is separate work.

## 0. Read first

- **RFC 074, whole,** and **RFC 063,** the 0.58 re-baseline whose order this follows.
- **prikk's letter 035** (`.git-exclude/upstream/prikk/receive/035-…`) and its 0.59.0 CHANGELOG
  (`git show 0.59.0:CHANGELOG.md` in `.git-exclude/projects/prikk`). The `### Output changes` section is the list of
  every changed sentence.
- **stikk's draft 025 §2 and §4,** which decisions 5 and 6 keep.
- **The suite's binaries note,** `reference_stikk_suite_prikk_bins` in the project memory, and RFC 063 Handoff A's
  install step. Run the suite with `--test-threads=1`.

## 1. The raise, and `ceiling-0.59` (decision 1)

1. **Install `ceiling-0.59`** from prikk's published 0.59.0 release, the Linux archive, checked against the
   release's own `.sha256`. Record the archive's checksum and `prikk --version` in the report. Don't build prikk
   yourself.
2. **First commit:** `VALIDATED_MAX_MINOR` from 58 to 59, and the doc comment's history line.
3. **Run the whole real-binary suite** at `ceiling-0.59` and at `ceiling-0.58`, and record both outputs verbatim.
   **Every failure at 0.59 is a finding.** Each one is fixed by a later decision here, or reported with its cause.
   None is skipped, and the ceiling isn't moved back.

## 2. Every matched string (decision 2)

**A script, `matched-strings.sh`,** that:
- extracts every prikk string stikk matches: `classify.rs`'s patterns, `glossary.rs`'s `*_CODE` constants
  (multi-line ones included), and any other `contains`/`starts_with`/`strip_prefix` on prikk's prose you find. Say
  where you looked;
- counts each one's sites in prikk's non-test source at `0.58.0` and `0.59.0`, case-insensitive where stikk lowercases;
- for each, says whether the **state** stikk keys on still prints it at 0.59. From a suite run where one reaches the
  state; from source where none does. **Say which.**

The architect's first count found none disappearing (RFC 074 F2). That's necessary, not sufficient. **A pattern whose
state no longer prints it is fixed or retired, with its reason.**

## 3. Glosses that defer (decision 3)

For every gloss keyed on a prikk sentence, compare it with what 0.59 prints in that state.
- **Where 0.59's sentence names the command that resolves the state, remove stikk's gloss at 0.59 and above.** Keep
  it below 0.59.
- **The generic incomplete-publication gloss** (`INCOMPLETE_PUBLICATION_CODE`, RFC 063 F9) is the known case. Check
  each 0.59 state that prints `repository mutation is blocked by incomplete ref publication;`.
- **Tests (unit):** each changed gloss is absent at 0.59 and present at 0.58, for the same sentence.

## 4. The switch, merge and adopt refusals (decision 4)

- **The banding:** stikk's own guard (`stikk-core/src/switch.rs`, `WORK_IS_QUEUED_FOR_THIS_BRANCH`) fires only below
  0.59. **At 0.59, prikk refuses, and stikk shows prikk's refusal verbatim.**
- **The offer:**
  - when prikk's refusal names `prikk seal --allow-no-audit --ref <branch>`, the refusal card offers stikk's Seal for
    that branch, with its usual consent, and then the switch again;
  - the branch comes from prikk's sentence, parsed and validated as a ref name;
  - an unparsable sentence offers nothing and shows prikk's words alone;
  - the same applies to `merge --into`, `--adopt` imports and `branch create` where prikk names a seal.
- **This touches `stikk-app`,** which is shared, and the terminal draws the offer. Keep the change to the refusal
  card's actions. **If it needs more, stop and ask.**
- **Tests:**
  - **(unit)** the guard fires at 0.58 and not at 0.59, for the same orientation;
  - **(unit)** the offer appears for a sentence naming a seal, and not for one that doesn't, or whose ref name is
    invalid. That's the run that fails it;
  - **(suite, 0.59)** the switch, merge and adopt refusals are reached and classified, and the offer's Seal then
    lets the switch through, with the file on disk unchanged;
  - **(suite, 0.58)** stikk's own guard refuses, and the file is kept.

## 5. The refusal-first readers (decision 5)

- **Every JSON reader asks first whether stdout is a `refusal-v1`.** If so, it returns a refusal with prikk's
  `message`, and its id for callers that route by it. Reuse `refusal_message` and `refusal_id_and_message`; don't
  copy them.
- **An unknown schema that isn't a refusal is still an environment error.**
- **Tests (unit):**
  - every reader, with a `refusal-v1` on stdout, gives the refusal and its message;
  - with an unknown schema, it errors;
  - removing the check from one reader makes its test fail. **Show that run.**

## 6. Waiting (decision 6)

- **stikk sets no `PRIKK_LOCK_WAIT_SECONDS`.** Show it: grep, and the environment stikk passes.
- **First, measure what stikk does with a successful command's stderr** (RFC 074 F5), and record it.
  - A `waiting …` line on a command that then succeeds must not read as a refusal or a warning.
  - The refusal at the bound, with its `(waited N s: …)` suffix, is classified as today's lock refusal.
- **Test (suite, 0.59),** with the wait made short through the suite's own environment (for example
  `PRIKK_LOCK_WAIT_SECONDS=2`), for a writer beside a held lock:
  - **success:** the holder exits within the bound;
  - **refusal:** it doesn't.
- **No sleeping loops:** one timed holder per case.

## 7. The fixture builder (decision 7)

If §1's suite run shows the builder affected by 0.59's `init`, `setup` or `key generate` changes, fix it and say
what changed. If it isn't affected, say so with the run that shows it.

## 8. Gates, and the public API

- **All gates,** the rustdoc gate included (never exempted), and `cargo test -p stikk-real-binary --test rfc_index`.
- **The suite at 0.59 and 0.58, green.**
- **`cargo public-api diff`** against 0.22.1 for every published crate, its output verbatim. Any Breaking row is
  named for the 0.23.0 release proposal.
- **`CHANGELOG.md`:** RFC 074's *Changelog* section, in the house style, under the unreleased heading.

## 9. Submit

`.git-exclude/review-request/074-prikk-0-59-rebaseline-a/review-request-v1.md`:
- §1's suite outputs and checksum;
- §2's table;
- each decision's tests with their failing runs;
- the stderr measurement;
- the public-API diff.

**Commits pushed and CI green before you submit.** If your session refuses a push, say so, and don't commit on top
of it.

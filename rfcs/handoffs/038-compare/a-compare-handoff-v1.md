# Handoff A — RFC 038: Compare (v1)

**Companion to:** [RFC 038](../../accepted/038-compare.md), **accepted 2026-10-14**, no open question. **Read the RFC
first**; this handoff does not repeat F1's measured table.
**Covers:** all seven decisions. 0.10.0's increment 3, the last one in scope.
**Not a release.**

> **What this increment is.** `FR-033`, the last named view stikk has not built. prikk computes the difference and
> its hunks; **stikk shows them, and computes nothing.** Every rule from RFC 035 A3 and RFC 036 applies: content is
> quoted byte-faithfully, names are identifiers, and nothing invisible reaches the terminal.
>
> **No `prep/` branch.** Submit on the local gates and the local real-binary suite. After approval, push to `main`,
> then dispatch the full-matrix suite and the supply-chain gate on `main` and report them (§8).

---

## 1. Measure first, at 0.51

Re-run `.git-exclude/reports/038-compare/measure038.sh` and confirm F1. **Then add what it lacks:**
- **two divergent branches**: commit and seal on `heads/side` after `branch switch`, then `diff --from heads/side
  --to heads/main`. Record the entries, and whether a rename declared on one side pairs (prikk's guide: across
  branches, an unpaired `deleted` plus `added` means "not paired here");
- **a merge block, if prikk can make one at 0.51**, so you can say what "the previous block in this ref's history"
  is there;
- **`minimal: false`**, if you can trigger it cheaply (a large file with its lines reversed); if not, say so, and
  test it with a stand-in.

**If anything differs from F1, stop and report.**

## 2. Commit 1: the seam and the reader

- **`Prikk::diff(repo, from, to)`**: each side is a point (a block id, or a ref) or the worktree. **The worktree
  form is the bare `diff`**, with no `--from` and no `--to`, so that the left side is what `commit` would author
  against (the tip plus the queue). Banded at **prikk ≥ 0.46** with **one** constant beside `TREE_FROM_MINOR`. Below
  it, spawn nothing and answer *unreported*.
- **The reader** (`diff-report-v1`), as RFC 038 decision 2 gives it:
  - each side's `target_block_id` is 64-hex, **or the word `worktree` exactly where `point` is `worktree`**;
  - `queued_patches` is read when present and is a non-negative integer. **Its absence means none queued only on
    the left side of a bare `diff`**; anywhere else stikk shows nothing about a queue;
  - per-status side objects (`added` has `to` only, `deleted` has `from` only, `renamed` has `from_path`, the rest
    have both). **A mismatch is an environment error.** Read absence per status, never as a general rule;
  - `minimal` is a boolean, `hunks` a list of strings, and `unsupported_paths` a list of `{path, refusal}`;
  - **an unmeasured status is kept as prikk's word**, with its hunks.
- **Tests:** a captured 0.51 report for every status in F1, the worktree form with a queue and an unsupported path,
  and §1's divergent-branch report. **At least one negative per rule above.**

## 3. Commit 2: the view-model, the view, and the three entry points

**The view-model (core)** carries structure, not composed text (RFC 036 decision 8, `Piece`). Each entry gives its
status word, its path(s) as names, both sides' metadata, `minimal`, and its hunks as **rows**:
- **the `@@` header**, as prikk wrote it (prose);
- **each body line as its marker plus its content**, the content quoted with RFC 035 A3's notation (`escape.rs`),
  **terminator inside the quotes**. Split a hunk with `split_inclusive('\n')`, and take the marker from the line's
  first character;
- **prikk's `\ No newline at end of file`** as prikk's words, not quoted as content;
- **bounded per entry** by `CONTENT_DISPLAY_BYTES`, with *N more bytes not shown*.

**The view:**
- **A heading naming both sides:** short block ids, refs as identifiers, *the worktree*, and *the tip plus N queued
  commits not yet sealed* when `queued_patches` is present on a bare `diff`'s left side.
- **Each entry** under prikk's status word, with: a rename's two paths; a `mode` entry's two modes in octal; a
  `binary` entry's two sizes and `content_id`s (never bytes); `minimal: false` with prikk's note, *not the
  shortest edit script*; and `unsupported_paths` with `commit`'s refusal verbatim.
- **Markers coloured** (`-`, `+`, context) in the theme's existing warn and accent, and **content wrapped with
  `wrap_verbatim`**, never `wrap_indented`.
- **It scrolls**, in Patch detail's idiom. `v` shows the selected entry's raw JSON. How an entry is selected is
  yours to design; say what you chose.
- **Below 0.46:** stikk's sentence, *stikk compares from prikk 0.46 on; this is 0.N*.
- **A refusal from `diff`** goes through the usual refusal path, verbatim. **An empty comparison** says *no
  differences* in stikk's words, which is true only when prikk listed no entries **and** no unsupported paths.

**The entry points (keys `m` and `d`; both are free):**
- **History:** `m` marks or unmarks the selected block. The mark is shown on its row, and the status bar or title
  says one is set. `d` compares **the marked block with the selected one, ordered older → newer by lineage**, and
  the heading says which side is which.
- **History and Block detail, nothing marked:** `d` compares the block with **the previous block in this ref's
  history**, which is the next older row in the loaded lineage, named as exactly that. **On the oldest loaded
  block**, `d` is unavailable with stikk's reason: *the previous block is not in the loaded history*. Do not fetch
  more history for it.
- **Changes:** `d` compares the worktree with what `commit` would author against (the bare `diff`). **The foot's
  *"per-file content diff awaits prikk support (UD-09)"* goes**, replaced by the `d` hint.
- **A compare against the same block twice** (mark and select the same row) is *no differences*, computed by prikk
  like any other comparison. Do not special-case it.
- Keys are listed in the Glossary's Keys section, in the pattern the other keys use.

## 4. `requirements.md`

**`UD-10` retired**, with RFC 038 F3's measurement (`cat` at a bare block id, 0.46 and 0.51). **`FR-033` delivered**
at prikk ≥ 0.46 for the three comparisons; comparing two different refs is named as **not built** (Later).

## 5. The changelog

Under `## Unreleased`: **`### Added`**, Compare from History, Block detail and Changes (prikk ≥ 0.46), saying what
each compares. **`### Fixed`**, the Changes foot no longer says a content diff awaits prikk. **`### Breaking`**, the
new `Prikk` method and any type it changes.

## 6. Tests

- Core, with a stand-in: the row shapes for every status; a CRLF line, a trailing-space line and a missing final
  newline, each rendering differently from its look-alike; the bound; `minimal: false`; the queue heading; the
  marked-pair ordering, whichever was marked first; the oldest-block reason.
- **Suite legs at 0.51:** History, two blocks marked and compared; Block detail against the previous block; Changes
  against the worktree with a queued commit present; §1's divergent branches if you reached them. **At 0.28**, the
  below-0.46 sentence.
- **Captures at exactly 80 columns:** a modified text file with a whitespace-only line change; a rename with hunks;
  a binary; a `mode`; the worktree form with an unsupported path; the below-0.46 sentence.

## 7. The traps

1. **stikk computes nothing.** No renumbering, no re-hunking, no rename inference, no "these look the same".
2. **`queued_patches` absent is "none queued" on a bare diff's left side only.**
3. **`worktree` is a word, not an id**, and only where `point` says so.
4. **Content lines are quoted; prikk's `@@` header and its no-newline note are not.** The marker is not content.
5. **Order older → newer** for a marked pair, whichever was marked first.
6. **Paste captures as the file holds them, and grep the request for decoded escapes** before submitting.

## 8. Gates, runs, submit (no branch)

**Before submitting:** the eight gates, and the real-binary suite **locally** at prikk 0.28.0 and 0.51.0. Package to
`.git-exclude/review-request/038-compare/review-request-v1.md`, leading with §1's measurement.

**After approval:** push to `main`, let CI run, then dispatch the full-matrix suite and the supply-chain gate on
`main` (`gh workflow run … --ref main`), and report every run id and result. A macOS- or Windows-only failure is fixed
by a follow-up commit on `main`.

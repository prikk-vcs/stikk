# RFC 038 — Compare: what changed between two points, in prikk's own hunks

**Status.** **Accepted by the project owner 2026-10-14.** **Proposed 2026-10-14** by the architect, as increment 3 of the authorized 0.10.0 scope
(`.git-exclude/release/0-10-0-scope.md`). **No open question.**
Measured against real prikk **0.46.0** and **0.51.0** binaries, **identical in every case** once ids are normalised.
Evidence: `.git-exclude/reports/038-compare/` (`measure038.sh`, `measure038-0.46.txt`, `measure038-0.51.txt`), and
prikk's guide in the clone (`docs/src/guide/diff.md`).
**Tracks.** `FR-033`, `FR-034`, `UD-10`, `C-T2b`, `C-T2c′`, `C-T4d`, `ER-02`, `T-T2`, `T-T4`; RFC 021 F3
(superseded), RFC 035 (decision 4 and A3), RFC 036, RFC 037.
**Touches.** `stikk-prikk` (a `diff` seam method and its reader), `stikk-core` (a compare view-model),
`stikk-tui` (the Compare view, and the keys that open it from History, Block detail and Changes),
`stikk-real-binary`, `requirements.md`, and the changelog.

## Summary

**`FR-033` is the last named view stikk has not built, and prikk 0.46 removed what blocked it.** The roadmap said
prikk refused arbitrary-point comparison (RFC 021 F3). That predates `prikk diff`, which compares **any two points**
(refs and bare block ids), or a point with **the worktree as `commit` would author it**, and reports each differing
path with its status, both sides' metadata, and **unified hunks prikk computed**.

Compare shows that report. stikk computes no difference itself.

## Findings

### F1 — the report, as measured

`diff --from <a> --to <b> --format json` → `diff-report-v1`:
`{schema_version, from, to, entries[], unsupported_paths[]}`.

- **`from`/`to`:** `{point, target_block_id}`, where `point` is the value as given. **The worktree side is
  `{"point": "worktree", "target_block_id": "worktree"}`**, a word and not an id. **A bare `diff`'s left side carries
  `queued_patches: N`** when commits are queued: the sealed tip *plus* that queue, a state no block names. The key
  is absent when nothing is queued.
- **Each entry:** `path`, `status`, `minimal` (bool), `hunks` (a list of strings), and the side objects `from`/`to`
  (`{kind, encoding, mode, size}`, plus `content_id` for a binary). **A side that does not exist is absent, not
  `null`**: an `added` entry has no `from`, and a `deleted` entry has no `to`. A `renamed` entry adds `from_path`.
- **Statuses measured:** `added`, `deleted`, `modified`, `renamed`, `mode` (hunks empty), `binary` (hunks empty;
  ids and sizes). prikk's schema also names `symlink`, *"not produced today"*.
- **Each hunk is prikk's unified text**: `"@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n"`. A CRLF line keeps its `\r`
  (`"+crlf\r\n"`). A missing final newline carries prikk's own line `\ No newline at end of file`. A trailing-space
  edit is `"-keep\n+keep  \n"`.
- **`unsupported_paths`** (worktree form only): `[{path, refusal}]`, with `commit`'s own refusal. It is always empty
  between two points.
- **Two identical points:** no entries, exit 0. **An absent ref:** exit 1, *"ref heads/nope does not exist in this
  repository"*.

### F2 — what prikk promises about the hunks

prikk's guide: 3 lines of context; **applying an entry's hunks to its left side reproduces the right byte for
byte**; renames are **declared, never inferred** (a delete and an add with the same bytes stay two entries); a
chain `a → b → c` is one entry. **`minimal: false`** means the line-diff hit its work bound and the hunks are larger
than the shortest, but still correct. *"You are always told."*

### F3 — `UD-10` is retired

`cat --path text.txt --ref <bare block id>` returns the file's bytes at that block (exit 0) at 0.46 and 0.51, and
`--format json` gives `path-content-v1` metadata. **Compare does not need it**, since the hunks carry the content,
but the requirement it blocked is no longer blocked.

### F4 — the Changes view says something false today

Its foot reads *"per-file content diff awaits prikk support (UD-09)"*. At the ceiling, `prikk diff` with no
arguments **is** that diff: the worktree against what `commit` would author against. That is `C-T2b`.

## Decisions

1. **One seam method, `diff(from, to)`**, where each side is a point (a block id, or a ref when the user named one)
   or the worktree. It runs **`diff [--from <a>] [--to <b>] --format json`**, and the worktree form is the bare
   `diff`. Banded at **prikk ≥ 0.46** with one constant. **Below it, nothing is spawned**: Compare is unavailable
   with stikk's reason (`C-T4d`), *stikk compares from prikk 0.46 on*. **No prose `diff` is parsed, ever.**

2. **The reader is strict where stikk relies on the shape, and keeps prikk's words where it does not:**
   `diff-report-v1`. Each side's `target_block_id` is a 64-hex id, **or the word `worktree` exactly where the point
   is `worktree`**. Each entry's side objects agree with its status: `added` has `to` and no `from`; `deleted` has
   `from` and no `to`; `renamed` has `from_path`; the rest have both. `minimal` is a boolean, and `hunks` is a list
   of strings. **A status stikk has not measured is kept as prikk's word**, with its hunks shown and nothing
   inferred. **Absence is read per status, never as a general rule.**

3. **Three places open Compare, each with a comparison that is honest there:**
   - **History:** `m` marks a block, and `d` compares the marked block with the selected one, ordered
     **older → newer** by the lineage whichever was marked first. The view says which is which.
   - **History and Block detail, with nothing marked:** `d` compares the block with **the previous block in this
     ref's history**, named as exactly that, not as "parent": a merge block has more than one parent, and the
     lineage is what the user is browsing. **The oldest loaded block has no previous one in view**, so `d` is
     unavailable there, with stikk's reason.
   - **Changes:** `d` compares **the worktree with what `commit` would author against**, which is the bare `diff`:
     the tip plus any queued commits. The view states the queue count when there is one. **This replaces F4's false
     sentence**: the foot says `d` shows it line by line.
   Comparing two different refs is **not** offered in this RFC (Later). prikk can do it, but the picker that would
   choose two refs is its own design.

4. **RFC 035's decision 4 is met here, in Block detail, for every block**: Block detail's `d` *is* "what this block
   changed", from `prikk diff`. Patch detail does not grow a second entry point. That rule's intent (never merged
   into the operation list, never for a queued patch) holds by construction.

5. **The Compare view:**
   - **A heading** that names both sides: block ids short, refs as identifiers (RFC 036), *the worktree*, and *the
     tip plus N queued commits not yet sealed* where prikk reports a queue.
   - **One section per entry**, under prikk's status word verbatim and the path as an identifier. A rename names
     both paths. A `mode` entry shows both modes in octal. A `binary` entry shows both sizes and both `content_id`s,
     **never bytes**.
   - **Hunks: the `@@` header as prikk wrote it, then each line as its marker (`-`, `+`, space) followed by the
     line's content, quoted with RFC 035 A3's notation, terminator inside the quotes.** So a trailing-space edit, a
     CRLF change and a missing final newline are all visible, and **two lines that differ in any byte never render
     alike**. prikk's `\ No newline at end of file` line is shown as prikk's words. Lines are hard-wrapped, never
     word-wrapped. **stikk never renumbers, recomputes or re-hunks.**
   - **`minimal: false`** puts prikk's note under that entry: *not the shortest edit script*.
   - **`unsupported_paths`** (worktree form) are listed with `commit`'s refusal verbatim, as the Changes view
     already does.
   - **Bounded** per entry by RFC 035's `CONTENT_DISPLAY_BYTES`, with *N more bytes not shown*, never a silent cut.
   - It **scrolls**, in Patch detail's idiom. Selection between entries is the handoff's to design. `v` shows
     prikk's raw JSON for an entry, as in Patch detail.

6. **A refusal from `diff`** (an absent ref, or a live `prikk mv` that `commit` would refuse) **is shown verbatim**
   in the usual refusal path (`ER-02`), never as an empty comparison.

7. **`UD-10` is retired in `requirements.md`**, with F3's measurement. **`FR-033` is delivered** at prikk ≥ 0.46 for
   the three comparisons in decision 3, and two-ref comparison is named as not built.

## Changelog

`### Added`: Compare, from History, Block detail and Changes (prikk ≥ 0.46). `### Fixed`: the Changes foot no longer
says a content diff awaits prikk.

## Delivery

**One handoff, issued on acceptance:** [Handoff A](../handoffs/038-compare/a-compare-handoff-v1.md), **in two commits:** the seam method and reader; then the view-model, the view and the
three entry points. It re-measures F1 at 0.51 first, **adding a divergent second branch** (`measure038.sh` only
compared two refs at the same point), and stops if the shape differs.

## What this RFC does not do

- **No comparison of two different refs** (Later).
- **No syntax highlighting, intra-line highlighting or side-by-side layout.** prikk's unified hunks are what is
  shown.
- **No stikk-computed difference**, at any version.

# RFC 037 — Read prikk's JSON where it now answers: every block's files, and an unpublished current branch

**Status.** **Accepted by the project owner 2026-10-14.** **Proposed 2026-10-14** by the architect, as increment 2 of the authorized 0.10.0 scope
(`.git-exclude/release/0-10-0-scope.md`). **No open question.**
Measured against real prikk **0.51.0** (and **0.46.0** for `tree`'s band floor) binaries. Evidence:
`.git-exclude/reports/037-read-prikk-json/` (`measure037.sh`, with `measure037-0.51.txt` and `measure037-0.46.txt`),
and the 0.51 re-check's `.git-exclude/reports/036-prikk-0-51-recheck/mtree-0.46-0.51.txt`.
**Tracks.** `FR-031`, `FR-032`, `FR-014`, `C-T2b`, `C-T2c′`, `ER-02`; RFC 006 (Block detail at the tip), RFC 032
(decision 5, *published* = membership in `branches[]`), RFC 034, prikk reply 020.
**Touches.** `stikk-prikk` (`block_state` and the branch-list reader), `stikk-core` (`block_detail`, `list_refs`),
`stikk-tui` (Block detail, the ref picker), `stikk-real-binary`, and the changelog.

## Summary

**Two things stikk cannot show today, and prikk now answers both in JSON.**

1. **Block detail shows the state's files only at a ref's tip**, from prose `checkout --patch-plan`, and says
   *"prikk replays only to the ref tip"* for every older block. **That stopped being true at prikk 0.46.** `tree
   --ref <block-id> --format json` lists any block's files.
2. **The ref picker cannot offer a branch that has never been sealed**: the state every new repository is in,
   and the branch stikk is already standing on. prikk 0.47 names it, in a field built for stikk.

## Findings

### F1 — `tree` answers for any block, by its bare id

Measured at **0.51 and 0.46** (`measure037-*.txt`), identical in shape. A lineage of two blocks: the first holds `a.txt`, the second deletes it and
adds `b.txt`. `tree --ref <block-id> --format json` reports **each block's own state**: `target_block_id` equal to the
id asked for, the first block listing `a.txt` and the tip listing `b.txt`, each with `size` and `mode`.

**At the tip it agrees with the prose stikk parses today.** Measured 2026-10-13 at 0.46 and 0.51: the same block
id, the same paths, and per-file `size` summing to `result content bytes`. It adds each file's mode.

### F2 — prikk's `--patch-plan` is the prose report letter 019 asked prikk to warn us about

`block_state` parses its `target block:`, `result content bytes:` and `file:` lines. prikk had planned a test pinning those lines and
decided not to add it once we said we would move off them (prikk's reply 022 §4). **The prose is now unguarded
upstream**, so moving off it at the ceiling is not optional housekeeping.

### F3 — prikk names an unpublished current branch, and only that

Measured at 0.51. `branch list --all --format json` carries a top-level **`unpublished_current_branch`**:
- **`"heads/main"`** in a fresh repository, and still after a commit is queued, while nothing is sealed;
- **`null`** once the first seal publishes it, when it appears in `branches[]` instead.

**`branch create` publishes at once**: the new branch is in `branches[]` immediately and never appears as
unpublished. So the field names exactly one thing: **the branch the user is on, before its first seal.** It arrived
in prikk 0.47.0 (prikk's own changelog); **at 0.46 the field is absent**, measured.

### F4 — why it must not become a `RefEntry`

RFC 032 decision 5 reads *"has this ref published history?"* as **membership in `refs()`**. prikk deliberately
kept `branches[]` unchanged and added a separate field (reply 020), **because stikk's answer showed that an
unmarked row would invert that claim.** A `RefEntry` for the unpublished branch would make Changes call it
published.

## Decisions

1. **Block detail's files come from `tree --ref <block-id> --format json` at prikk ≥ 0.46, for every block.** Pass
   the block's own id, never the ref name, so the answer is about the block the user opened. The state keeps its
   shape (the target block, the file list, total bytes as the sum of `size`), **plus each file's mode**, which the
   view shows. **The reader is strict**: `tree-listing-v1`, and `target_block_id` must equal the id asked for;
   anything else is stikk's environment error, never a wrong file list.
2. **Below 0.46 nothing changes**: the tip from prose `--patch-plan`, and the existing sentence for older blocks.
   That sentence becomes **conditional on the version**, because at ≥ 0.46 it would be false.
3. **The branch-list reader returns the unpublished current branch beside the refs, never among them.** It has
   three states, and never keys on absence:
   - *reported, none* (`null`);
   - *reported, this branch* (a validated ref name);
   - *not reported* (prikk < 0.47, where the field is absent).
   `refs()`'s list, and therefore RFC 032's *published* test, are unchanged.
4. **The picker offers it as its own row**, marked in stikk's words as **the branch you are on, not yet published**,
   and rendered as an identifier (RFC 036). Selecting it focuses that ref as any other: History shows its queued
   tier and no sealed blocks, and Changes works as it does today. **The row appears only when prikk reports the
   field**; below 0.47 the picker is as it is.
5. **A failure of either new read costs only its own section** (RFC 035 A3's rule): a failed `tree` costs Block
   detail its file list, not the block.

## Changelog

`### Added`: the picker offers the current branch before its first seal. `### Changed`: Block detail lists any
block's files, with modes, at prikk ≥ 0.46. Both go under `## Unreleased`.

## Delivery

**Two handoffs, independent of each other, issued on acceptance:** [A: every block's files](../handoffs/037-read-prikk-json-where-it-now-answers/a-every-blocks-files-handoff-v1.md) (which also carries RFC 036's History-tier clip) · [B: the unpublished current branch](../handoffs/037-read-prikk-json-where-it-now-answers/b-unpublished-current-branch-handoff-v1.md). A, every block's files (decisions 1, 2, 5); B, the unpublished current
branch (decisions 3, 4). **Each re-measures its own row at 0.51 before building**, and each asks for its changelog
entry.

## What this RFC does not do

- **No state comparison between blocks.** That is Compare (RFC 038).
- **No change to RFC 032's published test** or to the `--ref` retry, which still serves prikk 0.45 and 0.46.
- **No retirement of the prose `--patch-plan` path**: it stays for prikk < 0.46. No user is there, but the path
  already exists and costs nothing to keep.

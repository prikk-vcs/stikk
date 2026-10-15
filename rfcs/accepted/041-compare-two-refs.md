# RFC 041 — Compare two refs: choose the pair in the ref picker

**Status.** **Accepted by the project owner 2026-10-15.** **Proposed 2026-10-15** by the architect, as 0.11.0's increment 5 (`.git-exclude/release/0-11-0-scope.md`,
authorized; *"the first to move to 0.12.0 if the cycle runs long"*). **No open question.**
Grounded in RFC 038's delivered seam (`Comparison::Points`) and its measurement in delivery: renames pair across
branches when the file is in the common base (RFC 038, *Corrected by measurement in delivery*). prikk's guide: a
point is *"a ref — a received ref (`remotes/…`) too, read and not adopted — or a bare block id"*.
**Tracks.** `FR-033` (its last clause), `FR-014` (the picker), `C-T2b`, `ER-02`; RFC 037 B (the unpublished row), RFC
038.
**Touches.** `stikk-tui` (the ref picker and the Compare heading), `stikk-core` (the compare source),
`stikk-real-binary`, `requirements.md`, and the changelog. **No seam change.**

## Summary

RFC 038 compares two blocks in one lineage, or the worktree with what `commit` would author. **It left out the
comparison a branch user asks first: *what is different between this branch and that one?*** prikk answers it
(`diff --from <ref> --to <ref>`), and stikk's seam already sends it. What is missing is a way to **choose two refs**.
This RFC puts that choice in the ref picker, with the same two keys History uses: **`m` marks, `d` compares**.

## Decisions

1. **In the ref picker (`b`), `m` marks the selected ref and `d` compares the marked ref with the selected one.** The
   mark shows on its row (`●`, as in History), and the picker's title says a mark is set. `d` with nothing marked
   does nothing and says *mark a ref with `m` first*. A second `m` on the marked row unmarks it. **Closing the picker
   keeps the mark only until the next ref change**; a stale pair is worse than an empty one.

2. **Which side is which: the marked ref is `from`, the selected ref is `to`**, and the heading says so in words,
   e.g. *heads/feature (marked) → heads/main (selected)*. **Two refs have no lineage order** to put older first, as
   History does, so stikk does not invent one. It states the order the user chose.

3. **The heading names the block each side resolved to**, from `diff-report-v1`'s `target_block_id`, beside the ref
   name. **A ref can move, a block cannot**, and `r` re-runs the comparison against wherever the refs now point,
   saying so. **Never a cached pair across runs.**

4. **Every ref the picker lists can be a side**: branches, tags, and received refs (`remotes/…`), which prikk
   compares *"read and not adopted"*. **The unpublished current branch** (RFC 037 B's row) **cannot**: it has no
   block. `m` or `d` on it says *this branch has no published history to compare yet*, and points at Changes' `d`
   (the worktree against what `commit` would author), which is the comparison that branch does have.

5. **The Compare view is RFC 038's, unchanged**: the same entries, quoting, bound, raw view and refusal path. A
   pair of refs pointing at the same block is *no differences*, computed by prikk, not special-cased.

6. **`FR-033` delivered in full** at prikk ≥ 0.46: two blocks, a block and its predecessor, the worktree, and two
   refs. `requirements.md` drops *"two-ref comparison not built"*.

## Changelog

`### Added`: compare two refs from the ref picker (`m` then `d`), prikk ≥ 0.46.

## Delivery

**One handoff, issued on acceptance:** [Handoff A](../handoffs/041-compare-two-refs/a-compare-two-refs-handoff-v1.md). It **measures first** at 0.51, before building:
- two **diverged** branches, both directions;
- a **tag** against a branch;
- a **received ref** against a branch, if the suite can make one cheaply (`bundle import` or `sync`). **If it cannot,
  it says so**, and the received-ref case is tested with a stand-in;
- **the unpublished current branch named as `--from` and as `--to`**, recording what prikk says, so decision 4's
  sentence is checked against prikk rather than assumed.

**It stops if any of these differs from RFC 038's report shape.**

## What this RFC does not do

- **No arbitrary block-id entry** (typing an id). The picker offers refs; History offers blocks.
- **No three-way or merge-base view.** prikk's `diff` compares two points, and stikk shows that.

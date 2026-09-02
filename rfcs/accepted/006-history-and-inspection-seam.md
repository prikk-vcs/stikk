# RFC 006 — History &amp; inspection: read-seam growth and the patch-detail dependency

**Status.** Accepted (2026-09-02) — grow the seam for History (block lineage) and Block detail/state
tree now; **Patch detail (FR-030) is blocked on a prikk-side gap** and is split out until it lands.
Handoff:
[`../handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md`](../handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md).
**Tracks.** The roadmap's "Next" increment 3 — History (`FR-010…017`) and Patch/Block detail
(`FR-030…032`) — and how far prikk's current CLI actually lets it go.
**Touches.** `stikk-prikk` (new `Prikk` methods + confined parsers), `stikk-core` (new read
operations + view-models), `stikk-tui` (History and Block-detail views). Nothing mutates; nothing
below the seam's contract changes.

## Summary

The shell + Orientation increment (RFC 001) shipped. The next roadmap step is browsing history. This
RFC records what that increment can actually deliver against prikk 0.27.1, because a check of prikk's
command surface (verified at `04e9391`, not assumed) changes the scope the internal design and roadmap
implied.

## The finding that scopes this increment

prikk's CLI exposes history at **block granularity only**:

- **`prikk log [--ref] [--limit]`** lists each block on a ref with: block id, ref-state id,
  `update-seq`, `kind` (Root/Normal/Merge/…), `rollback-block`, **counts** of parents / patches /
  rollback-patches / required-attestations, and `previous-ref-state`. It carries **no per-patch
  detail, no patch ids, and (by prikk's no-clock, message-not-yet-persisted design) no message,
  author, or date.**
- **`prikk checkout --patch-plan` / `--snapshot-plan` / `--plan-only`** expose the *replayed state* —
  the result file set and content byte totals at a ref's tip — not a block's internal operations.
- **There is no `prikk show`, no `prikk diff`, and no command that enumerates the patch ids inside a
  block or renders a single patch's operations.** (The audit's completeness matrix already recorded
  `show` and `diff` as Missing; this confirms it against the seam's needs.)

**Consequence:** History browsing (block lineage) and a block's *state* (file tree via the plan
surfaces) are deliverable now. **Patch detail — a patch's operations rendered as a diff (FR-030) — is
not, because prikk does not expose the data through any public surface.** stikk must not fabricate it,
and must not pretend a block's patches are inspectable when only their count is.

## Decisions

1. **Grow the seam for read-history and read-state now** (design CT-03 categories `read-history`,
   `read-state`): add `Prikk::history(ref, limit)` (parse `log`) and `Prikk::block_state(ref)` (parse
   `checkout --patch-plan`'s result-file set). Parsing stays confined to `cli_backend/parse/`,
   version-gated, golden-fixture-tested, and **refuses rather than guesses** on an unrecognized shape
   (the UD-02 discipline the CLI backend already follows).
2. **Ship History as a block-lineage browser** (`FR-010/011/014/016` at block granularity) plus a
   **Block detail** view (metadata from `log` + the state file list from the plan). This is the
   honest, useful core.
3. **Split Patch detail out** as increment 3b, blocked on the upstream dependency below. History rows
   for a block show what prikk gives — counts and lineage — and say, where a user would expect to open
   a patch, that per-patch inspection awaits prikk support. No faked diffs, no invented patch ids.
4. **The unsealed queue tier** (`FR-010`) is rendered above the sealed lineage from `status` (queued
   count) — prikk exposes the count, not the queued patches' ids, so the tier shows the count as a
   distinct "not yet history" band, consistent with decision 3.

## Upstream dependency

**UD-09 (extends the requirements' UD-01…08, and is the same shape as UD-02).** prikk exposes no
machine-readable per-patch or per-block-patch-list inspection. Patch detail (FR-030), patch-id
enumeration within a block, and diff-aware search (`FR-013` content search) all wait on it. The clean
upstream ask, in priority order:

- `prikk log --format json` (block lineage as data, incl. the patch ids in each block), and
- a patch-content surface (`prikk show <patch>` or `--format json`) rendering a patch's operations
  with their preimages — the data already exists in the object model (EditText carries `old_span_text`
  etc.); it simply has no CLI output.

Until UD-09 lands, stikk's prikk encoder for History parses the human `log`/plan output under the
confined, version-gated, refuse-on-mismatch rule; Patch detail is not built. This is filed as a prikk
issue, mirroring how the requirements track UD-01…08.

## Open questions

- Whether to parse human `log` output now or wait for `--format json`. **Ruled:** parse now, confined
  and golden-fixture-pinned (UD-02 discipline), and adopt JSON when prikk offers it — History is worth
  shipping before UD-09, and the block-lineage shape is stable.
- Whether Block detail's state tree should use `--patch-plan` (replayed files) or a future dedicated
  surface. **Ruled for now:** `--patch-plan`'s result-file set, with modes/per-file content deferred
  to when a richer surface exists; the file list + byte totals are enough for a first Block detail.
- Where the focused-ref *selection* UI lives (History needs to switch the ref it lists). **Ruled:** a
  ref picker in History sets the client-side focused ref (design FR-055); still no HEAD, still no
  worktree mutation.

## Consequences

- `stikk-tui` gains a History view and a Block-detail view; the overlay layer and inert-text primitive
  from RFC 001 get their first heavy use (block ids, ref names, and the state file list are all
  repository-sourced and go through `inert`).
- The seam grows two read methods and two parsers; no mutation, no new trust surface.
- Patch detail and diff-aware search are explicitly deferred to 3b behind UD-09, recorded so the gap
  is a stated property, not a surprise.

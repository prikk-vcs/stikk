# Handoff — History &amp; Block detail (v1)

**Companion to:** RFC 006 (Accepted 2026-09-02). Inherits its state.
**Realizes:** ROADMAP "Next" increment 3 — History (`FR-010…017`) and Block detail (part of
`FR-031/032`). **Patch detail (`FR-030`) is out of this handoff** — it is increment 3b, blocked on
UD-09 (RFC 006).
**Design items:** `FR-010/011/014/016` (history browsing), `FR-031/032` (block detail, state tree — at
block granularity), `FR-055` (focused-ref selection), `TU-04` (view stack / navigation), and the
existing shell (`TU-01/02`), inert-text primitive (`C-T2a`), and overlay layer from RFC 001.

This is the program design and decision record for the increment. **Implementation, tests, and the
example follow it.** Where this handoff and RFC 006 or the design set disagree, the RFC/design wins and
this handoff is corrected first.

---

## 1. Scope

**In:**
- A **History view**: the sealed block lineage of the focused ref (newest tip first), with the
  unsealed **queue tier** above it, and a **ref picker** to change which ref is listed.
- A **Block detail view**: a block's metadata (from `log`) plus its **state file list** (from
  `checkout --patch-plan`), reached by opening a block row (view-stack push, `TU-04`).
- Two new seam methods (`history`, `block_state`) and their confined parsers.

**Out (do not build here):**
- **Patch detail** (`FR-030`) and any per-patch enumeration or diff — increment 3b, blocked on UD-09.
  Where a user would open a patch, show the block's patch *count* and a one-line note that per-patch
  inspection awaits prikk support. No faked diffs, no invented patch ids.
- Compare, Changes, mutations, merge, exchange, the refusal-explanation overlay *content* (increment 4
  — a History load failure uses the existing failure-body pattern for now), diff-aware search
  (`FR-013` content search — also UD-09).
- Per-file content or modes in the state tree (defer to a richer prikk surface); the file *list* +
  byte totals are the Block-detail state view for now.

---

## 2. The seam grows (`stikk-prikk`)

Two methods on the `Prikk` trait (categories `read-history`, `read-state` — CT-03). No mutation.

- **`fn history(&self, reff: &str, limit: usize) -> Result<History>`** — parse `prikk log --ref <reff>
  --limit <n>`. `History` carries an ordered `Vec<BlockRow>` (tip first) and the queue tier (see
  below). `BlockRow` fields, all from `log`: block id, ref-state id, `update_seq`, kind
  (Root/Normal/Merge/Repair/Import), `rollback_block: bool`, and the counts (parents, patches,
  rollback-patches, required-attestations), and `previous_ref_state: Option`. **No patch ids, no
  message/author/date — prikk does not emit them** (RFC 006).
- **`fn block_state(&self, reff: &str) -> Result<StateFiles>`** — parse `prikk checkout --patch-plan
  --ref <reff>`: the result **file list** and the total content byte count and the target block id.
  (This is the replayed tip state; a per-block-at-arbitrary-id state view waits on a richer surface —
  RFC 006 open question, ruled deferred.)
- **The queue tier**: reuse the existing `orientation`/`status` read — the queued-patch **count** — and
  surface it as a distinct "not yet history" band. prikk exposes the count, not the queued ids.

**Parsing rules (unchanged from the CLI backend's discipline, SEAM-03 / UD-02):** parsers live in
`cli_backend/parse/`, one per command; each **refuses with `StikkError::Environment` on an
unrecognized shape** rather than fabricating; golden fixtures pin the shapes (see §5). A new prikk
version that changes `log`'s format fails a fixture, not a user.

### Captured parse targets (golden fixtures — real output at prikk 0.27.1)

`prikk log` (two-block repo), the exact shape the `history` parser must read:

```
history repository: <path>/.prikk
ref: heads/main
block <64-hex>
  ref-state: <64-hex>
  update-seq: <n>
  kind: Normal
  rollback-block: false
  parents: 1
  patches: 1
  rollback-patches: 0
  required-attestations: 0
  previous-ref-state: <64-hex | <none>>
block <64-hex>
  ...
  kind: Root
  previous-ref-state: <none>
```

`prikk checkout --patch-plan`, the shape the `block_state` parser must read:

```
patch replay plan repository: <path>/.prikk
ref: heads/main
target block: <64-hex>
blocks replayed: <n>
patches replayed: <n>
operations applied: <n>
result files: <n>
result content bytes: <n>
  file: <repo-path>
  file: <repo-path>
note: ...
```

Both fixtures ship verbatim in `cli_backend/parse/tests.rs` (append to the existing status fixture).

---

## 3. The views (`stikk-tui`)

- **`view/history.rs`** — renders `History`: a scrollable list, **queue tier first** (a distinct
  band: "● N queued (not yet sealed)"), then block rows newest-first. Each block row: abbreviated id
  (expand/copy later), kind, `update_seq`, and the counts (e.g. "1 patch · 1 parent"); a rollback
  block and a merge block are visually distinct (kind drives a small label/colour, text-forward per
  NFR-A03). Selection moves with `j/k`/arrows; `Enter` opens Block detail. All ids/paths go through
  `inert` (C-T2a).
- **`view/block.rs`** — Block detail: the block's full metadata (from the `BlockRow`) and its state
  **file list** (from `block_state`), with the "patches: N — per-patch inspection awaits prikk
  support (UD-09)" note where patch detail would go. `Esc`/`q` pops back to History (view stack,
  TU-04).
- **Ref picker** — a small overlay (reusing the overlay layer) listing the repository's refs (from a
  new `refs()` seam read, or from `branch list` — see below), selecting one sets the client-side
  focused ref (`FR-055`) and reloads History. **No HEAD, no worktree change.**
- **Navigation** (`TU-04`): History is a top-level view reachable from Orientation (a key, e.g. `h`,
  and the palette later); Block detail is a push; the ref picker is an overlay. Wire these into the
  existing `keys::dispatch` `Action` seam — add `OpenHistory`, `OpenBlock`, `OpenRefPicker`, `Back`
  actions; do not hard-code keys elsewhere (RFC 002 still owns the eventual catalog).

*Refs list:* the ref picker needs the repository's refs. Prefer a dedicated `refs()` seam method
parsing `prikk branch [list] --all` + `tag list` (block-granular, already machine-parseable enough);
if that is more than this increment needs, scope the picker to `heads/main` + any `--ref` given at
launch and defer full ref listing to a follow-up. Ruled: **include the `branch list` parse** — it is
small and History is far more useful when you can switch refs.

---

## 4. Decision notes

- **Block granularity is the honest ceiling this increment** (RFC 006). History and Block detail show
  what prikk exposes; Patch detail waits on UD-09. The UI states the gap at the point a user meets it,
  rather than omitting the affordance silently — "where prikk lacks, stikk explains," the same stance
  as "where prikk refuses, stikk explains."
- **Parse human output now, adopt JSON later** (RFC 006). The block-lineage shape is stable; parsing
  is confined and golden-pinned; a `--format json` from prikk (UD-09) later swaps the parser
  generation without touching the views.
- **The seam stays read-only.** `history`/`block_state`/`refs` are `read-history`/`read-state`
  category — no mutation, no new trust surface, no key material.
- **Reuse, don't rebuild.** The overlay layer (ref picker), the inert-text primitive (all ids/paths),
  the view stack, the palette, and the failure-body pattern all come from RFC 001; this increment is
  their first heavy use, which is exactly why they were built first.

---

## 5. Security surface

No new trust boundary, no mutation, no secret. The one real surface is **untrusted content rendered in
bulk** — block ids, ref names, and repository file paths in the state list are repository-sourced and
can carry control sequences (threat T-T2). The inert-text primitive (`C-T2a`, built in RFC 001) must
wrap **every** such string: block/ref/ref-state ids, the ref-picker names, and each state file path.
This is the increment the primitive was built ahead of; a test asserts a hostile file path in the
state list renders inert. No threat-model change is needed (no new asset/flow/boundary); this section
records that the review happened (NFR-S07).

---

## 6. Test plan

- **Golden-fixture parser tests** (`cli_backend/parse/tests.rs`, TS-03): the `log` and `patch-plan`
  fixtures in §2 parse to the expected `History`/`StateFiles`; a missing/renamed field → `Environment`
  refusal, never a fabricated value; a `<none>` previous-ref-state parses as `None`.
- **Operation tests via `NullBackend`** (TS-02): extend the scripted backend with `history` /
  `block_state` / `refs` returns so the views are driven deterministically — a multi-block ref, a
  root-only ref, an empty/absent ref, a refusal (retired format), and a queue tier > 0.
- **Render tests via `TestBackend`** (TS-01): History renders the queue tier first, block rows
  newest-first with kind and counts, and a selected row; Block detail renders metadata + the state
  file list + the "awaits prikk support" note; a hostile file path renders inert (C-T2a); the ref
  picker overlay lists refs.
- **Navigation tests**: `dispatch` maps the new keys to `OpenHistory`/`OpenBlock`/`Back`/…; `Enter`
  pushes Block detail; `Esc` pops; the ref picker sets the focused ref and triggers a reload.
- Gates unchanged: `fmt --check`, `clippy -D warnings`, `test`.

---

## 7. Example (per project rules)

Extend `NullBackend` and add `crates/stikk-tui/examples/history_demo.rs`: the shell opening on
**History** against a scripted multi-block lineage (a Root + a couple of Normal blocks + a queue tier
of 2), navigable with no prikk and no repository. Update the crate's guide to mention it.

---

## 8. Acceptance criteria

1. `history`/`block_state` (and `refs`, if included) parse the real prikk output shapes (§2 fixtures);
   an unrecognized shape refuses, never fabricates.
2. History renders the queue tier + block lineage for the focused ref; `Enter` opens Block detail;
   `Esc`/`q` returns; a ref picker switches the focused ref and reloads.
3. Block detail shows metadata + the state file list + the explicit "per-patch inspection awaits prikk
   support (UD-09)" note. No faked patch detail anywhere.
4. Every repository-sourced string (ids, ref names, file paths) is rendered inert (C-T2a), asserted by
   a test.
5. No mutation, no new seam category beyond read-history/read-state, no repository write path, no key
   material.
6. `fmt` / `clippy -D warnings` / `test` green; a runnable `history_demo` example builds and runs
   against `NullBackend`.

---

## 9. Out of this increment, queued next

- **3b — Patch detail** (`FR-030`) + patch-id enumeration + diff-aware search (`FR-013`): all blocked
  on **UD-09** (prikk exposing per-patch content / `log --format json`). File the prikk issue; build
  when it lands.
- **Increment 4** — the refusal-explanation overlay content + the witness/finding glossary
  (`FR-110/111`) and the command palette (`FR-125`), which History's failures and the ref picker make
  more valuable.

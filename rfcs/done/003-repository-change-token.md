# RFC 003 — Repository change-token signal set

**Status.** Implemented (0.4.0 candidate; on `main` 2026-09-05, reviewed and approved) — **scope split
at acceptance: built the change token, deferred the fingerprint.** First increment of 0.4.0.
**Deferred, carried forward:** the **repository fingerprint** (decision 4) — prikk deliberately has no
repository identity, deriving one means walking the entire sealed history, it would not exist for a
repository with no sealed blocks, and `INV-5` already carries the protection it was meant to add.
Revisit only if a need appears that path-keying plus `INV-5` cannot meet. Handoff:
[`../handoffs/003-repository-change-token/change-token-handoff-v1.md`](../handoffs/003-repository-change-token/change-token-handoff-v1.md).
Originally proposed
**Tracks.** The concrete prikk-observable signals stikk uses to detect that a repository changed, so
cached derivations and armed previews can be invalidated. Referenced by the data model (`LC-4`,
`LC-9`, `LC-10`) and internal design (`change_token.rs`, `CT-05`) as deferred.
**Touches.** `stikk-core` (`change_token.rs`, `cache.rs` — the derivation layer was folded into
`stikk-core`; the `stikk-view` crate the internal design named was never built, see `stikk-04` MOD-04),
the operation layer's refresh step (`refresh.rs`), `stikk-state`'s `RepositoryHandle` (which carries no
fingerprint yet), and the seam if a dedicated `change_token()` request is added (`SEAM-02`).
**Blocks:** roadmap increment 6 — `SessionState` is keyed by the repository fingerprint (`DM-02`/`LC-9`)
this RFC must settle.

## Summary

stikk holds no repository authority (`INV-1`): every derived view and cache is valid only as long as
the repository has not changed underneath it. The data model calls the freshness primitive a
"change token" (`LC-4`) and requires that any view or cache computed under an older token be treated
as stale — reads refresh, armed mutation previews invalidate (`CT-05`). What the token is *made of*
was deferred. This RFC proposes the signal set, under one hard constraint: it must be **cheap** (it is
checked on every refresh and around every operation) and it must go **through prikk's public surface**
(`CON-1`) — stikk never reads `.prikk/` directly to compute it.

## The problem

Two distinct jobs share the primitive (`LC-9` names both):

1. **Same-repository change detection** — has anything about *this* repository changed since we last
   looked? Drives cache validity (`LC-10`) and the passive "repository changed — refreshed" notice
   (`OP-04`), and the preview↔execute binding that makes "another writer moved the ref between your
   preview and your click" safe (`OPL-02`).
2. **Different-repository detection** — is the repository at a remembered path actually the *same*
   repository, or a different one now occupying that path (so stale session/cache must be discarded,
   not misapplied)? This is the `RepositoryHandle` fingerprint (`DM-02`).

Both must be computed from what prikk exposes, not from `.prikk/` bytes stikk is forbidden to read.

## Proposed signal set (to validate, not yet fixed)

A change token is a small tuple derived from cheap prikk observations. Candidate signals, cheapest
first:

- **Ref-pointer state** — the current RefState ids of the repository's refs (via a read the seam
  already needs for the History and Orientation views). A ref advancing is the change stikk most needs
  to catch (it is what invalidates an armed merge/checkout preview).
- **Active-queue extent** — the queued-patch count / WAL extent (already read for Orientation). A
  commit between a preview and its execution changes this.
- **Worktree marker** — prikk's own dirty marker, when a view depends on worktree-vs-baseline state.

The token is the combination of whichever of these a given view depends on; a view caches under the
token of *its* inputs, not a global one, so an unrelated change does not needlessly invalidate it.

The repository **fingerprint** (`LC-9`) is a separate, coarser derivation: stable identity signals
that recognize a moved repository and distinguish a different one at the same path. Proposed source:
the genesis/root block identity plus the repository's own layout signals as prikk reports them — never
a hash of `.prikk/` bytes stikk read itself (`INV-1`). The exact signals are the main thing this RFC
must settle with prikk's read surface in front of it.

## The prikk-side question

prikk does not today expose a single cheap "has anything changed" endpoint. Two options:

- **Compose the token from existing reads** (ref pointers, queue extent, marker) that stikk performs
  anyway — no prikk change needed, at the cost of a few reads per refresh. Proposed for v1.
- **Ask prikk for a dedicated change signal** (a future prikk feature) — cheaper, but it is a
  prikk-side dependency, so it is recorded here as a possible upstream ask, not assumed
  (cf. requirement `UD-02`'s discipline).

## Consequences

- `cache.rs` (`LC-10`) becomes implementable: an entry is usable only if its key matches, its stamped
  token equals the current token, and the prikk version is unchanged.
- `OPL-02`'s preview↔execute binding becomes concrete: the preview stamps the token it was computed
  under, and execution refuses if the current token differs.

## Open questions — ruled at acceptance (2026-09-05)

**Q1 — is composing the token from existing reads cheap enough, or is a prikk-side signal needed?**
**Ruled: compose from existing reads; ask prikk for nothing.** The token is built from
`refs()` and `orientation()` — two calls stikk *already makes* at open and on every refresh — so it
costs **zero additional process spawns**. No upstream ask is filed, and none should be until something
is measured.

**Q2 — which signals identify a repository (fingerprint) versus its state (token)?** This turned out to
be the wrong question, and answering it properly changes this RFC's scope. See below.

## The finding that splits this RFC

This RFC proposed deriving a repository **fingerprint** from "the genesis/root block identity plus the
repository's own layout signals as prikk reports them." Checking prikk before designing against it —
the habit RFC 009 cost us a release to learn — shows that premise is wrong in three ways:

1. **prikk deliberately has no repository identity, and says so as a security property.**
   `trust-threat-model.md:123`: *"Repositories are anonymous. Identity lives in signer keys and in
   patch ids — never in a repository."* And `:143-147`: *"There is no repository identifier to spoof,
   no peer to impersonate, and no origin field a receiver could be fooled by, because none of the three
   exists. **That is a security property this design has, not a gap in it** … the design never creates
   one to begin with."* `non-goals.md:29` repeats it. stikk may still derive a *client-side* identity
   from a block id — identity does live in ids — but it must be documented as **stikk's own
   convenience**, never as a repository fact, and stikk must not build a mechanism that quietly wants
   prikk to grow one.
2. **It is expensive.** The root block is the *oldest* entry in `prikk log`, which prints newest-first
   with no `--reverse` and no "oldest" query. Deriving it means walking the entire history — at
   `NFR-P03`'s Tier-2 ceiling of 25,000 blocks, roughly a quarter of a million lines parsed, on open,
   for an identity check.
3. **It does not exist for every repository.** A repository with no sealed blocks has no root block, so
   the fingerprint would be `None` for exactly the repositories a user is most likely to be creating.

**And it is not load-bearing.** `INV-5` already requires that *every* stored reference is re-resolved
against prikk on load and that a miss degrades to a default; `LC-6` discards a focused ref prikk no
longer has. So stale `SessionState` applied to a different repository is already defanged — the
fingerprint is defence in depth behind a control that exists, not the control itself.

## Decisions

1. **Build the change token** (`LC-4`, `CT-05`, `OPL-02`). This is what 0.4.0's preview↔execute binding
   requires and it is cheap.
2. **The token is coarse and global, not per-input.** This RFC originally proposed per-view tokens so an
   unrelated change would not invalidate a view. Ruled against: over-invalidation costs a re-read the
   user barely notices; under-invalidation is unsafe. Start simple and strictly safe (`AR-05`'s balance
   rule); refine only if measured nuisance justifies it.
3. **Signal set: the ref pointers (`name → RefState id`) plus the queued-patch count and its target
   ref.** Both come from `refs()` and `orientation()`, already called at open and refresh — zero extra
   spawns. **The worktree marker is deliberately excluded**: it would need a `worktree-status` spawn per
   token, and the Changes view already carries its own worktree data, so a commit preview built from it
   is self-freshening.
4. **Defer the repository fingerprint**, and correct `DM-02`/`LC-9` to say so rather than leaving the
   data model describing a field that does not exist. `RepositoryHandle` keys session state by
   **canonical path** in the meantime, with `INV-5` carrying the protection.
5. **No upstream ask.** Unlike `UD-09`, this is not a gap to file against prikk — it is a property prikk
   intends. Recording it here so nobody later "discovers" the absence and proposes fixing it.

## Consequences

- `OPL-02`'s preview↔execute binding becomes implementable, which unblocks every mutation in 0.4.0.
- `LC-10`'s cache validity becomes implementable on the same primitive.
- The data model stops promising a fingerprint field that was never built and would have been expensive
  and largely redundant if it had been.
- stikk's design record now states that prikk's repository anonymity is intentional — a thing worth
  knowing before someone designs against it a second time.

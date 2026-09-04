# RFC 003 — Repository change-token signal set

**Status.** Proposed
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

## Open questions

- Is composing the token from existing reads cheap enough at Tier-2 scale (`NFR-P03`), or does a
  dedicated prikk signal become necessary? Measure before asking prikk for anything.
- Exactly which signals identify a *repository* (the fingerprint) versus its *state* (the token) —
  they must not be conflated, and the fingerprint must be stable across ordinary history growth while
  the token changes with it.

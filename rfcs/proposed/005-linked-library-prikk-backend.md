# RFC 005 — Linked-library prikk backend

**Status.** Proposed
**Tracks.** Adding a second implementor of the `Prikk` seam trait that links prikk's crates directly,
alongside the CLI backend. Referenced by the internal design (`SEAM-01`, MOD-02 "deferred, not
designed here").
**Touches.** `crates/stikk-prikk` only (a new `lib_backend` behind the existing `Prikk` trait);
nothing above the seam.

## Summary

The seam was built so this decision could be made later without disturbing callers: everything above
`stikk-prikk` depends on the `Prikk` trait, not on how prikk is reached (`AR-02`). The CLI backend
ships first for good reasons (`SEAM-01`); a linked-library backend — calling prikk's crates in-process
instead of shelling out — is an option for later. This RFC records when it becomes worth doing, what
it would cost, and why it must not be done yet.

## The CLI backend is right for now

Recorded so the trade is explicit (`SEAM-01`):

- It uses only prikk's **public, stable-ish CLI** — the surface prikk commits to, versioned and
  gated (`SEAM-05`).
- prikk's **library crates are pre-1.0** and state their APIs "may change without notice" — linking
  them now means tracking churn against an unstable surface (`UD-02`, `ASM-1`).
- The costs the CLI backend pays — process spawn overhead, and parsing output that is machine-readable
  only for `verify` (`UD-02`) — are real but bounded, and the parse surface is confined and
  golden-fixture-tested (`SEAM-03`).

## When the library backend becomes worth it

Two conditions, both of which are prikk's to reach, not stikk's:

1. **prikk's libraries stabilize** (a 1.0 or a stated stability commitment) — so linking them is not
   chasing a moving target.
2. **Performance demands it** — profiling shows process-spawn or output-parse cost dominating at a
   scale users hit (`NFR-P03`). Until measured, this is speculation, and stikk does not add a
   dependency on speculation.

## What it would cost and gain

- **Gain:** typed returns instead of parsed output (removes the `UD-02` fragility and the
  golden-fixture maintenance), and lower per-call overhead.
- **Cost:** stikk links prikk's crates, enlarging its own dependency graph and coupling stikk's build
  to prikk's; the version gate (`SEAM-05`) shifts from "which CLI output shape" to "which crate API",
  a harder compatibility surface.
- **Neutral by design:** because the seam trait is the whole contract (`SEAM-02`), adding
  `lib_backend` disturbs no operation, view-model, or frontend. The two backends can even coexist,
  selected at build time or runtime.

## Consequences

- No action now. This RFC exists so the option is designed *before* it is reached, per the RFC
  policy's own reasoning ("easier to make before an importer exists than after").
- When accepted, it becomes a `stikk-prikk`-local change: one new module implementing `Prikk`, one
  backend-selection point, and a version gate keyed on crate API rather than CLI shape.

## Open questions

- Backend selection: build-time feature, runtime flag, or both? Proposed: a build feature (`cli` vs
  `lib`) with `cli` default, decided when the RFC is accepted.
- Does linking prikk re-introduce anything stikk deliberately avoids (e.g. pulling prikk's own
  dependencies into stikk's audited surface)? Evaluate prikk's public crate graph at acceptance time,
  not now.
- Does the CLI backend stay supported after a library backend lands (for users who have `prikk` but
  not a matching source build)? Proposed: yes — keep both; the CLI backend is the portable default.

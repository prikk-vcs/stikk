# RFC 011 — Pre-1.0 API stability, and when `#[non_exhaustive]` is worth its cost

**Status.** Implemented (adopted and in force; both release-prep handoffs — 0.2.0 and 0.3.0 — have shipped)
— originally accepted (2026-09-04) — record the compatibility policy stikk's published crates actually
follow, decide **against** a blanket `#[non_exhaustive]` on data structs for now, and make it a
**1.0-readiness task** instead. Written because the 0.2.0 release forced the question and the answer
was not obvious in either direction. Handoff:
[`../handoffs/011-pre-1-0-api-stability-policy/release-prep-handoff-v1.md`](../handoffs/011-pre-1-0-api-stability-policy/release-prep-handoff-v1.md).

> **Decision 1 reverses a recommendation the owner had already authorized.** I advised adding
> `#[non_exhaustive]` "while the door is open"; measuring the codebase afterwards (75 cross-crate
> construction sites, 13 in the runnable examples) and re-reading the ROADMAP's own "treats its own
> APIs as unstable" changed the answer. It is accepted as the *recommended* course so the release is
> not blocked, and is **the owner's to override** — doing it anyway is a separate increment, not a
> change to this one.
**Tracks.** Versioning and compatibility of the six published crates (`NFR-R03`, `CON-5`, the ROADMAP's
"Releases and versioning"). Not a feature.
**Touches.** No code if accepted as recommended. The decision governs `stikk-model`, `stikk-prikk`,
`stikk-state`, `stikk-core`, `stikk-tui` public types, and the release-version rule the owner applies.

## Summary

RFC 009 added public fields to five published structs (`Handshake`, `Orientation`, `WorktreeStatus`, `OrientationView`, `ChangesView`). That made 0.1.1 the wrong version — for a `0.x` crate the
**minor** is the breaking position, and `^0.1.0` resolves `0.1.1`, so anyone constructing those structs
would break on a "patch". 0.2.0 is correct, and that part is settled.

The follow-on question is whether to add `#[non_exhaustive]` so future field additions stop being
breaking at all. My first recommendation was yes — "do it while the door is open." Checking the
codebase and the project's own stated policy changed the answer. This RFC records why, so the
reasoning is available the next time it comes up rather than re-derived.

## The findings

1. **Every `#[non_exhaustive]` in the workspace is on an enum; not one struct carries it.** Six enums
   have it (`StikkError`, `RequestCategory`, `Presentation`, `Target`, `NextTarget`,
   `OperationContext`); thirty-one public structs do not. That looks like an oversight and is closer to
   a coherent instinct: for an **enum**, adding a variant silently breaks every downstream `match`, and
   `StikkError`'s doc comment records that as "a lesson from the 2026 prikk audit." For a **struct**,
   adding a field breaks only *construction*, which is a narrower and louder failure.
2. **The cost is not small.** `#[non_exhaustive]` forbids struct-literal construction outside the
   defining crate. The workspace has **75 such cross-crate sites** today, **13 of them in the four
   runnable examples** — which exist to be read as documentation (`orientation_demo`, `history_demo`,
   `explanation_demo`, `changes_demo`). Converting them means constructors or builders for ~11 types
   and making the demos markedly more verbose to no reader's benefit.
3. **It would foreclose an out-of-crate `Prikk` implementation.** The seam's response types
   (`Orientation`, `History`, `BlockRow`, `StateFiles`, `RefEntry`, `WorktreeStatus`, `WorktreeEntry`,
   `Handshake`) must be *constructed* by any backend. Marking them non-exhaustive means no one outside
   `stikk-prikk` can implement the trait. That may well be the right boundary — `AR-02` says the seam is
   the only door and RFC 005 puts the second backend *inside* `stikk-prikk` — but it should be a stated
   decision, not a side effect of a compatibility tweak.
4. **The project already declared its position.** The ROADMAP states stikk "stays pre-1.0 too and
   **treats its own APIs as unstable**." Under that policy, a field addition riding a minor bump is the
   system working, not a failure of it. `#[non_exhaustive]` is a tool for keeping an API you have
   promised to keep compatible — which is a 1.0 concern.

## Decisions

1. **No blanket `#[non_exhaustive]` on data structs before 1.0.** The cost (finding 2) is real and
   immediate; the benefit is deferred and, under the declared unstable-API policy (finding 4), small.
   This is the *balance, not over-abstraction* rule (`AR-05`) applied to compatibility machinery.
2. **Keep `#[non_exhaustive]` on every public enum, and add it to any new one.** Silent `match`
   breakage is the failure mode worth paying for, and the existing six are correct.
3. **Version honestly instead.** Until 1.0, adding a public field to a published struct is a
   **minor** bump (`0.x`). It is not a patch. The release checklist must ask "did any public struct gain
   a field?" — RFC 009 shipped that mistake as far as my own written recommendation, and only a semver
   check caught it.
4. **`#[non_exhaustive]` becomes a 1.0-readiness task**, together with the constructor/builder work
   and an explicit ruling on whether an out-of-crate `Prikk` implementation is supported (finding 3).
   Recorded here so 1.0 planning inherits it rather than rediscovering it.

## Open questions

- **Is an out-of-crate `Prikk` implementation a supported extension point?** Not urgent while the
  answer costs nothing, but it decides finding 3 at 1.0. *Leaning:* no — the seam's whole value is that
  it is the single audited door to prikk, and a third-party backend would be an unaudited one. If that
  is the ruling, say it in the trait's own docs rather than enforcing it accidentally through
  `#[non_exhaustive]`.
- **Should `stikk-model` be held to a stricter bar than the rest?** It is the shared kernel; a
  downstream frontend would depend on it most and construct from it least. It is the best candidate for
  early `#[non_exhaustive]` if we ever want a partial application.

## Consequences

- 0.2.0 ships without an API-machinery refactor, and the release-prep increment stays small.
- Every future field addition to a published struct costs a minor bump. That is acceptable pre-1.0 and
  is now a stated property rather than an accident.
- The 1.0 checklist gains a concrete, pre-analyzed item instead of a vague "review the API".
- The reasoning is on the record, so the next person to propose blanket `#[non_exhaustive]` starts from
  finding 2's number (75 sites, 13 in the demos) rather than from first principles.

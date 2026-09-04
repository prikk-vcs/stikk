# RFC 007 — The explanation &amp; discovery surface: error taxonomy → presentation, glossary, palette

**Status.** Implemented (0.1.0) — the **mechanism** of "where prikk refuses, stikk explains" shipped:
the class→presentation mapping (ER-03), the refusal overlay, the glossary asset, the session refusal
history, and the command palette, wired to the surfaces that exist today.
**Deferred, carried forward (not built by this RFC):** live renderers for `RoutedIntoView`
(Verify/Doctor) and `InConfirmation` (limits), which land with those operations; the **next-step
catalog and witness/finding glossary entries** for merge/checkout/seal/trust, each landing with the
operation that produces it; and **persisting the refusal history** with the private/ephemeral-session
gate (LC-8), which lands with the `stikk-state` session store. Handoff:
[`../handoffs/007-explanation-and-discovery-surface/explanation-surface-handoff-v1.md`](../handoffs/007-explanation-and-discovery-surface/explanation-surface-handoff-v1.md).
**Tracks.** The roadmap's "Next" increment 4 — the refusal-explanation overlay + witness/finding
glossary (`FR-110/111/112`) and the command palette (`FR-125`).
**Touches.** `stikk-prikk` (a confined, version-gated **classifier** for the 0/1-exit + message →
`StikkError` class — UD-05), `stikk-core` (the single class→presentation mapping — ER-03 — and an
operation registry for the palette), `stikk-model` (already carries the seven-class `StikkError`),
`stikk-tui` (the refusal overlay, the glossary/help browser, the palette), and a shipped **glossary
product asset** (DM-09). Nothing mutates; nothing below the seam's contract changes.

## Summary

The founding stance — *where prikk refuses, stikk explains* (FR-110) — is a product surface, not an
error path (NFR-U02 keeps the happy path clear of it, but the explanation surface is the reason the
tool is trustworthy). The design set already specifies it in full: the refusal overlay pattern
(TU-08), the class→presentation mapping (ER-03/OP-03), the glossary asset (DM-09), the session refusal
history (DM-06/FR-112), and the command palette (TU-07/FR-125). This RFC records **how much of it is
buildable now** — because most operations that *produce* rich refusals (merge, checkout, seal, trust)
do not exist yet — and the **one fragile decision** the surface forces: classifying a refusal when
prikk's exit codes collapse to 0/1 (UD-05).

## The findings that scope this increment

1. **The taxonomy is already built.** `stikk_model::StikkError` ships the seven CT-04 classes with
   verbatim-message preservation, `source()`, and `#[non_exhaustive]` (RFC-era work). ER-03 asks for a
   **single** class→presentation mapping in `stikk-core`; there is not one yet — the TUI currently
   routes every seam error into an ad-hoc `Notice` overlay (RFC 006, increment 3a). This increment
   replaces that with the real mapping.
2. **Exit codes collapse to 0/1** (audit UD-05). prikk does not hand stikk a machine-readable error
   class; stikk must **classify by message text + the operation's context** (which category was run).
   This is the same fragility class as the human-output parsers (UD-02) and belongs in the same place —
   confined to `cli_backend/`, version-gated, and **degrading to a generic `Refusal` that still shows
   prikk's verbatim message** on an unrecognized message (NFR-I03, RR-5), never a fabricated class.
3. **Most refusal *sources* do not exist yet.** FR-110's power is next-step options *that actually
   exist* — "inspect evidence", "choose another baseline", "open Trust &amp; Keys". Merge, checkout,
   seal, and trust are future increments. So the **next-step catalog** is populated now only for the
   classes reachable today (a read refusal, a lock conflict, a not-ready, an environment fault, an
   internal fault), with the registry shaped so a future operation registers its own next-steps
   without touching the overlay.
4. **The glossary's witness/finding entries mostly wait on their sources** (merge evidence → 12 witness
   kinds; verify → finding codes). The glossary **asset and its degradation path** (missing code →
   "no gloss yet — showing prikk's message only") are built and tested now; the **Git→prikk
   terminology mapping** — usable immediately in Help/Terminology — is seeded in full. Witness and
   finding entries are added with the increments that surface them.

## Decisions

1. **One class→presentation mapping in `stikk-core`** (ER-03, OP-03): a single pure function maps a
   `&StikkError` to a `Presentation` value — `RefusalOverlay` · `Banner{jump: LockInspector}` ·
   `InlineGuidance{toward: TrustAndKeys}` · `RoutedIntoView{Verify|Doctor}` · `InConfirmation` ·
   `PlainStatement` · `FaultScreen`. Both frontends render from this one value; it is tested per class
   in isolation (TS-05). For increment 4 only `RefusalOverlay`, `Banner`, `InlineGuidance`,
   `PlainStatement`, and `FaultScreen` have live renderers; `RoutedIntoView` and `InConfirmation` are
   defined and return their target, with rendering landing alongside Verify/Doctor and the mutation
   confirmations.
2. **Next-steps are structured and stikk-authored — never parsed from the refusal message** (C-T2b,
   C-T4c). The overlay's actionable entries come from `(error class, attempted operation)`, from a
   registry stikk owns; prikk's message is shown **verbatim and inert** as quoted content, visually
   distinct from stikk's chrome. A hostile refusal string therefore cannot forge a "next step" the
   user could act on. prikk's message is additive-glossed, never rewritten (ER-02).
3. **The classifier lives in the seam, beside the parsers** (UD-05, internal-design line 63):
   `cli_backend` maps the drained 0/1 exit + message into a `StikkError` class by a **version-gated,
   fixture-pinned** table; an unrecognized message becomes `StikkError::Refusal { message }` (verbatim,
   generic gloss) — the safe degradation, never a wrong specific class. This mirrors the parser
   discipline exactly.
4. **The glossary is a shipped product asset** (DM-09) keyed by prikk codes, compiled into stikk and
   versioned with the release. A code with no entry degrades to the verbatim-only presentation
   (NFR-I03, RR-5); the message is never hidden. Increment 4 seeds the terminology mapping in full and
   the code-entry mechanism with a representative sample; witness/finding entries follow their sources.
5. **The command palette is backed by an operation registry** (TU-07/FR-125): every view/operation
   registers a name, binding, and required capability; entries the session cannot perform stay visible
   but **disabled with the reason** (FR-104). Increment 4 populates the registry with what exists
   (Orientation, History, ref picker, refresh, glossary/help, quit, theme where applicable) and is the
   spine future operations register into — the mechanical guarantee that palette parity is automatic.
6. **The session refusal history is in-memory this increment** (DM-06/FR-112): a capped ring of the
   session's refusals (verbatim message, class, attempted operation, capture time), revisitable from a
   key/overlay. **Persistence is deferred** (it needs the private/ephemeral-session gate, LC-8, wired
   through `stikk-state`); in-memory trivially honours LC-8 by persisting nothing.

## Key bindings (align to TU-05, evolving 3a's set)

- **`?`** grows from the 3a help card into the **Glossary / Help** browser (terminology mapping, key
  reference, and — as they arrive — witness/finding entries).
- **`:`** opens the **command palette** (new).
- Any refusal **auto-opens the refusal overlay** (TU-04 automatic transition), replacing 3a's `Notice`
  for `refusal`-class errors; the other classes route per decision 1.

## Upstream dependency

No new one. This increment is a consequence of the existing **UD-05** (exit codes collapse to 0/1):
were prikk to emit a machine-readable error class (or `--format json` errors), the classifier's
message-pattern table would shrink to a mapping over stable codes. Recorded here so the classifier's
fragility is a stated property, not a surprise; it degrades safely (decision 3) until then.

## Open questions

- **Persist the refusal history?** *Ruled for now:* in-memory only; persistence lands with the
  `stikk-state` session store and the LC-8 private-session gate, so the privacy control is built with
  the persistence, not bolted on after.
- **Where do next-step *actions* execute from the overlay?** *Ruled:* increment 4's next-steps are
  navigational (open a view, refresh, retry-after-your-action) — never a mutation, never an auto-retry
  (NFR-S04/C-E1). Mutating next-steps arrive only with the operations that own them, each behind its
  own preview+confirmation.
- **Glossary format on disk.** *Ruled:* a compiled-in asset (a static table or an embedded data file),
  not user-editable and not read from the repository or a network; versioned with the release
  (NFR-S07). Exact file shape is program-design detail in the handoff.

## Consequences

- `stikk-core` gains the one error-presentation mapping (ER-03) and an operation registry; `stikk-tui`
  gains the refusal overlay (TU-08), the glossary/help browser, and the palette (TU-07); the seam gains
  a classifier (UD-05). The 3a `Notice` overlay is retired for refusals in favour of the real pattern.
- The explanation surface is exercised against the refusals stikk can actually produce today (a bad
  ref, a retired format, a lock held, a not-ready readiness, an unparseable output) — real coverage,
  not stubs — and is structurally ready for merge/checkout/seal/trust to plug their next-steps and
  glossary entries in without touching the overlay.
- Verbatim truth (ER-02/C-T4c) and inert rendering (C-T2a) extend to refusal text; next-steps being
  stikk-authored (C-T2b) closes the "hostile message forges an action" path before any mutating
  operation exists to be tricked into.

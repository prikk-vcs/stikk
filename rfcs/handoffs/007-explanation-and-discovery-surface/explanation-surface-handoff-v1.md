# Handoff — The explanation &amp; discovery surface (v1)

**Companion to:** RFC 007 (Accepted 2026-09-02). Inherits its state.
**Realizes:** ROADMAP "Next" increment 4 — the refusal-explanation overlay + witness/finding glossary
(`FR-110/111/112`) and the command palette (`FR-125`).
**Design items:** `FR-110` (refusal carries verbatim + gloss + real next-steps), `FR-111` (glossary),
`FR-112` (session refusal history), `FR-125` (palette); `TU-07` (palette), `TU-08` (refusal overlay
pattern), `TU-05` (bindings), `ER-01…04` (error taxonomy + verbatim + centralized mapping),
`OP-03`/`CT-04` (class → presentation), `DM-06` (refusal history), `DM-09` (glossary asset), `UD-05`
(classify 0/1-exit); and the existing shell/overlay/inert-text from RFC 001 and the view stack from
RFC 006.

This is the program design and decision record for the increment. **Implementation, tests, and the
example follow it.** Where this handoff and RFC 007 or the design set disagree, the RFC/design wins and
this handoff is corrected first.

---

## 1. Scope

**In:**
- **One class→presentation mapping** in `stikk-core` (`present`): `&StikkError` → a `Presentation`
  value both frontends render from (ER-03).
- A **seam classifier** (`stikk-prikk`): the drained 0/1 exit + message → a `StikkError` **class**,
  version-gated and fixture-pinned, degrading to a verbatim `Refusal` on an unknown message (UD-05).
- The **refusal overlay** (`stikk-tui`, TU-08): ① prikk's message verbatim &amp; inert ② a plain-language
  gloss ③ stikk-authored next-steps that exist ④ glossary links for any named code ⑤ kept in session
  refusal history.
- A **glossary / help browser** (`stikk-tui`) bound to the **glossary asset** (`stikk-core`
  `glossary`, DM-09): the Git→prikk terminology mapping (seeded in full now), the key reference, and
  the code-lookup mechanism with its missing-code degradation.
- A **session refusal history** (`stikk-core` `RefusalHistory`, in-memory ring — DM-06/FR-112),
  revisitable from a key/overlay.
- A **command palette** (`stikk-tui`, TU-07) backed by an **operation registry** (`stikk-core`): every
  view/operation by name + binding + required capability; unavailable entries visible-but-disabled with
  a reason (FR-104).
- The **next-step catalog** for the classes reachable today (read refusal, lock-conflict, not-ready,
  environment, internal).

**Out (do not build here):**
- Live renderers for `RoutedIntoView` (Verify/Doctor don't exist) and `InConfirmation` (no mutation
  yet): the mapping **returns** these variants, but their rendering lands with those increments.
- Next-steps for merge/checkout/seal/trust and their **witness/finding glossary entries** — each lands
  with the operation that produces it; the registry and the glossary accept them without overlay
  changes.
- **Persisting** the refusal history and the private/ephemeral-session gate (LC-8) — deferred with the
  `stikk-state` session store (RFC 007 decision 6). In-memory only now.
- Any mutation, any auto-retry, any lock clearing (NFR-S04). Next-steps here are navigational only.

---

## 2. The class → presentation mapping (`stikk-core`, ER-03/OP-03)

A new module `present` with **one** pure function and its result type. This is the whole of ER-03: the
single place class becomes presentation, so the TUI and the future GUI cannot diverge (TS-05).

```rust
/// Where an error is shown (OP-03). Frontend-agnostic; the renderer switches on it.
pub enum Presentation {
    /// A full refusal overlay: verbatim message + gloss + next-steps + glossary links (TU-08).
    RefusalOverlay(RefusalCard),
    /// A non-modal banner with a jump target (lock-conflict → Lock inspector, FR-102/FR-106).
    Banner { message: String, jump: Option<Target> },
    /// Inline guidance toward a surface (not-ready → Trust & Keys, FR-104).
    InlineGuidance { detail: String, toward: Target },
    /// Routed into a content view, never a popup (integrity-finding → Verify/Doctor, OP-03).
    RoutedIntoView { target: Target, message: String },
    /// Belongs in the pre-execution confirmation, not after failure (limits, C-D2a).
    InConfirmation { message: String },
    /// A plain statement with the failing detail + original message (environment, NFR-I03).
    PlainStatement { detail: String, original: Option<String> },
    /// A fault screen: repository untouched, session preserved, read-only continuation (ER-04).
    FaultScreen { detail: String },
}

/// The content of a refusal overlay (FR-110). Every field is stikk-owned or verbatim prikk — never a
/// next-step parsed out of the message (C-T2b).
pub struct RefusalCard {
    pub verbatim: String,               // prikk's message, shown inert (C-T2a), copyable
    pub gloss: Option<String>,          // plain-language explanation; None ⇒ verbatim-only (RR-5)
    pub next_steps: Vec<NextStep>,      // stikk-authored, from (class, operation) — decision 2
    pub glossary_codes: Vec<String>,    // codes named in the message, resolvable in the glossary
}

pub struct NextStep { pub label: String, pub target: NextTarget }
/// Increment-4 next-steps are navigational only (NFR-S04): open a view, refresh, or "retry after you
/// resolve X yourself". No variant performs a mutation or an auto-retry.
pub enum NextTarget { OpenView(Target), Refresh, DismissAndResolveExternally }

/// A navigation target both frontends understand (a view id, an overlay). Extended as views land.
pub enum Target { Orientation, History, RefPicker, Glossary, /* future: LockInspector, TrustKeys, Verify, Doctor */ }

pub fn present(error: &StikkError, op: OperationContext) -> Presentation { /* the match */ }
```

**The mapping (per class, OP-03):**

| `StikkError` class | `Presentation` | Next-steps now |
|---|---|---|
| `Refusal` | `RefusalOverlay` | from `(op, class)` — e.g. bad ref → "Choose another ref" (`RefPicker`), "Refresh"; retired format → verbatim + "This is prikk's message; no stikk override exists" |
| `LockConflict` | `Banner{jump: None}` *(→ `LockInspector` when FR-102 lands)* | "Another writer is active — refresh when they finish" (`Refresh`) |
| `NotReady` | `InlineGuidance{toward: Glossary}` *(→ `TrustKeys` when FR-104 lands)* | inline: what's missing |
| `IntegrityFinding` | `RoutedIntoView{Verify}` *(renderer later)* | — |
| `Limits` | `InConfirmation` *(renderer later)* | — |
| `Environment` | `PlainStatement{original}` | verbatim + failing path |
| `Internal` | `FaultScreen` | "repository untouched; continue read-only" |

`OperationContext` is a small enum of what was attempted (`OpenRepo`, `LoadHistory`, `LoadBlockState`,
`ListRefs`, `Orient`, …), so the same class yields the right next-steps for the surface it came from.
It is set by `stikk-core` at each operation call site — the operation layer knows its own context.

---

## 3. The seam classifier (`stikk-prikk`, UD-05)

Today the CLI backend already turns a non-zero exit into `StikkError` somewhere generic. Make it a
**named, confined, version-gated** step, beside the parsers:

- New `cli_backend/classify.rs`: `fn classify(exit: ExitStatus, stdout: &str, stderr: &str, category:
  RequestCategory) -> StikkError`.
- Exit `0` with a parse already done ⇒ not an error (caller handles). Exit `1` (prikk's only failure
  code — UD-05) ⇒ inspect the **drained** message (EPIPE guard already drains, UD-04) against a
  **fixture-pinned table** of prikk message shapes: a lock message → `LockConflict`, a
  ref/path/format refusal → `Refusal`, a readiness message → `NotReady`, an I/O/permission/"not a
  prikk repository"/version-skew message → `Environment`.
- **Unrecognized message ⇒ `StikkError::Refusal { message }`** — verbatim, generic gloss downstream
  (RR-5/NFR-I03). Never a fabricated specific class, and never dropped.
- The `category` (from CT-03) disambiguates where the text alone is ambiguous (e.g. a bare "lock held"
  during a `read-*` vs a `publication` category), matching the design's "classify by message **+
  context**".

**Fixtures.** Capture the real prikk 0.27.1 failure messages that stikk can already provoke — a
non-existent ref, a retired/foreign directory ("not a prikk repository"), a lock held — verbatim into
`cli_backend/classify/tests.rs`, and assert each maps to the right class and that an invented message
degrades to `Refusal`. A prikk version that changes a message fails a fixture, not a user (SEAM-03).

---

## 4. The glossary asset (`stikk-core` `glossary`, DM-09)

- A compiled-in product asset (a static table, or an embedded data file parsed at startup — program
  choice), **not** user-editable, **not** read from the repository or a network (C-I3/C-E2). Versioned
  with the release (NFR-S07).
- **Keyed by prikk code** (witness kind, verify finding code). `fn lookup(code: &str) ->
  Option<&GlossaryEntry>`; a miss is the RR-5 degradation — the caller shows prikk's message only, with
  "no gloss yet for `<code>` — showing prikk's message" (NFR-I03). **The message is never hidden.**
- **Seed now, in full:** the **Git→prikk terminology mapping** (the §0 mapping — "revert" → rollback,
  "switch branch" → focused ref + checkout plan, "HEAD" → *there is none*, "staging" → *none*,
  "stash" → *none*, "amend" → *append a patch*, etc.), usable immediately in Help/Terminology and in
  copy that redirects Git-shaped expectations (external-design line 12).
- **Seed the code mechanism** with a representative sample entry + the miss path; the 12 merge witness
  kinds and the verify finding codes are added with FR-080 and FR-100 (they don't exist to surface yet).
- Each `GlossaryEntry`: `code`, `title`, `explanation` (plain language), and `see_also: Vec<code>`.
  Content is inert display text (rendered through `inert` at the frontend anyway).

---

## 5. The refusal overlay &amp; session history (`stikk-tui` + `stikk-core`)

**Refusal overlay (TU-08).** Replace 3a's `Overlay::Notice` (used for any error) with a
`Overlay::Refusal(RefusalCard)` that renders, top to bottom:

1. **prikk's message, verbatim**, in a clearly-quoted content block, **inert** (C-T2a) and visually
   distinct from stikk chrome (C-T2b) — a bordered/indented region labelled "prikk reported".
2. **The gloss** (stikk's plain-language explanation), clearly stikk's own voice, *below and separate*
   from the verbatim block — additive, never in place of it (ER-02/C-T4c). Absent ⇒ omit the gloss row
   (RR-5), never synthesize one.
3. **Next-steps** as a selectable list (`↑/↓`, `Enter` activates) — each a `NextStep`; activating
   `OpenView` pushes/*switches* to that view, `Refresh` re-runs the failed read, `DismissAndResolve…`
   closes with a one-line "resolve this yourself, then retry" (no auto-retry — NFR-S04).
4. **Glossary links** for any `glossary_codes`, opening the glossary browser at that entry.

The other `Presentation` variants render as: `Banner` = a one-line non-modal strip above the status
bar; `InlineGuidance` = a line in the affected view; `PlainStatement` = the existing failure body
(reused); `FaultScreen` = a full-screen "repository untouched, continue read-only" panel. `RoutedIntoView`
/ `InConfirmation` return their target but have no renderer yet (scope §1).

**Session refusal history (DM-06/FR-112).** A `stikk-core` `RefusalHistory`: a capped ring (e.g. 50)
of `{verbatim, class, operation, captured_at}`. The app appends on every refusal; a binding (propose
`R` — "recent refusals", distinct from `r` refresh) opens an overlay listing them newest-first, `Enter`
re-opens that refusal's card. **In-memory only** — nothing persisted (LC-8 honoured trivially). Capture
time is stikk's own clock stamped on stikk's record (never fabricated repository time — C-R1).

---

## 6. The command palette &amp; operation registry (`stikk-tui` + `stikk-core`, TU-07/FR-125)

- **Registry in `stikk-core`:** a static list of `Command { id, name, binding, category, min_capability
  }`. Each existing view/action registers one entry: Orientation, History, Ref picker, Refresh,
  Glossary/Help, Recent refusals, Quit (and Theme toggle where the config allows). This is the spine —
  when a future operation lands, it adds a registry entry and appears in the palette automatically
  (the mechanical parity guarantee, decision 5).
- **Palette overlay in `stikk-tui`** (`:`): a filter line + a fuzzy-matched list; each row shows the
  command name, its binding, and — if the session's capability is below `min_capability` — a **disabled**
  style with the reason ("MAINTAINER key not ready — see Trust &amp; Keys"), the entry still **visible**
  (FR-104/TU-07). `Enter` runs an enabled command's action; a disabled row is inert. Fuzzy match over
  refs and recent object ids is deferred until there are many (History already has the ref picker).
- **Capability** comes from the loaded `OrientationView.capability` (already computed, increment 1);
  read-only/viewer sessions see mutation entries disabled — though none exist yet, the *mechanism* is
  built and tested with the entries that do.

---

## 7. App / keys wiring (evolve 3a)

- **`keys::Action`** gains `OpenPalette`, `OpenGlossary` (rebind `?` from the help card to the glossary
  browser), `OpenRefusalHistory`, and palette/overlay navigation reuses the existing `Up/Down/Select/
  Back`. Dispatch stays context-free (3a discipline); the app resolves per top overlay.
- **`App`** routes every seam error through `stikk_core::present(err, ctx)` instead of building a
  `Notice`, and pushes the overlay/ banner/ inline/ fault the `Presentation` selects; it appends to
  `RefusalHistory` on refusal-class errors.
- **Auto-open on refusal** (TU-04): a refusal from any read operation opens the refusal overlay
  automatically — the 3a "history refusal → Notice" path becomes "→ present() → RefusalOverlay".
- **`:`** opens the palette; **`?`** opens glossary/help; **`R`** opens recent refusals. Update the
  status-bar hint and the help/glossary content to match.

---

## 8. Security surface (threat model `stikk-03`)

- **C-T2a — inert.** The verbatim refusal message, glossary content, and every code named go through
  `inert` before a cell. New hostile-input tests: a refusal message carrying an escape sequence renders
  inert in the overlay.
- **C-T2b — chrome vs content.** Next-steps are **stikk-authored**, derived from `(class, operation)`,
  never parsed from the message; the verbatim block is visibly a *quoted content region*, not stikk's
  own action list. Test: a refusal message containing text that looks like a "next step" or a fake
  confirmation produces **no** actionable entry from that text.
- **C-T4c / ER-02 — verbatim truth.** The gloss is a separate field shown below the verbatim message;
  a class with no gloss shows the message alone. Test: `present()` never returns a `RefusalCard` whose
  `verbatim` differs from the error's message.
- **NFR-I03 / RR-5 — degradation.** Unknown classifier message ⇒ `Refusal` verbatim; unknown glossary
  code ⇒ verbatim-only. Both tested.
- **NFR-S04 / C-E1 — no auto-retry.** No `NextTarget` performs a mutation or a silent retry; `Refresh`
  re-runs a **read** at the user's explicit activation. Test: the next-step set for a `LockConflict`
  contains no retry-the-mutation action.
- **DM-N1 — no secrets.** Nothing here reads key material; readiness stays presence-only (unchanged).

---

## 9. Decision notes (program-level; RFC 007 has the rationale)

1. **One `present()` in core, not per-frontend.** The GUI, when it lands, renders the same
   `Presentation` — parity is structural, not disciplined (ER-03).
2. **Classifier in the seam, degrading to `Refusal`.** Same home and same discipline as the parsers;
   the safe failure is "generic refusal, verbatim shown", never a wrong specific class (UD-05/RR-5).
3. **Next-steps from `(class, operation)`, never from the message.** Closes the hostile-message-forges-
   an-action path (C-T2b) before any mutating operation exists to be tricked (defence built early).
4. **Glossary seeded with terminology now; codes as their sources land.** The mechanism + degradation
   are the testable core; witness/finding entries without their producing operations would be untested
   fiction.
5. **Refusal history in-memory.** Persistence and its LC-8 privacy gate ship together later, so the
   privacy control is never an afterthought.

---

## 10. Test plan

- **TS-05 — presentation mapping:** `present()` asserted per `StikkError` class → the right
  `Presentation`; verbatim preserved; no-gloss path; next-steps come only from `(class, op)`.
- **Classifier (TS-03 discipline):** golden real prikk failure messages → correct class; invented
  message → `Refusal`; category disambiguation exercised.
- **Glossary:** known code → entry; unknown code → `None` (caller shows verbatim); terminology mapping
  present and complete for the seeded set.
- **Refusal overlay (TS-01, TestBackend):** renders verbatim + gloss + next-steps + glossary link;
  hostile escape in the message is inert; a message that mimics a next-step yields no action.
- **Refusal history:** append + cap + newest-first + re-open.
- **Palette:** lists registered commands with bindings; a below-capability entry is present-but-disabled
  with its reason; fuzzy filter narrows.
- **App wiring (NullBackend):** a scripted history refusal auto-opens the refusal overlay (not a
  `Notice`) and lands in the history; a scripted lock message becomes a banner; an environment fault a
  plain statement.
- **Gates:** `fmt` / `clippy --all-targets --all-features -D warnings` / `test` all green.

---

## 11. Example

`cargo run -p stikk-tui --example explanation_demo` (scripted `NullBackend`, no prikk, no repo):
open on Orientation; `:` shows the palette (with a mutation-ish entry disabled to demonstrate the
reason line); trigger a scripted **refusal** (e.g. open History on a ref the backend refuses) to
auto-open the refusal overlay with verbatim + gloss + next-steps + a glossary link; `?` opens the
glossary/terminology browser; `R` shows the session's refusals. Drives every surface this increment
adds against canned data.

---

## 12. Acceptance criteria

1. A single `stikk_core::present(&StikkError, OperationContext) -> Presentation` exists; every CT-04
   class maps per §2; verbatim message preserved (ER-02); tested per class (TS-05).
2. The seam classifier maps real prikk 0/1 failures to the right class and **degrades an unknown
   message to a verbatim `Refusal`** (UD-05/RR-5); fixture-pinned.
3. The refusal overlay shows verbatim (inert, quoted, distinct from chrome) + separate gloss +
   stikk-authored next-steps that exist + glossary links; **no next-step is derived from the message**
   (C-T2b) and none mutates or auto-retries (NFR-S04).
4. The glossary asset resolves seeded codes and the full terminology mapping, and degrades a missing
   code to verbatim-only without hiding the message (RR-5/NFR-I03).
5. The command palette lists registered commands with bindings; below-capability entries are
   visible-but-disabled with a reason (TU-07/FR-104).
6. The session refusal history captures and re-opens refusals, in-memory (FR-112/DM-06/LC-8).
7. Every repository/prikk-sourced string in these surfaces is inert (C-T2a).
8. `fmt` / `clippy -D warnings` / `test` green; `explanation_demo` builds and runs against `NullBackend`.

---

## 13. Out of this increment, queued next

- **Live `RoutedIntoView` / `InConfirmation` renderers** with Verify/Doctor (FR-100/101) and the first
  mutation's preview+confirmation (FR-120/121).
- **Merge/checkout/seal/trust next-steps** and the **12 witness kinds + verify finding codes** in the
  glossary — each with the operation that produces it.
- **Persisting the refusal history** and the **private/ephemeral-session gate** (LC-8) via
  `stikk-state`.
- **Palette fuzzy match over refs and recent object ids** once there are enough to warrant it.

# stikk — Internal Design Specification

| | |
|---|---|
| Document | stikk Internal Design (detailed design; white-box) |
| Version | v0.1 (draft for review) |
| Date | 2026-08-31 |
| Inputs | Requirements v0.2, External Design v0.2, Data Model & Lifecycle v0.1 (`stikk-05`), Threat Model v0.1 (`stikk-03`), prikk 0.27.1 reality, project rules (`.git-exclude/rules/`) |
| Scope | HOW stikk is built: crate/module decomposition, the operation layer, the prikk integration seam, the frontends, concurrency, error taxonomy, and testing strategy. Cites design-item IDs (BD/AC/CL/TU/GU/CF/FL/CT/OP), requirement IDs, and data IDs (DM/LC/INV). |
| ID scheme | `AR-` architecture · `MOD-` module/crate · `SEAM-` prikk integration · `OPL-` operation-layer · `FE-` frontend · `CC-` concurrency · `ER-` error handling · `TS-` testing |

Governing rules (`project-instructions-*.md`): Rust, 2024 edition, 2018+ module style (`foo.rs` + `foo/` subdir, no `mod.rs`); tests in sibling `tests.rs`/`tests/` modules, never `#[test]` inline in the implementation file; English only; design specs are the source of truth for tests (CON-6). Data-structure rule: long-term safety, maintainability, simplicity, balanced generality (INV-1…9 already encode this).

---

## 1. Architecture (AR-…)

### AR-01 — The layer cake

stikk is one workspace of five layers; dependencies point strictly downward (no cycles), mirroring the discipline stikk's own audit praised in prikk.

```
┌───────────────────────────────────────────────────────────┐
│  FRONTENDS         stikk-tui   │   stikk-gui                │  presentation only
├───────────────────────────────────────────────────────────┤
│  OPERATION LAYER   stikk-core  (the one shared operation    │  FR-123, CON-2
│                    set both frontends call; owns no I/O)    │
├───────────────────────────────────────────────────────────┤
│  DERIVATION &      stikk-view  (view-models, diff/tree      │  VW-*, DM-08
│  STATE             render, change-token, DerivedViewCache)  │
│                    stikk-state (config, sessions, exports)  │  DM-01..10, LC-*
├───────────────────────────────────────────────────────────┤
│  PRIKK SEAM        stikk-prikk (the ONLY code that talks to │  CON-1, SEAM-*
│                    prikk; hides the transport entirely)     │
├───────────────────────────────────────────────────────────┤
│  SHARED            stikk-model (id/ref/error types, the     │  ER-*, CT-04
│                    request/response category vocabulary)    │
└───────────────────────────────────────────────────────────┘
```

- **AR-02 — The seam is the only door to prikk.** Nothing above `stikk-prikk` knows *how* prikk is reached (subprocess vs. linked library — SEAM-01). This is the load-bearing decision of the whole design: it localizes CON-1, UD-02, UD-04, and the version-skew handling (NFR-R03) to one crate, and makes the two candidate transports swappable without touching the operation layer or frontends.
- **AR-03 — The operation layer owns no I/O and no widgets.** `stikk-core` is a pure orchestration layer: it turns a user intent into a sequence of seam requests + state reads/writes + view-model productions, applying the confirmation tiers (FR-121), preview-first rule (FR-120), and capability gating (AC-01…04). Both frontends drive the *same* `stikk-core` API — this is the mechanical guarantee behind TUI/GUI parity (FR-123); an operation present in one frontend only is impossible because neither frontend defines operations.
- **AR-04 — Frontends are thin.** `stikk-tui` and `stikk-gui` translate input events into `stikk-core` calls and render the view-models `stikk-core` returns. They hold no repository logic; they may hold widget state (scroll position, focus) that is never authority (INV-8).

### AR-05 — Why this shape (rule: balance, not over-abstraction)

The temptation is a generic "backend abstraction" mirroring prikk's object model. Rejected (DM-N2): mirroring prikk would invite treating stikk types as authority. Instead the seam exposes prikk's *operations* (verbs), not prikk's *storage* (nouns), so stikk can never accumulate a shadow object store. The one deliberate generality is the request/response **category** vocabulary (CT-03), shared by the seam and the operation layer, so a new prikk operation slots into an existing category with known idempotency/cancellability/lock semantics rather than inventing its own.

---

## 2. Crate & module decomposition (MOD-…)

Workspace members (Rust 2024, 2018+ module style throughout):

### MOD-01 — `stikk-model` (shared kernel, no I/O)
- `id.rs` — newtypes over prikk object ids/ref names (parse/validate/abbreviate; never fabricate).
- `error.rs` — `StikkError` enum with the CT-04 classes (`Refusal`, `LockConflict`, `NotReady`, `IntegrityFinding`, `Limits`, `Environment`, `StikkInternal`), each preserving prikk's verbatim message (NFR-I03, FR-110); implements `source()` and is `#[non_exhaustive]` (a lesson taken straight from stikk's audit of prikk's own `PrikkError`).
- `category.rs` — the `RequestCategory` enum (CT-03 nine categories) with each variant's declared `mutates`/`cancellable`/`lock` metadata as `const` data, so the operation layer reads policy off the type.
- `capability.rs` — `Capability` (Viewer/Author/Maintainer/Operator, AC-01…04) and the `readiness → capability` mapping.

### MOD-02 — `stikk-prikk` (the seam — SEAM-*)
- `lib.rs` — the `Prikk` trait (SEAM-02): one method per request category, taking typed requests, returning typed responses or `StikkError`. **This trait is the entire prikk contract**; everything above depends on the trait, not the implementation.
- `cli_backend.rs` + `cli_backend/` — the CLI-driving implementation (SEAM-03): spawns `prikk`, supplies args, **drains stdout/stderr fully before inspecting exit** (UD-04 EPIPE guard), classifies the 0/1 exit + message text into `StikkError` (UD-05), and parses output. Parsing is confined here and per-command: `verify --format json` is parsed as prikk's `verify-report-v1`; other commands' human output is parsed by **pinned, version-gated** readers that refuse rather than guess on an unrecognized shape (UD-02 discipline: never screen-scrape unpinned prose).
- `version.rs` — probes prikk's version at handshake (SEAM-05), gates the validated range (NFR-R03), and selects the parser generation.
- `env.rs` — reads `PRIKK_*` **presence** for readiness (LC-13, DM-N1); it is the only module allowed to look at those variables, and it never reads a `*_SEED` **value**, only whether it is set. Threat-model anchor (A-KEY).
- `null_backend.rs` (test only, sibling `cli_backend/tests.rs` style) — a scripted `Prikk` impl for deterministic operation-layer tests without a real repository (TS-02).
- *Deferred, not designed here:* a `lib_backend.rs` linking prikk crates directly. The trait (SEAM-02) exists precisely so this can be added later without disturbing callers; the choice is left to a follow-up RFC because prikk's libraries are pre-1.0 (UD-02, ASM-1).

### MOD-03 — `stikk-state` (stikk-owned durable data — DM-*, LC-*)
- `config.rs` (+ `config/`) — DM-01 load/validate/atomic-write, unknown-key preservation (INV-4, LC-1/2).
- `session.rs` — DM-04 SessionState, id-only anchors, focused-ref reconciliation (LC-6), private-mode gate (LC-8).
- `recents.rs` — DM-03.
- `handle.rs` — DM-02 RepositoryHandle + the fingerprint derivation (LC-9), which calls the seam only (never reads `.prikk/` — INV-1).
- `sync_progress.rs` — DM-05 (artifact *paths* only).
- `refusal_log.rs` — DM-06.
- `export.rs` — DM-10 ReportExport writer: temp-then-atomic-rename (LC-12), verbatim passthrough vs. stamped `stikk-export-v1` (INV-7). Applies the **redaction rule** stikk inherits from prikk (threat model C-I3, `trust-threat-model.md:210-211`): a stikk-authored export never contains blob bytes, raw span/replacement text, absolute host paths, or `.prikk` private paths; the same rule gates any diagnostic log stikk writes.
- `paths.rs` — resolves the CF-01 user-scope locations per platform; the one module that knows where stikk's files live, and it refuses any path inside a repository (INV-2 enforced in code).

### MOD-04 — `stikk-view` (derivation & rendering — VW-*, DM-08)
- `viewmodel.rs` (+ `viewmodel/`) — one module per VW-01…11 producing the view-model from seam responses.
- `diff_render.rs` — renders a prikk patch's operations as a human diff (FR-030); pure, testable against fixtures.
- `tree_render.rs` — state-tree rendering (FR-032).
- `change_token.rs` — LC-4 token acquisition + comparison; the shared staleness primitive (AR-05).
- `cache.rs` — DM-08 DerivedViewCache with the LC-10 validity rule and LRU bound; correctness never depends on it (INV-6).
- `glossary.rs` — DM-09 product-asset content, keyed by prikk codes with the missing-code degradation (NFR-I03).
- `size_guard.rs` — TU-11 oversized-renderable summary-then-expand (threat T-DO1).

### MOD-05 — `stikk-core` (operation layer — OPL-*)
- `op.rs` (+ `op/`) — one module per operation family (history, inspect, compare, work, refs, merge, rollback, exchange, trust, integrity, recovery), each a function taking typed intent + a `&dyn Prikk` + `&mut` state, returning a view-model or a staged preview.
- `confirm.rs` — the FR-121 tier machine and FR-120 preview-first enforcement (a mutating op cannot be executed without a matching confirmed preview token).
- `capability_gate.rs` — AC gating; every mutating op checks capability *and* re-checks readiness at the seam before executing (defense in depth with the seam).
- `refresh.rs` — the FR-106/LC-4 external-change reconciliation invoked around every operation.

### MOD-06 — `stikk-tui` (FE-*) and MOD-07 — `stikk-gui` (FE-*)
Frontends. Each: an input→intent mapper, a renderer per view (TU-01 / GU-01 inventories), the overlay layer (refusal TU-08, confirm TU-09, palette TU-07, glossary, background-ops), and i18n string binding (NFR-I01). They share nothing but `stikk-core` and `stikk-model` — no code path lets one frontend reach the seam or state except through `stikk-core` (AR-03).

### MOD-08 — `stikk` (the binary / launcher — CL-*)
Arg parsing for the launcher contract only (CL-01…07), the `config check`/`config path`/`--version` utilities, TTY detection (CL-06), and dispatch to the chosen frontend. Deliberately tiny; holds no operations.

---

## 3. The prikk seam in detail (SEAM-…)

### SEAM-01 — The transport question, isolated
Two candidate transports exist; the design commits to the **trait**, not the transport, and ships the CLI backend first:

| | CLI backend (v1) | Library backend (deferred) |
|---|---|---|
| Talks to | the `prikk` binary | prikk crates linked in |
| Honours CON-1 | yes (public CLI) | yes (public crate APIs) |
| Machine output | only `verify --format json` (UD-02) | typed returns |
| Stability | CLI command set (stable-ish) | pre-1.0, "may change without notice" |
| Risk | output parsing fragility (UD-02), EPIPE (UD-04), process overhead | churn against unstable APIs |
| Decision | **v1 default** — parse-surface confined + version-gated | follow-up RFC once prikk libs stabilize |

- **SEAM-02 — The `Prikk` trait** is the contract: nine methods matching CT-03's categories, plus `handshake()` (version + capability probe) and `change_token()`. Every method returns `Result<Response, StikkError>`; long/cancellable ones take a cancellation signal and report progress (OP-02). The trait is `Send + Sync`-bounded so the frontends can call it off the UI thread (CC-01).
- **SEAM-03 — Output-parsing containment.** All parsing lives in `cli_backend/parse/` with one module per prikk command and a **golden-fixture** test per parser generation (TS-03). A parser that meets an unrecognized shape returns `StikkError::Environment` naming the mismatch and the prikk version — it never fabricates a partial result (UD-02). When prikk grows `--format json` on more commands, a new parser generation is added beside the old; the old is retired only when the supported version floor rises.
- **SEAM-04 — Mutation calls are single-shot and pre-checked.** The seam never retries a mutating call (NFR-S04, INV via CT-05); the operation layer re-runs preconditions and lets the user decide. The seam surfaces `LockConflict` distinctly (FR-106) so the operation layer can present "another writer is active", not corruption.
- **SEAM-05 — Version gate.** `handshake()` records prikk's version; outside the validated range the seam still permits read-category calls where safe and refuses mutation categories, feeding OP-06's disabled-with-reason presentation.
- **SEAM-06 — No key material crosses the seam.** stikk passes prikk no seeds; prikk reads its own environment. The seam's `env.rs` reads only presence (LC-13). This is asserted by a test that greps the seam for any read of a `*_SEED` value (TS-04).

---

## 4. Operation-layer mechanics (OPL-…)

- **OPL-01 — Intent → plan → (preview) → confirm → execute → reconcile.** Every operation runs this pipeline: build a typed intent; ask the seam for a plan/preview where prikk offers one (checkout, deletion, merge, rollback, compaction — FR-120); render the preview and obtain the tiered confirmation (FR-121); execute the single seam mutation; reconcile affected view-models from fresh reads (FR-106). Read operations skip preview/confirm.
- **OPL-02 — Preview tokens bind preview to execution.** A confirmed preview yields a token stamped with the change-token (LC-4) it was computed under; execution refuses if the current change-token differs (CT-05) — the user re-previews. This is the mechanism that makes "another writer moved the ref between your preview and your click" safe (FL-08 step 6).
- **OPL-03 — The ceremonies are explicit state machines.** Seal (FL-06), rollback (FL-10), and the sync assistant (FL-12) are multi-step machines with persisted progress where the requirement says resumable (DM-05 for sync); each step is its own confirmed seam call; cancelling stops before the next call and reports completed steps (OP-02).
- **OPL-04 — Capability is checked twice.** Once in `capability_gate` for UI affordance (disable + reason), once at the seam boundary before the mutating call (readiness can lapse between render and click). Both derive from the same `readiness → capability` map (MOD-01).
- **OPL-05 — Errors never bypass the taxonomy.** Every fallible path returns `StikkError`; the operation layer maps class → presentation per OP-03 and never converts a refusal into a retry (NFR-S04).

---

## 5. Frontends (FE-…)

- **FE-01 — Shared view-model contract.** `stikk-core` returns view-models (`stikk-view` types); each frontend has a renderer per view-model type. Adding a view = one view-model + two renderers; the operation is defined once (AR-03).
- **FE-02 — TUI** (`stikk-tui`): a main loop with an immediate-mode or retained renderer (choice deferred to program design; either satisfies TU-01…12); the overlay stack (TU-02) is a view-model list rendered above the active view; keybindings resolve through the config's action-id map (CF-03); TTY-only (CL-06); degrades per TU-10.
- **FE-03 — GUI** (`stikk-gui`): a desktop toolkit (choice deferred to program design; must expose platform accessibility APIs — NFR-A02, and support i18n — GUI rule) rendering the same view-models into the GU-01 pane structure; drag-and-drop is constrained to the GU-05 legal targets by making drop handlers accept only the typed ids those targets expect (an illegal drag has no handler, so cherry-pick/stage gestures are structurally impossible, not merely hidden).
- **FE-04 — i18n.** All user-facing strings resolve through a locale catalog (en/ja/nb — NFR-I01); ids/paths/key-ids are never passed through translation (NFR-I02); prikk's verbatim messages are shown as-is with an optional localized gloss beside them (NFR-I03). Catalogs are data, hot-swappable at runtime (CF-05).
- **FE-05 — Progressive disclosure** (TU-12/GU-09) is a render-time depth parameter threaded from config/advanced-mode into each renderer; it changes *what fields render*, never which operations exist (NFR-U03).

---

## 6. Concurrency (CC-…)

- **CC-01 — One UI thread, seam off-thread.** Frontends keep the UI responsive (NFR-P01) by running seam calls on a worker; results post back to the UI thread. The `Prikk` trait is `Send + Sync`; view-models are owned values handed across the boundary (no shared mutable repository state exists to race — stikk holds none).
- **CC-02 — One mutating operation per repository in flight** (CT-05): the operation layer holds a per-repository mutation gate (an async lock in stikk's process); reads may run concurrently. This is *stikk-side* serialization for UX coherence; prikk's own locks remain the real guard (BD-04). The gate matters most for `bundle import`, whose object writes prikk performs **before** taking any lock (audit / `concurrency-locking.md:186-198`, "Known and accepted, not fixed here"): stikk's own gate ensures it never launches an import concurrently with another stikk mutation, and stikk never presents concurrent import as safe (threat model ASSUME-2).
- **CC-03 — No repository lock held across think-time** (NFR-R02): a seam mutation acquires and releases prikk's lock entirely within one call; stikk never opens a preview with a lock held. Cancellation between ceremony steps therefore never strands a lock.
- **CC-04 — Cache access** (DM-08) is single-writer/multi-reader within the process; a stale token discards rather than blocks (LC-10), so cache contention can never deadlock a render.
- **CC-05 — Kill-safety** (NFR-R01): all `stikk-state` writes are temp-then-atomic-rename; a crash mid-write leaves the previous file intact; a crash mid-operation leaves the repository to prikk's crash-safety and stikk re-reads on next launch.

---

## 7. Error handling (ER-…)

- **ER-01 — One error type, seven classes.** `StikkError` (MOD-01) carries the CT-04 class + prikk's verbatim message + structured context (path, object id, prikk version where relevant). `#[non_exhaustive]`, `source()` implemented — stikk applies to itself the fix its audit recommended for prikk's `PrikkError`.
- **ER-02 — Verbatim preservation is mandatory** (NFR-I03, FR-110): the seam captures prikk's message unmodified into the error; no layer above rewrites it; the refusal overlay shows it and adds the localized gloss separately (never in place of it).
- **ER-03 — Class → presentation is centralized** (OP-03): a single mapping in `stikk-core` decides overlay vs. banner vs. inline vs. routed-into-view, so presentation is consistent across frontends and testable in isolation (TS-05).
- **ER-04 — stikk-internal faults are contained** (OP-03): a panic/bug in stikk surfaces a fault screen stating the repository was untouched (true by INV-1), preserves session state (already flushed, LC-5), and offers read-only continuation. No stikk fault can express itself as a repository write, because only the seam writes and the seam only writes on an explicit confirmed mutation.

---

## 8. Testing strategy (TS-…)

Rule: design specs are the source of truth for tests (CON-6); test modules are siblings, never inline (`project-instructions-rust.md`).

- **TS-01 — Spec-traced unit tests.** Each `stikk-core` operation has tests asserting the FR/FL it realizes: preview-first (FR-120) — a mutating op with no confirmed preview token is rejected; tier enforcement (FR-121); capability gating (AC); verbatim-error preservation (ER-02).
- **TS-02 — Operation tests use the null backend** (MOD-02): a scripted `Prikk` returns canned responses/errors, so every FL branch (including refusals, FL-09) is exercised deterministically with no repository.
- **TS-03 — Golden-fixture parser tests** (SEAM-03): real prikk CLI outputs captured per command per version generation; a parser change that would misread a fixture fails. The `verify-report-v1` passthrough is byte-compared (INV-7/CT-02).
- **TS-04 — Security invariants as tests:** no seam read of a `*_SEED` value (LC-13/DM-N1, C-I1); `stikk-state::paths` refuses any repository-internal path (INV-2, C-E2 — *primary* control since prikk has no foreign-file backstop); an export/log carries no redaction-listed content (C-I3) and is always stamped or byte-verbatim (INV-7); Unverifiable never maps to a pass state (FR-035, C-T2c′). These encode the threat model's key controls (`stikk-03`) as gates, echoing prikk's own boundary-gate discipline.
- **TS-05 — Error-presentation tests** (ER-03): class → presentation mapping asserted per class.
- **TS-06 — Round-trip state tests** (LC-*): SessionState with a now-missing focused ref degrades to default (INV-5); a moved-repository fingerprint discards stale cache (LC-9/10); a corrupt state file resets to defaults (NFR-R01).
- **TS-07 — Integration (real prikk).** A small suite drives the real `prikk` binary through the CLI backend against a fixture repository (init→commit→seal→verify), mirroring prikk's own conformance approach; gated to the mutation platforms (NFR-T01). GUI testing uses the rules' `niri msg`/screenshot workflow (`project-instructions-gui-common.md`) at 1280×720.
- **TS-08 — Frontend parity test** (FR-123): a shared table of operations asserted present in both frontends' intent maps — a build-time check that neither frontend omits or invents an operation.

---

## 9. Traceability (design areas → requirements/design/data)

| Area | Realizes |
|---|---|
| AR-01…05 | CON-1/2, FR-123, BD-01…05, DM-N2, INV-1 |
| MOD-01…08 | the full requirement surface via the layers; `project-instructions-rust.md` module/test style |
| SEAM-01…06 | CON-1, UD-02/04/05, NFR-R03, NFR-S03, FR-106, CT-03, LC-13 |
| OPL-01…05 | FR-120/121, FR-106, AC-01…04, NFR-S04, CT-05, FL-06/08/10/12 |
| FE-01…05 | FR-123, TU-01…12, GU-01…09, NFR-A02, NFR-I01…I03, NFR-U01…U03, OS-1/OS-3 (structural absence in FE-03) |
| CC-01…05 | NFR-P01, NFR-R01/R02, CT-05, BD-04 |
| ER-01…04 | CT-04, FR-110, NFR-I03, OP-03, INV-1 |
| TS-01…08 | CON-6, every FR/FL under test; NFR-T01; GUI rule; threat-model controls (`stikk-03`) |

**Deferred to program design (next document):** TUI renderer style (immediate vs retained), GUI toolkit selection, the exact `stikk-export-v1` schema (promised by CT-02), the action-id catalog behind CF-03/TU-05, and the concrete change-token signal set (LC-4/LC-9) within the seam. All are bounded here; none reopen a requirement.

*End of Internal Design v0.1. Companion data document: `stikk-05` (Data Model & Lifecycle). Security document: `stikk-03` (Threat Model).*

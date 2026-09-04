# stikk RFCs

Design decisions for stikk, governed by the RFC lifecycle policy in
[`done/000-rfc-lifecycle-policy.md`](done/000-rfc-lifecycle-policy.md). Completed RFCs are never
deleted; they move to `done/`. **The folder is the source of truth for an RFC's state**, and the
`Status` field inside each file mirrors it.

stikk adopts the **five-folder variant** (`proposed/`, `accepted/`, `done/`, `archive/`, plus this
index): the maintainer's "accepted for implementation" is a distinct event from "implemented and
shipped", so an RFC moves `proposed/` → `accepted/` when its design is settled and an implementer may
start, then → `done/` when the work ships. Per the policy's granularity rule, an RFC moves to `done/`
when its **main design decision has shipped**, with anything deferred recorded in its Status field —
not held in `accepted/` until every last follow-up lands.

The initial design set (requirements, external design, internal design, data model, threat model)
lives in [`../docs/src/reference`](../docs/src/reference/) rather than as numbered RFCs — it was
produced as one coherent specification before the code. New, incremental design decisions are recorded
here as `NNN-slug.md`, numbered from `001`; numbers are stable forever and never reused.

## Proposed
_Open for review; an implementer should not start until an RFC moves to `accepted/`._

| ID | Title | Addresses |
|----|-------|-----------|
| 002 | [Action-id catalog and keybindings](./proposed/002-action-id-catalog-and-keybindings.md) | the stable action ids the config binds and the palette lists |
| 003 | [Repository change-token signal set](./proposed/003-repository-change-token.md) | cache validity, external-change / preview-staleness detection, and the repository fingerprint. **0.3.0, last** — it adds a method to the trait RFC 010 reshapes |
| 004 | [stikk-export report schema](./proposed/004-stikk-export-schema.md) | the versioned shape of stikk-authored report exports |
| 005 | [Linked-library prikk backend](./proposed/005-linked-library-prikk-backend.md) | a second seam backend, for when prikk's crates stabilize |
| 012 | [Post-0.2.0 correctness sweep](./proposed/012-post-0-2-0-correctness-sweep.md) | five review findings (read-only vs recovery, version-skew guidance, per-platform paths, `RefName` adoption, prikk 0.31's schema skew) **and the re-sequenced roadmap**. 0.3.0, second |

## Accepted
_Design settled; implementer may start; work has not yet shipped._

| ID | Title | Decision | Handoff |
|----|-------|----------|---------|
| 010 | [Off-thread seam and UI responsiveness](./accepted/010-off-thread-seam-and-ui-responsiveness.md) | worker thread + `Send + Sync` + handshake caching, delivering `NFR-P01`; **cancellation deliberately deferred** to `FR-100`. **0.3.0, first** — it reshapes the seam trait (2026-09-04) | [Off-thread seam](./handoffs/010-off-thread-seam-and-ui-responsiveness/off-thread-seam-handoff-v1.md) |
| 011 | [Pre-1.0 API stability policy](./accepted/011-pre-1-0-api-stability-policy.md) | a public struct gaining a field is a **minor** bump pre-1.0, so RFC 009 ships as **0.2.0**, not 0.1.1; blanket `#[non_exhaustive]` is deferred to 1.0 readiness (2026-09-04) | [0.2.0 release prep](./handoffs/011-pre-1-0-api-stability-policy/release-prep-handoff-v1.md) |

## Done (implemented)

| ID | Title | Shipped in | Deferred, carried forward | Handoff |
|----|-------|------------|---------------------------|---------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.1.0 (adopted, five-folder variant) | — | — |
| 001 | [Frontend toolkit selection](./done/001-frontend-toolkit-selection.md) | 0.1.0 | GUI toolkit undecided (own RFC when GUI work begins); TUI accessibility limitation to be documented | [TUI shell & Orientation](./handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md) |
| 006 | [History & inspection seam](./done/006-history-and-inspection-seam.md) | 0.1.0 | Patch detail (`FR-030`), patch-id enumeration, diff-aware search — all `UD-09` | [History & Block detail](./handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md) |
| 007 | [Explanation & discovery surface](./done/007-explanation-and-discovery-surface.md) | 0.1.0 | `RoutedIntoView`/`InConfirmation` renderers; merge/checkout/seal/trust next-steps + witness glossary; refusal-history persistence + `LC-8` gate | [Explanation surface](./handoffs/007-explanation-and-discovery-surface/explanation-surface-handoff-v1.md) |
| 008 | [Worktree changes & the Compare ceiling](./done/008-worktree-changes-and-the-compare-ceiling.md) | 0.1.0 | Compare (`FR-033`); per-file content diffs (`UD-09`); the `C` commit action; status-bar worktree marker. **Amended by RFC 009** | [Changes view](./handoffs/008-worktree-changes-and-the-compare-ceiling/changes-view-handoff-v1.md) |
| 009 | [prikk 0.30 re-baseline & parser fidelity](./done/009-prikk-0-30-rebaseline-and-parser-fidelity.md) | 0.1.1 candidate (on `main`) | real-binary integration suite (`TS-07`, threat-model `RR-9`); `FR-014`'s ref surface corrected but not completed (no `tag list` read); ref-name validation (`RefName` unused); richer `.prikkignore` surface | [Parser fidelity](./handoffs/009-prikk-0-30-rebaseline-and-parser-fidelity/parser-fidelity-handoff-v1.md) |

## Archive (withdrawn or superseded)

_None yet._

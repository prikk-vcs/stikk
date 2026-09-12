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
| 004 | [stikk-export report schema](./proposed/004-stikk-export-schema.md) | the versioned shape of stikk-authored report exports |
| 005 | [Linked-library prikk backend](./proposed/005-linked-library-prikk-backend.md) | a second seam backend, for when prikk's crates stabilize |

## Accepted
_Design settled; implementer may start; work has not yet shipped._

| ID | Title | Addresses | Handoff |
|----|-------|-----------|---------|
| 023 | [Three things stikk has and does not show](./accepted/023-what-stikk-has-and-does-not-show.md) | The prikk 0.28 Windows refusal stikk can explain and doesn't; four glossary explanations written, tested and unreachable; the AUTHOR key id open since RFC 014. **And `C-S2` — a control the coverage table lists as existing — has no implementation.** Q1 resolved by derivation, not transcription | [A: display gaps](./handoffs/023-what-stikk-has-and-does-not-show/a-display-gaps-handoff-v1.md) · B: pending |

## Done (implemented)

| ID | Title | Shipped in | Deferred, carried forward | Handoff |
|----|-------|------------|---------------------------|---------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.1.0 (adopted, five-folder variant) | — | — |
| 001 | [Frontend toolkit selection](./done/001-frontend-toolkit-selection.md) | 0.1.0 | GUI toolkit undecided (own RFC when GUI work begins); TUI accessibility limitation to be documented | [TUI shell & Orientation](./handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md) |
| 006 | [History & inspection seam](./done/006-history-and-inspection-seam.md) | 0.1.0 | Patch detail (`FR-030`), patch-id enumeration, diff-aware search — all `UD-09` | [History & Block detail](./handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md) |
| 007 | [Explanation & discovery surface](./done/007-explanation-and-discovery-surface.md) | 0.1.0 | `RoutedIntoView`/`InConfirmation` renderers; merge/checkout/seal/trust next-steps + witness glossary; refusal-history persistence + `LC-8` gate | [Explanation surface](./handoffs/007-explanation-and-discovery-surface/explanation-surface-handoff-v1.md) |
| 008 | [Worktree changes & the Compare ceiling](./done/008-worktree-changes-and-the-compare-ceiling.md) | 0.1.0 | Compare (`FR-033`); per-file content diffs (`UD-09`); the `C` commit action; status-bar worktree marker. **Amended by RFC 009** | [Changes view](./handoffs/008-worktree-changes-and-the-compare-ceiling/changes-view-handoff-v1.md) |
| 016 | [The seal ceremony](./done/016-the-seal-ceremony.md) | 0.4.0 candidate (on `main`) | the other seven gated operations; `MaintainerReadiness::Ready` unconstructible until prikk ships `trust maintainer check`; `Target::TrustKeys` has no renderer; **no `GlossaryEntry` explanation is rendered anywhere** (`FR-111`'s unbuilt half — four codes now ship with unreadable text) | [Seal ceremony](./handoffs/016-the-seal-ceremony/seal-ceremony-handoff-v1.md) |
| 022 | [Widening the real-binary suite](./done/022-widening-the-real-binary-suite.md) | 0.6.0 candidate (on `main`) | a gloss for prikk 0.28's Windows subdirectory refusal; an audit of every action's runtime; three `is_lock_conflict` clauses still source-read, each with a reachability note | [Suite widening](./handoffs/022-widening-the-real-binary-suite/suite-widening-handoff-v1.md) |
| 021 | [The prikk 0.38 re-baseline: `UD-09` retires, and a fabricated worktree entry](./done/021-prikk-0-38-rebaseline.md) | **0.5.0** | the four unblocked views, each its own RFC; `UD-10`; the symlink report; suite widening | [A: F0](./handoffs/021-prikk-0-38-rebaseline/f0-worktree-entry-scoping-handoff-v1.md) · [B: re-baseline](./handoffs/021-prikk-0-38-rebaseline/rebaseline-handoff-v1.md) |
| 020 | [MSRV 1.88 consistency & a supply-chain gate](./done/020-msrv-1-88-consistency-and-a-supply-chain-gate.md) | **0.5.0** | a `schedule:` trigger for the gate (owner's, deliberately open); `[graph] all-features = false` inert until a crate declares features | [Supply chain](./handoffs/020-msrv-1-88-consistency-and-a-supply-chain-gate/supply-chain-handoff-v1.md) |
| 019 | [The real-binary integration suite](./done/019-the-real-binary-integration-suite.md) | **0.5.0** | a `schedule:` trigger (owner's); the full `NFR-T01` platform matrix has never actually run — the Windows leg's `openssl` dependency is untested | [Integration suite](./handoffs/019-the-real-binary-integration-suite/integration-suite-handoff-v1.md) |
| 018 | [Post-0.4.0 correctness sweep](./done/018-post-0-4-0-correctness-sweep.md) | 0.4.1 | the Glossary's **wrap + scroll** as one increment — wrapping alone was tried and reverted (it turned truncated-but-present into absent) | [Correctness sweep](./handoffs/018-post-0-4-0-correctness-sweep/correctness-sweep-handoff-v1.md) |
| 017 | [prikk 0.33 re-baseline & classifier provenance](./done/017-prikk-0-33-rebaseline-and-classifier-provenance.md) | 0.4.0 candidate (on `main`) | `is_integrity_finding` re-grounded on real `verify` output (with `FR-100`); glosses for the five preconditions with no view to route into yet; the Trust & Keys **presentation** of a trust refusal (RFC 016) | [Classifier provenance](./handoffs/017-prikk-0-33-rebaseline-and-classifier-provenance/classifier-provenance-handoff-v1.md) |
| 015 | [prikk 0.32 re-baseline](./done/015-prikk-0-32-rebaseline.md) | 0.4.0 candidate (on `main`) | `FR-012`'s message filter (now possible); RFC 006 3b Patch detail (still blocked — `UD-09`'s content half); a message summary in the History row | [Re-baseline](./handoffs/015-prikk-0-32-rebaseline/rebaseline-handoff-v1.md) |
| 014 | [Commit: the first mutation](./done/014-commit-the-first-mutation.md) | 0.4.0 candidate (on `main`) | AUTHOR key id in the confirmation (own module, not by weakening `env.rs`'s guard); the pre-commit `UD-01` copy, now false for prikk ≥ 0.32; `Declined`'s inline path | [Commit](./handoffs/014-commit-the-first-mutation/commit-handoff-v1.md) |
| 013 | [Preview & tiered-confirmation machinery](./done/013-preview-and-confirmation-machinery.md) | 0.4.0 candidate (on `main`) | the `capability_gate`/palette unification — **must land before the first mutating palette command**; the `OPL-03` ceremony machines; `OPL-04`'s seam-side check | [Preview & confirm](./handoffs/013-preview-and-confirmation-machinery/preview-confirm-handoff-v1.md) |
| 003 | [Repository change token](./done/003-repository-change-token.md) | 0.4.0 candidate (on `main`) | the repository **fingerprint** — prikk has no repository identity by design; `INV-5` carries the protection it was meant to add | [Change token](./handoffs/003-repository-change-token/change-token-handoff-v1.md) |
| 011 | [Pre-1.0 API stability policy](./done/011-pre-1-0-api-stability-policy.md) | adopted; in force from 0.2.0 | blanket `#[non_exhaustive]` — a 1.0-readiness task, with the analysis pre-done | [0.2.0 prep](./handoffs/011-pre-1-0-api-stability-policy/release-prep-handoff-v1.md) · [0.3.0 prep](./handoffs/011-pre-1-0-api-stability-policy/release-prep-0-3-0-handoff-v1.md) · [0.4.0 prep](./handoffs/011-pre-1-0-api-stability-policy/release-prep-0-4-0-handoff-v1.md) · [0.5.0 prep](./handoffs/011-pre-1-0-api-stability-policy/release-prep-0-5-0-handoff-v1.md) |
| 012 | [Post-0.2.0 correctness sweep](./done/012-post-0-2-0-correctness-sweep.md) | 0.3.0 candidate (on `main`) | RFC 003 moved to 0.4.0; 19 rustdoc warnings + a rustdoc CI gate, scheduled with 0.4.0 planning | [Correctness sweep](./handoffs/012-post-0-2-0-correctness-sweep/correctness-sweep-handoff-v1.md) |
| 010 | [Off-thread seam & UI responsiveness](./done/010-off-thread-seam-and-ui-responsiveness.md) | 0.3.0 candidate (on `main`) | `NFR-P02` true cancellation + the Background Operations overlay's cancel action — both land with `FR-100` (verify) | [Off-thread seam](./handoffs/010-off-thread-seam-and-ui-responsiveness/off-thread-seam-handoff-v1.md) |
| 009 | [prikk 0.30 re-baseline & parser fidelity](./done/009-prikk-0-30-rebaseline-and-parser-fidelity.md) | 0.1.1 candidate (on `main`) | real-binary integration suite (`TS-07`, threat-model `RR-9`); `FR-014`'s ref surface corrected but not completed (no `tag list` read); ref-name validation (`RefName` unused); richer `.prikkignore` surface | [Parser fidelity](./handoffs/009-prikk-0-30-rebaseline-and-parser-fidelity/parser-fidelity-handoff-v1.md) |

## Archive (withdrawn or superseded)

_None yet._

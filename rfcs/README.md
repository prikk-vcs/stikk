# stikk RFCs

Design decisions for stikk, governed by the RFC lifecycle policy in
[`done/000-rfc-lifecycle-policy.md`](done/000-rfc-lifecycle-policy.md). Completed RFCs are never
deleted; they move to `done/`. **The folder is the source of truth for an RFC's state**, and the
`Status` field inside each file mirrors it.

stikk adopts the **five-folder variant** (`proposed/`, `accepted/`, `done/`, `archive/`, plus this
index): the maintainer's "accepted for implementation" is a distinct event from "implemented and
shipped", so an RFC moves `proposed/` → `accepted/` when its design is settled and an implementer may
start, then → `done/` when the work ships.

The initial design set (requirements, external design, internal design, data model, threat model)
lives in [`../docs/src/reference`](../docs/src/reference/) rather than as numbered RFCs — it was
produced as one coherent specification before the code. New, incremental design decisions are recorded
here as `NNN-slug.md`, numbered from `001`; numbers are stable forever and never reused.

## Proposed
_Open for review; an implementer should not start until an RFC moves to `accepted/`._

| ID | Title | Addresses |
|----|-------|-----------|
| 001 | [Frontend toolkit selection](./proposed/001-frontend-toolkit-selection.md) | the TUI toolkit (decided) and GUI direction — gates every interactive increment |
| 002 | [Action-id catalog and keybindings](./proposed/002-action-id-catalog-and-keybindings.md) | the stable action ids the config binds and the palette lists |
| 003 | [Repository change-token signal set](./proposed/003-repository-change-token.md) | cache validity and external-change / preview-staleness detection |
| 004 | [stikk-export report schema](./proposed/004-stikk-export-schema.md) | the versioned shape of stikk-authored report exports |
| 005 | [Linked-library prikk backend](./proposed/005-linked-library-prikk-backend.md) | a second seam backend, for when prikk's crates stabilize |

## Accepted
_Design settled; implementer may start; work has not yet shipped._

_None yet._

## Done (implemented)

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.1.0 (adopted, five-folder variant) |

## Archive (withdrawn or superseded)

_None yet._

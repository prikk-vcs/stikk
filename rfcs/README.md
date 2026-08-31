# stikk RFCs

Design decisions for stikk, under the four-folder RFC lifecycle policy (`proposed/`, `done/`,
`archive/`, and this index). Completed RFCs are never deleted; they move to `done/`. The folder is
the source of truth for an RFC's state.

The current design set (requirements, external design, internal design, data model, threat model)
lives in [`docs/src/reference`](../docs/src/reference/) rather than as numbered RFCs, because it was
produced as a single coherent specification before the code. New, incremental design decisions are
recorded here as `NNN-slug.md`, numbered from `001`.

## Proposed

The internal design deferred these decisions to the Program-Design phase; each is the natural first
RFC to write:

| Prospective | Decision to settle |
|---|---|
| TUI/GUI toolkit | which crates the frontends render with (deliberately unmade in the internal design) |
| `stikk-export` schema | the versioned shape of stikk-authored report exports (CT-02) |
| action-id catalog | the stable action ids the keybinding config binds (CF-03/TU-05) |
| change-token signals | the concrete prikk-observable signals the repository-change token reads (LC-4/LC-9) |
| library backend | adding a linked-library prikk backend behind the existing seam trait, once prikk's crates stabilize |

## Done

_None yet._

## Archive

_None yet._

# stikk

**stikk is a history browser and workbench for the [prikk](https://github.com/nabbisen/prikk)
version control system** — a terminal (TUI) and graphical (GUI) front-end over one shared operation
layer. The name is Norwegian for *to set a course, to take a bearing.*

Its founding property, from which the whole design follows: **stikk owns no repository authority and
no secrets.** Every repository fact is re-derived from prikk, and prikk — never stikk — reads signing
key material. Its stance mirrors prikk's own: *where prikk refuses, stikk explains.*

## Reading paths

- **New here?** Start with [Getting started](./guide/getting-started.md).
- **Reviewing the design?** The reference section is the full design set, in the order the project's
  workflow produced it: [Requirements](./reference/requirements.md) →
  [External design](./reference/external-design.md) →
  [Internal design](./reference/internal-design.md), with the
  [Data model](./reference/data-model.md) and [Threat model](./reference/threat-model.md) beside them.
- **Contributing?** See [Development](./contributing/development.md).

## Status

Foundation increment (0.1.0): the security-critical layers — the shared model, the prikk seam, and
the state layer — are implemented and tested; the launcher opens a repository and prints a one-shot
orientation. The interactive TUI is the next increment.

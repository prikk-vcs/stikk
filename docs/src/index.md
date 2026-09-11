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

**0.4.0 is where stikk writes.** The security-critical layers (shared model, prikk seam, state layer),
the interactive **TUI** (shell + Orientation, built on `ratatui` — RFC 001), **History** + Block detail
(RFC 006), the refusal-explanation and glossary surfaces (RFC 007), and **worktree Changes** (RFC 008)
are all implemented and tested; every seam read runs off the UI thread (RFC 010), and config/state
resolve per platform on Linux, macOS, and Windows (RFC 012). stikk can now **commit** a worktree
capture into prikk's active queue (RFC 014) and **seal** that queue into permanent, MAINTAINER-signed
history (RFC 016) — both behind preview-first, tiered-confirmation machinery that makes skipping a step
a compile error, not a review finding (RFC 013). Piped/CI invocation keeps the one-shot orientation.
stikk targets prikk **≥ 0.28**, validated through **0.38.0**. **Patch detail** is deferred behind
`UD-09` — prikk exposes no per-patch content yet — and **Compare** is deferred behind the same ceiling,
with a recorded future route (RFC 008); neither is "next", both are named gaps. Merge, sync, tag
create, and branch create/close remain unbuilt; a **Queue view** and **Trust & Keys** view are named
gaps too.

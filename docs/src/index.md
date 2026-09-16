# stikk

**stikk is a history browser and workbench for the [prikk](https://github.com/nabbisen/prikk)
version control system** — a terminal (TUI) front-end, with a graphical (GUI) one planned, over one shared
operation layer. The name is Norwegian for *to set a course, to take a bearing.*

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

**0.8.x keeps the screen true to the repository.** The security-critical
layers (shared model, prikk seam, state layer), the interactive **TUI** (shell + Orientation, built on
`ratatui` — RFC 001), **History** + Block detail (RFC 006), the refusal-explanation and glossary surfaces
(RFC 007), **worktree Changes** (RFC 008) and the **Queue view** (RFC 028) are all implemented and
tested; every seam read runs off the UI thread (RFC 010), and config/state resolve per platform on Linux,
macOS, and Windows (RFC 012). stikk can **commit** a worktree capture into prikk's active queue (RFC 014)
and **seal** that queue into permanent, MAINTAINER-signed history (RFC 016) — both behind preview-first,
tiered-confirmation machinery that makes skipping a step a compile error, not a review finding (RFC 013).
At prikk ≥ 0.39 commit is unavailable, with prikk's reasons, when prikk says it would refuse (RFC 027), and
the seal confirmation names the patches it freezes (RFC 028); a confirmed commit authors only the
worktree its preview showed (RFC 030). **stikk notices a change made outside it** — a commit, seal or
branch switch in another terminal — when the terminal regains focus or within five seconds while idle,
refreshes what is on screen, says *"repository changed outside stikk — refreshed"*, and makes an open
confirmation stale at once; `r` refreshes in place, and every refresh re-reads the ref its own screen
shows (RFC 031). **The Changes view and commit's confirmation say what a commit will author**: a
`prikk mv` whose two halves prikk lists is shown as one declared rename and counted, a declaration prikk
will not author as a rename is named instead, and a ref with no published history is named as one rather
than shown against a baseline it does not have (RFC 032). Piped/CI invocation keeps the one-shot
orientation. stikk targets
prikk **≥ 0.28**, validated through **0.42.0**, and on prikk 0.42 opens on prikk's current branch (RFC
029). **Patch detail** (`FR-030`) and **Compare** (`FR-033`) are not built: prikk ≥ 0.36 can render a
patch's content with `show`, so what waits is stikk's own work. Merge, sync, tag create, and branch
create/close remain unbuilt; a **Trust & Keys** view is a named gap too.

# Roadmap

How stikk gets from the current foundation to a usable history browser and workbench for prikk. This
is a direction, not a dated schedule: increments ship when they are correct and tested, and the order
can change as prikk evolves. Requirement and design ids (e.g. `FR-050`, `TU-01`, `UD-03`) refer to the
design set in [`docs/src/reference`](docs/src/reference/).

Guiding rules that do not change across the roadmap:

- **stikk owns no repository authority and no secrets** — every increment preserves this.
- **Preview-first, and *where prikk refuses, stikk explains*** — read and explanation surfaces come
  before, and stay ahead of, mutation surfaces.
- **One operation layer, two frontends** — the TUI is built first; the GUI reaches parity through the
  same operations, never a parallel implementation.
- **Design before implementation** — each milestone below is gated on its design items already
  existing in the reference set; new decisions are recorded as RFCs first.

## Now — shipped (0.1.0, foundation)

The security-critical layers, built and tested under the project gates:

- The workspace and lint discipline; `stikk-model` kernel; the `stikk-prikk` seam (CLI backend with
  the EPIPE guard, version gate, scripted backend, presence-only key reader); `stikk-state`
  (config, discovery, the repository-internal write refusal); the `stikk-core` `orient` operation;
  and the launcher (`--version`, `config check/path`, one-shot orientation).
- The two security invariants are enforced by test. 57 tests pass; clippy is clean under `-D
  warnings`.

## Next — the interactive read surface (toward 0.2)

The goal: a running TUI you can browse a repository with. Nothing here needs a mutation.

1. **Pick the TUI toolkit** — ✅ decided: **`ratatui` + `crossterm`** (RFC 001, accepted 2026-09-01;
   GUI toolkit spun out to a future RFC). It gates everything interactive.
2. **The TUI shell and Orientation view** (`TU-01/02/03`, `FR-002`) — ✅ shipped (`stikk-tui`): the
   header/status-bar/overlay layout, the view stack, global keys, and the live Orientation. `stikk
   <repo>` opens the TUI on a terminal; piped/CI keeps the one-shot print. Built per the
   [handoff](rfcs/handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md).
3. **History** (`FR-010…017`) with the unsealed queue tier + **Block detail** (`FR-031/032` at block
   granularity) — ✅ **3a shipped** (RFC 006): the seam grew `history`/`block_state`/`refs`, the
   operation layer gained `history_view`/`block_detail`/`list_refs`, and the TUI gained the view
   stack, the History and Block-detail views, and a ref picker — the first heavy use of the
   inert-text primitive and the overlay layer. `cargo run -p stikk-tui --example history_demo` drives
   all of it against a scripted backend. Built per the
   [handoff](rfcs/handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md).
   - **3b — Patch detail** (`FR-030`), patch-id enumeration, and diff-aware search (`FR-013`) are
     **split out, blocked on UD-09**: prikk exposes no per-patch content and no `show`/`diff`, only
     block-level `log`. stikk shows block lineage + a block's state file list now, and names the gap
     where a user would open a patch — never a faked diff. UD-09 (a `log --format json` + a
     patch-content surface) is filed upstream, mirroring UD-01…08.
4. **Refusal explanation overlay + the witness/finding glossary** (`FR-110/111`) and the **command
   palette** (`FR-125`). The explanation surface lands early, with the first read surfaces, because
   it is the product, not an error path.
5. **Compare** (`FR-033`) and **Changes** — worktree-vs-baseline (`FR-034`) computed via the
   replay/plan route while `worktree-status` is unusable upstream (`UD-03`).
6. **Session persistence and progressive disclosure** (`FR-122`, `TU-12`): resume the focused ref,
   view, and filters; default vs. advanced view depth.

## Then — the working cycle and explanation-heavy operations (toward 0.3)

Mutations, always preview-first with tiered confirmation (`FR-120/121`):

- **Commit → queue review → seal ceremony** (`FR-050/051/052`), including the informed-consent
  no-audit step and the capability gate re-checked at the seam.
- **Verify report browser and doctor/recovery** (`FR-100/101/102`), with the three-valued
  author-signature outcome rendered precisely (Sound / Unverifiable / a blocking failure) and locks
  never auto-cleared.
- **Branches and tags** (`FR-070/071`), the **focused-ref** model, and **checkout / deletion**
  planning and materialization (`FR-053/054`).
- **Merge evidence → plan → execution** (`FR-080…082`) and the **rollback flow** (`FR-083`) — the
  merge refusal path is where the explanation surface earns its place.

## Later — exchange, trust, and the GUI

- **Exchange**: bundle export/verify/import and the **sync assistant** (`FR-090…094`), with the input
  ceilings surfaced before an operation runs.
- **Trust & keys** (`FR-103/104`): adopted maintainer keys, TOFU-conflict-as-security-event, and
  signing readiness — still presence-only, still no seed ever stored.
- **The GUI** (`GU-01…09`): the same operations rendered natively, reaching TUI parity through the
  shared operation layer, with drag-and-drop constrained to prikk-legal targets.
- **Internationalization** (en / ja / nb) and accessibility hardening across both frontends
  (`NFR-I01`, `NFR-A01…04`).
- **Report export** (`CT-02`) and the versioned `stikk-export` schema.

## Deferred design decisions (the near-term RFC queue)

From the internal design, recorded in [`rfcs/README.md`](rfcs/README.md):

| Decision | Gates |
|---|---|
| TUI/GUI toolkit | everything interactive |
| `stikk-export` schema | report export |
| action-id catalog | the keybinding config |
| change-token signal set | cache validity and external-change refresh |
| linked-library prikk backend | performance, once prikk's crates stabilize (behind the existing seam trait) |

## prikk-side dependencies stikk is waiting on

Several stikk features are gated on prikk-core changes, not on stikk effort. stikk ships degraded,
honest behavior until they land, and adopts the clean path when they do. These double as upstream
issues for the prikk project (requirement `UD-01…UD-05`):

| Dependency | prikk gap | stikk behavior meanwhile |
|---|---|---|
| `UD-01` | patch messages are discarded; no author display name | commit collects a message and says core does not persist it; history shows ids/keys/paths |
| `UD-02` | machine-readable output only on `verify` | the seam parses confined, version-gated output and refuses rather than guesses; never screen-scrapes unpinned prose |
| `UD-03` | `worktree-status` is broken on ordinary repos | Changes is computed via the replay/plan route |
| `UD-04` | the CLI panics on EPIPE | the seam drains output fully (already implemented) |
| `UD-05` | exit codes collapse to 0/1 | the seam classifies by message + context |
| `UD-09` | no per-patch content, no patch-id enumeration, no `show`/`diff` — `log` is block-level only (RFC 006) | History shows block lineage + a block's state file list; Patch detail (3b) waits; the gap is named where a user would open a patch |

## Releases and versioning

stikk versions independently of prikk and declares, per release, the prikk range it was validated
against (currently `>= 0.27.x`; `NFR-R03`). Before a 1.0, the repository format and command surface of
prikk are still moving, so stikk stays pre-1.0 too and treats its own APIs as unstable. Changes are
recorded in [`CHANGELOG.md`](CHANGELOG.md); design decisions in `rfcs/`.

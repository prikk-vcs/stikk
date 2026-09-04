# RFC 001 — Frontend toolkit selection

**Status.** Implemented (0.1.0) — **TUI: `ratatui` on the `crossterm` backend**. Shipped as the TUI
shell + Orientation increment. See [Acceptance](#acceptance-2026-09-01).
**Deferred, carried forward (not built by this RFC):** the **GUI toolkit remains undecided** and is
spun out to its own RFC, to be written when GUI work begins (§GUI holds the shortlist and the
accessibility filter that are its input); and the **TUI accessibility limitation** ruled at acceptance
(terminals give assistive tech little to work with, so the accessible path is the future GUI) is still
to be stated plainly in the TUI's own docs. The handoff is
[`../handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md`](../handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md).
**Tracks.** Which rendering toolkits the TUI and GUI frontends are built on. Gates every interactive
increment (the internal design deferred this to Program Design deliberately).
**Touches.** `crates/stikk-tui` and `crates/stikk-gui` (neither exists yet); their dependency
additions; nothing below the operation layer — the seam, state, model, and core stay toolkit-agnostic
by design (`stikk-04` AR-03/AR-04).

## Summary

stikk's internal design leaves the frontends thin: they translate input events into `stikk-core`
operations and render the view-models `stikk-core` returns (`FE-01`). Choosing *how* they render is a
Program-Design decision, and this RFC makes it. It proposes to **decide the TUI toolkit now** — it
gates the next increment — and to **set a direction for the GUI without a final commitment**, because
the GUI is several increments away and its hardest constraint (platform accessibility) deserves a
decision made against a real interface, not in the abstract.

The decision is low-risk to reverse for one reason established by the architecture: the frontends
depend only on `stikk-core` and `stikk-model`, so a toolkit swap touches one crate and no
operation, view-model, or seam behaviour (`FE-01`, `FR-123`). This RFC is therefore about picking a
good default, not a permanent lock-in.

## Requirements the choice must satisfy

From the requirements and external design:

- **TUI** must be fully keyboard-operable (`NFR-A01`), run on standard terminals across Linux, macOS,
  and Windows (`NFR-T02`), degrade legibly on limited colour/Unicode (`TU-10`), and render the view
  inventory `TU-01` with an overlay layer that never destroys the view beneath it (`TU-02`).
- **GUI** must expose platform accessibility APIs — names, roles, focus order, keyboard operability
  for every action (`NFR-A02`) — support i18n (`NFR-I01`, and the GUI project rule), theme
  light/dark/system (`GU-07`), and mirror the TUI view inventory (`GU-01`).
- **Both** must keep the UI responsive with work off the UI thread (`NFR-P01`, `CC-01`), respect
  reduced-motion (`NFR-A04`), and add no dependency that reaches below the operation layer.
- **House rule:** *Less is more* — prefer a small, well-contained toolkit over a large framework the
  project routes around.

## TUI — recommendation

**Proposed: `ratatui` on the `crossterm` backend.**

Rationale:

- It is the de-facto standard immediate-mode terminal UI for Rust, which matches stikk's model
  exactly: `stikk-core` returns a fresh view-model, the frontend redraws it each frame — there is no
  retained widget tree to keep in sync with authority stikk does not own (`INV-8`).
- `crossterm` is pure-Rust and cross-platform, covering the three mutation platforms including
  Windows (`NFR-T02`), with raw-mode, colour, and Unicode-width handling that `TU-10`'s degraded
  paths can build on.
- The overlay layer (`TU-02`) is natural in immediate mode: overlays are extra draw passes over the
  same frame, so "never destroys the view beneath it" is structural, not bookkeeping.
- It adds a bounded, widely-audited dependency set — acceptable for stikk (which, unlike prikk, is not
  under a five-crate constraint) while still honouring *Less is more*.

Considered and not recommended for v1: `cursive` (retained-mode; a widget tree stikk would have to
reconcile with re-derived view-models, working against `INV-8`); hand-rolling on raw `crossterm`
(reinvents layout and widgets with no payoff).

## GUI — direction, decision deferred

The GUI is later (roadmap "Later"), and its selection turns on accessibility, which is hard to judge
before there is an interface to test. This RFC **does not choose** the GUI toolkit; it records the
shortlist and the filter, and proposes that the final choice be its own RFC written when GUI work
begins.

The filter, in priority order: (1) real platform-accessibility support (`NFR-A02`) — this is the
gate, not a nice-to-have; (2) keyboard operability for every action; (3) i18n; (4) not reintroducing
a browser/webview runtime (which would undercut stikk's small-surface posture and complicate the
no-network property). The shortlist to evaluate against a prototype: `egui` (immediate-mode, matches
the TUI's redraw model and the shared view-model shape; accessibility via AccessKit), `iced`
(Elm-like, retained), and the native GTK path (`gtk-rs`, strongest Linux accessibility, heaviest
dependency). No recommendation is made here on purpose.

## Consequences

- The next increment can create `crates/stikk-tui` against `ratatui`/`crossterm` and build the shell
  and Orientation view (`TU-01/02/03`) on the operation layer that already exists.
- `stikk-gui` is not created until its own RFC settles the toolkit; the roadmap already sequences it
  after the read and working-cycle surfaces.
- No crate below `stikk-core` gains a toolkit dependency — the layering audit (`AR-01`) stays true.

## Open questions

- Does `ratatui`'s accessibility story (screen-reader support is weak in terminals generally) meet
  `NFR-A01` for users who rely on assistive tech, or is a documented limitation the honest position
  for the TUI, with the GUI as the accessible path? This should be answered before 0.2 ships, not
  assumed.
- Should the TUI backend be abstracted (crossterm today, termion/termwiz later) or is committing to
  crossterm fine given the swap-cost analysis above? Proposed: commit to crossterm; revisit only if a
  platform gap appears.

## Acceptance (2026-09-01)

**Ruled:** the TUI is built on **`ratatui`** over the **`crossterm`** backend, for the reasons in
§TUI — the immediate-mode redraw model matches stikk's re-derive-a-view-model design, and `crossterm`
covers the three mutation platforms in pure Rust. Both are pinned to a specific version in the first
`stikk-tui` manifest and updated deliberately. The two open questions are carried, not blockers:

1. **Backend not abstracted for now** — commit to `crossterm` directly; revisit only if a platform gap
   appears (the frontends depend only on `stikk-core`, so a later swap is one-crate-local).
2. **TUI accessibility** is treated as a **documented limitation** rather than a claim: terminals give
   assistive tech little to work with, so the accessible path is the (future) GUI. This must be stated
   plainly in the TUI's own docs before 0.2 ships; it does not block the read surfaces.

**Scope decided by this acceptance:** TUI toolkit only. **The GUI toolkit remains undecided** and is
spun out to its own RFC to be written when GUI work begins (the shortlist and the accessibility filter
in §GUI are the input to it). Accepting this RFC does not authorize any GUI dependency.

**Unblocks:** the roadmap's "Next" increment 2 — the TUI shell and Orientation view (`TU-01/02/03`,
`FR-002`). Its program design, decision notes, and the security surface it activates are in the
handoff linked in the Status field. Implementation follows the handoff; nothing below `stikk-core`
changes.

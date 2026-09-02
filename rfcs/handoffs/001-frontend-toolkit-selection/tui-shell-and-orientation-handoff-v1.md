# Handoff — TUI shell & Orientation view (v1)

**Companion to:** RFC 001 (Accepted 2026-09-01). Inherits its state.
**Realizes:** ROADMAP "Next" increment 2 — the TUI shell and Orientation view.
**Design items:** `TU-01/02/03` (shell, views, status bar), `FR-002` / `VW-01` (Orientation),
`CL-06` (TTY discipline), `NFR-P01` (never block input), `NFR-A03/A04` / `TU-10` (degraded terminals),
`CC-05` / `NFR-R01` (kill-safe), and the `AR-03/AR-04` layering (the frontend is thin; it drives
`stikk-core` and renders its view-models, computing nothing).

This is the program design and the decision record for the increment. **Implementation, tests, and the
example follow this document; they do not precede it.** Where this handoff and RFC 001 or the internal
design disagree, the RFC/design wins and this handoff is corrected first (per the RFC lifecycle
policy).

---

## 1. Scope

**In:** a running, interactive terminal UI that opens a repository and shows the live Orientation the
launcher currently prints once — inside the shell (header, active view, status bar, overlay layer),
with a global key set, a background/loading indicator, refresh, and safe terminal setup/teardown.

**Out (later increments, do not build now):** History, Patch/Block detail, Compare, Changes; any
mutation or confirmation flow; the refusal-explanation overlay's *content* (the overlay *layer* is
built here, but its first real consumer is the next increment); the command palette; the GUI;
session persistence. Keep the surface to a read-only Orientation.

**The one behavioural change to the launcher:** `stikk [path]` launches the interactive TUI **when
stdout is a TTY**; when it is not (piped, redirected, CI), it keeps the existing one-shot orientation
print, which stays the machine-friendly path. `config check`, `config path`, `--version`, `--help`
are unchanged. This satisfies `CL-06` (the TUI owns the terminal only while attached to a TTY) without
losing the scriptable output.

---

## 2. The new crate — `crates/stikk-tui`

A new workspace member, per the internal design's `MOD-06`. It is the first crate to gain a rendering
dependency; nothing below `stikk-core` is touched (`AR-01` stays true — verify with the boundary of
imports).

- **Dependencies (pin exact versions in the manifest, update deliberately):** `ratatui` and
  `crossterm` (RFC 001 acceptance). `stikk-core`, `stikk-model`, `stikk-prikk` (for `NullBackend` in
  tests and the `CliBackend` in the launcher), `stikk-state`. No dependency reaches below `stikk-core`
  for logic.
- **Lints:** inherit the workspace lints (`unsafe` forbidden, `missing_docs` warn, panic-prone lints
  warn). The TUI must not `unwrap`/`expect`/panic on fallible paths in production code.
- **Module layout** (2018+ style, sibling tests — never `#[test]` inline):
  - `lib.rs` — crate docs; re-exports `run` (the entry point the launcher calls).
  - `app.rs` (+ `app/tests.rs`) — the application state and the run loop: owns the current view, the
    overlay stack, the background-op indicator, and the repository handle + a `&dyn Prikk`. Turns input
    events into `stikk-core` calls and holds the resulting view-models. **Holds no repository
    authority** (`INV-8`): a view-model is a rendered snapshot, re-sourced on refresh.
  - `terminal.rs` (+ `terminal/tests.rs`) — raw-mode enter/leave, alternate screen, and a
    **panic-safe guard** that restores the terminal on drop and on panic (see §6). TTY detection lives
    here (`CL-06`).
  - `shell.rs` — the frame layout (`TU-02`): header line, active-view region, one-line status bar,
    and the transient overlay layer above them. Pure layout; draws whatever the app hands it.
  - `status_bar.rs` (+ tests) — renders `TU-03`: repository short name, focused ref (defaults to
    `heads/main` this increment — there is no focused-ref *selection* yet), queue depth (hidden at 0),
    worktree marker (unknown this increment — orientation does not carry it yet; render `—`),
    capability badges `[RO]` `[AUT ✓/–]` `[MNT ✓/–]`, and a background-op indicator. **No "HEAD"** —
    it does not exist. Every badge has a text form (`NFR-A03`).
  - `view/orientation.rs` (+ tests) — the Orientation view (`TU-01`, `VW-01`): renders the
    `stikk_core::OrientationView` the operation layer already produces. Fields: prikk version + support
    (degraded-to-read-only note if unsupported), capability, signing readiness (author/maintainer,
    read-only), queue depth, a trailing-partial-WAL warning when non-zero, and `heads/main` state.
  - `overlay.rs` (+ tests) — the overlay layer primitive (`TU-02`): a stack of overlays drawn above
    the active view without destroying it. This increment ships one overlay — **Help** (a static key
    reference, `?`) — and the empty machinery the refusal/glossary/palette overlays plug into next.
  - `keys.rs` (+ tests) — the global key map for this increment (subset of `TU-05`): `q`/`Esc` quit
    (or close the top overlay first), `?` toggle Help, `r` refresh (re-source Orientation), `Ctrl-C`
    quit. Bindings are literals here; the configurable action-id catalog is RFC 002, not this
    increment — leave a single `dispatch(key) -> Action` seam so RFC 002 slots in without a rewrite.
  - `theme.rs` — light/dark/system palette selection from `stikk_state::Config` (`GU-07` applies to
    the TUI's colour choices too); a monochrome fallback (`TU-10`).

---

## 3. The render model (how the frontend stays thin)

One rule, from `AR-03/FE-01/INV-8`: **the TUI computes nothing about the repository.** The loop is:

1. On open (or refresh `r`): call `stikk_core::orient(&prikk, repo)` — the *only* place repository
   facts come from. While it runs, show a "loading…" state and keep input responsive (`NFR-P01`); the
   call is a handful of `prikk` invocations and is fast, but it still runs so as not to freeze the draw
   loop — do it on a worker and post the result back, or (acceptable for this single call this
   increment) call it before entering the draw loop and again on refresh, showing a spinner frame
   around it. The worker path is preferred and is the pattern History will need (`CC-01`); document
   whichever is chosen.
2. Store the returned `OrientationView` in the app.
3. Each frame, `shell` + `view::orientation` + `status_bar` render **from that stored view-model**,
   never from a live prikk call inside the draw.
4. A refusal or environment error from the seam is not a crash: it is rendered as an error state
   (this increment: a simple centred message carrying prikk's verbatim text — `NFR-I03`; the full
   refusal overlay with next-steps is the next increment). The taxonomy is already `StikkError`'s.

---

## 4. Decision notes

- **Immediate-mode, not retained (RFC 001).** Each frame is a fresh draw of the current view-model.
  This is why the frontend needs no widget-state reconciliation and cannot drift from authority it does
  not own.
- **Focused ref is not yet selectable.** Orientation is about `heads/main` by construction this
  increment. The status bar shows it as the focused ref so the concept is visible, but *changing* focus
  arrives with History. Do not build a HEAD.
- **Keys are literals now, action-seam ready.** RFC 002 (action-id catalog) will own bindings; this
  increment must not hard-code keys in a way that forces a rewrite — route every key through one
  `dispatch` function returning an `Action`.
- **TTY-gated launch, one-shot fallback.** See §1. This keeps CI/scripts working and honours `CL-06`.
- **No new seam methods.** Orientation uses `handshake` + `orientation`, which already exist. The seam
  does not grow this increment (it grows for History next).

---

## 5. Security surface of this increment

This increment adds a rendering surface but **no new trust boundary** and **no mutation**, so the
threat model's existing controls cover it; two are activated and one is set up for the next increment.

- **No repository risk (`INV-1`).** The TUI writes nothing to a repository; a bug here cannot corrupt
  one. It reads via the seam only.
- **No secret exposure (`C-I1`).** Orientation renders signing **readiness** (presence booleans) and
  key *ids* are not even shown yet — no seed value is reachable. Keep it that way: the status bar shows
  `✓/–`, never key material.
- **Terminal-safety is an availability control, not a corruption one (`C-D1`-adjacent, `NFR-R01`).**
  Raw mode and the alternate screen **must be restored on every exit path including panic** (§6). A
  front-end that leaves a user's terminal wedged is the failure to prevent here.
- **Set up untrusted-content escaping now, even though Orientation has none (`C-T2a`).** Orientation
  shows only prikk-authoritative values (versions, counts, a hex RefState id). But History/Patch detail
  (next increment) will render **untrusted repository content** — ref names, tag messages, patch
  operation text — which can carry terminal control sequences (threat `T-T2`). Build a single
  **inert-text render primitive** in this increment (escape/strip control characters before any
  repository-sourced string reaches the terminal) and route Orientation's few dynamic strings through
  it, so the control exists and is tested before the increment that truly needs it. This is the
  cheapest place to establish it and a hard prerequisite for History.

No change to the threat model document is required — this increment introduces no new asset, boundary,
or data flow; it exercises existing controls. (Per `NFR-S07`, the threat model is revisited when a data
flow, boundary, or the signing/trust surface changes; none does here. This section is the record that
the review happened and found nothing new.)

---

## 6. Terminal safety (the one thing that must not be gotten wrong)

- Entering raw mode + alternate screen happens once, guarded by an RAII type in `terminal.rs` whose
  `Drop` restores cooked mode and the main screen.
- Install a panic hook that restores the terminal **before** the default hook prints the panic, so a
  bug surfaces as a readable message on a working terminal, not a wedged one. The hook must be
  idempotent with the RAII `Drop`.
- `CL-06`: refuse to start the TUI when stdout is not a TTY — return the launcher's exit `4`
  ("environment unsuitable") rather than emit control sequences into a pipe. The one-shot path (§1)
  handles the non-TTY case instead.
- Minimum size handling (`TU-10`): below 80×24, draw a legible "terminal too small" frame rather than a
  broken layout. Monochrome fallback when colour is unavailable; every status conveyed by text/shape,
  not colour alone (`NFR-A03`). Respect reduced-motion by dropping the spinner for a static indicator
  (`NFR-A04`).

---

## 7. Test plan

TUIs are testable without a real terminal, and this one must be (CI runs headless):

- **Render tests (`TS-01`) with `ratatui`'s `TestBackend`:** draw the shell + Orientation into an
  in-memory buffer and assert on cell content — e.g. the version line, the capability, the readiness
  badges, the queue count, the trailing-partial warning when set. Deterministic, no TTY.
- **Seam via `NullBackend` (`TS-02`):** drive Orientation from the scripted backend so every state is
  reproducible — supported/unsupported prikk, a clean repo, a queued/partial-tail repo, and a refusal
  (retired format) rendering prikk's verbatim message.
- **Key dispatch tests:** `dispatch(key)` returns the expected `Action` for `q`/`Esc`/`?`/`r`/`Ctrl-C`,
  including "Esc closes the top overlay before quitting".
- **Inert-text primitive tests (`C-T2a`):** a string containing terminal control bytes renders with
  them escaped/stripped; a plain string is unchanged.
- **TTY-refusal test:** the launcher path returns exit `4` (or takes the one-shot branch) when stdout
  is not a TTY. (Terminal raw-mode/panic-restore is validated by a manual check plus a smoke test of
  the RAII guard's `Drop`; a full pty test is optional and may be deferred.)
- Gates unchanged: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test
  --workspace`.

---

## 8. Example (per project rules — runnable examples)

Add `crates/stikk-tui/examples/orientation_demo.rs`: launches the shell against a **`NullBackend`** with
a scripted orientation (a couple of queued patches, a published `heads/main`, maintainer readiness), so
anyone can see and drive the TUI with **no prikk binary and no repository**. This doubles as living
documentation of the render model and a manual smoke test. Document it in the crate README/guide.

---

## 9. Acceptance criteria

1. `stikk <repo>` on a TTY opens the interactive shell showing the live Orientation; `q`/`Ctrl-C`
   exits cleanly and the terminal is fully restored.
2. Piped/non-TTY invocation still prints the one-shot orientation (no regression).
3. `?` opens Help; `r` re-sources Orientation; `Esc` closes an overlay before quitting.
4. Orientation renders correctly for: supported & unsupported prikk, clean & queued & partial-tail
   repos, and a refusal (verbatim message shown) — all asserted via `TestBackend` + `NullBackend`.
5. The inert-text primitive exists, is used for repository-sourced strings, and is tested.
6. Terminal restored on panic (panic hook + RAII).
7. No crate below `stikk-core` gained a dependency; no new seam method; no repository write path.
8. `fmt` / `clippy -D warnings` / `test` all green. A runnable `orientation_demo` example builds and
   runs against `NullBackend`.

---

## 10. Out of this increment, queued next

History (`FR-010…017`) + Patch/Block detail (`FR-030…032`) — needs the seam's `read-history` /
`read-state` methods and is the first real consumer of the inert-text primitive and the overlay layer
built here. Then the refusal-explanation overlay content + glossary (`FR-110/111`). The ROADMAP already
sequences these.

# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## Unreleased

### Added — the interactive TUI shell & Orientation view

The first interactive surface, built on `ratatui` + `crossterm` (RFC 001), following the
`001` handoff. `stikk <repo>` on a terminal now opens a live TUI; piped/CI invocation keeps the
one-shot orientation print.

- **`stikk-tui` crate** — the thin terminal frontend: it renders `stikk-core` view-models and computes
  nothing about the repository (design AR-03/INV-8). Modules: the shell (header, active view, status
  bar, overlay layer), the Orientation view (`VW-01`/`FR-002`), the status bar (`TU-03` — repo,
  focused ref, queue, capability/readiness badges; never a "HEAD"), a Help overlay, global key
  dispatch through a single `Action` seam (ready for RFC 002), the light/dark/mono palette (fixed-RGB
  foreground/secondary text so labels stay legible on any terminal theme — NFR-A03), and the
  panic-safe terminal guard.
- **Security controls activated** (handoff §5): terminal restore-on-panic, and the **inert-text
  primitive** — every repository-sourced string is stripped of control characters before it reaches a
  cell (threat model C-T2a), built and tested now for the untrusted content History will render next.
- **Launcher** now opens the TUI on a TTY and falls back to the one-shot orientation off a TTY
  (design CL-06). No new seam method; nothing below `stikk-core` gained a dependency.
- **Tests & example**: render tests via `ratatui`'s `TestBackend` driven by the scripted `NullBackend`
  (headless, deterministic) plus a runnable `orientation_demo` example needing no prikk or repository.
  Workspace now at 84 tests; `fmt` / `clippy -D warnings` / `test` all green.

### Decisions & planning

- **RFC 001 accepted (2026-09-01): the TUI is built on `ratatui` + `crossterm`.** The GUI toolkit is
  deliberately left undecided and spun out to a future RFC. RFC 001 moved `proposed/` → `accepted/`.
- **Handoff for the TUI shell + Orientation increment** written and now realized: program design,
  decision notes, the security surface, the test plan, and acceptance criteria —
  `rfcs/handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md`.

## 0.1.0 — foundation

The first real code increment: a multi-crate workspace implementing the security-critical foundation
of the design set, built and tested under prikk-grade gates.

### Added

- **Workspace and lint discipline.** Five crates under a Cargo workspace (Rust 2024, MSRV 1.85):
  `stikk-model`, `stikk-prikk`, `stikk-state`, `stikk-core`, and the `stikk` launcher. `unsafe` is
  forbidden; public items must be documented; the panic-prone clippy lints warn.
- **`stikk-model`** — the shared kernel: the `StikkError` taxonomy (seven presentation classes,
  `#[non_exhaustive]`, with `source()` implemented — a lesson from the prikk audit); validated
  `ObjectId`/`RefName` newtypes; the nine `RequestCategory` values carrying their mutation and
  cancellability policy as data; and the `Capability`/`Readiness` derivation.
- **`stikk-prikk` — the prikk seam.** The `Prikk` trait, a `CliBackend` that drives the `prikk`
  binary (draining output fully before classifying the exit — the EPIPE guard), a version handshake
  with a validated-range gate, a scripted `NullBackend` for offline testing, and the presence-only
  key-readiness reader.
- **`stikk-state`** — user-scope config, session, and handle stores. A forgiving line-oriented config
  parser that preserves unknown keys and never blocks launch; repository discovery; and the path
  resolver's repository-internal write refusal.
- **`stikk-core`** — the operation layer, with the read-only `orient` operation composing a
  handshake, orientation, and derived capability into a view-model.
- **The `stikk` launcher** — `--version`, `--help`, `config check`, `config path`, and opening a
  repository to print a one-shot orientation (the interactive TUI is the next increment).
- **Documentation** — README, SECURITY.md, CONTRIBUTING.md, ROADMAP.md, and an mdBook under `docs/`
  carrying the full design set (requirements, external design, internal design, data model, threat
  model).
- **RFC process** — the RFC lifecycle policy adopted as `rfcs/done/000` (five-folder variant), with the
  five deferred Program-Design decisions drafted as proposed RFCs 001–005 (frontend toolkit, action-id
  catalog, change-token signals, export schema, library backend).

### Security

- The seam reads `PRIKK_*_SEED` **presence only, never their values** — enforced by a source-level
  guard test.
- stikk's path resolver **refuses any repository-internal write target** — the primary boundary
  control, since prikk has no foreign-file backstop.

### Notes

- 57 tests pass; `cargo clippy --workspace -- -D warnings` and `cargo fmt --check` are clean.
- The TUI/GUI toolkit choice, the `stikk-export` schema, the action-id catalog, and the change-token
  signal set remain deferred to Program Design, exactly as the internal design states.

# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## Unreleased

### Decisions & planning

- **RFC 001 accepted (2026-09-01): the TUI is built on `ratatui` + `crossterm`.** The GUI toolkit is
  deliberately left undecided and spun out to a future RFC. RFC 001 moved `proposed/` → `accepted/`.
- **Handoff for the TUI shell + Orientation increment** (the roadmap's next step) written: program
  design, decision notes, the security surface it activates, the test plan, and acceptance criteria —
  `rfcs/handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md`. Implementation
  follows the handoff.

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

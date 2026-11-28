# Development

See [CONTRIBUTING.md](https://github.com/prikk-vcs/stikk/blob/main/CONTRIBUTING.md) at the repository
root for the full contribution guide. In brief:

## Workflow

Design before implementation:

> Requirements → External Design → Internal Design → Program Design → Implementation → Testing

The design documents in the [reference section](../reference/requirements.md) are the **source of
truth for tests** — a test validates a design item, not merely the code. Cite the design item id
(e.g. `FR-050`, `SEAM-03`, `C-I1`) when you add or change behavior.

## Architecture in one paragraph

stikk is a five-layer workspace: `stikk-model` (shared kernel, no I/O) ← `stikk-prikk` (the seam —
the only code that talks to prikk) and `stikk-state` (user-scope config/session) ← `stikk-core` (the
one operation layer both frontends drive) ← `stikk-app` (the program every front end drives) ← the
front ends and the `stikk` launcher. Dependencies point strictly downward. The seam is the only door
to prikk; the operation layer owns no I/O and no widgets.

## Running the desktop app from source

The desktop app is `stikk-gui` (RFC 070). It is **not published** and nothing in a release builds
it: `cargo run -p stikk-gui` is the only way to start it today.

```sh
cargo run -p stikk-gui            # the repository discovered from the working directory
cargo run -p stikk-gui -- PATH    # a repository at PATH
```

It draws the same program the terminal front end draws (`stikk-app`), so it needs prikk exactly as
the terminal does; `STIKK_PRIKK_BIN` points at a specific prikk build, as everywhere else.

**Two crates, and the line between them** (RFC 069 rule 1):

- `stikk-gui-design` is the only place that names a colour, a size, a font, a spacing or a Snora
  style;
- `stikk-gui` holds the views, the key mapping, the reading report and the binary, and imports
  neither `snora::design` nor `iced::Color` — which is a check over its source, not a convention.

### The two instruments a measured run uses

Both are read by `stikk-gui` and **do nothing when unset**, which is every ordinary launch:

| Variable | What it does |
|---|---|
| `STIKK_GUI_LOG=<file>` | writes the program's whole state after every message, one line each. That log is what a real-window run asserts — recorded state, not pixels. |
| `STIKK_GUI_FONT=<family>` | names the interface font, for the label-fit guard's two bracketing fonts (RFC 069 rule 2). Unset, the font is the system's own. |

### The window probe

`cargo run -p stikk-gui --example window_probe` opens the real app on a scratch directory, opens the
Glossary through the program's own intent, logs every window fact it is given, and exits 0 only when
the viewport is confirmed from the window **and** nothing was read before it was (RFC 069 rule 8).
`--scale` sets the zoom and `--seconds` the deadline. It is an example, so no probe code is in the
app.

## Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

All three must pass. The desktop app's headless tests are part of the third, on every platform: they
need no display and no prikk, because every fixture is built through `stikk-app`'s own
`test-fixtures` feature.

## Two security invariants enforced by test

1. The seam reads no signing-key **value** — only presence (`crates/stikk-prikk/src/env`).
2. stikk never writes inside a repository (`stikk-state::paths::ensure_outside_repository`).

## Rust conventions

Rust 2024, MSRV 1.88. 2018+ module style (`foo.rs` + `foo/`, no `mod.rs`). Tests are **siblings**:
`src/foo/tests.rs` with `#[cfg(test)] mod tests;` in `foo.rs`, never `#[test]` inline in the
implementation file. No `unsafe`. No panics on fallible input.

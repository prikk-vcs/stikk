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
one operation layer both frontends drive) ← the frontends and the `stikk` launcher. Dependencies point
strictly downward. The seam is the only door to prikk; the operation layer owns no I/O and no widgets.

## Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

All three must pass.

## Two security invariants enforced by test

1. The seam reads no signing-key **value** — only presence (`crates/stikk-prikk/src/env`).
2. stikk never writes inside a repository (`stikk-state::paths::ensure_outside_repository`).

## Rust conventions

Rust 2024, MSRV 1.85. 2018+ module style (`foo.rs` + `foo/`, no `mod.rs`). Tests are **siblings**:
`src/foo/tests.rs` with `#[cfg(test)] mod tests;` in `foo.rs`, never `#[test]` inline in the
implementation file. No `unsafe`. No panics on fallible input.

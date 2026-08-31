# Contributing to stikk

Thanks for your interest. stikk is a front-end for the prikk version control system, and it holds
itself to prikk's own engineering discipline.

## Workflow: design before implementation

stikk follows a design-first workflow:

> Requirements → External Design → Internal Design → Program Design → Implementation → Testing

The design documents are in [`docs/src/reference`](docs/src/reference) and are the **source of truth
for tests** — a test validates a design specification, not merely the written code. Before proposing a
change that adds or alters behavior, check it against the relevant design item and cite the item id
(e.g. `FR-050`, `SEAM-03`, `C-I1`) in the change.

New design decisions are recorded as RFCs under `rfcs/` (the five-folder lifecycle: `proposed/`,
`accepted/`, `done/`, `archive/`, and an index `README.md`). An RFC moves `proposed/` → `accepted/`
when its design is settled and an implementer may start, then → `done/` when the work ships. Completed
RFCs are never deleted.

## Ground rules

- **Language & edition.** Rust, 2024 edition, MSRV 1.85. English for all code and docs.
- **Module style.** 2018+ modules: a `foo.rs` and a `foo/` directory coexist; no `mod.rs`.
- **Tests are siblings, never inline.** Put tests in `src/foo/tests.rs` with `#[cfg(test)] mod
  tests;` in `foo.rs` — not `#[test]` functions inside the implementation file. Split a large
  `tests.rs` into a `tests/` directory by the same line-count logic.
- **No `unsafe`.** The workspace forbids it; there is no FFI yet.
- **No panics on fallible input.** `unwrap`/`expect`/`indexing` lints warn; production code carries
  none. A front-end must never crash a user's terminal on malformed prikk output.

## Two security invariants you must not break

These are enforced by test; a change that trips them is a bug, not a style preference:

1. **The seam reads no signing-key value** — only presence (`stikk-prikk::env`). See the source-level
   guard test in `crates/stikk-prikk/src/env/tests.rs`.
2. **stikk never writes inside a repository** — `stikk-state::paths::ensure_outside_repository`
   refuses any repository-internal target.

## Before submitting

Run the gates the CI will run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

All three must pass. Keep changes scoped and cite the design items they realize.

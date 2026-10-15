# Contributing to stikk

Thanks for your interest. stikk is a front-end for the prikk version control system, and it holds
itself to prikk's own engineering discipline.

## Building from source

Most people do not need this — [the README](README.md#install) installs a prebuilt stikk and prikk
in two commands. Build from source to work on stikk itself.

stikk is Rust 2024 edition, MSRV **1.88**:

```sh
cargo build --release
```

The binary lands at `./target/release/stikk`. Run it against a repository:

```sh
./target/release/stikk /path/to/repo
```

stikk drives the external `prikk` binary at runtime; it is not a Cargo dependency. If prikk is not
on your `PATH`, point stikk at it:

```sh
export STIKK_PRIKK_BIN=/path/to/prikk
```

To see the TUI with no repository at all:

```sh
cargo run -p stikk-tui --example orientation_demo
```

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

- **Language & edition.** Rust, 2024 edition, MSRV 1.88. English for all code and docs.
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

## The README's images

The three images in `README.md` are **rendered by stikk's own views from invented data** (RFC 042),
never photographed: a scene is built from a scripted `NullBackend`, drawn through the real `App`
and the real views into an 80 x 24 buffer, and written out as SVG. Nothing of your machine is read
while they are made, so nothing of it can appear in them.

**If you change a view, the images go stale and `readme_images_are_current` fails.** Regenerate
them with:

```sh
cargo test -p stikk-tui regenerate_readme_images -- --ignored
```

Then look at what changed before committing it. Two other tests guard the result: one checks that
no image carries a home-directory path, this machine's user or host name, an email address, an
object id that is not one of the invented ones, the words `seed` or `secret`, or any character the
escape rules would flag; the other plants a fault for each of those rules and fails if the rule
does not fire.

## Before submitting

Run the gates the CI will run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

All three must pass. Keep changes scoped and cite the design items they realize.

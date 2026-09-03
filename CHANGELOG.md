# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## Unreleased

_Nothing yet._

## 0.1.0 — 2026-09-03

The first public release: a **read-only preview** of stikk — orientation, ref history, worktree
changes, and the refusal-explanation / glossary surfaces over the prikk version control system. It was
built increment by increment, design- and security-first, under prikk-grade gates (`fmt` /
`clippy -D warnings` / `test`). It performs **no repository mutations** yet; the working cycle
(commit → seal → merge, always preview-first) lands in later releases.

stikk drives the external `prikk` binary at runtime (not a Cargo dependency): this release is validated
against **prikk ≥ 0.27**, and the worktree-changes view needs **prikk ≥ 0.28**.

### Added

#### Foundation — workspace, kernel, seam, state, operation layer

- **Workspace and lint discipline.** Six crates (Rust 2024, MSRV 1.85) — `stikk-model`, `stikk-prikk`,
  `stikk-state`, `stikk-core`, `stikk-tui`, and the `stikk` launcher — under a virtual-manifest
  workspace. `unsafe` is forbidden; public items must be documented; the panic-prone clippy lints warn.
- **`stikk-model`** — the shared kernel: the `StikkError` taxonomy (seven presentation classes,
  `#[non_exhaustive]`, `source()` implemented — a lesson from the prikk audit); validated
  `ObjectId`/`RefName` newtypes; the nine `RequestCategory` values carrying their policy as data; and
  the `Capability`/`Readiness` derivation.
- **`stikk-prikk` — the prikk seam.** The `Prikk` trait; a `CliBackend` driving the `prikk` binary
  (draining output fully before classifying the exit — the EPIPE guard); a version handshake with a
  validated-range gate; a scripted `NullBackend` for offline testing; and the presence-only
  key-readiness reader.
- **`stikk-state`** — user-scope config, session, and handle stores: a forgiving config parser that
  preserves unknown keys and never blocks launch; repository discovery; and the path resolver's
  repository-internal write refusal.
- **`stikk-core`** — the operation layer both frontends drive, starting with the read-only `orient`.
- **The `stikk` launcher** — `--version`, `--help`, `config check`, `config path`, and opening a
  repository (the TUI on a TTY, a one-shot orientation print off one).

#### Interactive TUI — shell & Orientation (RFC 001)

- **`stikk-tui`** on `ratatui` + `crossterm`: the shell (header, active view, status bar, overlay
  layer), the Orientation view (`VW-01`/`FR-002`), the status bar (`TU-03` — repo, focused ref, queue,
  capability/readiness badges; never a "HEAD"), global key dispatch through a single `Action` seam, the
  light/dark/mono palette (fixed-RGB text so labels stay legible on any terminal theme — NFR-A03), and
  the panic-safe terminal guard.
- **The inert-text primitive** (`C-T2a`): every repository-sourced string is stripped of control
  characters before it reaches a cell.

#### History & inspection (RFC 006)

- The seam grew `history`, `block_state`, and `refs`; `stikk-core` gained `history_view`,
  `block_detail`, and `list_refs`; the app became a **view stack**.
- **History view** (the unsealed queue tier above the sealed block lineage) and **Block detail** (a
  block's metadata + the replayed tip state), with a **ref picker** for the client-side focused ref.
- Block granularity is prikk's ceiling: **Patch detail is deferred behind UD-09** — prikk exposes no
  per-patch content and no `show`/`diff`, so stikk shows lineage and a block's state file list and
  names the gap rather than faking a diff.

#### Explanation & discovery (RFC 007)

- **One class → presentation mapping** in `stikk-core` (`present`), so the TUI and a future GUI cannot
  diverge (ER-03), and a **confined, version-gated failure classifier** in the seam that maps prikk's
  collapsed 0/1 exit to an error class and **degrades an unknown message to a verbatim refusal** (UD-05).
- **The refusal overlay** (`TU-08`): prikk's message verbatim and inert, a plain-language gloss, and
  **stikk-authored next-steps** (never parsed from the message — `C-T2b`), plus glossary links.
- **The glossary asset** (`DM-09`): the Git→prikk terminology mapping, with a missing-code degradation
  that shows prikk's message rather than hiding it.
- **The in-memory session refusal history** (`DM-06`/`FR-112`) and the **command palette** backed by an
  operation registry (`TU-07`/`FR-125`), with below-capability entries shown disabled with a reason.

#### Worktree changes (RFC 008)

- The seam grew `worktree_status`; `stikk-core` gained `changes_view` (version-gated at **prikk ≥ 0.28**,
  with honest guidance below it rather than the pre-fix command).
- **The Changes view** — worktree-vs-baseline at the path level prikk reports
  (modified / missing / untracked / unsupported), with the **UD-08** display-only untracked filter (it
  always says a commit still captures the hidden files), the **UD-06** whole-worktree reminder, and the
  **UD-09** per-file-content-diff note.
- Verified against the live binary that prikk's `worktree-status` is **fixed as of prikk 0.28** (the
  audit's `UD-03` was a 0.27.x defect); a dirty worktree's non-zero exit is treated as a normal status,
  not a refusal. **Compare is deferred** — no honest two-tree command exists, and a partial one would
  mislabel differing files as identical (`T-T4`).

#### Distribution & docs

- **CI** for `fmt` / `clippy` / `test`; a **release** workflow (crates.io via Trusted Publishing, plus
  checksummed, build-provenance-attested binaries for six targets) and a **docs** workflow (mdBook + the
  rustdoc API → GitHub Pages), both least-privilege and tag/branch-gated.
- The workspace is laid out as **six peer crates under `crates/`** with a virtual root manifest, so bare
  `cargo` commands cover every crate by default.

### Security

- The seam reads `PRIKK_*_SEED` **presence only, never their values** — enforced by a source-level
  guard test.
- The path resolver **refuses any repository-internal write target** — the primary boundary control,
  since prikk has no foreign-file backstop.
- Repository/prikk-sourced strings are rendered **inert** (`C-T2a`); refusal next-steps are
  **stikk-authored, never parsed from prikk's message** (`C-T2b`); prikk's message is **preserved
  verbatim** (`ER-02`); and stikk **never fakes a diff** where prikk exposes no content (`T-T4`).

### Notes

- 164 tests pass; `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features
  -D warnings` are clean.
- Deferred behind upstream gaps, recorded as stated properties rather than surprises: Patch detail and
  Compare's content view (`UD-09`), and a two-tree compare command.
- RFCs accepted: 001 (frontend toolkit), 006 (history & inspection), 007 (explanation surface), 008
  (worktree changes). The GUI toolkit and several Program-Design decisions remain deferred by design.

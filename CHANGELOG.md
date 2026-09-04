# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## 0.2.0 — 2026-09-04

The prikk 0.30 re-baseline and parser-fidelity corrections (RFC 009). Running shipped 0.1.0 against a
real prikk 0.30.0 repository found that Orientation refused to open **any repository with queued
work**, at every prikk version stikk claimed to support — not new drift, but a golden fixture that was
written rather than captured. This release fixes that and three related parser defects, closes a live
threat-model violation, and re-baselines the validated prikk range.

### Breaking

For a `0.x` crate the minor version is the breaking position: `^0.1.0` resolves `0.1.1`, so a public
field added on what looked like a patch would break any downstream code constructing these structs with
struct-literal syntax. That is why this release is `0.2.0`, not `0.1.1` (RFC 011). Five structs gained
fields; none is `#[non_exhaustive]` (RFC 011 decides against adding it before 1.0 — see that RFC for the
reasoning):

| Crate | Struct | Field added |
|---|---|---|
| `stikk-prikk` | `Handshake` | `validated: bool` |
| `stikk-prikk` | `Orientation` | `queued_target: Option<String>` |
| `stikk-prikk` | `WorktreeStatus` | `queued_elsewhere: Option<String>` |
| `stikk-core` | `OrientationView` | `queued_target: Option<String>`, `prikk_validated: bool` |
| `stikk-core` | `ChangesView` | `queued_elsewhere: Option<String>` |

If you construct any of these five with struct-literal syntax, add the new field(s) before upgrading.

### Fixed

- **Orientation no longer refuses on queued work** (F1). `prikk status`'s `queued patches: N targeting
  <ref>` line — present since prikk 0.18.0 — is now parsed correctly, and `Orientation`/
  `OrientationView` carry the queue's target ref (`queued_target`), shown as "N queued · targeting
  `<ref>`".
- **An unpublished `heads/main` no longer becomes a fabricated object id** (F2). `status`'s
  `<not published>` sentinel — and any future unrecognized sentinel — is now recognized (or refused,
  never guessed) alongside `log`'s `<none>`.
- **An empty repository no longer produces a phantom ref** (F3). `prikk branch list --all`'s `no
  branches` line used to parse as `RefEntry { name: "no", id: "branches" }`; it is now the empty list.
  `branch list` cannot emit a tag — the seam's doc comments and the ref-picker gap are corrected to say
  so; a real `tag list` read is tracked, not built here.
- **A prikk usage error (exit `2`) no longer wears prikk's voice** (F6). prikk 0.28 split its exit
  contract into success / operational failure / usage error; a bad argument list stikk assembled now
  surfaces as a stikk-internal fault, never as one of prikk's own refusals.

### Security

- **Closed a live confident-but-wrong-picture violation** (F4; threat model `T-T4`/`C-T4c`, tracked as
  `RR-9`). When the active WAL holds queued patches for a ref other than the one being reviewed, prikk's
  `worktree-status` says so explicitly — those "untracked" paths may already be committed, unsealed
  work. Shipped 0.1.0 silently dropped that warning and then showed its own contradicting "a commit
  still captures them" banner. The Changes view now carries prikk's warning verbatim into a distinct
  band above the entries, and suppresses the contradicting claim while the warning is present.
- **`UD-08` retired.** `.prikkignore` shipped in prikk 0.29; the design set, the Changes view's copy,
  and the glossary no longer say prikk has no ignore mechanism. The malformed-`.prikkignore` refusal now
  has a glossary entry and a next-step that can actually resolve it.
- Every golden fixture in `stikk-prikk`'s parser tests is re-captured verbatim from a real prikk 0.30.0
  binary and carries a provenance comment naming the command and version; a regression test enforces
  the rule going forward.

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.30.0`** (owner-ruled 2026-09-04; `0.27.x`
  dropped — its `worktree-status` was already the `UD-03` defect stikk refused to run). A prikk above
  the validated ceiling still runs; Orientation states that its output shapes have not been checked,
  rather than silently assuming they have (`Handshake`/`OrientationView` gain a `validated` field
  alongside `supported`).

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

# Handoff — 0.2.0 release preparation (v1)

**Governing decisions:** [RFC 009](../../done/009-prikk-0-30-rebaseline-and-parser-fidelity.md) (the
content of this release, implemented and approved) and
[RFC 011](../../accepted/011-pre-1-0-api-stability-policy.md) (why the version is **0.2.0**, and why
`#[non_exhaustive]` is *not* part of this increment).
**Realizes:** the owner's release authorization of 2026-09-04. This is a **release-preparation**
increment: no behaviour changes, no new surfaces — the version, the changelog, and three pieces of
user-facing text that are currently wrong.

**Read this first:** every item below is a *correctness* fix to something a user reads. None is
cosmetic polish. The `--help` text in particular tells users the product does not yet do the main thing
it does.

---

## 1. Scope

**In:**
1. Workspace version `0.1.0` → **`0.2.0`**, including the five `workspace.dependencies` path entries.
2. `CHANGELOG.md`: `## Unreleased` → `## 0.2.0 — <release date>`, with a short "why this is a minor,
   not a patch" line.
3. `crates/stikk/src/main.rs`: the `USAGE` text and the module doc comment, both of which still say the
   interactive TUI is a future increment.
4. `README.md`: the `## Project Status` section — four `(todo)` placeholders and a crate table listing
   2 of 6 crates.
5. `docs/src/index.md` and `docs/src/guide/getting-started.md`: the "next increment" status lines and
   the incomplete key reference (see §5).

**Out (do not build here):**
- **`#[non_exhaustive]` on any struct.** RFC 011 decides against it before 1.0 — 75 cross-crate
  construction sites, 13 of them in the runnable examples. Do not add it, and do not add constructors
  or builders in anticipation.
- Any behaviour change, new test of behaviour, or seam/operation/view change. If you find a behaviour
  bug while doing this, **stop and report it** rather than fixing it inside a release-prep commit.
- **The tag, the version-control push, and the crates.io publish** — the owner's, always.

---

## 2. The version bump

`Cargo.toml` at the workspace root, two places:

```toml
[workspace.package]
version = "0.2.0"          # was 0.1.0

[workspace.dependencies]
stikk-model = { version = "0.2.0", path = "crates/stikk-model" }   # and the other four
```

`.github/workflows/release.yml`'s guard job requires **tag == workspace version**, so a `0.2.0` tag
fails unless both are updated. Run `cargo build --locked` afterwards: `Cargo.lock` records the
workspace crates' versions and must be regenerated and committed, or `--locked` fails in CI.

**Why 0.2.0 and not 0.1.1** (state this in the CHANGELOG, briefly): RFC 009 added public fields to
`Handshake`, `Orientation`, `WorktreeStatus`, `WorktreeEntry`, `OrientationView` and `ChangesView`,
none of which is `#[non_exhaustive]`. For a `0.x` crate the minor is the breaking position, so
`^0.1.0` would resolve a `0.1.1` and break any downstream construction of those structs. See RFC 011.

---

## 3. `stikk --help` and the launcher module doc *[the one that matters most]*

`crates/stikk/src/main.rs`:

- **Module doc (line ~5)** still reads "the interactive TUI/GUI render loop is the next increment (its
  toolkit is a Program-Design decision, deliberately not made here); until then, opening a repository
  prints a one-shot orientation."
- **`USAGE` (line ~32)** still reads "The interactive TUI is the next increment; opening a repository
  currently prints a one-shot orientation."

Both were true before RFC 001 shipped and have been false since. The TUI is the primary surface; the
one-shot print is the **non-TTY fallback** (`CL-06`). Rewrite both to say that, and while you are in
`USAGE`, make sure the described behaviour matches what the launcher does today.

**Also check the exit codes documented in the module doc against `CL-05`.** The doc claims "a subset of
external design CL-05" and lists `1` for runtime error, which `CL-05` does not define. Do **not**
change any exit code — just make sure the comment describes what the code does. If they genuinely
disagree with `CL-05`, report it; that is a design question, not a text fix.

---

## 4. `README.md` — the crates.io landing page

`README.md` is what crates.io renders for the `stikk` crate, so its `## Project Status` section is the
first thing a prospective user reads. It currently has:

- a **crate table with 2 of 6 rows** (`stikk`, `stikk-core`), each with `(todo)` as its Purpose;
- a bare `(todo)` after the table;
- a `### Project Structure` heading whose entire body is `(todo)`.

Fill it in:

- **All six crates**, each with a one-line purpose. The `description` field in each crate's
  `Cargo.toml` is already written, accurate, and the right length — reuse those rather than inventing
  new wording.
- **Project Structure**: the layer cake, short. `docs/src/contributing/development.md` already has a
  correct one-paragraph version ("stikk is a five-layer workspace…"); condense or reference it. Do not
  reproduce the internal design's diagram — the README stays concise per the project's own README rule.
- Keep the existing badge/link columns as they are.

---

## 5. Docs status lines

Both still describe shipped work as upcoming:

- `docs/src/index.md` — "Foundation (0.1.0) plus the first interactive surface… **History and Patch
  detail are the next increment.**" History shipped in 0.1.0; Patch detail is deferred behind `UD-09`,
  which is a different statement.
- `docs/src/guide/getting-started.md` — "More views — History, Patch and Block detail — are the next
  increment." Same problem. Its key list also documents only `?`, `r`, `q` and is missing `:` (palette),
  `b` (ref picker), `w` (Changes), `u` (untracked filter), `R` (recent refusals) — a user reading it
  cannot find most of the TUI.

Correct both to describe 0.2.0's actual surface, and name the deferrals as deferrals (`UD-09` for Patch
detail, RFC 008's route for Compare) rather than as "next".

Run `mdbook build` in `docs/` afterwards; the Pages deploy runs on every push to `main`.

---

## 6. Test plan

There is no behaviour to test. What must hold:

- **Gates green**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  --all-features --locked -- -D warnings`, `cargo test --workspace --locked` — **197 tests, unchanged**.
  A changed test count in a release-prep commit means something behavioural moved; stop and report.
- `cargo build --locked` succeeds with the regenerated `Cargo.lock`.
- `cargo build --examples -p stikk-tui --locked` still builds all four demos.
- `mdbook build` in `docs/` succeeds.
- **Read `stikk --help` and the rendered README yourself** before submitting. This increment is
  entirely about what a human reads; a green gate proves none of it.

---

## 7. Acceptance criteria

1. Workspace version and all five path dependencies are `0.2.0`; `Cargo.lock` is regenerated and
   committed; `cargo build --locked` succeeds.
2. `CHANGELOG.md` has a `## 0.2.0 — <date>` section (the existing `## Unreleased` body is already
   accurate — keep it) with a one-line note on why this is a minor rather than a patch.
3. `stikk --help` and the launcher module doc describe the TUI as the primary surface and the one-shot
   print as the non-TTY fallback. No exit code changed.
4. `README.md`'s Project Status lists all six crates with real purposes and a real Project Structure
   note; **no `(todo)` remains in the file**.
5. `docs/src/index.md` and `getting-started.md` describe 0.2.0's actual surface, name the deferrals as
   deferrals, and the guide's key reference is complete.
6. Gates green at **197 tests** (unchanged); examples build; `mdbook build` succeeds.
7. **No struct gained `#[non_exhaustive]`; no constructor or builder was added** (RFC 011).
8. Nothing tagged, pushed, or published.

---

## 8. Submit

The usual package to `.git-exclude/review-request/011-release-0-2-0-preparation/review-request-v1.md`.
Because nothing here is behavioural, the review will be a **reading** review: paste the final
`stikk --help` output verbatim and the rendered Project Status section into the request, so they can be
reviewed as text rather than as a diff.

Then it goes to the owner, who alone bumps nothing further, tags `0.2.0`, and authorizes the publish.

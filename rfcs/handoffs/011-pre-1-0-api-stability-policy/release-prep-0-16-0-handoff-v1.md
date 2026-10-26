# Handoff — 0.16.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The fifteenth release prepared under it.
**Issued** 2026-10-27, **effective on the project owner's authorization of the cut**, which the owner relays; the dev team
prepares by standing permission. The tag and the publish wait for the owner after review, on
[the 0.16.0 release proposal](../../../.git-exclude/release/0-16-0-proposal.md). **Read the proposal first.**
**Covers:** RFCs [050](../../done/050-exchange-bundles.md), [051](../../done/051-arch-linux-packages.md) and
[052](../../done/052-prikk-0-54-rebaseline.md), and RFC 013's Handoff B.
**Follows** [0.15.0's prep handoff](./release-prep-0-15-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.15.0` → `0.16.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines move.**

## 2. `## Unreleased` → `## 0.16.0 — <the date you push>`

- **A bold summary first**: the proposal's sentence, then what a user of 0.15.0 notices — `i`, `I`, `E` and what each
  previews; the Windows installer's prikk line; the AUR packages. Name RFCs 050, 051 and 052 once each. **Count the
  suite's cases.**
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`; omit an empty one.
- **Read it whole as a user who has 0.15.0**: one voice, no review-round vocabulary.

## 3. The Breaking table over `0.15.0..HEAD`

`cargo public-api` per crate and the five-category read. **Four rows are expected** (the proposal's). The Exchange row
has no signature behind it — keep its sentence saying so. List the rejected candidates.

## 4. User-facing text 0.16.0 makes false or incomplete

- **`crates/stikk/README.md` and `docs/src/index.md` `## Status`**, and `releasing.md`'s *"What a v0.15.x release is"*:
  0.16.x — exchange, prikk's own installer on Windows, the AUR. **Read `releasing.md`'s body**: its count of mutations
  (exchange adds two writes) and the seam's method count (twenty-five since RFC 050).
- **The README's feature paragraph** and its two counterparts: **one sentence** for sharing work as a bundle file.
- **The install page's Arch section keeps its *"not published yet"* paragraph.** The architect removes it after the
  release run's AUR job is verified. **Do not remove it in this prep.**
- **`validated through`**: `0.54.1` everywhere; nothing says 0.55 is published.

## 5. `ROADMAP.md`

- **`## Shipped — <0.16.0's summary line> (0.16.0, breaking)`**: increments 1–5, with rollback's withdrawal in one
  sentence.
- **`## Next — 0.16.0 …`** becomes **`## Next — 0.17.0, not yet scoped`**; carry the Homebrew tap (waiting) and
  SignPath (re-entered only with prikk's signing) under it, unchanged.

## 6. RFC stamps

RFCs **050, 051, 052**: *"0.16.0 candidate (on `main`)"* → **`0.16.0`**, in the index and each Status line (051's says
the first publication is by this release's run — keep that). RFC **013**'s row and Status: Handoff B shipped in
**0.16.0**. RFC 011's row gains this prep handoff. Run `rfc_index`.

## 7. Sweeps

`0\.15\.x`, `v0\.15`, `0\.15\.0` (live statements only), `validated through`, `0\.55`, the grep for invisible
characters, decoded escapes **and repeated sections**; `literal_guard` green.

## 8. Submit and runs (no branch)

The eight gates, the local suite at 0.28.0 + 0.54.1, and `release.yml`'s `awk` on the final `CHANGELOG.md`. Package to
`.git-exclude/review-request/011-release-0-16-0-preparation/review-request-v1.md`, leading with the extracted section.
**After approval:** push. Dispatch the **full-matrix** suite (`full_platform_matrix: true` — the bundle legs on macOS
and Windows for the first time), the supply-chain gate, `installers.yml`, and **`aur.yml` with `push` unchecked**
(the version input left at its default) on `main`. Report every run id and leg at that SHA, **the SHA to be tagged**.
**A red non-Linux leg stops the cut.**

# Handoff — 0.14.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The thirteenth release prepared under it.
**Issued** 2026-10-18, **effective on the project owner's authorization of the cut**, which the owner relays; the dev team
prepares by standing permission. The tag and the publish wait for the owner after review, on
[the 0.14.0 release proposal](../../../.git-exclude/release/0-14-0-proposal.md). **Read the proposal first.**
**Covers:** RFC [046](../../done/046-session-persistence.md), RFC 040's Handoff C and RFC 023's Handoff C.
**Follows** [0.13.0's prep handoff](./release-prep-0-13-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.13.0` → `0.14.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines move.**

## 2. `## Unreleased` → `## 0.14.0 — <date>`

- **A bold summary first**: the proposal's sentence, then what a user of 0.13.0 notices — the resume, re-checked
  against prikk; `STIKK_PRIVATE=1`; where the record lives and that deleting it is safe. Name RFC 046 once. **Count the
  suite's cases.** Mention the `C-E2` hardening **only as what now holds**, not as a vulnerability: no release ever
  wrote through it, so there is no `### Security` entry.
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`; omit an empty one.
- **Read it whole as a user who has 0.13.0**: one voice, no review-round vocabulary.

## 3. The Breaking table over `0.13.0..HEAD`

`cargo public-api` per crate and the five-category read. **One row is expected** (`stikk_tui::run`'s fourth argument).
`ensure_outside_repository`'s **behaviour** changed (it now refuses paths it used to allow): decide whether that is a
Breaking row for an outside caller of `stikk-state` and say why either way. List the rejected candidates.

## 4. User-facing text 0.14.0 makes false or incomplete

- **The manual-install section** (README and guide): `0.13.0` → **`0.14.0`** in every archive name, URL, checksum line
  and *"It prints `stikk 0.14.0`"*.
- **`crates/stikk/README.md` `## Status`**, **`docs/src/index.md` `## Status`**, **`releasing.md`**'s *"What a v0.13.x
  release is"*: 0.14.x — and **read `releasing.md`'s body** as you did last time.
- **The README's feature paragraph** and its two counterparts: **one sentence** that stikk reopens each repository where
  you left it (re-checked against prikk, stored outside it, `STIKK_PRIVATE=1` to opt out).
- **`validated through`** still `0.52.0` everywhere; nothing says 0.53 is published. **The README images' guard test**
  green.

## 5. `ROADMAP.md`

- **`## Shipped — <0.14.0's summary line> (0.14.0, breaking)`**: increments 1–3.
- **`## Next — 0.14.0 …`** becomes **`## Next — 0.15.0, not yet scoped`**: the package manager (the owner's decision),
  and nothing else — the architect scopes 0.15.0.

## 6. RFC stamps

RFC **046**: *"0.14.0 candidate (on `main`)"* → **`0.14.0`**, in the index and its Status line. RFC 011's row gains this
prep handoff. Run `rfc_index`.

## 7. Sweeps

`0\.13\.x`, `v0\.13`, `0\.13\.0` (live statements), `validated through`, `0\.53`, the grep for invisible characters,
decoded escapes **and repeated sections**.

## 8. Submit and runs (no branch)

The eight gates, the local suite at 0.28.0 + 0.52.0, and `release.yml`'s `awk` on the final `CHANGELOG.md`. Package to
`.git-exclude/review-request/011-release-0-14-0-preparation/review-request-v1.md`, leading with the extracted section.
**After approval:** push. Dispatch the **full-matrix** suite (`full_platform_matrix: true` — RFC 046's legs on macOS and
Windows for the first time), the supply-chain gate and `installers.yml` on `main`. Report every run id and leg at that
SHA, **the SHA to be tagged**. **A red non-Linux leg stops the cut**: report it, do not fix it under this handoff.

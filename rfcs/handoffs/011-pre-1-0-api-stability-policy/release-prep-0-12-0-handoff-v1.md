# Handoff — 0.12.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The eleventh release prepared under it.
**Issued** 2026-10-15, on the project owner's authorization of the cut; the dev team prepares by standing permission. The
tag and the publish wait for the owner after review, on
[the 0.12.0 release proposal](../../../.git-exclude/release/0-12-0-proposal.md). **Read the proposal first.**
**Covers:** RFCs [042](../../done/042-readme-key-visuals.md) and [043](../../done/043-prikk-0-52-rebaseline.md).
**Follows** [0.11.0's prep handoff](./release-prep-0-11-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **A small release.** `## Unreleased` is 53 lines. **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace
> entries.** Anything else: stop and report. **No branch.**

---

## 1. The bump

`0.11.0` → `0.12.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines move.**

## 2. `## Unreleased` → `## 0.12.0 — <date>`

- **A bold summary first**: the proposal's line, then what a user of 0.11.0 notices, naming each RFC once. **Count the
  suite's cases.**
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`. **No `### Security`** unless prep finds
  a fixed weakness that shipped (none is known). If you believe otherwise, stop and say why.
- **Read it as a user who has 0.11.0.**

## 3. The Breaking table over `0.11.0..HEAD`

`cargo public-api` per crate and the five-category read. **One row is expected**, `patch_detail_in_block`'s fifth
argument. Confirm it, and list the rejected candidates.

## 4. User-facing text 0.12.0 makes false

- **The README's and guide's manual-install section names `0.11.0`** in every archive name and URL, and in *"It prints
  `stikk 0.11.0`"*. **Move every one to `0.12.0`.**
- **`crates/stikk/README.md` `## Status`**, **`docs/src/index.md` `## Status`** and **`releasing.md`**'s *"What a v0.11.x
  release is"*: 0.12.x.
- **`validated through`**: every live statement says `0.52.0` already (RFC 043). **Confirm it.**
- **The README images** show no version, so they need nothing. **Confirm their guard test is green.**

## 5. `ROADMAP.md`

- **`## Shipped — <0.12.0's summary line> (0.12.0, breaking)`**: RFC 043 and RFC 042.
- **`## Next — carried into 0.12.0` → `carried into 0.13.0`**: remove the two shipped items. Keep the `wrap_hanging`
  consolidation, then package managers.

## 6. RFC stamps

RFCs **042** and **043**: *"0.12.0 candidate (on `main`)"* → **`0.12.0`**, in the index and their Status lines. Run
`rfc_index`.

## 7. Sweeps

`0\.11\.x`, `v0\.11`, `0\.11\.0` (live statements, the manual-install section especially), `validated through` (all
`0.52.0`), `0\.53` (nothing says it is published), and the grep for invisible characters, decoded escapes **and repeated
sections**.

## 8. Submit and runs (no branch)

The eight gates, the local suite at 0.28.0 + 0.52.0, and `release.yml`'s `awk` on the final `CHANGELOG.md`. Package to
`.git-exclude/review-request/011-release-0-12-0-preparation/review-request-v1.md`, leading with the extracted section.
**After approval:** push, then dispatch the suite, supply chain and `installers.yml` on `main`, and report every run id
at that SHA, **the SHA to be tagged**.

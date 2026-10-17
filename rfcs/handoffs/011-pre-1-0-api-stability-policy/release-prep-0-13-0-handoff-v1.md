# Handoff — 0.13.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The twelfth release prepared under it.
**Issued** 2026-10-17, **effective on the project owner's authorization of the cut**, which the owner relays; the dev team
prepares by standing permission. The tag and the publish wait for the owner after review, on
[the 0.13.0 release proposal](../../../.git-exclude/release/0-13-0-proposal.md). **Read the proposal first.**
**Covers:** RFCs [044](../../done/044-branches-and-tags.md) and [045](../../done/045-switch-branch.md), and RFC 040's
`wrap_hanging` carry.
**Follows** [0.12.0's prep handoff](./release-prep-0-12-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.12.0` → `0.13.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines move.**

## 2. `## Unreleased` → `## 0.13.0 — <date>`

- **A bold summary first**: the proposal's sentence, then what a user of 0.12.0 notices, naming RFC 044 and RFC 045
  once each, **with the keys** (`n`, `t`, `x`, `s`) and the two prikk floors (0.42, 0.46). **Count the suite's cases.**
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`. No `### Security` unless prep finds a
  fixed weakness that shipped (none is known); if you believe otherwise, stop and say why.
- `## Unreleased` is ~90 lines across three increments, written as each landed. **Read it whole as a user who has
  0.12.0**: one voice, no entry contradicting a later one (RFC 044's refusal wording changed in RFC 045 — the
  changelog must state only the final wording), no review-round vocabulary (*C1*, *v2*).

## 3. The Breaking table over `0.12.0..HEAD`

`cargo public-api` per crate and the five-category read. The proposal lists the expected rows; **confirm each, add any
it missed, and list the rejected candidates.**

## 4. User-facing text 0.13.0 makes false or incomplete

- **The manual-install section** (README and guide) names `0.12.0` in every archive name and URL, and in *"It prints
  `stikk 0.12.0`"*: **`0.13.0`**.
- **`crates/stikk/README.md` `## Status`**, **`docs/src/index.md` `## Status`** and **`releasing.md`**'s *"What a
  v0.12.x release is"*: 0.13.x.
- **The README's feature paragraph** (`README.md`, the one ending *"and commit or seal behind preview-first
  confirmation"*) and its counterparts in `crates/stikk/README.md` and the guide: add, in the same register, **creating
  a branch or a tag, closing a branch, and switching branch from the ref picker, each previewed before it runs**.
  Name the floors as that paragraph names others. **One sentence, not a section.**
- **`validated through`**: every live statement says `0.52.0`. **Confirm it.** Nothing says 0.53 is published.
- **The README images' guard test** is green (no image shows a key list or version). **Confirm.**

## 5. `ROADMAP.md`

- **`## Shipped — <0.13.0's summary line> (0.13.0, breaking)`**: the tidy, RFC 044, RFC 045.
- **`## Next — 0.13.0 …`** becomes **`## Next — 0.14.0, not yet scoped`**: the package-manager decision (the owner's),
  then session persistence (`FR-122`), as the 0.13.0 scope's *Out of scope* table ranks them. The architect scopes
  0.14.0; do not write more than those two lines.

## 6. RFC stamps

RFCs **044** and **045**: *"0.13.0 candidate (on `main`)"* → **`0.13.0`**, in the index and their Status lines. RFC 040's
row: its carry is shipped. Run `rfc_index`.

## 7. Sweeps

`0\.12\.x`, `v0\.12`, `0\.12\.0` (live statements, the manual-install section especially), `validated through` (all
`0.52.0`), `0\.53`, the Glossary lists `n` `t` `x` `s`, and the grep for invisible characters, decoded escapes **and
repeated sections**.

## 8. Submit and runs (no branch)

The eight gates, the local suite at 0.28.0 + 0.52.0, and `release.yml`'s `awk` on the final `CHANGELOG.md`. Package to
`.git-exclude/review-request/011-release-0-13-0-preparation/review-request-v1.md`, leading with the extracted section.
**After approval:** push. CI and Docs run on push. Dispatch the **full-matrix** suite (`full_platform_matrix: true` —
the first time RFC 045's legs, which write files, run on macOS and Windows), the supply-chain gate and `installers.yml`
on `main`. Report every run id and leg at that SHA, **the SHA to be tagged**. **A red non-Linux leg stops the cut**:
report it, do not fix it under this handoff.

# Handoff — 0.15.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The fourteenth release prepared under it.
**Issued** 2026-10-20, **effective on the project owner's authorization of the cut**, which the owner relays; the dev team
prepares by standing permission. The tag and the publish wait for the owner after review, on
[the 0.15.0 release proposal](../../../.git-exclude/release/0-15-0-proposal.md). **Read the proposal first.**
**Covers:** RFCs [047](../../done/047-merge.md), [048](../../done/048-delivery-for-new-users.md) and
[049](../../done/049-prikk-0-53-rebaseline.md), and RFC 034's Handoff C.
**Follows** [0.14.0's prep handoff](./release-prep-0-14-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.14.0` → `0.15.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines move.**

## 2. `## Unreleased` → `## 0.15.0 — <date>`

- **A bold summary first**: the proposal's sentence, then what a user of 0.14.0 notices — merging (mark, select, `M`,
  then `Enter`), the two-step install with prikk's own command, and ARM Linux. Name RFCs 047 and 048 once each. **Count
  the suite's cases.**
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`; omit an empty one.
  **Today they are out of order** (`Added`, `Changed`, `Fixed`, `Removed`, `Breaking`): reorder.
- **Read it whole as a user who has 0.14.0**: one voice, no review-round vocabulary (*C1*, *v2*, *Handoff*).

## 3. The Breaking table over `0.14.0..HEAD`

`cargo public-api` per crate and the five-category read. **Two rows are expected** (`Prikk::merge_plan`/`merge`;
`target_ids: Vec<TargetId>`). List the rejected candidates. `RequestCategory` and `ConfirmationSummary` are
`#[non_exhaustive]`; say whether any new field or variant changes that reading.

## 4. User-facing text 0.15.0 makes false or incomplete

- **`crates/stikk/README.md:63`** and **`docs/src/index.md:23`** `## Status`: *"v0.14.x installs in one command, prikk
  included …"* — **false since RFC 048**. Rewrite for 0.15.x: two steps, prikk's then stikk's; merge.
  **`releasing.md`**'s *"What a v0.14.x release is"*: 0.15.x, and **read its body**.
- **The README's feature paragraph** and its two counterparts: **one sentence** that stikk merges a branch into another
  when prikk proves it confluent (previewed, never into the current branch).
- **The install page and README** carry no version since RFC 048 A; **check** nothing re-introduced one, and that the
  *"What you should see"* capture's `0.14.0` is labelled as an example (*"the exact versions will differ"*) — it is;
  leave it.
- **`validated through`**: `0.53.0` everywhere; nothing says 0.54 is published. **The README images' guard test**
  green.

## 5. `ROADMAP.md`

- **`## Shipped — <0.15.0's summary line> (0.15.0, breaking)`**: increments 1–5.
- **`## Next — 0.15.0 …`** becomes **`## Next — 0.16.0, not yet scoped`**: **move the three decision points** from
  *Later* (SignPath, the tap, the prikk 0.54.0 re-baseline) under it, unchanged — the architect scopes 0.16.0.
- The *Shipped — 0.11.0* heading says *"installs in one command, prikk included"*: **leave it** — it was true of 0.11.0.

## 6. RFC stamps

RFCs **047, 048, 049**: *"0.15.0 candidate (on `main`)"* → **`0.15.0`**, in the index and in each Status line. RFC
**034**'s row and Status: Handoff C shipped in **0.15.0**. RFC 011's row gains this prep handoff. Run `rfc_index`.

## 7. Sweeps

`0\.14\.x`, `v0\.14`, `0\.14\.0` (live statements only), `with-prikk`, `no-prikk`, `prikk included`, `validated
through`, `0\.54`, the grep for invisible characters, decoded escapes **and repeated sections**.

## 8. Submit and runs (no branch)

The eight gates, the local suite at 0.28.0 + 0.53.0, and `release.yml`'s `awk` on the final `CHANGELOG.md`. Package to
`.git-exclude/review-request/011-release-0-15-0-preparation/review-request-v1.md`, leading with the extracted section.
**After approval:** push. Dispatch the **full-matrix** suite (`full_platform_matrix: true` — the merge legs on macOS
and Windows for the first time), the supply-chain gate and `installers.yml` on `main`. **`installers.yml`'s ARM job
will log *"archive named for the published 0.14.0; the binary in it is 0.15.0"*** — that is the case RFC 048 B's review
fixed; its version assertion must pass. Report every run id and leg at that SHA, **the SHA to be tagged**. **A red
non-Linux leg stops the cut**: report it, do not fix it under this handoff.

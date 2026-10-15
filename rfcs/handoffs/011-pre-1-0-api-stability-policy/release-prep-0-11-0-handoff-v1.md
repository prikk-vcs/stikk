# Handoff — 0.11.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The tenth release prepared under it.
**Issued** on the project owner's authorization of the cut; the dev team prepares by standing permission. The tag and
the publish wait for the owner after review, on [the 0.11.0 release proposal](../../../.git-exclude/release/0-11-0-proposal.md).
**Read the proposal first.**
**Covers:** RFCs [039](../../done/039-installation-for-everyone.md), [040](../../done/040-verify.md) and
[041](../../done/041-compare-two-refs.md), and RFC 037's Handoff C (the carried polish).
**Follows** [0.10.0's prep handoff](./release-prep-0-10-0-handoff-v1.md); its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything that changes behaviour, a test
> assertion or a workflow: **stop and report.** **No branch**: runs on `main` after approval.

---

## 1. The version bump

`0.10.0` → `0.11.0` in `[workspace.package]` and the five pins; `cargo update --workspace`; **only `stikk*` lines may
move in `Cargo.lock`.**

## 2. `## Unreleased` → `## 0.11.0 — <date>`

- **A bold summary first**: the proposal's line, then what a user of 0.10.0 will notice, naming each RFC once. **Lead
  with installation**, because it is what a new user meets first. **Count the suite's cases** (51 at `58ca970`).
- **Headings in order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`. **No `### Security` unless prep
  finds a fixed weakness that shipped**; the installers' checksums are a property of a new feature, not a fix. If you
  believe otherwise, stop and say why.
- **Merge entries one release now makes one story**: Handoff A and Handoff B of RFC 039 (the README text that B
  rewrote at the top must read as one install section, not two); Verify's two commits.
- **Read it as a user who has 0.10.0.**

## 3. The Breaking table, verified once over `0.10.0..HEAD`

`cargo public-api` per crate, plus the five-category read, as 0.10.0's prep did. **Five rows today**; confirm each,
merge where one type moved twice, and list the rejected candidates. `OperationContext` is `#[non_exhaustive]`, and
`keys`/`worker`/`view` are private.

## 4. User-facing text that 0.11.0 makes false

- **The README's and guide's manual-install section names `0.10.0` in every file name and URL** (`stikk-0.10.0-…`,
  `/download/0.10.0/`, *"prints `stikk 0.10.0`"*). **Move every one to `0.11.0`.** The one-line installer carries no
  version and needs nothing.
- **The Windows checksum instruction** works for both formats (RFC 039 B measured it); keep it.
- **`crates/stikk/README.md`**, `## Status`: a 0.11.x statement that leads with one-command install and Verify.
- **`docs/src/index.md`**, `## Status`, and **`docs/src/contributing/releasing.md`**'s *"What a v0.10.x release is"*:
  0.11.x. **Re-verify the unbuilt list against the trait** (fifteen methods now, with `verify`).
- **Read `README.md`, `crates/stikk/README.md` and the guide end to end.**

## 5. `ROADMAP.md`

1. **`## Shipped — <0.11.0's summary line> (0.11.0, breaking)`** after 0.10.0's: one bullet each for RFCs 039, 040 and
   041, and the polish.
2. **`## Next — 0.11.0 …` becomes `## Next — carried into 0.12.0, in order`**: remove the shipped items. Keep **the prikk
   0.52.0 re-baseline** first (unconditional; it waits only on prikk publishing), then **`block.rs`'s `said` to
   `wrap_hanging`** (RFC 040's carry), then **package managers** (the owner's decision). Keep each item's text.
3. **`Later`** unchanged.

## 6. RFC stamps and the deferred column

`rfcs/README.md`'s Done rows for **039, 040, 041**, and their Status lines: *"0.11.0 candidate (on `main`)"* →
**`0.11.0`**. **In the Deferred column, mark what this release resolved as `resolved in 0.11.0:`**, in place, as the
0.10.0 prep did: RFC 036's `queued_elsewhere_clauses`, RFC 037's column-0 wrap, and RFC 038's two-ref comparison. Run
`rfc_index` after.

## 7. Sweeps

`0\.10\.x`, `v0\.10`, `0\.10\.0` (live statements of the current version; **the README's manual-install section
especially**), `validated through` (all `0.51.0`), `0\.52` (nothing says prikk 0.52 is published), `not built yet`
(two-ref comparison appears in none), and the grep for invisible characters and decoded escapes.

## 8. Gates, runs, submit (no branch)

The eight gates and the local suite, and **`release.yml`'s own `awk`** on the final `CHANGELOG.md` (line count and
headings). Package to `.git-exclude/review-request/011-release-0-11-0-preparation/review-request-v1.md`, leading with
the `## 0.11.0` section **as the `awk` extracts it**, then `crates/stikk/README.md` as crates.io will show it, the
Breaking table's rejected candidates, and the lock diff.

**After approval:** push. CI and Docs run on push. Dispatch the full-matrix suite, the supply-chain gate **and
`installers.yml`** on `main`, and report every run id at that one SHA, **the SHA to be tagged**.

**This release's tag run is the installers' first real publication.** After the tag, the architect checks the four
scripts and their `.sha256` files, the six archive checksums in one format, a live `install.sh` run, `cargo binstall
stikk --version 0.11.0` downloading, and the attestation.

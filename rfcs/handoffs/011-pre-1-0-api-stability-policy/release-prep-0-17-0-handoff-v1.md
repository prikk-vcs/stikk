# Handoff — 0.17.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). This is the sixteenth release prepared under it.
**Issued** 2026-10-29. **The project owner authorized the cut** the same day; the dev team prepares it by standing
permission. The tag and the publish wait for the owner after review, on the 0.17.0 release proposal
(`.git-exclude/release/0-17-0-proposal.md`). **Read the proposal first.**
**Covers:** RFCs [053](../../done/053-get-files-back.md), [054](../../done/054-trust-read-only.md) and
[055](../../done/055-prikk-0-55-rebaseline.md), plus three carried text items (§5).
**Follows** [0.16.0's prep handoff](./release-prep-0-16-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Comments in `installers.yml` count as
> text (§5). Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.16.0` → `0.17.0` in `[workspace.package]` and the five pins. Run `cargo update --workspace`; **only `stikk*` lines
may move.**

## 2. `## Unreleased` → `## 0.17.0 — <the date you push>`

- **Start with a bold summary.** First the proposal's sentence, then what a user of 0.16.0 notices: `F` and `A` in
  Block detail and where they write; `T`; the bundle card's sealer rows and what *names* means. Name RFCs 053, 054 and
  055 once each, and **count the suite's cases**.
- **Headings in this order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`. Omit any that are
  empty.
- **Read it whole as a user who has 0.16.0.** It should read in one voice, with no review-round vocabulary. *"As
  amended"* and *"decision 3"* belong in RFCs, not here.

## 3. The Breaking table over `0.16.0..HEAD`

Run `cargo public-api` per crate, then the five-category read.

- **Three rows are expected**, as the proposal lists them.
- Check that the Additive list matches what `public-api` adds. RFC 054's renames (`NamesATrustedKey`, `NamesYourKey`)
  were never released, so they are additive, not Breaking.
- List the candidates you rejected.

## 4. User-facing text that 0.17.0 makes false or incomplete

- **`## Status` in `crates/stikk/README.md` and `docs/src/index.md`, and `releasing.md`'s *"What a v0.16.x release
  is"*:** each becomes 0.17.x. Add getting files back, the Trust view, and the sealer rows. Each program still
  installs itself.
  - **Remove the *"Trust & Keys view"* from their lists of unbuilt things.** Name what is still missing instead: adding
    and removing keys.
  - **Read `releasing.md`'s body** and correct its counts. There are **eleven mutations** (save a file and save a
    block's files join the nine; both are Exchange, `files.rs`). The seam has **thirty methods**; add `file_meta`,
    `save_file`, `extract_plan`, `extract` and `trusted_keys` to its list.
- **The feature paragraph in each README**, and its counterparts: **one sentence** each for getting a file back and
  for the Trust view.
- **The AUR, in `crates/stikk/README.md`.** Replace *"…and is on the AUR"* (`:65`, frozen on crates.io since 0.16.0)
  with the state at the cut, in the shape `docs/src/index.md` uses: the packages are on the way, and the install page
  says when they are published. **Keep the install page's *"not published yet"* paragraph.** The architect removes it
  after verifying that the packages are live.
- **`validated through`:** `0.55.0` everywhere.
- **The manual-install section of each README:** sweep for any version-bearing file name or URL. The installers use
  `latest`, so none is expected; confirm it.

## 5. The carried text items

1. **`installers.yml:552-553`:** the ARM job reasons from *"The published 0.14.0 has no aarch64 musl archive, so until
   0.15.0 is out there is nothing to download"*. Since 0.15.0, every release publishes that archive. Keep the job's
   real reason, which is that it builds at the commit under test so the installer and the build are tested together,
   and drop the history.
2. **`installers.yml:603`:** *"the prep commit sets the workspace to 0.15.0 while 0.14.0 is still the latest
   release"*. Say it without the numbers: the prep commit sets the workspace to the next version while the previous
   one is still the latest release. **`installers.yml` runs on this push**, because it changed. Report its run.
3. **`requirements.md:122`, `FR-030`'s note:** *"prikk has said it will fix the resolution point in 0.52.0 (A2) — not
   published as of this release, so the label stands…"*. Reword it as dated history. prikk fixed it in 0.52.0, and RFC
   043 bands the *"as at the end of this block"* label, so it shows only below 0.52.

## 6. `ROADMAP.md`

- **Add `## Shipped — <0.17.0's summary line> (0.17.0, breaking)`** with increments 1–3, and one sentence on decision 3's
  amendment: a bundle names its sealer, and Verify proves it.
- **`## Next — 0.17.0 …`** becomes **`## Next — 0.18.0, not yet scoped`**. Carry SignPath, the Homebrew tap and the
  AUR dispatch under it, unchanged. Move the trust write half (`FR-103`) out of *Later* into it **only as "the
  candidate"**; the scope is the owner's to authorize.

## 7. RFC stamps

- **RFCs 053, 054 and 055:** change *"0.17.0 candidate (on `main`)"* to **`0.17.0`**, in the index and in each Status
  line.
- **RFC 011's row** gains this prep handoff.
- Run `rfc_index`.

## 8. Sweeps

- **Version strings:** `0\.16\.x`, `v0\.16` and `0\.16\.0` in live statements only (history stays).
- **Other strings:** `validated through`, `Trust & Keys`, and `is on the AUR`.
- **Hidden damage:** the grep for invisible characters, for decoded escapes, **and for repeated sections**.
- `literal_guard` must be green.

## 9. Submit and runs (no branch)

**Before submitting:** run the eight gates, the local suite at 0.28.0 + 0.55.0, and `release.yml`'s `awk` on the final
`CHANGELOG.md`.

**Package it** to `.git-exclude/review-request/011-release-0-17-0-preparation/review-request-v1.md`, leading with the
extracted section.

**After approval:**

1. Push.
2. On `main`, dispatch:
   - the **full-matrix** suite (`full_platform_matrix: true`);
   - the supply-chain gate;
   - **`aur.yml` with `push` unchecked**, leaving the version input at its default.
3. `installers.yml` fires on the push itself. Report every run id and every leg at that SHA, which is **the SHA to be
   tagged**.

**A red non-Linux leg stops the cut.**

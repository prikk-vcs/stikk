# Handoff — 0.20.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). This is the nineteenth release prepared
under it.
**Issued** 2026-10-31. **The project owner authorized the cut** the same day, and the dev team prepares it by standing
permission. The tag and the publish wait for the owner after review, on the 0.20.0 release proposal
(`.git-exclude/release/0-20-0-proposal.md`). **Read the proposal first.**
**Covers:** [RFC 058](../../done/058-undo-one-block.md). Increment 2 (the prikk 0.56.0 re-baseline) is **not** in this
release: prikk is still at 0.55.0.

**Follows** [0.19.0's prep handoff](./release-prep-0-19-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.19.0` → `0.20.0` in `[workspace.package]` and the five pins. Run `cargo update --workspace`; **only `stikk*` lines
may move.**

## 2. `## Unreleased` → `## 0.20.0 — <the date you push>`

- **The entries are already there** (RFC 058's `### Added` and `### Breaking`). Keep the `### Breaking` table's three
  rows and their note as they stand.
- **Start with a bold summary.** Open with the proposal's sentence, then say what a 0.19.0 user notices:
  - `U` in History undoes a block, and history keeps both;
  - it is previewed file by file, and confirmed once;
  - sealing stays the next step;
  - it refuses rather than overwrite or record anything the user did not choose.

  Name RFC 058 once, and **count the suite's cases**.
- **Headings in this order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`. Leave out any that
  would be empty.
- **Read it whole as a user who has 0.19.0.**

## 3. The Breaking table over `0.19.0..HEAD`

**I ran `cargo public-api -p <crate> diff 0.19.0` on `12f26e6`.** It shows:

- **no removed and no changed item** in any crate;
- three additions that break a dependent, which are the changelog's three rows:
  - `Prikk::rename_path`;
  - `ConfirmationSummary`'s two fields;
  - `Overlay`'s three variants;
- **no change at all** in `stikk-model`, `stikk-state` or the launcher.

**Re-run it yourself on the prep commit** and confirm. Check the Additive list against what `public-api` adds, and list
the candidates you rejected.

## 4. User-facing text 0.20.0 makes false or incomplete

- **The status texts become 0.20.x.** These are `## Status` in `crates/stikk/README.md:64` and `docs/src/index.md:23`,
  and `releasing.md`'s *"What a v0.19.x release is"* (`:87`, `:89`). Each needs **one clause** for what is new:
  *undoes a block, as a new change*.
- **The counts change:**
  - the mutations go from thirteen to **fourteen**: add **undo** to the list in `crates/stikk/README.md:97` and
    `releasing.md:98`, in the list's own form;
  - the seam methods go from thirty-two to **thirty-three**, adding `rename_path` (`releasing.md:126`).

  **Recount both rather than trust these numbers.**
- **The AUR:** the crate README's *"on the way"* and the install page's *"not published yet"* both stand.
- **Sweep each README's manual-install section** for any version in a file name or URL. None is expected.

## 5. `ROADMAP.md`

- **Add `## Shipped — <0.20.0's summary line> (0.20.0, breaking)`** with increment 1. Say in one line that increment 2
  (the prikk 0.56.0 re-baseline) moves to 0.21.0, because prikk has not released 0.56.0.
- **Change `## Next — 0.20.0: undo one block` to `## Next — 0.21.0, not yet scoped`.**
  - Carry the re-baseline as **the one named candidate**, with its 0.56.0 contents unchanged.
  - Keep *"prikk's answers to letter 023"* and the decision points (SignPath, the Homebrew tap, the AUR) under it,
    unchanged.
  - **Name nothing else.** The scope is the owner's to authorize.
- **Update the *Later* entries this release touched.** Rollback (`FR-083`) stays. If a *Later* line implies stikk has
  no way to undo, add one clause pointing at undo (`FR-085`).

## 6. RFC stamps

- **RFC 058:** change *"0.20.0 candidate"* to **`0.20.0`**, in the index and in its Status line.
- **RFC 011's row:** add this prep handoff.
- Run `rfc_index`.

## 7. Sweeps

- **Version strings:** `0\.19\.x`, `v0\.19` and `0\.19\.0`, in live statements only.
- **Other strings:** `validated through` (still `0.55.0`).
- **The standing checks:** the grep for invisible characters, for decoded escapes, **and for repeated sections**.
- **`literal_guard`** must be green.

## 8. Submit and runs (no branch)

**Before submitting:** run the eight gates, the **local real-binary suite at 0.28.0 + 0.55.0**, and `release.yml`'s
`awk` on the final `CHANGELOG.md`.

**Package it** to `.git-exclude/review-request/011-release-0-20-0-preparation/review-request-v1.md`, leading with the
extracted section.

**After approval:**

1. Push.
2. On `main`, dispatch:
   - the **full-matrix** suite (`full_platform_matrix: true`);
   - the supply-chain gate;
   - `installers.yml`, which does not fire on this push;
   - **`aur.yml` with `push` unchecked**.
3. Report every run id and every leg at that SHA, **the SHA to be tagged**.

**A red non-Linux leg stops the cut.**

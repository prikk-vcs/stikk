# Handoff — 0.19.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). This is the eighteenth release prepared
under it.
**Issued** 2026-10-31. **The project owner authorized the cut** the same day, and the dev team prepares it by standing
permission. The tag and the publish wait for the owner after review, on the 0.19.0 release proposal
(`.git-exclude/release/0-19-0-proposal.md`). **Read the proposal first.**
**Covers:**
- [RFC 057](../../done/057-orientation-on-status-json.md);
- RFC 056's [Handoff B](../056-trust-adopt-and-remove/b-typed-card-layout-handoff-v1.md);
- RFC 053's [Handoff C](../053-get-files-back/c-going-back-guide-handoff-v1.md).

**Follows** [0.18.0's prep handoff](./release-prep-0-18-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.18.0` → `0.19.0` in `[workspace.package]` and the five pins. Run `cargo update --workspace`; **only `stikk*` lines
may move.**

## 2. `## Unreleased` → `## 0.19.0 — <the date you push>`

- **Two entries are missing, because the increments did not write them.** Add both in a user's words:
  - **`### Fixed`** (Handoff B): a long key id is no longer cut off in the typed confirmation, where it must be typed
    back; a value split across rows splits evenly, so a public key shows as 32 + 32; and an id made of hidden-character
    escapes no longer overflows its row, which matters because a bundle's sealer id is chosen by its sender.
  - **`### Added`** (Handoff C): the guide's *"Going back to an earlier block"*, with the three keys and what they
    leave behind.
- **Start with a bold summary.** Open with the proposal's sentence, then say what a 0.18.0 user notices:
  - Orientation reads prikk's JSON status;
  - two lines are now stikk's words with prikk's numbers and commands;
  - the commit card's commands are in backticks;
  - the typed card holds a long id;
  - the guide section.

  Name RFC 057 once, and **count the suite's cases**.
- **Headings in this order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`. Leave out any that
  would be empty.
- **Read it whole as a user who has 0.18.0.**

## 3. The Breaking table over `0.18.0..HEAD`

**I ran `cargo public-api -p <crate> diff 0.18.0` on `2f654c2`.** It shows:

- **two rows**: `stikk-prikk`'s `Orientation` (two field types) and `stikk-core`'s
  `OrientationView::interrupted_materialization`;
- **no removed item** in any crate;
- **no change at all** in `stikk-model`, `stikk-state` or `stikk-tui`.

The changelog already says this. **Re-run it yourself on the prep commit** and confirm. Check the Additive list
against what `public-api` adds, and list the candidates you rejected.

## 4. User-facing text 0.19.0 makes false or incomplete

- **The status texts become 0.19.x.** These are `## Status` in `crates/stikk/README.md:64` and `docs/src/index.md:23`,
  and `releasing.md`'s *"What a v0.18.x release is"* (`:87`, `:89`). Each needs **one clause** for what is new and
  visible: the typed card holds a long id, and the guide shows the way back. Orientation's JSON read is not a feature,
  so it does not go in a status line.
- **The counts stand:** thirteen mutations and thirty-two seam methods. No seam method was added. **Confirm both rather
  than assume them.**
- **The AUR:** the crate README's *"on the way"* and the install page's *"not published yet"* both stand.
- **Sweep each README's manual-install section** for any version in a file name or URL. None is expected.

## 5. `ROADMAP.md`

- **Add `## Shipped — <0.19.0's summary line> (0.19.0, breaking)`** with increments 1–3. Say in one line that
  increment 4 (a re-baseline) was not needed.
- **Change `## Next — 0.19.0 …` to `## Next — 0.20.0, not yet scoped`.**
  - Carry SignPath, the Homebrew tap and the AUR dispatch under it, unchanged.
  - Carry *"a consolidated letter to prikk is being drafted"* as one line.
  - **Name no candidate**: 0.20.0 depends on prikk's answers. The scope is the owner's to authorize.
- **Update the *Later* entries this release touched**, if any still describe the old state. The Orientation-on-JSON
  entry was removed in RFC 057; confirm that it is gone.

## 6. RFC stamps

- **RFC 057:** change *"0.19.0 candidate (on `main`)"* to **`0.19.0`**, in the index and in its Status line.
- **RFC 056's row:** change *"Handoff B (the typed card's layout) 0.19.0 candidate (on `main`)"* to shipped in
  **0.19.0**.
- **RFC 053's row:** change *"Handoff C (going back to an earlier block) 0.19.0 candidate (on `main`)"* to shipped in
  **0.19.0**.
- **RFC 011's row:** add this prep handoff.
- Run `rfc_index`.

## 7. Sweeps

- **Version strings:** `0\.18\.x`, `v0\.18` and `0\.18\.0`, in live statements only.
- **Other strings:** `validated through` (still `0.55.0`), and *"prikk reports"* in any user-facing text that RFC 057
  made stikk's.
- **The standing checks:** the grep for invisible characters, for decoded escapes, **and for repeated sections**.
  This release's text quotes escapes such as `‮`, so write them doubled where the file should show the escape.
- **`literal_guard`** must be green.

## 8. Submit and runs (no branch)

**Before submitting:** run the eight gates, the **local real-binary suite at 0.28.0 + 0.55.0**, and `release.yml`'s
`awk` on the final `CHANGELOG.md`.

**Package it** to `.git-exclude/review-request/011-release-0-19-0-preparation/review-request-v1.md`, leading with the
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

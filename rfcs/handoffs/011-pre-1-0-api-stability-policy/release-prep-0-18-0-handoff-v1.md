# Handoff — 0.18.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). This is the seventeenth release prepared
under it.
**Issued** 2026-10-30. **The project owner authorized the cut** the same day, and the dev team prepares it by standing
permission. The tag and the publish wait for the owner after review, on the 0.18.0 release proposal
(`.git-exclude/release/0-18-0-proposal.md`). **Read the proposal first.**
**Covers:** [RFC 056](../../done/056-trust-adopt-and-remove.md) and RFC 053's
[Handoff B](../053-get-files-back/b-carried-follow-ups-handoff-v1.md).
**Follows** [0.17.0's prep handoff](./release-prep-0-17-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.**

---

## 1. The bump

`0.17.0` → `0.18.0` in `[workspace.package]` and the five pins. Run `cargo update --workspace`; **only `stikk*` lines
may move.**

## 2. `## Unreleased` → `## 0.18.0 — <the date you push>`

- **A bold summary first.** Open with the proposal's sentence, then say what a 0.17.0 user notices:
  - `a` and `x` in the Trust view, and the typed key id;
  - the remove card's warning about Verify, and how adopting the key again undoes it;
  - the security-event card;
  - the dangling-symlink refusal and the hint row.

  Name RFC 056 once, and **count the suite's cases**.
- **Headings in this order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`. Leave out any that
  would be empty.
- **Read it whole as a user who has 0.17.0**: one voice, no review-round vocabulary.

## 3. The Breaking table over `0.17.0..HEAD`

Run `cargo public-api` per crate, then the five-category read.

- **Two rows are expected**, as the proposal lists them.
- **Check the Additive list against what `public-api` adds.** RFC 056 added many items, and 0.17.0's prep found two
  miscounts in a list like this one.
- List the candidates you rejected.

## 4. User-facing text 0.18.0 makes false or incomplete

- **`## Status` in `crates/stikk/README.md` and `docs/src/index.md`, and `releasing.md`'s *"What a v0.17.x release
  is"*:** each becomes 0.18.x, with adopting and removing keys.
  - **Each now says, falsely, that adopting and removing keys is not built.** These are the lines to fix:
    - `crates/stikk/README.md:113` and `:124`;
    - `docs/src/index.md:98`;
    - `releasing.md:120`.
  - **What is still missing** is tag adoption and author keys. Name those instead.
- **Correct `releasing.md`'s counts.**
  - **Thirteen mutations**: adopt a key and remove a key join the eleven (category `Trust`, confirmed by typing the key
    id).
  - **Thirty-two seam methods**: add `trust_add` and `trust_remove` to its list.
  - The same *eleven* stands in `crates/stikk/README.md:94`.
- **The feature paragraph in each README**, and its counterparts: extend the Trust view's sentence. It now adopts and
  removes keys, and warns before a removal would make your own history fail Verify.
- **The AUR.** The crate README's *"Its AUR packages are on the way"* stands, and so does the install page's *"not
  published yet"* paragraph.
- **Sweep each README's manual-install section** for any version in a file name or URL. None is expected; confirm it.

## 5. `ROADMAP.md`

- **Add `## Shipped — <0.18.0's summary line> (0.18.0, breaking)`** with increments 1–2. Say in one line that
  increment 3 (a re-baseline) was not needed: prikk is still at 0.55.0.
- **Change `## Next — 0.18.0 …` to `## Next — 0.19.0, not yet scoped`.** Carry SignPath, the Homebrew tap and the AUR
  dispatch under it, unchanged.
  - **Name the confirmation overlay's follow-up only as "a candidate":** its three carried items are listed in RFC
    056's *Delivered* section and in RFC 047's.
  - **The scope is the owner's to authorize.**
- **Update *Later*'s trust entry.** The write half shipped in 0.18.0. What remains is tag adoption, author keys and
  `FR-104`.

## 6. RFC stamps

- **RFC 056:** change *"0.18.0 candidate (on `main`)"* to **`0.18.0`**, in the index and in its Status line.
- **RFC 053's row:** change *"Handoff B (carried follow-ups) 0.18.0 candidate (on `main`)"* to shipped in **0.18.0**.
- **RFC 011's row:** add this prep handoff.
- Run `rfc_index`.

## 7. Sweeps

- **Version strings:** `0\.17\.x`, `v0\.17` and `0\.17\.0`, in live statements only.
- **Other strings:** `eleven mutations`, `thirty methods`, *"adopting and removing … not built"* and `validated
  through` (still `0.55.0`).
- **The standing checks:** the grep for invisible characters, for decoded escapes, **and for repeated sections**.
- **`literal_guard`** must be green.

## 8. Submit and runs (no branch)

**Before submitting:** run the eight gates, the local suite at 0.28.0 + 0.55.0, and `release.yml`'s `awk` on the final
`CHANGELOG.md`.

**Package it** to `.git-exclude/review-request/011-release-0-18-0-preparation/review-request-v1.md`, leading with the
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

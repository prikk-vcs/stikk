# Handoff — 0.21.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). This is the twentieth release prepared
under it.
**Issued** 2026-11-23. **The project owner authorized the cut** the same day, and the dev team prepares it by standing
permission. The tag and the publish wait for the owner after review, on the 0.21.0 release proposal
(`.git-exclude/release/0-21-0-proposal.md`). **Read the proposal first.**
**Covers:**
- [RFC 063](../../done/063-prikk-0-58-rebaseline.md), the prikk 0.58 re-baseline;
- [RFC 064](../../done/064-json-reader-skips-what-it-does-not-read.md), the JSON reader;
- [RFC 059](../../done/059-readable-colours.md), readable colours;
- [RFC 060](../../done/060-bigger-text.md), bigger text;
- [RFC 061](../../done/061-finding-your-place.md), finding your place.

The mouse (RFC 062) was withdrawn to the GUI and is not written.

**Follows** [0.20.0's prep handoff](./release-prep-0-20-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything else: stop and report.
> **No branch.** **The owner's philosophy applies to every sentence you write here:** *"APIs and UI/UX for users not
> to be confused or misunderstand"*. A user of 0.20.0 must be able to read the changelog and know what moved.

---

## 1. The bump

`0.20.0` → `0.21.0` in `[workspace.package]` and the five pins. Run `cargo update --workspace`; **only `stikk*` lines
may move.**

## 2. `## Unreleased` → `## 0.21.0 — <the date you push>`

- **The entries are already there**, under `### Changed` and `### Added`, from the five RFCs.
- **Add `### Breaking`**, from §3.
- **Start with a bold summary.** Open with the proposal's sentence. Then say what a 0.20.0 user notices first:
  - **the keys changed.** Give **a small table of old key → new key**, from RFC 061 decision 2: `Q`→`p`, `V`→`v`,
    `T`→`t`, `i`→`o`, `C`→`c`, `S`→`s`, `v`→`f`. Then one line: *"switch, merge, branch, tag, close, import, export,
    adopt, remove, undo and save are now in Enter's list and the palette"*;
  - **Enter** lists what can be done with the selected row, and `→` opens it;
  - colours readable on light terminals, and the themes;
  - text can be made about twice as big;
  - prikk 0.58.

  Name each RFC once, and **count the suite's cases** (100).
- **Headings in this order:** `### Breaking`, `### Added`, `### Changed`, `### Removed`, `### Fixed`. Leave out any that
  would be empty.
  - Under `### Removed`: the mouse never shipped, so it has no line.
  - Under `### Fixed`: the help not scrolling with ↓, and Block detail not scrolling. Both were older than this
    release, so they are fixes a 0.20.0 user benefits from.
- **Read it whole as a user who has 0.20.0.**

## 3. The Breaking table over `0.20.0..HEAD`

**I ran `cargo public-api -p <crate> diff 0.20.0..7975c08`** (the proposal, *Version*). The rows:
- **`stikk-core`:**
  - `palette` and `Command` are removed, replaced by `actions`;
  - `NO_FOCUSED_REF_REASON` becomes `no_focused_ref_reason()`;
  - `Evidence::ExplicitYes` and `TypedName` carry `Seen`;
  - `AdoptPreview::TwoKeys` carries `SecurityWarning`;
  - `trust::sealers` takes a fourth argument.
- **`stikk-tui`:**
  - `Palette::dark`, `light` and `from_theme` are removed;
  - `Overlay::Glossary` loses `offset`;
  - `Overlay::TrustConflict` carries `SecurityWarning`;
  - `Overlay` gains `ActionList`;
  - `Focus::BlockDetail` and `Focus::Changes` gain a field.
- **`stikk-prikk`:**
  - `TrustWritten` gains two variants;
  - `Queue` and `BundlePreviewRead` gain public fields. None of the three is `#[non_exhaustive]`.
- **`stikk-state`:** `Theme` gains `HighContrast` and `Plain`.
- **No change:** `stikk-model` and the launcher.

**Re-run it yourself on the prep commit** and confirm. List anything you add or reject, and why.

## 4. User-facing text 0.21.0 makes false or incomplete

- **The status texts become 0.21.x:** `## Status` in `crates/stikk/README.md` and `docs/src/index.md`, and
  `releasing.md`'s *"What a v0.20.x release is"*. Each needs **one clause** for what is new: *readable on light and
  dark terminals, bigger text, and a list of what you can do where you are*.
- **Recount, don't trust:** the mutations list and the seam methods count in `crates/stikk/README.md` and
  `releasing.md`. RFC 061 moved keys, not operations, so the counts may not change. Say what you found.
- **Keys named anywhere in prose outside the guide's key reference:** `key_literals` and `guide_keys` hold the shipped
  strings and the guide. **Also read the two READMEs and `releasing.md`** for any withdrawn key (`Q`, `V`, `T`, `C`,
  `S`, `U`, `F`, `A`, `E`, `I`, `i`, `M`, `n`, `x`, `a`, `R`, `o` as background operations, and `v` as raw), and
  rewrite each in the README's own form.
- **The install guide's *"What you should see"* transcript:** sweep its version (the carried rule).
- **Sweep each README's manual-install section** for any version in a file name or URL.

## 5. `ROADMAP.md`

- **Add `## Shipped — <0.21.0's summary line> (0.21.0, breaking)`**, with its increments, and the mouse's withdrawal in
  one line.
- **Change `## Next — 0.21.0: easy to read, and easy to find your way in` to `## Next — 0.22.0: choosing the GUI
  toolkit`.** Use the owner-authorized theme's words from the 0.21.0 section's last paragraph, and keep the candidates
  listed there.
  - Add one line: *"the scope is proposed once 0.21.0 is cut"*.
  - **Name nothing else.**
- **Update the *Later* entries this release touched:** the GUI line now says it is next.

## 6. RFC stamps

- **RFCs 059, 060, 061, 063 and 064:** each Status line's *"ships in **0.21.0**"* becomes *"shipped in **0.21.0**"*.
  The index rows already say *"shipped in 0.21.0"*.
- **RFC 011's row:** add this prep handoff.
- Run `rfc_index`.

## 7. Sweeps

- **Version strings:** `0\.20\.x`, `v0\.20` and `0\.20\.0`, in live statements only. The roadmap's shipped sections
  keep theirs.
- **Other strings:** `validated through` (now `0.58.0`).
- **The standing checks:** the grep for invisible characters, for decoded escapes, and for repeated sections.
- **`literal_guard`, `key_literals`, `guide_keys` and `emphasis_guard`** must be green.

## 8. Submit and runs (no branch)

**Before submitting:** run the eight gates, the **local real-binary suite at 0.28.0 + 0.58.0**, and `release.yml`'s
`awk` on the final `CHANGELOG.md`.

**Package it** to `.git-exclude/review-request/011-release-0-21-0-preparation/review-request-v1.md`, leading with the
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

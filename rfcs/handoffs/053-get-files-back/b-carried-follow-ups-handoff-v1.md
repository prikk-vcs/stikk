# Handoff B — RFC 053 and earlier reviews: the carried follow-ups (v1)

**Companion to:** [RFC 053](../../done/053-get-files-back.md) (its *Delivered* section lists three of these) and RFC
038's review v2 (the fourth). **0.18.0's increment 2**, authorized by the project owner on 2026-10-29
(`.git-exclude/release/0-18-0-scope.md`).
**Independent of RFC 056**, so it may land first.
**Not a release.**

> **What this increment is.** Four small fixes that earlier reviews carried *"until the file is next touched"*. They
> change one behaviour (§1), one row of one view (§2), and nothing else a user sees.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite and supply chain there.

---

## 1. "Already there" means what prikk means (`stikk-core`)

`files.rs:324` (`destination.exists()`) and `bundle.rs:726` (`output.exists()`) follow symlinks. A **dangling
symlink** at the destination therefore passes stikk's rule, and prikk then refuses it, because prikk's
`destination_exists` is `symlink_metadata(..).is_ok()`. Nothing is written either way, but stikk's own sentence should
answer first.

- **Change:** use `std::fs::symlink_metadata(path).is_ok()` in both places. Put it in one private helper if both
  modules can reach one; otherwise write it in each, with one comment naming prikk's rule.
- **Tests:** a dangling symlink at the destination is refused by stikk's rule, with **nothing spawned**, for a saved
  file, a saved folder and an export. Gate them with `#[cfg(unix)]`, because creating a symlink on Windows needs a
  privilege the runner may not have. Say that in the test's doc.

## 2. Block detail's key hint gets its own row (`stikk-tui`)

`view/block.rs:196` draws *"state … — N file(s), M byte(s) · F saves the selected file, A saves all:"* on one row, and
at 80 columns it wraps as *"… A saves / all:"*.

- **Change:** draw the state line as it was before RFC 053 (*"state … — N file(s), M byte(s):"*), and *"F saves the
  selected file, A saves all"* on **its own row** beneath it, in the dim style the view uses for hints.
- **Test:** at 80 columns, the hint is one row and is in the buffer whole. Re-take the Block detail capture.

## 3. `version.rs:113` is 151 characters

Rewrap that doc paragraph, and `:65` (132) while you are there, to the file's width. Change no words.

## 4. `RAW_MULTILINE` gets its `"to"` object

`stikk-tui/src/view/compare/tests.rs`, the fixture described as *"the shape `Json::render` really writes"*: a
`modified` entry always carries `"to"`, and this one omits it. Add the `"to"` object in the shape of
`stikk-prikk`'s `an_entrys_raw_fields_are_multi_line_indented_json` capture. No assertion should need to change. If
one does, stop and report.

## 5. The traps

1. **§1 is the only behaviour change.** Anything more is out of scope.
2. **No `CHANGELOG.md` entry for §3 or §4.** §1 gets a `### Fixed` line in `## Unreleased`, in a user's words: a
   broken shortcut at the destination is now refused by stikk, in its own words. §2 gets a `### Changed` line.
3. **Paste captures as the file holds them, and grep before committing.**

## 6. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/053-carried-follow-ups-b/review-request-v1.md`, with one section per item and the 80-column
capture from §2.

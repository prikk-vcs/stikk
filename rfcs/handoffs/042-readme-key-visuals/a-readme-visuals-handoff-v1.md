# Handoff A — RFC 042: README key visuals, rendered from invented data (v1)

**Companion to:** [RFC 042](../../accepted/042-readme-key-visuals.md), **accepted 2026-10-15 with the three views as
proposed**. **Read the RFC first.**
**Covers:** decisions 1–6.
**Not a release.** GitHub shows the README on push; crates.io shows it at the next publish.

> **The owner's condition is the design constraint:** *"my privacy must be respected and the local secrets must not be
> exposed."* **Every image is rendered from invented data by stikk's own views, never photographed.** If at any point a
> scene seems to need real data, a real repository, a real terminal, or reading anything from the machine, **stop and
> report**. That is the one thing this increment must not do.
>
> **No branch.** Local gates before submitting; after approval, push to `main` and dispatch the suite and supply chain
> there.

---

## 1. The generator: `crates/stikk-tui/examples/readme_visuals.rs`

- **Builds three scenes from `NullBackend`**, written for the purpose (RFC 042 decision 3): History with Block detail,
  Compare with one whitespace-only change among its entries, and Verify with a verdict and its stages. **Drive them
  through the real `App` and the real views**, the way the view tests do, at **80 × 24**.
- **Writes SVG**, one file per scene, to `docs/assets/readme/` (or to a directory given on the command line, which the
  test in §3 uses). SVG is text and holds no metadata block.
- **Reads nothing from the machine**: no `HOME`, `USER`, hostname, current directory or environment variable, and no
  prikk binary. **Any path in an image comes from the invented data.**
- **Deterministic**: the same code gives byte-identical files on every run and platform. No timestamps, no random ids,
  and fonts named rather than embedded.

## 2. The cell-to-SVG writer

Small and private (inside the example, or a `pub(crate)` module if the test needs it; **not public API**):
- one `<text>` run per row, splitting where the style changes, with each cell's foreground, background and bold or
  italic from the buffer;
- a monospace font stack (`ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`), a fixed cell width and height,
  and the theme's default background;
- **every character XML-escaped** (`&`, `<`, `>`, `"`), and box-drawing characters kept as they are. The views'
  escapes (`\u202e` and the rest) are already ASCII text by the time they reach a cell (RFC 036), so nothing invisible
  can reach the SVG either. **Assert that in the test.**

## 3. The guard test (RFC 042 decision 4)

In `stikk-tui`'s tests, or the suite crate if it needs a dependency the published crates should not carry:
- **regenerate into a temporary directory and compare byte for byte** with `docs/assets/readme/`. **On a difference,
  fail and print the one command that regenerates them**;
- **fail if any image contains**:
  - `/home/`, `/Users/` or `C:\Users\`;
  - the test machine's own `HOME`, `USER`/`USERNAME` and hostname, when set and non-trivial (at least 3 characters);
  - an `@` followed by a domain;
  - a 64-hex string that is not in the scenes' **declared list of invented ids**;
  - the words `seed` or `secret`, or any character `needs_unicode_escape` flags.

  **This test reads environment variables only to know what must *not* appear in the images. It never reads key
  material**, and it never prints those values. On a match it says which rule matched and in which file, not the
  value.

## 4. The README, and crates.io

- `README.md`: the three images **after the opening paragraph and before `## Install`**, small, each with alt text
  that says what it shows (*"stikk's History view, with one block's files beside it"*). **Relative paths.**
- `crates/stikk/README.md`: **measure first** whether crates.io renders an image from an **absolute** GitHub raw URL in
  a crate README, using the published 0.11.0 page or crates.io's documented behaviour, cited. **If it does**, use
  absolute URLs to the files on `main`. **If you cannot establish it, leave the images out of the crate README** and
  say so. A broken image on crates.io is worse than none.
- `CONTRIBUTING.md`: the one command that regenerates the images, and that the test fails until they are regenerated.

## 5. The changelog

Under `## Unreleased`, `### Added`: README images of History, Compare and Verify, rendered by stikk's own views from
invented data.

## 6. Submit (no branch)

The eight gates and the grep before committing. **Also run the privacy guard against the images yourself**, and show its
pass in the request. Package to `.git-exclude/review-request/042-readme-visuals/review-request-v1.md` with:
- **the three SVGs**, referenced by path. **I will open them myself**, so do not paste their XML;
- the list of invented ids and names;
- the guard's rules, each with a deliberately planted failure showing it fires (a planted `/home/x`, a planted
  undeclared 64-hex id, and so on), in a scratch copy, never in the committed images.

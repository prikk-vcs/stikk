# RFC 042 — README key visuals, made from invented data so nothing private can appear

**Status.** **Done 2026-10-15**: Handoff A delivered on `main` (`8a84c8b`, `991aebd`); on GitHub's README now, on crates.io at the next publish. **Accepted by the project owner 2026-10-15**, with the three views as proposed. **Proposed 2026-10-15** by the architect, on the project owner's suggestion the same day: *"It may be better
to new user candidates to put a few screenshot(s) (key visual only) on `README.md`. However, when the screenshots taken,
my privacy must be respected and the local secrets must not be exposed."* **No open question**; decision 2's choice of
views is the owner's to change on acceptance.
**Tracks.** `C-T2a` (what reaches a display), RFC 036 (rendering), RFC 039 (the README a new user reads first), the
project rule that stikk never reads or stores key material.
**Touches.** `stikk-tui` (one example, plus a small cell-to-SVG writer that is not public API), a test, `README.md`, and
one new folder of images.

## Summary

A new user decides from the README's first screen. A few images of stikk at work would help them more than any
sentence. **The owner's condition decides how they are made**: nothing of the owner's machine (no path, username,
hostname, repository content, key id or key material) may appear.

**So the images are not taken from a real terminal on a real machine.** They are **rendered by stikk's own TUI code,
from invented data**, through the same stand-in backend stikk's demos and tests already use, into an in-memory screen,
and written out as SVG. **Privacy holds by construction, not by cropping or reviewing a photo of a desktop.** The same
command regenerates them byte for byte, so a test notices if they drift from the real UI.

## Findings

### F1 — the pieces exist

- **`NullBackend`** (`stikk-prikk`) is a scripted stand-in for prikk, which the four demo examples (`orientation_demo`,
  `history_demo`, `changes_demo`, `explanation_demo`) already use: *"no prikk"*.
- **ratatui's `TestBackend`** renders any view into a buffer of cells, each with its symbol and colours. The view tests
  already draw every view this way at 80 columns.
- **What is missing** is only a writer from a buffer to an image, and a choice of scenes.

### F2 — what a real screenshot would leak

From the views stikk draws today: **the repository path** (Orientation, the status bar, the one-shot printout), **the
directory name**, **branch and tag names**, **file paths and content** (Changes, Patch detail, Compare), **commit
messages**, **key ids** (the signing line, rename assertions), and **whatever the terminal itself shows** (title bar,
prompt, hostname, window chrome, other windows). A real capture would need every one of these scrubbed by hand, every
time. **None of them exists in an invented scene.**

## Decisions

1. **Images are rendered, never photographed.** A committed generator, a `stikk-tui` example named `readme_visuals`,
   builds each scene from `NullBackend` data written for the purpose, draws it through the real view code into a
   `TestBackend`, and writes **SVG**. SVG is text, so it is reviewable and greppable, it stays sharp on every display,
   and it has no metadata block for a camera, OS or path to hide in. **No real prikk, no real repository, no real
   `HOME`, and no environment variable is read while generating.**

2. **Three images, chosen to show what stikk is for**, and the owner may swap any of them on acceptance:
   - **History with Block detail**: the lineage and one block's files, which is the core browse;
   - **Compare**: prikk's hunks, quoted byte for byte, including one whitespace-only change, stikk's most distinctive
     rendering;
   - **Verify**: a verdict and its stages, the newest headline.
   **80 columns**, the size every capture in this project has used, and the theme's default palette.

3. **The invented data is visibly invented, and owned by this project.** Repository `example-project`, refs
   `heads/main` and `heads/feature`, a small Rust-looking source tree (`src/main.rs`, `README.md`), messages like *"add
   the parser"*, key id `demo`, and block ids that are fixed fake hex. **Nothing copied from any real repository**, the
   owner's included.

4. **A privacy guard runs in CI.** A test regenerates the images and fails if:
   - **they differ from the committed files**, so the images can never drift from the real UI unnoticed; or
   - **any image contains** a `/home/`, `/Users/` or `C:\Users\` path, the generating machine's `$HOME`, `$USER` or
     hostname, an `@` that looks like an email address, a 64-hex string that is not one of the invented ids, or the
     words `seed` or `secret`.
   **The guard reads the SVGs, never the machine's keys.** It checks that the generator's environment does not leak; it
   does not go looking for key material.

5. **Where they go.** The images live in `docs/assets/readme/`. In `README.md` they sit **after the one-paragraph
   description and before `## Install`**, small, each with alt text describing what it shows. **The crates.io page**
   (`crates/stikk/README.md`) uses **absolute URLs** to the files on GitHub, since crates.io does not serve relative
   images, or omits them if that adds friction; the handoff measures which.

6. **Regenerating is one command**, documented in `CONTRIBUTING.md`, and **every release-prep handoff checks the images
   are current**. The test already does; the prep just reads its result.

## Changelog

`### Added`, under the docs: README images of History, Compare and Verify, rendered from invented data.

## Delivery

**One handoff, issued on acceptance:** [Handoff A](../handoffs/042-readme-key-visuals/a-readme-visuals-handoff-v1.md). It is not a release, and it can land at any time. GitHub shows the README on push, and
crates.io at the next publish.

## What this RFC does not do

- **No photograph of a real terminal**, ever, for these images.
- **No animated GIF or video.** That can come later, from the same generator, if wanted.
- **No change to any view.** If a scene looks poor, that is a design note for later, not a change made here for a
  picture.

## Delivered

**One handoff, two commits, green on `main` at `991aebd`** (CI, Docs, the full-matrix suite and supply chain).

- **Three SVGs in `docs/assets/readme/`**, History, Compare (with a whitespace-only change) and Verify. They are
  rendered by the real `App` and `shell::render` from `NullBackend` data in an invented `example-project`, with ids
  that are visibly placeholders. **Nothing is read from the machine.** The architect rendered them to PNG, looked at
  them, and scanned them independently; nothing private appears.
- **The generator is a `#[cfg(test)]` module, not an example** (ruled in review), so stikk's worker protocol stays
  private. `cargo public-api` showed no change. Regenerate with
  `cargo test -p stikk-tui regenerate_readme_images -- --ignored`.
- **The guard:** images current byte for byte; the committed files clean; every rule firing on a planted fault, in a
  second independent implementation too.
- **Looked at, not just read:** text pinned to the cell grid, and box-drawing drawn as rectangles.
- **crates.io:** absolute GitHub raw URLs (measured: crates.io renders them; `width` is stripped). The page tracks `main`.
- **Found through the picture:** five view titles were cut at 80 columns, not just Verify's. They were shortened in
  stikk's own words, never prikk's or a name, and a test now holds every scrolling view's title whole.

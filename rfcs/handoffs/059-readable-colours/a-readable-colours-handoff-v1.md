# Handoff A — RFC 059: readable colours (v1)

**Companion to:** [RFC 059](../../done/059-readable-colours.md), **accepted 2026-11-21**. **Read the RFC first**:
F1–F4 and decisions 1–12. **Covers:** all of it. **0.21.0's increment 2**, the first readability increment.
**Not a release.**

> **What this increment is.** stikk's default theme is unreadable on a light terminal: 1.4 : 1 for body text on
> white (F1). After this handoff:
> - body text is the terminal's own colour;
> - *system* asks the terminal whether its background is light or dark, and adds no colours of its own when it gets
>   no answer;
> - every colour stikk adds reaches 7 : 1, on the common backgrounds and on the one the terminal reports;
> - `high-contrast`, `plain` and `NO_COLOR` exist;
> - a test computes contrast from every rendered screen.
>
> **The Windows probe runs are still out** (RFC 059, "What the Windows runs confirm"). They change at most a guide
> sentence or decision 4's platform rule, and the architect amends the RFC if they do. **Start now.** If an amendment
> lands mid-work, it comes as a short note on this handoff.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the full-matrix suite and supply chain there.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does every chosen colour reach 7 : 1 on its references, at both depths? | unit | the test's name, and its printed table, which matches `palette059.txt` |
| Do depth, detection, fallback, `COLORFGBG`, `NO_COLOR` and the guard choose right? | unit | test names |
| Does every rendered screen keep 7 : 1? | headless (`TestBackend`) | the matrix test's name, the views it covers, and its count of cells checked |
| Do the new rows and themes read and fit? | headless | text captures of Orientation's `colours` row, and of one view in `high-contrast` and in `plain`, at 80 × 24 |
| Real terminals | **after your approval**: the architect on Linux, the owner on Windows | not yours to capture. Say in the request that the build is ready for them |
| Nothing else regressed | real binary, 0.28.0 + 0.58.0; then the full matrix in CI | counts |

**No screenshot from you.** The real-terminal checks are the architect's and the owner's, on the approved build.

## 1. Commit 1: the palette and its own test (decisions 2 and 3)

1. **`theme.rs`.** `Palette` keeps its five roles, `fg`, `dim`, `accent`, `ok` and `warn`, and gains **`bg:
   Option<Color>`**. That is `None` everywhere except `high-contrast` (commit 3).
   - In `dark` and `light`, `fg` is `Color::Reset`.
   - Each of `dim`, `accent`, `ok` and `warn` takes RFC 059 decision 3's table, **as one `RoleColour { rgb, indexed }`
     per role**, so both depths travel together.
   - `Palette::for_depth(depth)` returns the `Color`s to draw with: `Color::Rgb` at 24-bit, `Color::Indexed` at 256.
2. **No named ANSI colour** is left in `theme.rs`.
3. **The test, `every_chosen_colour_reaches_seven_to_one_on_its_references`:**
   - it uses the reference backgrounds of decision 3, as `const`s in `theme.rs`;
   - it uses the WCAG relative-luminance formula, written once, in a `pub(crate)` function the matrix test reuses;
   - 256-colour values are converted to RGB with the standard xterm table (the cube levels 0, 95, 135, 175, 215 and
     255, and the ramp `8 + 10 i`);
   - it **asserts ≥ 7.0** for every role × reference × depth, and prints the table.

   **Cross-check it once against `palette059.txt`**, and say in the request that the numbers agree to two decimals.

**Every existing test that asserts a palette colour** moves to the new values in this commit, with the RFC as the
reason.

## 2. Commit 2: depth, detection, fallback and the guard (decisions 4, 5, 6 and 9), with their tests

1. **The dependency:** `terminal-colorsaurus = "1.0.3"` in the workspace `Cargo.toml`, with a comment citing RFC 059
   F4, used by `stikk-tui` only. Run `cargo deny check` in this commit. If `deny.toml`'s exact license list needs a
   line, add it, and say so.
2. **A pure resolver, `theme::resolve(input) -> Resolved`**, which reads **nothing itself**. Its input is:
   - the configured theme, and whether the config set it explicitly. Today's `Config` cannot tell an explicit
     `system` from the default, and **it does not need to**: `NO_COLOR` applies when the theme is `system`, either
     way;
   - `NO_COLOR`, `COLORTERM`, `WT_SESSION` and `COLORFGBG`;
   - the platform;
   - **the detection's outcome**: an answered background RGB, or no answer.

   `Resolved` is the palette to draw with, the depth, and **the reason**, which commit 4 shows.
3. **The impure part is one function**, `theme::detect()`, called by `lib.rs`'s `run` **before**
   `TerminalGuard::enter()`. It sends **no query** unless the resolver would use one, which means `system` or
   `high-contrast`, with no `NO_COLOR` taking over first. It wraps `terminal_colorsaurus::color_palette` with the
   default `QueryOptions`.
   - `run` passes `Resolved` to `App::open`, which no longer calls `Palette::from_theme` (`app.rs:841`).
   - `readme_visuals.rs` passes an explicit dark `Resolved` (decision 11).
4. **The guard (decision 6).** With an answered background, any role below 7 : 1 **at the chosen depth** becomes
   `Color::Reset`, and `Resolved` counts how many did.
5. **`COLORFGBG`:** the last `;`-separated field. `7` or `15` is light; `0`–`6` or `8` is dark; anything else, or a
   non-number, is no answer.

**Tests, in this commit** (all against `resolve`, with no terminal):
- depth: `COLORTERM=truecolor`, `COLORTERM=24bit`, `WT_SESSION` set, Windows, and none of these;
- detection: light answered, dark answered, no answer with each `COLORFGBG` form, and no answer at all, which is
  `plain`;
- `NO_COLOR`: empty, which is ignored; set with `system`, which is `plain` with **no query wanted**; set with each
  explicit theme, where the config wins;
- the guard: a mid-grey `#808080` background and a tinted one (`#3b2f4a`), each with its count of fallen roles;
- **`detect()` is not called** for `dark`, `light` or `plain`, nor under `NO_COLOR` with `system`. Assert it
  through `resolve` saying whether it wants an answer, not by spawning anything.

## 3. Commit 3: high-contrast and plain (decisions 7 and 8), with their tests

1. **`plain`:** every role is `Color::Reset`, and `bg` is `None`.
2. **`high-contrast`:**
   - `bg` is `Some(black)` with `fg` white, or, when the terminal answered light, `Some(white)` with `fg` black;
   - every role is that one `fg`;
   - `shell.rs` paints `bg` over the **whole frame** first, under everything, overlays included.
3. **`stikk-state`'s config** accepts `high-contrast` and `plain` (`config.rs:92`). Unknown words keep today's
   warning. Update the config's documentation where the themes are listed.

**Tests:** the parser accepts both words; `high-contrast` paints every cell's background in both directions; `plain`
paints none and colours nothing.

## 4. Commit 4: the Orientation row (decision 10), with its test

**A `colours` row** in `view/orientation.rs`, in the existing `field(…)` form. It gives the reason, the depth, and the
guard's count when that count is not zero, in RFC 059 decision 10's words.

**Tests:** one per reason. Plus a capture at 80 × 24 in which the row's words are whole, asserted after flattening,
as the earlier captures do.

## 5. Commit 5: the headless matrix

**`every_screen_keeps_seven_to_one`**: **every view the shell can show** × `{dark, light, high-contrast, plain}` ×
`{80 × 24, 120 × 40}`, drawn with `TestBackend`, at **both depths** for dark and light.
- For each cell whose `fg` is not `Reset`, compute its contrast against:
  - its own `bg`, when one is painted (high-contrast);
  - otherwise, **every** reference background of the theme.
- **Assert ≥ 7.0**, and name the view, the cell and the theme on failure.
- **"Every view"** means every view the shell can show, including the overlays that have their own colours: the
  refusal card, the confirmation cards, the warning card and the help overlay. Use the existing capture helpers.
  List what you covered in the request, and say why anything is not covered.
- Print the number of cells checked.

## 6. Commit 6: the text, and the visuals (decisions 11 and 12)

- **`NFR-A03` and `GU-07`** in the reference docs, amended in RFC 059 decision 12's words.
- **The guide:** a short *"Colours"* section in *Getting started*:
  - the five themes, one line each;
  - `NO_COLOR`;
  - what the Orientation row says;
  - that `high-contrast` paints its own black and white.
- **README key visuals:** regenerate them with the generator RFC 042 built, and commit the new images.
- **`CHANGELOG.md`, `## Unreleased`:** the RFC's lines.

## 7. The traps

1. **Query before the alternate screen and raw mode**, never inside the UI loop. Never query a second time.
2. **The resolver is pure.** Every environment read happens in `run`, passed in as input, so the unit tests need no
   terminal and no environment variables.
3. **`Reset` is the terminal's own pair.** The matrix skips it. Never compare `Reset` against a reference.
4. **Choose the 256 values. Never round to them.** Decision 3's table is the 256 palette; nearest-colour rounding is
   what F3 warns against.
5. **No new `Color::` literal outside `theme.rs`.** Every colour comes from the palette, as it does today.
6. **Paste captures as the file holds them, and grep before committing.**

## 8. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Package to
`.git-exclude/review-request/059-readable-colours-a/review-request-v1.md`, and lead with:
- commit 1's table, against `palette059.txt`;
- the matrix's coverage and cell count;
- the captures;
- the new dependency's `cargo deny` result, and its crates per target.

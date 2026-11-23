# RFC 059 — Readable colours

**Status.** **Done 2026-11-23**: Handoff A delivered on `main` (`e298aa1`..`3e284a8`), green on the full platform matrix; checked on Linux by the architect and on Windows by the owner; ships in **0.21.0**. **Accepted by the project owner 2026-11-21**; [Handoff A](../handoffs/059-readable-colours/a-readable-colours-handoff-v1.md). **Proposed 2026-11-21** by the architect, as 0.21.0's increment 2 (`.git-exclude/release/0-21-0-scope.md`,
authorized 2026-11-04, amended 2026-11-18). **Written before the Windows probe runs, at the owner's direction
(2026-11-21): "Windows results confirm later."** Every decision below holds whichever way Windows answers (§
"What the Windows runs confirm"). **No open question.**
**Measured.**
- `.git-exclude/reports/059-scope-0-21/contrast.py` → `contrast-0.20.0.txt`: stikk 0.20.0's colours on black, dark
  and white.
- `.git-exclude/reports/059-readable-colours/probe/` (`terminal-colorsaurus` 1.0.3) →
  `stikk-probe-059-alacritty-0.17.0-wayland.txt`: Alacritty answers the background query in 7.7 ms, with `#2e3440`.
- `.git-exclude/reports/059-readable-colours/palette059.py` → `palette059.txt`: the values decision 3 proposes,
  checked against every reference background.
- Dependency cost: `cargo tree` of stikk and of the probe, per target, compared by crate name (F4).

**Not measured, documented:**
- **Windows Terminal and conhost:** the owner's probe runs confirm (§ "What the Windows runs confirm").
- **macOS Terminal:** `terminal-colorsaurus`'s terminal survey lists it as answering. No Mac is available (the
  scope's decision point).

**Tracks.** `NFR-A03` (amended by this RFC), `TU-10`, `GU-07`. RFC 060–062 build on it.
**Touches.**
- `stikk-tui`: `theme.rs`, `lib.rs`'s `run`, `app.rs`'s `App::open`, `shell.rs`, and the Orientation view.
- `stikk-state`: the `theme` setting.
- The workspace `Cargo.toml`: one dependency.
- `NFR-A03`, `GU-07`, the guide, the changelog, and the README key visuals.

## Summary

**On a light terminal, stikk's default text is nearly invisible.** The default theme, *system*, always picks the dark
palette, and stikk paints no background. So on white, which is macOS Terminal's default, body text is 1.4 : 1
(F1). Even the light palette only reaches 5.0–6.2 : 1, where WCAG asks 7 : 1 for low vision.

This RFC:
- writes **body text in the terminal's own text colour**, which the terminal already chose for its own background;
- has *system* **ask the terminal** whether its background is light or dark, and **fall back to no colours of
  stikk's own** when it does not answer;
- makes **every colour stikk chooses reach 7 : 1** against every common background of its theme, in 24-bit colour
  and on 256-colour terminals, and **checks it again against the background the terminal reported**;
- adds a **high-contrast theme**, in which stikk paints its own black and white;
- honours **`NO_COLOR`**;
- adds **a test that computes the ratios** from the palette and from rendered screens, so a later change cannot fall
  below them unnoticed.

## Findings

### F1 — stikk 0.20.0's colours, on the backgrounds people use

From `contrast-0.20.0.txt` and `palette059.txt`. Worst case over each theme's reference backgrounds; "on white" is
macOS Terminal's default.

| Role | Dark palette, on dark backgrounds | Dark palette, **on white** | Light palette, on light backgrounds |
|---|---|---|---|
| body text | 9.2 | **1.4** | 15.2 |
| labels (`dim`) | **5.1** | **2.5** | **5.7** |
| accent | named Cyan, the terminal's own | **2.0** | named Blue, the terminal's own |
| ok | named Green | **2.2** | **5.0** |
| warn | named Yellow | **1.7** | **4.7** |

- **The *system* theme never picks light** (`theme.rs`: `ConfigTheme::Dark | ConfigTheme::System => Self::dark()`).
  So the middle column is what every light-terminal user gets today.
- **Named colours are the terminal's own.** stikk cannot know their contrast, and the same name differs by terminal
  scheme.
- **No row reaches 7 : 1 in every column.**

### F2 — terminals can say what their background is

Most terminals answer `OSC 11` (*"what is your background colour?"*) and `DA1` (*"what are you?"*).
`terminal-colorsaurus` sends both. A terminal that does not know `OSC 11` still answers `DA1`, so it is recognised as
not answering at once, with no wait.
- **Measured here:** Alacritty 0.17.0 (Wayland) answered in **7.7 ms**, background `#2e3440`, *dark*.
- **Documented** (the crate's terminal survey, `doc/terminal-survey.md`):
  - **Answer:** Alacritty, foot, GNOME Terminal and the other VTE terminals, Konsole, kitty, WezTerm, iTerm2, **macOS
    Terminal**, xterm, VS Code, **Windows Terminal ≥ 1.22**, and **conhost**, at the OpenConsole commit the survey
    tested;
  - **Do not answer:** the Linux console, GNU Screen, ConEmu, MobaXterm, and a few others.
- **Detection needs both input and output to be a terminal.** The probe's own run without a TTY did not query.
  stikk already refuses to start without a TTY.
- **The crate's default wait is 1 s**, and it is reached only by a terminal that answers neither query. Over SSH, an
  answer can take longer than locally. Its caveat (sharing the terminal with a pager) does not apply to a full-screen
  program.

### F3 — what a terminal without 24-bit colour shows

stikk 0.20.0 already writes 24-bit colours (`dark.fg`, `dark.dim`, the light palette). A terminal without 24-bit
colour approximates them its own way. **macOS Terminal is the common case:** it sets `TERM_PROGRAM=Apple_Terminal` and
no `COLORTERM`.

The 256-colour palette's RGB values are standard: the 6 × 6 × 6 cube and the grey ramp. So **a colour chosen from it
has a known contrast**, as long as stikk chooses it itself rather than letting the terminal round. `COLORTERM` is
`truecolor` or `24bit` where a terminal promises 24-bit colour; Alacritty documents setting it.

### F4 — the dependency's cost

stikk keeps its dependency tree small on purpose (`json.rs:3-12`; RFC 020's supply-chain gate). `terminal-colorsaurus`
1.0.3 adds, by crate name, over stikk's own tree:
- **Linux and macOS:** `terminal-colorsaurus`, `terminal-trx`, `xterm-color`, `memchr`;
- **Windows:** the same four, and `windows-sys`.

Every one is MIT or Apache-2.0. The highest minimum Rust is 1.74, against stikk's 1.88. `cfg-if`, `libc` and `mio`
are already in stikk's tree, at nearby versions.

**The alternative is writing the query by hand.** That means raw byte reads with a timeout on Unix, and console VT
input on Windows, which is `terminal-trx`'s whole job. **Four small crates are the better trade**, and the supply-chain
gate covers them like every other crate.

## Decisions

1. **Five themes**, set by `theme =` in the config:
   - `system` (the default);
   - `dark`;
   - `light`;
   - `high-contrast`, which is new;
   - `plain`, which is new, and is also what *system* falls back to.

   An unknown word keeps the default, as today.
2. **Body text is the terminal's own text colour** in `system`, `dark`, `light` and `plain`. `Palette::fg` becomes
   `Color::Reset`. The terminal chose that pair for itself, and it is what the probe calls *terminal default text on
   its background*: 9.2 : 1 on Alacritty. **stikk paints no background** in these four.
3. **Every colour stikk chooses reaches 7 : 1** on every reference background of its theme. The references
   (`palette059.py`) are:
   - **dark:** `#000000`, `#0c0c0c` (Windows Terminal Campbell), `#1e1e1e` (VS Code), `#282c34` (One Dark), `#2e3440`
     (Alacritty, measured), `#002b36` (Solarized Dark);
   - **light:** `#ffffff` (macOS Terminal, Tango Light), `#fafafa` (One Half Light), `#fdf6e3` (Solarized Light).

   **Named ANSI colours are no longer used.** Each role has a 24-bit value, and a 256-colour value **chosen**, not
   rounded:

   | Role | dark, 24-bit | dark, 256 | light, 24-bit | light, 256 |
   |---|---|---|---|---|
   | `dim` (labels) | `#c1c5ca` | 251 `#c6c6c6` | `#4d535a` | 239 `#4e4e4e` |
   | `accent` | `#97cbf0` | 117 `#87d7ff` | `#135686` | 18 `#000087` |
   | `ok` | `#6ddb92` | 114 `#87d787` | `#185e2f` | 22 `#005f00` |
   | `warn` | `#f7ba64` | 222 `#ffd787` | `#764806` | 88 `#870000` |

   Every 24-bit value is at least 7.20 : 1 and every 256 value at least 7.20 : 1, worst case
   (`palette059.txt`).

   **`dim` is now close to body text in strength.** That is the cost of 7 : 1. Labels keep standing apart by
   position, as they do today, and RFC 060 decides weight.
4. **Which colour depth:**
   - **24-bit** when `COLORTERM` is `truecolor` or `24bit`, or `WT_SESSION` is set (Windows Terminal), or the platform
     is Windows. Microsoft's console blog (2016-09-22) announced 24-bit colour in the Windows console from Windows 10
     Insider build 14931; the owner's conhost run confirms it on a current Windows;
   - **256 colours** otherwise, including macOS Terminal.

   The Windows runs confirm what Windows Terminal and conhost report.
5. **`system` asks the terminal**, once, at startup, **before** stikk enters the alternate screen, through
   `terminal-colorsaurus` with its default 1 s wait. Then:
   - **an answer** gives `light` or `dark` by the crate's own `theme_mode()`;
   - **no answer, but `COLORFGBG` is set:** its last field picks the theme. `7` or `15` is light; `0`–`6` or `8` is
     dark; anything else counts as no answer;
   - **no answer at all** gives `plain`. That is safe on any background.

   **The explicit themes send no query**, so a user can always choose one to avoid it.
6. **The colours are checked again on the background the terminal reported** (`system` with an answer only). Any role
   whose colour, at the depth decision 4 picked, is below 7 : 1 on **that** background shows in the terminal's own
   text colour instead. The palette's references cover the common backgrounds, and this covers the rest, such as a
   mid-grey or a tinted scheme.
7. **`plain`: no colours of stikk's own.** Every role is `Color::Reset`, and no background is painted. Emphasis keeps
   bold, which RFC 060 rules on, and reverse video, which RFC 061 uses for the selected row. Every status already
   carries text or a shape (`NFR-A03`), so nothing is lost.
8. **`high-contrast`: stikk paints its own pair.** Over the whole frame, stikk paints **black text on white** when the
   terminal answered light, and **white on black** otherwise, so it is 21 : 1 whatever the terminal's scheme.
   - It queries like `system`, and does not need an answer.
   - Every role is that one text colour. Emphasis is bold and reverse video only, with no hues.
9. **`NO_COLOR`.** When it is *"present and not an empty string"* (no-color.org), `system` behaves as `plain`, and no
   query is sent. An explicit theme in the config wins over it, as no-color.org's FAQ asks: *"User-level configuration
   files and per-instance command-line arguments should override the `NO_COLOR` environment variable."*
10. **Where the user sees the outcome.** The Orientation view gains one row, `colours`, which says which theme is in
    use and why, in words:
    - *"light background, detected"*;
    - *"no answer from the terminal, so plain colours"*;
    - *"plain, because NO_COLOR is set"*;
    - *"dark, from the config"*;
    - *"high contrast, from the config"*.

    It also says the depth, *"24-bit colour"* or *"256 colours"*, and, after decision 6, how many roles fell back to
    the terminal's own colour. This is the support line: what a user reads out when colours look wrong.
11. **The README key visuals** (`readme_visuals.rs`) render with an explicit dark palette, as now, and are regenerated
    once, here, in the new values.
12. **`NFR-A03` is amended:** *"palettes meet WCAG AA contrast"* becomes *"every colour stikk chooses meets 7 : 1
    (WCAG AAA) on every reference background of its theme, and on the background the terminal reports"*. `GU-07`
    lists the five themes.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does every chosen colour reach 7 : 1 on its theme's references, at both depths? | unit | a test in `theme.rs` computes the ratios from the palette itself, with the same formula as `palette059.py`. It fails below 7.0 |
| Do the bands of decisions 4, 5, 6 and 9 choose right? | unit | `COLORTERM`, `WT_SESSION`, platform; an answer, no answer, `COLORFGBG` forms; `NO_COLOR` empty and set; an explicit theme beside `NO_COLOR`; decision 6's guard on a mid-grey and a tinted background |
| Does every rendered screen keep its colours at 7 : 1? | headless (`TestBackend`) | **every view × {dark, light, high-contrast, plain} × {80 × 24, 120 × 40}**. For each cell, the fg is compared with **every** reference background of its theme, and with its own painted background in high-contrast. Contrast is computed from the buffer. Cells with `Reset` are the terminal's own pair and are skipped |
| Does stikk detect, and fall back, in a real terminal? | real terminal: **Linux**, Alacritty, by the architect | the same session isolation as every real run (`XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `HOME`); a fixed size; Wayland; selected by pid. Run stikk in Alacritty dark, and in Alacritty with a white background. The evidence is the Orientation row's text, read from the screen, plus **one screenshot of the worst case**, the white one |
| Windows Terminal and conhost | real terminal: **Windows**, the owner | the probe runs already asked for (§ below); after delivery, one stikk run in Windows Terminal light: the Orientation row and one screenshot |
| macOS Terminal | documented | the survey, and decision 4's 256-colour path. Stated as a limit, as the scope rules |

## What the Windows runs confirm

The two probe runs asked for on 2026-11-18 answer three questions. The probe is rebuilt on 2026-11-21 to record
`COLORTERM` too, SHA-256 `daa98b68…`.

| Question | If yes | If no |
|---|---|---|
| Does Windows Terminal (light scheme) answer, as *light*? | decision 5 as written | `system` falls back to `plain` there, safely. Note it in the guide |
| Does conhost answer? | the same | `plain` in conhost; the guide's Windows section suggests Windows Terminal |
| What do they set: `COLORTERM`, `WT_SESSION`? | decision 4 as written | decision 4's platform rule already covers Windows. Narrow it only if a run shows a terminal that lacks 24-bit colour |

**No answer changes a decision's shape.** At most, it changes a guide sentence or narrows decision 4. If one does, it
is amended here before the handoff is issued.

## Delivery

**One handoff**, after acceptance. Its commits put each behaviour's tests beside it:
1. the palette and its unit test;
2. depth, detection and fallback;
3. high-contrast and plain;
4. the Orientation row;
5. the headless matrix;
6. the text and the visuals.

Then the architect's Linux real-terminal check, and the owner's Windows Terminal check.

## Changelog

- `### Changed`: on a light terminal, stikk is now readable. *system* asks the terminal whether its background is
  light or dark. Body text is the terminal's own colour, and every colour stikk adds reaches 7 : 1.
- `### Added`: a high-contrast theme; `plain`; `NO_COLOR` is honoured; the Orientation view says which colours are in
  use, and why.

## Delivered

**One handoff, six commits, `e298aa1`..`3e284a8`, green on `main`:** CI, Docs, supply chain (its first run with
`terminal-colorsaurus`: no new licence line, no new duplicate), and the real-binary suite on the full platform matrix,
100 cases on Linux, macOS and Windows.
- **The palette:** decision 3's table, held by a test that computes every ratio (72 assertions), and a second test
  that pins it to `palette059.txt`.
- **The resolver:** pure, with `detect()` the one impure call, made before the alternate screen.
- **Themes:** `high-contrast` paints its pair last, over every cell nobody coloured. `plain`.
- **Orientation's `colours` row.**
- **The headless matrix:** every screen in four themes, at both depths and four sizes. With RFC 060 and 061 on top,
  1,469,298 contrast checks, none below 7 : 1.

**In review, C1:** the query test was renamed and completed (`system` and `high-contrast` ask, nothing else does).

**Real terminal, Linux** (the architect; Alacritty 0.17.0, Wayland, isolated `HOME` and XDG directories;
`.git-exclude/reports/059-readable-colours/linux/real-terminal-059-7afcdb4.txt`):
- on Alacritty's default background, *"dark background, detected · 24-bit colour"*;
- on white, *"light background, detected · 24-bit colour"*;
- no role fell back on either background.

**Real terminal, Windows** (the owner, 2026-11-23; Windows 10.0.26300, Windows Terminal 1.24.12741.0;
`.git-exclude/reports/059-readable-colours/windows/prikk-0.58-color-scheme/`):
- **the probe, One Half Light:** answered in 22 ms, background `#fafafa`, *light*. The terminal's own pair is
  10.9 : 1;
- **the probe, Campbell:** answered in 2 ms, `#0c0c0c`, *dark*, at 12.2 : 1;
- **both runs report `COLORTERM` unset and `WT_SESSION` set**, which is decision 4's rule for 24-bit;
- **stikk itself** (a test build of `main` at `7592866`), in Windows Terminal with One Half Light: Orientation's row
  reads *"light background, detected · 24-bit colour"*, and the screen is dark text on light, with the key line
  (`prikk-58.png`).

**Not checked:**
- **conhost** (the older Windows console). Both probe runs were in Windows Terminal. No decision depends on it: if it
  does not answer, *system* falls back to `plain`, which is safe on any background (decision 5).
- **macOS Terminal:** documented only, as the scope rules.

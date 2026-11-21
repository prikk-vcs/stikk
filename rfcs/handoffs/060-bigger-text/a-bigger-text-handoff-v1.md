# Handoff A — RFC 060: bigger text (v1)

**Companion to:** [RFC 060](../../accepted/060-bigger-text.md), **accepted 2026-11-21**. **Read the RFC first**: F1–F5,
decisions 1–10, and its *Review* section. The review is the owner's philosophy applied, and it is why decision 3
reads as it does. **Covers:** all of it. **0.21.0's increment 3.** **Not a release.**

> **What this increment is.** Bigger text means a smaller grid, and stikk refuses anything below 80 × 24 (F1). After
> this handoff:
> - stikk works down to **60 × 15**;
> - every overlay **wraps to the width it is drawn at**;
> - an overlay that does not fit **scrolls with ↑ ↓**, and keeps its actions;
> - **no confirmation is offered before its whole text has been seen**;
> - the status bar cuts nothing silently;
> - the too-small screen says what to do;
> - italic is gone, and bold comes from two named styles.
>
> **The owner's philosophy applies to every choice you make here** (the RFC's *Review*):
> - *"finally clean, safe and secure, and robust and sophisticated design"*;
> - *"APIs and UI/UX for users not to be confused or misunderstand"*.
>
> Where this handoff leaves a choice open, choose by those two lines, and say how in the request.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the full-matrix suite and supply chain there.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does any line overflow its area, on any screen, at 60 × 15 or 72 × 20? | headless, **structural** | the matrix test's name, its screen count, and that it records zero overflows; plus one planted overflow, caught |
| Do the three overlays F3 found cut now read whole? | headless | flattened-fragment captures at 60 × 18, the sentences F3 quotes |
| Does an overlay scroll, keep its actions, and fail closed? | headless | test names; text captures at 60 × 15 of the help overlay and a confirmation card: first opened, after `↓` to the end, and at a height where the actions cannot fit |
| Does the status bar cut nothing silently? | headless | test names; a capture at 60 columns with a long ref |
| Does the too-small screen fit? | headless | captures at 59 × 15, 40 × 10 and 20 × 5 |
| No italic; bold from two styles only | unit, a source check | the test's name, and **every call site of `heading()` and `selected()`**, listed |
| Real terminal | **the architect**, after your approval | not yours. Say that the build is ready |
| Nothing else regressed | real binary, 0.28.0 + 0.58.0; then the full matrix in CI | counts |

## 1. Measure first

Run `.git-exclude/reports/060-bigger-text/layout060.sh <base SHA> <empty scratch dir>`, and diff it against
`layout060-7afcdb4.txt`. The screen list is RFC 059's, so expect the same 25 rows and the same three dumps. **Anything
else differing: stop and report.** At your head, rerun it with its guard edit removed: `MIN_*` will then be 60 × 15
by your commit 4. Every overlay row in part 1 should read `yes/0` at 60 columns.

## 2. Commit 1: panels wrap their own text, and lines cannot overflow quietly (decision 2)

1. **`Panel` and `ListPanel` (`overlay.rs:3833`, `:3901`) take their text unwrapped**, as `stikk_core::Piece`s or
   `String`s with their styles. They wrap to the inner width of the region they are finally drawn in, **after**
   `centered` clamps it. The field docs that say *"pre-wrapped by the caller"* change to say so.
   - Every overlay renderer stops wrapping for its panel: `render_refusal`, `render_commit_would_refuse`,
     `render_confirmation`, the result cards, `wrap_cells`'s callers, and the rest. **List each one you changed.**
   - The Glossary (`render_glossary`) wraps to its drawn width the same way.
2. **The line helpers record overflow in tests.** `push` (the views'), `wrap_indented`, `wrap_verbatim`,
   `wrap_hanging` and the panels' own wrap know the width they wrap to.
   - Under `cfg(test)`, a line wider than its area is recorded, with the view's or overlay's name, into a
     thread-local list that a test can drain.
   - In a release build, nothing is recorded, and nothing changes.
   - **Width is counted as `text.rs` counts it today**, in chars. Display width is the carried i18n item, and it will
     change the count in this one place.
3. **The overflow test:** RFC 059's matrix (`shell/tests.rs`'s `screens()`), drawn at 60 × 18 in one theme, drains
   the list and **asserts it is empty**, naming every overflow. At this commit the minimum is still 80 × 24, so draw
   with the guard bypassed as `layout060.sh` does, through a `cfg(test)` seam, not by editing the constant.
4. **One planted fault**, caught: a renderer wrapping to `width + 5`. Then remove it.
5. **The three F3 captures**, at 60 × 18, asserting the whole sentences F3 quotes after flattening.

## 3. Commit 2: scrolling, pinned actions, and fail-closed confirmation (decisions 3 and 4)

1. **Scroll state lives on the overlay**, as the Glossary's `offset: Cell<u16>` does: an offset, and **a latch,
   `seen_to_end`**, set once the last row of the text has been on screen and never cleared for that overlay's life.
2. **Keys:** `↑` `↓`, and `k` `j`, only, through the existing `Action::Up` and `Action::Down`. **No new key, and no
   Page Up, Page Down, Home or End** (decision 3; MacBook keyboards and Apple's Terminal guide).
   - **In an overlay with an action list:** `↓` scrolls the text until its end, and only then moves the highlight into
     the actions. `↑` reverses that: through the actions to the first, then up the text.
   - **Where the text fits:** `↓` moves the highlight at once, exactly as today. Assert that today's behaviour is
     unchanged at 80 × 24.
3. **What is said, once each** (decision 3): the title keeps `Panel`'s *"— lines a–b of c"* while text is hidden.
   One foot line, in `dim`, on its own row, says *"↓ more below"* or *"↑ more above"*, and is absent when nothing is
   hidden. **The Glossary gains the same foot line.** `ListPanel` keeps its own title, and gains the line only if
   rows are hidden below its window.
4. **Pinned actions:** the action list, the confirm line and `Esc` sit below the scrolled text, never scrolled. When
   the pinned part plus **one** row of text does not fit, **the overlay offers no confirmation**:
   - its body is the one sentence *"Make the window bigger to see the whole card."*;
   - only `Esc` works.
5. **Confirm only after seen** (Tier Two and Tier Three; decision 3). Until `seen_to_end` is set:
   - the confirm line reads *"↓ read to the end to confirm"*;
   - the confirm key, Enter (today's *"Enter to confirm"*, `overlay.rs:1925`), does **nothing**, and dispatches
     nothing. With an action list, the reading path decides: `↓` reaches the actions only after the end;
   - a typed confirmation (Tier Three: typing a key id) accepts typing, but does not confirm.

   Where the card fits, `seen_to_end` is set on the first draw, and nothing changes.

   **Where the gate lives:** in core's confirmation path, not the renderer alone. The TUI passes "seen" with the
   confirm action, and core refuses a confirm without it. A renderer bug must not open the gate.
6. **Ids split evenly** (decision 4): any id wrapped in a panel or view goes through `text.rs`'s `even_rows`. **No id
   is shortened for width** anywhere it is not already short.

**Tests:**
- `↓` path, `↑` path;
- the foot line present and absent;
- the confirm key inert before `seen_to_end`, live after, and nothing dispatched before (through the request
  recorder);
- the fail-closed height;
- the Glossary's foot line;
- a 64-character id at 60 columns splitting 32 + 32;
- today's 80 × 24 behaviour unchanged.

## 4. Commit 3: the status bar's last step (decision 5)

`status_bar.rs`'s `fit` keeps its four steps. **Add a fifth, last:** shorten the focused ref from its end with `…`, by
the same `shortened` the repository name uses, and only when it does not fit alone. **The badges are never dropped.**

**Tests:** at 60 columns, a long repository name, a long ref, both, and a ref longer than the bar. Every shortened
part ends in `…`, and the badges are present in every case.

## 5. Commit 4: the minimum and the too-small screen (decisions 1 and 6)

1. **`MIN_WIDTH` 60, `MIN_HEIGHT` 15** (`shell.rs:17-18`). Remove commit 1's test seam, now unneeded.
2. **The too-small screen** (`shell.rs:307`): RFC 060 decision 6's three sentences, word for word, with `{w} × {h}`.
   - It is wrapped to the window, in the terminal's own colours (`Reset`).
   - **It drops sentences from the last** until it fits, down to 20 × 5. Below that, it draws as much of the first
     sentence as fits, wrapped.
3. **The matrix**, RFC 059's, now also draws **60 × 15 and 72 × 20**. Commit 1's overflow assertion runs at both, in
   one theme.

**Tests:** the too-small screen at 59 × 15, 60 × 14, 40 × 10, 20 × 5 and 10 × 3, each wrapped, never cut.

## 6. Commit 5: no italic, and bold from two styles (decisions 7 and 8)

1. **Italic, three sites:**
   - `view/patch.rs:220`: *"Note: {today's sentence}"*;
   - `view/block.rs:271`: *"Patches not shown: {today's reason}"*;
   - `view/changes.rs:298`: no new words.

   All three become `dim`, with no modifier.
2. **`theme.rs` gains `heading()` and `selected()`**, each returning a `Style` with `Modifier::BOLD`.
   - Every one of today's 51 bold uses either becomes one of these two, by decision 8's definitions, or loses its
     bold.
   - **This RFC does not change how the selection looks.** `selected()` is today's look, routed through one place.
3. **The source check**, beside `literal_guard.rs`'s approach: no `Modifier::ITALIC` anywhere in `stikk-tui/src`, and
   `Modifier::BOLD` only in `theme.rs`. Tests are exempt where they assert a modifier.
4. **`readme_visuals.rs`** keeps reading the modifier from the buffer. Regenerate the README images if they change,
   and say whether they did.

**In the request:** every call site of `heading()` and `selected()`, each with the one line it styles, so the review
can check each against decision 8.

## 7. Commit 6: the text (decisions 9 and 10)

1. **The guide, *Getting started*: *"Making the text bigger"*.**
   - **The terminals:** Alacritty, GNOME Terminal (and the other VTE terminals), Konsole, kitty, WezTerm, Windows
     Terminal, conhost, macOS Terminal and iTerm2.
   - **For each:** the keys for bigger and smaller, and where the font size is set.
   - **Every entry cites that terminal's own documentation, by URL, in the guide**, and you fetch each one. If a
     terminal's documentation does not say, write *"see its settings"* rather than a key from memory, and list it in
     the request.
   - **macOS Terminal:** Apple's *Keyboard shortcuts in Terminal on Mac* lists *"Make fonts bigger — Command-Plus
     (+)"* and *"Make fonts smaller — Command-Minus (–)"*.
   - The section ends with: stikk works down to 60 × 15, and what the too-small screen means.
2. **`TU-10`**, in RFC 060 decision 10's words.
3. **`CHANGELOG.md`, `## Unreleased`:** the RFC's lines.

## 8. The traps

1. **No caller wraps for a panel any more** (commit 1). One wrapped-for-the-terminal string left behind is F3 again.
2. **The confirm gate is enforced in core, not only drawn** (commit 2, item 5). A renderer must not be able to open
   it.
3. **No new keys** (commit 2). Page keys are not portable, and two meanings for `↑` `↓` in one card is what decision
   3 removed.
4. **One idiom per fact:**
   - *where*: the title's *"lines a–b of c"*;
   - *which key*: the one foot line.

   Do not add a third.
5. **The badges are never dropped** (commit 3). They are the signing state.
6. **No key in the guide written from memory** (commit 6).
7. **`inert`** on every repository string you touch, ref names in the status bar included.
8. **Paste captures as the file holds them, and grep before committing.**

## 9. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Package to
`.git-exclude/review-request/060-bigger-text-a/review-request-v1.md`, and lead with:
- §1's diffs;
- the overflow matrix's count and the planted fault;
- the scroll and confirm captures;
- the `heading()` and `selected()` call sites;
- the guide's citations.

# RFC 060 — Bigger text

**Status.** **Accepted by the project owner 2026-11-21**, after a review against the owner's philosophy (§ *Review*); [Handoff A](../handoffs/060-bigger-text/a-bigger-text-handoff-v1.md), delivered on `main` (`ca15867`..`472e6e0`); [Handoff B](../handoffs/060-bigger-text/b-the-help-scrolls-with-down-handoff-v1.md), a defect the real-terminal check found: **↓ does not scroll the help**, and Enter does (older than RFC 060). Delivered (`74fa0ef`). [Handoff C](../handoffs/060-bigger-text/c-block-detail-scrolls-handoff-v1.md): Block detail cannot scroll at 60 × 15, and the height check that would have caught it. **Proposed 2026-11-21** by the architect, as 0.21.0's increment 3 (`.git-exclude/release/0-21-0-scope.md`,
authorized 2026-11-04, last amended 2026-11-21). **No open question.**
**Measured** (`.git-exclude/reports/060-bigger-text/`):
- `grid060.sh` → `grid060.txt`: Alacritty 0.17.0's cell size at six font sizes, read from the window's own size
  report (`TIOCGWINSZ`). Wayland (niri), output 2133 × 1200 logical at scale 1.2, isolated config so that the
  default font is used.
- `grids060.py` → `grids060.txt`: the grid those cells give on common screens, in logical pixels.
- `layout060.sh` → `layout060-7afcdb4.txt`: every screen of RFC 059's headless matrix, drawn at five sizes below
  80 × 24 in a `git archive` copy with the size guard lowered, and the three overlays that touch the right edge,
  drawn whole at 60 × 18.

**Not measured:** other terminals' fonts, and macOS. A terminal's cell proportions vary a little by font. The
height-to-width ratio here, about 2.3 : 1, is typical of monospace fonts, and the conclusion rests on that ratio.
**Tracks.** `TU-10` (amended by this RFC), `NFR-A03`, `GU-07`. RFC 024 (wrap, never clip). RFC 059 (the palette
roles). RFC 061 builds on these layouts.
**Touches.** `stikk-tui`: `shell.rs`, `overlay.rs`, `theme.rs`, three views (`patch.rs`, `block.rs`, `changes.rs`),
and the views' fit at the new minimum. `TU-10`. The guide. The changelog.

## Summary

Making the text bigger is the main thing a person with weaker eyesight does, and in a terminal **bigger text means a
smaller grid**. On a common 1366 × 768 laptop, text just 1.6 times Alacritty's default leaves 96 × 23 cells. stikk
needs 80 × 24, so it shows only *"terminal too small"* (F1).

This RFC:
- lowers the minimum to **60 × 15**, which fits text **2.3 times** the default on every screen measured;
- makes every overlay wrap and scroll in the space it is actually drawn in (F3);
- says what to do when the window is still too small;
- removes italic;
- keeps bold for headings and the selection only;
- adds *"Making the text bigger"* to the guide.

## Findings

### F1 — bigger text, smaller grid: rows run out first

The grid a full-screen Alacritty gives, by font size (`grids060.txt`; 11.25 is Alacritty's default):

| font.size (× default) | 1366 × 768 laptop | 1440 × 900 (13″ MacBook) | 1920 × 1080 | 2133 × 1200 (measured here) |
|---|---|---|---|---|
| 11.25 (1×) | 149 × 35 | 157 × 41 | 209 × 49 | 232 × 55 |
| 14 (1.2×) | 126 × 29 | 132 × 34 | 177 × 41 | 196 × 46 |
| 18 (1.6×) | **96 × 23** | 101 × 26 | 135 × 32 | 150 × 35 |
| 22 (2×) | **78 × 18** | **82 × 22** | 109 × 26 | 121 × 29 |
| 26 (2.3×) | **65 × 15** | **69 × 18** | **92 × 22** | 102 × 24 |
| 32 (2.8×) | **52 × 13** | **55 × 15** | **74 × 18** | **82 × 20** |

The bold cells are below today's 80 × 24, where stikk shows nothing but *"terminal too small"*.
- **Rows run out long before columns.** A cell is about 2.3 times taller than it is wide (11 × 26 px here), and
  screens are wider than they are tall.
- **Text 1.6 times bigger** already locks a 1366 × 768 laptop out, by one row.
- **Doubled text** leaves at least 78 columns and 18 rows on every screen measured.
- **2.3 times** leaves at least 65 × 15.

### F2 — the views already fit; the frame around them does not

Every screen of RFC 059's matrix, 25 of them, was drawn at 80 × 24, 72 × 20, 60 × 18, 60 × 16 and 60 × 15 with the
guard lowered (`layout060-7afcdb4.txt`, part 1). **Every screen's marker stays on screen at every size.** The views
(Orientation, History, Block detail, Changes, Queue, Verify, Trust, Merge plan, Patch detail, Compare, Bundle) wrap
already, through RFC 024's and RFC 036's rules. At 60 columns, **no view row touches the right edge** beyond the one
or two rows that do at 80 × 24 too.

### F3 — overlays wrap for a wider panel than the one drawn

At 60 columns, the refusal card, the *commit prikk would refuse* card and the help overlay cut words in the middle
(`layout060-7afcdb4.txt`, part 2). From the refusal card at 60 × 18:

```
│  prikk declined to read this ref's history. The ref may n│
│  or its name may be mistyped. stikk cannot override prikk│
```

and from the would-refuse card: *"… prikk reports that it would refus"*, *"… Not"*, *"… preview the co"*. **The text
was wrapped to a width wider than the panel**, and the panel then cut it. That breaks RFC 024's own rule, *wrap,
never clip*, at any width below the one the text was wrapped for.

### F4 — what the frame spends at 15 rows

The title row, the status bar, and the key line RFC 061 proposes take three rows. If RFC 061 lands differently,
only this count changes, and decision 3 does not. A framed view's borders take two
more. **That leaves 10 rows of content at 15**, enough for a list, which scrolls. It is not enough for every overlay
at once: a confirmation card or the help overlay is taller than that.

### F5 — italic and bold today

- **Italic, 3 uses, each a note or a hint in the label colour:**
  - Patch detail's footnote (`view/patch.rs:220`);
  - Block detail's *"patches: <why unreported>"* (`view/block.rs:271`);
  - Changes' compare hint, *"d: compare …"* (`view/changes.rs:298`).

  Many terminal fonts draw italic thin, or not at all.
- **Bold, 51 uses:**
  - 27 in `overlay.rs`;
  - 3 each in `changes.rs` and `bundle.rs`;
  - 2 each in `trust.rs`, `queue.rs`, `patch.rs`, `status_bar.rs` and `shell.rs`;
  - 1 each in the other six views.

  When most headings, labels, keys and values are bold, bold no longer marks anything.

## Decisions

1. **The minimum window is 60 × 15** (`shell.rs`'s `MIN_WIDTH`, `MIN_HEIGHT`), and `TU-10` says so.
   - **Why 15 rows and not 18:** 18 fits doubled text everywhere (F1), but 15 fits 2.3 times. The cost is F4's
     scrolling overlays, which decision 3 pays everywhere anyway.
   - **Why 60 columns:** it is below every column count in F1's 2.3× row, so rows, not columns, decide the minimum.
     At 60 columns, a view's label column and a 12-character short id still leave room for the value on the same
     row.
2. **Every overlay wraps to the width it is drawn at** (F3). **The cause:** `Panel` takes its body *"pre-wrapped by
   the caller"*, to a width the caller chose, and then `centered` narrows the panel to the terminal, so the panel
   cuts what the caller wrapped wider. **The fix is ownership:** `Panel` and `ListPanel` take their text unwrapped,
   and wrap it themselves to the inner width they are finally drawn at, computed once per draw. No caller wraps for a
   panel again. **This is RFC 024's rule, held
   at every width from 60 up**. **It is enforced where lines are made:** the shared line helpers (`push`, `wrap_*`,
   `wrap_hanging`) know the width they wrap to, and in tests they record any line wider than its area (§ Tests).
   *Width is counted in characters today, as `text.rs` does. Display width for CJK is the carried i18n item, and it
   changes the count in this one place.*
3. **An overlay taller than its space scrolls, with the keys every view already uses, and its actions never hide.**
   - **One pair of keys, `↑` `↓` (and `k` `j`), as in every view today.** No new keys:
     - stikk handles no Page Up, Page Down, Home or End anywhere;
     - a MacBook keyboard has none of them;
     - Apple's Terminal guide documents ⌘ Page Up and ⌘ Page Down as Terminal's own scrolling, and does not say
       that the plain keys reach a program.
   - **In an overlay with an action list,** `↓` first scrolls the text down to its end, and only then moves the
     highlight into the actions. `↑` works the same in reverse: through the actions, then up the text. **One path,
     read top to bottom**, whatever the window's size. At a size where the text fits, `↓` moves the highlight at once,
     exactly as today.
   - **Where you are, and what to press, said once each, the same way on every scrolling surface**, the Glossary
     included:
     - **the title** keeps RFC 024's idiom, *"— lines a–b of c"*, which `Panel` and the Glossary already use. RFC 024
       §4 says to follow it, not invent a second;
     - **one foot line names the key**, and only while something is hidden: *"↓ more below"* or *"↑ more above"*, in
       the label colour, on its own row, never over text.
   - **The actions are pinned below the text** (the action list, the confirm line and `Esc`), and never scroll
     away. If the pinned part and one row of text cannot both fit, the overlay **does not offer its confirmation**.
     It says *"Make the window bigger to see the whole card"*, and only `Esc` works. **Fail closed.**
   - **A confirmation is offered only once its whole text has been on screen** (Tier Two and Tier Three). Until then,
     the confirm line reads *"↓ read to the end to confirm"*, and the confirm key does nothing. **No one confirms an
     operation whose consequences they could not see.** At a size where the card fits, this is invisible.
4. **Ids that do not fit wrap, and are never cut.** A 64-character id on a 60-column screen goes to a continuation
   row, as Block detail's id rows already do (RFC 037 Handoff C). It **splits evenly** (32 + 32), through `text.rs`'s
   `even_rows`, the rule RFC 056's typed card set: one rule for every long value, so a split id still reads as one
   value. A short id (12 characters, `SHORT_ID_CHARS`) is used where a view
   already uses one, and **no view newly shortens an id** for width.
5. **The status bar at 60 columns cuts nothing silently.** It keeps `status_bar.rs`'s order:
   1. drop the key hint;
   2. shorten the repository name, marked with `…`;
   3. shorten prikk's default, marked with `…`;
   4. drop prikk's default.

   One step is added, last: **the focused ref is shortened from its end, marked with `…`**, the same rule as the
   repository name, and only if it does not fit alone. The badges are never dropped, because they are the signing
   state. The full ref name is always in the view's own title and in Orientation.
6. **The too-small screen says what to do.** Below 60 × 15 it shows, wrapped to the window, and as plain text in the
   terminal's own colours:
   > This window is too small for stikk: it needs 60 columns and 15 rows, and has {w} × {h}.
   > Make the window bigger, or the text a little smaller (often Ctrl and −; on a Mac, ⌘ and −).
   > Help: https://prikk-vcs.github.io/stikk/ → "Making the text bigger".

   - **The window comes first**, because the user made the text bigger on purpose.
   - **The keys are "often"**, because they differ by terminal: kitty's are Ctrl, Shift and −. ⌘ and − is Apple's
     documented *"Make fonts smaller"*.
   - **The guide is named by its address.** *"The guide"* alone tells a user nothing about where it is.

   It must itself fit down to 20 × 5. It wraps, and it drops its sentences from the last.
7. **No italic.** The three uses become plain text in the label colour, each worded to say what it is. The words are
   fixed here:
   - **Patch detail's footnote:** *"Note: {today's sentence}"*, for example *"Note: stikk reads a patch's content from
     prikk 0.39 on; this is 0.30."*;
   - **Block detail's line:** *"Patches not shown: {today's reason}"*, for example *"Patches not shown: stikk reads a
     patch's content from prikk 0.39 on; this is 0.30."*. Today's *"patches: …"* reads as a list's label with a
     strange value;
   - **the Changes hint** is a key hint already, starting with `d:`, and stays as it reads.

   **No `Modifier::ITALIC` remains anywhere in `stikk-tui`** (§ Tests).
8. **Bold for two things only, each defined so it cannot be misread:**
   - **a heading** is the title in a view's or an overlay's frame, or a line inside one that names the section below
     it and carries nothing else, such as *"What you can do:"*;
   - **the selection** is the selected row's marker and id. **This RFC does not change how the selection looks.** It
     routes today's look through one style, and RFC 061 decides the full-row highlight there, so nothing changes
     twice.

   **Bold comes from two named styles in `theme.rs`, `heading()` and `selected()`**, and nowhere else.
   - Every other bold use becomes the plain style of its role.
   - A key named in a hint, such as `S` or `Esc`, is not bold. The key line (RFC 061) gives keys their place.
9. **The guide gains *"Making the text bigger"*,** in *Getting started*. Per terminal: the keys that make text bigger
   and smaller, and where its font size is set.
   - **The terminals:** Alacritty, GNOME Terminal (and the other VTE terminals), Konsole, kitty, WezTerm, Windows
     Terminal, conhost, macOS Terminal and iTerm2.
   - **Each entry cites that terminal's own documentation.** The handoff gives the link for each, and nothing is
     written from memory.
   - The section also says that stikk works down to 60 × 15, and what the too-small screen means.
10. **`TU-10` is amended:** *"minimum 80 × 24 (below: a legible 'terminal too small' screen)"* becomes *"minimum 60 × 15
    (below: a screen that says what to do); no italic; bold for headings and the selection only; an overlay that does
    not fit scrolls with ↑ ↓ and offers its confirmation only once its text has been seen"*.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does every screen fit at the minimum, with no word cut? | headless (`TestBackend`) | RFC 059's matrix, extended with **60 × 15 and 72 × 20**, in one theme. **Structural:** the line helpers record every line wider than the area they were given (decision 2), and the test fails, naming the view and the line. Plus the existing captures' flattened-fragment check on the three overlays F3 found cut |
| Does an overlay taller than its space scroll, keep its actions, and fail closed? | headless | the help overlay, a confirmation card and the would-refuse card at 60 × 15. `↓` scrolls to the end, then moves into the actions; `↑` comes back. The *"more below"* line shows. **The confirm key does nothing until the text has been seen**, and works after. At a height where the pinned part cannot fit, no confirmation is offered and only `Esc` works |
| Does the too-small screen say what to do, and fit? | headless | at 59 × 15, 60 × 14, 40 × 10 and 20 × 5 |
| Does the status bar cut nothing silently? | headless | at 60 columns: a long repository name, a long ref, both, and a ref longer than the bar. Every shortened part ends in `…`, and the badges are always shown |
| No italic, and bold only from the two styles | unit (a source check, as `literal_guard.rs` does for literals) | no `Modifier::ITALIC` in `stikk-tui/src`; `Modifier::BOLD` only in `theme.rs`. The request lists every call site of `heading()` and `selected()`, so the review can check each against decision 8's definitions |
| Real terminal | **Linux**, the architect, after approval | Alacritty at font.size 26 (2.3×), as large as niri tiles it. The evidence is `TIOCGWINSZ`'s grid, as F1 measured, and `script` transcripts of the help overlay at that grid: first opened (*"↓ more below"*), then after `↓` to its end. **Recorded state, not a screenshot.** One screenshot is added only if the owner says where niri saves them |
| Windows and macOS | documented | the guide's entries cite each terminal's documentation. The owner may try Windows Terminal at a large font, but it is not required |

## Delivery

**One handoff, after acceptance.** Each behaviour's tests are in the same commit as the behaviour:
1. overlays wrap to their drawn width, and the line helpers record overflow (decision 2), with the structural check;
2. overlay scrolling, pinned actions and the confirm-after-seen rule (decision 3), and the even id split (decision 4);
3. the status bar's last step (decision 5);
4. the minimum and the too-small screen (decisions 1 and 6), with the matrix at 60 × 15 and 72 × 20;
5. no italic, and bold from two styles (decisions 7 and 8), with the source check;
6. the text (decisions 9 and 10).

## Changelog

- `### Changed`: stikk works in a window as small as 60 × 15, so text can be made about twice as big, or more, on a
  laptop. A window that is still too small says what to do. No italic; bold marks headings and the selection only.
- `### Added`: the guide's *"Making the text bigger"*, for each common terminal.

## Review, 2026-11-21: against the owner's philosophy

The owner asked for a careful review against two lines: *"finally clean, safe and secure, and robust and
sophisticated design"*, and *"APIs and UI/UX for users not to be confused or misunderstand"*. The first draft had
these faults, all now corrected above:
1. **Scrolling needed keys that do not reach the program everywhere** (PgUp and PgDn). It also gave `↑` `↓` two
   meanings in one card. Decision 3 now uses only the keys every view already uses, and one reading path: down to
   the end, then into the actions.
2. **A confirmation could be offered while its consequences were scrolled out of sight.** This was not safe. Decision
   3 now offers it only once the text has been on screen, and fails closed when the actions cannot fit.
3. **The too-small screen told a user to make the text smaller**, which undoes what they chose. It named keys as
   fact that differ by terminal, and pointed to *"the guide"* without saying where it is. Decision 6 now puts the
   window first, says *"often"* for the keys, and gives the address.
4. **The status bar could cut a long ref name with nothing to show it.** A cut name can read as another ref. Decision
   5 marks every shortening with `…`, and never drops the signing badges.
5. **"Note: patches: …" read awkwardly, and the wording was left to the handoff**, which is not final. Decision 7 now
   fixes the words.
6. **"Heading" was undefined, and the selection's look was split across two RFCs**, inviting an interim state.
   Decision 8 now defines both, and changes nothing about the selection's look here.
7. **The fit test was not checkable as written** (*"the text it was meant to draw"*). It is now structural: the line
   helpers record any line wider than its area. The unverifiable "bold only on heading rows" test is replaced by a
   listed set of call sites.
8. **A wrapped id split at an arbitrary column.** It now splits evenly, by the rule RFC 056 set.
9. **The real-terminal check needed a screenshot path this session cannot read.** It now relies on recorded
   transcripts, with a screenshot only if the owner allows one.
10. **The first correction to decision 3 invented a second "there is more" idiom.** `Panel` and the Glossary
    already say it in their titles (RFC 024 §4: follow it). Decision 3 now keeps that idiom for *where*, and adds one
    foot line for *which key*, on every scrolling surface. Found while writing the handoff, from `overlay.rs`'s
    `Panel` doc, which also showed F3's cause: the caller wraps, and the panel narrows afterwards.

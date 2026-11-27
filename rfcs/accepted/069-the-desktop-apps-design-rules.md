# RFC 069 — The desktop app's design rules

**Status.** **Accepted by the project owner 2026-11-27** (*"Accepted."*), with **Q1 answered: B, the system's own
interface font** (*"B"*). See Q1. [Handoff A](../handoffs/069-the-desktop-apps-design-rules/a-documentation-theme-and-font-handoff-v1.md).
**Proposed 2026-11-27** by the architect, as 0.23.0's increment 1 (`.git-exclude/release/0-23-0-scope.md`, revision 2,
authorized by the project owner 2026-11-25).
**Measured** on `main` at `7c77193`, against Snora 0.65.0 and iced 0.14.0 as RFC 065 measured them.
**Tracks.**
- RFC 065 (done), whose outcome builds `stikk-gui` on Snora and carries eight findings here;
- RFC 067 (done), the shared program a front end draws, and its reading gate (decision 4, Handoff C);
- RFC 060 and RFC 061, the terminal's reading gate and its one action catalog;
- RFC 001 (done), whose acceptance this RFC amends;
- knotra's analysis (`.git-exclude/reports/065-gui-toolkit/knotra-analysis-2026-11-25.md`), and the owner's GUI rules
  (`.git-exclude/rules/project-instructions-gui-common.md`).

**Touches.** This RFC writes rules. It builds nothing.
- **Its handoff edits documentation only:**
  - `docs/src/reference/requirements.md` (`NFR-A02`);
  - `docs/src/reference/external-design.md` (`GU-08`, and a new `GU-10` for this RFC's rules);
  - `docs/src/reference/internal-design.md` (`FE-03`);
  - RFC 001's Status (an amendment note);
  - the README's statement about assistive technology.
- **It also takes three measurements on this machine:**
  - whether iced reads the desktop's light or dark setting;
  - whether naming the interface font makes iced draw it;
  - which fonts bracket the label-fit guard.
- **The rules are built by RFC 070,** with the tests named in each rule.

## Summary

**stikk's desktop app is for people who find small, faint or crowded screens hard: older users, and users with low
vision.** It must also never confuse anyone (the owner's philosophy: *"finally clean, safe and secure, and robust and
sophisticated"*). RFC 065 chose the toolkit, Snora on iced, and found what that stack does and doesn't do by itself.
knotra, the owner's Snora app, has already paid for most of the lessons.

This RFC writes the rules the desktop app is built to, in seven groups:
1. **seeing:** type, colour and contrast;
2. **space:** layout, targets and zoom;
3. **hands:** the keyboard;
4. **reading:** the reading contract every front end keeps;
5. **words:** words, languages, and right-to-left text;
6. **calm;**
7. **what the app says about assistive technology.**

**Each rule has its reason, and the test that holds it.** A rule without a test is a wish. knotra's weakest rules are
the ones its guards only checked by name (its analysis, §4).

## Findings

### F1 — what Snora and iced give, and what they don't (RFC 065, measured)

| Property | By default | Measured in |
|---|---|---|
| Text, labels and focus ring against stikk's floors (7 : 1, 4.5 : 1, 3 : 1), in all four presets | **meets them** | M4 |
| Disabled controls | **fail:** label 1.48–2.90 : 1, outline 2.26–2.90 : 1 | M4 |
| Button height | **27.5–28.3 px;** 44.0 px with `tokens.targets.min = 44` | M4 |
| A scrollable's bar | **overlays its content** (`Scrollbar::default()` has `spacing: None`), so at 200 % it covered the end of a line | M5 |
| Zoom 115, 130 and 150 % | every specimen row fits at 1280 × 720 | M5 |
| Layout direction | Snora mirrors its regions under `LayoutDirection::Rtl`; **text inside them stays where it was** | the RTL check, R1 and R5 |
| An id inside Arabic text | **breaks** in a left-to-right line unless it's isolated (U+2066 … U+2069) | R4 |
| The accessibility bus | **nothing:** iced 0.14 registers no tree | M7 |
| Japanese input | the committed string reaches the program; the preedit shows in the field; Windows sends `Named(Process)` keys while composing | M6, W3 |
| The desktop's light or dark setting | `iced::system::theme()` and `theme_changes()`, through `mundy` on Linux (`iced_winit` feature `linux-theme-detection`, on by default) and through winit elsewhere | source; **measured by this RFC's handoff** |
| Headless layout | `iced_test::Simulator` substitutes Fira Sans when the default font is `Font::DEFAULT` | RFC 065 Handoff A, §3.4 |
| The default font in a real window | **not the system's font:** cosmic-text 0.15 sets the sans-serif family to **"Open Sans"** by name (`src/font/system.rs:157-159`, *"TODO: configurable default fonts"*), overriding fontconfig's choice. So iced draws Open Sans where it's installed (as on this machine), and a platform fallback where it isn't | source; **measured by this RFC's handoff** |

### F2 — the reading contract has two halves, and the desktop's half is new

`stikk-app` enforces its half: every new card on top is a new `Surface`, and the latch (`seen`) belongs to the
surface (RFC 067 Handoff C). **The front end's half** is to report a `Reading` only for what it laid out and the user
could see.
- **The terminal** measures every frame, and since 0.22.1 it reports nothing for a surface it hasn't drawn.
- **The desktop app** learns visibility from iced's `sensor` events, against iced's viewport.

**RFC 065 found the gap:** on Windows, a card reported its end visible on arrival, before the window's scaled size
had arrived (W2, attempt 1). iced's source shows the layout, the sensor and the window size all read one viewport.
What the evidence can't show is whether that viewport was the window on screen.

### F3 — knotra's lessons that cost it a release each (its analysis, §3)

- Disabled controls faded to about 2.2 : 1.
- Semibold Japanese fell back to a serif face.
- Guessed label widths broke words.
- 14 px was still too small for its users.
- A disabled ghost button gained an outline.
- Unstyled widgets escaped every test until the base theme was Snora's.
- Focus return after a card closes was never proved.
- Text was joined from fragments in English word order.

### F4 — stikk starts where knotra had to migrate to

These are already in place:
- **One layer enum:** `stikk_app::Overlay`.
- **One action catalog:** `stikk_core::actions`. The terminal's keys, its key line, its action list and its palette
  all come from it (RFC 061).
- **The reading gate is the program's.**
- **The letter rule exists:** `App::wants_text_input`. The terminal turns printable keys into text while a text
  card is open.

So the desktop app copies none of knotra's parallel structures. It draws stikk's.

## Decisions

Each rule names the test that holds it. **H** means a headless test (`iced_test`), run on every CI run. **R** means a
real-window check, run in the nested compositor by knotra's fixed-size method. **C** means a CI job on a named
runner. Tests are built by RFC 070. A rule RFC 070 can't yet test is stated as open, not as held.

### 1. One design crate, and the views import only from it

**`stikk-gui-design`** (`publish = false`) is the only place that names a colour, a size, a font, a spacing or a
Snora style. The views live in `stikk-gui` and import from it. They never import `snora::design` or
`iced::Color`.
- **Why:** one place to change, and a rule a compiler can hold. knotra's 212 hand-written size sites were kept
  consistent only by a guard that scanned names (its analysis, §4).
- **Test:** a `deny`-style check that `stikk-gui`'s source has no `snora::design` and no `iced::Color` path. That's a
  name scan, kept because it's cheap. It doesn't prove the sizes; rule 2's H test does that.

### 2. Type: sizes by purpose, from a role function

- **Roles, not sizes:**
  - `body` (16 px): anything read in order to act or understand;
  - `meta` (14 px): incidental metadata only (times, counts, ids beside their words);
  - `title` (18 px) and `heading` (24 px).
  - **Nothing is below 12 px,** and nothing a user must read in order to act is below 16 px.
- **Views call a role function** (`body(…)`, `meta(…)`, `title(…)`). No view names a size.
- **Bold, never Semibold,** and emphasis never carries meaning alone. Noto Sans CJK has no SemiBold, so Japanese
  falls back to a serif (F3). Bold exists in the sans family on all three platforms.
- **The system's own interface font, named explicitly** (Q1, the owner's ruling B):
  - **Linux:** the face fontconfig resolves for `sans-serif`, which is the user's own choice.
  - **Windows:** Segoe UI.
  - **macOS:** the system font. **Unverified:** there's no Mac here, and the documentation says so.
  - The font is named, so iced can't substitute Open Sans (F1).
  - The system's fallback fonts cover the user's languages, so Japanese and Arabic need no bundled face later.
  - **No font is bundled.**
- **Tests that hold for fonts stikk can't see:** the label-fit guard (rule 4) runs with **two installed fonts, the
  narrowest and the widest available on the CI runner,** chosen by their measured average advance width and recorded.
  A label that fits both fits any reasonable interface font. That's safe because no layout depends on exact text
  width (rule 4).
- **Test (H):** every text in every screen's tree, at every zoom, has a size from the role table. That's measured on
  the rendered tree, not by scanning names.

### 3. Colour: four presets, the desktop's setting, and floors

- **Presets:** Snora's `light_aaa`, `dark_aaa`, `hc_light` and `hc_dark`, as measured in RFC 065.
- **The default follows the desktop** (0.23.0 scope, decision 2, authorized). iced's `system::theme()` reports Light
  or Dark, and stikk opens in `light_aaa` or `dark_aaa` to match. When the desktop reports no preference, stikk opens
  in **`light_aaa`** (knotra: *"for many older eyes"*). A preset the user chooses is remembered, and wins.
- **Floors:**

  | What | Normal presets | High-contrast presets |
  |---|---|---|
  | Content text | 7 : 1 | 7 : 1 |
  | Labels, placeholders, **unavailable labels** | 4.5 : 1 | 7 : 1 |
  | Lines, the focus ring, badges against the page | 3 : 1 | 4.5 : 1 |

- **Unavailable controls stay readable and keep their shape** (RFC 065's ruling 2). `stikk-gui-design` overrides
  Snora's disabled style so the label meets 4.5 : 1 and the outline 3 : 1. An unavailable control never gains a
  shape its available form lacks (F3). That stays until Snora's own style meets the floors. It's a Snora issue, not
  a stikk exception.
- **Colour never alone:** every status is a symbol and words, and a colour only reinforces them. **Blue means "this
  acts"** and nothing else.
- **Tests:**
  - **(H) The contrast suite:** every text and every mark, in every screen, in all four presets, measured from the
    rendered pixels at 100 % and 150 %, against the table.
  - **(H)** The disabled overrides, which fail today without them. **That's the run that fails it.**
  - **(R)** The default follows the desktop on Linux: the mode the session reports, and the fallback when none is
    reported. The owner's setting is read, never changed.
  - **(C)** The same on GitHub's Windows runner, with the registry's `AppsUseLightTheme` set to 0 and then 1.
  - The handoff first measures whether iced reports the setting here at all.

### 4. Space: lines not panels, start and end, and room for every word

- **Lines, not panels:** lists are separated by 1 px lines. A raised surface means only "this floats above the
  screen" (a card on a scrim).
- **Start and end, never left and right,** in the design crate's API and in every rule and test. Content is aligned
  to the start edge **by the design crate**; no view aligns text itself (R5: Snora mirrors regions, not text).
- **No layout depends on English text length.** A label sits above its value when space is short, and a row of chips
  wraps; a chip's label never wraps mid-word (F3).
- **Every scrollable reserves its bar's room** (`Scrollbar::spacing`), so no text is ever drawn under it (F1, M5).
- **Tests:**
  - **(H) The label-fit guard:** every label fully visible, compared against **the scrollbar's own rectangle**, not
    only the viewport, because `visible_bounds` passes a covered glyph (RFC 065, M5). It runs at 100 % and 150 %, at
    1280 × 720 and at 800 × 600, in all four presets.
  - **(H)** The same guard with `Scrollbar::spacing` removed must fail on the commit confirmation at 150 %. **That's
    the run that fails it.**

### 5. Targets and zoom

- **44 px targets,** set explicitly (`targets.min = 44`). Snora's default is about 28 px.
- **One zoom,** 100, 115, 130 and 150 %, through iced's `scale_factor`.
  - Ctrl+Plus, Ctrl+Minus and Ctrl+0, from the catalog (rule 7).
  - Remembered in stikk's own config (`stikk-state`).
  - Text size never gives way to density.
  - **200 % isn't offered:** at 1280 × 720 it leaves 640 × 360 logical pixels, and RFC 065's specimen didn't fit.
- **Tests (H):** every control's hit area is at least 44 × 44 logical px, at every zoom. Zoom persists across a
  restart (logic level).

### 6. The keyboard: Snora's model, and stikk's catalog

- **Snora's keyboard model from the first line:**
  - its `Shortcuts` and `Escape` routing, F6 for zones, Tab for controls;
  - no model of stikk's own beside it (knotra retired its own for this reason).
- **One Escape closes a card, even from a text field, but never during an IME composition.** RFC 065 found the first
  Escape is taken by the field, on Linux and on Windows. The front end hands Escape to the program unless a
  composition is in progress.
- **`Named(Process)` keys are no action** (W3: Windows sends them while an IME composes).
- **A letter never acts while a text field has focus.** Focus, not the card, decides, because in a GUI a field can
  have focus outside a text card. `App::wants_text_input` stays the program's answer for text cards.
- **Focus settles after every update:** one step compares the top layer before and after, and puts focus on the new
  layer's entry, or back where it was when a card closes. **No open or close site touches focus.**
- **A card's entry is never a control that starts a change.** On a result, it's the way out. On a confirmation, it's
  "Back", and the confirming control is reached deliberately.
- **Tests:**
  - **(H)** Every catalog action reachable by its key from the screens where the catalog offers it.
  - **(H) Focus return:** after every card closes, focus is where it was before it opened. **knotra never proved
    this.**
  - **(H)** The Escape-from-field case, which fails today. **That's the run that fails it.**
  - **(R)** One keyboard pass through the reading screens, on the real window: the keystroke from the OS to the
    message is the one step headless can't reach.

### 7. The same catalog, the same letters

The desktop app binds **exactly the terminal's keys,** from `stikk_core::actions`.
- Its action list, palette, help and key line are drawn from the catalog. So, as in the terminal, a key exists only
  where the catalog binds it, and a sheet can't list a key that does nothing.
- Desktop-only keys (zoom, F6) are added to the catalog as GUI-scoped entries, not hard-coded.
- **Test (H):** the shortcuts sheet is built from the function the keys use, and every row's key performs its row's
  action (knotra's 11 of 19, F3).

### 8. The reading contract (the front end's half)

**A front end reports a `Reading` only for a surface it has laid out, in a viewport it has confirmed from the window
itself.**
- **Confirmed** means the front end has the window's physical size and scale factor from the window, not only from
  its own request, and the viewport it laid out against matches them.
- Until then it reports nothing, so the program's latch can't take a report for an end nobody could see (F2).
- It's the desktop counterpart of 0.22.1's terminal fix, where a surface not drawn reports nothing. It's stated for
  every front end.

**Tests:**
- **(H) RFC 067 Handoff C's four cases:**
  - a Loading card replaced by its confirmation starts unread;
  - a card never drawn reports nothing;
  - a card replaced below the top keeps the top's reading;
  - the two latches agree through a swap.
- **(H)** A reading reported before the viewport is confirmed is refused. That fails today, because the prototype
  reports at once. **That's the run that fails it.**
- **(R)** M3's real-window sequence is the desktop app's regression check, run in the nested compositor whenever its
  reading code changes.
- **(C) On GitHub's Windows runner:**
  - open straight onto the long confirmation at 250 %;
  - log the physical size, the system scale, the program's scale and the viewport;
  - assert that no `seen` arrives before they agree.

  That's RFC 065's open question, settled by a machine, not an owner session.

### 9. Every banner the program sets is drawn

`stikk-app` reports some refusals only as a banner: it closes the card and sets `App::banner()`. **The desktop app
draws every banner, in the status area, until the program clears it.**
- **Why:** in RFC 065's W4, prikk refused "nothing to commit", and the prototype drew no banner, so the card simply
  closed. A screen that silently does nothing is the confusion this project exists to prevent.
- **Test (H):** for every banner `stikk-app`'s tests produce, the desktop tree shows its text.

### 10. Calm

- **No animation that must be watched.** Any transition is at most 200 ms, and nothing depends on seeing it.
- **No shadows,** except the one that marks a floating card.
- **Show less:** nothing on an empty screen that has no use.
- **One primary action per area.**
- **No meaning by hover alone.** An unavailable action says why beside it.
- **Test:** the design crate exposes no animation or shadow token beyond these. That's a name check, stated as
  such.

### 11. Words

- **The desktop app uses the terminal's words:** the same catalog labels, the same glossary, the same refusal text
  from prikk shown verbatim. There's no second vocabulary.
- **Which prikk words stay visible** is decided once, for both front ends, through the catalog and the glossary
  (RFC 059–061). A word changed for one front end is changed for both.
- **One word per action.**
- **Plain wording first,** with prikk's own term in the glossary, one key away.
- **Test (H):** every label the desktop app draws for an action is the catalog's label for it.

### 12. Text that is not words: ids, refs, paths, keys and prikk's lines

**Every id, ref name, path, key id and line of prikk's text is shown as its own isolated left-to-right piece**
(U+2066 … U+2069), placed by the design crate's text function. It's never concatenated into a sentence by hand.
- **Why:** R4. Without isolation, an id inside Arabic text breaks into pieces. It costs nothing in English, and it
  makes the later right-to-left stage a matter of words, not of finding every id.
- **Test (H):** every such value the views render passes through the design crate's isolating function. The test
  checks the rendered text for the isolates.

### 13. Languages: constraints now, the catalog later

**Multilingual support is its own task, after the base screens and before the app is published** (the owner's
ruling; the scope's recorded task). Until then the desktop app is English. It's built so the task stays cheap:
- **No text is assembled from fragments.** Every sentence with a value is one template, with named placeholders.
- **One text source:** every word the desktop app shows comes from one place (the catalog, the glossary, or a
  template table in `stikk-gui`), keyed, so a second language is a second table.
- **No layout depends on English length** (rule 4).
- **For the later right-to-left stage:**
  - each line's direction comes from the interface language, not from its first word (R3);
  - mirroring is Snora's `LayoutDirection`;
  - the terminal stays left-to-right.
- **Test (H):** a source check that no view formats a user-facing string except through a template.

### 14. What stikk says about assistive technology

Path B accepted that iced exposes nothing to screen readers (M7). **The documentation says so plainly:**
- **The README's statement:** *"stikk's desktop app and its terminal front end don't yet work with screen readers.
  The desktop app is built on iced, which doesn't yet expose an accessibility tree. stikk will support screen
  readers when iced does."*
- **`NFR-A02`:** *"GUI: full keyboard operability with visible focus for every action. Platform accessibility APIs
  (names, roles, focus order) when the toolkit exposes them."*
- **`GU-08`:** the same change, with this RFC's rules referenced.
- **`FE-03`:** the toolkit is Snora on iced (RFC 065), and the same condition applies.
- **RFC 001's acceptance:** an amendment note, dated, pointing to RFC 065's outcome.

**Not promised:** following the OS text size or the OS high-contrast setting (knotra's analysis, *"What knotra does
not do"*). stikk's zoom and its two high-contrast presets are the answer, and the documentation says which to use.

## Q1 — the font (answered 2026-11-27: B)

**The question was whether to bundle Noto Sans** (OFL-1.1). The architect recommended it first, for identical glyphs
everywhere and exact tests.

**The owner asked whether that's good for various users,** some of whom are familiar with their system fonts. That
led to F1's last row: iced doesn't draw the system font today either. Three ways were put:
- **A. Bundle Noto Sans:** the same everywhere, but it overrides the user's own choice.
- **B. The system's own interface font, named explicitly:** familiar, follows the user's choice, and covers their
  languages. Tests bracket it with a narrow and a wide font.
- **C. Bundle, with a setting to use the system font.**

**The owner chose B.** Rule 2 states it. No font is bundled, so there's no licence question.

## What this RFC does not do

- **No code.** RFC 070 builds the design crate, the shell and the screens, with the tests named here.
- **No second language.** Rule 13 keeps the way open; the multilingual task walks it.
- **No delivery design:** that's RFC 071.
- **No change to the terminal front end.** Rules 7, 8, 11 and 12 describe what it already does, or will do in RFC 073.

## Evidence plan

| Question | Level | Where |
|---|---|---|
| Does iced read the desktop's light or dark setting on this machine, and fall back when none is reported? | real, read-only: the prototype logs `system::theme()` in the nested compositor; once as the session is, once with the portal unreachable (`DBUS_SESSION_BUS_ADDRESS` unset), which must give no preference and so `light_aaa`. **The owner's setting is read, never changed** | this RFC's handoff |
| Does naming the interface font work: does iced draw the face named, not Open Sans? | logic: the font database's answer for the name. Real: one capture in the nested compositor, its words read. **Linux** here (fontconfig's `sans-serif`). **Windows** (Segoe UI) is RFC 070's first Windows CI job. **macOS** is stated as unverified | this RFC's handoff (Linux) |
| Which installed fonts are the narrowest and the widest for the label-fit guard? | the average advance width of each candidate, measured by script, here and from the CI runner's font list | this RFC's handoff |
| Every rule's test | H, R or C, as each rule says | RFC 070 |

Nothing here needs the owner's desktop or a Windows session.

## Delivery

**One handoff after acceptance, Handoff A:**
- the documentation of rule 14;
- the theme measurement;
- the two font measurements (naming the interface font, and the narrow and wide test fonts).

RFC 070's handoffs build the rules. RFC 070 is written next.

## Changelog

None. Rule 14's documentation is a docs change; nothing ships.

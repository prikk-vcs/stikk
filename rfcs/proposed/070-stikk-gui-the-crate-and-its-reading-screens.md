# RFC 070 — `stikk-gui`: the crate and its reading screens

**Status.** **Proposed 2026-11-28** by the architect, as 0.23.0's increment 2 (`.git-exclude/release/0-23-0-scope.md`,
revision 2, authorized by the project owner 2026-11-25). **No open question.**
**Measured** on `main` at `89a6bdc`, and on RFC 065's prototype (`.git-exclude/reports/065-gui-toolkit/snora/proto/`).
**Tracks.**
- RFC 069, whose fourteen rules this RFC builds, with the tests each rule names;
- RFC 067, the program (`stikk-app`) every front end draws;
- RFC 065, whose prototype is the measured starting point, and whose outcome chose Snora 0.65 on iced 0.14;
- RFC 068, whose rule is that every advisory in the tree is seen.

**Touches.**
- **two new workspace crates,** both `publish = false`: `crates/stikk-gui-design` and `crates/stikk-gui`;
- `deny.toml`: the GUI tree's licences and advisories;
- `.github/workflows/ci.yml`: a GUI job on Linux, macOS and Windows;
- the release crate-list test, which must not count unpublished crates;
- `docs/src/contributing/`: how to run the desktop app from source.

**Nothing ships.** The release builds `-p stikk`, and the desktop app is published in a later release (the scope's
decision 1).

## Summary

**0.23.0 builds the desktop app's foundation and its reading screens, tested, and doesn't publish them.** The app
draws `stikk_app::App` (RFC 067). It adds no behaviour of its own. Everything it shows, every key it takes and every
reading it reports goes through the program's own types, as the terminal does.

**It's built in three handoffs, each reviewed before the next:**
- **A:** the design crate, the shell, the reading contract, Orientation and History.
- **B:** the other reading screens, and zoom.
- **C:** the action list, the palette, help and the glossary, and the read-only cards.

The writing cards (the commit message, the confirmations that change the repository, the ref-name and path
prompts) come in the release that publishes the app. Until then the desktop app opens no card that writes.

## Findings

### F1 — the prototype is the starting point, and how much it took

RFC 065's prototype drives the real `stikk-app` on a real repository. Its whole seam to the program is:
- **25 lines** turning iced's keys into `stikk_app::Intent`;
- **14 lines** reporting a `Reading`;
- the effects loop.

The other 438 lines of its 477 are drawing and measurement instruments (M9). **The program needs nothing new from
`stikk-app` to be drawn.** RFC 067's split holds.

### F2 — what the desktop app must draw (measured in `stikk-app`)

- **Twelve screens** (`App::focus`'s views): Orientation, Loading, History, Block detail, Trust, Changes, Queue, Patch
  detail, Compare, Merge plan, Bundle, Verify.
- **About 34 cards** (`stikk_app::Overlay`).
  - **Read-only:** Glossary, Loading, Refusal, Refusals, Stale, the action list, the palette, the ref picker, and
    the results that only report.
  - **Writing:** the commit message, the confirmations and consents, and the name, path and key prompts.
- **The terminal draws them in about 6,150 lines** (the scope's F6).

### F3 — what the GUI's tree brings (RFC 065 M1 and its `cargo deny` runs)

- **About 242 crates,** against the terminal launcher's 80. A stripped binary of about 18 MB, against 2.6 MB.
- **MSRV:** iced 0.14 and Snora 0.65 both declare **1.88**, the workspace's own. No raise is needed.
- **Licences beyond `deny.toml`'s list:** BSD-2-Clause (`arrayref`), BSD-3-Clause (`tiny-skia`, `tiny-skia-path`),
  ISC (`libloading`), CC0-1.0 (`hexf-parse`). **All permissive, so the owner's licence rule (2026-11-24) admits
  them.**
- **Advisories, all `unmaintained`, none a vulnerability** (2026-11-25, the prototype's lock): `paste`
  (RUSTSEC-2024-0436), `ttf-parser` (RUSTSEC-2026-0192), and `zvariant`, `zbus_names` and `zvariant_derive`
  (RUSTSEC-2026-0370/-0371/-0372, through the portal reader iced uses for the desktop's theme).
- **The release is unaffected:** `build-release.yml` and `installers.yml` build `-p stikk` only.

### F4 — what can be tested where (RFC 065 and RFC 069 Handoff A)

- **Headless (`iced_test` 0.14):**
  - the program's whole state is assertable after every step, and the rendered tree and its pixels can be
    measured (contrast, label fit, hit areas);
  - but **keys enter at the message boundary** (instruction events don't reach subscriptions);
  - **one `Emulator` capture per process** (two upstream defects);
  - the `Simulator` substitutes Fira Sans unless a font is named;
  - and **`system::theme()` never resolves** (the emulator drops `Action::System`).
- **Real windows, here:** the nested compositor at an asserted 1280×720, with the session script's checks (focus by
  pid, one instance, keys accounted for, captures sized).
- **Real windows in CI:** **not yet measured.** Whether GitHub's Windows and macOS runners can open the app's window
  and report its viewport is this RFC's Handoff A's first question (§7). RFC 069 rule 8's Windows test depends on
  it.

## Decisions

### 1. Two crates, and the line between them

- **`stikk-gui-design`** (RFC 069 rule 1) holds everything that names a look:
  - the role text functions (`body`, `meta`, `title`, `heading`);
  - the four presets with stikk's overrides (disabled controls, 44 px targets, `Scrollbar::spacing`);
  - the isolating text function for ids, refs, paths and keys (rule 12);
  - start-edge alignment;
  - the font choice (rule 2: the system's interface font, named);
  - the line, card and scrim styles.

  It depends on Snora and iced, and on nothing of stikk's.
- **`stikk-gui`** holds the views, the key mapping, the reading report, the effects loop and the binary. It depends
  on `stikk-app` and `stikk-gui-design`, **and never on `snora::design` or `iced::Color` directly** (rule 1's check).
- **Both are `publish = false`.**
  - The release crate-list test (`stikk-real-binary/tests/release_crates.rs`) counts published crates only, and is
    shown not counting these two.
  - The package gate (`cargo package --workspace`) excludes them by name, as it excludes `stikk-real-binary`.
- **The binary is `stikk-gui`, run from source** (`cargo run -p stikk-gui`). Its user-facing name and its packaging
  are RFC 071's.

### 2. The window and its launch

- **A launch size of 1280×720 logical** (the GUI rules' size, obtained at launch through iced's window settings), and
  a minimum size at which rule 4's guard passes at 100 %. **The app never resizes itself.**
- **The repository** is the working directory, or a path given on the command line, exactly as the terminal's
  `stikk` takes it.
- **One instance per repository is not enforced in 0.23.0.** The program and prikk already handle two readers, and
  prikk 0.59 waits for a writer (that's for RFC 074).
- **Isolated directories:** the app reads config and state through `stikk-state` as the terminal does. Its own zoom
  and preset choice live there (rules 3 and 5).

### 3. The shell

The GU-01 structure, drawn by RFC 069's rules:
- **one header line:** the repository, the screen's name;
- **the body;**
- **a status area** that carries the program's banner (rule 9) and its notices;
- **the key line,** drawn from the catalog.

Cards float on a scrim, opaque, and block the screen beneath for both input and pointer (knotra's RFC-034
lesson). The top layer is derived from `stikk_app::Overlay`; there's no parallel enum.

### 4. Keys

- **One function maps iced's key events to `stikk_app::Intent`,** through `stikk_core::actions` (rule 7). It's the
  prototype's 25 lines, grown to the catalog's full table.
- **On top of it, the rule 6 paths:**
  - Snora's `Shortcuts` and `Escape` routing, F6 and Tab;
  - **Escape from a text field closes its card, except during an IME composition;**
  - **`Named(Process)` is no action;**
  - **a letter never acts while a text field has focus;**
  - focus settles after every update, and returns where it was when a card closes.
- **Zoom keys** (Ctrl+Plus, Ctrl+Minus, Ctrl+0) are catalog entries scoped to the desktop app (rule 7).

### 5. The reading report (RFC 069 rule 8)

**Every card and every scrolling screen reports through one function,** which holds what the prototype's 14 lines
did plus rule 8's condition: **no `Reading` until the viewport is confirmed from the window.**
- **Confirmed** means the front end has the window's physical size and scale factor (`window::size`,
  `window::get_scale_factor`), and the viewport it laid out against agrees with them.
- Each report names the `Surface` it was measured for, as `stikk-app` requires.
- Sensors (iced's `sensor`) give `top shown` and `end visible`. The scrollable's own offsets give `more above`.

### 6. Theme, font and zoom

- **Theme (rule 3):**
  - at launch `system::theme()`, and while open `system::theme_changes()`, unless the user chose a preset;
  - the preset choice is remembered;
  - with no preference, `light_aaa`.
- **Font (rule 2):** the system's interface font, named.
  - **Linux:** the family fontconfig resolves for `sans-serif` (RFC 069 Handoff A: Noto Sans here; iced alone draws
    Open Sans).
  - **Windows:** Segoe UI.
  - **macOS:** the system font, unverified.
  - If the named family isn't installed, iced's own fallback draws, and the status area says nothing. It's a
    rendering matter, not the user's.
- **Zoom (rule 5):** 100, 115, 130 and 150 %, through iced's `scale_factor`, remembered.

### 7. Tests, and where each runs

Every RFC 069 rule's test, at the level it names:
- **H** (headless, every CI run, all three platforms);
- **R** (real, nested compositor, by the dev team, when the code a test covers changes);
- **C** (a CI job on a named runner).

The matrix: four presets × 100 % and 150 % × 1280×720 and 800×600, and the label-fit guard with **Liberation Sans and
DejaVu Sans installed by name** (RFC 069 rule 2).

**The CI job:**
- **Linux:** fmt, clippy, H tests, `cargo deny`, and one `fc-list` step that records the runner's inherited fonts.
- **macOS and Windows:** build and H tests.
- **The C tests** (rule 3's Windows theme, rule 8's Windows viewport, Segoe UI by name) run only after Handoff A shows
  the runner can open the window. If it can't, each C test is stated as open in the RFC's *Delivered*, with what
  was measured instead. **It's never quietly dropped.**

### 8. `deny.toml`

- **Licences:** BSD-2-Clause, BSD-3-Clause, ISC and CC0-1.0 join the allow list, each with a comment naming the crate
  that needs it.
- **Advisories:** each one in the GUI tree is either fixed by a version that exists, or recorded under RFC 068's rule:
  - a dated reason;
  - the path from `stikk-gui` to the crate;
  - why it doesn't reach the user, or what it would take;
  - what retires the entry.
- **The list is re-measured on this RFC's own lock,** not copied from the prototype's.
- **`unmaintained` isn't a vulnerability, and it isn't ignored silently either.**

### 9. What each handoff delivers

| Handoff | Delivers | Its tests |
|---|---|---|
| **A** | both crates; the shell; the key function; the reading report; **Orientation and History**; theme, font and the design crate's presets and overrides; `deny.toml`; the CI job; **first, whether the Windows and macOS runners can open the window** | rules 1, 2, 3, 4 (the guard and its failing run), 6 (focus return, Escape, `Process`), 8 (the four Handoff C cases and the unconfirmed-viewport refusal), 9 and 12 at H; one R keyboard pass and one R reading sequence on a long read-only card; C as Handoff A's first question allows |
| **B** | Block detail, Trust, Changes, Queue, Patch detail, Compare, Merge plan, Bundle, Verify; **zoom** | the H matrix over every new screen; rule 5's hit areas and zoom persistence |
| **C** | the action list, the palette (its filter is the first text field), help, the glossary, Refusal, Refusals, Stale, the ref picker, the read-only result cards | rule 6's letter rule on the palette's field; rule 7's sheet test; rule 11's labels; rule 13's template check; the R keyboard pass extended |

**The real-window reading sequence** (rule 8's R test) runs in A on a long read-only card (a Refusal or the
Glossary), because the commit confirmation is a writing card. It moves to the confirmation when the writing cards
come.

## What this RFC does not do

- **No writing card,** and no path in the desktop app that changes the repository.
- **No publishing, packaging or installer:** that's RFC 071.
- **No second language:** RFC 069 rule 13 keeps the way open.
- **No change to `stikk-app` or the terminal,** unless a handoff finds the program can't be drawn without one. Then it
  stops and asks, because the program is shared.

## Evidence plan

| Question | Level | Where |
|---|---|---|
| Every RFC 069 rule's test | H, R or C, as above | Handoffs A–C |
| Can GitHub's Windows and macOS runners open the app's window and report its viewport? | C, first in Handoff A | Handoff A |
| The GUI tree's licences and advisories on this RFC's lock | `cargo deny`, by script | Handoff A |
| The terminal is unchanged | its golden frames and the whole workspace suite | every handoff |

## Delivery

**Three handoffs, A, B and C, each reviewed before the next is issued.** Handoff A is issued when this RFC is
accepted.

## Changelog

**None in 0.23.0:** nothing a user installs changes. The contributing docs gain how to run the desktop app from
source.

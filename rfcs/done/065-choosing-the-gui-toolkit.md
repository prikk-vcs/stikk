# RFC 065 — Choosing the GUI toolkit: Snora first, measured

**Status.** **Done (decided) 2026-11-27: no trigger fired, and stikk's desktop app is built on Snora (path B)**; see *Outcome* below. **Accepted by the project owner 2026-11-24, with decision 6 answered: path B** (*"egui will be necessary only if a serious issue on iced and snora is found"*). See decision 6's ruling. **Proposed 2026-11-24** by the architect, as 0.22.0's increment 3 (`.git-exclude/release/0-22-0-scope.md`,
authorized by the project owner 2026-11-24, amended the same day: *"Snora prototype first."*). **No open question
before acceptance.** After the measurement, **the owner makes one decision** (decision 6).
**Measured** on `main` at `6e2a5f2`, where `stikk-app` exists (RFC 067), and from Snora's own repository at 0.65.0.
**Tracks.**
- RFC 001: the shortlist and the filter, *"(1) real platform-accessibility support (NFR-A02): this is the gate"*;
- RFC 067: the shared program a GUI draws;
- the toolkit survey (`.git-exclude/reports/065-gui-toolkit/toolkit-survey-2026-11-23.md`).

**Touches.** Nothing in the repository until the owner decides. The prototype is a measurement instrument, outside the
workspace and outside git, as RFC 059's probe was.

## Summary

**0.22.0's theme is the GUI toolkit.** The owner prefers iced's model, and its framework **Snora**, built on iced 0.14,
fits stikk on almost every count, measured from its source. It has none of what RFC 001 made the gate: **it
exposes nothing to a screen reader, and says so.**

This RFC builds **one Snora prototype**, drawn from the real `stikk-app`. It measures everything that can be measured
about it, the screen-reader absence included, so that the owner decides the gate on evidence. It does not decide the
gate itself.

## Findings

### F1 — what the gate protects

**RFC 001's filter puts screen readers first** (*"this is the gate, not a nice-to-have"*). Its acceptance accepted the
terminal front end's weak screen-reader support **because** *"the accessible path is the (future) GUI"*. Three design
statements repeat it:
- `NFR-A02`;
- `GU-08`: *"accessibility tree exposes names/roles/values for every control"*;
- `FE-03`: *"must expose platform accessibility APIs"*.

**So a GUI without screen readers changes more than the toolkit.** It leaves stikk, for now, with **no front end a
blind user can work with**.

### F2 — Snora, read from its source (0.65.0, 2026-11-17)

| Criterion | Snora |
|---|---|
| Licence, Rust | Apache-2.0, **1.88**: the same as stikk |
| Screen readers | **none.** *"No assistive-technology support: no accessibility tree, no AccessKit integration"* (its guide, *Accessibility*). Its stated position: *"snora will integrate an accessibility tree when iced exposes one"*, with no interim layer (RFC-045 in its repository) |
| Keyboard | every control it draws is a focus target, pressed by Enter and Space, with a focus ring of at least 3 : 1. Since 0.65 its own checkbox, toggle, radio, select and slider, because iced's cannot be reached. **The application routes Tab** |
| Visual | AA, AAA (7 : 1) and high-contrast presets, measured from drawn pixels by its own tests; 3 : 1 for non-text marks; logical, right-to-left-safe layout |
| Dependencies | 247 crates on Linux, 172 on Windows and 184 on macOS, resolved 2026-11-24. Outside stikk's `deny.toml`: BSD-2/3, ISC and CC0, all of which the owner's rule accepts (*"as long as it fits our license"*) |
| iced upstream | iced 0.14.0 (2025-12-07); **no 2026 release** (crates.io, 2026-11-24). Accessibility: issue #552 is open since 2020-10-05. PR #3281 was closed unmerged on 2026-03-14 by a maintainer: *"I'll work on this myself."* Draft PR #3111 (*"Accesskit integration"*, 2025-11-11) is still open. A code search of the repository finds no `accesskit`. **Intended upstream, with no date and no code** |

### F3 — what this machine can measure, and what it cannot

| Tool | Present | Used for |
|---|---|---|
| `iced_test` 0.14.0 | crate | headless runs with recorded state: every keyboard task, and the reading gate |
| `wtype` on niri (Wayland) | yes | the same keys, sent to the running prototype |
| fcitx5 with Mozc | yes | Japanese input on Wayland. **Whether `wtype`'s virtual keyboard reaches the input method is itself measured**, since it may not |
| AT-SPI (`gi.repository.Atspi`), Orca | yes | the accessibility tree, read directly, and Orca run once |
| mingw-w64 | yes | a Windows `.exe` for the owner's run |
| macOS | **no** | documented only, as before |

## Decisions

1. **One prototype, `snora-proto`,** at `.git-exclude/reports/065-gui-toolkit/snora/proto/`:
   - depends on **`stikk-app` by path**, `snora` 0.65 and `iced` 0.14;
   - on **Rust 1.88**, `--locked`, with its `Cargo.lock` kept in the report;
   - **written by the architect.** It is an instrument, and it is not shipped.
2. **It draws the real program, on a real repository.**
   - A script builds a scratch repository with prikk **0.58.0**, in an isolated HOME, as the measurement scripts do.
   - The prototype runs `stikk_app::App` with its worker, and turns iced's keys into `stikk_app::Intent`. **It does
     nothing the program has not decided.**
   - It shows four surfaces:
     - History, with the selected row;
     - Enter's action list;
     - **a confirmation card longer than the window.** iced's scrollable reports its viewport as
       `stikk_app::Reading`, and `Effect::Scroll` moves it;
     - **a commit message typed in Japanese.**
3. **What is measured.** Each measurement is a script or test whose output is the report, nothing added by hand:

   | # | Question | How |
   |---|---|---|
   | M1 | Does it build where stikk builds? | Linux on 1.88, `--locked`; the Windows `.exe` through mingw; the crate count, the clean build time and the stripped size beside stikk's |
   | M2 | Can every task be done from the keyboard? | `iced_test`: move in History, open the action list, open the confirmation, read it, confirm or go back, type the message. After each key, **the program's recorded state**. The same keys are then sent live with `wtype` |
   | M3 | Does the reading gate hold in a GUI? | `iced_test` and live: Enter on the long card does nothing until the end has been on screen, then confirms. A report from a closed card does not open the next |
   | M4 | Is the focus always visible, and the text readable? | one screenshot per preset (AAA dark, high contrast), the worst case, with the focus ring's and the text's contrast read from the pixels by a script |
   | M5 | Can the text be made bigger? | iced's scale factor at 1.0, 1.5 and 2.0 in a 1280 × 800 window: any clipping or overlap, recorded from the layout |
   | M6 | Does Japanese input work? | fcitx5 with Mozc on niri: the preedit is shown in the field, and the committed text reaches the program's message. If `wtype` cannot drive the input method, that is recorded, and the owner's Windows run covers it |
   | M7 | What does a screen reader get? | the AT-SPI tree under the prototype's window, dumped by script; then Orca, once, with its debug log. **Expected: nothing.** Measured, not assumed |
   | M8 | Can it be packaged as stikk is? | the binary's runtime libraries (`ldd`); whether the musl targets and the installers' static-binary assumption still hold; what an AUR package would need |
   | M9 | How much code does a front end need? | the prototype's lines that wire `stikk-app` (keys, reading, effects), as evidence on RFC 067's split |

4. **The owner's Windows run: about 20 minutes, with one page and a ready-built `.exe`,** as for RFC 059. It checks:
   - the keyboard tasks;
   - the two presets;
   - **Japanese input with the Microsoft IME**;
   - optionally, NVDA, to hear for yourself what it says, which is expected to be the window title alone.
5. **The report:** `.git-exclude/reports/065-gui-toolkit/snora/report.md`, produced by the scripts. It ends with the
   facts the decision needs, and no recommendation the evidence does not carry.
6. **Then the owner decides,** between two paths:
   - **A. Keep the gate.** Screen readers stay a requirement of the GUI. **An egui prototype is measured the same way
     next** (0.23.0), with Freya as the fallback. Snora stays the reference for keyboard and visual work.
   - **B. Amend the gate to *"screen readers when the toolkit exposes them"*,** and build stikk's GUI on Snora from
     0.23.0. This amends `NFR-A02`, `GU-08`, `FE-03` and RFC 001's acceptance in that RFC, and stikk's documentation
     says plainly that **neither front end serves a screen-reader user until iced does.** RFC 067 keeps the switch
     to another toolkit possible later, at the cost of one front end.

   **The owner's ruling, 2026-11-24: path B.** *"B. egui will be necessary only if a serious issue on iced and snora
   is found."* So the measurement no longer chooses between A and B. **It tests B for a serious issue.** "Serious"
   means one of the five triggers below, each tied to a measurement, so that the report cannot be read either way:

   | # | Trigger | Measured by |
   |---|---|---|
   | T1 | **A task cannot be done from the keyboard** | M2, and the owner's run |
   | T2 | **The reading gate cannot be held:** a confirmation can be executed unread, or can never be executed | M3 |
   | T3 | **Japanese input does not work on Windows or on Linux:** no preedit, or the committed text does not reach the program | M6, and the owner's run |
   | T4 | **A release target cannot be built or packaged:** the Windows, Linux x86_64 or aarch64 archives, the installers, or the AUR | M1, M8 |
   | T5 | **Focus or text cannot meet stikk's contrast** (7 : 1 text, 3 : 1 marks) with Snora's presets | M4 |

   - **Not a trigger:** the absence of screen-reader support, which path B accepts.
   - **A gap that Snora can close is a Snora issue, not a reason to leave.** Snora is the owner's project. For each
     gap, the report says whether it is Snora's to fix, iced's, or neither's, and the owner rules on whether it is
     serious.
   - **If no trigger fires,** 0.23.0's RFC creates `stikk-gui` on Snora, and amends `NFR-A02`, `GU-08`, `FE-03` and
     RFC 001's acceptance to *"screen readers when the toolkit exposes them"*. stikk's documentation then says plainly
     that neither front end serves a screen-reader user until iced does.
   - **If a trigger fires,** the owner decides between a fix in Snora and the egui prototype (path A's measurement).

   RFC 065 closes with the report and that outcome.

## Outcome (2026-11-27)

**Measured under the owner's GUI rules.** The dev team measured; the architect reviewed and reported
(`.git-exclude/reports/065-gui-toolkit/snora/report.md`). The owner ran M6 on Linux and W1 to W4 on Windows.

| # | Trigger | Fired? |
|---|---|---|
| T1 | A task cannot be done from the keyboard | **No.** Linux: M2, real and headless. Windows: W2 to W4 from the program's logs; W1 on the owner's word |
| T2 | The reading gate cannot be held | **No.** Linux: M3, real and headless. Windows: W2. One report at startup on Windows is unexplained (below) |
| T3 | Japanese input does not work | **No.** On both, the committed text reached the program, and the owner saw the preedit in the field |
| T4 | A release target cannot be built or packaged | **No toolkit trigger.** A static musl binary can't open a window, which holds for any winit toolkit, so the desktop app ships glibc builds. The Windows build is console-subsystem, a build setting. aarch64 is RFC 071's |
| T5 | Focus or text cannot meet stikk's contrast | **No.** Disabled controls fall below stikk's readability floor |

**The owner's rulings, 2026-11-27** (*"Yes"*, to the four the report put):
1. **Path B stands.** RFC 069 and RFC 070 build `stikk-gui` on Snora 0.65 and iced 0.14, and amend `NFR-A02`,
   `GU-08`, `FE-03` and RFC 001's acceptance to *"screen readers when the toolkit exposes them"*. M7 found that iced
   0.14 exposes nothing.
2. **Disabled controls:** stikk works around Snora's style in its own design crate. It's a tested rule in RFC 069, and
   a Snora issue until Snora changes it.
3. **T2's startup report on Windows** (a card reported its end visible on arrival, before the window's scaled size
   arrived) gets no fourth owner session. Instead:
   - **RFC 069's rule:** a front end reports a reading only against a viewport it has confirmed from the window
     itself;
   - **RFC 070's test:** a Windows CI test asserts that no reading arrives before the viewport matches the window.
4. **Reporting the two `iced_test` defects upstream** wasn't asked for. It stays open as the owner's call.

**Carried into RFC 069 and RFC 070:**
- every scrollable reserves its scrollbar's room;
- stikk ships its own fonts and sets the default font;
- 44 px targets, set explicitly;
- Escape leaves a text field and closes its card in one press;
- the desktop app draws every banner the program sets;
- `Named(Process)` keys during IME composition are no action;
- the right-to-left rules (isolation, start-edge alignment);
- the real-window reading-gate sequence becomes the desktop app's regression check, in the nested compositor, by the
  fixed-size method the knotra project shared.

## What this RFC does not do

- **No `stikk-gui` crate,** and no GUI dependency in the workspace. That is 0.23.0's RFC.
- **No egui or Freya prototype now** (the owner's direction). Path A adds one.
- **No change to `deny.toml`.** The prototype is outside the workspace. The licences return when a GUI crate joins
  (the owner's rule, 2026-11-24).

## Evidence plan

| Level | What |
|---|---|
| **Headless, recorded state** | `iced_test` for M2 and M3: the program's state after each key, not pixels |
| **Real, on Linux** | `wtype` on niri for M2, M3 and M6; AT-SPI and Orca for M7; `ldd` for M8 |
| **Pixels, minimal** | M4: one screenshot per preset, the worst case, read by a script; M5 from the layout |
| **The owner, on Windows** | decision 4's page |
| **Not measured** | macOS (documented only). The prototype's look beyond the two presets |

## Delivery

**No handoff.** The architect builds and measures, and the owner runs decision 4's page. The architect then brings
the report and decision 6 to the owner. Nothing enters the repository except this RFC, its amendment, and the index.

## Changelog

None. Nothing ships.

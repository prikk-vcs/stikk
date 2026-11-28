# Handoff A — RFC 070: the foundation, Orientation and History (v1)

**Companion to:** [RFC 070](../../accepted/070-stikk-gui-the-crate-and-its-reading-screens.md), building
[RFC 069](../../accepted/069-the-desktop-apps-design-rules.md)'s rules.
**Issued** 2026-11-28. **0.23.0 increment 2, the first of three handoffs.** B is issued after this one is reviewed.

> **What this is.** The desktop app's foundation:
> - two crates, the shell, the key function and the reading report;
> - the design crate's presets, overrides, roles, font and theme;
> - the first two screens, Orientation and History;
> - one card, the Glossary, drawn through the generic card frame (§4.4 says why);
> - `deny.toml`, and a CI job on Linux, macOS and Windows.
>
> **Every RFC 069 rule this handoff touches has its test, and each test that should fail today is shown failing
> first.**
>
> **Sanctioned:**
> - `crates/stikk-gui-design/`, `crates/stikk-gui/` (new);
> - `Cargo.toml` (workspace dependencies), `Cargo.lock`, `deny.toml`;
> - `.github/workflows/ci.yml`;
> - `crates/stikk-real-binary/tests/release_crates.rs`, if it needs to learn about unpublished crates;
> - `docs/src/contributing/` (how to run the app from source).
>
> **Not sanctioned:**
> - any change to `stikk-app`, `stikk-core`, `stikk-tui` or the release workflows. **If the program can't be drawn
>   without one, stop and ask.**
> - any writing card.

## 0. Read first

- **RFC 070, whole,** and **RFC 069, whole.** Each rule's *Test* line is the specification of what you build.
- **The prototype,** `.git-exclude/reports/065-gui-toolkit/snora/proto/src/main.rs`. Its key function (25 lines), its
  reading report (14 lines) and its effects loop are the measured starting point. Port what holds; don't port its
  measurement instruments (`STIKK_PROTO_*`).
- **RFC 065's report** (`.git-exclude/reports/065-gui-toolkit/snora/report.md`), for the facts behind the rules, and
  **RFC 069 Handoff A's review result**, for the pinned fonts and the theme.
- **The owner's GUI rules,** which govern §6's real-window checks, and the session tooling in
  `.git-exclude/reports/065-gui-toolkit/snora/`: `nested.sh`, `session.sh`, `assert-size.sh`, `account-keys.py`.

## 1. First: can GitHub's Windows and macOS runners open the window?

**The question** (RFC 070 F4, decision 7): can a runner open the desktop app's window, and report its physical size,
its scale factor and the viewport the app laid out against? RFC 069 rule 3's Windows test and rule 8's Windows test
both depend on the answer.

- **Build a probe,** `crates/stikk-gui/examples/window_probe.rs`:
  - it opens the real app on a scratch repository;
  - it opens the Glossary through the program's own intent, with no keys;
  - it logs each `window` event beside `window::size`, `window::get_scale_factor` and the viewport, until the
    viewport is confirmed (RFC 070 decision 5) or 30 s pass;
  - then it exits 0 or 1 accordingly.

  It's an example, so no probe code ships in the app.
- **Run it in the CI job on `windows-latest` and `macos-latest`** (and on Linux under the nested compositor, here).
- **Report what each runner did,** verbatim. If a runner can't open a window, say what failed, and leave that
  runner's C tests as **open**, with the reason, in your request. Don't drop them silently, and don't retry in a
  loop.

## 2. The crates

- **`stikk-gui-design`** and **`stikk-gui`,** both `publish = false`, edition and MSRV from the workspace (1.88).
- **Dependencies:** `snora = "=0.65.0"` with the `design` feature, and `iced = "=0.14.0"`, both pinned exactly as the
  prototype measured them. `stikk-gui` also depends on `stikk-app` and `stikk-gui-design`.
- **The two gates that list crates:**
  - `cargo package --workspace --exclude stikk-real-binary` must still pass. Exclude the two GUI crates by name if
    cargo needs it, and say which;
  - `release_crates.rs` must still name exactly the published crates. Show it rejecting a list that includes a GUI
    crate.
- **The release workflows are untouched:** they build `-p stikk`. Show that with `git diff --stat` on
  `.github/workflows/` (`ci.yml` only).

## 3. The design crate (RFC 069 rules 1–4, 10, 12)

| What | Rule | Its test (H unless said) |
|---|---|---|
| `body` 16, `meta` 14, `title` 18, `heading` 24; nothing below 12 | 2 | every text in the rendered tree of every screen, at 100 % and 150 %, has a role size. **Measured on the tree, not by scanning names** |
| Bold, never Semibold | 2 | the design crate exposes no Semibold weight (a name check, said as such) |
| the font: fontconfig's `sans-serif` on Linux, Segoe UI on Windows, the system font on macOS, named | 2 | the font database resolves the chosen name to that face; `Font::DEFAULT` resolves to Open Sans (the failing case, as RFC 069 Handoff A showed) |
| the four presets; **disabled overrides** to 4.5 : 1 label and 3 : 1 outline (7 : 1 and 4.5 : 1 in high contrast); `targets.min = 44`; `Scrollbar::spacing` | 3, 4, 5 | the contrast suite over every text and mark of Orientation, History and the Glossary, four presets × 100 % and 150 %, from the rendered pixels. **The disabled overrides removed must fail.** Hit areas ≥ 44 × 44 logical |
| start-edge alignment, done by the design crate | 4 | a view aligning text itself is a compile error, or a name check if it can't be one; say which |
| the isolating text function (U+2066 … U+2069) for ids, refs, paths, key ids and prikk's lines | 12 | every such value in the rendered text of History is wrapped in the isolates |
| lines, not panels; one card-and-scrim style; no shadow or animation token beyond that | 4, 10 | name checks, said as such |
| views import nothing from `snora::design` or `iced::Color` | 1 | a source check over `stikk-gui` |

## 4. The app

### 4.1 The shell (RFC 070 decision 3)

Header line, body, status area, key line.
- **The status area draws `App::banner()`** whenever it's set (RFC 069 rule 9).
- **Test:** for every banner `stikk-app`'s own tests produce, the desktop tree shows its text. Run it with the status
  area's banner drawing removed, and it must fail.

### 4.2 Keys (RFC 069 rules 6 and 7)

- **One key function** from iced's events to `stikk_app::Intent`, through `stikk_core::actions`. It's the prototype's
  25 lines, grown to the catalog.
- **`Named(Process)` is no action.** Test: a `Process` key changes no state.
- **Snora's `Shortcuts` and `Escape` routing, F6 and Tab.** No key model of stikk's own beside them.
- **Focus settles after every update.** **Test (H):** when the Glossary closes, focus is where it was before it
  opened, from Orientation and from History. Removing the settle step must fail.
- **Every catalog action Orientation and History offer is reachable by its key** (H).
- **Not in A:** the Escape-from-a-field case (rule 6) needs a text field, and the first is the palette's filter, in
  Handoff C.

### 4.3 The reading report (RFC 069 rule 8; RFC 070 decision 5)

**One function, with no `Reading` until the viewport is confirmed from the window.**
- **(H) Handoff C's four cases, on the desktop's report:**
  - a Loading card replaced by its next card starts unread;
  - a surface never drawn reports nothing;
  - a card replaced below the top keeps the top's reading;
  - the two latches agree through a swap.
  
  `stikk-app`'s test fixtures can open the cards these need.
- **(H) Refused until confirmed:** a reading offered before the viewport is confirmed is refused. The prototype's
  report, which reports at once, must fail this. **That's the run that fails it.**

### 4.4 The screens, and one card

- **Orientation and History,** drawn from the program's views, to RFC 069's rules.
- **The generic card frame,** on its scrim, and **the Glossary** drawn through it.
  - **Why the Glossary is here and not in C:** rule 8's real-window reading sequence needs a card longer than the
    window, and the commit confirmation is a writing card.
  - C completes help and the glossary's navigation; A draws its text and scrolls it.
- **Theme (rule 3):**
  - `system::theme()` at launch, and `theme_changes()` while open, unless a preset was chosen;
  - with no preference, `light_aaa`;
  - the choice is remembered through `stikk-state`.
  - **(H, logic level):** a theme-change message switches the preset when none is chosen, and leaves a chosen one
    alone. Overriding a chosen preset must fail.

## 5. `deny.toml` (RFC 070 decision 8)

- **Re-measure on this handoff's own lock,** with `cargo deny check`, and record its output verbatim.
- **Licences:** add only what the lock needs, each with a comment naming the crate. RFC 070 F3 expects BSD-2-Clause,
  BSD-3-Clause, ISC and CC0-1.0. **A licence outside those four, or one that isn't permissive, stops this step:** ask,
  because it's the owner's to rule.
- **Advisories:** fix by a version that exists where you can. Record each one you can't, under RFC 068's rule:
  - a dated reason;
  - the path from `stikk-gui`;
  - whether and how it reaches the user;
  - what retires it.

  `unused-ignored-advisory = "deny"` stays.

## 6. Real-window checks (R), in the nested compositor

By knotra's method, at an asserted 1280×720, with the session tooling (focus by pid, one instance, keys accounted
for, captures sized). Adapt `session.sh` to launch `stikk-gui` instead of the prototype. **Don't overwrite the
prototype's evidence.**
1. **One keyboard pass:** Orientation, then History, then ↓↑, then the Glossary, then back. State asserted after
   every key from the app's own log.
2. **One reading sequence on the Glossary at 200 %,** where it overflows:
   - it arrives `seen false`;
   - scrolling brings the end into view;
   - `seen true` only after that.

   Read the captures, and say what's on them.

**The app's state log for these checks** is enabled by an environment variable read by `stikk-gui` (as the
prototype's `STIKK_PROTO_LOG` was). Name it, document it in the contributing docs, and make it write nothing when
unset.

## 7. The CI job

- **Linux:**
  - fmt, clippy (`-D warnings`), the H tests, `cargo deny`;
  - **`apt-get install fonts-liberation fonts-dejavu-core`**, with each package's installed version printed;
  - the label-fit guard with both fonts by name;
  - one `fc-list` step recording what the runner inherits.
- **macOS and Windows:** build, the H tests, and the probe from §1.
  - **The label-fit guard runs on Linux only,** where its two fonts are pinned. Say so in the job.
- **The C tests** only where §1 showed the runner can open a window:
  - rule 3's theme on Windows (`AppsUseLightTheme` 0, then 1);
  - rule 8's viewport (no `seen` before the viewport is confirmed, on the Glossary at 250 %);
  - Segoe UI by name.

## 8. Gates

**All of them,** including the rustdoc gate (never exempted) and `cargo test -p stikk-real-binary --test rfc_index`.
**The terminal's golden frames are unchanged:** show the suite passing. **CI is green on all three runners** before
you submit.

## 9. Submit

`.git-exclude/review-request/070-stikk-gui-the-crate-and-its-reading-screens-a/review-request-v1.md`:
- §1's answer first, verbatim, per runner;
- then each test with its failing run beside its passing one;
- `cargo deny`'s output;
- the R checks' records, and what the captures show;
- what wasn't done, and why.

**Commits pushed and CI green before you submit.** If a push is refused by your session's approval, say so in the
request, and don't commit on top of it.

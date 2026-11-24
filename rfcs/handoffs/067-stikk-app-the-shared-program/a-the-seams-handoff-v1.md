# Handoff A — RFC 067: the seams, inside `stikk-tui` (v1)

**Companion to:** [RFC 067](../../done/067-stikk-app-the-shared-program.md), **accepted 2026-11-24**. **Read the
RFC first**: F1–F3 and decisions 3, 4, 5 and 8. **Covers:** decisions 3, 4, 5 and 8. Decisions 1, 2, 6 and 7 are
Handoff B. **0.22.0's increment 2, first half.** **Not a release.**

> **What this increment is.** Nothing moves between crates yet. Inside `stikk-tui`, the three places where the
> program and the terminal touch are cut, so that Handoff B can move the program by `git mv` and import fixes alone:
> - **keys:** the program takes an `Intent`, not a crossterm `KeyEvent`;
> - **colours:** the program holds no palette;
> - **reading position:** the program decides and keeps the evidence, and the drawing holds the offsets.
>
> **A user sees no change, and the golden frames prove it.** They are committed first, and every later commit leaves
> them byte for byte as they were. **No branch.**
>
> **Sanctioned:** `crates/stikk-tui/**`, and text. **`stikk-core` does not change** (RFC 067: *"No change to
> `stikk-core`"*). If a seam seems to need it, stop and report.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does anything a user sees change? | headless | **the golden manifest's diff is empty after commit 1**, at every later commit (show `git diff <c1>..HEAD -- <manifest>`: nothing) |
| Do the golden frames cover every view and overlay? | compile time | the exhaustive scene map (§1.3): a new `Overlay` or `Focus` variant does not compile until it has a scene |
| Are the keys still wired? | `handle_key` | the existing key-press tests, unchanged in what they press |
| Is a confirmation still closed until its card is seen? | unit | the RFC 060 gate tests, plus §4.6's three new tests |
| Does `↓` still read to the end first? | `handle_key` | the existing reading-path tests, unchanged in what they assert |
| Does it work against prikk? | real binaries | the local suite at 0.28.0 + 0.58.0 |
| Is the count kept? | `cargo test` | the test count before and after each commit: it may grow and must not fall |

## 1. Commit 1: the golden frames, before anything else

**One test module, `crates/stikk-tui/src/golden.rs`** (`#[cfg(test)]`), with a committed manifest,
`crates/stikk-tui/src/golden/frames.txt`.

1. **A scene** drives the real `App` and the real `shell::render` into a `TestBackend` buffer, as `readme_visuals.rs`
   does. It uses invented data and the same `NullBackend` scripting, and nothing from the machine.
2. **The matrix:**
   - **every scene at 60×15, 80×24, 120×40 and 200×60,** with the `dark` palette;
   - **every scene at 80×24** in every other palette: `light`, `high_contrast(false)`, `high_contrast(true)` and
     `plain`;
   - **every scene that scrolls, a second time after `↓` has been pressed** through `handle_key` until the text is at
     its end. That is the path `Seen` depends on.
3. **Coverage is checked by the compiler.** Write one function from each `Overlay` variant, and one from each `Focus`
   variant, to the scene that shows it, **with no `_` arm**. A variant added later then cannot compile without a scene.
   Each scene must reach the variant it claims; assert it with `app.top_overlay()` or `app.focus()`.
4. **The serialization covers what a user sees:** every cell's symbol **and** its style (foreground, background,
   modifiers), row by row. `buffer_text` alone drops the style, and the palettes differ only in style.
5. **The manifest holds one line per frame:** `<scene> <size> <palette> <state> <digest>`.
   - The digest is **64-bit FNV-1a, written out in the test** (about ten lines). Add **no dependency**: `DefaultHasher`
     is not promised to be stable across Rust releases, and a hash crate is more than a regression check needs.
   - **On a mismatch,** the test writes the frame's full text to `target/golden-frames/<scene>-<size>-<palette>-<state>.txt`
     and fails, naming every frame that differs.
   - **A mode that rewrites the manifest** (an env var, `STIKK_GOLDEN_WRITE=1`) is used **once, in commit 1**, and
     never again in this handoff.
6. **Commit 1 changes no other file.** In the request, report:
   - the manifest's line count;
   - the time the module takes;
   - one frame's full text, the History scene at 80×24 `dark`, so the reviewer can see what is being compared.

## 2. Commit 2: `Intent`, and `App::handle`

1. **`keys::Action` becomes `keys::Intent`.** The variants are unchanged: `None`, `Quit`, `BackWithoutQuit`, `Up`,
   `Down`, `Run(&'static CatalogAction)`, `Input(char)` and `Backspace`. `keys::dispatch(KeyEvent, bool) -> Intent`
   stays where it is.
2. **`App::handle(&mut self, intent: Intent) -> Effect`** is today's `handle_key` body after the `dispatch` call. In
   this commit `Effect` has one variant, `None`. Commit 4 adds `Scroll`, and introducing the return type now keeps
   commit 4's diff about scrolling alone.
3. **`handle_key` becomes a free function** in `stikk-tui`:

   `pub fn handle_key(app: &mut App, key: KeyEvent) -> Effect`

   It runs `dispatch(key, app.wants_text_input())`, then `app.handle(..)`. The run loop and **every test that pressed
   a key** call it. Do not change what a test presses. **`App` has no method that takes a `KeyEvent` any more.**
4. **App's own module must not name crossterm or ratatui after this commit,** apart from what commits 3 and 4 remove.
   Say in the request which references remain, and that the later commits remove them.

## 3. Commit 3: the colours leave `App`

1. **`App` loses `resolved: Resolved`.** `App::new` and its test constructors no longer take a `Resolved` or a
   `Palette`.
2. **`palette()` and `colours_row()` leave `App`.**
   - The drawing gets the palette as a parameter: `shell::render(app, &resolved, frame)`, or a small `Look` struct, as
     you judge. Say which.
   - Orientation's colours row is composed by the drawing, from the same `Resolved::colours_row()`.
3. **The run loop owns the `Resolved`** it detected at startup. Nothing about detection changes (RFC 059 decision 5).
4. **`readme_visuals.rs` and the golden module** pass the palette the same way. The README images must not change:
   `readme_images_are_current` stays green, with no image rewritten.

## 4. Commit 4: the reader's place

**This is the commit with the risk in it. Read RFC 067 decision 4 again, and F2.3.**

1. **`Surface`, an opaque token** (`Copy`, `Eq`, with no public constructor) for the current overlay and the current
   scrolling view.
   - App hands it out through `focus()` and `top_overlay()`, or one accessor, as you judge.
   - **It changes whenever the surface does,** generalizing `overlay_generation`. A new overlay, a popped overlay, or
     a view replaced by a reload each get a new one.
2. **`Reading { surface, more_above: bool, at_end: bool }`** and **`App::report_reading(&mut self, Reading)`.**
   - A report whose `surface` is not current is **ignored**.
   - A report for the current overlay with `at_end: true` records that the card was **seen**. Seen never goes back to
     false for that surface.
3. **`Effect::Scroll { surface, direction: Up | Down }`.** `handle(Intent::Down)` and `handle(Intent::Up)` decide as
   `nav_down` and `nav_up` decide today, reading the **last report** where they read `overlay_scroll()` today.
   - When the reading path says *"scroll"*, they return the effect and change nothing else.
   - **The scrolling views** (Block detail, Trust, Changes, Queue, Patch, Merge plan, Bundle, Verify) return the same
     effect where they set their `Cell<u16>` today.
4. **No report yet must behave as today.** Today, a card that has never been drawn has `max == 0`, so `at_end()` is
   true and `↓` moves the cursor, while `seen` is false. Keep both: **no report means "at the end" for the scroll
   decision, and "not seen" for the evidence.** The handler tests that never draw depend on it. Write a test that pins
   it.
5. **The offsets move to the drawing.**
   - `overlay::Scroll` and App's ten `Cell<u16>` fields leave `App`, and `Focus` stops carrying `&Cell<u16>`.
   - `stikk-tui` keeps a **`Viewports`** (the name is your call), keyed by `Surface`. It holds offset and max, applies
     `Effect::Scroll`, clamps where `fit` clamps today, and produces the `Reading` after each draw.
   - **The run loop's order:** draw (with `&App` and `&mut Viewports`), then `app.report_reading(..)` for each surface
     drawn, then read the next key. Today's keys likewise act on the last draw's answer, so the order is unchanged.
   - **App holds no `Cell` after this commit.** Say so, with the `grep`.
6. **Three new tests, in App's tests, through `handle` and `report_reading` with no drawing:**
   1. a confirmation card with no report cannot be confirmed;
   2. an `at_end` report for a **previous** surface does not open the current card;
   3. an `at_end` report for the current surface opens it, and a later `more_above` report does not close it again.

## 5. The traps

1. **The golden manifest is written once.** If a frame changes after commit 1, the change is the defect. Do not
   rewrite the manifest; find the cause.
2. **`Seen` is made in exactly one place,** from a report for the current surface. `grep` for `Seen::WholeCard`, and
   show every hit.
3. **No handler is renamed** and no catalog action changes. RFC 067: only `keys::Action` → `Intent`.
4. **Key-press tests keep pressing keys.** Do not turn a `handle_key` test into a handler call.
5. **Count the tests** at each commit, and give the counts in the request.

## 6. Submit (no branch)

**Run:**
- gates 1–3;
- the golden module at every commit: `git rebase -x` or a loop, shown;
- the local real-binary suite at 0.28.0 + 0.58.0;
- `cargo public-api -p stikk-tui diff 0421f7f..HEAD`, for the prep's `### Breaking` table.

**Package it to** `.git-exclude/review-request/067-stikk-app-the-shared-program-a/review-request-v1.md`, with:
- each commit's diff summary;
- §4.5's `grep`;
- §5.2's `grep`;
- the test counts.

**After approval:** push, then report CI, Docs, supply chain, the full-matrix suite and `release-trial.yml` at the
pushed SHA.

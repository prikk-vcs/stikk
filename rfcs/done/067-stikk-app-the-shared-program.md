# RFC 067 — `stikk-app`: the program, shared by every front end

**Status.** **Done (implemented) 2026-11-24** at `3c6238b`, shipping in 0.22.0; see *Delivered* below. **Accepted by the project owner 2026-11-24**; [Handoff A](../handoffs/067-stikk-app-the-shared-program/a-the-seams-handoff-v1.md), the seams, **delivered 2026-11-24 at `43e2408`** (golden manifest unchanged, 1622 tests, full matrix green); [Handoff B](../handoffs/067-stikk-app-the-shared-program/b-the-move-handoff-v1.md), the move, issued 2026-11-24. **Proposed 2026-11-24** by the architect, as 0.22.0's increment 2 (`.git-exclude/release/0-22-0-scope.md`,
authorized by the project owner 2026-11-24). **No open question.** The owner takes **one action at the 0.22.0 cut**
(decision 7).
**Measured** on `main` at `0421f7f`, from `crates/stikk-tui/src`.
**Tracks.**
- RFC 001: the GUI direction;
- RFC 060: the shared `Scroll`, and the confirmation that waits until the card has been seen;
- RFC 061: the action catalog, and `App::handle_key`;
- RFC 065: the toolkit choice, which this RFC keeps reversible.

**Touches.**
- a new crate, `crates/stikk-app`;
- `crates/stikk-tui`;
- the workspace `Cargo.toml` and `deny.toml`;
- `release.yml`'s two crate lists;
- `docs/src/contributing/`.

## Summary

**Everything stikk *does* lives today inside the terminal front end.** `stikk-tui` holds:
- the program: what each key means, what each card offers, when a confirmation may be accepted, and the worker that
  talks to prikk;
- the terminal drawing.

A GUI built beside it would either depend on a terminal crate or copy the program. **A copy is two programs that
must agree,** which is the opposite of the owner's *"finally clean"*.

This RFC moves the program into a new crate, **`stikk-app`**, which knows no terminal. The terminal front end becomes
what a GUI will be: a drawer of `stikk-app`'s state and a translator of its own input events into `stikk-app`'s
intents.

**A user sees no change.** The proof is that every frame stikk draws is byte-identical before and after (decision 8).

## Findings

### F1 — the program is already almost free of the terminal

| File | Lines | References to the terminal crate | What it is |
|---|---|---|---|
| `app.rs` | 6,216 | **1**, the `KeyEvent` in `handle_key` | the program's state and every handler |
| `worker.rs` | 793 | **0** | the thread that runs prikk |
| `keys.rs` | 106 | its `KeyEvent` mapping only | the intents (`keys::Action`), and the mapping from crossterm keys to them |
| `overlay.rs` | 4,613 | 503, all of them in drawing | lines 38–537 are state, the `Overlay` enum and its kinds; the rest is drawing |
| `theme.rs`, `text.rs`, `title.rs`, `shell.rs`, `status_bar.rs`, `key_line.rs`, `view/*` | — | all drawing | stays |

- **App's fields are all private** (39 fields, none `pub`).
- **The drawing reads App through methods,** about 18 of them, for example `focus()`, `top_overlay()`, `keys_here()`
  and `in_flight_count()`. Each view receives `stikk-core` view types, a cursor and an offset, not the `App`.

**So the program can move without a rewrite.**

### F2 — three seams are still crossed, and each one belongs to the drawing

1. **The key event.** `App::handle_key` takes a crossterm `KeyEvent` and hands it to `keys::dispatch`. That function
   already turns it into a terminal-free intent (`keys::Action`: `Up`, `Down`, `Run(&CatalogAction)`, `Input(char)`
   and so on). Only the event type and the mapping are terminal-specific.
2. **The colours.** `App` holds the theme resolution (`Resolved`), and it uses it only twice:
   - `palette()`, which the drawing calls;
   - `colours_row()`, the Orientation row that describes the colours.

   Both are about drawing. `Resolved` holds terminal colours and the terminal's colour depth.
3. **Where the reader is in a text.** Scroll offsets are terminal rows held in `App`:
   - the overlays' shared `Scroll`: `Cell`s for offset, max and seen, written back by the drawing (`overlay.rs:3685`);
   - eight `Cell<u16>` offsets that `Focus` hands out, for Block detail, Trust, Changes, Queue, Patch, Merge plan,
     Bundle and Verify. App holds them in ten `Cell<u16>` fields.

   `nav_down` asks the drawing's answer, `at_end`, to decide whether `↓` scrolls the card or moves into its actions.
   **`Seen::WholeCard`, the evidence a confirmation needs, is made from that drawing-side state** (`app.rs:6107`).

   A GUI scrolls in pixels, through its toolkit, and has no rows to report.

### F3 — what a second front end would need, measured against the third seam

**The reading path has to stay the same in every front end** (RFC 060 decision 3): `↓` reads the card to its end
and only then moves into the actions, and a card is confirmable only once it has been seen whole.

**If the reading path lived in the drawing,** each front end would implement it again, and the safety rule would
be implemented twice. **If the scroll arithmetic lived in the program,** the program would need to know rows.
Neither is acceptable.

**The split that satisfies both:**
- **the program owns the decision and the evidence:** whether `↓` scrolls or moves, and whether this card was seen;
- **the front end owns the position:** offsets, maxima, viewport arithmetic.

**They speak through two small values,** one in each direction (decision 4).

### F4 — publishing a new crate

- **The crate name is free:** crates.io returns 404 for `stikk-app` (2026-11-24).
- **crates.io attaches a trusted publisher only to a crate that exists,** so a new crate's first publish is done by
  hand, once (`docs/src/contributing/releasing.md:33`).
- **`release.yml` names the six crates in dependency order, in two places:** the plan step and the publish step.

## Decisions

1. **A new crate, `crates/stikk-app`,** at the workspace version, licence and Rust 1.88. Its dependencies are
   `stikk-model`, `stikk-prikk`, `stikk-state` and `stikk-core`, **and nothing that draws.** The dependency order
   becomes:

   **model → prikk / state → core → app → tui → launcher.**
2. **What moves into `stikk-app`:**
   - `App`, `Focus`, `Operation`, and every type they hold;
   - the worker (`worker.rs`);
   - the `Overlay` enum with `SavePathKind`, `RefNameKind`, `RefWriteKind` and their `impl`s (`overlay.rs:38–537`);
   - the intents, `keys::Action`, renamed **`Intent`** to say what it is, with its variants unchanged.

   **What stays in `stikk-tui`:**
   - every drawing function and type (`Body`, `Panel`, `ListPanel`, the views, the shell, the status bar, the key
     line, the title);
   - the theme and its resolution;
   - text wrapping;
   - the terminal setup and the run loop;
   - **`keys::dispatch`**, from a crossterm `KeyEvent` to an `Intent`.
3. **Input: `App::handle(Intent)`.** The one function from an intent to the handlers, the body of today's
   `handle_key`.
   - `stikk-tui` keeps **`handle_key(&mut App, KeyEvent)`** as a free function: `dispatch`, then `handle`.
   - **RFC 060 Handoff B's lesson holds:** the tests that press keys keep pressing them through `stikk-tui`'s
     `handle_key`. *A test that calls a handler proves the handler; only a test that presses a key proves the wiring.*
4. **The reader's place: the program decides, the front end measures.**
   - **The front end reports, after it lays a text out:**

     `app.report_reading(Reading { surface, more_above, at_end })`

     - `surface` is an opaque token App hands out with each overlay and each scrolling view (today's
       `overlay_generation`, generalized);
     - **a report for any surface but the current one is ignored**, so a late report cannot mark a newer card seen.
   - **The program answers each intent that might scroll with an effect:** `handle` returns `Effect::Scroll(Up | Down)`
     when the reading path says *"scroll this text"*, and `Effect::None` otherwise. The front end performs it.
   - **`Seen::WholeCard` is made only inside `stikk-app`,** from an `at_end: true` report for the current overlay's
     surface. The confirmation gate (RFC 060, enforced in `stikk-core`) is unchanged. **A front end that never
     reports leaves every confirmation closed, which is the safe failure.**
   - **The offsets move to `stikk-tui`:** the overlays' `Scroll`, and App's ten `Cell<u16>` fields, which `Focus` no
     longer carries. `stikk-tui` keeps them keyed by surface.
5. **The colours leave `App`.** `App::new` no longer takes a `Resolved`. `stikk-tui`'s drawing holds the palette and
   composes Orientation's colours row itself, from the same `Resolved::colours_row()`.
6. **`stikk-app` cannot grow a terminal dependency.** `deny.toml`'s `[bans]` gains:

   ```toml
   deny = [
     { crate = "ratatui", wrappers = ["stikk-tui"] },
     { crate = "terminal-colorsaurus", wrappers = ["stikk-tui"] },
     { crate = "crossterm", wrappers = ["ratatui-crossterm"] },
   ]
   ```

   - **ratatui and the colour query are allowed only under `stikk-tui`,** and crossterm only through ratatui.
   - The rule is checked by the supply-chain workflow on every push, not remembered.
   - **Measured 2026-11-24 with cargo-deny 0.20.2,** in a scratch worktree of `0421f7f`. Today's tree passes
     (`bans ok`). A `ratatui` planted under `stikk-core` is refused (*"crate 'ratatui = 0.30.0' is explicitly
     banned"*), and so is a direct `crossterm = "0.29"`.
7. **Publishing.**
   - `release.yml`'s two crate lists gain `stikk-app` between `stikk-core` and `stikk-tui`.
   - **The owner, at the 0.22.0 cut, before the tag:** publishes `stikk-app` 0.22.0 by hand, once, and attaches the
     trusted publisher, by `releasing.md`'s documented first-publish procedure. The architect writes the exact steps
     into the release-prep handoff.
   - The tag's crates job then finds `stikk-app` present and publishes the rest.
8. **No user-visible change, proved by the frames.**
   - **Before anything moves,** a golden-frame test records every view and every overlay at four sizes: 60×15, 80×24,
     120×40 and 200×60. It records each in the three themes, and each after a scripted scroll to the end, as
     `TestBackend` buffers in text. It is committed first.
   - **It must pass unchanged, byte for byte, at every later commit.**
   - The test suite's count may grow and must not fall: a moved test is moved, not dropped.

## What this RFC does not do

- **No GUI code, and no GUI dependency.** RFC 065's prototype lives outside the workspace.
- **No rename of a handler, a field or a catalog action**, beyond `keys::Action` → `Intent`.
- **No change to `stikk-core`.** `Seen` and `Evidence` stay where they are.
- **`stikk-app`'s public API is not frozen.** It is pre-1.0 under RFC 011, like every crate. The GUI's needs will
  shape it in 0.23.0.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does anything a user sees change? | headless, every view × 4 sizes × 3 themes, plus scrolled | the golden frames (decision 8), committed before the move and **unchanged after** |
| Are the keys still wired? | stikk-tui, through `handle_key` | the existing key-press tests, unchanged in what they press |
| Is the confirmation still closed until the card is seen? | stikk-app | the existing RFC 060 gate tests, **plus:** a report for a stale surface does not open it, and no report keeps it closed |
| Does `↓` still read to the end first? | stikk-tui, through `handle_key` | the existing reading-path tests, unchanged |
| Is `stikk-app` free of the terminal? | supply chain | `cargo deny check bans` with the wrappers. A `ratatui` and a `crossterm` planted in `stikk-app`, run once locally, are both refused: shown in the review request, then reverted |
| Does stikk still work with prikk? | real binaries | the full suite at 0.28.0 + 0.58.0, on the full matrix |
| Is the release build still whole? | CI | `release-trial.yml` (RFC 066), on the push |

## Delivery

**Two handoffs, in order**, so each diff can be reviewed on its own:
- **A: the seams, inside `stikk-tui`.**
  - the golden frames first;
  - then `Intent` and `App::handle`;
  - the colours out of `App`;
  - `Reading` and `Effect`, with the offsets moved to the drawing.

  **No file moves crates.** Each commit leaves the frames unchanged.
- **B: the move.**
  - `stikk-app` is created, and `App`, the worker, `Overlay` and `Intent` move into it with their tests;
  - the `deny.toml` bans;
  - `release.yml`'s lists;
  - the contributor docs.

  The frames stay unchanged.

## Changelog

`### Changed` (for contributors): *stikk's program, everything but the terminal drawing, lives in a new crate,
`stikk-app`, so that the terminal front end and a future GUI share one program. Nothing a user sees changes.* The
`### Breaking` rows are taken from `cargo public-api diff` at the prep: `stikk-tui` loses `App`, `Focus`, `Overlay`
and `keys::Action`.

## Delivered

**On `main` at `3c6238b`, 2026-11-24.** Two handoffs, each reviewed by the architect and approved with fixes.

**Handoff A, the seams, at `43e2408`:**
- the golden frames: 460 frames, 46 scenes, with coverage checked by the compiler. Committed first, and unchanged
  ever since;
- `Intent` and `App::handle`;
- the colours out of `App`;
- `Surface`, `Reading` and `Effect`, with the offsets in the drawing's `Viewports`. `Viewports` keeps only live
  surfaces;
- a test that the drawing's latch and the program's agree.

**Handoff B, the move, at `3c6238b`:**
- the new crate `stikk-app` holds `App`, the worker, `Overlay`, `Session`, `Intent`, `Effect`, `Surface` and
  `Reading`, at its root, with private modules;
- `stikk-tui` keeps the drawing, `keys::dispatch` and `handle_key`, and re-exports nothing from `stikk-app`;
- `deny.toml` bans ratatui, terminal-colorsaurus and crossterm outside the terminal front end;
- three guards scan the published crates, derived from the workspace;
- `release_crates` holds `release.yml`'s two lists to the workspace, in dependency order;
- **`push_overlay` is crate-private.** Its test twin is behind `test-fixtures`, and `test_fixtures_feature` keeps that
  feature out of every `[dependencies]` entry and `default` list.

**Evidence, checked by the architect:**
- the golden manifest is unchanged from `281be9a` to `3c6238b`;
- tests 1611 → 1626, never falling, and `stikk-tui`'s 639 became 198 + 441;
- at `3c6238b`, all green: CI, Docs, supply chain (with the new bans), the full-matrix real-binary suite, and the
  release trial in all nine jobs;
- `cargo tree -p stikk -e features` shows `test-fixtures` off in the launcher's build.

**For the 0.22.0 prep:**
- the `cargo public-api` tables from both handoffs go into `### Breaking`. Every item removed from `stikk-tui` still
  exists under `stikk_app::`, and `Focus` is now `Send` and `Sync`;
- **the owner publishes `stikk-app` by hand, once** (decision 7), with the steps in the prep handoff.

# Handoff C — RFC 067: the reading gate holds again (v1)

**Companion to:** [RFC 067](../../done/067-stikk-app-the-shared-program.md), decision 4, and RFC 060 decision 3.
**A regression fix for 0.22.0, released as 0.22.1.** Issued 2026-11-25. **Not a release:** the release prep is a
separate handoff, after this one is reviewed.

> **What is wrong.** In 0.22.0, a confirmation card that overflows the window **is confirmed by one `Enter`, unread.**
> 0.21.1 refuses the same `Enter`. Measured on the real binaries (`.git-exclude/reports/reading-gate-2026-11-25/`):
>
> ```
> binary: stikk 0.22.0
> 60x15: before the Enter: Confirm card drawn says read-to-the-end; after one Enter: queued patches 1
> 60x14: before the Enter: Confirm card NOT drawn; after one Enter: queued patches 1
> 160x50: before the Enter: Confirm card drawn; after one Enter: queued patches 1
> binary: stikk 0.21.1
> 60x15: before the Enter: Confirm card drawn says read-to-the-end; after one Enter: queued patches 0
> 60x14: before the Enter: Confirm card NOT drawn; after one Enter: queued patches 0
> 160x50: before the Enter: Confirm card drawn; after one Enter: queued patches 1
> ```
>
> At 60×15 the card said *"read to the end to confirm"*, and the `Enter` committed anyway. At 60×14 the card was never
> drawn (the too-small notice was), and the `Enter` committed a card nobody saw.
>
> **Sanctioned:** `crates/stikk-app/src/app.rs` (and a new private module beside it, if you take §1's type),
> `crates/stikk-tui/src/{viewports.rs,overlay.rs,lib.rs,golden.rs}`, their tests, and `CHANGELOG.md`. **No public API
> change** in any crate: `cargo public-api diff` against 0.22.0 must be empty for `stikk-app`. **Anything else, stop
> and report.** **No branch.**
>
> **RFC 072 Handoff A pauses.** Keep any of its commits local; push nothing of it until 0.22.1 is tagged.

---

## 0. The two causes

**F1. A card that takes another card's place keeps the other card's surface** (`stikk-app`).
A preview pushes `Overlay::Loading` (one `push_overlay`, one new surface). When the answer arrives, the Loading card is
replaced **in place**, `*slot = Overlay::Confirmation { … }` (`app.rs:5370`, and 25 more `*slot = Overlay::…` sites),
**without minting a surface.** The Loading card is short, so its first draw reports `at_end: true` for that surface and
the latch sets. The Confirmation is the same surface, so it is born *seen*. `push_overlay`'s own doc says it is
*"the only way an overlay reaches the stack, so the reading latch cannot be inherited by a card that was never read"*,
and the in-place swap is a second way.

The 53 `self.overlays.remove(index)` sites each mint by hand on the line before. That is the same rule kept by
discipline, 53 times. It held at every remove and was never applied to a replace.

**F2. The TUI reports *"at the end"* for a surface it never laid out** (`stikk-tui`).
`Viewports::reading` (`viewports.rs:70`) answers `at_end: scroll.is_none_or(Scroll::at_end)`, and `ui_loop` reports
it after every draw (`lib.rs:162`). With the too-small notice up, `shell::render` returns before any card is drawn
(`shell.rs:90`), and the report still says the end is on screen. The program latches it. RFC 067 decision 4 says
the opposite: *"a front end that never reports leaves every confirmation closed, which is the safe failure."* The
translation in 0.22.0 merged two answers that 0.21.1 kept apart:
- **may `↓` move into the actions?** For an undrawn card the answer is *"yes, at the end"*. `App::at_end`'s
  *"no report means at the end"* keeps that answer, and it stays.
- **has the reader seen the end?** For an undrawn card the answer is *"no"*, and only a draw can make it yes.

`Viewports::apply` also creates an entry for a surface never drawn (`viewports.rs:35`), so **"has an entry"** can't
stand for **"was drawn"**.

**Why the 0.22.0 tests did not see it.** RFC 067's evidence row tested a stale surface and no report. It didn't test
a card replaced in place, or a report for an undrawn card. The test *"the drawing's latch and the program's agree"*
never ran through a Loading → Confirmation swap, which is exactly where they disagree: the drawing said *"read to the
end"*, the program said *seen*.

## 1. Commit 1: a card that takes another's place is a new card (`stikk-app`)

1. **One way to replace, and one way to remove, and both mint.** Every `*slot = Overlay::…` and every
   `self.overlays.remove(index)` goes through two methods beside `push_overlay`/`pop_overlay`:
   - **`replace_overlay(index, overlay)`**: mints a new `overlay_surface` **when `index` is the top**. A card replaced
     below the top changes nothing the reader is reading; `pop_overlay` mints when the reader returns to it.
   - **`remove_overlay(index)`**: mints, as the 53 hand-written lines do now.

   **Prefer making it structural**: move `overlays` and `overlay_surface` (and the minting counter they need) into
   a small private type whose fields `app.rs` cannot reach, with `push`, `pop`, `replace`, `remove`, `top`, `get`
   and **`top_mut` for editing the card's own fields** (its `error`, `typed`, `acknowledged`). If the type costs
   more than a short module, keep the two methods and say why in the request.
2. **`top_mut` is for edits to the same card, never a different card.** A whole-card assignment through it
   (`*top = Overlay::…`) is a replacement and goes through `replace`. Check every `overlays.last_mut()` site
   (`app.rs:1492`, `:1510`, `:1857`, `:4035`, `:4061`, `:4129`, `:4223`) and list each in the request with which
   kind it is.
3. **Fix `push_overlay`'s doc** so it states the rule it now keeps: *every card that reaches the top, by push,
   replace or pop, has a surface no earlier card had.*

## 2. Commit 2: a report is made only for what was drawn (`stikk-tui`)

1. **`Scroll` records that the renderer measured it.** The renderer's per-draw measurement (the call that writes
   `max` and the drawing's latch) also sets a `measured` flag. `Default` leaves it false, so an entry created by
   `Viewports::apply` or `Viewports::of` is not measured until it is drawn.
2. **`Viewports::reading(surface) -> Option<Reading>`**: `None` unless that surface's `Scroll` was measured. Its doc
   states both answers from §0 F2, and that `None` is how *"not drawn"* reaches the program.
3. **`ui_loop` (`lib.rs:162`) and `golden.rs:125` report only `Some`.** The program's *"no report"* defaults then do
   the rest: `↓` still moves, and the gate stays closed.
4. **Fix `reading`'s doc**, which today says an undrawn surface *"reads as … the end is on screen"*.

**Out of this fix:** keys still act while the too-small notice is up. After this commit no confirmation can act
through it, but other keys still do. That is a design question for 0.23.0 (I'll record it), not part of the patch.

## 3. Commit 3: the changelog

`CHANGELOG.md`, `## Unreleased`, **`### Fixed`**:

> A confirmation card that did not fit the window could be confirmed with one `Enter` without being read, and one
> hidden behind the "window too small" notice could be confirmed unseen (a regression in 0.22.0; 0.21.1 was not
> affected). The card again waits until its last line has been on screen.

## 4. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does a card that replaces a Loading card start unread? | logic (`stikk-app`) | **one test per preview whose answer replaces a Loading card**: commit, seal, trust write, save/switch, import, export, merge, undo, ref write, and any other you find from the `*slot = Overlay::` sites. Each: push the preview, report `at_end: true` for the Loading card's surface, deliver the answer, then assert **`top_card()`'s surface differs** and **`!seen_whole_card()`**. Then a report `at_end: true` for the new surface opens it. List the sites and the test that covers each. |
| Does a card replaced below the top leave the reader's surface alone? | logic | one test: a card over a Loading card; the answer arrives; the top's surface and its latch are unchanged |
| Is an undrawn card never reported as read? | headless TUI | `Viewports::reading` is `None` for a surface with no entry **and for one created by `apply` only**; a draw at 60×14 (too small) followed by `Enter` on a Confirmation leaves the queue unchanged and the card open |
| Do the drawing's latch and the program's agree through a swap? | headless TUI | the existing agreement test (`wiring/tests.rs:962`), extended: through a Loading → Confirmation swap at a height where the card overflows, both say *not seen*; after scrolling to the end, both say *seen* |
| Does the real binary hold the gate again? | real binary | `.git-exclude/reports/reading-gate-2026-11-25/probe.sh` on the head's release build, **output pasted verbatim**. The expected output is `60x15 … queued patches 0`, `60x14 … queued patches 0` and `160x50 … queued patches 1`. Usage: `probe.sh <abs path to target/release/stikk> <scratch dir> <record file>`. It needs prikk 0.58.0 at `~/.local/share/stikk-test-prikk/ceiling-0.58/bin/prikk`. |
| No public API change | review | `cargo public-api diff 0.22.0..HEAD` for `stikk-app`: empty |
| Everything else | gates | gates 1–8 (none exempted) and the local suite at 0.28.0 + 0.58.0 |

## 5. The traps

1. **Mint only when the top changes.** Minting on a replace below the top would reset the latch of the card the
   reader *is* reading, which would erase the record that they scrolled to its end.
2. **Don't make `App::at_end` answer "no" for an unreported card.** That changes what `↓` does on every
   undrawn card, and every handler test depends on it (its doc says so). F2 is fixed in the TUI's report, not in the
   program's default.
3. **"Has an entry" is not "was drawn".** `apply` creates entries. Use the `measured` flag.
4. **No source-scan guard** (a test that greps for `*slot =`). The behavioural tests in §4 are the guard. The
   structural type in §1 is what makes a bypass hard to write.
5. **The prototype is not yours to change.** `.git-exclude/reports/065-gui-toolkit/snora/` reads `stikk-app` by
   path. I'll rerun it after this lands.

## 6. Submit (no branch)

Gates 1–8 and the local suite. Package it to
`.git-exclude/review-request/067-stikk-app-the-shared-program-c/review-request-v1.md`, with §4's evidence, the list
of replace and `last_mut` sites and their kinds, and the probe's output.

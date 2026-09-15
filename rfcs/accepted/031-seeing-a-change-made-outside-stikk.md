# RFC 031 — Seeing a change made outside stikk

**Status.** **Accepted by the project owner 2026-09-15; Q1 ruled (b), Q2 ruled (b).** Proposed the same day by the
architect: 0.8.0's first increment.
**Tracks.** `FR-106`, `OP-04`, `LC-4`, `CT-05`, `NFR-R02`, `NFR-P01`, `TU-03`, RFC 003 (the change token), RFC 010
(the off-thread seam), RFC 030.
**Touches.** `stikk-tui` (the UI loop, `App`, the terminal guard); possibly `stikk-core` (stamping a token with an
Orientation read); on delivery, `requirements.md` and `external-design.md`.

## Summary

**stikk sees a change made outside it only when it happens to read again**: on open, on `r`, or after its own
commit or seal. A `prikk commit`, `seal` or `branch switch` in another terminal leaves stikk showing the old
state, and nothing says so.

The design has promised otherwise since 0.1.0. `OP-04` names the notice (*"repository changed outside stikk —
refreshed"*), and RFC 003 built the comparison that produces it. **Nothing calls it.**

This RFC wires it. **A check is cheap for prikk, but it must not be visible as work, and it must never be taken for
a change stikk made itself.**

## Findings

### F1 — the notice renders nowhere

`stikk_core::staleness_notice` exists and is tested, and **nothing calls it**. The TUI holds no change token at all:
`change_token` is read only inside `confirm` and `execute` (RFC 013, RFC 030), and stamped nowhere else. Found in
RFC 030's review, where the architect's handoff had wrongly assumed the notice already fired.

### F2 — what that costs a user

- **prikk's default in the status bar** (RFC 029) lags a terminal `prikk branch switch` until the next read.
- **History, the Queue view and Changes** show the state from when they were opened.
- **A confirmation armed on state that has since moved** is stopped only at Enter (RFC 030). By then the user has
  read the card, and decided on it.

### F3 — a check is cheap for prikk

One change-token read is three spawns: `branch list --all`, `tag list` and `status`. **Measured at prikk 0.42.0:**

| Repository | Sum of medians |
|---|---|
| 20 files, 1 block | **1.0 ms** |
| 301 files, 20 blocks, 11 branches, 1 queued patch | **3.0 ms** |

### F4 — but every request stikk makes is visible

Every request goes through `App::dispatch`, which **adds an entry to the Operations list** (capped at 20) and counts
in the status bar's **`⟳ n`**. A periodic check sent that way would flicker `⟳ 1` every few seconds, and push the
user's own entries out of the Operations list with work they never started. Both surfaces exist to answer *"is
something I asked for still running?"* (RFC 010).

### F5 — focus events exist, but are off and unreliable

stikk's crossterm (0.29) can report `FocusGained`/`FocusLost` once `EnableFocusChange` is sent. **stikk never sends
it.** Where it works, it is the moment that matters: the user coming back from the terminal where they ran prikk.
**It does not always work:** tmux forwards focus events only with `focus-events on`, and some terminals send none.
So **focus alone cannot be the mechanism.**

### F6 — `r` does not refresh every screen

`App::reload` re-reads Orientation, and the top screen **only if it is History or the Queue**. **Changes and Block
detail stay as they were.** A detected change can refresh no more than `r` does, so this gap is F2's too.

## Decisions

1. **stikk stamps a change token with every Orientation read** (on open, on `r`, and after its own commit or seal),
   read in the same worker request, so the token and what Orientation shows describe the same moment.
2. **A check is a silent background read.**
   - It enters neither the Operations list nor `⟳ n`.
   - **Its failure is not surfaced**; the next check simply runs.
   - **At most one check is in flight.**
   - **No check is sent while the user's own work is pending:** any request still running, and any commit or seal
     flow between its preview and the Orientation read that follows it.

   So a check can never race a change stikk makes itself.
3. **When checks run:** see Q1.
4. **On a detected change, stikk refreshes what is on screen, in place, and says so.**
   - Orientation, and the top screen, are re-read with their views kept visible while the read runs (RFC 010 §5).
   - The banner is `OP-04`'s exact words, *"repository changed outside stikk — refreshed"*, from
     `staleness_notice`.
   - The token is re-stamped.
5. **`r` refreshes Changes, and Block detail for the tip**, closing F6, through the same path decision 4 uses.
6. **An armed confirmation when a change is detected:** see Q2.
7. **Words are core's.** `staleness_notice` already owns the banner; the TUI decides only when to ask.
8. **Tests are `App` tests over scripted responses, without sleeping** (handoff §8's rule):
   - a check that finds no change says nothing;
   - a change refreshes and banners;
   - no check is sent while work is pending;
   - stikk's own commit never produces the notice;
   - checks never touch the Operations list or `⟳`.

   The real-binary suite already asserts that a raw prikk mutation moves the token
   (`change_token_is_stable_across_a_read_and_moves_across_a_mutation`).
9. **On delivery**, `FR-106` and `OP-04` record what is built. `TU-03` records that a check is not counted in `⟳`.

## Open questions

### Q1 — when does stikk look?

- **(a) On focus and on `r` only.** Nothing periodic runs. Instant where the terminal reports focus, and nothing at all
  where it does not (F5).
- **(b) On focus, and every few seconds while stikk is idle.** Focus makes the common case instant where supported, and
  the interval covers terminals that report no focus, and a split pane beside the one where prikk ran. At F3's
  measured 1–3 ms, a check every **5 seconds** is well under a tenth of a percent of one core. **The interval is a
  constant the handoff names, not a setting.**
- **(c) Every few seconds only.** Simple, and always a little late, even where focus would have been immediate.

**My lean is (b), at 5 seconds.** It is the only option that is both prompt where it can be and correct where it
cannot. The cost is measured, and decision 2 keeps it invisible.

### Q2 — what happens to a confirmation that is open when a change is detected?

- **(a) Leave it.** RFC 030 stops it at Enter, as today. The user may read and decide on a card for state that no
  longer exists.
- **(b) Replace it at once with the stale overlay**, cause *Repository* (RFC 030's words). **Nothing is armed**, and the
  one next step is to preview again. This is what `CT-05` already says: *"after any external-change detection, armed
  previews are marked stale and must be re-generated."*

**My lean is (b).** It is the design's own contract, and it stops a decision before it is made rather than after.
**Its limit, stated:** the token does not include the worktree (RFC 003; RFC 030 F2), so an editor's autosave is
**not** detected this way. That stays RFC 030's re-read at Enter.

### RULED by the project owner, 2026-09-15: Q1 (b), Q2 (b)

**Handoff A builds this:**

- **Q1 (b):** stikk checks **when the terminal reports focus returning**, and **every 5 seconds while it is idle**.
  - The interval is a named constant, not a setting.
  - A check is decision 2's silent read.
  - Where the terminal reports no focus, the interval alone serves.
- **Q2 (b):** a confirmation open when a repository change is detected is **replaced at once by the stale overlay**,
  cause *Repository*, in RFC 030's words. Nothing stays armed, and the one next step is to preview again, as `CT-05`
  requires. **Its limit stands:** a worktree-only change is not in the token, and is RFC 030's to catch at Enter.

## Delivery

**One handoff**, issued on the ruling. **Its first commit moves the workspace to `0.8.0`**, as RFC 027 A's
first commit did for 0.7.0.

## What this RFC does not do

- **No filesystem watcher.** It would add a dependency, and the only place a repository change is visible is inside
  `.prikk/`, which stikk never reads (`CON-1`).
- **No worktree polling.** A check does not run `worktree-status`, so an edit in the worktree is not noticed until `r`,
  opening Changes, or RFC 030's re-read at Enter. **The Changes view refreshes on a detected repository change or `r`,
  not on a file saved.**
- **No lock held**, and nothing blocks another writer (`NFR-R02`).
- **Not the prikk 0.43 re-baseline**, which prikk's reply 013 schedules. It is its own RFC when prikk 0.43.0 publishes.

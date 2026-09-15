# Handoff — seeing a change made outside stikk (v1)

**Companion to:** [RFC 031](../../accepted/031-seeing-a-change-made-outside-stikk.md). Accepted 2026-09-15; **Q1 ruled
(b)**, **Q2 ruled (b)**.
**This handoff is all of RFC 031:**
- a change token stamped with every Orientation read;
- a silent check on focus and every 5 seconds while idle;
- on a detected change, what is on screen refreshed in place, with `OP-04`'s notice;
- an open confirmation made stale at once;
- `r` refreshing Changes and the tip's Block detail.

**0.8.0's first increment.** §2 opens the release.
**Design items:** `FR-106`, `OP-04`, `LC-4`, `CT-05`, `NFR-R02`, `NFR-P01`, `TU-03`, RFC 010, RFC 030.

> **Three rules decide whether this is correct, and each one is easy to get backwards:**
>
> 1. **The token is read before the Orientation it is stamped with**, never after (§3). Read after, a change
>    landing between the two reads is absorbed silently and never noticed.
> 2. **A check is never sent while any request is running or an Orientation read is pending** (§4). That, and not
>    anything commit- or seal-specific, is what stops stikk from noticing its own commit as an outside change. **It
>    deliberately does not stop checks while a confirmation merely waits on the user**, because Q2 needs exactly
>    that moment.
> 3. **A check is invisible as work** (§4). It never enters the Operations list or `⟳ n`.

---

## 1. Scope

**In, in this order:**
1. The workspace moves to 0.8.0 (§2).
2. The token is stamped with Orientation (§3).
3. The silent check (§4).
4. When checks run: focus and the interval (§5).
5. A detected change (§6).
6. `r` refreshes Changes and the tip's Block detail (§7).
7. Tests (§8), then docs, changelog and Breaking (§9).

**Out:**
- a filesystem watcher;
- worktree polling (a check never runs `worktree-status`);
- any setting for the interval;
- the prikk 0.43 re-baseline;
- any change to RFC 030's re-read at Enter, which stays the guard for worktree-only changes.

## 2. The workspace moves to 0.8.0 — its own commit, first

- **`[workspace.package] version`** and **the five `stikk-*` pins** in the root `Cargo.toml` become `0.8.0`.
- **Refresh the lockfile for workspace members only:** `cargo update --workspace`. The `Cargo.lock` diff must show
  **only** the seven `stikk-*` versions moving. **If any third-party package moves, stop and report.**
- **`CHANGELOG.md` gains an empty `## Unreleased`** above `## 0.7.0 — 2026-09-15`.
- **Gates 1–3 on the MSRV**, then commit it alone, as `94a7315` did for 0.7.0.

## 3. The token is stamped with every Orientation read (decision 1)

- **The worker's `RequestKind::Orient` arm reads the change token first, then Orientation**, and answers both in
  one response. Name its shape in the crate's idiom (a pair, or a small struct).
  - **Why this order:** a change that lands between the two reads is then in Orientation but not in the token, so
    the next check differs and refreshes, which is harmless. The other order would put the change in the token
    and not in what is on screen, and **no check would ever notice it.** `confirm::preview` stamps its token before
    `compute` for the same reason.
- **`App` holds the last stamped token.** `apply_orient` stamps it **whenever the token read succeeded**, whatever
  Orientation's own result was.
- **A failed token read leaves the previous stamp as it was**, or none.
- **Every Orientation read stamps:** on open, `r`, a detected change (§6), and after stikk's own commit or seal
  (already dispatched in `apply_commit_confirm_execute` and `apply_seal_confirm_execute`).
- **The first stamp says nothing.** It is not a change.

## 4. The silent check (decision 2)

- **A new request kind** (for example `ChangeCheck`) runs `stikk_core::change_token`, and its response carries the
  `Result<ChangeToken>`.
- **It is sent through a path that does not touch `operations`**: a separate dispatch, or a flag on the existing one.
  It adds **no `Operation`**, so **`in_flight_count()` and `operations()` are unchanged by it**, before, during and
  after.
- **`App` tracks at most one check in flight** (for example `check_pending: Option<u64>`). A response whose `seq`
  does not match is discarded, like every other stale response.
- **A check is sent only when all of these hold:**
  1. a token has been stamped;
  2. no check is already pending;
  3. **`in_flight_count() == 0`**, so no request the user caused is running (a preview, a confirm-and-execute, a
     History read, anything);
  4. **`orientation_pending` is `None`.**

  **Nothing else suppresses it**: not an open confirmation, not an open overlay. §6's Q2 depends on checking while a
  confirmation waits on the user.
- **A failed check is silent:** no banner and no overlay; the next opportunity simply sends another.
- **Why this is enough to never flag stikk's own writes.** Pressing Enter dispatches the confirm-and-execute request,
  so something is running and no check is sent. Its success dispatches Orientation, so `orientation_pending` is set
  and still no check. That Orientation read stamps a token taken **after** the commit. **Show this in a test** (§8),
  including a check that was already in flight when Enter was pressed. The worker is FIFO, so that check runs before
  the commit and returns a token equal to the stamp.

## 5. When checks run (Q1 (b))

- **`const CHANGE_CHECK_INTERVAL: Duration = Duration::from_secs(5);`** in `stikk-tui`, with a doc comment citing
  RFC 031 Q1 and F3's measured cost. **Not a setting.**
- **Time is passed in, never read inside `App`**, so tests never sleep:
  - `App::tick(now: Instant)` sends a check when §4's conditions hold **and** `CHANGE_CHECK_INTERVAL` has passed since
    the last check was **sent** or the last token was **stamped**, whichever is later;
  - `ui_loop` calls it once per iteration with `Instant::now()`.
- **Focus.**
  - `TerminalGuard::enter` sends `EnableFocusChange` after the alternate screen.
  - `restore` sends `DisableFocusChange` before leaving it. `restore` is also the panic path, and stays idempotent.
  - `ui_loop` maps `Event::FocusGained` to `App::focus_gained(now)`, which sends a check **at once** when §4's
    conditions hold, and counts as "sent" for the interval.
  - `FocusLost` is ignored.
  - **If enabling focus change fails, stikk runs on without it**, since the interval still serves. It must not fail
    startup.
- **`POLL` stays 50 ms.** `tick` does no I/O unless it sends a check, and a check is one message on the existing
  channel.

## 6. A detected change (decisions 4 and 7, and Q2 (b))

When a check's token **differs** from the stamp, in this order:

1. **Stamp the new token.**
2. **An armed confirmation goes stale (Q2).** If the top overlay is `Overlay::Confirmation`, or `Overlay::SealConsent`
   (seal's second act, still armed):
   - replace it with the stale overlay for that operation — `commit`, or `seal` — built through `present()` from
     `StikkError::Stale { operation, cause: StaleCause::Repository }`, so the words are RFC 030's, from core;
   - **clear `pending_commit` and `pending_seal` exactly as `App::back` does** for those overlays, so nothing stays armed;
   - its one next step, *Preview again*, works as it does today.

   **`CommitMessage` is not armed** (no preview exists yet), so leave it.
3. **Refresh what is on screen, in place**, through the same function `r` uses after §7: Orientation, and the top
   screen, whichever it is.
4. **Set the banner to `staleness_notice`'s words**, *"repository changed outside stikk — refreshed"*, **after** the
   refresh, since `reload` clears the banner.

When the tokens are **equal**, nothing happens: no banner, no refresh, no overlay change.

## 7. `r` refreshes Changes and the tip's Block detail (decision 5)

**Today `reload` refreshes the top screen only when it is History or the Queue** (RFC 031 F6).

- **Changes** gains an in-place refresh, in the shape `History` and `Queue` already have: a `refreshing: Option<u64>`
  slot, the view kept visible while the read runs, and `apply_changes` handling the top-refresh case as
  `apply_queue` does.
  - Keep `hide_untracked` as the user set it.
  - On error, keep the view and surface the error with `OperationContext::LoadChanges`.
- **Block detail refreshes only for the tip** (`is_tip`), because an older block's detail cannot change.
  - This needs a `refreshing` slot too, so `Screen::BlockDetail(BlockDetailView)` becomes a struct variant, which is
    breaking (§9).
  - A non-tip Block detail is left as it is.
- **`reload` is the one function** §6 calls. Do not write a second refresh path.

## 8. Tests

**`App` tests, over scripted responses, with synthetic `Instant`s. No test sleeps.**

1. **No check before the first stamp.** A check with no stamp is never sent, and the first stamp says nothing.
2. **The interval:**
   - a `tick` at 4.9 s sends nothing;
   - at 5 s it sends exactly one check;
   - another `tick` while that check is pending sends nothing.
3. **Focus:** `focus_gained` sends a check at once, and resets the interval.
4. **Suppression.** No check is sent while:
   - any request is running (History, a preview, a confirm-and-execute);
   - `orientation_pending` is set.
5. **Invisible:** `operations()` and `in_flight_count()` are identical before sending a check, while it is pending, and
   after its response, whether equal, different or failed.
6. **Equal:** no banner, no dispatch, no overlay change.
7. **Different:** the stamp updates, Orientation and the top screen's refresh are dispatched, and the banner is exactly
   `OP-04`'s words.
8. **A failed check:** nothing shown; the next `tick` past the interval sends another.
9. **stikk's own commit is never an outside change:**
   - a confirmation is confirmed, so no check while it runs;
   - success, so no check while Orientation is pending;
   - Orientation stamps, so a following equal check shows no banner;
   - **and a check already pending when Enter was pressed** returns the pre-commit token, equal to the stamp, with no
     banner.
10. **Q2, commit:** a `Confirmation` is open and a different token arrives. Assert the stale overlay (cause
    *Repository*, operation `commit`), `pending_commit` is `None`, and the refresh and banner of test 7.
11. **Q2, seal:** the same with `SealConsent` open, and `pending_seal` is `None`.
12. **Not armed:** a `CommitMessage` open when a change is detected stays open.
13. **§7:**
    - `r` on Changes refreshes in place with `hide_untracked` kept;
    - on tip Block detail it refreshes;
    - on a non-tip Block detail it does not.
14. **The worker's Orient arm reads the token before Orientation.** Use a scripted backend that records call order, and
    assert `change_token` came first.

**Terminal focus cannot be unit-tested honestly** (escape sequences to a real terminal). **Verify it by hand and report
exactly what you saw:**
- stikk in a terminal, switch away, run a raw `prikk commit` in another window, switch back — the banner and the
  refresh appear at once;
- the same inside tmux with `focus-events off` — they appear within 5 seconds;
- the terminal is restored normally on quit, **and after a forced panic**, with focus reporting off. Check that typing
  in the shell afterwards shows no stray `^[[I` or `^[[O`.

## 9. Docs, changelog, Breaking

**On delivery:**
- **`FR-106`** and **`OP-04`** record what is built: checks on focus and every 5 seconds while idle, a silent read, the
  refresh and notice, and an open confirmation made stale.
- **`TU-03`** records that a change check is not counted in `⟳`.
- **`CT-05`** is already true, and needs no change.

**Changelog, `## Unreleased`:**
- **`### Added`**: stikk notices a change made outside it — a commit, seal or branch switch in another terminal —
  when the terminal regains focus or within 5 seconds. It refreshes what is on screen and says *"repository changed
  outside stikk — refreshed"*. A confirmation that is open when that happens becomes stale at once.
- **`### Changed`**: `r` refreshes the Changes view, and the tip's Block detail, in place.
- **State the limit:** an edit to a file in the worktree is not noticed this way. It is caught at Enter (RFC 030), or
  by `r`.

**Breaking, by API diff.** Expect `Screen::Changes` gaining `refreshing`, and `Screen::BlockDetail` becoming a struct
variant, both breaking for struct-literal construction or an exhaustive `match`. `App::tick` and
`App::focus_gained` are additive. The worker's request and response kinds are crate-private.

## 10. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite and
supply-chain run ids at one SHA, and Docs after the push.

## 11. Acceptance criteria

1. **§2's version commit, alone:** the lockfile moves only `stikk-*`, and `## Unreleased` is added.
2. **The token is read before Orientation** in one worker request, and stamped on every successful token read; a
   failed read keeps the previous stamp; the first stamp is silent.
3. **The check:** silent, at most one, sent only under §4's four conditions, and never in the Operations list or `⟳`.
4. **When checks run:** `CHANGE_CHECK_INTERVAL` is 5 s; `tick` and `focus_gained` take `Instant`; focus reporting is
   enabled on entry, disabled on restore (the panic path included), and never fails startup.
5. **A detected change:** re-stamp; an armed `Confirmation` or `SealConsent` replaced by the stale overlay (cause
   *Repository*, through `present()`), with pending flows cleared as `back` clears them; the refresh through `reload`;
   the banner in `OP-04`'s exact words, after it.
6. **`r` refreshes Changes and the tip's Block detail in place.**
7. **§8's fourteen tests,** and the hand verification reported as observed.
8. **§9's docs and changelog, and a Breaking table built by API diff.**
9. **Eight gates** under the toolchain rule; run ids at one SHA.
10. **Nothing tagged or published.**

## 12. Submit

Package to `.git-exclude/review-request/031-change-awareness/review-request-v1.md`.

**In this order:**
1. **Test 9**, stikk's own commit never flagged, including the check already pending at Enter.
2. **Tests 10 and 11**, Q2.
3. **Test 5**, invisibility.
4. **The hand verification**: focus, tmux without focus events, restore after a panic.
5. **The Breaking table.**

**And tell me whether 5 seconds felt right at the keyboard**, with a terminal beside stikk. If it felt slow or noisy,
say which, and what you measured.

**Push once approved.**

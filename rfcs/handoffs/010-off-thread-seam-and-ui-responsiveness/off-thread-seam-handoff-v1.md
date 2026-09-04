# Handoff — the off-thread seam (v1)

**Companion to:** [RFC 010](../../done/010-off-thread-seam-and-ui-responsiveness.md)
(Accepted 2026-09-04). Inherits its state.
**Realizes:** the first increment of **0.3.0** ([RFC 012](../../accepted/012-post-0-2-0-correctness-sweep.md)'s
re-sequenced roadmap). It goes first because it reshapes the seam trait; RFC 012 and RFC 003 follow it.
**Design items:** `NFR-P01` (the UI never blocks — **a Must, currently unmet**), `CC-01` (one UI thread,
seam off-thread), `SEAM-02` (`Prikk` is `Send + Sync`), `SEAM-05` (handshake recorded at open),
`TU-03` (background-operation indicator), `TU-01` (Background Operations overlay), `AR-03`/`AR-04`
(the frontend computes nothing; `stikk-core` owns no I/O policy).

**This is a breaking release increment** — the trait gains a bound and `stikk-tui`'s public `App` API
changes shape. That is expected and allowed in 0.3.0 (RFC 011); do not contort the design to avoid it.

> **What this increment is not.** It does **not** add cancellation, a cancellation token, or any control
> labelled "cancel" — RFC 010's Q3/Q4 rulings explain why at length. If you find yourself reaching for
> `Child::kill()` or rewriting `Command::output()`, stop: you are outside scope and about to
> destabilize the `UD-04` EPIPE guard for no measurable gain.

---

## 1. Scope

**In:**
1. `Prikk: Send + Sync` (`SEAM-02`). **Verified additive** — `CliBackend` is `{ program: OsString }`,
   `NullBackend` is `Handshake` + five `Result<T, String>` fields, and there is no `Rc`/`RefCell`/`Cell`
   anywhere in `stikk-prikk`. Neither implementor changes.
2. A **worker thread** driving every `stikk-core` operation, with the UI thread rendering and polling
   input throughout.
3. **Per-view load states** that are actually reachable, and **stale-response discarding** (§4 — the
   real correctness risk of this increment).
4. **Handshake caching** so the version probe stops costing a process spawn per operation.
5. The `TU-03` **`⟳ n` indicator** and the `TU-01` **Background Operations overlay** (listing only —
   no cancel action).

**Out (do not build here):**
- **Cancellation of any kind**, `Child::kill()`, or changes to `Command::output()` / the EPIPE guard.
  Lands with `FR-100` (verify). RFC 010 Q3.
- A **thread pool** — one worker (RFC 010 Q2).
- An **async runtime**. `std::thread` + `std::sync::mpsc` only; no new dependency (RFC 010 Q1).
- Any change to what an operation *computes*, any new seam method, any RFC 012 or RFC 003 work.

---

## 2. Threading shape

**`std::thread::scope`, not `Arc`.** The worker must *borrow* `&impl Prikk`, so
`stikk_tui::run(repo, prikk, config)` keeps its signature and the launcher keeps passing a borrowed
`CliBackend`. Sketch:

```rust
pub fn run(repo: &Path, prikk: &(impl Prikk + Sync), config: &Config) -> Result<()> {
    let (req_tx, req_rx) = mpsc::channel::<Request>();
    let (res_tx, res_rx) = mpsc::channel::<Response>();
    std::thread::scope(|scope| {
        scope.spawn(|| worker(prikk, repo, req_rx, res_tx));   // borrows prikk
        ui_loop(config, req_tx, res_rx)                        // owns the terminal
    })
}
```

The worker loop is: receive a `Request`, call the matching `stikk-core` operation, send a `Response`.
It ends when the request channel closes (the UI thread drops its sender on quit), so no shutdown flag
is needed. **`stikk-core` stays synchronous and unchanged** (RFC 010 decision 3) — the worker is simply
the thread that calls it.

**The UI loop must not block on the response channel.** Keep `event::poll(POLL)` for input and
`res_rx.try_recv()` each pass. **Lower `POLL` from 250 ms to ~50 ms**: it currently bounds how long a
*response* waits to be applied, and 250 ms of latency after a fast read would be a visible stutter that
did not exist before. Draw each pass regardless.

---

## 3. Request / response protocol

One enum each, in `stikk-tui` (they are frontend↔worker plumbing, not operations):

```rust
enum Request { Orient, History { reff: String }, BlockState { reff: String, row: BlockRow, is_tip: bool },
               Refs, Changes { reff: String } }
enum Response { Orient(Result<OrientationView>), History(Result<HistoryView>), … }
```

Every `App` method that currently takes `prikk: &impl Prikk` — `open`, `reload`, `open_history`,
`open_changes`, `open_ref_picker`, `select`, `select_screen`, `activate`, `activate_view`,
`run_command` — **stops taking it** and instead *sends a `Request`*. The `App` gains one entry point,
`apply(&mut self, response: Response)`, which does what the old success/error arms did. Route errors
through `present()` exactly as today: `surface()` is unchanged and is called from `apply`.

---

## 4. Stale responses — the correctness risk *[read this twice]*

Today a load is synchronous, so a result cannot arrive after the user has moved on. Off-thread, it can.
The failure is concrete: the user presses `w` (Changes), the read is slow, they press `Esc` back to
Orientation, and the response lands — pushing a Changes screen they did not ask for and are not
looking at. Worse with a refusal: an overlay appears over an unrelated view.

**Required mechanism — a generation counter.** `App` holds `next_seq: u64` and, for the pending work,
the sequence it is waiting on. Every `Request` carries a `seq`; every `Response` echoes it; `apply`
**discards any response whose `seq` is not the one currently awaited**. Bumping the counter on every
navigation (including `back()`) invalidates in-flight work implicitly — which is exactly the
"stop waiting" semantics RFC 010 decision 4 describes, achieved without touching the child process.

This is not optional polish. Without it the increment introduces a class of bug the synchronous code
could not have, and reviewers will look for it first.

---

## 5. Load states must be reachable

`OrientationState::Loading` exists today and **no user can ever observe it**, because `App::open`
completes the load before the first draw. After this change it is real, and the pushed screens need the
same treatment: History, Block detail and Changes are currently pushed only on success, so there is no
state for "asked for, not yet arrived."

Add a pending representation — a `Screen::Loading { what: &'static str }` pushed immediately and
replaced by `apply`, or an `Option` inside each screen; **your call, but say which and why in the
review request.** Whichever you choose, a load in flight must render something honest ("loading
history…"), never a blank pane and never the previous view frozen.

The status bar's `⟳ n` indicator (`TU-03`) shows the count of in-flight requests, and the Background
Operations overlay (`TU-01`) lists them with their outcome once finished. **No cancel action** — the
overlay is a listing this increment.

---

## 6. Handshake caching (`SEAM-05`)

`orient()` and `changes_view()` each call `prikk.handshake()`, so opening Changes costs two process
spawns and `history_view()` costs two calls of its own. `SEAM-05` describes the handshake as recorded
**at open**, once.

**Ruled: cache inside `CliBackend` with a `std::sync::OnceLock<Handshake>`.** It is `Send + Sync` when
its contents are, keeps every signature unchanged, keeps `NullBackend` untouched (it spawns nothing and
its `with_version` builder must keep working), and matches the design's own "recorded at open"
semantics — a session does not notice prikk being upgraded underneath it, which is correct.

Do **not** instead change `changes_view`/`orient` to take a `Handshake` parameter. It is a larger
signature change for the same effect, and it pushes a caching decision into the operation layer, which
owns no I/O policy (`AR-03`).

---

## 7. Security surface

No new trust boundary, no mutation, no key material, no new parsing. Two things to hold:

- **`INV-8` still holds.** A view-model arriving from the worker is still a rendered snapshot, never
  authority; `apply` stores it and nothing else re-derives from it.
- **`ER-03` stays the single mapping.** Errors arrive on the channel as `Result`s and go through
  `present()` in `apply` — do not add a second error path for "worker failed". If the worker thread
  itself panics, the channel closes; treat that as `StikkError::Internal` → `FaultScreen` (`ER-04`),
  whose "the repository was not touched" wording remains true, since only the seam writes and this
  increment adds no mutation.

No threat-model change is required (no new asset, flow or boundary). Record that the review happened,
per `NFR-S07`.

---

## 8. Test plan

- **The bound**: a compile-time assertion that `CliBackend` and `NullBackend` are `Send + Sync`
  (`const fn assert_send_sync<T: Send + Sync>() {}`). Cheap, and it fails loudly if someone later adds
  interior mutability to a backend.
- **Stale-response discarding (§4)** — the increment's headline test: construct an `App`, dispatch a
  request, navigate away, then `apply` the now-stale response, and assert **no screen was pushed and no
  overlay opened**. Also assert the in-order case still applies normally.
- **Load states (§5)**: a `TestBackend` render asserting the loading text appears for a screen whose
  response has not arrived, and that `apply` replaces it.
- **Handshake caching**: with a counting test backend (or by asserting `CliBackend` calls the program
  once across two operations), prove the probe happens once per session.
- **No regression in routing**: the existing `app/tests.rs` cases that assert refusal→overlay,
  lock→banner and version-guidance→banner must still pass, now driven through `apply`.
- **Gates**: `fmt` / `clippy --all-targets --all-features -D warnings` / `test`. The count will rise;
  say by how much and why in the review request.

**Do not** write a test that sleeps to wait for the worker. Drive `apply` directly with constructed
responses — the threading is `std` and does not need proving; what needs proving is the state machine.

---

## 9. Example

Update the four demos so they still run: they construct an `App` and drive it, and the methods they
call are changing shape. `orientation_demo` should now visibly show the loading state before the
scripted response lands — the state that has never been observable. Keep them readable; they are
documentation (RFC 011 §finding 2 counts 13 construction sites in them, so expect churn there).

---

## 10. Acceptance criteria

1. `Prikk` is `Send + Sync`; both backends satisfy it unchanged; a compile-time assertion proves it.
2. Every seam-driven operation runs on a worker thread borrowed via `std::thread::scope`; the UI thread
   polls input and draws throughout. **No `Arc`, no async runtime, no new dependency.**
3. **A response for work the user has navigated away from is discarded** (§4), proven by test.
4. A load in flight renders an honest loading state in every view that can have one; `⟳ n` shows the
   in-flight count; the Background Operations overlay lists running and finished work.
5. The handshake is probed **once per session** for `CliBackend`, proven by test; `NullBackend` and its
   `with_version` builder are unchanged.
6. **No cancellation of any kind was added**, `Command::output()` and the EPIPE guard are untouched, and
   no UI string says "cancel".
7. Every error still reaches the user through `present()` (`ER-03`); a worker panic surfaces as a fault
   screen, not a hang.
8. Gates green; all four demos build and run.

---

## 11. Submit

Package to `.git-exclude/review-request/010-off-thread-seam-and-ui-responsiveness/review-request-v1.md`.

Say explicitly: which pending-state representation you chose (§5) and why; the test count delta; and —
because this is the one thing a green suite cannot show — **how you convinced yourself the UI actually
stays responsive**. A screenshot proves a frame drew; it does not prove input was accepted while a read
was in flight. Describe what you did.

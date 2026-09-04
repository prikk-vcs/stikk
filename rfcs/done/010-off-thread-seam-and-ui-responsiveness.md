# RFC 010 — The off-thread seam and UI responsiveness

**Status.** Implemented (0.3.0 candidate; on `main` 2026-09-04, reviewed and approved with no corrections) — moved every seam call off the UI thread behind a worker, gave the
`Prikk` trait the `Send + Sync` bound its design always specified, and cache the handshake per session.
**First increment of 0.3.0** (RFC 012's re-sequenced roadmap), because it reshapes the seam trait and
everything after it would otherwise be re-touched. Handoff:
[`../handoffs/010-off-thread-seam-and-ui-responsiveness/off-thread-seam-handoff-v1.md`](../handoffs/010-off-thread-seam-and-ui-responsiveness/off-thread-seam-handoff-v1.md).

> **Scope narrowed at acceptance: true cancellation is deferred, deliberately.** This RFC delivers
> `NFR-P01` (the UI never blocks), which was live and unmet.
> **Deferred, carried forward (not built by this RFC):** `NFR-P02` — true cancellation, and the
> Background Operations overlay's cancel action, which land together with `FR-100` (verify), the first
> operation long enough for cancellation to mean anything. Until then no stikk control is labelled
> "cancel". It does **not** deliver `NFR-P02`
> (cancellation), because every operation `NFR-P02` names — verify, bundle build/import, sync accept —
> **does not exist yet**, and implementing cancellation would mean rewriting the `UD-04` EPIPE guard for
> a benefit nothing can measure. See the rulings under §Open questions. stikk will not offer a control
> labelled "cancel" that does not cancel.
**Tracks.** `NFR-P01` (the UI never blocks input), `NFR-P02` (long operations are cancellable),
`CC-01` (one UI thread, seam off-thread), `SEAM-02` (the trait is `Send + Sync`-bounded), `OP-02`
(background operations), and the `TU-01`/`TU-03` background-operation surfaces.
**Touches.** `stikk-prikk` (trait bounds; a cancellation parameter on the long categories),
`stikk-tui` (the run loop, `App`'s load states, the background-operations overlay, the status-bar
indicator). `stikk-core` stays **synchronous** — see decision 3. No new seam method, no mutation.

## Summary

stikk's internal design was explicit that seam calls run off the UI thread (`CC-01`) and that the
`Prikk` trait is `Send + Sync` so they can (`SEAM-02`). Neither is true in the shipped code. Every
repository read — opening the app, History, Changes, the ref picker, refresh — runs synchronously
inside the render loop, and the trait carries no thread bounds at all.

This was a deliberate, *scoped* concession in RFC 001's handoff: for that increment's single
orientation call, it said the blocking path was "acceptable for this single call this increment", that
"the worker path is preferred", and that it "is the pattern History will need." Three increments later
the concession is still in force, now covering five call sites, and no RFC records it as a deviation.
That is the shape of debt this project's method exists to prevent, so this RFC names it and pays it.

`NFR-P01` is a **Must** requirement. It is currently unmet.

## The findings that scope this increment

1. **Every seam call blocks the render loop.** `App::open` → `load`, `open_history`, `open_changes`,
   `open_ref_picker`, and `reload` all call `stikk-core` synchronously from
   `stikk_tui::run`'s event loop. Each one spawns a `prikk` process and waits. Input is not polled and
   no frame is drawn until it returns; `OrientationState::Loading` exists but can never be observed,
   because the load completes before the first draw.
2. **The `Prikk` trait has no `Send + Sync` bound**, so the worker cannot be introduced without
   changing the trait — which is exactly why it should change now, while `CliBackend` and `NullBackend`
   are its only implementors and no mutating method exists. A linked-library backend (RFC 005) would
   arrive into whichever shape we leave here.
3. **No method carries a cancellation signal or reports progress**, though `SEAM-02` specifies both for
   the long/cancellable ones and `RequestCategory::cancellable_in_flight` already encodes *which* those
   are. `NFR-P02` is therefore unmet as well.
4. **The handshake is re-run on every operation.** `orient` and `changes_view` each call
   `prikk.handshake()`, spawning `prikk --version` before doing their real work — so opening Changes
   costs two process spawns, and `history_view` costs two (`log` + `status`). `SEAM-05` describes the
   handshake as something stikk records **at open**. Caching it per session removes a spawn from every
   operation and is a prerequisite for a sane worker protocol.
5. **The background-operation surfaces specified in the design do not exist**: `TU-01`'s Background
   Operations overlay and `TU-03`'s `⟳ n` indicator have nothing to show, because nothing runs in the
   background.

**What this costs today, honestly stated.** On a small repository the reads are fast and the freeze is
imperceptible — which is why it has not hurt yet. The cost is not today's latency; it is that the
architecture cannot express a slow operation at all. `NFR-P03` Tier 2 admits 25,000 blocks; prikk's own
`verify` is linear (measured at 27.04 ms for 160 blocks in prikk 0.30's own correction of its docs), so
a Tier-2 verify is seconds, not milliseconds — and merge evidence, `bundle import`, and `sync accept`
are the heavy operations the 0.3 and later roadmap adds. Every one of them lands on a loop that has no
way to stay responsive.

## Decisions

1. **`Prikk` becomes `Send + Sync`** (`SEAM-02`, as designed). `CliBackend` and `NullBackend` already
   satisfy it; the bound is additive for callers today.
2. **The frontend owns the threading.** `stikk-tui` runs seam-driving operations on a worker thread and
   receives results on a channel; the run loop keeps polling input and drawing at its existing cadence.
   A view in flight renders its `Loading` state — which becomes reachable for the first time.
3. **`stikk-core` stays synchronous, and no async runtime is introduced.** The operation layer keeps
   its plain blocking signatures and is simply *called from* the worker. This preserves `AR-03` (core
   owns no I/O policy), keeps parity mechanical (`FR-123` — the GUI calls the same synchronous
   operations from its own scheduler), avoids pulling a runtime into a no-network terminal tool
   (*Less is more*), and keeps `NullBackend`-driven tests deterministic and thread-free.
4. **No cancellation token, and no "cancel" affordance, in this increment** *(revised at acceptance —
   the original decision proposed a cooperative token; see §Open questions Q3/Q4 for why that was
   wrong to do now)*. A view whose read is in flight may be **left** — the user navigates away and the
   result is discarded when it arrives — and the UI says exactly that ("stop waiting"), never "cancel".
   Mutating categories remain uncancellable in flight and single-shot regardless (`SEAM-04`,
   `NFR-S04`); for them cancellation was always a *pre-execution* concept, which the preview stage
   already provides.
5. **The handshake is performed once per session and cached**, and the version gate reads the cached
   value. `changes_view`'s `≥ 0.28` check (RFC 008) and `orient`'s support flag both consume it.
6. **The background-operation surfaces become real, minus cancel**: an operations registry the worker
   reports into, and the `TU-03` `⟳ n` status-bar indicator. The `TU-01` Background Operations overlay
   lists running and finished operations; it gains its cancel action with `NFR-P02`, not here. What
   this makes observable is that work *is happening off the UI thread* — which today a user cannot
   tell, because the UI simply freezes.

## Open questions — all four ruled at acceptance (2026-09-04)

**Q1 — `std::thread` + `mpsc`, or a small scheduler?** **Ruled: `std::thread` + `mpsc`**, and
specifically **`std::thread::scope`**. Scoped threads let the worker *borrow* `&impl Prikk` rather than
requiring `Arc` or a `'static` bound, so `stikk_tui::run(repo, prikk, config)` keeps its signature and
the launcher keeps handing in a borrowed `CliBackend`. No dependency added.

**Q1a — is `Send + Sync` actually free?** **Verified, not assumed:** `CliBackend` is
`{ program: OsString }`; `NullBackend` holds `Handshake` plus five `Scripted<T> = Result<T, String>`
fields. Neither has interior mutability — no `Rc`, `RefCell` or `Cell` anywhere in `stikk-prikk`. The
bound is purely additive for both existing implementors.

**Q2 — one worker or a pool?** **Ruled: one.** `CT-05` permits concurrent reads, but stikk's views are
driven one at a time by a single user. Revisit when a view genuinely fans out — Compare's
materialize-two-tips route (RFC 008) would be the first real candidate.

**Q3 — kill the `prikk` child on cancel, or drain and drop?** *This was left explicitly to be "decided
with evidence."* The evidence says **do neither yet, and do not ship a cancel affordance**:

- **Every operation `NFR-P02` names does not exist.** It lists verify, deep history walks, bundle
  build/import, and sync accept. Of these only history walks exist, bounded by `--limit 200`.
- **The current read set is milliseconds.** prikk's own measured figure for its most expensive
  operation is `verify` at **27.04 ms for 160 blocks**, and it is linear; `log`, `status` and
  `worktree-status` are cheaper still.
- **Kill-cancellation would mean rewriting the `UD-04` EPIPE guard.** Killing requires replacing
  `Command::output()` — which drains both pipes to EOF, and *is* the guard — with `spawn()` plus
  manual draining. That is the most safety-critical code in the seam, and rewriting it to cancel
  operations that finish in milliseconds is speculative complexity of exactly the kind RFC 005 refuses
  to add on unmeasured grounds.

So: the worker makes the UI responsive, and a user who navigates away from an in-flight read has its
result discarded on arrival. The UI calls that **"stop waiting"**, not "cancel", because that is what
it does. True cancellation lands with `FR-100` (verify) — the first operation long enough for it to
mean anything, and the point at which destabilizing the EPIPE guard buys something measurable.

**Q4 — does `stikk-core` take the cancellation token?** **Ruled: no token at all**, following Q3. A
parameter that is threaded through every operation and honoured by none is worse than no parameter — it
is a lie told in the type signature. Adding it later is a minor bump, which RFC 011 already establishes
as cheap and expected pre-1.0.

## Consequences

- `NFR-P01` and `NFR-P02` move from *specified* to *met*, and `OrientationState::Loading` becomes a
  state the user can actually see.
- The seam's trait signature changes **once**, while it has two implementors and no mutating method —
  instead of during the working-cycle increment, when it would have a dozen call sites and a
  confirmation pipeline on top of it.
- RFC 005 (linked-library backend) inherits a trait that already carries the right bounds, so it does
  not have to re-open them.
- Handshake caching removes one process spawn from every operation — a correctness-neutral efficiency
  gain that also simplifies the worker protocol.
- The debt is recorded either way: if the owner sequences this **after** increment 6, this RFC is the
  standing record of a Must-priority requirement knowingly unmet, which is the honest position. It is
  not, however, the recommended one.

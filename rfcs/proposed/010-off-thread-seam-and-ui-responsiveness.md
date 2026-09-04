# RFC 010 — The off-thread seam and UI responsiveness

**Status.** Proposed (2026-09-04) — move every seam call off the UI thread behind a worker, give the
`Prikk` trait the `Send + Sync` bound its design always specified, cache the handshake per session, and
make cancellation and progress real. Proposed **before** the 0.3 working cycle, because the change
alters the seam's trait signature and is far cheaper before mutating operations exist than after.
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
4. **Cancellation is a cooperative token passed to the cancellable categories**
   (`RequestCategory::cancellable_in_flight` already names them: the four read categories). A cancelled
   read abandons its result; it never leaves stikk state inconsistent, because stikk holds no repository
   authority (`INV-1`). Mutating categories remain **uncancellable in flight** and single-shot
   (`SEAM-04`, `NFR-S04`) — cancellation applies *before* execution only, which is the preview stage.
5. **The handshake is performed once per session and cached**, and the version gate reads the cached
   value. `changes_view`'s `≥ 0.28` check (RFC 008) and `orient`'s support flag both consume it.
6. **The background-operation surfaces become real**: an operations registry the worker reports into,
   the `TU-03` `⟳ n` status-bar indicator, and the `TU-01` Background Operations overlay listing
   running/finished operations with cancel. This is what makes `NFR-P02` observable rather than claimed.

## Open questions

- **`std::thread` + `mpsc`, or a small scheduler?** *Recommendation: `std::thread` + `mpsc`.* One
  worker with a request/response channel covers every read stikk performs; it adds no dependency, and
  `CC-02`'s eventual per-repository mutation gate is a mutex in the same process, not a reason for a
  runtime. Revisit only if a genuinely concurrent read workload appears.
- **One worker or a small pool?** *Recommendation: one, initially.* `CT-05` allows concurrent reads,
  but stikk's views are driven one at a time by a single user; a pool is speculation until a view
  fans out (Compare's two-tree materialize would be the first real candidate).
- **Does a cancelled `prikk` child get killed, or drained and dropped?** *Open — decide with evidence.*
  Killing risks leaving prikk mid-write on a future mutating call; draining honours the `UD-04` EPIPE
  guard, which is why the seam drains today. Proposed: **drain and drop for reads**, and never cancel a
  mutation in flight (decision 4), which sidesteps the dangerous half entirely.
- **Should `stikk-core` operations take the cancellation token, or only the seam?** Proposed: the seam
  takes it and the operation layer threads it through, so a multi-call operation (`history_view` makes
  two seam calls) can stop between calls.

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

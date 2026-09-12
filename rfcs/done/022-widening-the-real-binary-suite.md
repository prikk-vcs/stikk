# RFC 022 — Widening the real-binary suite: the five surfaces, and the arms nothing can reach

**Status.** **Done** — shipped on `main` 2026-09-12 (`8221c15`, `3938d02`, `4613cd2`), 0.6.0 candidate. The suite now drives **ten of ten** seam methods at both ends on three platforms, and its first widened run found a real prikk 0.28 Windows defect. Accepted by the project owner 2026-09-12, Q1 left unruled and **ruled by the architect the same day on evidence** (below). Opens 0.6.0. Scheduled at RFC 021 Handoff B's review, where the
suite's first real job found nothing **in what it checks** — and the six stale classifier fixtures that
re-baseline did move were found by hand, which is the cost RFC 019 exists to remove.
**Tracks.** `TS-07`, `TS-03`, `UD-02`, `UD-05`, `NFR-T01`, `T-T4`, `FR-106`.
**Touches.** `crates/stikk-real-binary`, `.github/workflows/real-binary.yml`, `classify.rs`'s arm
documentation, and one line of 0.5.0's changelog claim.

## Summary

RFC 019 built a suite that drives real prikk binaries at both ends of the supported range. It works: it
proved the client-side refusals stikk prevents are refusals prikk genuinely gives, and a first full
platform matrix found a Windows failure nobody had predicted. **What it does not do is cover half the
seam**, and the half it misses includes the arm this project has already shipped a wrong gloss from.

**Widening is cheap, and that is measurable rather than hopeful.** The 0.5.0 prep run took 64s
(ubuntu), 74s (macOS), 116s (Windows) per platform — **dominated by two `cargo install prikk`
invocations**. The test body itself ran 3.25s for ten fixture repositories. The expensive part is
already paid; more surfaces cost seconds.

## Findings

### F1 — it is five of ten, not four of nine, and the published number is mine to correct

The `Prikk` trait has **ten** methods. The suite exercises **five**:

| Exercised | How |
|---|---|
| `handshake` | the version guard calls `Prikk::handshake` on every test at both ends |
| `commit`, `seal` | directly, with repository state asserted after |
| `orientation`, `history` | re-read after mutation to confirm the state |

| Unexercised |
|---|
| `worktree_status`, `block_state`, `refs`, `tags`, `change_token` |

**`handshake` was covered all along and nobody counted it.** I wrote *"four of nine"* into the 0.5.0
proposal, asked the dev team to state it in the changelog, and they did — so **the shipped 0.5.0
changelog understates the suite's coverage.** It is modesty rather than a false claim, so it is not a
patch release; it is corrected here and in 0.6.0's entry, and recorded as mine.

### F2 — three of the four clauses behind "another writer is active" have never been seen *[the sharp one]*

RFC 017 F4 narrowed `is_lock_conflict` from a prefix match to four semantic clauses, each naming a real
lock. **Only one of the four is live-captured:**

| Clause | Provenance | Can it be provoked? |
|---|---|---|
| `lock already exists` | **captured live** — a pre-placed `active.lock` | yes, and is |
| `belongs to a different repository authority` | source-read (`lock.rs`) | needs a second repository authority |
| `cas mismatch` | source-read (`refs.rs`) | **no — prikk says their guard cannot fire** |
| `changed during planning` | source-read (`rollback_draft.rs`) | needs an in-flight rollback draft stikk does not build |

**This is the arm whose gloss is `FR-106`'s "another writer is active"** — the claim that was false on
the full-queue path, rendered above prikk's own words, and that RFC 017 existed to remove. Three
quarters of it rests on reading prikk's source.

**And prikk told us one of those clauses is dead on their side**, in their reply to letter 004:

> while writing a test for one of the four that stay, we found our ref CAS guard **cannot fire through
> the publish path at all** — the same comparison is already established under locks held across both
> reads. It stays, as defence against a future locking regression, and it is now documented as such
> instead of reading like a live gate.

So stikk carries an arm for a message that cannot currently arrive, matching a guard its author
documents as dormant. **That is not RFC 017 F2's defect** — the string exists and is citable, unlike the
five arms that matched prose nobody ever emitted. But it is not a captured arm either, and RFC 017
decision 1 said *"no arm survives without a captured message behind it."* **We have been running three
arms on citations for a release.** See Q1.

### F3 — the suite cannot tell a broken harness from a broken product

RFC 021's Windows failure produced **five identical panics** — `prikk key generate (author) failed at
0.38` — because every test builds its own fixture repository first. One broken setup step, five red
tests, no signal distinguishing "the harness could not construct a repository" from "stikk got the
answer wrong."

**At five surfaces that is tolerable noise. At ten it is a misleading picture of a release's health**,
on a gate whose entire purpose is telling a human whether to ship.

### F4 — per-test isolation is worth keeping, and now there is a number

Ten fixture repositories for five tests (each test, both versions) cost **~0.3s each**. A shared
fixture would couple tests — one test's mutation changing another's preconditions — for a saving
invisible next to `cargo install`. **Keep isolation; say why, with the measurement**, so the next person
tempted to optimise it finds the reason rather than the opportunity.

### F5 — both dispatch-only workflows will fail at checkout, on nobody's schedule

`actions/checkout@v4` is being force-run on Node 24 with a deprecation notice on every job of both
workflows. When the forcing stops they fail **at checkout**, before anything runs — and these are
precisely the two workflows nobody triggers between releases. **The failure would land at a release, on
a gate that blocks it.** Cheap to fix while the file is open.

## Decisions

1. **Drive the five unexercised surfaces** against real binaries at both ends, and **assert state, not
   parse** — RFC 019's discipline, which is the whole reason the suite found anything: `refs`/`tags`
   after a real `branch create`/`tag create`; `block_state` on a sealed block; `change_token` across a
   mutation that must move it and a read that must not; `worktree_status` on a dirty tree.
2. **`worktree_status` at 0.38 carries F0's own regression test.** The fabricated-entry case is
   currently pinned by a captured fixture only. Provoking `prikk mv` of a change-kind-named path
   against a real 0.38 binary makes the suite the guard — **this is the single highest-value addition**,
   because it is the one defect of this class that reached a shipped release.
3. **Provoke every classifier arm that can be provoked; for the rest, the arm says why not.** An arm
   surviving on a citation carries, in the code, the reason it cannot be reached and what would change
   that — checked each re-baseline like a fixture's provenance. A citation without a reachability note
   is the thing RFC 017 was about, one step removed.
4. **Separate harness failure from product failure** (F3). Fixture construction failing is not a test
   failure about stikk; it should say so, once, rather than N times.
5. **Keep per-test isolation**, with F4's measurement in the comment.
6. **Pin `actions/checkout` to a release that does not depend on forced Node** (F5).
7. **Correct the coverage claim** (F1) in 0.6.0's changelog, naming that 0.5.0's understated it.

## What this RFC does not do

**It does not widen the supported-version matrix.** Both ends, as now. Exercising the middle is a
different question with a different cost curve, and nothing has asked for it.

**It does not add a `schedule:` trigger.** Still deliberately the owner's, on both workflows.

**It does not chase coverage for its own sake.** Three arms are unreachable for reasons that are not
stikk's to fix, and Decision 3 is honesty about that rather than a plan to reach them.

## Q1 — RULED by the architect, 2026-09-12: **(a)**, on a stronger basis than the question assumed

**The owner accepted without answering, and the question did not need them**: I said the answer depends
on whether RFC 017 Decision 4's binding on `FR-106`'s gloss holds in the current code. It does — and the
reason is better than the binding.

**There is no gloss.** `present()`'s arm is:

```rust
StikkError::LockConflict { message } => Presentation::Banner {
    message: message.clone(),   // prikk's own words, verbatim
    jump: None,
},
```

**stikk adds nothing to a lock conflict.** Grepped every occurrence of *"another writer is active"* in
the workspace: all of them are comments narrating RFC 017's history, one stale doc comment (F6 below),
and **two test assertions checking its absence** (`present/tests.rs:344`, `:379`). The only places
"another writer" reaches a user are fixtures where **prikk's own message** contains it — and those
assertions check prikk's words survive, which is `ER-02`, not a stikk claim.

**So (c)'s premise is gone.** I wrote that deleting all three would be *"the only safe answer"* if the
gloss could assert a writer without evidence. It cannot, because it asserts nothing. The worst case from
keeping an unreachable arm is prikk's verbatim message rendered as a banner rather than a refusal card —
no fabricated claim, either way.

**(a) stands, with Decision 3's reachability note**, and the asymmetry in the original lean is now
decisive rather than arguable: keeping costs nothing until the condition arrives; deleting costs correct
classification if prikk's locking ever regresses, which is exactly the scenario their dormant guard
exists for.

### F6 — found while ruling this: the taxonomy's own doc comment is stale

`stikk-model/src/error.rs:29` says `LockConflict` is *"Presented as 'another writer is active', never as
corruption (design FR-106)."* **It is presented as prikk's verbatim message in a banner.** The sentence
was true when written and describes a presentation that no longer exists — the F1 shape, in the error
taxonomy's own documentation, and the source of my own wrong premise in Q1. **Fix it in this increment.**

### The original question, for the record

**Q1 — does RFC 017's rule mean *captured*, or *citable*?** I wrote *"no arm survives without a captured
message behind it"*, and three arms have survived a release on source citations. The sharpest case is
`cas mismatch`, which prikk documents as dormant.

- **(a) Keep all three, each carrying its reachability note** (Decision 3 as written). If prikk's
  locking ever regresses, their guard fires and stikk classifies it correctly rather than showing a bare
  refusal. Symmetry: prikk keeps the guard as defence in depth; stikk keeps the arm for the same reason.
- **(b) Delete `cas mismatch` specifically**, since its author says it cannot fire, and keep the other
  two. Narrowest reading of RFC 017.
- **(c) Delete all three.** Strictest reading: an arm nobody has ever seen fire is a hypothesis, and
  `FR-106`'s gloss asserting an active writer is exactly the claim this project has already got wrong
  once.

**My lean: (a), and the reachability note is what makes it defensible rather than lazy.** A deleted arm
degrades to a verbatim refusal — safe, which is why deletion was right for the five invented arms. But
those matched text prikk never emitted; these match text prikk emits from code paths that exist. **The
asymmetry that matters: deleting costs us correct classification if the condition ever arrives; keeping
costs us nothing until it does, provided the gloss is honest.** `FR-106`'s gloss is the real exposure,
and RFC 017 Decision 4 already bound it — *may not assert another writer unless stikk has evidence of
one*. **If that binding holds, (a) is safe; if it does not, (c) is the only safe answer.** Verifying
which is true of the current code is the first thing this increment should do, and it is why I am not
ruling this myself.


## Carried forward, recorded at completion 2026-09-12

- **A gloss for prikk 0.28's Windows subdirectory refusal.** A Windows user on the supported floor gets `invalid name: backslashes are not allowed in repository paths` for a path they never typed a backslash into. Honest and baffling. `present()`'s message-shape recognition is the mechanism; its own increment, because this one forbade behaviour changes.
- **An audit of every GitHub action's runtime**, written down once, so the next deprecation is a lookup rather than a survey. `actions/checkout` is pinned to `v5` across all five workflows; nine other actions are unaudited and named in the review request.
- **Three `is_lock_conflict` clauses remain source-read**, each now carrying a reachability note saying what would make it provokable. `changed during planning` is to be revisited **with** the Rollback increment, not after it.

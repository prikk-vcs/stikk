# RFC 015 — The prikk 0.32 re-baseline: `UD-01` retires and `UD-09` narrows

**Status.** Accepted (2026-09-06, by the owner) — handoff:
[`../handoffs/015-prikk-0-32-rebaseline/rebaseline-handoff-v1.md`](../handoffs/015-prikk-0-32-rebaseline/rebaseline-handoff-v1.md).
Originally proposed 2026-09-06 — re-baseline on prikk **0.32**, retire **`UD-01`**, narrow
**`UD-09`**, and stop dropping information prikk now gives us. Not a version bump: two upstream
dependencies this project has carried since 0.1.0 have changed, and one of them unlocks a surface
RFC 006 deferred.
**Tracks.** `ASM-2`/`NFR-R03` (version honesty), `UD-01` (messages), `UD-09` (patch enumeration),
`FR-011`/`FR-012` (per-patch display and filtering), and the standing obligation to re-validate.
**Touches.** `stikk-prikk` (the `log` parser and its fixtures, a new refusal shape, the version
ceiling), `stikk-core` (the history view-model), `stikk-tui` (History rendering), and the design set
(`UD-01`, `UD-09`, `FR-011`, and the commit copy).

## Summary

RFC 014's implementer found, while building commit, that prikk 0.32 had retired the very note stikk
was told to transport. Following that up found considerably more: **`prikk log` now prints a line per
patch, carrying the patch id and its message.** RFC 006's founding finding — *"`log` carries no
per-patch detail, no patch ids"* — has stopped being true, and stikk is currently discarding both.

That is the RFC 009 F4 mistake in a new place: prikk supplies information, stikk drops it silently.
Nothing is *wrong* on screen today, which is exactly why it needs an increment rather than a bug fix.

## The findings

All verified against prikk 0.32.0 (the released tag), not its `main`.

### F1 — `log` now enumerates patch ids and messages

`print_history` at `0.32.0` emits, between `required-attestations:` and `previous-ref-state:`:

```
  patch <64-hex patch id>: <message>
```

one line per patch **that carries a message**. `-m` has always been mandatory, so every patch authored
by 0.32+ has one.

**stikk does not break on this, and that is the problem.** `parse::history` appends unrecognized lines
to the block's group and looks fields up by prefix, so the new lines are silently ignored. A message
also cannot forge a field, because every one is prefixed `patch <id>: ` and therefore never *starts*
with a label — so this is safe, just deaf.

### F2 — `UD-01` retires, and the shipped copy is now false

Messages are stored (schema 4, tag 6), shown in `log`, and prikk's *"validated but not stored"* note is
gone from its source. stikk's commit overlay still says *"core does not yet persist it"* — false for
anyone on 0.32.

### F3 — `UD-09` **narrows**; it does not retire

Precision matters here, because it is tempting to over-read F1. `UD-09` covered three things:

| | Status |
|---|---|
| patch-**id** enumeration within a block | **Retired** for messaged patches (F1) |
| per-patch **content** (`show`/`diff`, operations, preimages) | **Still absent** |
| `log --format json` | **Still absent** |

So RFC 006's increment 3b (Patch detail as a *diff*) stays blocked; what becomes possible is naming
patches, not rendering them. **Do not let this be read as "Patch detail is unblocked."**

### F4 — the enumerated list and the patch count can legitimately disagree *[the honesty risk]*

A patch written before 0.32 carries no message and therefore **contributes no line** — prikk's own
comment says *"absence is the truth, not a placeholder."* So a block can report `patches: 3` and emit
one `patch …:` line.

If stikk renders the list without the count's context, a user reading a repository with history from
both sides of the upgrade sees a block that **appears to contain one patch when it contains three**.
That is a `T-T4` confident-but-wrong picture, manufactured by stikk out of correct prikk output.

### F5 — a second, different skew refusal shape

0.32 is forward-incompatible like 0.31 (schema 4 now). The repository-level refusal keeps the shape
RFC 012 already glosses (`does not accept envelope schema 4`), but a **bundle** offered directly
refuses earlier and differently:

```
error: malformed persisted data: invalid PatchPurpose canonical form: canonical encoding error: unknown PatchPayload field tag: 6
```

Nothing in stikk recognizes that. It degrades to a verbatim `Refusal` — safe, unglossed.

### F6 — the reclassification is coming, but is not here yet

Upstream RFC 132 (prompted by our 2026-09-05 report, landed within a day) reclassifies RFC 014's two
refusals to `precondition not met:`. It is **7 commits past the `0.32.0` tag — unreleased.** RFC 014's
classifier already matches the stable part of both messages and needs no change. Recorded so this
increment does not "fix" something that is not broken, and so the next re-baseline knows to check it.

## Decisions

1. **Parse the `patch <id>: <message>` lines** into the block row, id and message both, with fixtures
   **captured** from 0.32 per RFC 009 §0.
2. **Render them in History, always with the count**, and state plainly when they disagree: *"3 patches;
   1 has a message — patches written before prikk 0.32 carry none."* Never render the list alone
   (F4). The list is not the block's contents; it is the subset prikk can name.
3. **Messages are repository content**: `inert` at every call site (`C-T2a`), and a message is a
   *display string*, never a next-step, never a glossary trigger (`C-T2b`).
4. **Retire `UD-01`** in the design set, and correct the commit overlay copy. State the range honestly:
   messages persist on prikk ≥ 0.32 and are discarded below it — stikk supports both, so the copy is
   version-conditional, not a blanket claim in either direction.
5. **Narrow `UD-09` precisely** (F3) rather than retiring it, and restate what RFC 006 3b still needs.
6. **Gloss the bundle-decode skew shape** (F5) alongside RFC 012's repository-level one.
7. **Raise the validated ceiling to 0.32** — only after re-capturing every fixture against it, per
   RFC 009's rule. `FR-012`'s message filter is *not* built here; it becomes possible, and it is its
   own increment.

## Upstream dependency

**`UD-01` closes.** `UD-09` narrows to *content* plus `--format json`; both remain filed, and the
content surface stays stikk's highest-value ask — it is what still blocks Patch detail and Compare.

RFC 013's queued-patch enumeration ask (for the *seal* ceremony) is untouched by this: `status` still
reports the queue as a count and a target ref. Sealed patches are now nameable; queued ones are not.

## Open questions — one settled, one deliberately left as work

**Q1 — History list, Block detail, or both?** **Ruled: Block detail carries the list; History keeps
the count as it is.** A block holds N patches, so a History row would either show N messages (and stop
being a scannable lineage — `NFR-U01`, `TU-12`) or show one and pick arbitrarily. Block detail is
already where a block's contents live, and it is the view whose *"per-patch content inspection awaits
prikk support (UD-09)"* note has become partly false and must be corrected anyway.

A message summary *in the History row* is deliberately deferred, not rejected: it is the thing that
would make a lineage of hex ids readable, but any summarization rule (first message? most recent?)
would be invented against no real multi-patch blocks. Revisit with repositories in front of us.

**Q2 — what does a repository straddling the upgrade actually look like?** **Deliberately not settled
here.** F4 is reasoned from prikk's source, not observed, and it is the finding the user-facing copy
depends on. It is *work for the increment*, not a decision for me: build the block, look at it, then
write the copy. See the handoff §2.

## Consequences

- stikk stops discarding evidence prikk now provides, which is the specific failure RFC 009 F4 cost a
  release to learn.
- A dependency carried since 0.1.0 closes, and the design set stops asserting something false.
- `UD-09` gets an honest, narrower statement instead of a stale blanket one — and the *content* half is
  named clearly enough that nobody reads F1 as more than it is.

# RFC 013 — The preview and tiered-confirmation machinery

**Status.** Implemented (0.4.0 candidate; on `main` 2026-09-05, reviewed and approved) — handoff:
[`../handoffs/013-preview-and-confirmation-machinery/preview-confirm-handoff-v1.md`](../handoffs/013-preview-and-confirmation-machinery/preview-confirm-handoff-v1.md).
Originally proposed 2026-09-05 — build the gate every mutation in 0.4.0 sits behind: preview-first
made **structural rather than disciplined**, confirmation tiers derived from the request category, and
the preview↔execute binding on RFC 003's change token. **No mutation ships in this increment.**
**Tracks.** `FR-120` (preview-first), `FR-121` (confirmation tiers), `OPL-01…05`, `TU-09` (the
confirmation overlay), `CT-05`, `CC-02`, and the threat model's `C-T4a…e` — the controls that have so
far only had read surfaces to protect.
**Deferred, carried forward (not built by this RFC):** the `capability_gate` / command-palette
**unification** — `OPL-04` names `capability_gate` as the frontend's UI-affordance check, but RFC 007's
palette still greys out on a bare `Capability` with no tier awareness, so the two could disagree once a
mutating command exists (the palette would offer what `confirm` refuses under read-only). **Must land
before the first mutating command enters the palette registry** — the commit increment. Also deferred:
the ceremony state machines (`OPL-03`), and the seam-side half of `OPL-04`'s double check, which arrives
with the first real mutation.
**Touches.** `stikk-model` (a tier vocabulary, one new error class), `stikk-core` (`confirm`,
`capability_gate`, the `PreviewToken`), `stikk-tui` (the `TU-09` overlay and the `InConfirmation`
renderer RFC 007 deferred). No seam method; no mutation.

## Summary

Everything stikk has shipped so far is a read. The design set has always specified the machinery that
gates mutations — preview-first, three confirmation tiers, a preview bound to the state it was computed
under — and none of it exists. This RFC builds it **before** the first mutation, so that commit and seal
arrive into a gate rather than bringing one with them.

The load-bearing decision is that **preview-first should be impossible to bypass, not merely required**.
`FR-123`'s frontend parity is guaranteed because neither frontend can define an operation; `FR-120`
should be guaranteed the same way — by making `execute` unable to be called without a value only a
preview can produce.

Two findings from checking prikk (below) shape what the machinery must be honest about, and one of them
means a requirement is **not satisfiable as written**.

## The findings

### F1 — prikk previews five mutations and not the two we ship first

Verified against prikk 0.31:

| Has a prikk-side plan | Has none |
|---|---|
| `checkout --plan-only` / `--snapshot-plan` / `--patch-plan` / `--patch-delete-plan`, `merge-evidence`, `merge-plan`, `inverse-plan`, `rollback-preview`, `compact --plan-only` | **`commit`**, **`seal`** |

This splits previews into two classes that must never be blurred, because `C-T4a` says the user
confirms *prikk's plan*, not stikk's summary of it:

- **Class A — prikk's own plan, rendered verbatim.** Everything in the left column. stikk transports it;
  it does not paraphrase, summarise, or "improve" it.
- **Class B — stikk-composed, and labelled as such.** `commit` and `seal`. stikk derives what it can and
  must say whose derivation it is.

**Class B is not automatically dishonest**, and for `commit` it is well-founded: prikk's own
`worktree-status` guide states the command *"answers 'what would the next commit author?', not merely
'what differs from the last seal.'"* — the same replay baseline `commit` authors against. The Changes
view is therefore a legitimate commit preview, and we can say so with a citation rather than a hope.

### F2 — `FR-052`'s seal ceremony is not satisfiable as written *[the important one]*

`FR-052` requires the ceremony to show *"exactly which patches will seal into one block on which ref."*
**stikk cannot show which patches.** prikk exposes the queued **count** and its **target ref**
(`status`: `queued patches: N targeting <ref>`) and nothing that enumerates queued patch ids —
`sync pending` lists *accepted* (received) patches, a different thing, and RFC 006 recorded the same
ceiling for the History view's queue tier.

So the honest seal ceremony can state: how many patches, which ref they target, and what block they
will become. It cannot list them. **This is exactly the shape of gap this project refuses to paper
over** — and finding it now, rather than during the seal increment, is why the machinery is being built
first.

## Decisions

1. **Preview-first is structural.** `execute` takes a `PreviewToken` that only a preview operation
   produces and that is **consumed** on use. There is no code path from an intent to an execution that
   does not pass through a preview, because the type system does not offer one. `FR-120` stops being a
   rule reviewers must remember.
2. **The token binds to the change token** (`OPL-02`). It carries the `ChangeToken` (RFC 003) current
   when the preview was computed, plus the intent it previewed. Execution re-reads the change token and
   **refuses on any difference** — the user re-previews. This is the mechanism that makes "another
   writer moved the ref between your preview and your click" safe, and it is why RFC 003 came first.
3. **A new error class, `StikkError::Stale`**, for that refusal. Not `Refusal` (prikk did not refuse —
   stikk did), not `LockConflict` (nothing is locked). `present()` routes it to a re-preview prompt, and
   **never to a retry**: retrying an execution whose preconditions moved is precisely `NFR-S04`'s
   prohibition. `StikkError` is `#[non_exhaustive]`, so adding the variant breaks no matcher.
4. **Tier is derived from `RequestCategory`, never declared per operation.** A new operation cannot
   forget to be gated, the same mechanical guarantee as `FR-123`:

   | Category | Tier |
   |---|---|
   | `ReadHistory`, `ReadState`, `WorktreeAnalysis`, `Integrity` | **1** — free, no confirmation |
   | `QueueMutation` | **2** — explicit yes |
   | `Publication`, `Exchange` | **3** — restate, then explicit yes |
   | `Trust`, `Recovery` | **3-typed** — restate, then **type the target name** (`FR-102`/`FR-103`) |

5. **Read-only is enforced in the tier machine and re-checked at the seam** (`OPL-04`'s double check),
   using `Readiness::may_operate` — which RFC 012 F-a moved to `Readiness` precisely because
   `Capability` had already discarded the fact it needs. Tier 1 is unaffected; tiers 2–3 are refused
   outright under `STIKK_READ_ONLY=1` (`NFR-S01`, `FR-121`).
6. **Tier 1 stays free, and this is a standing constraint, not a default.** `RR-6` records confirmation
   fatigue as a residual risk; a confirmation on a read is how tier 3 stops being read. No read surface
   gains a confirmation, ever, without amending `FR-121`.
7. **`CC-02`'s per-repository mutation gate is already satisfied** by RFC 010's single worker: one
   worker means one seam call in flight, so mutations serialize structurally. Record it; do **not** build
   a second mechanism. If the worker ever becomes a pool, this is the constraint that must be preserved.
8. **No mutation ships here.** The machinery is built and proven against a scripted, non-mutating
   operation exercising every tier. But it is **designed against three concrete consumers** — commit
   (tier 2, Class B preview), seal (tier 3, Class B, plus the F2 ceiling and the no-audit consent step),
   and checkout (tier 3, Class A) — so it is not an abstraction built for imagined needs.

## Upstream dependency

**A queued-patch enumeration surface** (F2). `prikk status` reports the queue's size and target; nothing
lists the queued patch ids. It blocks `FR-052`'s "exactly which patches" and RFC 006's queue tier.
The clean ask: patch ids in `status`, or a `--format json` on it. Recorded alongside `UD-09`'s content
surface, and **lower priority than it** — the seal ceremony degrades honestly to count-and-target,
whereas UD-09 blocks whole features.

Until it lands, the seal ceremony states the count, the target ref, and the resulting block, and says
plainly that stikk cannot enumerate them — the `UD-09` pattern applied to a ceremony.

## Open questions — all three settled at acceptance (2026-09-05)

**Q1 — does the `PreviewToken` carry the rendered preview, or only its identity?** **Ruled: neither, and
the question was framed wrongly.** Separate two things it conflated:

- **The preview** — a `ChangesView`, prikk's plan text, a merge evidence report — is large, per-operation,
  and *the user is looking at it*. It stays in the view.
- **What the confirmation restates** is small and uniform, and `TU-09` already specifies it exactly:
  the operation, the target ids, the counts, the capability consumed, and the consequence class.

So the token carries a **`ConfirmationSummary`** — that fixed set, composed at preview time from
authoritative values — and the confirmation renders from it. This satisfies `C-T4e` (a confirmation
restates prikk-authoritative values, never attacker-supplied display strings) without making the token
generic over every preview type, and without re-deriving anything at confirm time, which would
reintroduce exactly the drift the token exists to prevent.

**Q2 — where does the ceremony state machine live (`OPL-03`)?** **Ruled: not built here, and the token
is deliberately shaped so it never needs to span one.** A `PreviewToken` is **single-use and
step-scoped**. A multi-step ceremony (seal, rollback, the sync assistant) is *N gated steps*, each with
its own preview→confirm→execute and its own freshness check — never one token held across the whole
ceremony. A ceremony spans user think-time by design; a token stamped at step one and executed at step
four would assert a freshness it does not have, which is the precise failure `OPL-02` exists to prevent.

**Q3 — should tier 3-typed apply to `seal`?** **Ruled: no** — accepted by the owner with the
recommendation. `FR-121` reserves typed confirmation for lock clearing and trust changes; seal already
carries its own unchecked-by-default no-audit consent step (`FR-052`). Two distinct deliberate acts is
enough, and a third is the fatigue `RR-6` names. Cheap to revisit if sealing turns out to feel too easy
in practice.

**A decision that followed from Q1/Q2, and goes further than the RFC as proposed:** make **confirmation**
structural too, not just preview. `preview()` yields a `PreviewToken`; `confirm()` consumes it and
yields a **`ConfirmedToken`**; `execute()` takes only a `ConfirmedToken`. Since `confirm` is the only
producer of the latter, and it is where the tier's evidence (an explicit yes, or the typed target name)
and the read-only/capability checks live, **an execution that skipped confirmation is as unrepresentable
as one that skipped the preview.** Decision 1 said preview-first should not be a rule reviewers
remember; the same argument applies to the confirmation, and costs one more type.

## Consequences

- Commit, seal and every later mutation arrive into a gate they cannot bypass, rather than each
  re-implementing one.
- `Presentation::InConfirmation` — defined by RFC 007 and rendererless since — finally has a renderer.
- `FR-052` is known to be unsatisfiable as written **before** anyone builds the seal ceremony against
  it, and the requirement will be amended to what is honest rather than quietly under-delivered.
- The threat model's `C-T4a…e` stop being controls over read surfaces and become the thing standing
  between a user and a real prikk mutation. That is the whole reason this increment precedes the
  mutations it gates.

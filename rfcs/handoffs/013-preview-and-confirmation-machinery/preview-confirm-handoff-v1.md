# Handoff — the preview and tiered-confirmation machinery (v1)

**Companion to:** [RFC 013](../../accepted/013-preview-and-confirmation-machinery.md) (Accepted
2026-09-05, by the owner). Inherits its state.
**Realizes:** 0.4.0's second increment, after [RFC 003](../../done/003-repository-change-token.md)'s
change token, which this consumes. **The gate every later mutation sits behind.**
**Design items:** `FR-120` (preview-first), `FR-121` (tiers), `TU-09` (the confirmation overlay),
`OPL-01…05`, `CT-05`, `CC-02`, `NFR-S01`/`NFR-S04`, and the threat model's `C-T4a…e`.

> **No mutation ships in this increment.** You are building the gate, not walking through it. If you
> find yourself adding a `Prikk` method that writes, stop — that is the commit increment.

**Read RFC 013's F2 before starting.** Drafting it found that `FR-052` is not satisfiable as written;
that is not this increment's problem to fix, but it is the reason the gate is being built first.

---

## 1. Scope

**In:**
1. `PreviewToken` → `ConfirmedToken` (§2) — the two-step type chain that makes preview-first *and*
   confirmation structural.
2. `ConfirmationSummary` (§3) — the `TU-09` fact set, composed at preview time.
3. Tier derivation from `RequestCategory` (§4), with read-only and capability enforcement.
4. `StikkError::Stale` and its `present()` routing (§5).
5. The `TU-09` confirmation overlay and the **`Presentation::InConfirmation` renderer** RFC 007 defined
   and left rendererless (§6).
6. A **scripted, non-mutating** operation exercising every tier, as the test vehicle (§7).

**Out:** any real mutation or writing `Prikk` method; the seal/rollback/sync **ceremony state machines**
(`OPL-03` — RFC 013 Q2 rules the token shape so they are not needed yet); `DerivedViewCache`; the
action-id catalog (RFC 002, still proposed).

---

## 2. The two tokens — the whole point of the increment

```rust
// Produced only by a preview. Single-use, step-scoped.
pub struct PreviewToken { /* intent, ConfirmationSummary, ChangeToken, tier */ }

// Produced only by `confirm`. The only thing `execute` accepts.
pub struct ConfirmedToken { /* … */ }
```

- `preview(prikk, intent) -> Result<(PreviewView, PreviewToken)>`
- `confirm(token, evidence) -> Result<ConfirmedToken>`
- `execute(prikk, confirmed) -> Result<Outcome>`

**Neither token has a public constructor**, and no `From`/`Into` shortcut between them. `execute` takes
`ConfirmedToken` **by value**, so it cannot be replayed. That is the guarantee: an execution that
skipped the preview, or skipped the confirmation, is **unrepresentable** — not merely forbidden. If you
find yourself needing a `PreviewToken::new()` for a test, that is a signal the test should go through
`preview()` with a scripted backend, not a signal to add the constructor.

**`ChangeToken` is checked twice**, and both matter: once in `confirm` (cheap, catches the world moving
while the user read the preview) and once at the top of `execute` (the real guard, immediately before
the seam call). A difference at either point is `StikkError::Stale` (§5) — never a retry.

**Step-scoped, not ceremony-scoped** (RFC 013 Q2). A token is for one preview→confirm→execute step. A
multi-step ceremony will be N of these. Do not add a way to hold one open across steps.

---

## 3. `ConfirmationSummary` — what the confirmation restates

`TU-09` specifies it exactly, so this is a fixed, uniform struct, not a per-operation blob:

- the **operation** (human name),
- the **target ids** and **counts** — from prikk-authoritative values, never from display strings a
  repository could influence (`C-T4e`),
- the **capability consumed** (`AUTHOR`/`MAINTAINER`/operator),
- the **consequence class** — in stikk's own words: what becomes permanent, and what does not.

Composed at **preview time** and carried in the token. **Do not re-derive it at confirm time** — that
reintroduces exactly the drift the token exists to prevent (RFC 013 Q1).

Every string in it is stikk-authored or a prikk-authoritative id. Route ids through `inert` at render
(`C-T2a`) as everywhere else.

---

## 4. Tiers, read-only, capability

Derive the tier from `RequestCategory` — **never** a per-operation declaration (RFC 013 decision 4), so
a future operation cannot forget to be gated:

| Category | Tier | Evidence `confirm` requires |
|---|---|---|
| `ReadHistory`, `ReadState`, `WorktreeAnalysis`, `Integrity` | 1 | none — these never reach `confirm` |
| `QueueMutation` | 2 | an explicit yes |
| `Publication`, `Exchange` | 3 | restate, then an explicit yes |
| `Trust`, `Recovery` | 3-typed | restate, then **the typed target name**, matched exactly |

Put the mapping on `RequestCategory` itself, beside `mutates()` — it is category policy as data, which
is what `CT-03`/`AR-05` already made that type for.

**`confirm` is where the gates live** (`OPL-04`'s first check):

- **read-only refuses tiers 2–3 outright** (`NFR-S01`, `FR-121`), via `Readiness::may_operate` — the
  method RFC 012 F-a moved to `Readiness` precisely because `Capability` had discarded the fact it
  needs. Refuse with `NotReady`, not `Stale`.
- **capability** must satisfy the tier (`AUTHOR` for 2, `MAINTAINER` for 3's publications).
- The seam re-checks before the mutating call (`OPL-04`'s second check) — that half arrives with the
  first real mutation; leave the hook obvious.

**Tier 1 must stay free** (RFC 013 decision 6). No read gains a confirmation. `RR-6` records
confirmation fatigue as the residual risk that makes tier 3 stop being read.

---

## 5. `StikkError::Stale`

A new variant on the `#[non_exhaustive]` enum — adding it breaks no matcher. It carries what moved
(operation, and that the repository changed), not prikk's words: **prikk did not refuse; stikk did.**

`present()` routes it to a **re-preview prompt** — the user's next action is to look again, not to try
again. Explicitly **not** a retry, and not an auto-retry (`NFR-S04`, `C-E1`): a `NextStep` that re-runs
the *preview* is correct; one that re-runs the *execution* is the prohibited thing.

Do not reuse `Refusal` (it would put stikk's words in prikk's voice, breaking `ER-02`'s premise) or
`LockConflict` (nothing is locked; `FR-106`'s "another writer is active" is a different situation).

---

## 6. The `TU-09` overlay, and `InConfirmation`

`Presentation::InConfirmation` has existed since RFC 007 with no renderer. Give it one, and give tier 3
its restate-then-confirm shape and tier 3-typed its exact-match text entry (reuse the palette's
text-entry plumbing from RFC 007).

The overlay renders **from the `ConfirmationSummary`**, never from a re-read. Chrome stays visibly
stikk's (`C-T2b`): a content pane must not be able to look like a confirmation dialog.

Cancelling is free and always available, and leaves **no** trace of an intent to execute.

---

## 7. Test plan — this is a safety gate, so test the negatives hardest

- **Structural, and worth stating in the review request:** confirm by inspection that `execute` cannot
  be reached without both tokens, and that neither has a public constructor. A test cannot prove this;
  the type signatures can.
- **Staleness**: a change token that moves between preview and confirm → `Stale`; between confirm and
  execute → `Stale`. Both, separately.
- **`Stale` never yields a retry step** — assert the next-step set for `Stale` contains no action that
  re-runs the execution. This is the `NFR-S04` regression test for this increment.
- **Read-only**: with `STIKK_READ_ONLY=1`, tier 2 and tier 3 both refuse at `confirm`, with and without
  keys present. Tier 1 is unaffected.
- **Capability**: tier 3 refuses at `Author`; tier 2 refuses at `Viewer`.
- **Tier 3-typed**: a wrong name, a near-miss, and empty input all refuse; only an exact match passes.
- **Single-use**: a `ConfirmedToken` is consumed by `execute` and cannot be reused (by type, not by a
  runtime flag — if you need a flag, the type is wrong).
- **`C-T4e`**: a `ConfirmationSummary` built from a scripted backend whose ref name carries control
  characters renders inert and cannot forge chrome.
- Gates green; state the count delta.

---

## 8. Acceptance criteria

1. `execute` takes a `ConfirmedToken` by value; `ConfirmedToken` is produced only by `confirm`;
   `PreviewToken` only by `preview`; neither has a public constructor. **Preview-first and
   confirmation are structural, not documented.**
2. The change token is checked in **both** `confirm` and `execute`; either mismatch is `Stale`.
3. `Stale` exists, is routed by `present()` to a re-preview, and **no next-step re-runs an execution**.
4. Tier derives from `RequestCategory`; read-only refuses tiers 2–3 at `confirm`; capability is enforced
   per tier; tier 1 is unchanged and free.
5. `Presentation::InConfirmation` has a renderer; tier 3-typed requires an exact match.
6. **No mutation was added** — no `Prikk` method writes, and the test vehicle is a scripted no-op.
7. Gates green; count delta stated; all four demos build.
8. Nothing tagged, pushed, or published.

---

## 9. Submit

Package to `.git-exclude/review-request/013-preview-and-confirmation-machinery/review-request-v1.md`.

Call out: **the type signatures of `preview`/`confirm`/`execute` verbatim** — they are the deliverable,
and a reviewer should be able to see the guarantee in them without reading a body. Plus the count delta,
and anything you found that contradicts RFC 013. Every increment for five running has turned up
something the RFC got wrong; assume this one has too and go looking.

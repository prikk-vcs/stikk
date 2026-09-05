# Handoff — commit, the first mutation (v1)

**Companion to:** [RFC 014](../../accepted/014-commit-the-first-mutation.md) (Accepted 2026-09-05).
Inherits its state.
**Realizes:** `FR-050` / `FL-05` — the first operation stikk performs that changes a repository,
arriving into the gate [RFC 013](../../done/013-preview-and-confirmation-machinery.md) built.
**Design items:** `FR-050`, `FL-05` (its step order is normative — see §3), `FR-121` tier 2,
`UD-01`/`UD-06`/`UD-08`, `C-D2a`, `C-T4a…e`, `OPL-04`.

> **stikk has never written to a repository before.** Everything up to now has been a read, and every
> invariant this project holds has so far only had reads to protect. Build accordingly: the gate is
> already there and cannot be bypassed, so the work is making the *preview* honest and the *refusals*
> preventable.

---

## 1. Scope

**In:**
1. `Prikk::commit` — the first seam method that writes (§2).
2. The commit operation: preview (Class B, re-read), message step, tier-2 confirmation, execute (§3).
3. **Preventing** the two refusals prikk would otherwise give us (§4) — cross-ref, and clean worktree.
4. The active-patch threshold warning **in the preview** (§5).
5. The `capability_gate`/palette unification — RFC 013's deferral, due exactly here (§6).
6. `OPL-04`'s seam-side re-check (§7), built honestly.

**Out:** queue review (`FR-051`) and the seal ceremony (`FR-052`) — the next increments; anything
`--text-edits` (RFC 014 F1: it is a no-op, do not offer a control that changes nothing); RFC 002's
action-id catalog (§6 is the narrow fix, not the catalog).

---

## 2. The seam grows a writing method

```
prikk commit --from-worktree --ref <reff> -m <message>
```

Category **`QueueMutation`** — which is what makes it tier 2 (RFC 013 decision 4), so do not declare a
tier anywhere.

**Parse the success output** into a result carrying prikk's own facts, captured per RFC 009 §0 (a
**real** capture, with a provenance line):

```
recorded worktree patch in active WAL
baseline ref: heads/main
patch id: <64-hex>
WAL sequence: <n>
operations: <n>
referenced blobs: <n>
text edits: <n>
  create-file <path>
  …
note: multi-operation text diff minimization, … remain later increments
note: the message is validated but not stored -- it will not appear in `prikk log`; …
```

**Carry both `note:` lines verbatim** (`ER-02`). The message-fate note is prikk saying `UD-01` in its
own words, which is better copy than ours and costs nothing to transport. Do not paraphrase, do not
suppress.

**This method must never be retried** (`SEAM-04`, `NFR-S04`). It is single-shot; a failure returns and
the user decides.

---

## 3. The operation — `FL-05`'s order is normative

`FL-05` steps 3→6, in that order, and the order is not yours or mine to rearrange:

1. **`C` from the Changes view.**
2. **Message prompt** — required non-empty, carrying the `UD-01` notice. Its own step, **before** the
   confirmation, so `TU-09`'s confirmation stays a *restatement* rather than a form: a thing you are
   confirming must not change while you confirm it.
3. **Preview + tier-2 confirmation.** The `ConfirmationSummary` carries the whole-worktree capture
   counts and the **AUTHOR key id to be used** (`FL-05` step 5 names it explicitly).
4. **Execute**, then show prikk's result verbatim (patch id, operation counts, both notes).

**The preview re-reads the worktree inside `preview()`** (RFC 014, ruled). Do **not** reuse the
`ChangesView` already on screen: RFC 013's token stamps the change token *at preview time*, and reusing
a view loaded minutes ago while stamping freshness now makes the token assert what the view does not
have. One extra `worktree-status` read; it is what buys the token its meaning.

**Label the preview as stikk's derivation** (RFC 013 F1, Class B — prikk has no commit dry-run). It is
well-founded and you may say why: prikk's own guide states `worktree-status` *"answers 'what would the
next commit author?', not merely 'what differs from the last seal.'"* Cite that; do not overclaim past
it. Carry the `UD-06` whole-worktree reminder, and RFC 012's corrected `UD-08` caveat when untracked
entries are hidden.

---

## 4. Prevent the two refusals — do not merely classify them

Both were verified against prikk 0.31 (RFC 014 F2, F6). Both fail closed, and **stikk knows about both
before arming anything.**

**Cross-ref.** If the queue is non-empty and `Orientation::queued_target` ≠ the focused ref, prikk
refuses with `lock conflict: active WAL is owned by <a>; requested ref <b>`. **Do not offer the
commit.** Explain: the queue belongs to `<queued_target>`, you are focused on `<focused_ref>`; the steps
that exist are to seal first, or to focus the queue's ref.

Both ref names come from **stikk's own authoritative sources** — the intent's ref, and
`queued_target` from `orientation()`. **Never parse them out of prikk's refusal** (`C-T2b`).

**Clean worktree.** prikk refuses with `invalid name: worktree has no node-addressed changes to
commit`. The preview already knows the worktree is clean, so commit is **visibly unavailable with its
reason** (`C-T4d` — disabled-with-reason, never hidden and never a silent no-op).

**Still classify correctly for the race.** Prevention is not a guarantee: the queue can move between
preview and execute. If the cross-ref refusal reaches the seam anyway, classify it **distinctly, not as
`LockConflict`** (RFC 014 decision 2) — nothing is locked, and `LockConflict` will grow a jump to a lock
inspector when `FR-102` lands. A new `StikkError` class carrying prikk's verbatim message; next-steps
from `(class, operation)` as always.

---

## 5. The threshold warning belongs in the preview

prikk warns at `PRIKK_ACTIVE_PATCH_WARN` (800) and refuses at `_LIMIT` (1000). stikk already reads the
queue count. `C-D2a` is explicit: stikk *"surfaces the limit it is about to hit, not just the failure
after"* — so the preview says how close the queue is, before the confirmation, not after a refusal.

Read the thresholds from the environment the same way prikk does, and **if they are absent use prikk's
documented defaults** — but say in the copy that they are defaults, since an operator can change them
and stikk cannot know they did.

---

## 6. The `capability_gate` / palette unification — due now

RFC 013 deferred this with a deadline of "before the first mutating command enters the palette
registry". Commit **is** that command.

The palette currently greys out on a bare `Capability` (`Command::available_to`/`unmet_reason`, RFC 007)
with no tier awareness, while `confirm` enforces the tier-aware `capability_gate` including read-only.
Left as is, the palette would **offer a commit that `confirm` then refuses** under `STIKK_READ_ONLY=1`.

Make the palette's affordance use the same check `confirm` enforces. **This is the narrow fix, not
RFC 002's action-id catalog** — that RFC will absorb this when its own increment comes; do not build it
here.

---

## 7. `OPL-04`'s seam-side re-check, built honestly

Add it: the seam re-reads readiness before spawning `commit`, and refuses if AUTHOR is not ready.

**And write down what it is worth.** For AUTHOR readiness this is **constant within a process** —
`PRIKK_AUTHOR_SEED` presence cannot change underneath a running session. It is built because a future
readiness source genuinely can change (`FR-104`'s trust-policy read for MAINTAINER) and because the
seam is the right place for it. A comment claiming it catches a lapse today would be theatre; say what
it actually does.

---

## 8. Security surface — the first one that matters

- **`NFR-S04`** — no auto-retry, anywhere. A failed commit returns; the user decides. Test it.
- **`C-T4a`/`C-T4c`** — the preview is stikk's derivation and says so; prikk's result and both `note:`
  lines are transported verbatim, never summarised away.
- **`C-T4d`** — the clean-worktree and cross-ref cases are *disabled with a reason*, never hidden.
- **`C-T2a`** — paths, ref names and the patch id are prikk-sourced: inert at every call site.
- **`C-I1`** — the confirmation names the AUTHOR **key id** (`FL-05` step 5). Presence only, never a
  seed, never a value. This is the first time a key id reaches a mutation surface; keep it an id.
- **`NFR-S07`**: this increment adds stikk's **first write data-flow**. The threat model has always
  assumed one (`C-T4a…e` were written for it), so the controls exist — but confirm and record whether
  the document needs an update now that the flow is real, rather than assuming it does not. Say what
  you concluded either way.

---

## 9. Test plan

- **Prevention, both cases**: cross-ref (queue targets another ref) and clean worktree each make commit
  unavailable with a reason, and **no seam call is made** — assert the scripted backend saw nothing.
- **The race**: a cross-ref refusal arriving from the seam classifies to the new class, **not**
  `LockConflict`.
- **Order**: message before confirmation; a commit cannot execute without both.
- **No auto-retry**: a failed commit produces no next-step that re-runs it.
- **Read-only**: `STIKK_READ_ONLY=1` refuses at `confirm`, and the palette shows commit disabled with
  the same reason — the §6 regression test.
- **Verbatim**: both prikk `note:` lines reach the result unchanged; the patch id renders inert.
- **Threshold**: a queue near the warn threshold surfaces it in the preview.
- **Against the real binary**: commit once through `CliBackend` to a probe repo and confirm the patch
  lands in the WAL. A scripted test cannot prove stikk can actually write — and this is the increment
  where that matters most.
- Gates green; state the count delta.

---

## 10. Acceptance criteria

1. `Prikk::commit` exists, is `QueueMutation`, is never retried, and its output parses from a **captured**
   fixture with provenance.
2. `FL-05`'s order is built as written: message step, then confirmation, then execute.
3. The preview re-reads inside `preview()`, is labelled stikk's derivation, and carries `UD-06`/`UD-08`.
4. Cross-ref and clean-worktree commits are **prevented with a reason and no seam call**; a cross-ref
   refusal arriving anyway classifies distinctly from `LockConflict`.
5. Both prikk `note:` lines are transported verbatim; the result shows patch id and operation counts.
6. The palette and `confirm` agree — no command is offered that `confirm` would refuse.
7. The seam-side readiness re-check exists, with a comment saying what it is actually worth.
8. Verified once against the real prikk binary, not only `NullBackend`.
9. Gates green; count delta stated; demos build. Nothing tagged or published.

---

## 11. Submit

Package to `.git-exclude/review-request/014-commit-the-first-mutation/review-request-v1.md`.

Call out: the real-binary result (§9); what you concluded about the threat model (§8); and — as every
increment for six running has produced — whatever you found that contradicts RFC 014. **Push it
yourself once the review says Approved** (`.git-exclude/specs/02-implementer-handoff.md` §6, owner
ruling 2026-09-05); that hand-off step is no longer mine.

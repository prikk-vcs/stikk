# Handoff — the prikk 0.32 re-baseline (v1)

**Companion to:** [RFC 015](../../accepted/015-prikk-0-32-rebaseline.md) (Accepted 2026-09-06).
Inherits its state.
**Realizes:** the 0.4.0 increment after [RFC 014](../../done/014-commit-the-first-mutation.md).
**Design items:** `UD-01` (retires), `UD-09` (narrows), `FR-011` (per-patch display), `ASM-2`/`NFR-R03`
(version honesty), `C-T2a`/`C-T2b`, `T-T4`, `TS-03` (captured fixtures).

> **The point of this increment is that nothing looks broken.** stikk parses 0.32's `log` without
> error and shows a correct screen — it just silently discards the patch ids and messages prikk now
> gives it. That is the RFC 009 F4 shape: prikk supplies evidence, stikk drops it. There is no failing
> test to chase; you are closing a gap, not fixing a break.

---

## 1. Scope

**In**, in this order:
1. **Verify first** (§2) — re-capture every fixture against 0.32, and *observe* the straddling case.
2. Parse `  patch <id>: <message>` into the block row (§3).
3. Render it in **Block detail**, always with the count (§4) — the honesty-critical part.
4. Retire **`UD-01`**: the design set, and the commit overlay copy (§5).
5. Narrow **`UD-09`** precisely, including the now-partly-false note in `view/block.rs` (§6).
6. Gloss the **bundle-decode** skew shape (§7).
7. Raise the validated ceiling to **0.32**, last (§8).

**Out:** `FR-012`'s message filter (now *possible*, its own increment); RFC 006 3b Patch detail as a
rendered diff (still blocked — `UD-09`'s content half); anything about upstream RFC 132's
reclassification (**unreleased** — RFC 014's classifier already handles it, do not touch it); the
History row showing messages (RFC 015 Q1, deliberately deferred).

---

## 2. Verify before you build — both halves

**Fixtures.** Re-capture every constant in `cli_backend/parse/tests.rs` from a real **0.32.0** binary
(the released tag, not prikk's `main` — `main` carries unreleased changes we are not validating).
Report the diff explicitly, as RFC 012 did: the `log` fixture *will* differ; say what else did or did
not.

**The straddling case — this is the one that matters.** RFC 015 F4 is reasoned from prikk's source and
**not observed**. Build a block containing both a pre-0.32 patch and a 0.32 patch, and look at the real
`log` output. You will need an older binary (build the `0.31.1` tag into a scratch worktree, as RFC 014
did) to author the first patch, then 0.32 for the second, then seal.

**Report what you actually see**, including whether the `patches:` count and the number of `patch …:`
lines disagree the way F4 predicts. If they do not — if prikk emits something for messageless patches
after all — **stop and report**: the copy in §4 is built on that disagreement being real, and I would
rather rewrite this handoff than have you build against my inference.

---

## 3. The parser

The new line sits between `required-attestations:` and `previous-ref-state:`:

```
  patch <64-hex patch id>: <message>
```

Parse the id with `ObjectId::parse` (RFC 009 F2's discipline — refuse a non-id rather than pass it
through) and take the message as everything after the first `": "`. **A message may contain anything**,
including colons, newlines-as-escapes, and text resembling a field label — it cannot forge a field,
because the line always *starts* `patch `, but do not write a parser that assumes otherwise.

Add the pair to `BlockRow` as a list. **An empty list is normal**, not an error: pre-0.32 blocks have
none. Refuse only on a malformed id or a line shaped `patch …` that has no `": "` at all.

---

## 4. Block detail — the count is not optional *[the honesty-critical part]*

Render the list in Block detail, and **never render it without the block's `patches:` count beside
it.** When they disagree, say why in plain words — for example:

> 3 patches · 1 with a message. Patches written before prikk 0.32 carry none — that is absence, not
> an empty message.

**Why this is the acceptance-critical behaviour:** a block reporting `patches: 3` while listing one
patch, with no explanation, tells a user the block contains one patch. It contains three. That is a
`T-T4` confident-but-wrong picture *manufactured by stikk out of correct prikk output* — the same
failure class as RFC 009 F4, arrived at from the opposite direction. Write the copy from what you
observed in §2, not from this paragraph.

Messages are **repository content**: `inert` at every call site (`C-T2a`); a display string only —
never a next-step, never a glossary trigger (`C-T2b`). A hostile message must not be able to forge
chrome, and there must be a test proving it.

---

## 5. `UD-01` retires — and the copy is version-conditional

Messages persist on prikk **≥ 0.32** and are discarded below it. stikk supports both, so **neither a
blanket "not persisted" nor a blanket "persisted" is true.** The commit overlay's copy must depend on
the handshake version, the way `changes_view`'s 0.28 gate already does.

Update `UD-01` in `requirements.md` to record it as **retired at prikk 0.32**, with the range statement
— the same shape RFC 012 used for `UD-08`. The commit-result overlay already transports prikk's own
notes and needs no change; it is the *pre-commit* copy that asserts.

---

## 6. `UD-09` narrows — say exactly what is left

`crates/stikk-tui/src/view/block.rs:95` currently reads *"per-patch content inspection awaits prikk
support (UD-09)"*. That is now **partly false**: ids and messages have arrived; content has not.

Correct it, and the `UD-09` entry in `requirements.md`, to the narrower truth — patch **ids** are
enumerable for messaged patches; patch **content** (operations, preimages, `show`/`diff`) and
`log --format json` are not. Also check the doc comments at `stikk-prikk/src/lib.rs:75` and
`stikk-core/src/history.rs:32`, both of which state the old ceiling.

**Do not overstate the retirement.** RFC 006's Patch detail — a patch rendered *as a diff* — remains
blocked. What arrived is the ability to name a patch, not to show one.

---

## 7. The bundle-decode skew shape

0.32 is forward-incompatible (schema 4). The repository-level refusal keeps the shape RFC 012 already
glosses. A **bundle** offered directly refuses earlier and differently:

```
error: malformed persisted data: invalid PatchPurpose canonical form: canonical encoding error: unknown PatchPayload field tag: 6
```

Add a glossary entry and a `(class, operation)` next-step for it, beside RFC 012's. **Capture the real
message** rather than copying it from here. Same rule as before: do not widen the integrity classifier
— let it stay a `Refusal` and let the gloss do the work.

---

## 8. The ceiling, last

Raise `VALIDATED_MAX_MINOR` to **32** only after §2's fixtures come back clean. If any shape other than
`log`'s differs, that is a finding to report before the ceiling moves.

---

## 9. Test plan

- **Parser**: a block with messages, a block without, a **mixed** block (§2's straddling case), a
  malformed patch id **refuses**, a message containing `": "` and one resembling a field label both
  parse intact.
- **Block detail (acceptance-critical)**: when `patches:` exceeds the number of listed patches, the
  rendered buffer contains the explanation — assert the *disagreement* case specifically, not just the
  happy one. A hostile message renders inert and produces no actionable entry.
- **Copy**: below 0.32 the commit overlay says messages are not persisted; at or above, it does not.
  Both asserted.
- **Fixtures**: every constant carries a provenance line naming **0.32.0**.
- Gates green; state the count delta.

---

## 10. Acceptance criteria

1. Every fixture re-captured against 0.32.0, the diff reported, and the straddling case **observed and
   described** — not inferred.
2. `patch <id>: <message>` parsed; malformed ids refuse; an empty list is normal.
3. Block detail shows the list **and** the count, and explains a disagreement in words drawn from what
   you observed.
4. `UD-01` recorded retired at 0.32; the commit copy is version-conditional and true on both sides.
5. `UD-09` narrowed in the design set **and** in all three code doc sites; Patch detail is still
   described as blocked.
6. The bundle-decode shape has a captured fixture, a gloss, and a next-step; the classifier is not
   widened.
7. `VALIDATED_MAX_MINOR` is 32.
8. Gates green; demos build; nothing tagged or published.

---

## 11. Submit

Package to `.git-exclude/review-request/015-prikk-0-32-rebaseline/review-request-v1.md`.

Lead with **§2's two verification results** — the fixture diff, and what a straddling block actually
looks like. Those are the increment's foundation; everything else is built on them. Then the count
delta, and whatever you found that contradicts RFC 015 — eight increments running have each produced
one.

**Push it yourself once the review says Approved** (`.git-exclude/specs/02-implementer-handoff.md` §6).

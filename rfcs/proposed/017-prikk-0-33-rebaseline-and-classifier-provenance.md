# RFC 017 — The prikk 0.33 re-baseline, and five classifier arms with no upstream behind them

**Status.** Proposed (2026-09-06). Opened to re-baseline on prikk 0.33.0 — a release stikk's own
letter caused. Verifying that the two reworded messages still classify correctly meant reading prikk's
whole error taxonomy for the first time, and **that** found the real problems: the failure classifier
matches on five strings prikk has never emitted, one of its tests enshrines an invented message that
asserts the opposite of prikk's actual policy, and one arm puts a false gloss on the commit path today.
**Tracks.** `FR-003` (refuse-and-explain unopenable targets), `FR-104` (signing readiness), `FR-106`
(lock conflicts), `UD-05`, `C-T4a–e`, `NFR-I03`/`RR-5`, and the version gate.
**Touches.** `stikk-prikk` (`cli_backend/classify.rs` and its fixtures, `version.rs`), `stikk-core`
(`present`, if a class changes), `docs/src/reference/requirements.md`.
**Gates.** RFC 016. F5 and F6 are seal's preconditions, and neither was visible from outside prikk.

## Summary

prikk shipped **0.33.0** on 2026-09-06, and two of the three things in it are ours: the error-class
fix we reported, and a `setup`/`key` entrance that changes which repositories stikk will meet. The
re-baseline itself is small and, pleasingly, **already correct** — RFC 014's classifier matched the
stable semantic part of the message rather than the class prefix, so prikk changing the prefix broke
nothing (F1).

The sweep that proved it did not stay small, and it found two things.

**Five of the classifier's nine matching arms key on text prikk does not emit** — not reworded, never
emitted, at any version we support. They have been in the tree since 0.1.0. RFC 009 eradicated
invented fixtures from the *parsers* and never looked at the *classifier*; this is the same defect in
the room next door (F2).

**And one arm fires too often, on the commit path, today.** prikk's full-queue refusal is classified
as a lock conflict, so shipped stikk tells a user *"another writer is active"* immediately above
prikk's own verbatim *"run `prikk seal` before committing again"* (F4). That is the first case in this
project of a stikk-authored gloss contradicting the evidence printed beside it, and it is live.

## Findings

Verified against the **prikk 0.33.0 released tag**, by reading the taxonomy at
`crates/prikk-error/src/lib.rs` and every construction site that reaches a command stikk calls. Where
this RFC quotes a message, it is a `git show 0.33.0:` quote, not a recollection.

### F0 — the complete `PrikkError` prefix set, captured

stikk has classified prikk's errors for four releases without ever having the list. It is twelve
prefixes, and it is now written down so the next sweep is a comparison rather than an expedition:

| Prefix | Variant |
|---|---|
| `canonical encoding error:` | `CanonicalEncoding` |
| `invalid object id:` | `InvalidObjectId` |
| `invalid signature:` | `InvalidSignature` |
| `invalid name:` | `InvalidName` |
| `object type mismatch: expected …, got …` | `ObjectTypeMismatch` |
| `unsupported format version: N` | `UnsupportedFormatVersion` |
| `malformed persisted data:` | `MalformedData` |
| `integrity error:` | `Integrity` |
| `lock conflict:` | `LockConflict` |
| **`precondition not met:`** | **`Precondition`** — new in 0.33.0 |
| `unsupported object type:` | `UnsupportedObjectType` |
| `i/o error:` | `Io` |

`PrikkError` is `#[non_exhaustive]`, so this list grows without a breaking change upstream. **It is a
snapshot, not a contract** — which is the argument for never matching a prefix in the first place.

### F1 — the reclassification is safe, and the reason it is safe is the design *[verified, no fix]*

prikk 0.33.0 changed both messages our letter reported:

```
- error: lock conflict: active WAL is owned by heads/main; requested ref heads/other
+ error: precondition not met: active WAL is owned by heads/main; requested ref heads/other

- error: invalid name: worktree has no node-addressed changes to commit
+ error: precondition not met: worktree has no node-addressed changes to commit
```

`is_cross_ref_conflict` matches `"active wal is owned by"` **and** `"requested ref"` — the semantic
part, deliberately not the class word — so it still fires, still returns `CrossRef`, and `present()`
routes by variant rather than by text. The second message never matched an arm and still does not.
**Nothing in shipped stikk misbehaves at 0.33.0.**

One artefact is stale: `classify/tests.rs`'s cross-ref fixture is captured from **0.31.1** and quotes
a message prikk no longer emits. That is a fixture to re-capture, not a bug.

*This is the first time a deliberate fragility choice has been tested by upstream drift and held. It
is also what prikk asked us not to change, in their words: the degradation rule is "why we can improve
these messages at all."*

### F2 — five arms match text prikk has never emitted *[a shipped defect, since 0.1.0]*

Searched prikk 0.33.0 for each string the classifier keys on:

| Arm | String | Found in prikk 0.33.0? |
|---|---|---|
| `is_environment` | `not a prikk repository` | **no** |
| `is_environment` | `retired repository format` | **no** — appears only in prikk's *prose*, never as output |
| `is_environment` | `unsupported prikk version` | **no** — this is stikk's own version-gate wording |
| `is_lock_conflict` | `another writer` | **no** |
| `is_lock_conflict` | `ref-state precondition` | **no** |
| `is_integrity_finding` | `verify finding` | **no** |

These are not messages that were reworded. Searching prikk's full history, `retired repository format`
has **never** been an error string in that project. The arms were written from what prikk's
documentation *describes*, and a description is not an output.

**The worst of it is a test.** `classify/tests.rs`'s
`a_retired_format_is_environment_with_the_migration_message` asserts on:

```
error: retired repository format 2; run `prikk migrate` to upgrade
```

prikk has **no `migrate` subcommand**, and its real message for this condition says the opposite:

```
error: integrity error: this repository uses format 2, which prikk no longer supports (this version
requires format 6). format-2 support was removed after 0.19.0; migration from format 2 is not
supported.
```

So stikk's test encodes a belief that prikk offers a migration path prikk explicitly states it does
not. **This is RFC 009's defect exactly** — a fixture written rather than captured, agreeing with
itself forever — and it is the reason RFC 009's rule must be a property of the repository rather than
of one sweep.

**Nothing wrong has ever reached a user from this**, and the reason is the degradation rule: an arm
that never fires falls through to a verbatim `Refusal`, which shows prikk's own words. The safe
default absorbed four years of a wrong map. That is the rule working, not an excuse for the map.

### F3 — `FR-003`'s classification half is unmet; its verbatim half holds

A retired-format repository (F2's real message) matches no arm and lands as `Refusal` → a refusal
overlay, rather than `Environment` → a `PlainStatement`. `FR-003` asks stikk to *"refuse-and-explain
unopenable targets: retired repository formats (surface prikk's own migration message verbatim)"*.
**The verbatim half is satisfied.** The classification half is not: a repository stikk cannot open at
all is presented as a refusal *within* a view rather than as a plain statement that the target cannot
be opened.

A foreign directory, meanwhile, *is* classified `Environment` — but through the `no such file` arm,
because prikk's discovery failure surfaces as `i/o error: No such file or directory`, not through the
`not a prikk repository` arm written for it. **The behaviour is right by accident and nobody knew.**

### F4 — six real messages classify as lock conflicts and are not, and one is on the commit path *[a shipped defect, live now]*

**Start with the one users will meet.** prikk refuses a commit when the patch queue is full
(`node_authoring.rs:238`, verbatim at 0.33.0):

```
error: lock conflict: active WAL has 64 queued patches, at or above the configured limit (64);
run `prikk seal` before committing again
```

Nothing is locked. No other writer exists. Waiting does not help — the queue is full and the user must
seal. stikk classifies it `LockConflict`, whose `FR-106` gloss says **another writer is active**.

**So shipped stikk renders a false gloss directly above prikk's own words telling the user the truth.**
RFC 014 shipped commit, this is commit's ordinary full-queue path, and it is the first case in this
project where stikk's gloss *contradicts the verbatim message printed beside it*. `C-T2b` says the
next steps are stikk's to author; it does not say stikk may author one that disagrees with the
evidence on the same card.

Enumerating every `PrikkError::LockConflict` site at 0.33.0 and applying prikk's **own** test for its
new variant — *"nothing here is transient and waiting does not help — the caller must change what they
asked for"* — six of ten are preconditions wearing a lock's name:

| Site | Message (abbreviated) | Is a lock held? |
|---|---|---|
| `node_authoring.rs:238` | `active WAL has N queued patches, at or above the configured limit …; run \`prikk seal\`` | no — **commit path** |
| `active.rs:85` | `active WAL has N queued patches … run doctor or seal before appending again` | no |
| `refs.rs:133` | `repository mutation is blocked by incomplete ref publication; run verify/doctor …` | no |
| `rollback_draft.rs:158` | `rollback-draft requires an empty active WAL` | no |
| `rollback_verify.rs` | `rollback-draft-verify requires an active WAL containing only the rollback draft` | no |
| `seal_from_accepted.rs:189` | `sealing from an accepted claim requires an empty active WAL …` | no |

The remaining four are genuine: two real held locks, a CAS mismatch, and a ref that moved during
planning — all transient, all helped by re-reading and retrying.

This is RFC 012 F-b and RFC 014 F2's shape for the third and fourth time, and prikk fixed two
instances of it in 0.33.0 at our report. **It is an upstream finding as much as ours** — see letter
004 — but stikk must not wait for a fix it cannot schedule: `FR-106`'s gloss is stikk's, and stikk can
stop asserting a writer it has not seen.

### F5 — a trust refusal degrades to a bare refusal, losing `FR-104`'s routing *[gates RFC 016]*

`verify_signer_trusted` refuses with, verbatim at 0.33.0:

```
error: invalid signature: maintainer signer key id <ID> is not trusted by policy
error: invalid signature: maintainer signer public key does not match trusted key <ID>
```

`is_not_ready` matches `"not ready"`, `"no signing key"`, `"key not adopted"`, or
`"maintainer" AND "required"`. The first message contains `maintainer` but not `required`; neither
matches. **Both degrade to a plain `Refusal`** — honest, but without the Trust & Keys routing
`FR-104` exists to provide.

RFC 016 decision 2 says the ceremony "states plainly that a trust refusal is possible and would come
from prikk." **This is that refusal, and stikk currently has nothing to say when it arrives.**

### F6 — the adoption gate covers eight operations, not one *[widens RFC 016 F2]*

`GatedOperation` at 0.33.0 declares: `Seal`, `Merge`, `SyncBuild`, `SyncSeal`, `SyncAdoptTag`,
`TagCreate`, `BranchCreate`, `BranchClose`.

RFC 016 F2 found stikk's `[MNT ✓]` badge over-claiming, and framed it as seal's problem. It is not.
**Every one of those eight operations requires a maintainer key adopted in the repository's trust
policy** — including branch creation and tag creation, which stikk will reach long before merge or
sync. The three-valued readiness RFC 016 rules is therefore not a seal feature; it is the gate for a
whole class of future actions, and it should be built where all eight can read it.

### F7 — `prikk setup` and `prikk key`: a boundary to declare before someone crosses it

0.33.0 adds `prikk setup`, `prikk key generate`, and `prikk key public --seed-env <NAME>`. Two
consequences:

**A line stikk must not cross, written down now rather than after.** `prikk key generate` *creates*
key material and `prikk key public --seed-env` *reads a seed*. `C-I1` is presence-only: stikk checks
that `PRIKK_*_SEED` is set and never reads its value. **stikk must never invoke either command**, and
must never offer "generate a key for me" as an affordance — prikk's own note is that at least one seed
lands in terminal scrollback. The refusal is easy to write today and hard to retrofit after a helpful
increment adds it.

**`prikk setup` adopts a maintainer key**, so repositories created by prikk ≥ 0.33 satisfy F6's gate
by construction, while stikk still cannot verify that on **any** version it supports. That does not
soften the three-valued readiness — it means *unknown* will be the common case for correct
repositories, which is precisely why it must render as unknown and not as a failure either.

One thing not to copy: `setup` prints `policy: required=1`, which prikk's own RFC 138 §3 records as a
**hard-coded literal read from nothing**. stikk must not repeat it as though it were policy.

## Decisions

1. **No classifier arm survives without a fixture captured from a real prikk binary.** Every arm in
   `classify.rs` is either re-grounded on a captured message or **deleted**. An arm that cannot be
   provoked against a real binary is not evidence of a condition; it is a guess with a test.
2. **Delete the invented retired-format fixture and its arm**, and re-ground `FR-003` on the real
   message: `integrity error: this repository uses format N, which prikk no longer supports`. Match
   the stable part (`no longer supports`), never the version numbers or the removal release.
3. **Classify by the *condition*, not the prefix, and never by prose.** Where a message must be
   matched, match the semantic clause that names the condition (F1's rule, now the general one). The
   class word is prikk's to change — 0.33.0 proves it.
4. **`FR-106`'s gloss may not assert another writer unless stikk has evidence of one.** F4's six
   preconditions are recognized on their own captured strings — the full-queue case first, since
   commit already meets it — and glossed by what they are. Where stikk cannot tell, the gloss
   **drops the claim** rather than guessing: a verbatim refusal with no gloss is honest; "another
   writer is active" beside prikk's "run `prikk seal`" is not.
5. **Route trust refusals to `NotReady`** (F5), on the captured `is not trusted by policy` /
   `does not match trusted key` strings, so `FR-104`'s Trust & Keys guidance reaches the one place it
   was designed for. This lands **with RFC 016**, which needs it.
6. **Three-valued maintainer readiness is built for all eight gated operations** (F6), not for seal.
   RFC 016 remains its first consumer; the type does not belong to it.
7. **Declare the `prikk key` / `prikk setup` boundary** in the threat model (`C-I1`'s neighbourhood):
   stikk never invokes a command that creates or reads key material.
8. **Version ceiling 32 → 33**, the single-source `VALIDATED_MAX_MINOR`, with the ceiling grep run
   across `crates/` as well as `README.md` and `docs/` — the omission that let a stale ceiling ship in
   0.3.0.

## What this RFC does not do

**It does not widen `is_integrity_finding`.** RFC 012 F-e's caution stands, and F0 makes it sharper:
`integrity error:` is now known to be the prefix of *both* schema skew and retired formats, neither of
which is a verify finding. The arm's `verify`/`doctor`/`finding` strings are unfounded (F2), and the
fix is to re-ground them on real `verify` output — **which stikk cannot capture until `FR-100` builds
the Verify view.** Until then the arm should be narrowed to what can be captured, not widened.

**It does not touch the version gate's floor.** stikk supports prikk ≥ 0.28; every decision here must
hold across 0.28–0.33, which is why none of them depends on a 0.33-only surface.

## Consequences

- The rule RFC 009 established — **captured, never written** — becomes a property of the whole seam
  rather than of the parsers RFC 009 happened to be looking at.
- stikk's map of prikk's failures is grounded for the first time, and F0 makes the next re-baseline a
  diff instead of a discovery.
- Two of RFC 016's premises change before it is accepted: its capability finding is eight times
  larger than stated, and the trust refusal it promises to handle honestly turns out to be unhandled.
- **A live, user-visible wrong claim is corrected** (F4) — not caught before shipping. Commit shipped
  in 0.4.0's candidate and already routes through it; the full-queue path is ordinary use, not an edge.
  It was found by reading upstream's error sites rather than by a user hitting it, which is the only
  part of this worth being pleased about.

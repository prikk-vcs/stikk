# Handoff — the 0.33 re-baseline and the classifier's provenance (v1)

**Companion to:** [RFC 017](../../accepted/017-prikk-0-33-rebaseline-and-classifier-provenance.md)
(Accepted 2026-09-06). Inherits its state.
**Realizes:** the 0.4.0 increment after [RFC 015](../../done/015-prikk-0-32-rebaseline.md), and the one
that gates [RFC 016](../../proposed/016-the-seal-ceremony.md).
**Design items:** `UD-05` (classification), `TS-03` (captured fixtures), `FR-003` (unopenable targets),
`FR-104` (signing readiness), `FR-106` (lock conflicts), `C-I1` (presence-only key material),
`C-T4a–e`/`T-T4` (no confident-but-wrong picture), `NFR-I03`/`RR-5` (verbatim degradation),
`ASM-2`/`NFR-R03` (version honesty).

> **Read this before anything else.** RFC 017's finding is that stikk's failure classifier matches
> strings prikk has never emitted, because they were written from prikk's *prose* instead of captured
> from prikk's *output*. **You cannot fix that with a better-written table**, and RFC 017 contains a
> table. Every message in it — the six in F4 especially — I read out of prikk's **source** at the
> 0.33.0 tag. **That is not a capture, and I have made exactly this mistake before**: RFC 009 carries a
> correction block where I asserted prikk's behaviour from reading its tree and the implementer
> disproved it against the real binary. Treat my table as a list of places to look, and let the binary
> settle every question.

---

## 1. Scope

**In**, in this order:
1. **Provoke and capture** (§2) — the whole increment rests on this. Nothing else starts first.
2. Re-ground or **delete** every classifier arm (§3). Deletion is a success, not a shortfall.
3. Fix `FR-106`'s gloss so it never asserts a writer stikk has not seen (§4) — *the live defect*.
4. Re-ground `FR-003` on prikk's real retired-format message (§5).
5. Add the trust-refusal arm (§6) — classifier only; its presentation is RFC 016's.
6. Declare the `prikk key` / `prikk setup` boundary (§7).
7. Raise the validated ceiling to **0.33**, last (§8).

**Out:**
- **The three-valued `MaintainerReadiness`** and anything about the `[MNT ✓]` badge — that is RFC 016,
  and RFC 017 F6 only establishes how wide it must be.
- **Trust & Keys next-steps and the ceremony's presentation** of a trust refusal — RFC 016. You add
  the classification here; what stikk *says* lands with seal.
- **Widening `is_integrity_finding`** — see §3's note. Narrow it; never widen it.
- Anything about prikk's `main` branch. **0.33.0 is the released tag; that is what we validate.**
  prikk has unreleased work in flight (a trust read surface). It is not ours yet and must not appear
  in this increment in any form.
- The `Precondition` variant's other six sites upstream. We reported them; we do not wait for them.

---

## 2. Provoke and capture — the foundation *[do this first, report before building]*

**Get a real prikk 0.33.0 binary** (build the released tag, as RFC 014 did for 0.31.1). Then work the
inventory below. For each row: **provoke the condition, capture stderr verbatim, and record the exact
command that produced it.**

`prikk setup` is new in 0.33 and makes several of these cheap — it creates a repository, generates both
keys, and adopts the maintainer key in one step.

| # | Arm | String it matches today | How to provoke | If you cannot |
|---|---|---|---|---|
| 1 | `is_environment` | `not a prikk repository` | run any read in an empty directory | delete the string (§3) |
| 2 | `is_environment` | `no such file` | same as 1 — **check which arm actually catches it** | — |
| 3 | `is_environment` | `permission denied` | `chmod 000` a `.prikk` directory | delete |
| 4 | `is_environment` | `could not` + `prikk` | unknown origin — hunt for it | delete |
| 5 | `is_environment` | `unsupported prikk version` | **this is stikk's own wording** — confirm no prikk path emits it | delete |
| 6 | `is_environment` | `retired repository format` | see §5 | see §5 |
| 7 | `is_cross_ref_conflict` | `active wal is owned by` + `requested ref` | `commit --from-worktree --ref heads/other` against a WAL owning `heads/main` | must not fail |
| 8 | `is_lock_conflict` | `lock` + (`held`\|`conflict`\|`already`) | two concurrent mutations, or an orphaned lock file | narrow to what you caught |
| 9 | `is_lock_conflict` | `another writer` | — | delete |
| 10 | `is_lock_conflict` | `ref-state precondition` | — | delete |
| 11 | `is_lock_conflict` | `cas mismatch` | — | delete |
| 12 | `is_not_ready` | `not ready` / `no signing key` / `key not adopted` / `maintainer`+`required` | unset each `PRIKK_*` env var in turn | delete per string |
| 13 | `is_integrity_finding` | `finding` / `verify` / `doctor` | `prikk verify` on a sealed repository | see §3 |
| 14 | **F4's six** | — | **§4** | **§4** |
| 15 | **the trust refusal** | — | **§6** | **§6** |

**Report the inventory before you build.** A table with three columns — arm, captured message (or
"could not provoke"), and the command — is the deliverable of this section, and I want to see it
before §3 changes a line. **If a row disagrees with RFC 017, the RFC is wrong and I will amend it.**

**Every fixture that survives carries a provenance line naming `0.33.0` and the command that produced
it.** No exceptions, including the ones already there: the cross-ref fixture is captured from **0.31.1**
and quotes a message prikk no longer emits — re-capture it.

---

## 3. Re-ground or delete — the rule

**An arm survives only if you captured the message it matches.** No arm is kept because it looks
plausible, because a design document describes the condition, or because deleting it feels like a
regression.

**Deleting is safe by construction**, and this is worth internalizing rather than taking on trust: an
unmatched message degrades to `StikkError::Refusal`, which shows prikk's own words verbatim and offers
no retry (`RR-5`/`NFR-I03`). Five arms have been dead since 0.1.0 and **no user has ever seen anything
untrue because of it** — the degradation absorbed the whole error. prikk's own reply asked us not to
change that behaviour, in their words: it is *"why we can improve these messages at all."*

So: a deleted arm costs a *gloss*, never the truth. **A wrong arm costs the truth.**

**Where you keep an arm, match the semantic clause, never the class prefix.** F1 is the evidence:
prikk changed `lock conflict:` → `precondition not met:` on the very message RFC 014 classifies, and
nothing broke, because the match was on `active wal is owned by` + `requested ref`. The prefixes are
prikk's to change and `PrikkError` is `#[non_exhaustive]`.

**`is_integrity_finding`: narrow, never widen.** RFC 012 F-e's caution stands and RFC 017 F0 sharpens
it — `integrity error:` is the prefix of *both* schema skew and retired formats, and neither is a
verify finding. If §2 row 13 captures real `verify` output, ground the arm on that. **If it does not,
reduce the arm to nothing and leave `Integrity`-category failures degrading.** The Verify view is
`FR-100` and does not exist; an arm routing into a view we have not built is worse than no arm.

---

## 4. `FR-106`'s gloss — the live defect *[acceptance-critical]*

Today, on an ordinary full queue, shipped stikk renders:

> **another writer is active** *(stikk's gloss)*
> `error: lock conflict: active WAL has 64 queued patches, at or above the configured limit (64); run `prikk seal` before committing again` *(prikk's words)*

**stikk's explanation contradicts the evidence printed beside it, on the same card, on the commit
path.** Commit shipped in the 0.4.0 candidate; this is not an edge case.

**Provoke it first.** `PRIKK_ACTIVE_PATCH_LIMIT` is a policy env var — set it low, commit twice, and
capture what you get. Then work RFC 017 F4's other five sites. **My six are read from source, not
captured** (see this handoff's opening note): confirm each, and tell me about any I missed or any I
called a precondition that turns out to hold a lock.

**The rule for the fix:** `FR-106`'s gloss may assert another writer **only where stikk has evidence of
one.** Three outcomes, in preference order:

1. **Recognized precondition** — a captured string, its own gloss saying what is actually true (the
   queue is full; seal first). The full-queue case must reach this outcome; it is the one users meet.
2. **Genuine lock conflict** — a captured string, `FR-106`'s existing gloss, unchanged.
3. **Neither** — **drop the gloss and show prikk's message alone.** A refusal card with no explanation
   is honest. "Another writer is active" over "run `prikk seal`" is not.

**Do not** add a `Precondition` variant to `StikkError` speculatively, and do not restructure the
taxonomy. If the captured evidence turns out to need one, stop and raise it — that is a design change
and it is mine to make, not yours to infer. `C-T2b` still holds throughout: next-steps are
stikk-authored, and now they must also **agree with the verbatim text on the same card**.

---

## 5. `FR-003` — the invented fixture, and what replaces it

Delete `a_retired_format_is_environment_with_the_migration_message` and the string it drove. It asserts
`run \`prikk migrate\` to upgrade`; prikk has **no `migrate` command**, and its real message says
migration is **not supported**. The test has agreed with itself since 0.1.0.

prikk's real refusal, read from source at 0.33.0 (**capture it if you can**):

```
error: integrity error: this repository uses format 2, which prikk no longer supports
(this version requires format 6). format-2 support was removed after 0.19.0;
migration from format 2 is not supported.
```

**Provoking it needs a format-2 repository**, which needs prikk ≤ 0.19 to create. That may cost more
than the arm is worth. **Your call, and either answer is acceptable** — say which you did:

- **Captured**: ground the arm on the stable clause (`no longer supports`), never the format numbers or
  the removal version. Both change; the clause does not.
- **Not captured**: **delete the arm.** `FR-003`'s verbatim half already works through degradation, and
  an unfounded arm is what this RFC exists to remove. Do not replace one guess with a better guess.

Then amend `FR-003` in `docs/src/reference/requirements.md` to state what stikk actually does: surfaces
prikk's migration message verbatim, and (if the arm survived) classifies it as an environment failure.
**Do not amend it to claim a behaviour you did not test.**

While you are there, row 1 of §2 has a subtlety worth reporting: a foreign directory *is* classified
`Environment` today — but I believe through the `no such file` arm, not the `not a prikk repository`
arm written for it. **Confirm which arm catches it.** If I am right, the behaviour is correct by
accident, and the fix is to ground it on what actually arrives.

---

## 6. The trust refusal — classifier only

prikk refuses a maintainer whose key is not adopted, read from source at 0.33.0:

```
error: invalid signature: maintainer signer key id <ID> is not trusted by policy
error: invalid signature: maintainer signer public key does not match trusted key <ID>
```

`is_not_ready` catches neither, so both degrade to a bare `Refusal` — honest, but without `FR-104`'s
routing.

**Provoking this is cheap now.** `prikk setup` adopts one maintainer key; export a *different*
`PRIKK_MAINTAINER_KEY_ID` / `PRIKK_MAINTAINER_SEED` and attempt a gated operation. Capture both
messages if you can reach both.

**Route them to `StikkError::NotReady` on the captured strings.** That is all this increment does.

*This refines RFC 017 decision 5, which said the whole of F5 lands with RFC 016.* On reflection the
split is cleaner: **017 owns the classifier, 016 owns what stikk says.** You are capturing the strings
in §2 regardless, and an arm without them is what we are removing. The Trust & Keys next-steps, the
readiness copy, and the ceremony's handling stay with seal.

**Note for §2's report, not for building:** the same gate covers eight operations upstream — seal,
merge, both sync paths, adopt-tag, tag create, branch create, branch close. You are not building for
those. It is why the arm is not called `is_seal_not_ready`.

---

## 7. The `prikk key` boundary — written down before someone crosses it

prikk 0.33 adds `prikk key generate` (creates key material) and `prikk key public --seed-env <NAME>`
(reads a seed). **stikk must never invoke either**, and must never offer "generate a key for me."
`C-I1` is presence-only: stikk checks that `PRIKK_*_SEED` is set and never reads its value.

Record it in `docs/src/reference/threat-model.md` beside `C-I1`, as a **rule with a reason** — prikk's
own `setup` warns that at least one seed lands in terminal scrollback, and key management is not a
history browser's job. Point users at `prikk setup` verbatim instead.

**A test would be better than a sentence** if one is cheap: an assertion that no `key`/`setup`
subcommand string appears in `cli_backend.rs`'s command surface. Add it if it fits the existing shape;
say so if it does not.

**Do not** parse or quote `prikk setup`'s `policy: required=1` line anywhere. prikk records it as a
hard-coded literal printed in the voice of a query; repeating it would make stikk the third site.

---

## 8. The ceiling, last

Raise `VALIDATED_MAX_MINOR` to **33** only after §2's inventory is reported and §3–§6 are green.

**Grep for stale ceiling strings across `crates/`, `examples/`, `README.md`, `docs/`, and
`CHANGELOG.md` — all of them.** In 0.3.0 I scoped that grep to `README.md docs/` and let
`"validated through 0.30"` ship inside `crates/`, in the feature built for version honesty. Do not
inherit my mistake. `validated_ceiling_display()` is the single source; anything else stating a number
is a bug.

---

## 9. Test plan

- **Provenance (the point of the increment)**: every fixture in `classify/tests.rs` carries a
  `0.33.0` provenance line and the command that produced it. A fixture without one is a defect.
- **Deleted arms**: for each, a test proving the message it *used* to match now degrades to a verbatim
  `Refusal` with prikk's text intact. Deletion must be observably safe, not assumed safe.
- **F4 (acceptance-critical)**: the captured full-queue message classifies to something whose gloss
  does **not** claim another writer, and a render test asserts the visible card contains both prikk's
  verbatim text and a gloss consistent with it. **Assert the contradiction is gone**, not merely that
  a gloss exists.
- **Cross-ref**: the re-captured 0.33 message still classifies `CrossRef`. This is F1's regression
  guard — it is the arm that survived the drift.
- **Trust refusal**: both captured messages classify `NotReady`.
- **`FR-003`**: whichever branch of §5 you took, tested as described there.
- **Hostile input**: unchanged obligations — `C-T2a` inert rendering, `C-T2b` no message-derived
  next-step. A captured prikk message that resembles a next-step must not become one.
- Gates green; state the test-count delta.

---

## 10. Acceptance criteria

1. §2's inventory reported **before** any arm changed, with captured messages and the commands.
2. Every surviving arm grounded on a captured 0.33.0 message; every ungrounded arm **deleted**, with a
   degradation test.
3. The invented retired-format fixture and its arm are gone.
4. The full-queue refusal no longer renders a gloss that contradicts prikk's verbatim text, proven by
   a render test.
5. `FR-106`'s gloss asserts another writer only where a writer was evidenced.
6. Trust refusals classify `NotReady`; no Trust & Keys presentation added (that is RFC 016).
7. `is_integrity_finding` is narrowed or removed — **not widened**.
8. The `prikk key` / `setup` boundary is in the threat model.
9. `FR-003` amended to what was tested, not to what was intended.
10. `VALIDATED_MAX_MINOR` is 33; the ceiling grep covered `crates/` too.
11. Gates green; demos build; nothing tagged or published.

---

## 11. Submit

Package to `.git-exclude/review-request/017-prikk-0-33-rebaseline-and-classifier-provenance/review-request-v1.md`.

**Lead with §2's inventory.** It is the increment's foundation and the only part I cannot check by
reading the diff. Then the F4 before/after, then what you deleted and why.

**Tell me every place my table was wrong.** RFC 017's F4 list is read from prikk's source; I have
asserted prikk's behaviour from its tree before and been disproved by the binary, which is how RFC 009
got its correction block. Nine increments running have each produced a finding that contradicted my
RFC. **A report that contradicts nothing is a report I will read twice.**

**Push it yourself once the review says Approved** (`.git-exclude/specs/02-implementer-handoff.md` §6).

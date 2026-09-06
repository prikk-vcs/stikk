# RFC 016 — The seal ceremony, and the readiness stikk has been over-claiming

**Status.** **Done** — shipped on `main` as a 0.4.0 candidate, 2026-09-06 (`b71c7c9` + `3b511aa` + `56fe249`), after three review rounds. Accepted by the project owner 2026-09-06 behind RFC 017; originally proposed 2026-09-06 — the most consequential action stikk will ever offer: freezing
queued patches into immutable, MAINTAINER-signed history. Investigating it found that **stikk's
MAINTAINER readiness badge claims something it cannot know**, which must be fixed before the ceremony
can honestly gate on it.
**Tracks.** `FR-052` (as amended 2026-09-05), `FL-06`, `FR-104` (signing readiness), `FR-103` (trust
management), tier 3 of `FR-121`, and `C-T4d` (capability honesty).
**Touches.** `stikk-prikk` (a `seal` seam method, the second that writes; readiness), `stikk-model`
(`Readiness`), `stikk-core` (the ceremony operation), `stikk-tui` (the ceremony's steps).
**Depends on.** RFC 017 — its F5 (trust refusals degrade to a bare refusal) and F6 (the adoption gate
covers eight operations, not seal alone) are this RFC's preconditions.

## Amended 2026-09-06, on prikk's replies to letters 001 and 002

Three of the four findings below changed after the prikk team answered. **The design did not change;
two of its justifications did, and one was wrong in a way worth naming.**

- **F1's justification is retied.** `--allow-no-audit` is confirmed **scaffolding**, not a permanent
  part of prikk's contract — so justifying our consent step by *"standing where prikk required an
  acknowledgement"* would become false the day the flag goes. It is now tied to the ceremony.
- **F2 described the gate wrongly, and prikk's own design set says so.** Adoption is **object trust,
  not ref authority**; it is also **eight operations wide**, not seal's alone.
- **F3's request has been accepted upstream and ruled** — and that does *not* retire the unknown
  state. See the revised upstream section for why.

*Everything below is re-verified against the **prikk 0.33.0** released tag; the original text was
written against 0.32.0.*

## Summary

Seal is where every control this project has built earns its keep at once: preview-first, tier-3
confirmation, informed consent, verbatim truth, and no-auto-retry all apply to a single irreversible
act. The machinery exists (RFC 013), the first mutation has shipped (RFC 014), and `FR-052` has already
been amended to what prikk can actually tell us.

Checking prikk's seal surface first turned up four things. One is a defect in shipped stikk that has
nothing to do with seal and everything to do with whether the ceremony can be honest.

## The findings

Verified against prikk 0.32.0 (the released tag).

### F1 — `--allow-no-audit` is mandatory, and that makes `FR-052`'s consent step load-bearing

`prikk seal` **refuses without `--allow-no-audit`**, as a **usage error (exit 2)**:

```rust
if !allow_no_audit { return Err(CliError::Usage("seal scaffold requires --allow-no-audit")) }
```

So stikk will **always** pass it — and per RFC 014 F6, an exit 2 from stikk is a *stikk bug*, not a
user-facing refusal. The flag is prikk's mechanism for forcing the caller to acknowledge that no audit
ran. If stikk passes it on the user's behalf without a deliberate human act, **stikk has silently
converted prikk's "you must acknowledge" into "stikk acknowledged for you."**

`FR-052`'s *"presented as informed consent, not a pre-ticked box"* is therefore **structural, not
stylistic**.

**Amended: the step is tied to the ceremony, not to prikk's flag.** prikk answered that
`--allow-no-audit` is **scaffolding** — the refusal calls seal a *"scaffold"*, a successful seal prints
*"audit plugins remain later PRs"*, and the milestone that would replace it (**M4 — WASM Plugin and
Audit**) is unscheduled with the attestation slice not started. They declined to tell us what replaces
it, because nobody has decided, and said so rather than sparing us a rework — and they offered the
correction this paragraph now carries:

> Tie it to the ceremony instead of to our flag. Then the day the flag changes, your wording is still
> true and only ours moves.

That is right, and it is the same discipline as `C-T4`: **a claim whose truth depends on someone
else's unscheduled decision is a claim stikk should not make.** So the consent step exists because
**the act is irreversible** — sealing freezes queued patches into immutable, signed history — and
prikk's flag is *corroboration that upstream agrees an acknowledgement belongs here*, not the reason.
The step survives the flag's removal unchanged; only its copy loses a sentence.

### F2 — MAINTAINER readiness is over-claimed today *[a shipped defect]*

`seal` calls `verify_signer_trusted(&layout, signer, GatedOperation::Seal)`: the MAINTAINER key must be
**adopted in the repository's trust policy**, not merely present in the environment.

stikk checks only presence — `maintainer_ready: is_set(MAINTAINER_KEY_ID) && is_set(MAINTAINER_SEED)`.
So the status bar shows **`[MNT ✓]`** for a maintainer whose key the repository does not trust, and
`Capability::Maintainer` is derived from it. That is a capability claim stikk cannot support: the
badge asserts a gate is open that will refuse.

This is live now, independent of seal — `C-T4d` requires capability to be honest, and `FR-104`
explicitly includes *"whether the trust policy knows the maintainer key"* in readiness. stikk has never
implemented that half.

**Amended — I described the gate wrongly, twice.**

**It is object trust, not ref authority.** The original text above read *"the badge says you can
publish history"*. prikk's `MaintainerTrustPolicy` says otherwise, in its own words at the 0.33.0 tag:
a `Block`/`RefState` is trusted if *any* adopted key signed it — *"object trust, not ref authority;
adopting a key never lets it move a ref (`RefStore::publish` still requires a signature from this
operator's own signer)."* Adoption means **prikk accepts that key's signatures on objects**. prikk's
RFC 138 §7.3 rules that wording letting a caller conclude otherwise *"is a defect, not a nuance"* — so
the badge, the ceremony copy, and the glossary must all say what adoption is, and I had it wrong first.

**And it is eight operations wide, not seal's.** `GatedOperation` declares `Seal`, `Merge`,
`SyncBuild`, `SyncSeal`, `SyncAdoptTag`, `TagCreate`, `BranchCreate`, `BranchClose`. Framing this as
seal's problem was an artefact of having arrived through seal. See **RFC 017 F6**: the three-valued
readiness is the gate for all eight, and seal is only its first consumer.

### F3 — stikk cannot check adoption, because prikk cannot list it

`prikk trust maintainer` offers **`add` and `remove` — and no `list`.** There is no way through the CLI
to ask which maintainer keys a repository trusts. `verify --format json` covers publication trust but
is a full verification, which `NFR-P05` forbids running implicitly on open.

So stikk cannot compute the missing half of F2. It can only stop claiming it.

**Amended — and one trap named, because it is the obvious wrong fix.** `prikk verify` prints
`sealed-block <id>: <key_id>` per sealed block, which looks like an answer. prikk's reply says plainly
that it is not, for two reasons neither of us could see from the other side:

- **it is historical signer attribution, not current policy** — a key revoked since still prints;
- **it does not exist before the first seal** — an unsealed repository prints nothing, *which is
  exactly the moment the ceremony asks*.

**No future increment may resolve the unknown state from `verify` output.** It would be a wrong answer
delivered confidently, in the requirement written against confident wrong answers, at the moment the
question matters most.

**This also makes `FR-103` unsatisfiable as written** — *"Trust management: adopted maintainer keys
list"* — the third requirement in this project written against a surface prikk does not have, after
`FR-052` and `FR-051`.

### F4 — seal's cross-ref refusal is worded differently from commit's

Seal refuses a cross-ref attempt with `active WAL is owned by {actual}; requested seal ref is
{ref_name}` — note **"requested seal ref is"**, where commit says **"requested ref"**. RFC 014's
`is_cross_ref_conflict` matches `"active wal is owned by"` **and** `"requested ref"`, so it will **not**
fire for seal; the refusal degrades to a plain verbatim `Refusal`.

Safe, but inconsistent, and the better next-steps would be lost. As with commit, stikk can **prevent**
it: `Orientation::queued_target` already says which ref the queue belongs to.

**Amended: at 0.33.0 the two now differ in class prefix as well.** commit's message became
`precondition not met: active WAL is owned by …; requested ref …`, while seal's carries **no class
prefix at all**. The same repository fact, one release later, reaches a caller under two different
prefixes and two different trailing clauses. **That is the argument for decision 5** — matching the
stable semantic clause was a guess when RFC 014 made it and is evidence now.

Seal also refuses an **empty queue** (`active WAL has no patch records to seal`) — likewise preventable,
and stikk already knows the count.

## Decisions

1. **Stop claiming MAINTAINER readiness stikk cannot verify** (F2/F3), with a **three-valued**
   readiness — ready / not ready / **unknown** — never a boolean caveated in prose. The unknown state
   must never render as a pass, applying `C-T2c′`'s existing rule to the second place it belongs. See
   Q1, which settles the shape and why.
2. **The ceremony gates on presence, warns about adoption, and never promises success.** Tier 3
   (`FR-121`), MAINTAINER capability required, and the confirmation states plainly that a trust
   refusal is possible and would come from prikk.
3. **The no-audit acknowledgement is its own step, unchecked, and cannot be defaulted** (F1). It is not
   a line in the confirmation summary; it is a distinct act, **because the act it precedes is
   irreversible** — not because prikk currently demands a flag. Its copy states what is true of the
   ceremony (no audit ran; this cannot be undone), never what is true of prikk's scaffolding. Seal
   remains **untyped** (RFC 013 Q3, owner-ruled): two deliberate acts, not three.
4. **Prevent the empty-queue and cross-ref refusals** (F4), as RFC 014 does for commit, from
   `queued_target` and the queue count — never by parsing prikk's refusal (`C-T2b`).
5. **Widen cross-ref recognition to seal's wording** for the race case, keeping the match on the stable
   semantic part (`active WAL is owned by`) rather than either command's trailing phrasing.
6. **Amend `FR-103`** to what is knowable, as `FR-051`/`FR-052` were — and record that the upstream
   ask is filed, accepted and ruled, but unreleased.
7. **Route the trust refusal to `NotReady`, not a bare refusal** — RFC 017 F5. Decision 2 promises the
   ceremony will say a trust refusal may come; this is what stikk says when it does. The ceremony
   cannot ship without it.
8. **Say what adoption means wherever it is named** (F2 amended): prikk accepts that key's signatures
   on objects. Never "may publish here".

## Upstream dependency — asked, accepted, ruled, unreleased

**Sent as letter 002; prikk opened, accepted and ruled RFC 138 the same day.** The ruling is
**both** surfaces, with machine-readable output:

- `prikk trust maintainer list` — enumerate adopted keys;
- `prikk trust maintainer check --key-id <ID>` — the fallback stikk offered, which prikk judged *"may
  be the better shape"* because it matches `verify_signer_trusted`'s own question;
- **`--format json` on both**, and **`check` exits `0` for a negative answer**, because *"key X is not
  trusted"* is not a failure. prikk's own §7.2 records why: exiting `1` there *"would file a
  successful query as an operational failure — precisely the conflation the stikk project reported to
  us in their first letter."*

**This does not retire the unknown state, and reading it that way would be the mistake.** Three
reasons, in increasing order of how long they last:

1. **It is unreleased.** No handoff has been written. stikk designs against released tags — RFC 009's
   lesson, and this project does not get to relearn it on the seal ceremony.
2. **stikk supports prikk ≥ 0.28.** Even after it ships, every supported version below it still cannot
   answer the question. Unknown is not a gap waiting to close; it is **version-conditional**, and it
   is permanent for as long as the support floor is.
3. **Q1 predicted exactly this and it is now testable.** The claim was that *unknown* resolves into
   *ready*/*not ready* **with no change to the type or the UI's shape** when the surface lands. That
   claim now has a date attached. If it turns out to be wrong, this RFC was wrong.

**Prefer `check` when it lands.** It answers the ceremony's actual question, needs no enumeration, and
its `0`-on-negative contract means stikk reads an answer rather than classifying an error.

**Also recorded, from prikk's RFC 138 §3:** `policy: required=1` is printed by prikk at two sites and
**read from nowhere** — a literal in the voice of a query. stikk must never quote it as policy, and
must not add a third site.

## Open questions — settled 2026-09-06, one by reversal

**Q1 — should `Capability::Maintainer` derive from presence alone, caveated in copy?**
**Ruled: no. Readiness becomes three-valued, and the unknown state must never render as a pass.**

*(This reverses my own lean, on the owner's challenge that it did not meet the project's "clean, safe
and secure, robust and sophisticated" bar. It did not, and the design set already said why.)*

The lean collapsed **three** states into two. Maintainer readiness is:

| | |
|---|---|
| key material present **and** adopted in trust | **ready** |
| key material absent | **not ready** |
| present, adoption **unverifiable** (F3 — prikk offers no `trust maintainer list`) | **unknown** |

Rendering *unknown* as `[MNT ✓]` is precisely what `C-T2c′` forbids in the one place this project has
already faced the same shape: the three-valued author-signature outcome, where **Unverifiable**
*"must never render as a pass/green state"*. A green check on a claim stikk cannot verify is the same
error, and I proposed it.

So: a three-valued `MaintainerReadiness`, not a boolean plus a sentence. That is

- **clean** — one concept reused from `FR-035`'s precedent rather than a second, prose-only mechanism;
- **safe** — the unknown state cannot be mistaken for a pass, by construction rather than by wording;
- **robust** — when prikk grows `trust maintainer list`, *unknown* resolves into *ready*/*not ready*
  with no change to the type or the UI's shape. The boolean-plus-caveat would have had to be
  re-architected at that point;
- **sophisticated** in the way this project means it — the absence of an upstream surface becomes
  visible in the type system instead of hidden in a caveat someone will eventually delete.

`Capability::Maintainer` therefore derives from **ready or unknown** (the affordance is still offered —
hiding seal from someone whose key *is* adopted would be its own wrong picture, `C-T4d`), but the badge
and the ceremony render the three states distinctly, and the ceremony's copy is driven by the state
rather than shown unconditionally.

**Q2 — does the ceremony re-read orientation between the consent step and execute?**
**Ruled: no, and because the token already covers it, not merely to keep things small.** RFC 003's
change token includes the queued count and its target ref, so a queue that moves between consent and
execute *is* caught, by the mechanism built for exactly that. A second read would be a second mechanism
for one property — less clean — and would imply an atomicity guarantee stikk cannot give, since it holds
no lock across think-time (`NFR-R02`, `CT-05`).

## Consequences

- The first irreversible act stikk offers is gated by a consent step that exists because prikk demanded
  one, not because a designer liked ceremony.
- A shipped capability over-claim is corrected — found by asking what seal actually requires rather
  than by anyone hitting it.
- Three requirements now stand amended against prikk's real surface (`FR-051`, `FR-052`, `FR-103`),
  each with the upstream ask that would restore it recorded rather than the requirement quietly
  under-delivered.


## Carried forward, recorded at completion 2026-09-06

- **The other seven gated operations.** `MaintainerReadiness` is built for all eight; merge, both sync
  paths, adopt-tag, tag create, branch create and branch close are unbuilt. Each lands with its own
  increment and needs no change to the type.
- **`MaintainerReadiness::Ready` is unconstructible** until prikk releases `trust maintainer check`
  (their RFC 138 — accepted and ruled, unreleased). Q1's robustness claim is untested until then, and
  `Unknown` stays reachable afterwards for every session below stikk's support floor.
- **`Target::TrustKeys` has no renderer.** The trust-refusal card points toward Trust & Keys and offers
  `Refresh` as its only action, because the view does not exist yet.
- **🔎 Found during review: no `GlossaryEntry`'s `explanation` is rendered anywhere.** `render_glossary`
  shows the key list and the Git→prikk terminology only; `glossary_codes` surfaces a code's *name* on a
  refusal card, never its text. **Four codes now ship with explanations no user can read**
  (`.prikkignore`, `SCHEMA_SKEW_CODE`, `FULL_QUEUE_CODE`, `TRUST_REFUSAL_CODE`), and this increment
  added the fourth. That is `FR-111`'s unbuilt half, and it was found only because a review suggestion
  assumed the surface existed. **It should be scheduled, not carried indefinitely** — the entries are
  written, tested, and inert.
- **`render_stale`'s sizing** — folded into this increment's push rather than deferred (review v3).

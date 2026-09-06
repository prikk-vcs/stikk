# RFC 016 — The seal ceremony, and the readiness stikk has been over-claiming

**Status.** Proposed (2026-09-06) — the most consequential action stikk will ever offer: freezing
queued patches into immutable, MAINTAINER-signed history. Investigating it found that **stikk's
MAINTAINER readiness badge claims something it cannot know**, which must be fixed before the ceremony
can honestly gate on it.
**Tracks.** `FR-052` (as amended 2026-09-05), `FL-06`, `FR-104` (signing readiness), `FR-103` (trust
management), tier 3 of `FR-121`, and `C-T4d` (capability honesty).
**Touches.** `stikk-prikk` (a `seal` seam method, the second that writes; readiness), `stikk-model`
(`Readiness`), `stikk-core` (the ceremony operation), `stikk-tui` (the ceremony's steps).

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
stylistic**: it is the only thing standing where prikk put a required acknowledgement.

### F2 — MAINTAINER readiness is over-claimed today *[a shipped defect]*

`seal` calls `verify_signer_trusted(&layout, signer, GatedOperation::Seal)`: the MAINTAINER key must be
**adopted in the repository's trust policy**, not merely present in the environment.

stikk checks only presence — `maintainer_ready: is_set(MAINTAINER_KEY_ID) && is_set(MAINTAINER_SEED)`.
So the status bar shows **`[MNT ✓]`** for a maintainer whose key the repository does not trust, and
`Capability::Maintainer` is derived from it. That is a capability claim stikk cannot support: the
badge says *you can publish history* and the attempt will be refused.

This is live now, independent of seal — `C-T4d` requires capability to be honest, and `FR-104`
explicitly includes *"whether the trust policy knows the maintainer key"* in readiness. stikk has never
implemented that half.

### F3 — stikk cannot check adoption, because prikk cannot list it

`prikk trust maintainer` offers **`add` and `remove` — and no `list`.** There is no way through the CLI
to ask which maintainer keys a repository trusts. `verify --format json` covers publication trust but
is a full verification, which `NFR-P05` forbids running implicitly on open.

So stikk cannot compute the missing half of F2. It can only stop claiming it.

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

Seal also refuses an **empty queue** (`active WAL has no patch records to seal`) — likewise preventable,
and stikk already knows the count.

## Decisions

1. **Stop claiming MAINTAINER readiness stikk cannot verify** (F2/F3). `Readiness` splits what it
   knows from what it does not: key material **present** is knowable; **trust adoption** is not. The
   badge and the capability derive from presence, and the UI says what that means — *"key material
   present; whether this repository trusts it is not knowable until you seal."* No invented state, no
   silent over-claim.
2. **The ceremony gates on presence, warns about adoption, and never promises success.** Tier 3
   (`FR-121`), MAINTAINER capability required, and the confirmation states plainly that a trust
   refusal is possible and would come from prikk.
3. **The no-audit acknowledgement is its own step, unchecked, and cannot be defaulted** (F1). It is not
   a line in the confirmation summary; it is a distinct act, because it stands where prikk required
   one. Seal remains **untyped** (RFC 013 Q3, owner-ruled): two deliberate acts, not three.
4. **Prevent the empty-queue and cross-ref refusals** (F4), as RFC 014 does for commit, from
   `queued_target` and the queue count — never by parsing prikk's refusal (`C-T2b`).
5. **Widen cross-ref recognition to seal's wording** for the race case, keeping the match on the stable
   semantic part (`active WAL is owned by`) rather than either command's trailing phrasing.
6. **Amend `FR-103`** to what is knowable, as `FR-051`/`FR-052` were — and file the upstream ask.

## Upstream dependency

**New: `prikk trust maintainer list`.** It unblocks `FR-103`, and it is what would let stikk answer
"can I seal?" honestly instead of deferring to the attempt. Rank it **with** the queued-patch
enumeration ask and below `UD-09`'s content surface: it does not block seal, it makes seal's
readiness truthful.

## Open questions

- **Should `Capability::Maintainer` still be derived from presence alone?** It gates *affordance*, and
  offering a seal that may be refused is arguably right (`C-T4d` prefers disabled-with-reason to hidden,
  but this is enabled-with-caveat). *Leaning: yes, derive from presence and caveat it* — hiding seal
  from someone whose key *is* adopted would be worse than offering one that may refuse. Settle in the
  handoff.
- **Does the ceremony re-read orientation between the consent step and execute?** RFC 013's token
  already stamps a change token, so the machinery covers staleness; this asks whether the *queue count*
  shown at step 1 should be re-read before executing. *Leaning: no* — the token is the mechanism, and a
  second read would imply a guarantee it cannot give.

## Consequences

- The first irreversible act stikk offers is gated by a consent step that exists because prikk demanded
  one, not because a designer liked ceremony.
- A shipped capability over-claim is corrected — found by asking what seal actually requires rather
  than by anyone hitting it.
- Three requirements now stand amended against prikk's real surface (`FR-051`, `FR-052`, `FR-103`),
  each with the upstream ask that would restore it recorded rather than the requirement quietly
  under-delivered.

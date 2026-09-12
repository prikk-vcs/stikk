# Handoff B — the readiness rebuild: `key status`, `binding`, and a model that stops guessing (v1)

**Companion to:** [RFC 026](../../accepted/026-readiness-after-the-key-directory.md) (Accepted
2026-09-12, **Q1 ruled (b)** — read the ruling, it decides what happens on 0.40).
**Follows:** [Handoff A](a-rebaseline-handoff-v1.md) and its
[addendum](a-addendum-json-validation-handoff-v1.md), both approved and pushed. The ceiling is 0.41 and
the suite runs there, which is what makes this handoff possible.
**Design items:** `FR-104`, `FR-103`, `C-I1a–e`, `C-S2`, `C-T2c′`, `C-T4d`, `AC-01…04`, `ASM-2`.
**Not in this handoff:** RFC 026 F7's two render leftovers. They are **Handoff C**, written and waiting
at [`c-render-leftovers-handoff-v1.md`](c-render-leftovers-handoff-v1.md) — F7 said they land here or
get their own increment, and they are getting their own increment rather than a third carry.

> **The twelve failing suite tests are this handoff's acceptance test.** A shipped them red on purpose.
> They go green here, or the model is still wrong.

---

## 0. What I measured before writing this, and what it changes

I built a real prikk **0.41.0** and read both its output and its source. **Three things in RFC 026 are
wrong about the binary**, and you would have found each of them by building against the RFC:

| RFC 026 F2 says | The 0.41.0 binary |
|---|---|
| `source` is `seed-file-override` / `key-directory` / **`absent`** | **two values only** — there is no `absent`; an unusable key is `usable: false` with a `reason` |
| the report carries **`legacy_variable_set`** | **the field does not exist.** `key-status-v1` is `role`, `source`, `path`, `usable`, `reason`, `key_id`, `key_id_source`, `public_key`, `binding` — nine fields, measured and read from prikk's emitter |
| — | **`public_key` and `binding` are both `null`** whenever the seed is not usable, and `binding` is also `null` when there is no repository to ask |

**Do not build the stale-variable warning against `legacy_variable_set`.** §4 says what to do instead —
and the answer keeps `env.rs` alive rather than deleting it, which is the better outcome anyway.

**And one finding of my own, which widens F4.** prikk's key id **always** has a value: when
`PRIKK_<ROLE>_KEY_ID` is unset it defaults to the role's own name, and `key_id_source` says which
(`environment` or otherwise). stikk's `key_id.rs` returns `None` when the variable is unset, so **on the
default setup — no variable exported, which is what `prikk setup` produces — stikk's confirmation shows
no signing key id at all, while prikk would sign as `author`.** F4 found that we name a key that may not
be the one that signs; this is the same card showing *nothing* where prikk has an answer. Both are the
same defect and both are fixed by reading the id from `key status` instead of from the environment.

## 1. Scope

**In:**
1. The seam method (§2) — `key status` behind one `Prikk` method, version-banded.
2. The readiness type (§3) — mirroring prikk's `binding` instead of inventing a vocabulary.
3. The three bands (§4) — ≤ 0.39 env presence, 0.40 unverifiable-and-says-so, ≥ 0.41 `key status`.
4. The key-id display gate (§5) — F4 plus the widening above.
5. **`C-S2`** (§6) — the control the threat model says three times is not implemented.

**Out:** F7's render leftovers (Handoff C). The four unblocked views. The `refused paths:` surface.

## 2. The seam method

`key status` is a prikk read, so it goes through the seam like every other one — **an eleventh `Prikk`
method**, not an environment read that happens to shell out. `stikk-core` must not learn that readiness
has versions.

```
fn readiness(&self, repo: &Path) -> Result<Readiness>
```

**It takes a repository path because `binding` needs one** — prikk computes `binding` only when asked
about a repository, and `binding` is the whole point. All four of today's call sites
(`cli_backend::commit`, `cli_backend::seal`, `orient`, `seal`'s read path) already have one in scope; I
checked.

**Parse it with the readers Handoff A built.** `key-status-v1` is a fourth schema of the same shape, its
name checked before any field is read, its names and ids validated at the boundary. `public_key` is an
id-shaped 64-hex value — validate it the way `ObjectId` fields are validated, and say so if prikk ever
sends something else.

**Version-gate it in `reads_json`'s neighbourhood**, from the cached handshake, and nowhere else. One
number, one place — the `0.30`-in-two-places lesson (RFC 015) is the reason this instruction keeps
appearing.

**The extra spawn is correct, not a cost to optimize away.** `commit` and `seal` re-check readiness at
the seam (`OPL-04`), and today that re-check *cannot catch a lapse* because environment presence is
constant within a process — the code says so in its own comment. Against `key status` it genuinely can:
a maintainer key adopted, revoked, or rebound between orientation and seal changes the answer. One
process spawn on a human-confirmed action is the price of the check finally meaning something. **Say in
the comment that it now catches what it was written for**, and delete the apology.

## 3. The type

**Decision 3 is binding: mirror prikk's vocabulary, do not collapse it and then explain the collapse.**

Today `Readiness` is `author_ready: bool` plus a three-valued `MaintainerReadiness`. That asymmetry was
right when AUTHOR had nothing to be unsure about. It is not right now: `binding` applies to both roles,
and `mismatch` applies to both.

**Per role, two axes, because prikk has two:**

- **usable** — `usable: true/false`, with prikk's own `reason` when false (`missing`,
  `override-missing`, `readable-by-others (mode 0644)`, `undecodable`). **Carry the reason verbatim**
  (`ER-02`); do not re-word it, and do not map four reasons onto one "not ready".
- **binding** — `matches` / `unrecorded` (author) / `not-adopted` (maintainer) / `mismatch`, plus
  **absent**, which prikk spells `null` and means "there was no usable seed to bind, or no repository
  to ask". Absent is a real state, not a parse failure.

**Two different unknowns, and conflating them is the trap in this handoff.** stikk already has one and
is about to acquire another:

| | means | capability | why |
|---|---|---|---|
| **`Unknown`** (today's `MaintainerReadiness::Unknown`) | key material is present; adoption is unverifiable **on this prikk** (≤ 0.33, and ≤ 0.39 as stikk reads it) | **granted, with the caveat rendered** | RFC 016's rule: offer the action, let prikk refuse, never render the caveat as a pass |
| **`Unverifiable`** (new, 0.40 only) | stikk cannot see whether there is key material at all | **withheld**, and the UI says *unknown* — never *not ready* | Q1(b): honest and actionable beats confident and wrong |

**They must not share a variant.** One grants and one withholds; a single `Unknown` that sometimes does
each is the shape `C-T2c′` exists to forbid.

**`mismatch` is a refusal-to-arm** (Decision 4). Not a caveat on an offered action: there is a usable
seed and a known id, and prikk **will refuse at signing time**, so offering the action is offering a
failure. Withhold the capability and say which two things disagree.

**`unrecorded` arms.** prikk accepts a first signature and binds the id then — so commit must be offered.
It is the *display* that must be honest about it (§5), not the capability.

**`not-adopted` does not arm for MAINTAINER.** This is the state RFC 025 wanted a fourth variant for
(`trusted: false`, neither `NotReady` nor `Unknown`); `binding` names it, and prikk refuses the seal.
**Withhold, with the existing trust glossary entry as the next step** — `TRUST_REFUSAL_CODE` already
explains adoption, and this is the first time stikk can point at it *before* the refusal rather than
after.

**`Capability::derive` stays the one fold**, including `STIKK_READ_ONLY`. Every gate keeps going through
it; none of them grows its own rule.

## 4. The three bands, and what `env.rs` becomes

| prikk | readiness from | notes |
|---|---|---|
| **≤ 0.39** | environment presence — today's rule | correct there; `_SEED` is the mechanism and `binding` does not exist |
| **0.40 exactly** | **nothing. `Unverifiable`** | Q1(b), ruled: name the cause and name the fix — *prikk 0.40 moved seeds to a key directory and stikk cannot see them; prikk 0.41 answers this directly, upgrade to it*. Do **not** probe with `key public --role`; the ruling says why, in three reasons, and the third is that a probe would reintroduce F4 |
| **≥ 0.41** | `key status --format json` | `binding` and all |

**`env.rs` does not die. It changes job**, and keeps its guard:

1. It is the **≤ 0.39 band's answer**, unchanged, still presence-only.
2. It becomes the **stale-variable detector at ≥ 0.40** — which is what `legacy_variable_set` was going
   to give us and does not exist. stikk can see this itself: `PRIKK_<ROLE>_SEED` **set** on a prikk that
   has stopped reading it is a fact `env.rs` can establish with the presence probe it already has, with
   no new capability and no value ever materialized. Warn plainly: the variable is set, this prikk
   ignores it, and it is not what will sign.

**`env_module_never_materializes_a_variable_value` must still pass, unweakened.** If the rebuild makes
that test awkward, the rebuild is in the wrong module — `key_id.rs` exists because that trade was
refused once already, and its own doc records why.

## 5. The key-id display

Two defects, one fix.

- **F4**: we render `Signing key id: <id>` plainly on commit's and seal's confirmations. On a repository
  where `binding` is `unrecorded` the id we name **may not be the key that signs** — and that is the
  *default* state of every freshly created repository, so it is the first commit a new user makes.
- **§0's widening**: when `PRIKK_<ROLE>_KEY_ID` is unset we render **nothing**, while prikk signs as the
  role's own name.

**The rule, which is prikk's own guidance from letter 007 §2:**

| `binding` | the card shows |
|---|---|
| `matches` | the id, plainly. This is the only case that earns a plain statement |
| `unrecorded` | the id, **said to be unbound** — it will bind on this first signature. Honest confirmation, not a hedge |
| `not-adopted` / `mismatch` | not a caveat on an armed action — the action is withheld (§3) and the card says which two things disagree |
| absent (`null`) | no id claim at all |

**Read the id from `key status` at ≥ 0.41**, not from the environment: that is what fixes the silent
case, and `key_id_source` tells you whether the operator chose it or prikk defaulted. `key_id.rs` stays
for the ≤ 0.40 bands and keeps its mirror-image guard.

**Render tests at 80 columns for every row above**, per the standing rule, with `TestBackend` buffer
captures rather than screenshots. The `unrecorded` wording is the one a new user meets first — write it
as though it is the only sentence they will read, because it is.

## 6. `C-S2`, at last

The threat model says **three times** that this control does not exist (§3.2, the A-KEY row, the
coverage table). RFC 023 F4 found it listed as in force when it never was, and the correction is written
there in the sharpest possible terms — *"the document most responsible for telling a reader what
protects them is the worst possible place for a claim it cannot support."*

**`key status` carries `public_key`, which is the value that makes the control implementable**
(Decision 6): stikk compares the public key in effect against the documented example public inputs and
flags a match as unsafe, persistently (`FR-104`). **By the public value, never by storing a secret** —
that is the whole design, and `C-I1a–e` is untouched by it because a public key is public.

- The example values are **prikk's published documentation inputs**. Source them from prikk's docs,
  record where each came from and at which version, and put them behind one constant with that
  provenance written down. A sweep rule that finds them is `git ls-files | xargs grep -ln "<value>"`.
- **A match is a persistent flag, not a one-time toast.** A user who dismissed it and kept working must
  still see it at the next confirmation.
- **The three `NOT IMPLEMENTED` markers come off only when it is real**, and all three in the same
  commit as the implementation. Not before, and not one of them.
- If the example inputs turn out not to be published anywhere citable, **stop and say so** rather than
  inventing a list. A control that flags the wrong values is worse than the honest marker it replaced.

## 7. Gates

The eight, plus the suite on the full matrix.

**And the acceptance test this handoff actually has:** the **twelve** real-binary tests A shipped red go
**green at 0.41**, with 0.28 still green. That is fourteen of fourteen. If some go green and some do not,
report which and why before changing anything else — a partial green is a finding, not a step.

**The `0.38`/`0.41` sweep** is not needed again; A did it. **A `PRIKK_.*_SEED` sweep is**: after this,
every live claim that stikk reads readiness from `PRIKK_*_SEED` presence must say which band it is
describing. `git ls-files | xargs grep -ln "PRIKK_"` — grep is a floor, not a ceiling, and illustrative
text drifts exactly like renderer text.

## 8. Acceptance criteria

1. `key status` reads through one `Prikk` seam method, version-banded from the cached handshake, the
   number in one place; parsed by A's readers with the schema name checked first and names, ids and
   `public_key` validated at the boundary.
2. The readiness type mirrors prikk's `binding` vocabulary for **both** roles, carries prikk's `reason`
   verbatim, and keeps `Unknown` and `Unverifiable` as **separate** variants — one granting with a
   caveat, one withholding.
3. `mismatch` and `not-adopted` withhold the capability and name the disagreement; `unrecorded` arms.
4. The three bands behave as §4 says, 0.40 reporting unverifiable with cause **and** fix, and no
   `key public --role` probe anywhere.
5. `env.rs` keeps the ≤ 0.39 band and gains the stale-variable detection; its
   never-materializes guard passes **unweakened**.
6. The key-id display gates on `binding`, reads the id from `key status` at ≥ 0.41, and is covered by
   80-column render tests for `matches`, `unrecorded`, and absent.
7. `C-S2` implemented with sourced, provenance-recorded example values; all three threat-model
   `NOT IMPLEMENTED` markers removed in the same commit — or, if the values are not citable, none of it
   done and the reason reported.
8. Eight gates green; the suite **fourteen of fourteen** on the full matrix at 0.28 and 0.41, run ID
   named; the `PRIKK_` sweep done.
9. Nothing tagged or published.

## 9. Submit

Package to `.git-exclude/review-request/026-b-readiness-rebuild/review-request-v1.md`.

**Lead with the suite going green** — the same twelve, named, at 0.41, beside A's red output. That is
the whole argument of RFC 026 in one diff, and nothing else in this increment says it as well.

**Then lead me through `Unknown` versus `Unverifiable`** at whichever place you found them hardest to
keep apart. I have asserted they are different; you will be the first person to find out where that is
expensive, and I would rather hear it than have it quietly collapsed.

**Then the first-commit card**, rendered at 80 columns, on a fresh repository where `binding` is
`unrecorded`. That sentence is the one a new user meets before any other, and today it is either a
confident claim stikk cannot support or nothing at all.

**Push once approved.**

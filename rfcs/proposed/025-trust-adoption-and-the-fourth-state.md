# RFC 025 — Trust adoption: constructing `Ready`, implementing `C-S2`, and a state the type does not have

**Status.** Proposed (2026-09-12). The next 0.6.0 increment, and the first since 0.4.0 that changes what
stikk can tell a user rather than correcting what it told them.
**Tracks.** `FR-103`, `FR-104`, `C-S2`, `C-T2c′`, `C-T4d`, `AC-01…04`, `NFR-P05`, and RFC 023's two
carried render items.
**Touches.** `stikk-prikk` (a trust seam read), `stikk-model` (`MaintainerReadiness`), `stikk-core`
(`orient`, `present`), `stikk-tui` (the badge), `docs/src/reference/threat-model.md`.

## Summary

`MaintainerReadiness::Ready` has been unconstructible since RFC 016 built it. prikk 0.34 shipped the
surface that resolves it, and RFC 023 B built the other half — stikk now knows its own MAINTAINER key
id. **The two compose into one call.**

Verified against a real prikk 0.38.0:

```json
$ prikk trust maintainer check --key-id maintainer --format json
{"schema_version":"trust-check-v1","key_id":"maintainer","trusted":true,
 "public_key":"8ea3ea800339bafecc12cfd813e393d66a8403450333e024dbe5b2289536f6b9"}

$ prikk trust maintainer check --key-id not-adopted --format json     # exit 0
{"schema_version":"trust-check-v1","key_id":"not-adopted","trusted":false,"public_key":null}
```

**One call answers both open questions**: `trusted` resolves readiness, and `public_key` is the value
`C-S2` compares. `list` is not needed — it enumerates a policy to answer a question `check` answers
directly, which is what letter 006 said we would reach for.

**And it turns up a state the type does not have.** See F2, which is this RFC's substance.

## Findings

### F1 — `check` composes with RFC 023 B, and needs no enumeration

`check --key-id <id>` needs *our* MAINTAINER key id. **RFC 023 B built exactly that** — the key-id
module, for `FL-05`'s confirmation. So the chain is: key-id module → `check` → readiness. Where no id is
set there is nothing to check, and the answer is the one stikk already gives.

`trusted: false` exits **`0`** — prikk's own RFC 138 §7.2 ruling, that a negative answer is a successful
query. So a "not adopted" answer arrives as data, not as an error to classify.

### F2 — `trusted: false` is neither `NotReady` nor `Unknown` *[the substance]*

`MaintainerReadiness` has three states, and their doc comments say what they mean:

| State | Means today |
|---|---|
| `Ready` | key material present **and** adopted |
| `NotReady` | **key material absent** |
| `Unknown` | present; adoption **unverifiable** |

**`trusted: false` on prikk ≥ 0.34 is none of them.** The key material is present, so it is not
`NotReady`. Adoption is not unverifiable — we just verified it, and the answer was no. **It is a fourth
thing: present, checked, and definitively not adopted.**

**This is not pedantry, because the three states drive `Capability::derive`.** `Ready | Unknown` grant
`Maintainer`; `NotReady` does not. RFC 016 Q1 chose to grant on `Unknown` with an argument that was
explicitly about **uncertainty**:

> the affordance is still offered — hiding seal from someone whose key *is* adopted would be its own
> wrong picture (`C-T4d`)

**That argument does not survive certainty.** Where prikk has told us the key is not adopted, offering
the seal ceremony offers an act that **will** be refused — and the user discovers it after a tier-3
confirmation and a no-audit acknowledgement, at the moment of execution. `C-T4d` cuts the other way
here: the confident-but-wrong picture is the affordance, not its absence.

See Q1.

### F3 — RFC 016 Q1's robustness claim, now testable

RFC 016 ruled the three-valued type partly on this:

> when prikk grows `trust maintainer check`, *unknown* resolves into *ready*/*not ready* **with no
> change to the type or the UI's shape**. The boolean-plus-caveat would have had to be re-architected at
> that point.

**Half of that holds and half does not.** The UI's shape is unaffected — the badge is already
three-valued and `Ready` already has a glyph nothing has rendered. But **the type does need a fourth
state** (F2), because the claim assumed `trusted: false` collapses into `NotReady`, and it does not.

**That is a correction to my own RFC and it should be recorded as one.** The three-valued decision was
still right — a boolean would have been worse and the badge shape survives — but the specific claim
that the type would not change was wrong, and it was wrong for a reason nobody could see before the
surface existed.

### F4 — `C-S2` becomes implementable, exactly as RFC 023 Q1 resolved

`check` returns the adopted `public_key`. RFC 023 Q1 established the comparison values are **derived by
prikk's own `key public` from its published example seeds, verified by a suite test** — captured from
prikk's output, not transcribed from its prose.

**The dangerous case is now reachable and checkable**: a user who adopted the public key of a published
example seed gets `trusted: true` **and** a compromised `public_key`. That is precisely what `C-S2`
exists to flag, and stikk can now see it.

### F5, F6 — RFC 023's two render leftovers, riding along

Both live in files this increment touches anyway, and both are small:

- **`render_refusal` loses the `│ ` quote prefix on wrapped verbatim lines.** RFC 024 deliberately left
  it — fixing framing as a side effect of a sizing change was the bundling that RFC ruled out — and left
  it as **a one-argument change**: the continuation indent is currently four spaces.
- **The backslash gloss hedges about a fact stikk holds.** *"If this repository is on prikk 0.28…"* —
  `OrientationView` carries `prikk_version`. A conditional about something you know is a withheld
  answer, which was RFC 023's own theme.

## Decisions

1. **One seam read: `trust maintainer check --key-id <our id>`**, version-gated to prikk ≥ 0.34. Below
   it, `Unknown` — unchanged, and permanent while the floor is 0.28.
2. **Eager, not lazy.** The badge is always on screen; if stikk can know, showing `?` is a withheld
   answer. This is a third spawn on open, alongside `handshake` and `orientation`. **It is not an
   `NFR-P05` concern** — that requirement is about *verify*'s linear cost; `check` reads one policy file
   — but the cost is real and the increment should measure it rather than assume.
3. **`C-S2` implemented** per RFC 023 Q1's derivation, and the threat model's three *not implemented*
   markers removed **only** once it is.
4. **F5 and F6 ride along.** Both are small, both are in files this increment opens, and both have been
   carried once already.
5. **RFC 016 Q1's claim is corrected in RFC 016 itself**, not only here. A robustness claim that turned
   out half-wrong should be readable where it was made.

## Open question

**Q1 — what is the fourth state, and what capability does it grant?**

- **(a) A fourth variant** — `NotAdopted`, say. `Capability::derive` grants `Maintainer` on
  `Ready | Unknown` and not on `NotAdopted | NotReady`. Honest, and the badge gets a fourth rendering.
- **(b) Widen `NotReady`** to "not usable for publishing, whether absent or unadopted", and distinguish
  the two only in copy. Keeps three states; makes one of them mean two things.
- **(c) Keep three states and map `trusted: false` to `NotReady`**, accepting that a badge reading "no
  key material" is wrong for a user who has key material.

**My lean: (a), and the capability question is why.** Under (b) or (c) the badge and the capability
cannot disagree with each other — but a user with a present-but-unadopted key needs to be told something
different from a user with no key at all: **their action is `prikk trust maintainer add`, not "set up a
key".** Different cause, different fix, different sentence.

**What makes this genuinely open**: (a) means a fourth glyph in a status bar that already carries five
badges, and RFC 016 chose three-valued partly *because* `C-T2c′`'s precedent was three-valued. A fourth
state is defensible; a fourth state nobody can read at a glance is not. **If the badge cannot express
four states legibly, (b) with very careful copy may be the better trade** — and that is a rendering
judgement I would rather have made against a render than against a paragraph.

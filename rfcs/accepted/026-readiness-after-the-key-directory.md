# RFC 026 — Readiness after the key directory: the 0.41 re-baseline, and a model that is wrong today

**Status.** **Accepted by the project owner 2026-09-12**, Q1 ruled (b) by the architect the same day. Proposed 2026-09-12. **Delivered in three handoffs**: A the re-baseline (**landed 2026-09-13**, with an addendum), B the readiness rebuild, C F7's two render leftovers. **Split from two to three by the architect 2026-09-13**: F7's items hang off nothing in B and sharing a diff with the readiness model would make one review carry two arguments — F7 allows them their own increment and this is it. **Supersedes [RFC 025](../archive/025-trust-adoption-and-the-fourth-state.md)**,
withdrawn the same day. Three prikk releases — 0.39, 0.40, 0.41 — against a validated ceiling of 0.38.
**Tracks.** `FR-104`, `FR-103`, `C-I1a–e`, `C-S2`, `C-T2c′`, `C-T4d`, `AC-01…04`, `UD-02`, `ASM-2`,
and RFC 023's two carried render items.
**Touches.** `stikk-prikk` (`env.rs`'s replacement, `key_id.rs`, three JSON parsers), `stikk-model`
(`MaintainerReadiness` and its author counterpart), `stikk-core`, `stikk-tui`, the threat model.

## Summary

**stikk's signing-readiness model is wrong at prikk 0.40, which is published today.** Not incomplete —
wrong in both directions, on the path where being wrong matters most.

prikk 0.41 ships the fix we asked for and more: **`prikk key status [--role R] [--format json]`**
(`key-status-v1`), which answers readiness **from the same computation `commit` and `seal` use**. It is
tagged, not yet published.

**This RFC is the re-baseline 0.38 → 0.41 with the readiness rebuild inside it**, because readiness
cannot be built against a version we have not validated, and validating three releases while rebuilding
the model separately would leave stikk half-converted across a release.

## Findings

### F1 — the model is wrong at 0.40, and 0.40 is published

`read_readiness_with` computes `is_set(PRIKK_*_KEY_ID) && is_set(PRIKK_*_SEED)`. prikk 0.40 stopped
reading `PRIKK_*_SEED` and made setting one a refusal:

| At prikk 0.40 | stikk shows | Truth |
|---|---|---|
| keys in the key directory, nothing exported — **the correct setup** | not ready; commit and seal hidden | the user can do both |
| a stale `export PRIKK_AUTHOR_SEED` | ready | prikk **refuses** |

**stikk 0.5.0 is shipped and runs above its ceiling by design**, so this is live for anyone on 0.40
today.

### F2 — `key status` is a better answer than the one RFC 025 was going to build

Per role: `source`, the `path`, `usable` with a `reason` (`missing`, `readable-by-others (mode 0644)`,
`override-missing`, `undecodable`), the `key_id` and whether it came from the environment or the
default, the `public_key`, and — given a repository path — `binding`.

**Corrected 2026-09-13, measured against a real 0.41.0 binary and its emitter, not against the letter
this finding was written from.** Three things above were wrong when written, and each would have been
built against:

| As written from letter 007 | The 0.41.0 binary |
|---|---|
| `source` is `seed-file-override` / `key-directory` / **`absent`** | **two values only** — an unusable key is `usable: false` with a `reason`, not an `absent` source |
| the report carries **`legacy_variable_set`** | **no such field.** `key-status-v1` has nine: `role`, `source`, `path`, `usable`, `reason`, `key_id`, `key_id_source`, `public_key`, `binding` |
| — | **`public_key` and `binding` are both `null`** when the seed is not usable; `binding` is also `null` with no repository to ask |

**`legacy_variable_set`'s absence is not a gap to wait on.** stikk can establish the same fact itself:
`PRIKK_<ROLE>_SEED` **set** on a prikk that has stopped reading it is a presence question, which is
exactly what `env.rs` already answers without materializing a value. Handoff B §4 rules it that way, and
it keeps `env.rs` alive with its guard intact rather than retiring the module. **It is still worth
telling prikk**, since the letter promised the field.

**Every not-ready state exits `0`.** prikk's reasoning is ours: *"a non-zero exit would make it
indistinguishable from a repository that cannot be read."*

**And it is one computation, not a second opinion** — prikk's changelog says `commit`, `seal` and
`key status` now answer from one place, so `usable: true` is a prediction of what the signing path will
do. **That is strictly better than RFC 025's `trust maintainer check`**, which answered adoption only.

### F3 — the state space is larger than either project's type had

RFC 025 found that `trusted: false` is neither `NotReady` nor `Unknown` and proposed a fourth variant.
**`binding` independently names the same state — `not-adopted` — and one more neither of us had:**

| `binding` | meaning |
|---|---|
| `matches` | the id is bound to the key that will sign |
| `unrecorded` | author only: this id has never signed here; it will bind on first use |
| `not-adopted` | maintainer only: the id is not in the trust policy |
| `mismatch` | the id **is** recorded/adopted, to a **different** public key |

**`mismatch` is the one to sit with.** It is not "not ready" — there is a usable seed and a known id.
It is the two disagreeing, which prikk refuses at signing time. Neither RFC 016's three-valued type nor
RFC 025's proposed fourth had anywhere to put it.

### F4 — RFC 023 B's key-id display is not yet honest, and prikk told us how to fix it

We render `Signing key id: dev-author` on commit's and seal's confirmations. prikk measured what we
asked in letter 007 §2:

> an AUTHOR id binds on first use per repository, and **before that first use the id you display and the
> seed we would sign with can disagree without a refusal** — after it, they cannot.

**So on a repository where the id is `unrecorded`, our confirmation names a key that may not be the one
that signs — and that is the *default* state, not an edge case.** Measured on a real 0.41.0, immediately
after `prikk setup`:

```json
{"role":"author",     "usable":true, "key_id":"author",     "binding":"unrecorded"}
{"role":"maintainer", "usable":true, "key_id":"maintainer", "binding":"matches"}
```

`setup` adopts the maintainer key, so maintainer binds at once; **no author signature exists yet, so
every freshly created repository starts `unrecorded`.** The first commit a new user makes is exactly the
case where our confirmation is least entitled to name a key plainly. Their guidance is the fix: *"Show the id when `binding` is `matches`; when it is
`unrecorded`, say so."*

This is the `C-T2c′` shape a third time: **an answer that is true, an answer that is unknown, and a UI
that renders them identically.**

**Widened 2026-09-13, measured.** prikk's key id **always** has a value: with `PRIKK_<ROLE>_KEY_ID`
unset it defaults to the role's own name, and `key_id_source` says which. stikk's `key_id.rs` returns
`None` there — so on the default setup, the one `prikk setup` produces, **stikk's confirmation shows no
signing key id at all while prikk would sign as `author`.** F4 above is a card naming a key that may not
sign; this is the same card naming nothing where prikk has an answer. One defect, one fix: read the id
from `key status` rather than from the environment.

### F5 — 0.39 retires the last three prose parsers

`log`, `branch` and `tag` gained `--format json` (`log-report-v1`, `branch-list-v1`, `tag-list-v1`) —
letter 006 §1's request, and **the entire remainder of stikk's prose parsing**. Each of the three has
already cost a re-baseline.

`worktree-status` also now names the paths `commit` would refuse, which answers letter 006 §4 and lets
commit's preview **prevent** the symlink refusal it currently only passes through.

### F6 — `C-I1e` holds, and the interim probe is sanctioned

prikk confirmed `key public --role <role>` reads a seed file they own, prints only a public key, creates
nothing — **so `C-I1e` holds whether or not we call it**, and RFC 135 §9.6 is undisturbed. They name it
an acceptable interim presence probe with two stated limits: its missing-file message differs, and it
says nothing about binding.

**So we are not forced to duplicate prikk's path resolution at 0.40** — which was the thing letter 007
most wanted to avoid.

### F7 — RFC 023's two render leftovers, carried again

`render_refusal`'s lost `│ ` on wrapped verbatim lines (now a one-argument change, RFC 024 §5), and the
backslash gloss hedging about a `prikk_version` stikk holds. **Both have been carried twice. They land
here or they get their own increment; they do not get carried a third time.**

## Decisions

1. **One increment, staged, not two.** The re-baseline and the readiness rebuild are the same change to
   the same model. Splitting them leaves stikk half-converted across a release.
2. **Readiness becomes version-conditional in three bands**, and the seam says which band it is in:
   **≤ 0.39** env presence (today's rule, correct there); **0.40** the sanctioned `key public --role`
   probe; **≥ 0.41** `key status --format json`.
3. **The readiness type mirrors `binding`** rather than inventing a parallel vocabulary. Where prikk
   distinguishes `matches` / `unrecorded` / `not-adopted` / `mismatch`, stikk does not collapse them and
   then have to explain the collapse.
4. **The key-id display gates on `binding`** (F4). Shown plainly when `matches`; qualified when
   `unrecorded`; and `mismatch` is not a caveat, it is a refusal-to-arm.
5. **The three prose parsers retire** (F5) — the JSON forms are the parsers from here.
6. **`C-S2` ships with this**, not separately: `key status` carries `public_key`, which is the value
   RFC 023 Q1's derivation compares. The threat model's three *not implemented* markers come off only
   when it does.
7. **F7 lands here.**

## What this RFC does not do

**It does not raise the ceiling to 0.41 until 0.41 is published.** It is tagged today. The re-baseline
targets whatever is published when the work starts, and the suite's version guard enforces that.

**It does not build the four unblocked views.** Still each their own RFC.

## Open question

**Q1 — what does stikk do on prikk 0.40, in the window before 0.41 publishes?**

0.40 is published now and the model is wrong on it. 0.41 fixes it and is tagged, not released.

- **(a) Build the 0.40 band** (F6's `key public --role` probe) and the 0.41 band, shipping both.
  Correct on every supported version, and the 0.40 code is dead the day everyone upgrades — but "dead
  when everyone upgrades" is true of every version band in a range starting at 0.28.
- **(b) Skip 0.40's band**: treat 0.40 as a version where readiness is **unknown and said to be
  unknown**, and only implement `key status`. Less code, honest, and it hides commit from a 0.40 user
  who can commit — the same user-visible outcome as the bug, honestly labelled.
- **(c) Wait for 0.41 to publish** and do one band. Cleanest code; leaves the live wrong picture standing
  for however long publication takes, which is not ours to schedule.

### RULED by the architect, 2026-09-12, after the question stopped being one

**prikk 0.41.0 published the same day.** I wrote that Q1 was the owner's *"because it is a schedule
judgement, not a design one"*, and the schedule judgement has been made by events: the window (c) was
waiting for has closed, and (a)'s band no longer has a window to serve.

**What remains is design, and it is mine. Ruled: (b), refined — and this is a narrowing of my own
lean.**

**The band is exactly one version.** ≤ 0.39 the env model is correct; ≥ 0.41 `key status` answers;
**0.40 alone** is the odd version out. And 0.40 was the latest published release for roughly a day
before 0.41 superseded it.

**So on prikk 0.40 exactly, stikk reports signing readiness as unknown, names the cause, and names the
fix** — *prikk 0.40 moved seeds to a key directory and stikk cannot see them; prikk 0.41 answers this
directly, upgrade to it* — rather than probing. Three reasons:

1. **It is honest and actionable**, which is the standard, not merely honest. A user is told what stikk
   cannot determine, why, and what makes it determinable. That is the shape stikk already uses below the
   0.28 floor.
2. **The probe's cost is permanent and its benefit is not.** `key public --role` needs its own parse and
   its own fixtures, re-verified at every re-baseline from now on, to serve one version that was current
   for a day.
3. **It cannot mislead.** The probe answers presence but says nothing about `binding`, so a 0.40 user
   would get a confident `[AUT ✓]` on a repository where the id may not be the key that signs — the
   defect F4 exists to remove, reintroduced in the band built to avoid a different one.

**What I am giving up, and it is real**: a user on 0.40 sees commit and seal unavailable when they could
commit. That is a worse outcome than the probe would give them, and I am choosing it because the
alternative is a permanently-maintained code path that can state something wrong. **The owner may
overrule; if 0.40 turns out to have real users, (a) is still available and nothing here forecloses it.**

### The original question, for the record

**My lean is (a)**, and the reason is that (b) and (c) both accept a user being told the wrong thing —
or nothing — about whether they can commit, on a published prikk, to save code we would delete later.
**This project has repeatedly chosen the honest surface over the smaller one**, and the probe is
sanctioned by prikk rather than invented by us.

**What makes it a question rather than a ruling**: (a) commits us to a band whose only purpose is a
window that may close in days, and if it closes before the increment ships we will have built and
reviewed something nobody runs. **That is the owner's call because it is a schedule judgement, not a
design one.**

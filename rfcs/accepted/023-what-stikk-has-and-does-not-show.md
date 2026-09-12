# RFC 023 — Three things stikk has and does not show, and a control it says it has

**Status.** **Accepted by the project owner 2026-09-12**, Q1 left unruled and **resolved by the architect the same day — by a fourth option none of the three was** (below). Proposed 2026-09-12. The next 0.6.0 increment after RFC 022. Groups three small gaps that
share a shape — **stikk holds the information and withholds it** — and records a fourth thing found
while tracing them, which is not that shape at all.
**Tracks.** `FL-05`/`FL-06`/`FL-10` (the key id), `FR-111` (the glossary), `FR-104`, `C-S2`, `C-I1`,
`T-T4`, `ER-02`.
**Touches.** `stikk-prikk` (a key-id read), `stikk-core` (`present`, `glossary`), `stikk-tui`
(the glossary overlay), `docs/src/reference/threat-model.md`.

## Summary

Three things a user cannot see that stikk already knows or already ships. None is a defect in the sense
of a wrong answer — each is an answer withheld — and all three are small.

**The fourth item is different in kind.** Tracing the first three found that **`C-S2` — a control the
threat model's own coverage table lists as existing — has no implementation anywhere**, and as written
it may not be implementable without doing the thing RFC 017 condemned.

## Findings

### F1 — a Windows user on the supported floor gets a baffling refusal, and stikk knows why

RFC 022's widened suite found that prikk 0.28 cannot commit a file in a subdirectory on Windows
(`path.to_str()` yielding `src\main.rs`, refused by prikk's own `RepoPath::parse`; fixed in prikk
0.29.0 by RFC 124's separator-safe converter).

A Windows user on prikk 0.28 — **a supported configuration** — committing an ordinary project sees:

```
error: invalid name: backslashes are not allowed in repository paths
```

**Verbatim, honest, and baffling: they typed no backslash.** stikk knows the cause, the affected range,
and the fix, and says none of it.

**The mechanism already exists** — `present()`'s message-shape recognition, as used for schema skew
(RFC 012 F-e) and the full queue (RFC 017 F4). This is a third instance of a solved pattern, not a new
one.

### F2 — four glossary explanations are written, tested, and unreachable

`glossary.rs` ships four `GlossaryEntry` values with full explanations (`.prikkignore`,
`SCHEMA_SKEW_CODE`, `BUNDLE_DECODE_SKEW_CODE`, `FULL_QUEUE_CODE`). **`code_entries()` is iterated
nowhere outside `glossary.rs`** — zero call sites in the whole workspace. A refusal card surfaces a
code's *name* in its `glossary:` line; nothing renders the text that explains it.

**This is `FR-111`'s unbuilt half**, found during RFC 016's review and carried since. The entries are
authored, reviewed, covered by tests, and inert.

**And it cannot be fixed by rendering alone**, which is why it is grouped here rather than done long
ago: RFC 018 tried wrapping the glossary and reverted it, because without scroll, wrapping turned
truncated-but-present into absent — **as few as one of eleven terms reachable at 80×24**.
`Overlay::Glossary` carries no offset and every render is `.scroll((0, 0))`. **Wrap, scroll, and the
explanations are one increment or none.**

### F3 — the AUTHOR key id, open since RFC 014

`FL-05` step 5 requires commit's confirmation to summarize *"the AUTHOR key id to be used."* It shows
`Consumes: AUTHOR` — a capability, not an id. `FL-10` inherits the same requirement; `FL-06` wants the
MAINTAINER id.

RFC 014's review deferred it **with an instruction naming where it should land** — *"reading a key id
belongs in its own module… it will be wanted again for the MAINTAINER key id in the seal ceremony and
in Trust & Keys. Build it once, there."* RFC 016 was that site, and **I wrote its handoff without
re-reading RFC 014's review**, so the instruction was never carried. Recorded in 0.4.0's proposal as my
error; unbuilt since.

**A key id is not key material.** `C-I1` is presence-only for **seeds**; an id is an identifier prikk
itself prints. The module reads `PRIKK_*_KEY_ID`'s value and **must not widen `env.rs`'s seed guard** to
do it.

### F4 — `C-S2` is listed as a control and does not exist *[different in kind]*

The threat model says:

> **C-S2** stikk recognizes the documented example public inputs and **flags them as unsafe**
> persistently (FR-104), by pattern of the public value — never by storing the secret.

**There is no implementation.** Zero matches in the workspace for it or anything like it. And the threat
model's own coverage table (`§ No key material in stikk`) lists `C-S2` beside `C-I1a–d` and `SEAM-06`
**as though all three were in force.** Two are; this one never was.

**Why it was never built is the interesting part, and it is not neglect.** Three things have to be true
at once, and until recently none was:

1. **stikk must see a public value.** It sees a key *id* from the environment and, at **prikk ≥ 0.34**,
   adopted `public_key`s from `trust maintainer list --format json`. The second arrived three weeks ago.
2. **stikk must know which values are compromised.** prikk's `security-setup.md` says *"Any seed or key
   values published in Prikk's README, quick start, docs, tests, review packages, or issue comments are
   public examples… compromised by publication."* Today that is **three distinct 64-hex values** across
   prikk's docs — two obviously synthetic, one derived.
3. **stikk must get that list from somewhere defensible.** And here is the problem.

**The list lives in prikk's prose. There is no CLI surface that names it.** Encoding values read from
upstream documentation is *precisely* what RFC 017 F2 condemned — five classifier arms written from
prikk's prose rather than captured from its output, dead since 0.1.0. **A hardcoded list of example keys
would be the same defect wearing a security hat**, and it would drift silently every time prikk adds a
doc example.

See Q1.

## Decisions

1. **Deliver in two handoffs.** **A**: F1 and F2 — both are display, both are small, and F2's three
   parts (wrap, scroll, explanations) are one thing. **B**: F3, the key-id module, which touches four
   surfaces and has a `C-I1` boundary to hold. **Not one handoff**: RFC 021 Decision 5's lesson, and
   these are independent enough that bundling buys nothing.
2. **F1 uses `present()`'s existing shape recognition**, keyed on a captured message, with prikk's words
   verbatim beside the gloss (`ER-02`). The gloss names the range and the fix: **prikk 0.28 on Windows;
   upgrade to 0.29**.
3. **F2 is wrap + scroll + explanations together**, or not at all. Shipping wrap alone is a regression
   this project has already made and reverted once.
4. **F3 builds the module once**, for `FL-05`, `FL-06`, `FL-10` and a future Trust & Keys — as RFC 014's
   review asked — and **does not widen `env.rs`'s seed guard.**
5. **`C-S2` is marked *not implemented* in the threat model now**, whatever Q1 decides. A coverage table
   that lists a control nobody built is the `T-T4` shape aimed at ourselves: **the document most
   responsible for saying what protects a user is the one making a claim it cannot support.** Correcting
   the record does not wait on deciding the fix.

## What this RFC does not do

**It does not build Trust & Keys** (`FR-104`'s view) — F3 builds what that view will read, nothing more.
**It does not construct three-valued `Ready`** — `FR-103`'s increment, which will read the same
`trust maintainer list` surface Q1 turns on, and should probably follow whichever way Q1 goes.

## Q1 — RESOLVED 2026-09-12: **(d)**, which I did not see when I wrote the question

**The question assumed stikk must either transcribe prikk's prose or ask prikk for a surface. It has to
do neither: prikk's own binary will derive the comparison values, and the suite can verify it.**

Verified against a real prikk 0.38.0. prikk's docs publish **two seeds and one public key**:

```
PRIKK_AUTHOR_SEED="00112233…"        (seed, public key NOT published)
PRIKK_MAINTAINER_SEED="11112222…"    (seed)
--public-key "a00899df…"             (public key)

$ prikk key public --seed-env <the maintainer seed>
a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91   ← exactly the published one
$ prikk key public --seed-env <the author seed>
3ccd241cffc9b3618044b97d036d8614593d8b017c340f1dee8773385517654b   ← derivable, never published
```

**So the design is:**

- stikk stores **public keys only** — `3ccd241c…` and `a00899df…`. No secret, satisfying `C-S2`'s own
  *"never by storing the secret"* literally rather than by argument.
- **A suite test derives them from the published seeds using prikk's own `key public` and asserts the
  stored list still matches.** The values stikk compares against are therefore **captured from prikk's
  output**, verified every run — not transcribed from prose. **RFC 017 F2's objection dissolves**, and it
  was the whole reason I could not rule this.
- At runtime stikk compares adopted `public_key`s from `trust maintainer list --format json`
  (prikk ≥ 0.34) against that list.
- **`C-I1e` is not violated.** It binds the product, not the harness — RFC 019's ruling — and the harness
  spawns `key public` via `std::process::Command`, never through `CliBackend`. The seeds appear only as
  test inputs, and they are compromised-by-publication rather than secrets.

**What survives, weakly: drift.** If prikk publishes a *new* example seed, nothing notices. **So the
letter of (a) is still worth sending and is no longer blocking** — the control works without it. Drafted
as a follow-up, ranked below the increments, and explicitly not a dependency.

**(c) is not foreclosed.** `C-S2`'s wording remains the owner's; (d) satisfies it as written, so no
amendment is needed, but narrowing it later remains available.

**Scheduling:** `C-S2`'s implementation reads `trust maintainer list`, which is the same surface
`FR-103`'s three-valued `Ready` needs. **They ship together, as a third increment** — not in Handoff A
or B. **Decision 5 does not wait for it**: the threat model is corrected in Handoff A.

### The original question, for the record

**Q1 — where does the compromised-key list come from, if anywhere?**

- **(a) Ask prikk for a surface that names its own published example values.** A `prikk key check --public-key <HEX>`,
  or a documented constant list in `--format json` — something a consumer can read rather than transcribe.
  **Four of four such requests have landed**, twice within a day, and this one is squarely the shape they
  respond to: it exposes something they already know, commits them to no new semantics, and has a named
  consumer. **It also serves them**: any front-end, and their own `verify`, has the same problem.
- **(b) Encode the three values with provenance and re-check them every re-baseline**, exactly like a
  captured fixture, accepting that the source is prose. Honest about what it is, and it works today — but
  it is the RFC 017 F2 shape with a good reason, and the re-baseline checklist grows an item that no
  command can verify.
- **(c) Amend `C-S2`** to what stikk can do without a list — for example, flag only what prikk itself
  flags, if it ever does — and record the rest as an upstream dependency.

**My lean: (a), with (b) as the interim** — encode the three, provenance and all, *and* send the letter,
so the control exists now and stops depending on transcription later. **But `C-S2` is a threat-model
control and its wording is the owner's**, not mine to amend, and (c) is a real option I do not want to
foreclose by treating (a) as obviously right.

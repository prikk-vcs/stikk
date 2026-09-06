# Handoff — the seal ceremony (v1)

**Companion to:** [RFC 016](../../accepted/016-the-seal-ceremony.md) (Accepted 2026-09-06, as amended
that day on prikk's replies). Inherits its state.
**Realizes:** the 0.4.0 increment after
[RFC 017](../../done/017-prikk-0-33-rebaseline-and-classifier-provenance.md), whose F5 and F6 are this
increment's preconditions.
**Design items:** `FR-052` (as amended), `FR-103` (to be amended here), `FR-104`, `FR-121` tier 3,
`FL-06`, `AC-01…04`, `OPL-04`, `C-T2c′` (three-valued outcomes), `C-T4d` (capability honesty),
`C-T2a`/`C-T2b`, `T-T4`, `ER-02`, `NFR-S04`, `NFR-R02`/`CT-05`, `TS-01`/`TS-02`/`TS-03`.

> **This is the most consequential thing stikk will ever offer.** Sealing freezes queued patches into
> immutable, MAINTAINER-signed history. There is no undo — not in stikk, not in prikk. Every control
> this project has built exists for this moment: preview-first, tier-3 confirmation, informed consent,
> verbatim truth, no auto-retry.
>
> **And the increment's first job is not seal.** It is fixing a capability claim stikk has been making
> since 0.1.0 and cannot support. Build that first (§3), on its own, for all eight operations that
> need it — not as a seal feature.

---

## 1. Scope

**In**, in this order:
1. **Verify against the real binary** (§2), at both ends of the supported range.
2. **Three-valued `MaintainerReadiness`** in `stikk-model` (§3) — the foundation, eight operations wide.
3. The **badge and capability** (§4) — unknown must never render as a pass.
4. The **seam**: `seal` (§5), the second method that writes.
5. **Prevention** of the empty-queue and cross-ref refusals, and widening cross-ref recognition (§6).
6. The **ceremony** in `stikk-core` and `stikk-tui` (§7), including the consent step (§8).
7. The **trust refusal's presentation** (§9) — RFC 017 F5's deferred half.
8. **Amend `FR-103`**, and close RFC 017's full-queue next-step (§10).

**Out — and two of these are traps, not merely scope:**

- **`prikk trust maintainer list` / `check`.** prikk accepted and ruled them (their RFC 138) on
  2026-09-06 and **has not released them**. Not in 0.33.0, not in our range. **Nothing in this
  increment may reference, detect, or prepare for them beyond the type shape §3 already gives.**
- **🪤 Resolving readiness from `prikk verify` output.** `verify` prints `sealed-block <id>: <key_id>`,
  which looks like an answer and is not: it is *historical signer attribution*, not current policy (a
  revoked key still prints), and it **does not exist before the first seal** — exactly when the
  ceremony asks. RFC 016 F3 records this as a **rejected approach** by name. Do not adopt it, and if
  it looks tempting while you are in there, that is the trap working as documented.
- **🪤 `NFR-P05`**: no implicit full verification on open. Readiness must not become a reason to run
  `verify`.
- **Typed confirmation for seal.** Owner-ruled untyped (RFC 013 Q3). **Two deliberate acts, not three.**
- **Re-reading orientation between consent and execute.** RFC 016 Q2 ruled no — the change token
  already covers it (§7).
- Merge, sync, adopt-tag, tag create, branch create/close. §3 serves all eight; you build none of them.
- Queue review (`FR-051`) — the next increment.

---

## 2. Verify before you build

**Two binaries: 0.33.0 and 0.28.0** (the floor). Seal's surface must work across the whole range.

**Already confirmed by me, from prikk's source at three tags — confirm against the binaries anyway:**
`--allow-no-audit` exists at **0.28.0, 0.30.0 and 0.33.0** with identical wording
(`"seal scaffold requires --allow-no-audit"`). The flag is not a 0.33-only surface.

**Capture, at both ends:**
- A **successful seal**'s full stdout. This is a parser input and needs golden fixtures (`TS-03`).
  **Report whether the shape differs between 0.28.0 and 0.33.0** — if it does, that is a finding and
  the parser is version-gated, as `changes_view` already is.
- The **empty-queue** refusal (`active WAL has no patch records to seal`).
- The **cross-ref** refusal — note it is worded `requested seal ref is`, where commit says
  `requested ref` (§6).
- Both **trust refusals** — RFC 017 captured these; re-confirm they still arrive from `seal`
  specifically, since RFC 017 provoked them through a gated operation generally.

**One thing I want you to look for that I cannot predict.** prikk 0.33's `seal` prints
*"audit plugins remain later PRs"* on success. **Report its exact text and whether it appears at
0.28.0 too.** It is prikk's own words about the scaffolding, and whether stikk surfaces it is a copy
decision I would rather make from the real string than from prikk's changelog.

---

## 3. `MaintainerReadiness` — the foundation *[build first, alone, and get it reviewed in your head before moving on]*

Today `Readiness { maintainer_ready: bool }` is set by `env.rs` from **presence alone**
(`is_set(MAINTAINER_KEY_ID) && is_set(MAINTAINER_SEED)`). prikk requires the key to be **adopted in
the repository's trust policy**. stikk has never implemented that half, and the badge has been
claiming it since 0.1.0.

Replace the bool with a three-valued type in `stikk-model`:

```rust
pub enum MaintainerReadiness {
    /// Key material present **and** adopted in the repository's trust policy.
    Ready,
    /// Key material absent.
    NotReady,
    /// Key material present; adoption **unverifiable** — no supported prikk can answer.
    Unknown,
}
```

**`Ready` is unconstructible today, and that is correct, not a gap.** No prikk in 0.28–0.33 exposes a
way to check adoption, so `env.rs` yields `Unknown` on presence and `NotReady` on absence. Document
the variant the way RFC 017 documented `StikkError::IntegrityFinding`: unreachable now, present
because it is the shape the answer arrives in.

**Why three values and not a bool plus a caveat**, so you can defend it in review: `C-T2c′` already
rules this exact shape for the three-valued author signature, where **Unverifiable must never render
as a pass**. A green check on a claim stikk cannot verify is the same error. And when prikk ships the
check, `Unknown` resolves into `Ready`/`NotReady` **with no change to this type or the UI's shape** —
a bool-plus-caveat would have to be re-architected at that point. That claim is now testable against a
real release; it is the RFC's own stated bet.

**This is not seal's type.** prikk's `GatedOperation` covers **eight** operations — seal, merge, sync
build, sync seal, sync adopt-tag, tag create, branch create, branch close. Put it where all eight can
read it and name them in the doc comment. Seal is its first consumer, not its owner.

**Semver:** a public struct field changes type. Per RFC 011, for 0.x the **minor** is the breaking
position and 0.4.0 has not shipped — so this is absorbed, not deferred. Say so in the review request.

**Wording rule, everywhere adoption is named** (RFC 016 F2 as amended, and prikk's own RFC 138 §7.3):
adoption means **prikk accepts that key's signatures on objects**. It is **object trust, not ref
authority** — adopting a key never lets it move a ref. Wording that lets a reader conclude "may publish
here" is a defect, not a nuance. I had this wrong in RFC 016's first draft; do not inherit it.

## 4. The badge and the capability

**`Capability::derive` grants `Maintainer` on `Ready` *or* `Unknown`.** The affordance stays: hiding
seal from someone whose key *is* adopted would be its own wrong picture (`C-T4d`). What changes is what
stikk *claims*, not what it *offers*.

**The badge renders three states distinctly, and `Unknown` never renders as a pass.** `[MNT ✓]` is for
`Ready` only — which means, today, it is never rendered at all.

**Do not invent the visual language.** `FR-035`'s three-valued author signature already solved this
problem in this codebase; find how Unverifiable is rendered there and match it. One concept, reused.

## 5. The seam — `seal`

The second `Prikk` method that writes. Follow `commit`'s shape exactly (`stikk-prikk/src/lib.rs`,
`cli_backend.rs`, `null_backend.rs` with `with_seal_*` builders).

- **`--allow-no-audit` is passed unconditionally**, every time. prikk refuses without it as a **usage
  error (exit 2)**, and per RFC 014 F6 an exit 2 from stikk is a **stikk bug**, never a user-facing
  refusal. There is no configuration for this and no code path that omits it.
- Parse the success output into a result type, confined and version-gated (`UD-02`), pinned to §2's
  captured fixtures. **Carry prikk's own notes verbatim** as `commit` does — a `Vec<String>`, never
  fixed named fields (RFC 014's finding: the note set is not stable across versions).
- Refusals route through the classifier as usual.

## 6. Prevention, and the cross-ref widening

**Prevent both refusals stikk can see coming**, as RFC 014 does for commit — from
`Orientation::queued_target` and the queue count, **never** by parsing prikk's refusal (`C-T2b`):

- **Empty queue** — stikk already knows the count. Do not arm a ceremony that will be refused.
- **Cross-ref** — `queued_target` already says which ref the queue belongs to.

**Then widen cross-ref recognition anyway** (RFC 016 decision 5), for the race that survives
prevention. Seal words it `active WAL is owned by {actual}; requested seal ref is {ref}` — note
**`requested seal ref is`**, where commit says **`requested ref`**. `is_cross_ref_conflict` currently
requires `"requested ref"`, so it does **not** fire for seal.

**Match on `active wal is owned by` alone** — the clause both commands share. Do not add a second
alternation for seal's trailing phrase: at 0.33.0 these two messages already differ in *both* the
trailing clause and the class prefix (commit's became `precondition not met:`; seal's carries no prefix
at all). The trailing phrasing is exactly the part that drifts. **Both messages must be captured
fixtures**, and there must be a test proving both classify `CrossRef`.

## 7. The ceremony

Use RFC 013's machinery unchanged: `preview() → PreviewToken → confirm() → ConfirmedToken →
execute()`. Neither token has a public constructor and `execute` takes by value, so skipping a step
**does not compile**. Do not add a parallel path.

- **Tier 3** (`FR-121`), `Capability::Maintainer` required, `capability_gate` as `commit` uses it (the
  palette unification landed with RFC 014 §6 — it is not a blocker).
- **The confirmation states plainly that a trust refusal is possible and would come from prikk**, and
  says what stikk does and does not know about adoption. Driven by the `MaintainerReadiness` state,
  not shown unconditionally.
- **It never promises success.**
- **No re-read of orientation between consent and execute** (RFC 016 Q2). RFC 003's change token
  already carries the queued count and target ref, so a queue that moves *is* caught by the mechanism
  built for it. A second read would be a second mechanism for one property, and would imply an
  atomicity guarantee stikk cannot give — it holds no lock across think-time (`NFR-R02`, `CT-05`).

Every prikk-sourced string through `text::inert` (`C-T2a`); no next-step derived from message text
(`C-T2b`).

## 8. The consent step — and the sentence that must not appear

**Its own step. Unchecked. Cannot be defaulted.** Not a line in the confirmation summary — a distinct
act, because the act it precedes is irreversible.

**The rule on its copy, and this is the part I want you to get exactly right.** RFC 016 originally
justified this step as *"standing where prikk required an acknowledgement."* prikk answered that
`--allow-no-audit` is **scaffolding** — their seal calls itself a scaffold, a successful seal prints
that audit plugins are later work, and the milestone that replaces it is unscheduled. They then told us
plainly they **cannot say what replaces it, because nobody has decided**, and advised:

> Tie it to the ceremony instead of to our flag. Then the day the flag changes, your wording is still
> true and only ours moves.

**So the copy states what is true of the ceremony** — no audit ran; this freezes history; it cannot be
undone. **It must not justify itself by prikk's flag**, must not say prikk requires this
acknowledgement, and must not imply the requirement is permanent. The step survives the flag's removal
with no edit.

**A test should assert this**, not just a reviewer: the consent copy contains no reference to
`--allow-no-audit` as its reason. That is a claim whose truth depends on someone else's unscheduled
decision, and this project does not make those.

## 9. The trust refusal's presentation — RFC 017 F5's other half

RFC 017 routed `maintainer signer key id … is not trusted by policy` and `… does not match trusted
key …` to `StikkError::NotReady`, which `present()` already sends toward `Target::TrustKeys` carrying
prikk's verbatim words. **The classification is done; the guidance is not.**

Give it next-steps and copy that say what actually has to happen: the key must be **adopted in this
repository's trust policy**, which is done outside stikk (`prikk trust maintainer add`), and which stikk
**cannot verify afterwards** on any supported prikk. Say that last part — a user who adopts the key and
comes back to an unchanged `Unknown` badge must not think stikk is broken.

Object trust, not ref authority (§3's wording rule), here too.

## 10. `FR-103`, and one deferral this increment closes

**Amend `FR-103`** — *"Trust management: adopted maintainer keys list"* — to what is knowable on
0.28–0.33, as `FR-051` and `FR-052` were. Record that the upstream ask is **filed, accepted and ruled
(prikk RFC 138: both `list` and `check`, with `--format json`), and unreleased**, and that stikk's
support floor means `Unknown` remains reachable even after it ships. Do not write a date.

**Close RFC 017's full-queue next-step.** `present()` currently offers only `Refresh` for the
full-queue refusal, with a comment saying *"No Seal ceremony exists yet to jump to (RFC 016)."* One
now exists — add it. That is a navigation to a separate, fully-confirmed operation, not a retry
(`NFR-S04` is intact).

---

## 11. Test plan

- **Readiness (`stikk-model`)**: all three states; `Capability::derive` grants `Maintainer` on `Ready`
  and `Unknown`, `Viewer` on `NotReady`; `read_only` still collapses everything.
- **The badge (`TS-01`, headless render)**: assert `Unknown` renders **distinctly from** `Ready` and
  **contains no pass/✓ marker**. Assert the *absence*, not merely the presence of something else —
  `C-T2c′` is a prohibition.
- **Seam (`TS-03`)**: seal's success output parses from captured fixtures at **both** 0.28.0 and
  0.33.0; a malformed shape **refuses** (`Environment`), never guesses; notes are carried verbatim as
  a list.
- **Classifier**: both cross-ref wordings (commit's and seal's, both captured) classify `CrossRef`.
- **Prevention (`TS-02`, scripted `NullBackend`)**: an empty queue and a cross-ref target both stop
  **before** the ceremony arms — assert prikk was never called.
- **The ceremony**: the happy path; the consent step cannot be skipped (a compile-fail doc-test if the
  existing shape supports one, otherwise a test that the ceremony refuses without it); a trust refusal
  mid-ceremony renders prikk's words verbatim with Trust & Keys guidance; a token whose change token
  no longer matches yields `Stale`, not a retry.
- **Consent copy (§8)**: contains no `--allow-no-audit` justification.
- **Hostile input**: a repository-sourced string in the ceremony renders inert and produces no
  actionable next-step.
- Gates green; state the count delta.

## 12. Acceptance criteria

1. §2's captures reported **before** anything else, both binaries, including seal's success shape at
   each end and whether they differ.
2. `MaintainerReadiness` is three-valued, lives where all eight gated operations can read it, names
   them, and documents `Ready` as currently unconstructible and why.
3. Adoption is described as **object trust, not ref authority**, everywhere it is named.
4. `Unknown` never renders as a pass, proven by an absence assertion.
5. `Capability::derive` grants `Maintainer` on `Ready` or `Unknown`.
6. `seal` passes `--allow-no-audit` unconditionally; fixtures captured at both ends of the range.
7. Empty-queue and cross-ref are **prevented**, and the cross-ref classifier matches both wordings on
   the shared clause.
8. The ceremony uses RFC 013's machinery unchanged; consent is a distinct, undefaultable act; no
   orientation re-read between consent and execute.
9. The consent copy is true of the ceremony and independent of prikk's flag, with a test.
10. The trust refusal has guidance that names adoption and admits stikk cannot verify it afterwards.
11. `FR-103` amended; RFC 017's full-queue next-step now offers the ceremony.
12. Nothing references `trust maintainer list`/`check`; nothing reads `verify` output for readiness.
13. Gates green; demos build; a screenshot of the ceremony (tiled window, never floated).
14. Nothing tagged or published.

## 13. Submit

Package to `.git-exclude/review-request/016-the-seal-ceremony/review-request-v1.md`.

**Lead with §2's captures**, then the readiness type — those are the foundation and the part I cannot
check by reading a diff.

Then tell me what contradicts RFC 016. **Ten increments running have each produced a finding that
contradicted my RFC or my handoff, and the last one produced two that were mine.** This RFC has already
been amended once on evidence I did not have; assume it is wrong somewhere and that you are the one
positioned to see it.

**One specific invitation.** If the consent step, as specified in §8, reads as ceremony for its own
sake once you have it on screen — say so. It is justified by irreversibility, and irreversibility is
real, but I have not seen it rendered. **A second deliberate act that feels like a formality teaches
users to click through the first one**, and that would be worse than not having it.

**Push it yourself once the review says Approved** (`.git-exclude/specs/02-implementer-handoff.md` §6).

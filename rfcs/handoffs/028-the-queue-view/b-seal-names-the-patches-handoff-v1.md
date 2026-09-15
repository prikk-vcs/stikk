# Handoff B — seal names what it freezes (v1)

**Companion to:** [RFC 028](../../accepted/028-the-queue-view.md). Accepted 2026-09-13, Q1 ruled (b); Handoff A
delivered.
**This handoff is decision 4:** seal's confirmation names the patches it freezes, at prikk ≥ 0.39. It is
separate from A because it changes a signing confirmation (RFC 016), which deserves its own review.
**Starting point:** `origin/main` with RFC 028 Handoff A delivered. You build on `Prikk::queue`,
`QueueReport` and core's Queue words as A shipped them.
**Design items:** `FR-052`, `FL-06`, `C-T2a`, `C-T2b`, `C-T2c′`, `C-T4d`, RFC 016, RFC 024.

> **The rule for this card, and the reason for it.** A seal confirmation holds the sentence a maintainer most
> needs, the consequence and its trust-refusal warning, and its affordance. RFC 024 found that card shipping
> for three releases with its footer clipped. **So the list of patches is the only thing on the card that may
> give up height, and when it does, it says how many it did not show.** Nothing else is ever clipped to make
> room for it.

---

## 1. Scope

**In, in this order:**
1. Seal's `compute` reads the queue once (§2).
2. `ConfirmationSummary` carries what the seal freezes, in core's words (§3).
3. The confirmation card renders it, yielding height by the rule above (§4).
4. Tests (§5), docs, changelog and Breaking (§6).

**Out:**
- a patch's operations on the card (the Queue view lists them);
- any change to the consent step (`SealConsent`) or to seal's two acts (RFC 016 decision 3);
- scrolling inside the confirmation;
- commit's card.

## 2. One queue read in seal's `compute` (decision 4)

**Today** seal's `compute` takes the count and target from `orientation` (RFC 028 F4).

- **At prikk ≥ 0.39**, the empty-queue block, the cross-ref block, `counts` and the list come from **one
  `prikk.queue(repo)` read**, so the count and the list describe the same moment.
  - **Empty:** `count == 0` blocks, with today's words.
  - **Cross-ref:** `QueueTarget::Ref(t)` with `t != reff` blocks, with today's words.
  - **Missing or malformed metadata, or no target:** not blocked. prikk decides, as today, when prose gives a
    sentinel.
- **Below 0.39**, `queue` answers `Unreported` from the same prose read, so the blocks are unchanged. The card
  says the list is not reported (§3).
- **Orientation is still read for one thing: prikk's current branch**, for RFC 029's notice. That is not a count,
  and a queue or branch that moves between the two reads makes the confirmation stale through RFC 030's change
  token. **Say in the code why two reads are safe here, in one sentence.**

## 3. What the summary carries — words in core

**`ConfirmationSummary` gains what the seal freezes**, as a small type in core. It is `None` for commit, and for
seal it has two forms:

| Form | When | Carries |
|---|---|---|
| **listed** | prikk ≥ 0.39 | one row per patch, in queue order, plus an optional foot |
| **unlisted** | below 0.39 | one line |

**The words, exactly:**

- **A patch row:** `{short id}  {message}`.
  - `{short id}` is the **same short form History's block rows use** for a block id.
  - `{message}` is **the message line core's Queue view already produces**: the first line with `…` for more, or
    `(no message)`. Reuse that function; do not write a second.
  - When the message is not reported (0.39–0.41), the row is the short id alone.
- **The foot, listed at 0.39–0.41 only:** `prikk 0.{minor} does not report a queued patch's message.`
- **Unlisted:** `prikk 0.{minor} does not list queued patches.`
- **The remainder line**, used when §4 cannot show every row: `and {m} more — Esc, then Q, lists all {n}`.
  Produce it with a core function, so the TUI decides only *how many* fit, never *what they say*.

**Every row carries repository text** (an id, a message), so every row renders through `inert` (`C-T2a`).

## 4. The card

**Where:** directly under the counts line (`{n} patches`), indented as the target ids are, and **above**
`Consumes:`.

**The height rule.** The card's other prose and its affordances are laid out first, in rows. The list is then
given what remains:
- **If every row fits**, every row shows, followed by the foot if any.
- **If not**, as many rows as fit **with one row kept for the remainder line**, then that line. `{m}` is exactly
  the rows not shown.
- **If not even one patch row and the remainder line fit**, the remainder line alone reads `and {n} more — Esc,
  then Q, lists all {n}`.
- **The foot, when present, is kept whole before any patch row is given up.** It is the statement of what the rows
  cannot say.

**It must be exact, not estimated.** RFC 024's defect was an estimate. Count wrapped rows the way `Panel` does, and
test it at the heights below. If `Panel` cannot give the list the remaining height without a change to `Panel`
itself, **make the smallest change that gives it a measured region, and say what you changed.**

**Unlisted** is one line, and never yields.

## 5. Tests

**Core:**
- seal's summary at 0.42 lists both patches with their messages;
- at 0.41 short ids only, plus the foot;
- at 0.30 the unlisted line;
- `counts` equals the queue's `count`;
- both blocks come from the queue read at ≥ 0.39 (a `NullBackend` whose Orientation disagrees with its queue
  proves which was read);
- the remainder function's words, byte-exact.

**TUI captures of the seal confirmation, at 80 columns:**
1. **80×24:** twelve patches, a maintainer consequence at its longest (`Unknown`), and RFC 029's branch notice.
   - Every non-list prose line is whole, the consequence included.
   - The footer is present.
   - The remainder line's `{m}` equals twelve minus the rows shown.
2. **80×50:** the same twelve, every row shown, and no remainder line.
3. **80×24:** two patches at 0.41: rows plus the foot.
4. **80×24:** unlisted, below 0.39.
5. **The smallest height at which the footer still renders:** the list yields entirely to the single remainder
   line, and nothing else is clipped.

**Real-binary suite:**
- **At 0.42**, with two queued patches: `seal_preview`'s summary names both, and its short ids are prefixes of the
  queue's ids, with the messages as committed. **Seal it**, and the block's patch ids begin with those short ids,
  so the card named what froze.
- **At 0.28:** the unlisted line, and `counts` equal to Orientation's count.

## 6. Docs, changelog, Breaking

**On delivery:**
- **`FR-052`** records that the ceremony names the patches at ≥ 0.39: the short id and message, with a remainder
  line when the card cannot hold them all. Below 0.39 it says prikk does not list them. The 2026-09-05 and
  2026-09-12 amendments are kept, as the requirement says.
- **`FL-06` step 3** says the confirmation names the patches, and its *"never the patch ids"* is corrected with a
  dated note.

**Changelog, `## Unreleased`:**
- **`### Changed`**: on prikk ≥ 0.39, the seal confirmation names each patch it will freeze, with its message on
  prikk ≥ 0.42. When they do not all fit, it says how many more there are, and where to see them.

**Breaking, by API diff.** Expect `ConfirmationSummary`'s new field, the new type, and the remainder function's
export.

## 7. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite and
supply-chain run ids at one SHA, and Docs after the push.

## 8. Acceptance criteria

1. **Seal's `compute`:** one queue read for the blocks, the count and the list at ≥ 0.39; Orientation only for the
   current branch, with the one-sentence reason in the code.
2. **The summary:** the listed and unlisted forms, in §3's exact words; the Queue view's message function reused;
   every row inert.
3. **The card:**
   - §4's height rule, exact;
   - the consequence and the footer never clipped;
   - `{m}` true in every capture.
4. **§5's tests, captures and suite legs.**
5. **§6's docs, changelog, and a Breaking table built by API diff.**
6. **Eight gates** under the toolchain rule; run ids at one SHA.
7. **Nothing tagged or published.**

## 9. Submit

Package to `.git-exclude/review-request/028-b-seal-names-the-patches/review-request-v1.md`.

**In this order:**
1. **The 80×24 capture with twelve patches**, and the arithmetic behind its `{m}`.
2. **The smallest-height capture.**
3. **The suite leg where the sealed block's ids begin with the card's short ids.**
4. **The Breaking table.**

**And tell me whether the remainder line's *"Esc, then Q"* reads right in the ceremony.** It sends a maintainer
out of a signing flow to look. If that feels wrong at the keyboard, say so, and say what would read better.

**Push once approved.**

# Handoff C — the two render leftovers, not carried a third time (v1)

**Companion to:** [RFC 026](../../accepted/026-readiness-after-the-key-directory.md), **F7**.
**Follows:** [Handoff B](b-readiness-rebuild-handoff-v1.md) — **do B first**, because both touch
`overlay.rs` and B's key-id work is the larger of the two diffs.
**Design items:** `C-T2b`, `ER-02`, `C-T4d`, RFC 023 F1/F2, RFC 024 §5.

> F7's words: *"Both have been carried twice. They land here or they get their own increment; they do
> not get carried a third time."* This is the increment. It is small on purpose, so that being small is
> never again the reason it slips.

---

## 1. The quote bar that stops at the first wrapped row

`overlay.rs::render_refusal` renders prikk's verbatim message inside a quoted region — `  │ ` on the
first row of each line, four spaces on every wrapped continuation. So a refusal long enough to wrap
loses the bar exactly where the reader most needs to know they are still inside prikk's words and not
stikk's.

**RFC 024 left this deliberately, and left a note saying so**, because fixing framing inside a sizing
change was the bundling that RFC 024 §1 ruled out. Its parting gift is that **it is now a
one-argument change** — the `"    "` passed to `wrap_indented`.

**What to check beyond the one argument:**

- `wrap_indented` counts the indent against the width, so the bar and the spaces are the same display
  width and the wrap points do not move. **Assert that**, rather than assuming it: a bar that reflows
  the text is a different change from a bar that colours it.
- The continuation rows must carry the **warn** colour on the bar and the **fg** colour on the text,
  matching the first row exactly. A half-styled continuation looks like a rendering bug rather than a
  quote.
- `render_refusals` (the plural, list form) — check whether it has the same shape and the same defect.
  If it does, fix both; if it does not, say so. **Do not assume from the name.**

**Render tests at 80 columns**, `TestBackend` buffer captures, with a verbatim message long enough to
wrap **three** rows — two only proves the second row, and the defect is about every row after the first.

## 2. The backslash gloss, hedging over a version stikk holds

`glossary.rs`'s `BACKSLASH_PATH_CODE` entry explains two opposite situations to every reader:

> *"On Windows with prikk 0.28, it is prikk's own defect and you typed no backslash … prikk fixed that
> in 0.29.0 — upgrade the prikk binary … Anywhere else, a file in the worktree really does have a
> backslash in its name; rename it outside stikk."*

**Both halves are true, and stikk already knows which one applies** — it holds the handshake version and
it knows the platform. Making a reader decide which paragraph is about them, using facts stikk has, is
the hedge F7 names.

**The narrowing, and its limit — this is the part to get right:**

- On **non-Windows at any version**, and on **any platform at prikk ≥ 0.29**, the first half cannot
  apply. Show the second half only.
- On **Windows at prikk 0.28 exactly**, both remain genuinely possible. Show both, in that order.

**Do not turn this into a single verdict.** RFC 023 F1 measured that
`backslashes are not allowed in repository paths` is prikk's **generic** validator, byte-identical from
0.28 through 0.38, and I re-verified it at three tags: a lone *"prikk 0.28 built this path, upgrade"*
would contradict the evidence for a Linux user whose file really does have a backslash in its name. The
dev team found that; it stands, and it is the reason this is a narrowing and not a rewrite.

**Shape:** the glossary is a static table of `&'static str` explanations today. Two entries selected by
context is the smaller change and keeps the table static; a context argument on the lookup is the other
option. **Pick one and say why** — if the static split makes the two texts drift apart, that is an
argument for the other, and I would rather read your reasoning than my guess.

**Tests:** the gloss is chosen in `present.rs`; cover all three cases (non-Windows, Windows ≥ 0.29,
Windows 0.28) hermetically, the way `paths.rs` tests its platform enum rather than the real platform.

## 3. Gates

The eight. The real-binary suite is untouched by this and must stay **fourteen of fourteen** after B —
name the run, and if it is not fourteen, that is B's regression and this handoff stops.

## 4. Acceptance criteria

1. Wrapped verbatim rows carry the quote bar, styled as the first row, with wrap points unchanged and
   that assertion tested.
2. `render_refusals` checked and reported either way; fixed if it shares the defect.
3. An 80-column render test with a message wrapping to at least three rows.
4. The backslash gloss narrowed by platform and version, with **both halves kept on Windows at 0.28
   exactly**, and the RFC 023 F1 evidence cited where the narrowing is written.
5. The gloss-selection shape chosen with the reasoning stated.
6. Three hermetic tests for the three cases.
7. Eight gates green; suite still fourteen of fourteen.
8. Nothing tagged or published.

## 5. Submit

Package to `.git-exclude/review-request/026-c-render-leftovers/review-request-v1.md`.

**Lead with the three-row refusal card at 80 columns**, before and after. One buffer capture settles
this in a way no prose about it can.

**Then tell me whether `render_refusals` shared the defect.** I have not read it closely enough to say,
and I would rather ask than guess in a handoff.

**Push once approved.** RFC 026 closes when this lands — A, B and C together — and moves to `done/`.

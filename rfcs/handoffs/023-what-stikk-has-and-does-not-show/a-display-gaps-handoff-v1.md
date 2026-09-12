# Handoff A — the two display gaps, and a threat-model correction (v1)

**Companion to:** [RFC 023](../../accepted/023-what-stikk-has-and-does-not-show.md) (Accepted
2026-09-12; **Q1 resolved by the architect** the same day, by a mechanism none of its three options
was — read it, it changes what a later increment will build).
**This handoff is F1, F2 and Decision 5.** F3 — the key-id module — is Handoff B and follows.
**Design items:** `FR-111`, `ER-02`, `T-T4`, `C-T2a`/`C-T2b`, `C-S2`, `NFR-A03`.

> **Neither of these is a wrong answer. Both are answers withheld** — stikk holds the information and
> a user cannot reach it. That makes them small, and it also makes them easy to under-build: the test
> for each is whether the thing is *reachable*, not whether the code exists.

---

## 1. Scope

**In:** F1's gloss (§2); F2's wrap + scroll + explanations, as one thing (§3); the `C-S2` correction
(§4).

**Out:** the key-id module (Handoff B). `C-S2`'s **implementation** — it reads `trust maintainer list`
and ships with `FR-103`'s three-valued `Ready`, as one increment; **this handoff corrects the record
only.** Trust & Keys' view. Anything that changes what `classify()` returns — F1 is a *presentation*,
and the message already reaches `present()` correctly.

## 2. F1 — the prikk 0.28 Windows refusal

**Capture it first.** Build real prikk **0.28.0**, and on a Linux box you can still provoke the
message's *shape* — but the defect itself is Windows-only, so **capture the real thing from the
Windows leg of the suite**, which already runs there. The message, from RFC 022's finding:

```
error: invalid name: backslashes are not allowed in repository paths
```

**Use `present()`'s existing shape recognition**, the third instance of a solved pattern — `.prikkignore`
(RFC 009 F5), schema skew (RFC 012 F-e), full queue (RFC 017 F4). A stable-substring constant in
`glossary.rs` beside the other four, a gloss constant in `present.rs`, checked before the generic
`(class, operation)` fallback.

**What the gloss must say, and what it must not:**

- **It is prikk's defect, fixed in prikk 0.29.0** — name the version, because the user's action is to
  upgrade prikk, not to change anything about their repository or their paths.
- **The user typed no backslash.** Say so. The whole reason this gloss exists is that prikk's message is
  true and reads like an accusation.
- **prikk's words stay verbatim beside it** (`ER-02`), as every other gloss does.
- **Do not claim the commit is impossible** — a top-level file commits fine at 0.28 on Windows; it is
  subdirectory paths. RFC 022's own test skips only the subdirectory half for exactly that reason. **A
  gloss that overstates the limitation is the same wrong-picture failure pointing the other way.**

**Test**: the captured message reaches the gloss; a render test proves the gloss and prikk's verbatim
text are both on screen at 80×24 — the RFC 016 C1 lesson, since this is a card with prose on it.

## 3. F2 — wrap, scroll, and the four explanations, as one thing

**RFC 018 shipped wrap alone and reverted it**, because without scroll it turned truncated-but-present
into absent: **as few as one of eleven terms reachable at 80×24**. That revert is recorded in
`render_glossary`'s own comment. **Do not repeat it: the three parts land together or none do.**

**Scroll.** `Overlay::Glossary` carries no offset and every render is `.scroll((0, 0))`. It needs an
offset in the variant, key handling that moves it, and a clamp that cannot scroll past the end. The
overlay layer already has selectable lists with cursors — **follow whatever is there rather than
inventing a second scrolling idiom.**

**Then wrap**, as `render_refusal` does (`Wrap { trim: false }`).

**Then the explanations.** `code_entries()` is iterated **nowhere** outside `glossary.rs` — the four
entries ship authored, tested and inert. Render them in the Glossary overlay: code, title, explanation,
`see_also`. A refusal card's `glossary: <code>` line currently names a code with no way to read it;
**after this, that line points at something a user can actually reach.**

**Acceptance is reachability, not existence.** A render test at **80×24** must prove: the last
terminology entry is reachable by scrolling, and **an explanation's closing words** — not its opening —
appear on screen. Asserting the head of wrapped text passes on truncated content, which is how RFC 016's
C2 survived its first fix.

**`NFR-A03`**: whatever key scrolls, it is discoverable — the Keys section of this very panel is where a
user looks, so it lists itself.

## 4. Decision 5 — correct the threat model now

`C-S2` has **no implementation anywhere in the workspace**, and the threat model's coverage table
(§ *No key material in stikk*) lists it beside `C-I1a–d` and `SEAM-06` as though all three were in
force. Two are.

**Mark it not implemented, in both places** — the control's own bullet and the coverage table — and say
what it is waiting on: prikk ≥ 0.34's `trust maintainer list`, and the derivation mechanism Q1 resolved.
**Name the increment it ships with** (`FR-103`'s three-valued `Ready`).

**This is documentation only and it is the most important line in this handoff.** A coverage table that
lists a control nobody built is the `T-T4` shape aimed at ourselves: the document most responsible for
telling a reader what protects them, making a claim it cannot support. **Correcting it does not wait on
building it.**

## 5. Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
cargo deny check
```

The real-binary suite is not required here — **nothing in this handoff changes a parsed surface**. Say
so rather than running it and implying it checked something.

## 6. Acceptance criteria

1. F1's gloss keyed on a **captured** message; names prikk 0.29 as the fix; says the user typed no
   backslash; does not overstate the limitation to whole commits.
2. F1 render-tested at 80×24 with gloss **and** prikk's verbatim text both on screen.
3. F2 ships wrap, scroll and explanations **together**; the Glossary's own Keys section lists the scroll
   key.
4. F2 render-tested at 80×24 proving the **last** terminology entry is reachable and an explanation's
   **closing** words appear.
5. `C-S2` marked not implemented in the control bullet **and** the coverage table, naming what it waits
   on and which increment ships it.
6. All eight gates green; the suite deliberately not run, and that said.
7. Nothing tagged or published.

## 7. Submit

Package to `.git-exclude/review-request/023-a-display-gaps/review-request-v1.md`.

**Lead with two 80×24 renders**: the Windows-refusal card, and the Glossary scrolled to its end with an
explanation visible. Both are screens; show them.

**Then tell me what you found that the RFC did not.** F2 in particular has been deferred twice — once
by RFC 016's review, once by RFC 018's revert — and something that survives two attempts usually has a
reason nobody wrote down.

**Push once approved.** Handoff B follows.

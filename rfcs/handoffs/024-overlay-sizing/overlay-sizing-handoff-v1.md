# Handoff — overlay sizing, and the gate that makes it stay fixed (v1)

**Companion to:** [RFC 024](../../accepted/024-overlay-sizing-and-the-fix-that-did-not-travel.md)
(Accepted 2026-09-12; **Q1 ruled (c)** the same day, with the property stated as two assertions —
read that section first, F5 changed what this increment covers).
**Design items:** `T-T4`, `OP-03`, `FR-120`/`FR-121`, `NFR-A03`, `TS-01`.

> **Three live defects, all found by rendering rather than reading, all shipped:**
> seal's confirmation has never shown `Enter to confirm` on the path every user takes (any height);
> the commit message prompt clips its footer; and **the ref picker clips its own cursor** — hold ↓ past
> the fold and the selection goes somewhere invisible.
>
> **RFC 016 C2 already fixed this defect once**, in `render_refusal`. It did not travel: three of
> fourteen renderers use the fix, eleven still guess. **The gate in §4 is the part of this increment
> that matters most**, because a correct fix that does not travel is what produced all three.

---

## 1. Scope

**In:** the shared shape (§2); the three live defects (§3); the gate (§4).

**Out:** `render_refusal`'s lost `│ ` on wrapped verbatim lines — **framing, not sizing**, and it stays
next in line; bundling it would blur what this increment proves. The backslash gloss reading
`prikk_version`. Any change to what an overlay *says*. A redesign of the overlay layer: scrollable lists
may keep a bespoke layout provided they satisfy §4.

## 2. The shared shape

**Wrap in stikk, then anchor.** Both halves, because they do different jobs:

- **Measure**: `text::wrap_indented(text, width, indent)` (RFC 023 F2) wraps so `lines.len()` is a fact
  rather than an estimate. `Paragraph::wrap` computes the wrap inside the widget and `line_count` is
  behind ratatui's unstable `rendered-line-info` — which is why RFC 016 C2 could not measure and chose
  to anchor instead. **That constraint is gone; the choice it forced is not binding any more.**
- **Anchor**: the action/footer region sized exactly and laid out at the bottom, as
  `render_refusal` does today — *"whatever runs out of room under pressure is prose, never an action."*

**Replacing eleven copies of a heuristic with eleven copies of a better heuristic is not the fix.** One
shape, used by every prose-plus-actions overlay.

**Where a renderer legitimately differs, say so in its doc comment** rather than leaving the reader to
infer it from an absent call. A bespoke layout that satisfies §4 is fine; an undocumented one is how
this happened.

## 3. The three live defects

1. **`render_confirmation`** — `lines.len() + 4` against a consequence that wraps to four rows. Seal's
   `MaintainerReadiness::Unknown` text is the default path (RFC 016 F3: it is the only value any
   supported prikk can produce). **Verify the fix at 80×24 *and* at 80×40** — the clipping was present
   at both, so a fix that only works when the terminal is generous is not a fix.
2. **`render_commit_message`** — same measurement, footer off screen at 80×24.
3. **`render_ref_picker`** — clips the cursor (F5). **Window to the selection**: the selected row is on
   screen at every cursor position, including the last. Check `render_palette` and `render_refusals`
   for the same shape while you are there and **report what you find either way** — if they already
   window, that is worth knowing; if they do not, they are the same defect.

## 4. The gate — the half that matters *[Q1 ruled (c)]*

**Two assertions, over an exhaustive match on `Overlay`.** The enum is not `#[non_exhaustive]`, so a
test inside the crate can match every variant and **a fourteenth variant will fail to compile until
someone covers it.** That is the mechanism; the assertions are:

**1. No silent clipping.** For every variant, at 80×24: either its last content row is on screen, **or**
the render carries a viewport indicator saying there is more. The Glossary already does the second
(`lines 108–127 of 128`, RFC 023 F2) — **follow that, do not invent a second idiom.**

**2. A cursor is always visible.** For every variant that has a selection: the selected row is on screen
at every cursor position. **No "or" clause on this one** — a hidden selection has no honest form.

**Build each variant with content that actually exceeds the viewport.** A gate fed short fixtures
passes while asserting nothing, which is the failure mode every render test in this project has had to
be proven against. **Prove both assertions non-inert** by reverting one fix at a time and watching the
right one fail.

**Where an overlay cannot satisfy assertion 1 without an indicator, add the indicator** — that is the
honest outcome, not an exemption. If you reach a variant where neither assertion can be stated
sensibly, **stop and report**: that is a design question and it is mine.

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

**The real-binary suite is not required** — nothing here touches a parsed surface. Say so rather than
running it and implying it checked something.

## 6. Acceptance criteria

1. One shared prose-plus-actions shape, measuring with `wrap_indented` and anchoring the action region;
   any renderer that legitimately differs says why in its doc comment.
2. Seal's confirmation shows `Enter to confirm · Esc to cancel` at **80×24 and 80×40**, with seal's real
   `Unknown` consequence.
3. The commit message prompt shows its footer at 80×24.
4. The ref picker's selected row is on screen at every cursor position, including the last of forty.
5. `render_palette` and `render_refusals` checked for the same shape and the finding reported either way.
6. The gate exists as an **exhaustive match** over `Overlay`, asserting no-silent-clipping and
   cursor-visibility, fed content that exceeds the viewport, and **both assertions proven non-inert**.
7. All eight gates green; the suite deliberately not run, and that said.
8. Nothing tagged or published.

## 7. Submit

Package to `.git-exclude/review-request/024-overlay-sizing/review-request-v1.md`.

**Lead with three 80×24 renders** — seal's confirmation, the commit message prompt, and the ref picker
with the cursor on the fortieth entry. They are the defects; show them fixed.

**Then tell me what the gate found that the RFC did not.** Thirteen variants have never been rendered
against a common property, and this project's record is that the first exhaustive pass over anything
finds something. **If it finds nothing, say so plainly and say how you convinced yourself it is not
inert** — the answer I have come to trust most from you.

**Push once approved.**

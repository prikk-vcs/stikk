# RFC 024 — Overlay sizing: the confirm affordance seal has never shown, and a fix that did not travel

**Status.** **Accepted by the project owner 2026-09-12**, Q1 left unruled and **ruled by the architect the same day** — the property is statable, and finding out took one more measurement (F5). Proposed 2026-09-12. The next 0.6.0 increment. Opened on a defect found while re-scoping
RFC 023's follow-up, which turned out to be larger than the follow-up.
**Tracks.** `T-T4`, `OP-03`, `FR-120`/`FR-121` (a confirmation the user can act on), `NFR-A03`,
`TS-01`.
**Touches.** `crates/stikk-tui/src/overlay.rs`, `text.rs`, and their tests.

## Summary

**stikk's seal confirmation has never shown `Enter to confirm · Esc to cancel` on the path every user
takes.** Not at 80×24, not at 80×40 — at any height. It shipped that way in 0.4.0, 0.4.1 and 0.5.0.

The cause is a height estimate counting **logical** lines where the widget renders **wrapped** rows.
**This project already found that defect, in `render_refusal`, and fixed it — RFC 016 C2.** The fix was
correct and never travelled: **three** of fourteen overlay renderers use it; **eleven** still guess.

## Findings

### F1 — the confirm affordance is missing on seal's default path, at every terminal height

`render_confirmation`:

```rust
let height = (lines.len() as u16 + 4).min(area.height.saturating_sub(2));
```

`lines.len()` counts logical lines. Seal's consequence under `MaintainerReadiness::Unknown` is **one
logical line that wraps to four rows**, so the `+4` headroom is spent and the footer falls outside the
box. Measured at 80×**24, 26, 28, 30, 34, 40** — **absent at all six.** The terminal's size is not the
constraint; the estimate is.

**`Unknown` is the only value any supported prikk can produce** (RFC 016 F3). This is not an edge case,
it is the default path of the most consequential operation stikk offers.

**And it is pre-existing** — the control, rendering the same summary with `signing_key_id: None`, is
clipped identically. RFC 023 B did not cause it and does not worsen it.

### F2 — `render_commit_message` clips too

Same shape, same measurement: `Esc to cancel` is off screen at 80×24 on the commit message prompt. **Two
of the fourteen clip with ordinary content**, which is enough to say the heuristic is not merely
mistuned.

### F3 — RFC 016 C2 was right and did not travel

That review found this exact defect in `render_refusal` and required the fix that shipped: a prose
region plus an **actions region sized exactly and anchored at the bottom**, so *"whatever runs out of
room under pressure is prose, never an action."*

It was applied to `render_refusal`, later to `render_stale` and one more. **Eleven renderers kept
`lines.len() + N`.** Nobody checked the siblings, and the review that found it — mine — asked for the
instance rather than the class.

### F4 — the mechanism was right for its constraints, and RFC 023 changed the constraints

RFC 016 C2 chose **anchoring** over **measuring**, and correctly: `Paragraph::wrap` computes the wrap
inside the widget, and `line_count` is behind ratatui's unstable `rendered-line-info`. There was no way
to know the rendered height.

**RFC 023 F2 built one.** `text::wrap_indented(text, width, indent) -> Vec<String>` wraps in stikk so
`lines.len()` is a fact — built for the Glossary, for exactly this reason, and applicable to every
overlay.

**So the two mechanisms now compose**: wrap in stikk and the height is exact; anchor the actions and
the exactness stops being load-bearing. RFC 016 C2 had to choose; this increment does not.

## Decisions

1. **Fix F1 and F2**, which are live in a shipped release.
2. **Adopt one shared shape** for every overlay that is prose-plus-actions: wrap with `wrap_indented`
   so the height is measured, and anchor the action region so a wrong measurement costs prose. **Eleven
   copies of a heuristic is the defect; replacing them with eleven copies of a better heuristic is not
   the fix.**
3. **Gate the property, not the instances** (see Q1). RFC 016 C2 fixed an instance and the class
   survived; this increment must not leave the same opening.
4. **Not a 0.5.1.** Nothing false is stated and nothing is blocked — `Enter` works and the Glossary
   documents it. A missing affordance is not a wrong claim, which is the line 0.4.1 crossed and this
   does not. Ships in 0.6.0. *(Architect's call; the owner may overrule.)*

## What this RFC does not do

**It does not redesign the overlay layer.** Scrollable lists (`render_ref_picker`, `render_palette`,
`render_refusals`) are a different shape and may legitimately keep a bespoke layout — see Q1.

**It does not touch `render_refusal`'s lost `│ ` quoting** on wrapped verbatim lines, carried from
RFC 023 B. That is framing rather than sizing, and bundling it would blur what this increment proves.
It stays next in line.

### F5 — the ref picker clips its own cursor *[found while trying to state Q1's property]*

Forty refs, cursor on the last:

```
last-entry-on-screen = false        first-entry-on-screen = true
```

**`render_ref_picker` does not window to the cursor. It renders from the top and clips.** A user holding
↓ past the fold watches nothing move while the selection travels somewhere invisible — and the ref
picker has shipped since 0.1.0.

**Widened at implementation, 2026-09-12:** `render_palette` and `render_refusals` had the identical
defect — **three of the four lists, not one.** **`Recent refusals` was a second live instance**: its ring
holds fifty records and about twenty fit at 80×24, so its cursor could sit invisibly exactly as the ref
picker's did. The palette ships ten commands and fit by luck rather than by design. F5 is the sizing
story told again about windowing.

**This is what made Q1 answerable.** I had been trying to state *"the last row is visible, or the overlay
says it scrolls"*, which is fuzzy because a list legitimately continues past the fold. The ref picker
shows the real property has two halves, and both are sharp.

## Q1 — RULED by the architect, 2026-09-12: **(c)**, with the property stated as two assertions

**The owner accepted without answering, and the question needed a measurement rather than a decision.**
I said I could not state the property crisply and that a fuzzy gate is worse than none. F5 supplied the
missing half.

**The property is not "nothing is clipped." It is: *nothing is clipped silently, and the cursor is never
the thing clipped.*** Two assertions, each sharp:

1. **No silent clipping.** For every overlay: either its last content row is on screen, **or** the render
   carries a viewport indicator saying there is more. The Glossary already does the second —
   `lines 108–127 of 128` in its title (RFC 023 F2). An overlay that clips and says so is honest; one
   that clips in silence is the defect.
2. **A cursor is always visible.** Where an overlay has a selection, the selected row is on screen at
   every cursor position. This is the half F5 violates, and it needs no "or" clause.

**Both are assertable over an exhaustive match on `Overlay`'s thirteen variants** — the enum is not
`#[non_exhaustive]`, so a fourteenth fails to compile until covered. That is RFC 022 §7b's shape, the
only mechanism here that has caught the architect as readily as the implementer.

**And (c)'s helper is still worth building**, because a gate tells you the twelfth renderer is wrong
without making the right thing easy. But **the gate is the half that matters**: RFC 016 C2 built the
right mechanism and eleven siblings kept the old one, which is precisely what a gate would have stopped.

### The original question, for the record

**Q1 — a helper, or a gate, or both?**

Decision 3 says gate the property. The shape of that gate is the question, because the enum makes an
unusual option available: **`Overlay` is not `#[non_exhaustive]` and has thirteen variants, so a test
inside the crate can match it exhaustively — and a fourteenth variant then fails to compile until
someone covers it.**

- **(a) A shared helper, no gate.** Cheapest. But a helper can be bypassed by the next renderer, which
  is precisely how eleven siblings kept the old shape after RFC 016 C2.
- **(b) A gate, no helper.** A test rendering every variant at 80×24 and asserting its last content row
  is on screen. Catches the class, including any future bespoke layout — but leaves eleven
  hand-maintained sizings to drift under it.
- **(c) Both.** The helper makes the right thing easy; the exhaustive-match gate makes the wrong thing
  fail to compile.

**My lean: (c), and the gate is the half that matters.** RFC 022's §7b did the same thing for the RFC
index — both sides on disk, so assert them against each other — and it is the only mechanism this
project has that has caught the architect as readily as the implementer.

**What I am not sure of, and why this is a question rather than a ruling:** whether every variant
*has* a last content row that must be reachable. A scrollable list legitimately continues past the
fold — that is what scrolling is for. So the gate's assertion may need to be *"either the last row is
visible or the overlay advertises that it scrolls"*, which is a more interesting property and a harder
one to state. **If it cannot be stated crisply, (b) is worse than useless** — a gate that asserts a
fuzzy property gets relaxed the first time it is inconvenient.

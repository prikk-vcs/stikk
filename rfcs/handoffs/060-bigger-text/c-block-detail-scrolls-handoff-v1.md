# Handoff C — RFC 060: Block detail scrolls, and no view stops without saying so (v1)

**Companion to:** [RFC 060](../../accepted/060-bigger-text.md), decisions 2 and 3. **A defect the dev team found** while
justifying Handoff B's sweep (its review request §6). **0.21.0.** **Not a release.** Small. **Starts after Handoff B
is pushed.**

> **What is wrong.** At 60 × 15, Block detail shows eleven content rows and stops. It has no position, no foot line,
> and no scroll. `prev-ref-state`, the state section and the patches line cannot be reached at that size, and nothing
> says so. **That is a silent cut, which RFC 060 exists to remove.** Handoff A's overflow recorder did not see it,
> because it records rows **wider** than their area, not rows **below** it.
>
> **No branch.** Local gates and suite, then, after approval, push and dispatch the full-matrix suite and supply chain.

## 0. Evidence plan

| Question | Level | Evidence |
|---|---|---|
| Can every row of Block detail be reached at 60 × 15, by keys? | headless, through `handle_key` | test names, and a capture at the top and at the end |
| Does any view or overlay have rows below its area without showing a position? | headless, the whole matrix | the guard's name, its coverage, and one planted fault caught |
| Real terminal | the architect, after approval | not yours |
| Nothing regressed | unit, real binary 0.28.0 + 0.58.0; the full matrix in CI | counts |

## 1. Block detail scrolls

1. **`Screen::BlockDetail` gains the offset the other scrolling views keep**, as `Screen::Queue` and `Screen::Trust`
   do.
   - The **title** carries RFC 060's position, through `Title`, while rows are hidden.
   - The **key line** names `↑↓`, as on every view.
2. **↓ and ↑ follow the view's two halves.**
   - **Where there is a selectable row** (a file or a patch), they move the selection, and **the window follows it**,
     so the selected row is always on screen. Above the first row, the header rows (block, state, previous ref
     state, patches) scroll into view as ↑ goes past the first row.
   - **Where there is none,** they scroll the content.

   **One rule for every view with a header and a list.** Say in the request whether any other view has this shape,
   and if so, whether it already behaves this way.
3. **Nothing else about Block detail changes**: its rows, its words, Enter's list (RFC 061).

## 2. The guard: no view stops without saying so

**Widen Handoff A's structural check from width to height.** Every view and overlay that draws a list of rows into
an area reports to the recorder how many rows it had and how many it drew. Where it drew fewer, **the frame's title
must carry a position**. If not, that is an overflow, named like the others.

**Run it over RFC 059's whole matrix at 60 × 15 and 72 × 20.** Then plant one fault, Block detail's offset removed
again, and show it caught.

## 3. Traps

1. **The selected row is never off screen.**
2. **Press keys through `handle_key`** (Handoff B), not methods.
3. **Paste captures as the file holds them, and grep before committing.**

## 4. Submit (no branch)

Gates 1–3 and 5, and the local suite at 0.28.0 + 0.58.0. Package to
`.git-exclude/review-request/060-bigger-text-c/review-request-v1.md`, with:
- the commit;
- the two captures;
- the guard's coverage and the planted fault;
- the counts.

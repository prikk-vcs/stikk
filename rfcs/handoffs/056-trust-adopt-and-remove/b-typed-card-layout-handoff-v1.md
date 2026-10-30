# Handoff B — RFC 056: the typed card's layout (v1)

**Companion to:** [RFC 056](../../done/056-trust-adopt-and-remove.md). Its *Delivered* section carries both items here,
from review v1 of [Handoff A](./a-adopt-and-remove-handoff-v1.md). **0.19.0's increment 2**, authorized by the project
owner on 2026-10-30 (`.git-exclude/release/0-19-0-scope.md`).
**Independent of RFC 057**, but **land it after RFC 057's handoff** if both are in flight, because both re-capture the
commit card.
**Not a release.**

> **What this increment is.** Two layout fixes in `stikk-tui`, and nothing else:
> 1. a long id is **never cut off where the user must type it back**;
> 2. a value split across rows is split **evenly** (a 64-hex key as 32 + 32, not 63 + 1).
>
> **A correction to the scope's third item.** *"Prose flush against the right border"* (carried since RFC 047) was
> already fixed in **0.16.0**, by `a6e4a7c` (*"a right gutter for the panels' prose, not their identifiers"*). The
> architect carried it in error. Do not redo it, but confirm it in §3's captures.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite and supply chain there.

---

## 1. Even splits (`stikk-tui/src/text.rs`)

When stikk hard-splits **one value** that does not fit a row (`wrap_verbatim`, and `wrap_indented`'s hard-split of an
over-long word), it currently fills each row and leaves the remainder on the last one, so a 64-hex key splits 63 + 1.

**Change:** split into the fewest rows that fit, then make the rows **as equal as possible**. They differ by at most one
character, and longer rows come first.

- 64 characters with 63 available → 32 + 32.
- 120 characters with 63 available → 60 + 60.
- A value that fits on one row is unchanged.

Use **one helper** for both functions.

**Tests:**

- the three cases above;
- a value of exactly the available width;
- a multi-byte string: count characters, not bytes, as the existing helpers do.

## 2. The typed confirmation never clips (`stikk-tui/src/overlay.rs`, `render_confirmation`)

Three rows are drawn unwrapped today: the target rows, *"Type "…" to confirm:"* and the echo. Pinned by
`capture_rfc056_typed_confirmation_at_80_columns`.

- **The target rows.** A value that does not fit is hard-split (with §1's even split), and its continuation rows are
  aligned under the value's column, not under the role label.
- **The prompt.**
  - Where *"Type "<id>" to confirm:"* fits the panel width, keep it exactly as it is.
  - Where it does not, draw *"Type this to confirm:"* on one row, then the id hard-split beneath it at a four-space
    indent, in the accent style.
- **The echo.** *"› "* followed by what was typed, hard-split with continuation rows indented under the first typed
  character.
- **Re-pin the test.** `capture_rfc056_typed_confirmation_at_80_columns` now asserts the **opposite** of what it
  pinned:
  - with a 120-character id at 80 columns, the id appears whole across the prompt's rows, and whole across the
    echo's rows once typed;
  - no rendered line exceeds 80 columns.

  Rename it to say what it now proves, and keep its doc's history line: *pinned in RFC 056 A, fixed in B*.

**The cursor.** If the prompt shows a cursor, it stays at the end of the echo's last row. Check this, and add a test if
the overlay positions one.

## 3. Captures at 80 columns

Re-take and paste them as the file holds them:

- RFC 056's four cards and the typed confirmation with the 120-character id;
- the bundle sealer rows (RFC 054), which use `wrap_verbatim`;
- one merge confirmation, to confirm that the right gutter from `a6e4a7c` is still in place.

## 4. The traps

1. **Layout only.** No sentence changes, and nothing in `stikk-core`.
2. **`inert` and `identifier` stay where they are.** Split the escaped text, never the raw text, so an escape is never
   cut in half. Add a test: an id with an escaped character exactly at the split point stays whole on one row.
3. **Paste captures as the file holds them, and grep before committing.**

## 5. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/056-typed-card-layout-b/review-request-v1.md`, leading with §2's before and after.

# RFC 064 — stikk's JSON reader skips any valid value it does not read

**Status.** **Done 2026-11-21**: Handoff A delivered on `main` (`eda9412`..`d0ebe55`), green on the full platform matrix; shipped in **0.21.0**. **Accepted by the project owner 2026-11-21**; [Handoff A](../handoffs/064-json-reader-skips-what-it-does-not-read/a-skip-what-is-not-read-handoff-v1.md). **Proposed 2026-11-21** by the architect, as an addition to 0.21.0
(`.git-exclude/release/0-21-0-scope.md`, "Addition, 2026-11-19"). **Promised to prikk** in our letter 024 §2, as sent
by the project owner. prikk acknowledged it in their 033, and holds to the old constraints until their owner rules.
**No open question.**
**Measured** at `main` `5ae370a` against a real prikk **0.58.0** `status-report-v1`:
`.git-exclude/reports/064-json-reader/measure064.sh` → `measure064-0.58.txt`.
**Tracks.** `NFR-R03`; RFC 026 §6 (why stikk has its own reader); prikk's 032 questions 2 and 5, and our 024 §2.
**Touches.** `stikk-prikk/src/json.rs` and its tests, and `parse_json.rs` only where a `match` on `Json` must name the
new variants. No public API changes: `Json` is `pub(crate)`.

## Summary

stikk reads prikk's JSON with its own small reader, so its dependency tree stays one crate deep (`json.rs:3-12`). The
reader refuses four kinds of value **anywhere in a document**, including fields stikk never reads:
- a negative number;
- a fraction or an exponent;
- an integer above `u64::MAX`;
- nesting deeper than 16 levels.

So one additive prikk field holding `-1` makes a whole report unreadable, for every released stikk (F1).

This RFC keeps every refusal **for a field stikk reads**, and accepts any valid JSON in a value it only skips. The
reader still builds the whole tree, so the change is two inert variants that no accessor accepts.

## F1 — measured: the refusal does not care whether the field is read

The method (`measure064.sh`): take a real prikk 0.58.0 `status-report-v1`, then put each form in two places:
- **(a)** a new top-level field, `x_added_field`, which no reader looks at;
- **(b)** `trailing_partial_wal_bytes`, which both readers read.

Then run both of stikk's real readers of that schema, `orientation` and `queue`, through one temporary test in a
`git archive` copy.

| Form | (a) a field stikk does not read | (b) a field stikk reads |
|---|---|---|
| `-1` | **refused:** *"unexpected `-`"* | refused, same |
| `0.5` | **refused:** *"a fractional or exponent number, where stikk expects an integer"* | refused, same |
| `1e3` | **refused**, same sentence | refused, same |
| `18446744073709551616` (u64::MAX + 1) | **refused:** *"an integer too large for stikk to hold"* | refused, same |
| 17 nested arrays | **refused:** *"nested deeper than stikk reads"* | refused, same |
| 1000 nested arrays | **refused**, same | refused, same |
| the report unchanged | read | read |

The (b) column is right and stays. **The (a) column is what this RFC changes.**

*At delivery, `measure064.sh` at `d0ebe55` (`measure064-0.58-at-d0ebe55.txt`) reads every (a) row. The (b) rows are
now refused by `orientation` alone, naming the field. `queue` never reads `trailing_partial_wal_bytes`, and refused
column (b) above only because the whole parse failed. That is decision 3 working: each reader refuses the fields it
reads. For the depth rows, the refusal is the wrong-type sentence, because the skipped value starts 16 levels below
the field.*

## Decisions

1. **Numbers: the full JSON grammar, with two outcomes.** The reader accepts RFC 8259's whole number grammar,
   `-? int frac? exp?`.
   - **A non-negative integer up to `u64::MAX`** is `Json::Number(u64)`, as today.
   - **Every other valid number** is a new `Json::OtherNumber(String)`, holding its text exactly as prikk wrote it.
   - **Text that is not a JSON number at all** is refused, as today: `-` alone, `1.`, `.5`, `1e`, `+1`.
   - The documented leading-zero laxity stays as it is (`json.rs:30-35`).
2. **Depth: past the limit, skip without recursing.** At a depth beyond `MAX_DEPTH` (16, unchanged), the reader
   neither recurses nor refuses. It **scans the one value** with a counter, not the call stack:
   - brackets and braces must balance;
   - strings must be well formed, escapes included;
   - every token must be one JSON allows.

   The result is a new `Json::TooDeep`, which carries nothing. Malformed input inside the skipped value is still
   refused, so "skip" never means "accept anything".

   **The scan must use constant stack space, whatever the depth.** A test feeds it 100 000 levels.
3. **No accessor accepts either new variant.**
   - `u64_field` on an `OtherNumber` is refused, with the field's name: *"field `{key}` is a number stikk does not hold
     (negative, fractional, or larger than 2^64 − 1)"*.
   - Every accessor on a `TooDeep` value is refused: *"field `{key}` is nested deeper than stikk reads"*.
   - `kind_of` names both.
   - **So every field stikk reads keeps exactly today's rule**, and only the sentence improves: it now names the
     field.
4. **The raw view.** `Json::render` feeds Patch detail's and Verify's raw views.
   - An **`OtherNumber`** is written as its own text. The text is checked against the number grammar, so it is ASCII
     digits, `-`, `.`, `e`, `E` and `+`, and needs no escaping.
   - A **`TooDeep`** is written as a fixed placeholder of stikk's: `"…nested deeper than stikk shows…"`, as a JSON
     string. **Never the skipped text itself.** It was never parsed into strings, so it never passed through `inert`.
5. **Every exhaustive `match` on `Json`** in `parse_json.rs` names the two new variants. Each falls to the same branch
   as a value of the wrong type. **None of them may be read as a value**: no `TooDeep` becomes "empty", and no
   `OtherNumber` becomes 0.
6. **The module doc** (`json.rs:1-40`) says what the reader now accepts and refuses, and why: our letter 024 §2.

## Tests and evidence plan

| Question | Level | Where |
|---|---|---|
| Does each form in an **unread** field leave the report readable, with the same result as without it? | unit | for each of F1's six forms, and 100 000 levels, inject an extra field into a real 0.58 report (`measure064.sh`'s fixture, captured into the tests). Every reader with a captured report must return a value **equal** to the one it reads from the plain report |
| Does each form in a **read** field still refuse, and name the field? | unit | the same forms in one read field per numeric reader: `log`, `status` (both readers), `worktree-status`, `tree`, `diff`, `verify`, `show`, `bundle preview`, `cat`, `extract` |
| Is malformed input inside a skipped value still refused? | unit | an unclosed string, a stray `}`, a bare word, and an invalid escape, each beyond depth 16 |
| Is the stack flat? | unit | 100 000 levels, in a default-sized test thread |
| Does the raw view stay safe? | unit | `render` of an `OtherNumber` gives its text; of a `TooDeep`, the placeholder; no skipped byte reaches the output |
| Does F1's (a) column flip, and (b) stay? | measurement | re-run `measure064.sh` at the head. Its (a) rows read **read**, its (b) rows are refused |

**No real-binary leg.** No prikk output holds any of these forms today, which is what prikk's 033 promises to keep.
The suite is run as usual: green at 0.28.0 + 0.58.0, unchanged.

## Delivery

**One handoff**, issued on acceptance. It is independent of the readability RFCs, and can go before or alongside RFC
059.

## Changelog

`### Changed`: stikk now reads a prikk report that carries a value it does not use, such as a negative or
fractional number, or deep nesting, in a field it skips. Before, such a field made the whole report unreadable.

## Delivered

**One handoff, three commits, `eda9412`..`d0ebe55`, green on `main`:** CI, Docs, supply chain, and the real-binary
suite on the full platform matrix (run `44850823616`): 100 cases on Linux, macOS and Windows at 0.28.0 + 0.58.0,
unchanged, as expected.
- `Json::OtherNumber` and `Json::TooDeep`.
- An iterative `skip_value` past `MAX_DEPTH`, which stays 16. It holds the parser's grammar, and 100 000 levels are
  scanned in a default test thread.
- Every reader with a captured report compared, with and without each form in an unread field: 18 runs × 7 values.
- One field per numeric reader is still refused, naming it.
- `parse_json.rs` needed no new arm: every `match` already sends a wrong type to a refusal.

**What it does and does not change.** The promise in our letter 024 §2 is kept **from stikk 0.21.0**. Releases up to
0.20.0 keep the old reader, which is why 024 asked prikk to keep the old constraints until 0.21.0 is the oldest stikk
in use.

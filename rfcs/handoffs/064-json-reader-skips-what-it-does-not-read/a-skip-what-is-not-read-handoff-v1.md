# Handoff A — RFC 064: stikk's JSON reader skips any valid value it does not read (v1)

**Companion to:** [RFC 064](../../done/064-json-reader-skips-what-it-does-not-read.md), **accepted 2026-11-21**.
**Read the RFC first**: F1 and decisions 1–6. **Covers:** all of it. **An addition to 0.21.0**, promised to prikk in
our letter 024 §2. **Not a release.**

> **What this increment is.** Today stikk refuses a whole prikk report when **any** field, even one stikk never reads,
> holds a negative or fractional number, an integer above `u64::MAX`, or more than 16 levels of nesting (F1). After
> this handoff, such a value in a skipped field costs nothing, and the same value in a field stikk reads is refused,
> as today, with the field named.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the full-matrix suite and supply chain there.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does each F1 form in an unread field leave every reader's result unchanged? | unit | test names, and the list of readers covered |
| Does each form in a read field still refuse, and name the field? | unit | test names |
| Is malformed input past depth 16 still refused? | unit | test names |
| Is the stack flat at 100 000 levels? | unit | the test, run in a default-sized thread |
| Does the raw view stay safe? | unit | `render` tests |
| Does F1's column (a) flip, and column (b) stay? | measurement | `measure064.sh` re-run at your head, diffed against `measure064-0.58.txt` |
| Nothing else regressed | real binary, 0.28.0 + 0.58.0; then the full matrix in CI | counts |

No headless or real-terminal capture: nothing drawn changes, except the raw view's placeholder, which is unit-tested.

## 1. Measure first

Run `.git-exclude/reports/064-json-reader/measure064.sh <the base SHA> <0.58 prikk> <empty scratch dir>`, and diff it
against `measure064-0.58.txt`, ignoring the build lines. **Anything differing: stop and report.** At your head, the
same script is the last row of §0.

## 2. Commit 1: numbers (decisions 1 and 3), with their tests

1. **`Json::OtherNumber(String)`** in `json.rs`.
2. **The number scanner takes RFC 8259's grammar**, `-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [+-]? [0-9]+)?`,
   **except** that the documented leading-zero laxity stays (`007` is still 7).
   - A result with no `-`, no fraction and no exponent that fits `u64` is `Number`.
   - Every other valid number is `OtherNumber`, with its exact text.
   - Text that fails the grammar is refused with today's kind of sentence: `-`, `-a`, `1.`, `.5`, `1e`, `1e+`, `+1`.
3. **`value()` dispatches `-`** to the number scanner, where today it says *"unexpected `-`"*.
4. **The accessors.**
   - `u64_field` on an `OtherNumber` gives decision 3's sentence, naming the field.
   - `kind_of` names it, *"a number stikk does not hold"*, so every other accessor's wrong-type sentence names it too.
5. **`render`** writes an `OtherNumber` as its text (decision 4).
6. **`parse_json.rs`.** Make every exhaustive `match` on `Json` compile. A new variant goes wherever a wrong type
   goes, and is **never** read as a value (decision 5). List each site you touched in the request.

**Tests, in this commit** (in `json/tests.rs` unless said otherwise):
- the grammar: each valid form → its variant; each invalid one → refused;
- `u64_field` on each `OtherNumber` form → refused, naming the field;
- **for each reader with a captured report in `parse_json/tests.rs`**, the report plus an extra top-level field
  holding each of `-1`, `0.5`, `1e3` and `18446744073709551616` reads **equal** to the plain report;
- the same four forms in **one read field per numeric reader** → refused, naming the field. That is the field in F1's
  column (b) for `status`, and one field each for `log`, `worktree-status`, `tree`, `diff`, `verify`, `show`, `bundle
  preview`, `cat` and `extract`.

## 3. Commit 2: depth (decisions 2 and 3), with its tests

1. **`Json::TooDeep`**, carrying nothing.
2. **At `depth > MAX_DEPTH`, `value()` calls a skipper** instead of refusing. The skipper:
   - **is iterative:** an explicit `Vec` of open containers, never recursion. **Its call-stack use must not grow with
     depth**;
   - **checks the same grammar the recursive parser does**: objects of `"key": value` pairs, commas between, matching
     closers, strings through the existing string scanner (escapes and all), `true`, `false`, `null`, and numbers
     through commit 1's scanner;
   - **builds nothing**, and returns `Json::TooDeep` once the value closes;
   - refuses malformed input exactly as the parser would, so a skipped value is never "anything".
3. **Every accessor on `TooDeep`** is refused with decision 3's sentence. `kind_of` names it.
4. **`render`** writes `TooDeep` as decision 4's fixed placeholder string, **never** the skipped text.
5. **`MAX_DEPTH` stays 16.** Update its doc: past it, the reader skips rather than refuses.

**Tests, in this commit:**
- 17 and 1000 levels in an unread field, for the same readers as commit 1 → equal to the plain report;
- the same in a read field → refused, naming the field;
- **100 000 levels**, unread → reads. Run it in the default test thread: a recursive skipper overflows there, and
  that is the point;
- **malformed input past depth 16** → refused: an unclosed string, a stray `}` inside `[`, a bare word, `[1,,2]`, `{"a"
  1}`, an invalid escape `"\q"`, and input that ends mid-value;
- `render` of a `TooDeep` → the placeholder, and none of the skipped bytes. Use a skipped value holding a control
  character in a string to prove it.

## 4. Commit 3: the text (decision 6)

- **`json.rs`'s module doc:** what the reader accepts and refuses, now in two halves (a read field, a skipped value),
  and why, citing our letter 024 §2 and RFC 064. The *"no floats"* sentence becomes *"no floats **read**"*.
- **`CHANGELOG.md`, `## Unreleased`, `### Changed`:** the RFC's line.

## 5. The traps

1. **The skipper must be iterative.** A recursive one passes every test except the 100 000-level one, and that test
   is the reason for the rule.
2. **Never render skipped text.** It never went through the string decoder, and so never through `inert`.
3. **No `OtherNumber` or `TooDeep` may become a value.** Not 0, not empty, not `None`: wherever stikk reads, they are
   refused.
4. **Keep the leading-zero laxity exactly as it is.** Changing it is outside this RFC.
5. **Paste captures as the file holds them, and grep before committing.**

## 6. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Package to
`.git-exclude/review-request/064-json-reader-skips-what-it-does-not-read-a/review-request-v1.md`, and lead with:
- §1's diff, and `measure064.sh` at your head: column (a) reads, column (b) refuses;
- the readers covered by the unread-field tests;
- the `parse_json.rs` match sites you touched.

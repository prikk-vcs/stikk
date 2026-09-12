# Handoff A addendum — the boundary the JSON readers do not enforce (v1)

**Companion to:** [Handoff A](a-rebaseline-handoff-v1.md), whose work is otherwise accepted — see
`.git-exclude/reviewed/026-a-rebaseline/review-result-v1.md`.
**This is small and bounded.** Three changes in `parse_json.rs`, their tests, and two doc lines.
**It is A's, not B's:** the JSON readers are A's deliverable, and mixing a parse-boundary fix into B's
readiness rebuild would make one diff carry two unrelated arguments.
**Design items:** `INV-9`, `UD-02`, RFC 012 F-d, RFC 009 F2, `ER-02`.

> **Nothing here changes what stikk reads.** It changes what stikk refuses to believe.

---

## 1. Restore `INV-9`/`UD-02` on the JSON path (C1)

`parse.rs`'s module doc claims every ref name crossing this boundary goes through `RefName::parse`, and
every id through `ObjectId::parse`. `parse_json.rs` does neither, so at prikk ≥ 0.39 — every prikk
anyone runs — the claim is false.

**Validate, in `parse_json.rs`, at the same boundary and in the same error class the prose readers use:**

| Reader | Field | Through |
|---|---|---|
| `history` | `ref` | `RefName::parse` |
| `history` | `block_id`, `ref_state_id`, `previous_ref_state_id` (when present) | `ObjectId::parse` |
| `patch_messages` | `patch_id` | `ObjectId::parse` |
| `refs` | `ref_name` | `RefName::parse` |
| `refs` | `ref_state_id` | `ObjectId::parse` |
| `tags` | `ref_name` | `RefName::parse` |
| `tags` | `target_block_id` | `ObjectId::parse` |

**Keep the struct field types as they are.** `RefName`/`ObjectId` guard the boundary and return
`String` — `parse.rs`'s `required_ref_field` doc records why, and departing from that precedent here
would be a second argument in a fix.

**The error must name the field and the value**, the way the prose errors do, because a shape prikk
would never emit means stikk misread prikk — an environment fault, not a refusal.

**Two of your own tests are the proof this is missing**: `a_received_ref_is_flagged_by_its_array` and
`an_absent_received_array_is_an_empty_list` pass `"ref_state_id": "a"` and succeed. Give them real ids,
and add a test per field family asserting the refusal — including a **control character in a ref name**,
which the prose reader refuses today and the JSON reader accepts.

## 2. `closed` must not fail silently (C2)

`entry.bool_field("closed").unwrap_or(false)` is the only silent field in three parsers, and it is the
one carrying a fact the picker shows.

Measured against prikk 0.41's own emitter and a real repository: `branches` entries **always** carry
`closed` and it is real data; `received` entries **never** do, by prikk's deliberate design. The array
already tells you which case you are in.

- **`branches`**: required — `entry.bool_field("closed")?`.
- **`received`**: structurally `false`, not read.

**Same shape, same round:** `patch_messages` returns an empty list for a *wrong-typed* field as well as
an absent one. Absent-is-tolerated and wrong-type-is-an-error are different rules; `refs` already gets
this right with `Some(_) => value.array_field(key)?`, and one file should not hold three different
answers to one question.

## 3. One comment describes prikk behaviour prikk does not have (C3)

> *"`received` is absent on a repository that has never received one …"*

A fresh 0.41 repository that has never received anything emits `"received": []`. **prikk always emits
it.** Keep the tolerance; correct the claim — in the comment and in
`an_absent_received_array_is_an_empty_list`'s docstring. Say the array is always present at 0.41 and
that stikk tolerates omission anyway, so the next reader knows which half is measured.

## 4. Two lines in `json.rs`'s module doc

Both are about the swap this project has reserved the right to make, so they belong beside the paragraph
that reserves it:

- **Duplicate keys**: `Json::get` takes the first match; `serde_json` takes the last. prikk emits none,
  so it matters only on the day someone swaps the reader — and silently, which is why it is written down.
- **Leading zeros**: `007` reads as `7` where strict JSON refuses. The "no floats" paragraph is careful
  to say what the reader refuses; say where it is laxer too.

## 5. Out

**`env.rs` is still untouched.** Everything about readiness is B's, unchanged.

`refused paths:` is not this addendum's either — it is the increment after B (review result, §3a).

## 6. Gates

The eight, plus the suite. **The suite stays red at 0.41 and that is still correct** — these changes are
in the read path and the failures are the readiness gate. What must not change:

- `the_three_reports_parse_at_both_ends_json_above_prose_below` **still passes at both ends on all three
  platforms.** It passed in `34701639181` while everything around it failed; if it stops passing, a
  validation call is refusing something a real prikk actually emits, which is the one way this addendum
  can do harm. Name the new run ID.
- The failure count stays **2 passed / 12 failed**. A thirteenth failure is this addendum's.

## 7. Acceptance criteria

1. Every name and id in all three JSON readers validated through `RefName::parse`/`ObjectId::parse`, in
   the prose readers' error class, naming field and value.
2. A refusal test per field family, including a control-character ref name and a non-64-hex id.
3. No hand-written test fixture in `parse_json/tests.rs` uses a placeholder id any more.
4. `closed` required on `branches`, structurally `false` on `received`; `patch_messages` errors on a
   wrong-typed field and tolerates an absent one.
5. C3's two claims corrected; the tolerance kept.
6. `json.rs`'s module doc carries the duplicate-key and leading-zero notes.
7. `env.rs` diff still empty.
8. Eight gates green; matrix run named; still 2 passed / 12 failed; the three-reports test still passing.
9. Nothing tagged or published.

## 8. Submit

Package to `.git-exclude/review-request/026-a-rebaseline/review-request-v2.md`.

**Lead with the refusal tests** — the shapes that now fail, printed, beside what the prose reader does
with the same shape. That symmetry is the whole point of the fix, and it is the thing I could not see in
v1 because nothing in the suite was looking for it.

**Push once approved** — your five commits plus these, as one set, after my six design commits. B follows
immediately.

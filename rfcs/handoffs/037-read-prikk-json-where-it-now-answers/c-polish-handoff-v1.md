# Handoff C — two carried polish items: wrapping to the field's own indent, and the last composed sentence (v1)

**Companion to:** [RFC 037](../../done/037-read-prikk-json-where-it-now-answers.md) (its carried item) and
[RFC 036](../../done/036-repository-text-renders-as-it-is.md) (its carried item). **No new RFC**: both are recorded
carries, scoped as 0.11.0's increment 3 (`ROADMAP.md`, *Next — 0.11.0*).
**Not a release.** Small, and it touches views that Verify (RFC 040) will not, so it can land in parallel with that
design.

> **No branch.** Local gates and the local suite before submitting; after approval push to `main`, and dispatch the
> full-matrix suite and supply chain on `main`.

---

## 1. Long rows wrap to their own indent, not to column 0

**Where:** `view/block.rs` renders through `Paragraph … .wrap(Wrap { trim: false })` (`block.rs:236`). ratatui's wrap
continues a long line **at column 0**, so a 64-hex id field (`block`, `ref-state`) and the below-0.46 state sentence
spill under the labels at 80 columns. **Nothing is cut**, but a reader loses which field a continuation belongs to.
RFC 037's review found it; **check `view/history.rs` for the same pattern** and fix it there too if it has it.

**Do:**
- **Wrap these rows yourself, before the paragraph**, the way every other view already does (`text::wrap_indented`
  for prose, `text::wrap_verbatim` for identifiers and ids). Continuation rows go **at the value's own column**,
  under the value and not under the label.
- **Drop `Wrap` from that `Paragraph`.** The line count must be exact, because Block detail's patch cursor and
  scroll depend on it (RFC 023 F2: why stikk wraps by hand). **Pin the cursor arithmetic with a test.**
- **Ids are identifiers** (RFC 036): hard-split, never word-wrapped.

**Tests:** at 80 columns, a block whose `block` and `ref-state` rows wrap, with every continuation starting at the
value column, every character of each id present once and in order, and the patch cursor still selecting the right
row below them. The same for the below-0.46 sentence.

## 2. `queued_elsewhere_clauses` returns pieces

**Where:** `stikk-core/src/changes.rs:638`. `queued_elsewhere_clauses(queued_ref: &str, focused_ref: &str) -> [String;
4]` composes a sentence around two ref names. Its only production caller, `view/changes.rs:171`, passes them
**already rendered** as identifiers. That is the last place core takes frontend-rendered text, the pattern RFC 036
review v1 §2.3 ruled out for the GUI's sake.

**Do:** return the clauses as **`Vec<Piece>`** (a `Piece::Name` for each ref, `Piece::Said` for the words), as the
other four sites do since RFC 036, and have the view render them through `text::compose`. **The words do not change.**
**A Breaking row** in `## Unreleased` (`stikk-core`, a public signature), and the Additive paragraph stays as it is.

**Tests:** the existing clause tests, now over pieces; a ref with a double space and one with a bidi override render
as identifiers inside the sentence (`on_a_row`).

## 3. The changelog

Under `## Unreleased`: **`### Fixed`**, long rows in Block detail (and History, if it had the same problem) wrap
under their own value instead of at the left edge; **`### Breaking`**, `queued_elsewhere_clauses` returns pieces.

## 4. Submit

The eight gates, the local suite, and the grep before committing. Package to
`.git-exclude/review-request/037-c-polish/review-request-v1.md`, with the 80-column captures **as the files hold
them**.

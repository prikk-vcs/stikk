# Handoff B — RFC 040's carry: Block detail's `said` onto `view::wrap_hanging` (v1)

**Companion to:** [RFC 040](../../done/040-verify.md), whose *Carried forward* names this. **No new RFC**: 0.13.0's
increment 1 (`.git-exclude/release/0-13-0-scope.md`, authorized 2026-10-17).
**Not a release.** **No branch.**

> **A pure refactor.** `view/block.rs`'s `said` (line 383) wraps a first row and its hanging rows to their own widths,
> inline. That is exactly what `view::wrap_hanging` (`view.rs:32`) does, the helper RFC 040 added after this fault was
> written a third time. **Fold `said` into it, so the fault cannot be written a fourth time.**

---

1. **`said` calls `wrap_hanging`** for its wrapping, and keeps only what is its own (styling the rows). If nothing is
   left, remove it and call the helper directly.
2. **No row of any capture changes.** Block detail's existing tests and 80-column captures pass **unchanged**, and the
   README images' guard test (`readme_images_are_current`) stays green. **If either moves, stop and report**: this
   commit must not change anything a user sees.
3. **No public surface changes** (`cargo public-api`).
4. **Changelog:** nothing a user observes changes, so **no entry**.
5. **Submit:** the eight gates and the grep before committing. Package
   `.git-exclude/review-request/040-b-wrap-hanging-tidy/review-request-v1.md` with the diff's size and the unchanged
   tests named. **After approval:** push, then the suite and supply chain on `main`.

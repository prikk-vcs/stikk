# Handoff C — two carried items: the Glossary's `V`, and the overlay gate's missing case (v1)

**Companion to:** [RFC 040](../../done/040-verify.md) (`V` is its key). **No new RFC**: 0.14.0's increment 1
(`.git-exclude/release/0-14-0-scope.md`, authorized 2026-10-18). Both items were found delivering RFC 044 and carried
since. **Not a release.** **No branch.**

---

1. **The Glossary lists `V`.** `glossary_lines` in `crates/stikk-tui/src/overlay.rs` lists `Q` but not `V` (open
   Verify, shipped in 0.11.0). Add it beside `Q`, in the same register as its neighbours. If the longer list pushes
   anything a test asserts off its render, say which and how you kept it reachable — **do not** shrink another entry
   to make room.
2. **The overlay clipping gate builds a `CommitWouldRefuseDeclarations` case.** `coverage()` in
   `crates/stikk-tui/src/overlay/tests.rs` classifies it; `cases()` never builds one, so it is the one variant the
   80-column gate does not exercise (the assertion reads 17 cases against 18 variants — now more, after RFCs 044/045;
   **count them**). Build one with realistic content (several declarations, one long path), make the count assertion
   compare **cases to variants**, and if the case clips, that is a finding: **stop and report** rather than fix the
   overlay under this handoff.
3. **Changelog:** item 1 is a `### Fixed` line (a shipped key the Glossary never listed). Item 2 is test-only: no entry.
4. **Submit:** two commits, the eight gates, the grep. Package
   `.git-exclude/review-request/040-c-two-carries/review-request-v1.md`. **After approval:** push, then CI's result on
   `main`.

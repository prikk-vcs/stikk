# Handoff A — F0: stop fabricating a worktree entry (v1)

**Companion to:** [RFC 021](../../accepted/021-prikk-0-38-rebaseline.md) (Accepted 2026-09-12),
**Decision 0** — *fix F0 before anything else, and separately.* This handoff is only F0. The
re-baseline proper (ceiling, fixtures, requirement amendments) is Handoff B and follows this one.
**Design items:** `T-T4` (no confident-but-wrong picture), `UD-02` (confined parsing), `TS-03`
(captured fixtures), `FR-034`.

> **This is the only finding in RFC 021 that is wrong in a shipped release.** On prikk ≥ 0.38, after
> `prikk mv "modified draft.txt" renamed.txt`, stikk's Changes view shows three entries where prikk
> reported two — the third a file that does not exist, in a state prikk never reported. It reaches
> users because stikk deliberately runs above its validated ceiling. **prikk predicted the exact line
> by reading our parser; I reproduced it against a real 0.38.0 binary and our real
> `parse::worktree_status`.**

---

## 1. Scope

**In:** the parser fix (§2), the captured fixture and tests (§3), the changelog line (§4).

**Out:** raising the ceiling (Handoff B — the real-binary suite's version guard will not run at 0.38
until it moves, and it must not be edited to); `worktree-status --format json` (0.38-only, and our
floor is 0.28); any change to what the Changes view *renders*; every other RFC 021 finding.

**Ships in 0.5.0, not a 0.4.2.** Ruled by the architect: reaching the defect needs prikk ≥ 0.38, a
path whose first token is a kind word, *and* a `prikk mv` — three conditions coinciding — where 0.4.1's
defect was on screen for every user on one keypress. 0.5.0 is the next release and already in motion.
The owner can override.

## 2. The fix — scope the scan, do not extend the reject list

`parse.rs:382–386`:

```rust
let entries = text
    .lines()
    .filter(|line| line.starts_with(' ') || line.starts_with('\t'))
    .filter_map(parse_worktree_entry)
    .collect();
```

**This reads every indented line in the document.** 0.38's rename section is indented too:

```
worktree: changed against baseline
  missing modified draft.txt — tracked file is absent from the worktree
  untracked renamed.txt — worktree file is not in the baseline
live rename declarations: 1
  modified draft.txt -> renamed.txt        ← first token "modified" → passes the kind check
note: each declaration above is authored into the next `prikk commit` …
```

**Make the scan section-aware:** entries are the indented lines **between the `worktree:` headline and
the next flush-left line**, whatever that line is (`live rename declarations:`, a `note:`, or nothing).
Nothing outside that region is an entry, regardless of its first word.

**Do not fix it by rejecting lines containing `->`.** That is a heuristic against this one collision
and leaves the next indented section — whatever prikk adds next — to fabricate the same way. The
defect is the missing section boundary, and the fix is the boundary.

**Two things the boundary must survive**, so state them in the doc comment:

- **Pre-0.38 output has no rename section.** Entries run from `worktree:` to the first `note:` or to
  end of input. The fix must parse 0.28–0.37 output identically to today — the existing fixtures are
  that regression suite.
- **The headline may say `clean`**, with no entries at all. An empty region is normal.

## 3. Fixture and tests

**Capture, do not transcribe.** Build a real 0.38.0 binary (`cargo install prikk --version 0.38.0
--locked --root …`, as RFC 019's suite does), `prikk setup`, create `"modified draft.txt"`, commit,
seal, `prikk mv "modified draft.txt" renamed.txt`, run `worktree-status --ref heads/main`, and capture
stderr+stdout verbatim. **Provenance line names 0.38.0 and the exact commands** — and says it is
**above the validated ceiling**: a parser fixture is a pure-function input and may be captured from any
real binary, but it must not be read as validation of 0.38.

Tests, in `parse/tests.rs`:

1. **The captured 0.38 case: exactly 2 entries** (`missing`, `untracked`) — assert the count *and* the
   absence of any entry whose path contains `->`. Count alone would pass a parser that dropped the
   wrong line.
2. **The pathological path**: a file literally named `modified`, renamed. Same assertion. This is the
   narrowest form of prikk's prediction and the one a `->` heuristic would also catch — include it so
   the test suite distinguishes the real fix from the heuristic.
3. **A synthetic future section**: indented lines under a flush-left label that is *not* one prikk
   emits today, placed after the entries. Must be ignored. **This is the test that proves the fix is
   the boundary and not the rename.**
4. **Every existing worktree-status fixture unchanged** — 0.28 through 0.32 captures parse to the same
   entries as before.
5. **Clean headline, empty region** — no entries, no error.

## 4. Changelog

`## Unreleased` → `### Fixed`, one entry, and say what it showed: *on prikk ≥ 0.38, a renamed path whose
name began with a change-kind word appeared in Changes as a modified file that did not exist.* Name
that prikk's letter predicted the line before it was reproduced — it is the second time an upstream
reading of our code has been exactly right, and the changelog is where that is worth recording.

## 5. Gates

The standard set, with packaging excluded as ruled:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
```

**Plus the real-binary suite at its current matrix (0.28 and 0.33)** — it does not exercise 0.38 yet,
but it is the proof that the scoping did not change what pre-0.38 parses to. Run it and paste the
result.

## 6. Acceptance criteria

1. The entry scan is bounded by the `worktree:` headline and the next flush-left line; the doc comment
   says so and says why.
2. No `->` reject heuristic anywhere.
3. A 0.38.0 fixture, captured with provenance, parses to exactly 2 entries with no `->` path.
4. The pathological-name and synthetic-future-section tests both pass.
5. All existing worktree-status fixtures parse unchanged.
6. Real-binary suite green at 0.28 and 0.33.
7. Changelog entry present; no behaviour change beyond the fabricated entry disappearing.
8. Nothing tagged or published.

## 7. Submit

Package to `.git-exclude/review-request/021-f0-worktree-entry-scoping/review-request-v1.md`.

**Lead with the before/after entry list on the captured fixture** — three entries, then two. Then the
synthetic-section test, since it is the one that would fail on a heuristic fix.

**Push once approved.** Handoff B is issued after this lands.

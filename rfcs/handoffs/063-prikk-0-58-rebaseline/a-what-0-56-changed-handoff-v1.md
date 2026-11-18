# Handoff A — RFC 063: what prikk 0.56 changed (v1)

**Companion to:** [RFC 063](../../accepted/063-prikk-0-58-rebaseline.md), **accepted 2026-11-18**. **Read the RFC
first**, F1–F10 and decisions 1–11. **Covers:** decisions 1–5 and 9–11. Decisions 6–8 are
[Handoff B](./b-what-prikk-now-checks-handoff-v1.md), which starts after this one is approved.
**0.21.0's increment 1.** **Not a release.**

> **What this increment is.** The validated ceiling moves from 0.55 to 0.58. Eight real-binary cases fail with only
> the constant moved (F1), and every one is explained. At the end of this handoff, the suite is green at 0.28.0 and
> 0.58.0, and stikk has:
> - stopped committing to a branch other than prikk's current one;
> - started offering a merge into the current branch from 0.56;
> - kept refusing a switch with work queued for the target, in new words;
> - learned to read an interrupted seal (0.58);
> - glossed prikk's publication sentences.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite with `full_platform_matrix: true`, and the supply-chain gate, there.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does prikk still do what the RFC measured? | raw prikk | §1's two diffs |
| Does each parser read the new field, and refuse a wrong shape? | unit | test names |
| Does each version gate pick the right path at 0.55, 0.56, 0.57 and 0.58? | unit | test names |
| Do the new sentences render and fit? | headless (`TestBackend`) | the Queue view's head, and the commit refusal, at 80 × 24 and 120 × 40, dark and light, as text captures from the buffer |
| Does stikk's own call path meet the real prikk as decided? | real binary, 0.28.0 + 0.58.0 | the suite's counts per commit (§8) |
| macOS and Windows | real binary, full matrix | CI after the push |

**No real-terminal screenshot is needed.** Nothing here changes colour, layout or a card's form, so the headless
level proves it.

## 1. Measure first

1. **Reproduce F1.** Run `.git-exclude/reports/rebaseline-0-58/measure-ceiling.sh 9a0c355 58 <empty scratch dir>
   <out dir>`. The out dir is a scratch directory, not the reports folder. Diff its `measure-ceiling-0.58.txt` lines
   against the one in the reports folder. **The same 8 failures, or stop and report.**
2. **Reproduce F2–F8.** Run `.git-exclude/reports/063-prikk-0-58-rebaseline/measure063.sh <0.58 prikk> <empty scratch
   dir>`, and diff it against `measure063-0.58.txt`. Ignore ids. **Anything else differing: stop and report.**

## 2. Commit 1: the constant, and the suite's fixtures (decisions 1 and 5)

1. **`VALIDATED_MAX_MINOR = 58`** in `stikk-prikk/src/version.rs`. Its doc comment says, in one paragraph each, what
   0.56–0.58 changed for stikk: RFC 063 F2–F9, and that the suite needed fixture and behaviour changes, unlike RFC
   055's. Check that CI's version matrix (`print_version_matrix`) now installs `0.58.0`.
2. **Distinct key ids** (`stikk-real-binary/src/support.rs:92-93`). Each `Fixture::build` gets its own author and
   maintainer ids: the role word plus a process-wide counter (`maintainer-1`, `author-1`, …).
   - **Keep the ids out of every assertion.** A test that needs one reads it from the fixture's field.
   - Add **`Fixture::build_with_maintainer_id(bin, id)`**, the only way to build a clash on purpose. Its doc comment
     says why it exists (RFC 063 F5).
3. **`rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import`.** Its receiver uses
   `build_with_maintainer_id` with the sender's id (`real_binary.rs:9459`).
   - **At ≥ 0.56:** assert that the import is refused, that stdout's `refusal-v1` id is `forged-publication-signature`
     (via a raw `--format json` run beside stikk's own call), and that nothing landed.
   - **Below 0.56:** today's assertions, unchanged.
   - The name stays; add one doc line saying what changed at 0.56.
4. **`rfc045_the_preview_is_what_the_switch_does_at_the_ceiling`, step 4** (`real_binary.rs:~6470`). Queue the work
   for a branch whose files **match** the worktree's, as `measure063.sh` §C does: make a fresh branch from the current
   tip, and commit to it **before** anything is sealed on it. The assertion stays `Blocked(WORK_IS_QUEUED_FOR_THIS_BRANCH)`
   with zero spawns.
5. **`rfc056_removing_shrinks_the_list_and_a_removed_id_is_prikks_refusal_at_the_ceiling`.** It asserts on a
   substring of 0.56's sentence, *"is not currently adopted (it was removed)"*. Handoff B replaces this with the card.

**After this commit, at 0.28.0 + 0.58.0, only `rfc047` and `rfc029b` fail.** Say so in the request, with the counts.

## 3. Commit 2: a merge into the current branch, from 0.56 (decision 2)

1. In `stikk-prikk/src/version.rs`, add **`MERGE_INTO_CURRENT_FROM_MINOR: u32 = 56`**, documented with F2. Export it
   as the other bands are.
2. **`stikk-core/src/merge.rs`, `blocked_reason` (`:445-479`).** Return `INTO_THE_CURRENT_BRANCH` only below the band.
   It needs the handshake: read it once, beside the orientation.
3. **`stikk-tui/src/app.rs:2117-2124`** repeats the check before dispatching. Give it the same band, from the
   handshake the app already holds. **One helper in core, called by both**, so the two places cannot drift.
4. **The merge card**, when *into* is prikk's current branch and the band holds, gains decision 2's line, *"Your folder
   is on this branch, so the merged files are written into it."*, after the plan.
5. `WORK_IS_QUEUED_FOR_INTO` is unchanged, at every version.

**Tests:**
- unit: the guard at minor 55 (blocked) and 56 (not blocked), and the card line present only in the second case;
- real binary at ≥ 0.56: rewrite `rfc047_a_merge_into_the_current_branch_leaves_the_worktree_behind` as
  **`rfc047_a_merge_into_the_current_branch_writes_the_worktree_from_0_56`**. A clean merge through stikk's own
  preview and execute writes `c.txt` with the sealed bytes. A dirty worktree gets prikk's refusal through `present()`,
  with `readme.txt` unchanged and the branch unmoved;
- real binary below 0.56 (the floor): the old test's assertions, kept as their own case.

## 4. Commit 3: commit goes to prikk's current branch; the switch's words (decisions 3 and 4)

1. **`stikk-core/src/commit.rs`, inside `compute`.** Read orientation, as it already does for `branch_notice`. When
   **all** of these hold, return `CommitReadView::Blocked(<decision 4's sentence>)`, so no token exists:
   - the handshake's minor ≥ `CURRENT_BRANCH_FROM_MINOR`;
   - `current_branch` is `CurrentBranch::Branch(c)`;
   - `c` ≠ the target.

   The sentence names both branches through `inert`. Put it in a `pub fn` beside the other sentences, so the TUI and
   the tests share it.
2. **`Unresolved`, `NotReported`, and below 42:** unchanged, with the notice.
3. **Seal is unchanged** (decision 4).
4. **`stikk-core/src/switch.rs:56-57`,** `WORK_IS_QUEUED_FOR_THIS_BRANCH`, takes decision 3's words, as they stand in the
   RFC.
5. **The guide.** In `docs/src/guide/getting-started.md`'s *Where stikk opens* paragraph, the last sentence keeps its
   meaning. Add one sentence after it: *"`C` commits to prikk's current branch, because that is where your folder's
   files belong; to commit to another branch, switch to it first (`b`, then the branch, then `s`)."*

**Tests:**
- unit: blocked at minors 42 and 58 when the branches differ, and **zero spawns past the reads**; allowed when they
  match; allowed with the notice for `Unresolved`; allowed with no notice below 42;
- real binary: rewrite `rfc029b_prikks_current_branch_is_reported_and_named_on_both_confirmations`:
  - **commit:** at ≥ 0.42, the preview for `heads/main` while `heads/dev` is current is `Blocked` with decision 4's
    sentence, and nothing changed (`status` before and after);
  - **seal:** its notice is still asserted, so the leg needs work queued for a branch that is not current. stikk
    cannot make that now, and prikk refuses a switch away from queued work (`measure063.sh` §H, at 0.55 and 0.58). So
    queue it with a **raw** `prikk commit --from-worktree --ref heads/main` while `heads/dev` is current, before
    anything is sealed on `heads/dev`, so that their files match (§C's setup). Then assert stikk's seal preview for
    `heads/main` carries the notice;
  - **the no-`--ref` default:** keep that leg, measured on `heads/dev` **without** a stikk commit to another branch
    first.

## 5. Commit 4: an interrupted seal (decision 9)

1. In `version.rs`, add **`ALREADY_SEALED_FROM_MINOR: u32 = 58`**.
2. **`parse_json.rs`, `queue()` (`:1113-1140`).** At or above the band, `queue.already_sealed` must be present.
   - `null` means none.
   - An object means its `block_id`, a lowercase 64-hex string. Anything else is the parser's shape error, naming the
     field.
   - `route` is not read.
   - Below the band, the field is not looked for.
   - The queue model gains `already_sealed_in: Option<…block id type…>`.
3. **`stikk-tui/src/view/queue.rs`.** When it is `Some`, a head above the list gives decision 9's sentence, with the
   short id as the Queue view already shortens ids.
4. **The seal confirmation**, when the queue it seals is already sealed, gives the same one line. Read it in
   `seal_preview`'s `compute`, so the card and the token describe one instant.

**Tests:**
- unit: the parser at 58, with `null`, an object, the field missing, and a wrong type; and at 57, where the field is
  absent and that is accepted;
- headless: the Queue view's head at both sizes and themes (§0);
- real binary at ≥ 0.58, **`rfc063_an_interrupted_seal_is_read_as_one_and_stikks_seal_finishes_it`**:
  - build the state as `measure063.sh` §A does: commit, copy `.prikk/active/default/` aside, seal, copy it back;
  - **first assert, by a raw `status --format json`, that prikk reports `already_sealed`.** If it does not, fail as a
    harness failure: the layout moved (RFC 063 §Tests);
  - then: stikk's queue read has `already_sealed_in` equal to that block, and stikk's seal preview and execute finish
    it;
  - afterwards the queue is empty, and the branch tip is still that block, so no new block was made.

## 6. Commit 5: the publication sentences (decision 10)

Add three entries to `stikk-core/src/glossary.rs`, in the existing entries' form, and route them in `present.rs` as
those are:

| Matched on (lowercased, as `classify` lowercases) | Gloss |
|---|---|
| `repository mutation is blocked by incomplete ref publication;` (**with the semicolon**) | decision 10, first |
| `a seal stopped after it published, before it cleaned the queue` | decision 10, second |
| `another command is publishing` | decision 10, third |

- **The semicolon is the point** (F9): 0.58's stopped-seal sentence starts with the same words and then a **colon**.
  The two need different glosses.
- **The order of matching is the table's.** A unit test feeds 0.58's stopped-seal sentence (measured, §A2) and asserts
  it gets the second gloss, not the first.
- **The moved-directory and history-replay sentences get no gloss.** Two unit tests pin that each, under both class
  words, degrades to a verbatim refusal, in the form of `classify/tests.rs:283`'s loop. Take the sentences from RFC 063
  F9.
- The classifier is not changed: none of these is a lock, and RFC 017 §3's rule holds.

## 7. Commit 6: the range, the record (decision 11)

- **`0.58.0`** in every place RFC 063 decision 11 lists. `grep -rn "0\.55\.0" README.md crates/stikk/README.md docs/src`
  before and after, and in the request, give every remaining hit with why it stays.
- **`ASM-2`**, appended in its shape: *"…, and to **0.58.0 by RFC 063**, 2026-11-…: …"*. One sentence each on F2, F4
  and F5, and that it was the first re-baseline whose fixtures had to change for a prikk security fix.
- **`CHANGELOG.md`, `## Unreleased`:**
  - `### Changed`: validated through prikk 0.58.0; a commit goes to prikk's current branch, with the switch named for
    another; merging into the branch you are on is offered from prikk 0.56; a switch with work queued for the target
    says what it would do.
  - `### Added`: an interrupted seal is shown as one, and Seal finishes it.

  Handoff B adds its own lines.

## 8. The traps

1. **Each version gate reads the handshake, never the constant `VALIDATED_MAX_MINOR`.** The ceiling is not a band.
2. **Decision 4 blocks inside `compute`, before a token exists.** A refusal after confirmation would tell the user too
   late.
3. **`inert` on every branch name and id in a sentence.** These are repository strings (`C-T2a`).
4. **The stopped-seal leg must never pass without prikk reporting the state** (§5).
5. **Match the generic sentence with its semicolon** (§6).
6. **Paste captures as the file holds them, and grep before committing.**

## 9. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Give the suite's counts after commit 1 and after
the last commit. Package to `.git-exclude/review-request/063-prikk-0-58-rebaseline-a/review-request-v1.md`, and lead
with:
- §1's two diffs;
- a table of the eight failing cases: which commit fixed each, and how;
- the new legs and the counts per platform;
- §0's captures.

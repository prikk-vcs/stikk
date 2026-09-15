# Handoff — what a commit will author (v1)

> **Amended 2026-09-16.** RFC 032's amendments A1–A4 rule on this handoff's first review: an unpublished ref's words also
> depend on its queue (A2), commit's clean-blocked reason carries the source-present notice (A3), and rows 1 and 3's
> deletion is `delete-file` in `prikk commit`'s output and `delete-node` in `status-report-v1` (A4). Where this handoff
> disagrees with them, the amendments win.

**Companion to:** [RFC 032](../../accepted/032-what-a-commit-will-author.md). Accepted 2026-09-16; **Q1 ruled (a)**.
**This handoff is all of RFC 032:**
- declared renames marked when prikk lists both halves;
- declarations prikk will not author named, never counted;
- a ref with no published history named as one;
- both on the Changes view and on commit's confirmation.

**0.8.0's second increment**, on `origin/main` with RFC 031 delivered.
**Design items:** `FR-034`, `FR-050`, `T-T4`, `C-T2b`, `C-T2c′`, `ER-02`, `UD-06`, `UD-08`, RFC 027, RFC 030 (amendment A1),
RFC 031.

> **Three traps, and each one fails silently.**
>
> 1. **Publication state must not live inside `ChangesView`** (§4). RFC 030 re-reads the worktree at Enter, builds a view
>    through `changes::from_status`, and requires it to **equal** the preview's view. If the preview's view carried a
>    `published` flag read from `refs()`, the re-read would not carry it, and **every commit would go stale**. RFC
>    030's positive-control suite test commits the fixture's `readme.txt` on an **unpublished** `heads/main`, so it will
>    catch this. **It must stay green unmodified.**
> 2. **A rename is marked only when prikk lists both halves** (§3). prikk keeps reporting a declaration after its
>    destination is deleted or renamed again, and then commits a delete, not a rename (RFC 032 F6). **A mark drawn from
>    the declaration alone is the defect this RFC exists to prevent.**
> 3. **A declaration whose source is present again is a notice, never a prevention** (§5). prikk's per-entry verdict
>    says `authored` while `commit` refuses (F7). RFC 027 prevents only on prikk's verdict, and that ruling stands.

---

## 1. Scope

**In, in this order:**
1. Re-measure (§2).
2. The rename analysis, in core, from the worktree report alone (§3).
3. Publication state, beside the view (§4).
4. The words (§5).
5. The Changes view and the untracked filter (§6).
6. Commit's confirmation (§7).
7. Tests (§8), then docs, changelog and Breaking (§9).

**Out:**
- stikk comparing file contents (F3);
- running `prikk mv`, or clearing a declaration;
- preventing F7's commit;
- any change to what `prikk commit` authors;
- any change to RFC 030's re-read;
- Patch detail and Compare.

## 2. Measured at prikk 0.42.0 — verify each, do not copy

Setup for every row except D: `a.txt` and `keep.txt` committed and sealed on `heads/main`. "mv" is `prikk mv a.txt b.txt`,
which moves the file itself.

| # | Then | `worktree-status` `changes` | `declarations` | `prikk commit` |
|---|---|---|---|---|
| **A** | mv | `missing a.txt`, `untracked b.txt` | `a.txt → b.txt` | `rename-path a.txt → b.txt` |
| **B** | mv; `keep.txt` edited | A's two, plus `modified keep.txt` | `a.txt → b.txt` | `edit-text keep.txt`, `rename-path a.txt → b.txt` |
| **C** | mv; `b.txt` edited | **identical to A** | `a.txt → b.txt` | `rename-path a.txt → b.txt`, **`edit-text b.txt`** |
| **1** | mv; `b.txt` deleted | `missing a.txt` | still `a.txt → b.txt` | **`delete-node a.txt`** |
| **2** | mv; `a.txt` recreated | `untracked b.txt`, `authored` | still `a.txt → b.txt` | **refuses:** `precondition not met: a.txt -> b.txt: the source is present in the worktree again; …` |
| **3** | mv; shell `mv b.txt c.txt` | `missing a.txt`, `untracked c.txt` | still `a.txt → b.txt` | **`delete-node a.txt`, `create-file c.txt`** |
| **5** | mv; shell `mv b.txt a.txt` | **none** | still `a.txt → b.txt` | **refuses** with 2's text and advice |
| **D** | fresh repository, files written, nothing committed | `tracked_files: 0`, every file `untracked` | `[]` | first commit; `status`: `heads/main RefState: <not published>` |

**Also measure, and report:**
- **A fresh, empty repository with no files.** Does `worktree-status` report `clean`? §5 has a sentence for each
  answer.
- **Whether any command but `init` leaves a focused ref unpublished.** RFC 032 F4 reasons that only `heads/main` can
  be.
- **prikk 0.38.0's prose for A**, to confirm the prose reader's declarations drive the same analysis. The suite does not
  run 0.38, so a fixture test holds it.

## 3. The rename analysis — in core, from the worktree report alone (decisions 1 and 2)

**In `changes::from_status`, which stays a pure function of `WorktreeStatus`.** For each declaration, in this order:

1. **Source present again** — no `missing` entry at `old_path` (rows 2 and 5).
2. **Destination absent** — a `missing` entry at `old_path`, but no `untracked` entry at `new_path` (rows 1 and 3).
3. **Paired** — a `missing` entry at `old_path` **and** an `untracked` entry at `new_path` (rows A, B and C).

**The order matters:** row 5 has no entries at all, so checking the destination first would misname it.

**Carry the result in the view-model**, named in the crate's idiom:
- each declaration's state;
- on `ChangeEntry`, **for paired entries only**, which half it is: the source, naming its destination, or the
  destination, naming its source;
- the count of paired renames.

`declarations` stays as RFC 030 made it.

**These fields are derived from the report alone**, so RFC 030's preview and re-read still build equal views. **Do not
read anything else into them.**

## 4. Publication state — beside the view, never inside it (decision 5)

- **A ref has published history when it appears in `prikk.refs(repo)`**, compared by name, closed and received refs
  included. Checking membership, rather than `heads/main RefState`, is what makes it true for any ref at every supported
  prikk.
- **The Changes operation** reads `refs()` in the same core call as `worktree_status`, and returns the publication state
  **beside** the `ChangesView`: a small pair, or a new return type. `Screen::Changes` holds it beside its view, and the
  in-place refresh (RFC 031) updates both.
- **Commit's `compute`** reads `refs()` after the worktree report and puts only the words on `ConfirmationSummary`.
  **`CommitPreview::changes`, and the `ChangesView` the `CommitToken` owns, carry nothing from it.**
- **A failed `refs()` read fails the operation**, as every other read there does. Never guess "published".
- **The race is accepted, and documented in a comment:** two reads, so a terminal `prikk seal` between them can show
  §5's words once. RFC 031 refreshes within five seconds, and RFC 030's token stops a confirmation armed on the old
  state.

## 5. The words — core's, exactly

`{old}`, `{new}` and `{ref}` are repository text, rendered through `inert`.

| Where | When | Words |
|---|---|---|
| Changes, second counts row | prikk ≥ 0.38 | append ` · renames {pairs}` |
| Changes, a paired **source** row | paired | after prikk's note: `· declared rename → {new}` |
| Changes, a paired **destination** row | paired | after prikk's note: `· declared rename ← {old}` |
| Changes, once under the entries | at least one pair | `a declared rename is authored as a rename; prikk does not report whether its content also changed` |
| Changes, and commit's card | destination absent | `declared rename {old} → {new}: {new} is not in the worktree, so prikk will not author it as a rename` |
| Changes, and commit's card | source present again | `declared rename {old} → {new}: {old} is present again, and prikk refuses to commit until the declaration is resolved` |
| Changes headline, replacing `N change(s) against baseline` | no published history, and changes | `{ref} has no published history — every file is listed as untracked, and a commit would be its first` |
| Changes headline, replacing `clean against baseline` | no published history, and clean (if §2 measures it) | `{ref} has no published history, and nothing in the worktree to commit` |
| Commit card, under the targets | no published history | `{ref} has no published history: this would be its first commit` |
| Commit card, counts | at least one pair | a `renames` count, with the pair count |
| Commit card, one line under the counts | at least one pair | `each rename is also counted above as one missing and one untracked path` |

- **Styles:** stikk's annotations are distinguishable from prikk's note, which keeps its style. The two unmatched-declaration
  sentences and the no-published-history line are in `warn`.
- **No way out is offered for "source present again"** (RFC 032 F8). If you find one that works at 0.42, **report it; do not
  add it.**
- **Measure every sentence against §2's rows before using it.** If one is not true of its row, **stop and report**.

## 6. The Changes view, and the untracked filter

- **Entry rows render §5's annotations** after prikk's note, and wrap rather than clip (RFC 024).
- **The unmatched-declaration sentences and the content sentence** sit under the entries, above the whole-worktree line.
- **The untracked filter (`u`, `UD-08`) never hides a paired destination.** It is half of a rename the commit will author.
  - Its hidden-count sentence counts only what it actually hid.
  - With the filter on, a paired source row's `→ {new}` still has its destination row on screen.

## 7. Commit's confirmation

- **`ConfirmationSummary` gains** what §5's card rows need: the no-published-history line, the unmatched-declaration
  sentences, and the rename line. `counts` gains `("renames", pairs)` when pairs exist.
- **Where they render** (`render_confirmation`):
  - the no-published-history line under the targets, beside RFC 029's branch notice;
  - the rename count in the counts line, and its explanation under it;
  - the unmatched-declaration sentences under that.
- **None of these displaces the consequence or the footer** (RFC 024; RFC 028 B). They are prose rows, measured like the rest.
- **Seal's confirmation is unchanged.**

## 8. Tests

**Core, over `NullBackend` or constructed `WorktreeStatus` values shaped exactly like §2's rows:**
1. **The analysis for every row:** A, B and C paired; 1 and 3 destination absent; 2 and 5 source present again.
   **Row 5 must come out "source present again", not "destination absent"**, which proves the order.
2. **`WORKTREE_RENAME_0_38_FIXTURE` (prose) analyses as paired**, so prikk 0.38's declarations drive the same result.
3. **Every §5 sentence**, byte-exact, in the view-model and in the summary.
4. **Publication:** a ref in `refs()` is published; a ref absent from it is not; a failed `refs()` read fails the operation.
5. **The trap-1 guard, in core:** preview a commit on an unpublished ref, re-read an equal worktree, confirm, and assert the
   commit runs and is not `Stale`.

**TUI captures at 80 columns:**
- paired rows with their annotations and the content sentence;
- both unmatched-declaration sentences;
- the unpublished headline;
- the untracked filter on, with a paired destination still shown;
- commit's card with renames counted, its explanation, an unmatched sentence and the no-published-history line, **with the
  consequence and footer whole**.

**Real-binary suite, at prikk ≥ 0.38** (skip announced below):
- **A:** view and summary as §5, then `prikk commit` authors one `rename-path`.
- **B** and **C:** marked, counted, the content sentence shown; commit authors what §2 says.
- **1** and **3:** no mark, no rename count, the destination-absent sentence; commit authors what §2 says.
- **2:** the source-present sentence; `prikk commit` refuses with prikk's text.
- **5:** records 0.42's behaviour as §2 states it. **If a later prikk clears the declaration, this test fails with a message
  naming the change**; it does not assert that a loop is right.

**At both ends — D:** the unpublished headline and card line; `commit_preview` then `commit_confirm_and_execute` commits, not
stale; then `prikk seal`, after which the Changes operation reports published history.

**RFC 030's `rfc030_a_file_added_between_preview_and_confirmation_is_stale_then_a_fresh_preview_commits` stays green,
unmodified.**

## 9. Docs, changelog, Breaking

**On delivery:**
- **`FR-034`** records that the Changes view marks a declared rename when both halves are listed, names declarations prikk will
  not author, and names a ref with no published history.
- **`FR-050`** records the confirmation's rename count and its lines.

**Changelog, `## Unreleased`:**
- **`### Added`**: a file renamed with `prikk mv` is shown as one declared rename, on the Changes view and in commit's
  confirmation, when prikk lists both halves.
- **`### Added`**: a declaration prikk will not author is named, not counted: its destination is gone, or its source is back,
  and prikk refuses.
- **`### Fixed`**: a ref's first commit no longer reads as untracked files "against baseline". stikk says it has no published
  history.
- **State F3's limit** in the Added entry: prikk does not report whether a renamed file's content also changed.

**Breaking, by API diff.** Expect at least:
- `ChangeEntry`'s new field;
- `ChangesView`'s new field or fields;
- the Changes operation's return type, if it changes;
- `ConfirmationSummary`'s new fields;
- `Screen::Changes`' new field.

## 10. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV; gate 6 on
stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite and supply-chain run ids at one
SHA, and Docs after the push.

## 11. Acceptance criteria

1. **§2 re-measured**, including the empty repository, which commands can leave a ref unpublished, and 0.38's prose.
2. **The analysis:** pure, in `from_status`, in §3's order; marks only on paired entries; row 5 classified correctly.
3. **Publication state:** beside the view, never inside it or inside `CommitPreview::changes`; from `refs()` membership; a failed
   read fails the operation; the race documented.
4. **§5's words, byte-exact**, each measured against its row; no way out offered for "source present again".
5. **The untracked filter** never hides a paired destination.
6. **Commit's card:** the rename count, its line, the unmatched sentences and the no-published-history line, with the consequence
   and footer whole.
7. **§8's tests, captures and suite legs**, with RFC 030's positive control unmodified and green.
8. **§9's docs, changelog, and a Breaking table built by API diff.**
9. **Eight gates** under the toolchain rule; run ids at one SHA.
10. **Nothing tagged or published.**

## 12. Submit

Package to `.git-exclude/review-request/032-what-a-commit-will-author/review-request-v1.md`.

**In this order:**
1. **§2's re-measurement**, including the three extra checks.
2. **The trap-1 guard**: core test 5, and RFC 030's suite test green.
3. **The suite legs for rows 1, 3 and 5**, the cases where a declaration and the commit disagree.
4. **The Changes captures, and the commit card capture.**
5. **The Breaking table.**

**And tell me whether any worktree state you tried makes a declaration and prikk's `changes` disagree in a way §3's three states
do not cover.** RFC 032 measured seven rows; the ones it did not are the ones that matter.

**Push once approved.**

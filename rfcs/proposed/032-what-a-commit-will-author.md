# RFC 032 — What a commit will author: declared renames, and a ref with no published history

**Status.** **Proposed 2026-09-16** by the architect: 0.8.0's second increment, taking the roadmap's items 2 and 4
together, because both are the same failure. **One open question** (Q1), yours.
**Tracks.** `FR-034`, `FR-050`, `T-T4`, `C-T2b`, `C-T2c′`, `ER-02`, `UD-06`, RFC 008, RFC 021 (F0), RFC 030
(amendment A1), letter 013 (drafted).
**Touches.** `stikk-core` (the Changes view-model, commit's confirmation summary); `stikk-tui` (the Changes view);
`stikk-real-binary`; on delivery, `requirements.md`.

## Summary

**Before a user commits, stikk shows what the commit will author, and in two ordinary cases it shows something
else.**

- **A file renamed with `prikk mv`** is shown as one file deleted and another added, on the Changes view and in the
  confirmation's counts. prikk authors **one rename**. stikk has read prikk's rename declarations since RFC 030,
  but only to notice a change at Enter; it renders none of them.
- **A repository's first commit** is shown as *N untracked* against a baseline, with nothing saying there is no
  baseline, and that this commit would be the ref's first.

Both are `T-T4`'s confident-but-wrong picture, in the one place stikk exists to be right: just before a signature.

## Findings

All measured on prikk 0.42.0 binaries, the last against 0.28.0 too.

### F1 — a declared rename is shown as a delete and an add

`a.txt` sealed, then `prikk mv a.txt b.txt`, which moves the file itself:

| Surface | What it says |
|---|---|
| `worktree-status --format json`, `changes` | `missing a.txt`, `untracked b.txt` |
| `worktree-status --format json`, `declarations` | `[{"old_path": "a.txt", "new_path": "b.txt"}]` |
| `worktree-status` prose (≥ 0.38) | the two entries, then `live rename declarations: 1`, `a.txt -> b.txt`, and prikk's note that each declaration *"is authored into the next `prikk commit` as a RenamePath"* |
| **what `prikk commit` authors** | **`rename-path a.txt → b.txt`**, one operation |
| **stikk's Changes view** | `missing a.txt`, `untracked b.txt`, and nothing else |
| **stikk's commit confirmation** | counts `missing 1`, `untracked 1` |

**prikk's own prose says it plainly, and stikk's view drops the sentence.** The declaration is in the report stikk
already parses (RFC 030 A1).

### F2 — beside other changes, a rename is its own operation

The same rename, with `keep.txt` also edited: `changes` lists all three, and `prikk commit` authors **two
operations**, `edit-text keep.txt` and `rename-path a.txt → b.txt`.

### F3 — prikk's report cannot say whether a renamed file's content also changed

The same rename, then **`b.txt` edited after the move**. `worktree-status` is **identical to F1's**, both `changes`
and `declarations`, but `prikk commit` authors **`rename-path a.txt → b.txt` and `edit-text b.txt`**.

**So from prikk's report, a rename and a rename-plus-edit are indistinguishable.** stikk must not show a declared
rename as *only* a rename (`C-T2c′`). **Letter 013 asks prikk** whether a declaration could say so.

### F4 — a ref with no published history reads as N untracked files

A freshly initialised repository, two files written:

| Surface | prikk 0.42 | prikk 0.28 |
|---|---|---|
| `status` | `heads/main RefState: <not published>` (plus `current branch: heads/main`) | `heads/main RefState: <not published>` |
| `worktree-status` | `tracked_files: 0`, every file `untracked` | `tracked files: 0`, every file `untracked` |
| **stikk's Changes view** | *"2 change(s) against baseline"*, two untracked rows | the same |
| **stikk's commit confirmation** | counts `untracked 2` | the same |

**"Against baseline" is false: there is none.** The commit would be the ref's first. prikk's deliberate model reads
an unpublished ref as an empty baseline (the roadmap's item, measured at 0.28 and 0.41), so **nothing is wrong in
prikk, and the words are stikk's to fix**.

**Which refs can be focused while unpublished.** `heads/main` on a fresh repository: stikk opens on it (RFC 029), and
the empty picker offers it. `branch create` publishes at an existing target, and `branch switch` needs an existing
branch, so **by prikk's own commands no other ref reaches stikk unpublished**. The handoff re-measures this rather
than trusting help text.

### F5 — where a user sees either

Commit's confirmation shows **counts only**. The Changes view (`w`) is where a user reads paths before pressing
`C`. **Both** show F1 and F4 wrong, so both change.

## Decisions

1. **The Changes view shows declared renames**, from `ChangesView::declarations`:
   - a **renames** row in the header counts: `renames N`. It is shown at prikk ≥ 0.38, and omitted below, where no
     rename can be declared (RFC 030 A1's version fact);
   - each declaration's two entries are **marked as one rename** — how, see Q1;
   - **once, under the entries**, stikk's words: *"a declared rename is authored as a rename; prikk does not report
     whether its content also changed"* (F3). It is shown only when a declaration exists, and **removed on the day
     prikk reports it** (letter 013).
2. **Commit's confirmation counts renames.** A `renames` count joins the others, and one line under the counts, only
   when a declaration exists: *"each rename is also counted above as one missing and one untracked path"*. That keeps
   prikk's per-kind counts intact (`ER-02`) and says why they look doubled.
3. **A ref with no published history is named as one**, from **whether the ref appears in prikk's `refs()`**, read in
   the same core operation as the worktree report. That is true for any ref, not only `heads/main`, and holds at every
   supported prikk.
   - **Changes view**, in place of *"N change(s) against baseline"*: *"{ref} has no published history — every file is
     listed as untracked, and a commit would be its first"*.
   - **Commit confirmation**, one line under the targets: *"{ref} has no published history: this would be its first
     commit"*.
4. **Words are core's**, as every view's are. The TUI renders them.
5. **The suite drives both at real binaries:**
   - **at prikk ≥ 0.38:** F1's rename, with the view-model's renames and pairing and the confirmation's count, then
     `prikk commit` authoring `rename-path`. **Plus F3's rename-and-edit**, asserting the view makes no claim that
     content is unchanged. **Below 0.38 the skip is announced**;
   - **at both ends:** F4's fresh repository, with the no-published-history words in both places, then the first
     commit.
6. **Breaking inside 0.8.0**, which is already breaking (RFC 031): `ChangesView` and `ChangeEntry` gain what decisions
   1 and 3 need.

## Open question

### Q1 — how does the Changes view show the two entries a declaration pairs?

- **(a) Annotate them, keeping prikk's two rows.** `missing a.txt — declared rename → b.txt`, and `untracked b.txt —
  declared rename ← a.txt`. Every path prikk listed stays listed, under prikk's kind (`ER-02`), and the pairing is
  stikk's annotation beside it.
- **(b) Fold them into one row.** `renamed a.txt → b.txt`, replacing both. It reads the way the commit authors, but it
  is stikk rewriting prikk's report: the missing and untracked entries prikk gave are no longer on screen, and the
  header's counts would no longer match the rows beneath them.

**My lean is (a).** It adds what prikk's JSON already says without taking away what its `changes` says, and the
counts still match the list, which RFC 027 made a rule.

## Delivery

**One handoff**, after Q1 is ruled.

## What this RFC does not do

- **No content comparison by stikk.** stikk reads no file bytes to guess whether a renamed file was edited (F3); it
  says prikk does not report it, and letter 013 asks.
- **No declaration authoring.** stikk never runs `prikk mv`.
- **Nothing about Patch detail or Compare**, which are their own RFCs.
- **No change to what `prikk commit` authors**, or to RFC 030's re-read at Enter, which already compares declarations.

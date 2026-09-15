# RFC 032 — What a commit will author: declared renames, and a ref with no published history

**Status.** **Accepted by the project owner 2026-09-16; Q1 ruled (a).** Proposed the same day by the architect: 0.8.0's
second increment, taking the roadmap's items 2 and 4 together, because both are the same failure. **Revised before
acceptance** after the owner asked about risks to user operation and data safety: F6–F8 are measured, and decisions
1–4 were narrowed to what they found. **Amended 2026-09-16** by the architect after the handoff's first review and
prikk's reply 014 (see *Amendments*).
**Tracks.** `FR-034`, `FR-050`, `T-T4`, `C-T2b`, `C-T2c′`, `ER-02`, `UD-06`, `UD-08`, RFC 008, RFC 021 (F0), RFC 027
(Q1 (b)), RFC 030 (amendment A1), RFC 031, letter 013, prikk reply 014.
**Touches.** `stikk-core` (the Changes view-model, commit's confirmation summary); `stikk-tui` (the Changes view);
`stikk-real-binary`; on delivery, `requirements.md`.

## Summary

**Before a user commits, stikk shows what the commit will author, and in two ordinary cases it shows something
else.**

- **A file renamed with `prikk mv`** is shown as one file deleted and another added, on the Changes view and in the
  confirmation's counts. prikk authors **one rename**. stikk has read prikk's rename declarations since RFC 030, only
  to notice a change at Enter, and renders none of them.
- **A repository's first commit** is shown as *N untracked* against a baseline, with nothing saying there is no
  baseline, and that this commit would be the ref's first.

**And a declaration is not the same thing as a rename.** prikk keeps reporting a declaration after the worktree stops
matching it, and then commits something else, or refuses (F6–F8). **So stikk may call a rename a rename only when
prikk's report shows both of its halves.**

## Findings

All measured on prikk 0.42.0 binaries, F4 against 0.28.0 too.

### F1 — a declared rename is shown as a delete and an add

`a.txt` sealed, then `prikk mv a.txt b.txt`, which moves the file itself:

| Surface | What it says |
|---|---|
| `worktree-status --format json`, `changes` | `missing a.txt`, `untracked b.txt` |
| `worktree-status --format json`, `declarations` | `[{"old_path": "a.txt", "new_path": "b.txt"}]` |
| `worktree-status` prose (≥ 0.38) | the two entries, `live rename declarations: 1`, `a.txt -> b.txt`, and prikk's note that each declaration *"is authored into the next `prikk commit` as a RenamePath"* |
| **what `prikk commit` authors** | **`rename-path a.txt → b.txt`**, one operation |
| **stikk's Changes view** | `missing a.txt`, `untracked b.txt`, and nothing else |
| **stikk's commit confirmation** | counts `missing 1`, `untracked 1` |

### F2 — beside other changes, a rename is its own operation

The same rename, with `keep.txt` also edited: `prikk commit` authors **two operations**, `edit-text keep.txt` and
`rename-path a.txt → b.txt`.

### F3 — prikk's report cannot say whether a renamed file's content also changed

The same rename, then **`b.txt` edited after the move**. `worktree-status` is **identical to F1's**, yet `prikk commit`
authors **`rename-path a.txt → b.txt` and `edit-text b.txt`**. **A rename and a rename-plus-edit are
indistinguishable from the report** (`C-T2c′`). Letter 013 asks prikk.

### F4 — a ref with no published history reads as N untracked files

A freshly initialised repository, two files written:

| Surface | prikk 0.42 | prikk 0.28 |
|---|---|---|
| `status` | `heads/main RefState: <not published>` | the same |
| `worktree-status` | `tracked_files: 0`, every file `untracked` | `tracked files: 0`, every file `untracked` |
| **stikk's Changes view** | *"2 change(s) against baseline"* | the same |
| **stikk's commit confirmation** | counts `untracked 2` | the same |

**"Against baseline" is false: there is none.** This is prikk's deliberate model, not a defect, so the words are
stikk's to fix. By prikk's own commands (`branch create` publishes; `branch switch` needs an existing branch), **only
`heads/main` reaches stikk unpublished**; the handoff re-measures this.

### F5 — where a user sees either

Commit's confirmation shows **counts only**; the Changes view (`w`) is where a user reads paths before `C`. **Both**
show F1 and F4 wrongly.

### F6 — a declaration outlives the rename it declared, and then commit authors something else

`prikk mv a.txt b.txt`, then the worktree changes under the declaration:

| Then the user… | `worktree-status` | `declarations` | **`prikk commit` authors** |
|---|---|---|---|
| **deletes `b.txt`** | `missing a.txt` | still `a.txt → b.txt` | **`delete-node a.txt`** — no rename |
| **renames `b.txt` to `c.txt` in the shell** | `missing a.txt`, `untracked c.txt` | still `a.txt → b.txt` | **`delete-node a.txt`, `create-file c.txt`** — no rename |

**A view that marked a rename from the declaration alone would say "renamed to b.txt" over a commit that deletes
`a.txt`.** That is `T-T4`'s confident-but-wrong picture, one keypress before a signature. **This finding is why
decision 1 pairs only what prikk lists.**

### F7 — with the source present again, commit refuses, but the report says it would author

`prikk mv a.txt b.txt`, then `a.txt` recreated beside `b.txt`:
- `worktree-status` lists `untracked b.txt`, **`authoring: "authored"`**, and the declaration.
- **`prikk commit` refuses:** *"precondition not met: a.txt -> b.txt: the source is present in the worktree again; the
  declared move was not completed on disk. Run `prikk mv` again, or move b.txt back to a.txt to clear the
  declaration before committing"*.

**prikk's per-entry verdict and its commit disagree here**, so RFC 027's prevention, which rests on prikk's verdict
alone (Q1 (b)), cannot catch it. **Nothing is lost**: the refusal writes nothing. But stikk offers a commit prikk will
refuse. Letter 013 asks prikk to mark such an entry refused, which would make RFC 027 prevent it with no stikk change.

### F8 — prikk's own way out of F7 does not clear the declaration

Moving `b.txt` back to `a.txt`, as both prikk's note and its refusal advise, leaves **no changes** in `worktree-status`,
**the declaration still listed**, and **`prikk commit` refusing with the same advice**, which the user has just
followed. **stikk must not repeat advice it measured to loop.** Letter 013 reports it.

## Decisions

1. **A rename is marked only when prikk's report shows both halves:** a `missing` entry at `old_path` **and** an
   `untracked` entry at `new_path`, beside the declaration.
   - **Those two rows are marked as one rename** (Q1).
   - **The header counts `renames N` over these pairs only.** It is shown at prikk ≥ 0.38, and omitted below, where no
     rename can be declared.
   - **Once, under the entries, when any pair exists:** *"a declared rename is authored as a rename; prikk does not
     report whether its content also changed"* (F3). It is removed on the day prikk reports that.
2. **A declaration without both halves is named for what it is, and never counted as a rename:**
   - **its destination is not listed** (F6): *"declared rename {old} → {new}: {new} is not in the worktree, so prikk will
     not author it as a rename"*. That is true of both F6 rows as measured;
   - **its source is present again** (F7, F8): *"declared rename {old} → {new}: {old} is present again, and prikk refuses
     to commit until the declaration is resolved"*. **It is a notice, not a prevention**, keeping RFC 027's ruling. It is
     shown on the Changes view and on commit's confirmation. **No way out is offered in stikk's words** until one is
     measured to work (F8).
   - The handoff measures each wording on the binary before using it.
3. **The untracked filter (`u`, `UD-08`) never hides a paired destination.** It is half of a rename the commit will
   author, not an ordinary untracked file.
4. **Commit's confirmation counts renames honestly:**
   - **`renames N`** over pairs only, with one line when N > 0: *"each rename is also counted above as one missing and one
     untracked path"*;
   - **each unmatched declaration** gets decision 2's sentence on the card.
5. **A ref with no published history is named as one**, from **whether the ref appears in prikk's `refs()`**, read in the
   same core operation as the worktree report, for any ref, at every supported prikk:
   - **Changes view**, in place of *"N change(s) against baseline"*: *"{ref} has no published history — every file is
     listed as untracked, and a commit would be its first"*;
   - **commit confirmation**, one line under the targets: *"{ref} has no published history: this would be its first
     commit"*.
   - **The race, stated.** `refs()` and `worktree-status` are two reads, so a terminal `prikk seal` between them could
     show these words once for a ref that has just been published. **RFC 031's check notices the seal and refreshes
     within five seconds**, and **RFC 030's change token stops a confirmation armed on the old state**. The window is
     accepted, not hidden.
6. **Words are core's.** The TUI renders them.
7. **The suite drives every case at real binaries, at prikk ≥ 0.38** (skip announced below):
   - F1, F2, F3;
   - **F6's two rows**, asserting no rename is marked and `prikk commit` authors what the table says;
   - **F7**, asserting the notice is shown and `prikk commit` refuses with prikk's text;
   - **F8**, recording what 0.42 does. **If it changes, the test fails and says so**, rather than asserting a loop is
     correct.

   **F4 at both ends.**
8. **Breaking inside 0.8.0**, which is already breaking (RFC 031): `ChangesView` and `ChangeEntry` gain what decisions
   1, 2 and 5 need.

## Open question

### Q1 — how does the Changes view show the two entries a paired rename has?

- **(a) Annotate them, keeping prikk's two rows.** `missing a.txt — declared rename → b.txt` and
  `untracked b.txt — declared rename ← a.txt`. Every path prikk listed stays listed under prikk's kind (`ER-02`), and
  the counts still match the rows, which RFC 027 made a rule.
- **(b) Fold them into one row.** `renamed a.txt → b.txt`, replacing both. It reads as the commit authors, but it
  rewrites prikk's report, and the header's counts would no longer match the rows beneath them.

**My lean is (a).** After F6–F8 it is also the safer one. **A folded row is only as true as the pairing behind it**,
while two annotated rows still show exactly what prikk listed if a declaration and the worktree ever disagree in a way
not measured here.

### RULED by the project owner, 2026-09-16: (a)

**A paired rename keeps prikk's two rows, each annotated as half of one declared rename.** Every path prikk listed stays
on screen under prikk's kind, and the header's counts still match the rows. The pairing is decision 1's: both halves
listed, or no mark.

## Amendments — 2026-09-16, from the handoff's first review and prikk's reply 014

### A1 — F4 corrected: `prikk commit` can queue onto a ref never created

The dev team measured, at 0.42.0: **`prikk commit --from-worktree --ref heads/other`, on a ref never created, succeeds**
and queues a patch, with `branch list --all` still empty. So *"only `heads/main` reaches stikk unpublished"* is true of
**stikk's reachability** — the picker offers only `refs()` plus the unpublished `heads/main` row, and prikk's current
branch can be switched only to an existing branch — **not of prikk**. Decision 5's membership test serves any ref either
way, so no decision changes.

### A2 — decision 5 widened: an unpublished ref's queue is its baseline

**Measured at 0.42.0 and 0.38.0:** after a first commit on an unpublished `heads/main`, left unsealed, `refs()` still lists
no branch, `status` still says `<not published>` — and `worktree-status` reports the queued files as **tracked and
unchanged**. A file added afterwards is the only untracked entry, and a second commit adds to the queue. **So "every file is
listed as untracked, and a commit would be its first" is false one ordinary step after row D.**

**The words are chosen from the queue as well as `refs()`.** Orientation's `queued_patches` and `queued_target` say whether
patches are queued **for this ref**, and commit's `compute` already reads Orientation; the Changes operation adds that read.
The race is the one decision 5 already accepts, over three reads.

| Unpublished ref, and… | Changes headline | Commit card, under the targets |
|---|---|---|
| **no queued patch for it**, changes | *"{ref} has no published history — every file is listed as untracked, and a commit would be its first"* | *"{ref} has no published history: this would be its first commit"* |
| **no queued patch for it**, clean | *"{ref} has no published history, and nothing in the worktree to commit"* | — (commit is blocked as clean) |
| **{n} queued patch(es) for it**, changes | *"{ref} has no published history yet — its {n} queued patch(es) are the baseline here, and nothing is sealed"* | *"{ref} has no published history: this adds to its {n} queued patch(es), and nothing is sealed until the queue is sealed"* |
| **{n} queued patch(es) for it**, clean | *"{ref} has no published history yet — nothing in the worktree beyond its {n} queued patch(es), and nothing is sealed"* | — (commit is blocked as clean) |
| a queue **for another ref**, or a count with **no target reported** | *"{ref} has no published history"* | *"{ref} has no published history"* |

**Only prikk's facts decide the row**: `refs()` membership, `queued_target == {ref}`, `queued_patches`, and `clean`. **Nothing
is inferred from `tracked`.**

### A3 — row 5 never reaches the card, so commit's blocked reason carries the notice

Row 5's report is **`clean: true`** (prikk's reply 014 confirms it, with `changes: []` and `refused_count: 0`), so commit's
`compute` blocks it as clean before arming anything, and the card's source-present sentence is unreachable there. **The
blocked reason is incomplete** — prikk would refuse for the declaration, not only because nothing changed. **When a
declaration's source is present again, the clean-blocked reason gains decision 2's sentence**, and the analysis runs on
clean reports too.

**A way out, only where it is measured.** prikk's reply 014 measured that, with the source back and the destination gone
(row 5's state), **`prikk mv {new} {old}` "nets to no move, dropped"**: the declaration is gone and there is nothing to commit.
**Where stikk measures the same at 0.42.0**, and only in that state — no `untracked` entry at `{new}` — decision 2's
source-present sentence gains *"; in a terminal, prikk mv {new} {old} drops it"*. **With both copies present (row 2), no way
out is offered**: prikk's measured route there is setting one copy aside by hand, and stikk does not advise moving a user's
files.

### A4 — an operation's name depends on the surface that prints it

The dev team measured rows 1 and 3's deletion as **`delete-file`** in `prikk commit`'s printed output; the architect's probe
read **`delete-node`** from `status-report-v1`'s queue. **Both are right.** A test names the surface it reads, and asserts that
surface's word.

### A5 — prikk 0.43.0 will report what commit does with each declaration (reply 014)

prikk ruled, for 0.43.0 (which *"will not cut without it"*), additive fields in `worktree-status-report-v1` on each declaration:
- **`resolution`**: `"rename"`, `"deletion"`, `"deletion-ignored"`, `"never-tracked"` or `"refused"`, from the classifier
  `commit` itself uses;
- **`refusal`**, byte-for-byte what `commit` prints, on a refused declaration, counted in **`refused_declaration_count`**;
- **`content_changed`** and **`mode_changed`**, on a declaration that resolves to a rename.

**So this RFC's three-state inference is the path for prikk 0.28–0.42.** At 0.43 the re-baseline (the roadmap's 0.43 item)
replaces it with `resolution`, **prevents on `resolution: "refused"`** — prikk's own verdict, so within RFC 027's ruling — and
replaces F3's content sentence with `content_changed`/`mode_changed`. **prikk names the fields final only when 0.43.0
publishes**, and stikk measures them before relying on them.

## Delivery

**One handoff**, issued on the ruling.

## What this RFC does not do

- **No content comparison by stikk** (F3); it says prikk does not report it, and letter 013 asks.
- **No prevention of F7's commit.** RFC 027 prevents only on prikk's verdict, and letter 013 asks prikk to give one.
- **No declaration authoring or clearing.** stikk never runs `prikk mv`.
- **Nothing about Patch detail or Compare.**
- **No change to what `prikk commit` authors**, or to RFC 030's re-read at Enter, which already compares declarations.

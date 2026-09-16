# RFC 033 — The prikk 0.43 re-baseline: what a declaration resolves to, and a checkout that stopped part-way

**Status.** **Proposed 2026-09-16** by the architect, the day prikk 0.43.0 published. **One open question (Q1).**
Measured against real prikk **0.28.0**, **0.42.0** and **0.43.0** binaries (`cargo install --locked`), against the
**published stikk 0.8.0** from crates.io, and by running stikk's own real-binary suite at 0.28 and 0.43 with only the
validated ceiling raised, in a scratch copy of `c25339f` (the 0.8.0 tag). Evidence:
`.git-exclude/reports/033-prikk-0-43-rebaseline/`.
**Tracks.** `ASM-2`, `NFR-R03`, `FR-034`, `FR-050`, `C-T2b`, `C-T2c′`, `ER-02`, `T-T4`, RFC 027 (Q1 (b)), RFC 030,
RFC 032 (A3, A5, A6, A7), stikk letters 012 and 013, prikk letter 015.
**Touches.** `stikk-prikk` (the ceiling; the declaration fields of `worktree-status-report-v1`; the marker in
`status`), `stikk-core` (the declaration analysis; commit's preview; Orientation), `stikk-tui` (Orientation, the
Changes view), `stikk-real-binary`, the captured fixtures, and on delivery `requirements.md`.

## Summary

**Nothing is broken for anyone running stikk 0.8.0 on prikk 0.43** — measured, not read (F0). This RFC is about
what 0.43 lets stikk say that it could not say before.

prikk 0.43.0 answers both of stikk's last two letters:
- **every rename declaration now carries prikk's own verdict** — what `commit` will do with it, whether the renamed
  file's content or mode changed, and the refusal `commit` would print (letter 013);
- **a refused checkout writes nothing**, and one that genuinely stopped part-way is reported, not discovered at the
  next commit (letter 012).

**So RFC 032's inference can give way to prikk's verdict — almost everywhere.** One measured state is the exception:
**with a directory at a rename's destination, prikk's report says `rename` while `prikk commit` records a deletion**
(F2). RFC 032's rule gets that state right today. **Q1 asks which stikk believes where the two disagree.**

Measuring also found **a sentence stikk 0.8.0 ships that gives a reason it cannot know** (F5) — my ruling, and the
second correction to the same sentence.

## Findings

### F0 — stikk 0.8.0 keeps working on prikk 0.43

- **The real-binary suite: 31 of 32 cases pass** at prikk 0.28 and 0.43, with only the validated ceiling raised.
- **The one failure is a test pinning prikk 0.42's wording.** `rfc032_a_declaration_whose_source_is_back_…` asserts the
  exact refusal prikk printed at 0.42; at 0.43 prikk's refusal names its ways out (F4). **No product code reads that
  wording** — searched, not assumed.
- **Both JSON readers tolerate the added keys.** `worktree-status-report-v1` gains `refused_declaration_count` and four
  keys per declaration; `status-report-v1` gains `interrupted_materialization` and `provisional_worktree`, both `null`
  on an ordinary repository. **No `schema_version` changed.**
- **The published stikk 0.8.0 runs its one-shot orientation on a 0.43 repository whose checkout stopped part-way**,
  exit 0, and says prikk 0.43 is newer than it has validated. **It says nothing about the stopped checkout** (F6).

**So no 0.8.1 is needed.** A user on 0.43 sees what 0.8.0 shows, with prikk's own refusals — which now name their ways
out — carried verbatim.

### F1 — each declaration now resolves, and it matches `commit` in every state but one

Measured at 0.43.0, one fresh repository per row, **`prikk commit` run in every row it names**
(`measure033.txt`, `measure033b.txt`, `measure033c.txt`):

| State after `prikk mv a.txt b.txt` | `resolution` | content / mode | `prikk commit` does | Agrees? |
|---|---|---|---|---|
| A — nothing else | `rename` | false / false | `rename-path` | yes |
| B — an unrelated file edited | `rename` | false / false | `edit-text keep.txt`, `rename-path` | yes |
| C — `b.txt` edited | `rename` | **true** / false | `edit-text b.txt`, `rename-path` | yes |
| C2 — `b.txt` made executable | `rename` | false / **true** | `change-perm b.txt`, `rename-path` | yes |
| 1 — `b.txt` deleted | `deletion` | null | `delete-file a.txt`, *"destination is gone"* | yes |
| 3 — shell `mv b.txt c.txt` | `deletion` | null | `delete-file a.txt`, `create-file c.txt`, *"destination is gone"* | yes |
| 2 — `a.txt` recreated | **`refused`** | null | refuses with the declaration's `refusal`, byte for byte | yes |
| 5 — shell `mv b.txt a.txt` | **`refused`**, with `clean: true` | null | refuses with the declaration's `refusal` | yes |
| `prikk mv` twice, `a → b → c` | `rename`, rewritten to `a → c` | false / false | `rename-path a.txt -> c.txt` | yes |
| `b.txt` listed in `.prikkignore` | `deletion-ignored` | null | `delete-file a.txt`, *"destination is **ignored**"* | yes |
| an untracked file moved | `never-tracked` | null | `create-file`, *"source was never a tracked node"* | yes |
| **`b.txt` replaced by a directory** | **`rename`** | false / false | **`delete-file a.txt`**, *"destination is **ignored**; recorded as a deletion, not a rename"* | **no — F2** |
| `b.txt` replaced by a symlink | `rename` | true / true | refuses the whole commit, over the **path** | see F3 |

**`refused_declaration_count` counted rows 2 and 5 exactly**, and **`worktree-status` still exits 0 in row 5**, as prikk
ruled: the exit code keeps its path-level meaning.

### F2 — a directory at the destination: the report says `rename`, and `commit` records a deletion

Reproduced in **three fresh repositories** — a directory holding a file, twice, and an empty directory:
- **`worktree-status`:** `resolution: "rename"`, `content_changed: false`, `mode_changed: false`, no refusal;
- **`prikk commit`:** `delete-file a.txt`, then *"declaration a.txt -> b.txt: destination is ignored; recorded as a
  deletion, not a rename"*.

**`commit`'s line is word for word the one it prints for a `.prikkignore`'d destination**, and that state resolves
`deletion-ignored`. So `commit` treats a directory at the destination as ignored, and the report's classifier does not.
prikk's letter says the two come from the same function and cannot disagree; **here they do**. Letter 014 reports it.

**Why it matters here:** stikk would state a rename on commit's confirmation, one keypress before a signature, for a
commit that records a deletion. **RFC 032's rule gets this state right today**: prikk lists `missing a.txt` and
`untracked b.txt/q.txt`, never `untracked b.txt`, so the two halves are not both listed.

**Data is not at risk in any reading.** The commit authors exactly what the worktree holds; only its description of the
history's shape would be wrong.

### F3 — `rename` does not mean the commit succeeds

With a symlink at the destination, the declaration resolves `rename`, with content and mode both changed, while the
path is refused and **`commit` refuses the whole commit**. prikk's letter says as much: read a commit's prospects as
`refused_count == 0` **and** `refused_declaration_count == 0`.

**RFC 032 A7 already holds for this:** stikk promises no outcome while prikk reports a refusal.

### F4 — a refused declaration's refusal now names its way out, and prikk measured it

- **Row 5:** *"…the source is present in the worktree again, so the declared move is not what the worktree holds. Run
  `prikk mv b.txt a.txt` to drop the declaration, or `prikk mv a.txt b.txt` to make the move again"*.
- **Row 2:** *"…both paths exist in the worktree … Set one copy aside first -- `prikk mv` refuses while both are there:
  delete a.txt and commit to author the rename, or delete b.txt and run `prikk mv b.txt a.txt` to drop the declaration"*.

**The refusal on the declaration is byte-identical to what `commit` prints** after `error: precondition not met: `.

**RFC 032 A3 added stikk's own measured way out** for row 5, because prikk's 0.42 refusal gave advice that looped. **At
0.43 prikk's refusal carries it**, and more precisely than stikk's words.

### F5 — stikk 0.8.0's destination-absent sentence gives a reason it cannot know

RFC 032 A6 made the sentence *"declared rename {old} → {new}: **{new} is not a file in the worktree**, so prikk will not
author it as a rename"*. **With the destination listed in `.prikkignore`, `b.txt` is a file in the worktree**, and
prikk lists only `missing a.txt`. The consequence the sentence states is true; **the reason is false** (`C-T2b`).

**Below 0.43 the report cannot tell "gone" from "ignored"** — an ignored file is not listed at all — so stikk has no
business naming a reason. Measured at 0.43; the handoff re-measures at 0.42.

### F6 — a checkout that stopped part-way is reported, three ways

Letter 012's state, made by prikk **0.42** and read by **0.43**:

| Where | What 0.43 says |
|---|---|
| `status` prose | a new line: *"interrupted materialization: a checkout or branch switch stopped part-way; move aside any file it named, then run prikk checkout --patch-materialize --ref heads/main or prikk branch switch heads/main"* |
| `status --format json` | `"interrupted_materialization": {"routes": ["prikk checkout --patch-materialize --ref heads/main", "prikk branch switch heads/main"]}`; `null` otherwise |
| `doctor` | `warning [PRIKK-DOCTOR-INTERRUPTED-MATERIALIZATION]`, *"…and `commit` refuses"*, with the same routes |
| `commit` | **`precondition not met:`** *"worktree materialization was interrupted, so the worktree is not verified against its baseline and nothing can be committed; run …"* |

**At 0.42 that same commit was `integrity error:`**, which stikk presents as an integrity finding although nothing is
corrupt — the half of this item the roadmap has carried since reply 013.

**stikk 0.8.0 shows none of the first three.** A user sees an ordinary Orientation, then a commit that refuses.

**And letter 012 itself is fixed:** the same checkout at 0.43 refuses with *"… 1 path(s) in the way: shared.txt …
(nothing was written)"*, writes nothing, sets no marker, and the next commit behaves normally. **stikk runs no checkout**,
so nothing is built for it; RFC 030's carried note closes at 0.43.

### F7 — a provisional worktree: not reproduced

prikk says `checkout --snapshot-materialize` of a block **this repository has not replay-verified** writes a provisional
worktree, and **history-deriving commands refuse until `prikk verify` clears it**. Those are the commands stikk's History
reads.

**Two attempts did not produce one:**
- **a block sealed in its own repository** is already replay-verified: *"provisional: no"*;
- **a block received through `bundle import`**: its tip is not a checkpoint, so `--snapshot-materialize` refuses.

**It is not designed from a letter.** Letter 014 asks prikk for a measured recipe, and the handoff reproduces it
before stikk's words exist.

### F8 — what 0.43 changes that stikk does not use

`bundle export` and `sync build` of a history that deletes an edited text file; `bundle import`'s closing note; a
re-checkout on Windows counting unchanged files. **stikk runs none of these.** Recorded, and nothing built.

## Decisions

1. **The ceiling rises to 43, alone and first**, as the constant's own documentation prescribes, so the suite reports
   what the raise costs. **The one suite assertion pinning 0.42's refusal wording becomes version-aware**: 0.42's text
   below 0.43, 0.43's text at and above it.

2. **At prikk ≥ 0.43 a refused declaration makes commit unavailable** — **RFC 027 Q1 (b)'s rule**, prevention only on
   prikk's own verdict, which RFC 032 could not apply because prikk had no verdict to give.
   - **The trigger is `refused_declaration_count ≥ 1`**, beside RFC 027's `refused_count ≥ 1`.
   - **The reason carries each refused declaration's `refusal` verbatim** (`ER-02`), attributed to prikk. It names
     prikk's own measured ways out, so **stikk adds no suffix of its own at ≥ 0.43** (F4). Below 0.43, RFC 032 A3's
     words are unchanged.
   - **Row 5 is checked before "nothing to commit".** Its report is `clean: true`, and a clean check that ran first
     would hide prikk's verdict behind stikk's.
   - **Where prikk's refusal advises deleting a file**, it is prikk's advice, carried verbatim and attributed. **stikk
     still offers no such advice in its own words.**

3. **At prikk ≥ 0.43, a rename's content and mode come from prikk.** On a rename, `content_changed` and `mode_changed`
   replace RFC 032's `RENAME_CONTENT_NOTE`, which is said only below 0.43. **A7 stands:** while `refused_count ≥ 1`,
   stikk promises no outcome — the symlink state reports both changed while commit refuses (F3).

4. **At prikk ≥ 0.43 the other resolutions are named exactly**, each worded only after it is measured:
   - `deletion` — the destination is gone, and prikk records a deletion;
   - `deletion-ignored` — the destination is ignored, and prikk records a deletion;
   - `never-tracked` — the source was never tracked, so prikk creates the destination and drops the declaration.

5. **Below prikk 0.43, the destination-absent sentence states only what the report shows** (F5):
   *"declared rename {old} → {new}: prikk's report does not list {new}, so prikk will not author it as a rename"* — true
   of a deleted, moved, ignored or directory destination alike.

6. **Where `resolution: "rename"` and the two listed halves disagree — Q1.**

7. **A checkout that stopped part-way is shown, and commit is unavailable while it stands.**
   - **At ≥ 0.43, Orientation says so**, from prikk's report, with prikk's routes verbatim.
   - **At ≥ 0.43, commit's preview is unavailable with that reason** — prikk's own verdict, since its `doctor` and
     `commit` both say commit refuses.
   - **At 0.42, the commit refusal is glossed** as what it is: a checkout that stopped part-way, **not corruption**, with
     the ways out prikk measured in reply 013. stikk cannot show the state earlier on 0.42, because 0.42 does not
     report it.

8. **A provisional worktree is measured before it is designed** (F7). The handoff reproduces it — from prikk's answer
   to letter 014, or independently — and records which of stikk's reads refuse. **If History or Orientation refuse,
   prikk's reason is shown and must not be classified as an integrity finding.** What stikk says beyond that comes back
   to the architect with the measurement; **it does not hold the rest of Handoff A.**

## Open question

### Q1 — where prikk's `resolution` and the two listed halves disagree, which does stikk believe?

**Measured once, in three repositories (F2):** a directory at the destination. prikk's report resolves `rename`;
`prikk commit` records a deletion; RFC 032's rule, both halves listed, says "not a rename" and is right.

- **(a) Both must agree to call it a rename — recommended.** At ≥ 0.43 stikk marks a rename only where
  `resolution == "rename"` **and** both halves are listed. Where prikk resolves `rename` but a half is missing, stikk
  says what `commit` was measured to do — the destination-absent sentence — and letter 014 reports the defect. **Every
  other resolution is prikk's alone.** *Cost:* one rule of inference survives past 0.43, removed when prikk fixes it.
- **(b) Believe `resolution` alone.** It follows prikk's stated contract, and it is the simplest code. *Cost:* until
  prikk fixes F2, **commit's confirmation names a rename that the commit does not author** — a confident wrong picture,
  one keypress before a signature (`T-T4`). The user's data is unaffected; the history's shape is not what they
  confirmed.
- **(c) Adopt the markers and prevention now, and resolution only when prikk fixes F2.** Decisions 2, 3 and 7 go ahead;
  stikk keeps RFC 032's inference for which declarations are renames until a prikk release resolves the directory
  state correctly. *Cost:* two sources of truth for longer, and `deletion-ignored` and `never-tracked` wait.

**My recommendation is (a).** It uses prikk's verdict wherever prikk is right, which is everywhere measured but one
state, and never lets stikk confirm a rename that commit does not author. (b) is what I would pick if prikk had not
measured wrong in the one place that reaches a signature.

**On the owner's question from RFC 032 — risk to data or to user operation:** none of the three options writes, loses or
alters a file. **They differ only in whether a confirmation can describe the commit wrongly.** (a) and (c) cannot; (b)
can, in one state, until prikk ships a fix.

## Delivery

**Two handoffs, in order.**

- **A — the re-baseline**, issued on acceptance: decision 1 (the ceiling, alone, then the suite's version-awareness);
  captures of 0.43's reports (the two new `status` keys, the marker in prose and JSON, the declaration fields);
  decision 7 (the stopped checkout at ≥ 0.43, and the 0.42 gloss); and decision 8's measurement.
- **B — declarations**, issued once A is on `main` and Q1 is ruled: decisions 2 to 6.

**A comes first** because it gives B its fixtures, and because the suite at 0.43 is what tells B what it changed.

## What this RFC does not do

- **No checkout, bundle or sync** in stikk (F6, F8).
- **Nothing below prikk 0.43 changes**, except decision 5's sentence and decision 7's 0.42 gloss.
- **No reading worktree bytes** to guess a content change below 0.43 (`CON-1`).
- **No advice in stikk's own words to delete or move a user's files.** prikk's advice is carried verbatim and
  attributed.
- **No provisional-worktree wording** until the state is reproduced (decision 8).
- **Nothing about Patch detail or Compare.**

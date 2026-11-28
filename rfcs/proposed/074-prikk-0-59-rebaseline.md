# RFC 074 — The prikk 0.59.0 re-baseline

**Status.** **Proposed 2026-11-28** by the architect, as 0.23.0's conditional increment
(`.git-exclude/release/0-23-0-scope.md`, revision 2: *"if prikk releases 0.59.0 before the cut"*; it did, on
2026-11-28). The owner set the order on 2026-11-28 (*"Authorized on both."*): after RFC 070's Handoff A, with the reply
to prikk's letter 034 drafted between them. **No open question.**
**Measured** on `main` at `c8b78f4`, against prikk's source at the `0.58.0` and `0.59.0` tags
(`.git-exclude/projects/prikk`) and its 0.59.0 CHANGELOG. The published binary is installed on the owner's machine
(`prikk 0.59.0`).
**Tracks.**
- RFC 063, the 0.58.0 re-baseline, whose order this follows: the raise comes early, so the suite reports its cost;
- prikk's letter 035 (0.59.0: what changed in what stikk reads);
- stikk's draft 025, whose §2 promises one reader change (decision 5);
- RFC 019's real-binary suite.

**Touches.**
- `crates/stikk-prikk`: the validated ceiling, the readers and the classifier;
- `crates/stikk-core`: the switch guard and the glosses;
- `crates/stikk-real-binary`: the suite against 0.59;
- the suite's binaries (`~/.local/share/stikk-test-prikk/`);
- `CHANGELOG.md`.

## Summary

**prikk 0.59.0 changes no JSON document, no value set and no exit code** (its CHANGELOG: 22 forms, 48 captured states,
compared on both binaries). **What it changes is behaviour and prose.** Four changes reach stikk:
1. **A writer now waits** up to 5 s for a lock or a publication whose holder is running, where it refused at once.
2. **`branch switch` refuses** when the target branch's own queue holds a path the switch would change. In 0.56–0.58
   it overwrote or deleted the committed file. `merge --into`, `archive`/`bundle import --adopt` and `branch create`
   refuse likewise for another branch's queue.
3. **About thirty refusal sentences are rewritten,** most of them to name a command that works.
4. **A lost format marker,** `init` over one, and `setup` and `key generate` into a missing directory now behave
   differently.

**This RFC raises stikk's validated ceiling to 0.59, measures what that costs, and makes stikk show the new refusals
with prikk's own words and the way out.** It also makes every JSON reader read a refusal document first, so a
refusal shows prikk's message whichever command printed it.

## Findings

### F1 — the ceiling and the suite

- **`VALIDATED_MAX_MINOR` is 58** (`stikk-prikk/src/version.rs:160`). With 0.59 installed, stikk runs and says the
  version is unvalidated, as designed.
- **The suite refuses a binary the constant doesn't name** (RFC 021's order), and its binaries are `floor`,
  `ceiling-0.55`, `ceiling-0.57` and `ceiling-0.58`. **There's no `ceiling-0.59` yet.**

### F2 — the strings stikk matches are all still printed (measured in source)

**stikk recognises prikk's prose in two places:**
- 19 classifier patterns (`stikk-prikk/src/cli_backend/classify.rs`);
- glossary codes (`stikk-core/src/glossary.rs`).

**Every one was counted in prikk's non-test source at both tags. None disappears in 0.59:**
- the counts hold or grow;
- `another command is publishing` goes from 4 to 18 sites;
- `repository mutation is blocked by incomplete ref publication;` from 2 to 3;
- `active WAL is owned by` from 4 to 5.

**That's necessary, not sufficient.** A sentence can still be printed but no longer be the one a given state prints.
The suite's runs, and Handoff A's systematic list, settle what this count can't.

### F3 — the generic incomplete-publication sentence is no longer generic

stikk's gloss for `repository mutation is blocked by incomplete ref publication;` (RFC 063 F9) tells a reader who
keeps meeting it to run Verify, because prikk's sentence named no command (`signer-backed seal retry` isn't one).
**0.59 rewrites it per state,** each naming the command that resolves it:
- another command publishing: *run this command again when it finishes*;
- a completable lead: `prikk ref complete <ref>`;
- a stray candidate: `prikk doctor --remove-ref-candidates`.

**stikk's gloss would now sit beside a sentence that already says what to do,** and might contradict it.

### F4 — the switch: stikk already refuses, and now prikk does too

**stikk refuses a switch to a branch the queue holds work for** (`stikk-core/src/switch.rs`,
`WORK_IS_QUEUED_FOR_THIS_BRANCH`, measured at 0.52). At 0.59 prikk refuses the same switch, before any write. Its
refusal names the paths and the exact seal that keeps them (letter 035 §2):
```
refusing to switch to heads/feature: its queue holds 1 commit(s) that change queued.txt, which the switch would overwrite or delete on disk (nothing was written)
… `prikk seal --allow-no-audit --ref heads/feature` publishes them and keeps your files
```
**stikk doesn't guard** `merge --into`, `--adopt` imports or `branch create`. At 0.59 prikk refuses those for another
branch's queue, and stikk shows the refusal through its classifier.

### F5 — waiting

- **A writer waits up to 5 s per holder.** After 1 s it prints one line on **stderr** (`waiting for the active lock
  held by pid <N> …`). At the bound, it refuses with today's sentence plus `(waited 5 s: …)`.
- **stikk sets no timeout** (none in `stikk-prikk` or `stikk-app`'s worker), so the wait reaches the user as time on
  the Loading card.
- **What stikk does with a successful command's stderr** isn't yet measured. A `waiting …` line on a command that
  then succeeds must not read as a refusal or a warning (Handoff A).

### F6 — a refusal document on a reader stikk checks by schema

Today `verify` and `worktree-status` print nothing on stdout at a refusal. RFC 174 (accepted by prikk's owner, not
built, no version promised) would print `refusal-v1` there. **stikk's readers would call it "an unknown schema
version — upgrade stikk"** (`parse_json.rs`, `report()`), and prikk's message would be lost. stikk already reads
`refusal-v1` for the commands that print it today (`refusal_message`, `refusal_id_and_message`).

## Decisions

### 1. The ceiling, first

**Raise `VALIDATED_MAX_MINOR` to 59 as the first commit,** then install `ceiling-0.59` from prikk's published release,
with its checksum recorded. Then run the whole suite against it.
- **Every failure is a finding,** fixed in its own decision or reported. None is skipped.
- **The ceiling isn't moved back** to make the suite pass.

### 2. The matched strings, systematically

**List every prikk string stikk matches,** from both files, by script. For each, record:
- the prikk source sites at 0.58.0 and at 0.59.0;
- whether the state stikk keys on still prints it at 0.59, from a real run where the suite reaches that state, or
  from source where it doesn't (said which).

**A pattern whose state no longer prints it is fixed, or retired with its reason.**

### 3. Glosses defer to a sentence that names its way out (F3)

**Where prikk's 0.59 sentence names the command that resolves the state, stikk shows prikk's sentence and adds no
gloss of its own.** stikk doesn't paraphrase a sentence that already says what to do (`ER-02`).
- A gloss stays only where prikk's words still leave the reader without a next step. Then it says that step, and
  never contradicts prikk.
- The generic incomplete-publication gloss is re-measured against each 0.59 state that prints the sentence.
- **Below 0.59 the old gloss stays,** because the old sentence still names no command there.

### 4. The switch, merge and adopt refusals (F4)

- **At 0.59 and above, stikk lets prikk refuse the switch,** and shows prikk's refusal: the paths and the seal.
  stikk's own guard stays for prikk below 0.59, where prikk overwrote the files. **One refusal per version, never
  two.**
- **The way out is offered as stikk's own action:**
  - prikk's refusal names `prikk seal --allow-no-audit --ref <branch>`, which is stikk's Seal for that branch;
  - so the refusal card offers Seal, with its usual consent, and then the switch again.

  The card shows prikk's sentence verbatim beside the offer.
- **`merge --into`, `--adopt` imports and `branch create`:** stikk shows prikk's refusal verbatim. Where it names a
  seal, the same offer applies.
- **Tests:**
  - **(suite, at 0.59)** each refusal is reached and classified;
  - **(unit)** the banding: stikk's guard fires below 0.59 and not at 0.59;
  - **(suite, at 0.58)** stikk's guard still refuses, so the files are kept.

### 5. Every JSON reader reads a refusal document first (F6)

**Before a reader checks its own schema, it asks whether stdout is a `refusal-v1`.** If it is, the result is a
refusal carrying prikk's `message` (and its id, for a caller that routes by it), whatever command printed it.
- Then `verify` and `worktree-status` show prikk's words the day RFC 174's change ships, and stikk's draft 025 §2 is
  kept.
- **The order is the one rule:** an unknown schema that isn't a `refusal-v1` is still an environment error, as today.
- **Tests (unit):**
  - every reader, given a `refusal-v1` on stdout, returns the refusal with its message;
  - given an unknown schema that isn't one, still errors.

  Removing the refusal check from one reader must fail its test. **That's the run that fails it.**

### 6. Waiting (F5)

- **stikk keeps prikk's default wait** and doesn't set `PRIKK_LOCK_WAIT_SECONDS`. Waiting for a writer that's still
  running is better for a person than an immediate refusal (stikk's draft 025 §4).
- **The Loading card shows during the wait,** as it does for any slow command.
- **A successful command's `waiting …` line on stderr** is neither a refusal nor a warning in stikk.
  - The refusal at the bound, with its `(waited 5 s: …)` suffix, is classified as today's lock refusal.
  - **Test (suite, at 0.59):** a writer run beside a held lock, with the wait made short through the environment.
    It's both successful (the holder exits within the bound) and refused (it doesn't).

### 7. The format marker, `init` and `setup` (prikk's CHANGELOG)

**stikk runs neither `init` nor `setup` for a user.** Only the suite's fixture builder does, on fresh directories.
- The suite shows whether the builder is affected. If it is, the builder is fixed and the change is said.
- **A lost format marker** reaches the user as prikk's new sentence, which names `prikk format upgrade`. stikk shows
  it verbatim (decision 3).

## What this RFC does not do

- **No new view of prikk's new commands** (`doctor --remove-ref-candidates`): stikk doesn't run `doctor`.
- **No RFC 174 reader** beyond decision 5: nothing in RFC 174 is built.
- **No change to the floor** (0.28), and no work for old bands beyond keeping the switch guard where it protects
  files (no users yet: the ceiling is what's measured).

## Evidence plan

| Question | Level | Where |
|---|---|---|
| What the raise costs | the real-binary suite at `ceiling-0.59` | Handoff A |
| Every matched string, at both tags | a script over stikk's source and prikk's | Handoff A |
| The switch, merge and adopt refusals, and the offer | suite at 0.59 and at 0.58; unit for the band | Handoff A |
| The refusal-first readers | unit, each with its failing run | Handoff A |
| Waiting: success and refusal | suite at 0.59, with a short wait | Handoff A |
| The public API | `cargo public-api diff` against 0.22.1, stated in the release proposal | Handoff A |

## Delivery

**One handoff, A,** after acceptance. It's worked beside RFC 070's handoffs, and the 0.23.0 cut doesn't wait for it.

## Changelog

**For 0.23.0:**
- *stikk now validates prikk 0.59.*
- **At 0.59 and above:** switching to a branch whose queued work the switch would overwrite is refused with prikk's
  own words, and stikk offers to seal that work first.
- **The same for merging and importing** into the current branch.
- A refusal shows prikk's own message on every command.

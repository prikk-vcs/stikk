# RFC 063 — The prikk 0.58 re-baseline

**Status.** **Done 2026-11-21**: Handoff A delivered on `main` (`6f213da`..`a51f7cf`), Handoff B (`4185b1c`..`6487aff`); green on the full platform matrix at both heads; shipped in **0.21.0**. **Clarified in review**, recorded in *Delivered*. **Accepted by the project owner 2026-11-18**; [Handoff A](../handoffs/063-prikk-0-58-rebaseline/a-what-0-56-changed-handoff-v1.md) (decisions 1–5, 9–11) · [Handoff B](../handoffs/063-prikk-0-58-rebaseline/b-what-prikk-now-checks-handoff-v1.md) (decisions 6–8, after A). **Proposed 2026-11-18** by the architect, as 0.21.0's increment 1 (`.git-exclude/release/0-21-0-scope.md`,
amended and authorized by the project owner 2026-11-18). prikk **0.56.0** (2026-11-08), **0.57.0** (2026-11-14) and
**0.58.0** (2026-11-18) are released, in prikk's letters 030 and 031. **No open question.**
**Measured** against the pinned real prikk binaries **0.28.0**, **0.55.0**, **0.57.0** and **0.58.0**. All the evidence
is script output:
- `.git-exclude/reports/rebaseline-0-58/`:
  - `measure-ceiling.sh` → `measure-ceiling-0.55.txt`, `-0.57.txt`, `-0.58.txt`, `suite-ceiling58-0.28-0.58.txt`;
  - `measure-commit-off-current-branch.sh` → `commit-off-current-branch-0.55.txt`, `-0.58.txt`.
- `.git-exclude/reports/063-prikk-0-58-rebaseline/`: `measure063.sh` → `measure063-0.55.txt`, `measure063-0.58.txt`.

**Tracks.** `ASM-2`, `NFR-R03`. RFC 029 B (the current branch on both confirmations), RFC 045 F5 (a switch with work
queued for the target), RFC 047 F3 (a merge into the current branch), RFC 050 (bundles), RFC 054 (the sealer on the
bundle card), RFC 056 (the two-keys card), RFC 028 (the Queue view), RFC 040 (Verify).
**Touches.**
- `stikk-prikk`: `version.rs`, `cli_backend.rs`, `parse_json.rs`.
- `stikk-core`: `commit.rs`, `merge.rs`, `bundle.rs`, `trust.rs`, `glossary.rs`, `present.rs`.
- `stikk-tui`: `app.rs`, `view/bundle.rs`, `view/queue*`, `overlay.rs`.
- `stikk-real-binary`: `support.rs`, `tests/real_binary.rs`.
- The range stated in the READMEs, the docs, the install page's sample output, `ASM-2`, the roadmap and the changelog.

## Summary

prikk 0.56 made the worktree follow the current branch, as stikk's letter 023 asked. That fixes two things stikk
guarded against, and it changes how a commit to another branch behaves. 0.56–0.58 also tell stikk, before an import,
whether a bundle's seals check out and what the import would refuse.

With only the ceiling raised, **8 of the 97 real-binary cases fail at 0.58** (F1). Each failure is understood (F2–F6).
This RFC:
- raises the ceiling to 0.58;
- **lifts** stikk's refusal of a merge into the current branch, from 0.56;
- **keeps** its refusal of a switch with work queued for the target;
- **commits only to prikk's current branch**;
- gives the suite **distinct key ids**;
- adopts four visible gains: the seal outcomes and the refusals on the bundle card, the two-keys card for a removed id,
  an interrupted seal in the Queue view, and the right words for prikk's publication refusals.

## Findings

### F1 — the suite, with only the constant moved

The method: a `git archive` copy of `main` at `9a0c355`, with `VALIDATED_MAX_MINOR` set, and the suite run against the
floor (0.28.0) and that ceiling (`measure-ceiling.sh`).

| Ceiling | Unit tests | Real-binary suite |
|---|---|---|
| 0.55 (today's) | 1443/1443 | 97/97 |
| 0.57 | 1443/1443 | 89/97 |
| 0.58 | 1443/1443 | 89/97: the same 8 as at 0.57, with the same messages apart from the version |

The eight failures:
- `rfc047_a_merge_into_the_current_branch_leaves_the_worktree_behind`;
- `rfc045_the_preview_is_what_the_switch_does_at_the_ceiling`;
- `rfc029b_prikks_current_branch_is_reported_and_named_on_both_confirmations`;
- `rfc050_an_export_refuses_the_two_destinations_and_round_trips_otherwise`;
- `rfc050_an_import_lands_and_an_older_bundle_is_declined`;
- `rfc053_a_received_branchs_plan_is_provisional_at_the_ceiling`;
- `rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import`;
- `rfc056_removing_shrinks_the_list_and_a_removed_id_is_prikks_refusal_at_the_ceiling`.

### F2 — a merge into the current branch now brings the worktree with it

The setup: `heads/feature` adds `c.txt`; the current branch is `heads/main` (`measure063.sh` §B).
- **0.58, clean worktree:** the merge exits 0, prints `materialized into the worktree: files written: 1`, and `c.txt`
  is on disk.
- **0.58, dirty worktree:** it is refused, and nothing moves: *"the worktree is not clean against heads/main:
  readme.txt (modified); commit it, or restore those files, before merging heads/feature into heads/main"*.
- **0.55, dirty worktree:** the merge goes through and leaves the worktree behind. That is RFC 047 F3, which stikk's
  `INTO_THE_CURRENT_BRANCH` guard exists for (`merge.rs:162`, checked in `merge.rs:445-479` and again in the ref
  picker, `app.rs:2117-2124`).

`rfc047` fails because the merged file is now there ("F3 — the merge wrote no file").

### F3 — a switch with work queued for the target still hides that work

The setup: `heads/feature` is made from `heads/main`, so their files match, and the current branch is `heads/main`.
`commit --ref heads/feature` queues the new `queued.txt` for it (`measure063.sh` §C, §F). Then `branch switch
heads/feature`:
- **0.55:** `files deleted: 0`, and `queued.txt` stays.
- **0.58:** `files deleted: 1`, and **`queued.txt` is gone from disk.** `worktree-status` then says the worktree is
  *behind* `heads/feature`, and the next commit is refused. Sealing puts the work into the block, so **nothing is
  lost**, but the user sees their file vanish.

stikk's `WORK_IS_QUEUED_FOR_THIS_BRANCH` (`switch.rs:56-57`, checked at `switch.rs:119-126`) prevents this, and it
**stays**. `rfc045` fails one step earlier, in its setup: at 0.56+, its commit onto `heads/feature`, whose files differ
from the worktree's, is refused (F4). The observation is held for the next letter to prikk
(`hold-for-next-letter.md`).

### F4 — a commit goes only where the worktree's files came from

From 0.56, `commit --ref R` is refused when R's files differ from those the worktree was last written from. It
**succeeds** when they match. Then the *current* branch no longer matches the worktree, and its next commit is refused
(`measure-commit-off-current-branch.sh`).

The setup: the current branch is `heads/dev`, made from `heads/main`. stikk commits the new `note.txt` to `heads/main`
and seals it. The next commit on `heads/dev`:
- **0.55:** exit 0, and **it silently carries `note.txt` into `heads/dev` as well** (`operations: 2`, `create-file
  note.txt`).
- **0.58:** refused: *"refusing to commit: heads/dev is now at Block …, and this worktree was last written from Block
  …; 1 path(s) the worktree holds content for, but it does not match heads/dev's current state …: note.txt; run `prikk
  branch switch heads/main` …"*.

**How a stikk user gets there.** Commit (`C`) and seal (`S`) act on the **focused ref** (`app.rs:1563`, `:1633`,
`require_focus` at `:5488`). Picking another ref in the ref picker focuses it (`app.rs:3190-3209`). From there, `C`
commits the folder's files to that ref, after the notice *"This targets heads/main. prikk's current branch is
heads/dev …"* (`confirm.rs:450-465`). A queue for a branch that is not current can only come from such a commit: prikk refuses a switch away from a
branch with work queued for it, at 0.55 and 0.58 alike (`measure063.sh` §H). **So the flow is one keypress away.** Before 0.56 it mixed two branches' work
silently; from 0.56 it leaves a refusal for later. `rfc029b` fails on that later refusal.

### F5 — imports refuse a seal that fails against an adopted key

From 0.56, an import is refused when a bundle is sealed under a key id the receiver adopted, but by a different key
(`measure063.sh` §D1):
- the import's `refusal-v1` id is `forged-publication-signature`;
- prikk's sentence says *"a forgery, or another maintainer using the same key id"*, and names the fix.

The suite gives every fixture the id `maintainer` (`support.rs:92-93`), so a sender and a receiver built as two
fixtures clash. That breaks `rfc050` ×2 and `rfc053`.

This is **not only a fixture artefact.** prikk 0.45 gave *new* keys distinct default ids, but **existing keys keep the
role word** (prikk's CHANGELOG, 0.45.0). Two people whose keys predate 0.45 meet exactly this refusal. `rfc054`'s
second case builds that clash on purpose (`real_binary.rs:9459`). It asserts verify fails *after* the import, and prikk
now refuses at the import instead.

### F6 — what the bundle preview now says

| | 0.55 | 0.56 | 0.57 | 0.58 |
|---|---|---|---|---|
| `sealed_by_signatures[]`, each `{key_id, outcome}`, outcome `sound` / `unverifiable` / `invalid` | — | yes | yes | yes |
| `import_refusals[]`, each `{id, message, …}`: what `bundle import` would refuse | — | — | partial | **the import's whole first-pass decision** |

These are read from prikk's sources, and measured at 0.58 (§D):
- **D1, the clash:** `outcome: invalid`, with `import_refusals` naming `forged-publication-signature`.
- **D2, the sealer's key not adopted:** `outcome: unverifiable`, no refusals, and the import lands.
- **D3, the sealer's key adopted under its id:** `outcome: sound`, no refusals, and the import lands.

stikk reads `sealed_by` as strings only (`parse_json.rs:1546-1592`), and never reads `import_refusals`. Its card says
*"A bundle names its sealer but does not prove it. Import it, then Verify (V) checks each signature"*
(`NAMES_BUT_DOES_NOT_PROVE`, `trust.rs:198`). **From 0.56 that sentence is false:** the preview does check each seal.

### F7 — the removed-id conflict, and prikk's JSON for it

`trust maintainer add --format json` exists from 0.56 (`measure063.sh` §E; it is an unknown argument at 0.55):
- **On success:** `trust-add-v1`, with `key_id`, `public_key`, `newly_added` and `adopted_count`.
- **On a key-id conflict:** `refusal-v1`, with the id `maintainer-key-id-adopted-with-different-key` or
  `maintainer-key-id-bound-to-removed-key`.

stikk runs `add` without `--format` (`cli_backend.rs:1256-1275`), and catches only the *current* clash itself,
before running it: the `Two keys, one id` card (`trust.rs:549-561`, `overlay.rs:3140-3200`). A **removed** id is not in
the list, so the preview says Ready. prikk then refuses, and the user gets a generic *"prikk refused to adopt this
key"* (`present.rs:595`). `rfc056` fails because prikk's sentence for that case changed, as we asked.

### F8 — a seal that stopped after it published

The state: a `seal` published its block and was killed before it emptied the queue. It is reproduced on disk by
restoring `.prikk/active/default/` after a seal (`measure063.sh` §A). prikk recognises it: `verify` gives the way-out
heading *"a seal stopped after it published, before it cleaned the queue"*.

| | 0.55 | 0.58 |
|---|---|---|
| `status --format json` | **exit 1**, `integrity error: lifecycle replay: …` | exit 0. The queue is listed, with `queue.already_sealed: {"block_id", "route": "prikk seal --allow-no-audit"}` |
| `commit` | refused with the generic *"repository mutation is blocked by incomplete ref publication; run verify/doctor and use signer-backed seal retry"* | refused: *"repository mutation is blocked by incomplete ref publication: a seal stopped after it published, before it cleaned the queue (the queued patches are already sealed in block …); run `prikk seal --allow-no-audit`"* |
| `seal --allow-no-audit` (stikk's seal) | finishes it | finishes it |

stikk's Queue reader ignores `already_sealed` (`parse_json.rs:1113-1140`). At 0.58 it would therefore show these
patches as ordinary queued work, which is *true to the files but false about what Seal will do*: Seal adds no new
block, it finishes the old one.

### F9 — prikk's sentences about publications

stikk's classifier matches none of these (`classify.rs`; no clause for "another command", "interrupted publication",
or "ref complete"), so each reaches the user verbatim as a refusal. Their wording is from prikk 0.58's source:
- **The generic sentence,** `repository mutation is blocked by incomplete ref publication; run verify/doctor and use
  signer-backed seal retry` (`refs.rs:448`). prikk says it can reach a **healthy** repository while another command
  publishes a tag, and asks that it not be shown as damage (031 §4). It also still comes from an interrupted
  publication (`in_flight.rs:146`). *Not reproduced here: prikk saw it on tmpfs beside a `tag create` loop, and
  stikk has no reason to build that race.*
- **The stopped-seal sentence** above (F8). It begins with the same words, then a **colon**, not a semicolon.
- **`another command is publishing <ref> (pid N, from its lock); run this command again when it finishes`**
  (`in_flight.rs:119`), a `Precondition`.
- **`the directory <path> was moved or replaced while this command ran, so nothing was published; run the command
  again, and `prikk verify` to see what it left behind`** (`directory.rs:117`).
- **`the history does not replay cleanly at block <id> (<reason>); run `prikk verify``** (`confirmed_history.rs:129`),
  an integrity error.

### F10 — nothing else 0.56–0.58 changed breaks stikk

Each of these was checked against stikk's code:
- **JSON shapes.** Every change is additive. stikk's JSON readers pick out their fields and ignore the rest (no
  `deny_unknown_fields`).
- **`cat --format json` on a missing path** (stikk's `file_meta`) now also prints `refusal-v1` (`path-not-found`) on
  stdout. stikk classifies stderr, which is unchanged (§G1).
- **`cat --output` keeps the recorded mode** (0.57). stikk's *save a copy* gets it for free.
- **Non-ASCII names in new (format 8) repositories** are `untracked` and `authored` at 0.58, where 0.55 refused them
  (§G2–G3). stikk reads both verdicts (`parse_json.rs:917`). Display width is the carried i18n item.
- **`unlock --yes` on an *unknown* holder** now refuses. stikk never runs `unlock`.
- **`bundle import --adopt`** exits 0 when it does not adopt. stikk never passes `--adopt`.
- **New fields stikk does not read yet:** `worktree-status`'s `behind_ref`, and `doctor`'s debris findings moved to
  `info`. Each is a 0.22.0 candidate. *(This RFC also listed `verify`'s new in-flight finding. prikk's letter 033
  corrected its 031: that id is in `verify`'s and `doctor`'s **text only**, never in `verify --format json`, so there
  is nothing for stikk to read.)*

## Decisions

1. **`VALIDATED_MAX_MINOR = 58`.** Its doc comment says what 0.56–0.58 changed for stikk (F2–F9). CI installs
   `0.58.0`.
2. **Merge into the current branch is offered from 0.56** (F2). Add `MERGE_INTO_CURRENT_FROM_MINOR = 56`. At or above
   it:
   - `blocked_reason` and the ref picker skip `INTO_THE_CURRENT_BRANCH`;
   - prikk's dirty-worktree refusal reaches the user verbatim, which already says what to do;
   - the merge card, when *into* is the current branch, gains one line: *"Your folder is on this branch, so the
     merged files are written into it."*

   Below 0.56 the guard stays, word for word. `WORK_IS_QUEUED_FOR_INTO` stays at every version.
3. **A switch with work queued for the target stays refused at every version** (F3). Its words become:
   > work is queued for this branch; seal it first. A switch now would take those files out of your folder until
   > they are sealed.

   They stay true below 0.56, where the work was absent from the target's sealed files.
4. **Commit goes only to prikk's current branch** (F4), from `CURRENT_BRANCH_FROM_MINOR` (42), where prikk reports
   one. When the focused ref is a branch other than the current branch, `commit_preview` returns a blocked outcome,
   and nothing is spawned. The words:
   > This commits the files in your folder, and your folder is on heads/dev. To commit to heads/main, switch to it
   > first: b, then the branch, then s.

   These are the ref picker's keys: `b` opens it (`NO_FOCUSED_REF_REASON`), and `s` switches (`app.rs:1897`).
   - **The same rule below 0.56,** because there it mixed two branches' work silently (F4, 0.55). That costs no
     user anything: no flow that worked as the user meant is lost.
   - **When the current branch is `Unresolved`, or below 0.42,** the commit is allowed with today's notice.
   - **Seal keeps its notice and stays allowed for any ref.** Seal acts on what is already queued, and sealing is
     prikk's own way out of every state in F3, F4 and F8.
   - **`branch_notice` keeps serving seal.** For commit it is now reached only when the current branch is
     `Unresolved`.
5. **The suite's fixtures get distinct key ids** (F5). `Fixture::build` gives each fixture its own author and maintainer
   id, for example the role word plus a per-process counter. One new constructor builds a fixture under a **given**
   maintainer id, so the clash is written in the test that wants it.
   - **`rfc054`'s clash case** builds its receiver under the sender's id. **At ≥ 0.56** it asserts that the import is
     refused and nothing lands. **At ≥ 0.58** it also asserts that the bundle card says, before the import, that it
     would be refused (decision 6). Below 0.56 it keeps today's assertion.
   - **`rfc050` ×2 and `rfc053`** need no change beyond the distinct ids. Each must pass at 0.58 unchanged.
6. **The bundle card reads what prikk checked** (F6).
   - **From 0.56, `sealed_by_signatures`.** Each sealer row shows prikk's outcome in words, and stikk's own name match
     is no longer the judgement:
     - `sound`: *"signed by a key this repository trusts, and the signature checks out"*;
     - `unverifiable`: *"this repository trusts no key under this id, so the signature cannot be checked here"*;
     - `invalid`: a **security warning** (decision 8).

     `NAMES_BUT_DOES_NOT_PROVE` stays only below 0.56. An unknown outcome is a shape error, as everywhere in the
     parser.
   - **From 0.58, `import_refusals`.** When it is non-empty, the card says *"prikk would refuse this import"*, lists
     each refusal's `message` verbatim, **and does not offer Import.** Each refusal is keyed by its **`id`**, never by
     its words. Below 0.58 the list is not read, because at 0.57 it was partial.
   - prikk's note on the preview stands: a second pass, or a writer in between, can still refuse. So the import's own
     refusal path stays as it is.
7. **`trust maintainer add` reads JSON from 0.56** (F7).
   - It passes `--format json`, and reads `trust-add-v1` on success.
   - On failure it reads stdout's `refusal-v1` `refusal` **id**, in the way `extract_report` already does
     (`cli_backend.rs:209-227`).
   - Both key-id conflicts, `…-adopted-with-different-key` and `…-bound-to-removed-key`, open the existing `Two keys,
     one id` card, with prikk's message as its detail. For a removed id, one added line: *"This id was removed here,
     but stays bound to the key it was first adopted with."*
   - Below 0.56: prose, as today.
8. **"Security warning" means the `Two keys, one id` card's form.** Its title, heading, rows and Esc-only actions are
   used for both decision 6's `invalid` seal and decision 7. No new overlay type: the existing card gains a caller,
   not a twin. For the `invalid` seal, the heading is *"A seal in this bundle does not match the key you trust under
   its id"*, and the body is:
   > it may be forged, or another person signs under the same id. Do not import it until you have asked the sender.
   > If it is theirs, they must sign under a distinct id, and you adopt that one.
9. **An interrupted seal shows as what it is** (F8). From 0.58, the Queue reader reads `queue.already_sealed` as
   `Option<block id>`. It must be present, and null is a value; the `route` is not read, since stikk's Seal is that
   route.
   - **The Queue view** heads its list: *"These changes are already sealed, in block <short id>. A seal was
     interrupted before it finished tidying up. Seal (S) finishes it, and adds no new block."*
   - **The seal confirmation** says the same in one line.
   - **Below 0.58,** the exit-1 refusal stays as it is.
10. **The right words for the publication sentences** (F9). These are glosses, keyed in `glossary.rs` as the existing
    entries are:
    - **The generic sentence, matched with its semicolon:** *"This can happen while another prikk command is
      publishing, and it is not damage. Try again. If it keeps happening, open Verify (V)."*
    - **The stopped-seal sentence (colon):** *"A seal was interrupted after it published. Seal (S) finishes it."*
    - **`another command is publishing`:** *"Another prikk command is publishing right now. Nothing was changed. Try
      again when it finishes."*
    - **The moved-directory and history-replay sentences:** no gloss. Each already names its next step, and a unit
      test pins that each degrades to a verbatim refusal under the classifier.
11. **The range is stated as `0.58.0`** everywhere it now says `0.55.0`:
    - `README.md` and `crates/stikk/README.md`;
    - `docs/src/index.md`, `docs/src/guide/getting-started.md`, and the install page's sample output;
    - `ASM-2`, appended in its shape;
    - the roadmap's *Releases and versioning*.

    The roadmap's 0.20.0 section keeps 0.55.0, which is what 0.20.0 shipped. `docs/src/contributing/releasing.md` is
    checked, and changed only where it states the range rather than an example.

## Tests and evidence plan

| Question | Level | Where |
|---|---|---|
| Each parser reads the new field or document, and refuses a wrong shape | unit | `parse_json` tests, from the 0.58 captures in `measure063-0.58.txt` |
| Each version gate (42, 56, 58) picks the right path | unit | `commit`, `merge`, `bundle`, `trust` and `queue` tests, at 0.55, 0.56, 0.57 and 0.58 handshakes |
| Each card and line reads as decided, and fits | headless (`TestBackend`) | the bundle card (three outcomes, refusals), the two-keys card (two new callers), the Queue view's head, at 80 × 24 and the wide size, dark and light |
| prikk does what F2–F8 measured, through stikk's own calls | real binary, floor + ceiling | the eight cases above, rewritten per decisions 2–9. New legs: merge into the current branch at ≥ 0.56 (clean writes the file; dirty is prikk's refusal); commit to a non-current branch is blocked and spawns nothing; the stopped-seal state, built as `measure063.sh` §A does, seen by the Queue reader and finished by stikk's seal; the bundle card's three outcomes; `add` of a removed id opens the two-keys card |
| The eight failing cases, rewritten | real binary | `rfc045` step 4 queues its work for a branch whose files match, as `measure063.sh` §C does, so the switch refusal is what is tested. `rfc029b` asserts the blocked commit, and measures the no-`--ref` default without a stikk commit elsewhere first. `rfc056` reads prikk's new sentence in Handoff A, and the two-keys card in Handoff B |
| Nothing else regressed on macOS and Windows | real binary, full matrix | CI after the push. This RFC changes code the suite exercises |

**The stopped-seal leg restores `.prikk/active/default/`**, which is prikk's internal layout. The leg must first assert
that prikk's own `status` reports `already_sealed`. If the layout moves, the leg then fails as a harness failure, never
as a pass.

No real-terminal capture is needed: the visible changes are text and existing card forms, and the headless level
proves them.

## Delivery

Two handoffs, issued on acceptance:
- **A, the ceiling: what 0.56 changed** (decisions 1–5, 9–11). The suite is green at 0.28.0 and 0.58.0 at its end.
  Commit 1 moves only the constant and the fixture ids. The behaviour changes follow, each with its legs.
- **B, what prikk now checks for you** (decisions 6–8). This is the bundle card and the two-keys card. It starts after
  A is approved.

## Changelog

- `### Changed`: validated through prikk 0.58.0; a commit goes to prikk's current branch, and offers a switch for
  another; merging into the branch you are on is offered from prikk 0.56.
- `### Added`: the bundle card shows whether each seal checks out, and what an import would refuse; an interrupted
  seal is shown as one; the two-keys warning for a removed id.

## Out of scope (0.22.0 candidates)

- one-step going back (`branch create --from <block id>`);
- counting what removing a key breaks (`block_seals`);
- `parent_block_ids` for undo, and undoing a merge;
- `uncommit`;
- `bundle preview --adopt`;
- `worktree-status`'s `behind_ref`.

## Delivered

**Two handoffs, thirteen commits, green on `main`.** At both heads, every run passed: CI, Docs, supply chain, and the
real-binary suite on the full platform matrix.
- **Handoff A:** `6f213da`..`a51f7cf`, 99 cases on Linux, macOS and Windows at 0.28.0 + 0.58.0. Full-matrix run
  `44428306851`.
- **Handoff B:** `4185b1c`..`6487aff`, 100 cases. Full-matrix run `44839614689`.

The eight failures of F1 were each fixed by the decision that explains them:
- `rfc050` ×2, `rfc053`, `rfc054` and `rfc056`, by the per-fixture key ids;
- `rfc045`, by queueing onto a branch whose files match;
- `rfc047`, by decision 2. It is split in two, and the below-0.56 half announces that it is skipped with the pinned
  pair;
- `rfc029b`, by decision 4.

New legs: the interrupted seal, read as one and finished by stikk's Seal; and the three ways a receiver stands
towards a sender's key.

**Clarified in review, as delivered:**
- **Decision 2.** For a merge that writes this worktree, the card's *"Nothing is written to the worktree"* clause is
  **replaced** by `MERGE_WRITES_THIS_WORKTREE`, not joined to it.
- **Decision 4.** At 0.58 the no-`--ref` default can only be measured **before** anything is committed to another
  branch, so `rfc029b` measures it first. Seal's notice is measured on a queue built by a raw commit to a branch
  whose files match.
- **Decision 6, order.** prikk's `import_refusals` are read **after** stikk's own guards. prikk's list is wider than
  F5's case (an older bundle comes back as `rollback`), and stikk keeps its own sentence for what it already
  declines. The bundle card itself still lists prikk's refusals.
- **Decision 6, the trust list.** **prikk's seal outcomes do not depend on stikk reading the trust list** (Handoff B
  review v1 D1). A list stikk cannot read costs only the key shown beside a warning. The warning then leaves out its
  `trusted here` row, and says in its body that the key could not be read.
- **Decision 7.** A **removed** id's conflict can only come from prikk, at execute (F7). It reaches the same card
  through the write's result, not through the preview.
- **Decision 8.** The warning is one type, `SecurityWarning`, rendered by the one card, with three callers.

**Carried, small:** `parse_json.rs`'s Orientation reader's doc says a missing key is an error, but
`heads_main_ref_state` is read as optional. Fix it at the next change of that reader.

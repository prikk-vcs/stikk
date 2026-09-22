# RFC 034 — The prikk 0.46 re-baseline: a ref with no published history stopped being readable

**Status.** **Done 2026-09-22** — Handoffs A and B delivered on `main` (`ad95152`, `8a23045`); a **0.9.0 candidate**.
**Accepted by the project owner 2026-09-22**, **Q1 ruled (b)** the same day: no 0.8.1 — stikk has no
users yet, so the repair rides in 0.9.0 with the re-baseline. **Amended on acceptance (A1)** — the ceiling raise comes
*first*, not last. **Proposed 2026-09-22** by the architect, the day prikk's letter 017 arrived. **Supersedes
[RFC 033](../archive/033-prikk-0-43-rebaseline.md)**, whose prikk is two releases old and whose open question prikk
answered in 0.44.0.
Measured against real prikk **0.28.0**, **0.43.0**, **0.44.0**, **0.45.0** and **0.46.0** binaries
(`cargo install --locked`), and by running the **published 0.8.0's own suite** — a scratch copy of `c25339f` with
only the validated ceiling raised — at 0.28 and 0.46. Evidence:
`.git-exclude/reports/034-prikk-0-46-rebaseline/`.
**Tracks.** `ASM-2`, `NFR-R03`, `FR-034`, `FR-050`, `FR-030`, `FR-033`, `UD-09`, `UD-10`, `C-T2b`, `ER-02`, `T-T4`,
RFC 027 (Q1 (b)), RFC 029, RFC 030, RFC 032, stikk letter 014, prikk letter 017.
**Touches.** `stikk-prikk` (the ceiling; the ref argument of `worktree_status` and `history`; the refusal
classifier), `stikk-core` (the declaration analysis; commit's preview; Orientation), `stikk-tui`,
`stikk-real-binary`, the captured fixtures, and on delivery `requirements.md`.

## Summary

**prikk 0.45.0 broke something stikk 0.8.0 shipped a week ago — the path a repository's first commit takes.**
**stikk has no users yet** (the owner, 2026-09-22), so this is a correctness and scheduling matter, not an incident.

On prikk ≥ 0.45, `worktree-status --ref <R>` and `log --ref <R>` **refuse** when `R` has no published history —
*"precondition not met: ref heads/main does not exist in this repository"*. Until a repository's first **seal**, that is
every ref. So the Changes view, History **and commit's preview** all refuse on a brand-new repository. **RFC 032's
"a ref with no published history" — the increment 0.8.0 led with — cannot run at all there.**

prikk changed this deliberately, as *"`log` and `worktree-status` exit 1 for an explicitly named absent ref"*. **An
unpublished ref is not an absent ref**, and prikk's own default path still agrees: **without `--ref`, both commands
still report.** That is both the bug report (letter 015) and the workaround.

The rest of the news is good. **prikk fixed what letter 014 reported** (F5), and **0.46's `tree`, `cat` and `diff`,
with a bare block id accepted everywhere read-only, close the last dependency under Patch detail and Compare** (F6)
— the two roadmap items that have been blocked longest.

## Findings

### F0 — what the published 0.8.0 does on prikk 0.46

The 0.8.0 suite, ceiling raised only: **26 of 32 pass; 6 fail.** Four are one regression; two are tests pinning
prikk's old wording.

| Failing test | Why | Kind |
|---|---|---|
| `rfc032_a_ref_with_no_published_history_is_named_and_its_first_commit_is_not_stale` | `changes_view` refuses | **regression** |
| `rfc029b_an_unpublished_heads_main_reads_as_an_empty_history_at_both_ends` | History refuses | **regression** |
| `rfc030_a_file_added_between_preview_and_confirmation_is_stale_then_a_fresh_preview_commits` | `commit_preview` refuses | **regression** |
| `queued_elsewhere_arrives_as_prikks_note_below_0_39_and_as_its_ref_above` | `worktree_status --ref heads/other` refuses | **regression** |
| `rfc032_a_declaration_whose_source_is_back_…` | pins prikk 0.42's refusal text | test only |
| `rfc032_a_declaration_without_its_destination_…` | pins *"destination is ignored"*; 0.44 says *"destination is a directory"* — the fix stikk asked for | test only |

**A repository with published history is unaffected**, which is why 26 pass.

### F1 — it entered in 0.45.0, and only the explicitly named ref is refused

One fresh repository, one untracked file, `heads/main` never published:

| prikk | `worktree-status --ref heads/main` | `log --ref heads/main` |
|---|---|---|
| 0.43.0 | the full report | an empty history, exit 0 |
| **0.44.0** | **the full report** | — |
| **0.45.0** | **refuses**, exit 1 | — |
| 0.46.0 | refuses, exit 1 | refuses, exit 1 |

**And at 0.46, with no `--ref` at all, both still report** — `worktree-status` lists the untracked file, `log` returns
zero blocks, each naming `"ref": "heads/main"` beside `"current_branch": "heads/main"`. **`prikk commit
--ref heads/main` still queues a first patch**, and everything reads normally again after the first `seal`.

So prikk's own default disagrees with prikk's explicit form about whether that ref exists.

### F2 — the workaround is verifiable, not a guess

The ref-less report **names the ref it used**. So stikk can drop `--ref` only when prikk's `current_branch` equals the
ref it wants, and then **confirm from the report itself** that it got that ref. If the returned ref differs, stikk has
read something it did not ask for and must say so rather than show it (`T-T4`).

**It is not a general substitute:** an unpublished ref that is *not* prikk's current branch stays unreadable on
≥ 0.45, and prikk offers no way to ask for it. stikk says so plainly rather than showing an empty view.

### F3 — two refusals stikk classifies by text changed, and one new one arrived

`is_environment` matches `no such file`, `permission denied`, and `uses format` + `no longer supports`.

| State | prikk 0.43 | prikk 0.46 | stikk's class |
|---|---|---|---|
| a directory holding no repository | `i/o error: No such file or directory` | **`precondition not met: no prikk repository at <path>`** | **no longer environment** |
| a format-7 repository read by prikk 0.43 | — | **`unsupported format version: 0`** | **not environment** |

Both now fall through to an ordinary refusal. **stikk still shows prikk's words** (`ER-02`), so nothing is invented —
but the class drives stikk's own next steps, and "point stikk at a repository" and "your prikk is too old for this
repository" are exactly the two an environment class exists to give.

**`unsupported format version: 0` names 0, not 7.** Letter 015 reports that too.

### F4 — repository format 7 is a pairing hazard, and stikk never causes it

- **A repository created by prikk ≥ 0.45 is format 7 from birth** — measured: `format upgrade` on a fresh 0.46
  repository answers *"already 7; nothing changed"*.
- **prikk 0.44 and older cannot read it at all**, and there is no downgrade.
- **stikk never upgrades anything** — it writes nothing inside a repository (`CON-1`) and runs no `format` verb. The
  hazard reaches stikk only as F3's refusal, when a user's `STIKK_PRIKK_BIN` is older than the repository.

### F5 — letter 014 is answered, and one claim I cannot confirm

- **The directory destination is fixed in 0.44.0**, exactly as reported: it resolves **`deletion`**, and `commit` says
  *"destination is a directory; recorded as a deletion, not a rename"*. **RFC 033's Q1 is moot** — prikk removed the
  disagreement rather than stikk choosing a side.
- **`content_changed` and `mode_changed` are `null` when the destination is not a regular file**: measured on a FIFO
  destination at 0.46, which also reports `refused_count: 1` with `resolution: "rename"` — F3 of RFC 033 stands.
- **prikk says 0.43.0 hung forever on a FIFO destination. I could not reproduce that**: 0.43 exited promptly under a
  10-second timeout in my repository. Recorded as prikk's claim, not as stikk's measurement.

### F6 — 0.46 closes the last dependency under Patch detail and Compare

Measured at 0.46, in one repository:

- **`prikk tree --ref <ref|block-id> --format json`** (`tree-listing-v1`) lists present leaf paths with `mode`,
  `size`, `encoding`, and `content_id` on binary entries. **A bare block id resolves**: `tree --ref <block-id>`
  returned the same entries as the ref. A genuinely non-UTF-8 file is `encoding: "binary"` **with** a `content_id`;
  a file holding NUL bytes that is valid UTF-8 is `text`.
- **`prikk cat --path <p> --ref <ref|block-id>`** returns a file's bytes at a point.
- **`prikk diff --format json`** (`diff-report-v1`) compares two points, or — bare — the current branch against the
  worktree. An entry carries `path`, `status`, `from`/`to` metadata, `minimal`, and unified `hunks`:
  `{"path": "a.txt", "status": "modified", "minimal": true, "hunks": ["@@ -1,2 +1,3 @@\n one\n two\n+three\n"]}`.

**`UD-10` retires** — a block id is addressable as a content root, which was the stated blocker on `FR-033` Compare.
**`FR-030` Patch detail and `FR-033` Compare are unblocked**, and `diff` also answers `UD-09`'s per-file content half
for the Changes view. **Each is its own RFC, not this one.**

### F7 — what RFC 033 measured that still holds

Carried forward, re-verified at 0.46 where the text changed: the **interrupted-materialization marker** in `status`
prose, JSON and `doctor`, with `commit` refusing it as a precondition; **prevention on a refused declaration**
(`refused_declaration_count`); **content and mode per rename**; and **the destination-absent sentence that gives a
reason stikk cannot know** (RFC 033 F5 — *"{new} is not a file in the worktree"* is false when the destination is
ignored).

## Decisions

1. **The unpublished-ref read is repaired before the ceiling moves.** At any prikk, when the target ref is absent from
   `refs()`:
   - if prikk's `current_branch` equals it, stikk reads `worktree-status` and `log` **without `--ref`**, and **accepts
     the report only if it names that ref** (F2);
   - otherwise stikk says the ref has no published history and that this prikk will not report it — **never an empty
     view** (`C-T2c′`).
   This restores RFC 032's words, RFC 029's empty history, and commit's preview for a first commit.

2. **The environment class is matched on what prikk says now, as well as what it said before** (F3): the
   no-repository answer and a format refusal from a too-old binary classify as environment, on their semantic clauses
   — `no prikk repository at`, and `unsupported format version` — never on the class prefix, which is the rule that
   survived every previous sweep.

3. **The ceiling rises to 46 first** — see amendment A1 — and the two wording-pinned tests become version-aware.

4. **RFC 033's carried work lands with this re-baseline** (F7), re-measured at 0.46: the marker, prevention on a
   refused declaration, content and mode per rename, and the corrected destination sentence.

5. **Patch detail, Compare and the per-file diff get their own RFCs** (F6). This one measures the ceiling and repairs
   what broke; it builds no view.

6. **Letter 015 goes to prikk**: an unpublished ref is not an absent one, with F1's table and the default-path
   inconsistency; and `unsupported format version: 0` naming the wrong number.

## Open question

### Q1 — does the repair ship as 0.8.1 now, or wait for 0.9.0?

**The facts.** prikk 0.46 is current on crates.io, so a new user installing both today gets a refusal on their first
Changes view, History and commit preview, until their first seal. **No data is at risk** — prikk's refusal is shown
verbatim, nothing is written, and a repository with published history is unaffected.

- **(a) An 0.8.1 patch now, decisions 1 and 2 only — recommended.** Smallest possible change to the released line,
  no new features, no ceiling move. It puts first-run back within days rather than weeks.
- **(b) Fold it into 0.9.0** with the whole re-baseline. One release instead of two. *Cost:* first-run stays broken on
  the current prikk for as long as 0.9.0 takes, and 0.9.0 now has Patch detail and Compare in front of it.
- **(c) Wait for prikk to restore the behaviour.** prikk has moved fast for us, and this is their regression. *Cost:*
  stikk's shipped release stays broken on a prikk it does not control, for a fix that is ours to make in one place.

**My recommendation was (a), and letter 015 goes regardless.**

### RULED by the project owner, 2026-09-22: (b)

**"No need to care about stikk user because no actual user yet."** That removes (a)'s only argument — first-run
urgency — so **there is no 0.8.1**, and decisions 1 and 2 ride in 0.9.0 with the rest of the re-baseline.

**The repair is still built, rather than waiting for prikk** (which would be (c)). stikk supports prikk from **0.28**
upward, so **0.45 and 0.46 stay inside that range whatever prikk does next**, and anyone pinning either one meets this
for as long as stikk supports them. If prikk restores the behaviour, the workaround narrows to a two-release band
instead of being permanent, and it retires entirely if stikk's floor ever rises above 0.46.

**What the ruling changes about order:** nothing is urgent, but nothing moves without it either — **the ceiling cannot
rise to 0.46 while four suite cases fail**, and Patch detail and Compare both need 0.46's verbs. So this RFC still
comes before them.

## Amendment A1 — 2026-09-22, on acceptance: the ceiling raise comes first

**Decision 3 as proposed had the order backwards**, and it would have left Handoff A unable to measure its own repair.

`VALIDATED_MAX_MINOR`'s own documentation states the rule: *"The raise comes early in a re-baseline, not last. The
real-binary suite refuses to run against a binary this constant does not name, so raising it is what lets the suite
report what the raise cost."* RFC 029 did exactly that (`9b0a6e4`, the ceiling alone).

**So Handoff A raises the ceiling to 46 in its own first commit**, with the suite then red on the four regressions of
F0 — *expected, and reported in the review request rather than hidden* — and repairs them in the commits that follow.
**The raise is not the re-baseline**: Handoff B still carries decision 4's work, re-measured at 0.46.

## Delivered

**Two handoffs, in order, both green on `main`.**

**A — the repair and the ceiling** (`5df4b68` the ceiling alone, `0793297`, `7484431`, `a938cb2`, `ad95152`).
- **The ceiling is 46**, raised first, with the suite red at that commit on F0's four regressions and the redness
  reported rather than hidden.
- **A ref with no published history reads again.** On prikk's own clause, stikk re-runs the command without `--ref`
  and **accepts the report only when it names the ref that was asked for**; every other outcome returns prikk's
  original refusal. Nothing consults `refs()` or `current_branch`, and **an absent ref still refuses** — driven by a
  test asserting the current branch's name never leaks into another ref's answer.
- **The environment class** gained prikk's no-repository and too-old-binary answers, matched on their clauses, with
  **no format number parsed** — a second capture at `version: 9` stops that dependency forming.
- **The two wording pins are version-aware**: the source-present refusal was reworded at **0.43**, and the directory
  destination at **0.44**.
- **Runs:** CI `35709723018`, the suite `35709723045` (full matrix), supply chain `35709726600`; on `main`, CI
  `35710637853` and Docs `35710637900`.

**B — the re-baseline, on prikk's own verdict** (`f45c24a`, `f6ca0af`, `3ab8fca`, `ef25f68`, `31d5aae`).
- **Declarations come from prikk's `resolution` at ≥ 0.44** — `rename`, `deletion`, `deletion-ignored`,
  `never-tracked`, `refused` — each with its own measured sentence, and RFC 032's inference below the band. **The band
  is justified by a captured 0.43 report read at both 43 and 44**, resolving `rename` then `deletion`.
- **A rename's content and mode come from prikk**, and RFC 032's *"prikk does not report whether its content also
  changed"* is **asserted absent** inside the band. `null` renders nothing: unknown is not "unchanged".
- **A declaration prikk would refuse makes commit unavailable**, with prikk's refusal verbatim and **no way out in
  stikk's words** — checked **before** the clean-worktree block, which is what keeps prikk's verdict visible on the
  row prikk calls clean.
- **A checkout that stopped part-way is shown** in Orientation, from prikk's prose sentence (`ER-02`), and blocks
  commit's preview — **while Changes and History keep working**, asserted at real binaries.
- **37 suite cases green** at 0.28 and 0.46, and by hand at 0.44 and 0.45.
- **Runs:** CI `35716937849`, the suite `35716937497` (full matrix, all three platforms), supply chain
  `35716940461`; on `main`, CI `35718339874` and Docs `35718339861`.

### Carried forward

- **The `--ref` retry exists only because prikk 0.45 refuses a named unpublished ref.** Letter 015 asks prikk to
  restore it; the retry retires when prikk does and stikk's floor passes that release.
- **An unpublished ref that is not prikk's current branch stays unreadable** at ≥ 0.45, and stikk says so rather than
  showing an empty view. **Narrowed 2026-09-22 by prikk's reply 018:** they measured that `branch create` *also*
  refuses in that state, so a second unpublished ref **cannot be created before the first seal** — the state is
  unreachable, and the retry covers every unpublished ref that can exist today. **prikk's fix is on their `main`**
  (`6dee6c4f`, `7405ec3b`) and ships in their next release, where a named current branch reads published or not;
  it was also **wider than stikk measured** — `tree` and `diff` refused the same way, neither of which stikk drives
  yet.
- **prikk 0.43 is a release stikk does not build declaration behaviour on** — its classifier is wrong in one measured
  state, and prikk reports it can hang on a FIFO destination.
- **Orientation is prose-parsed.** Reading the marker from `status --format json` instead is its own increment
  (roadmap), and nothing waits on it.
- **The mode row skips where a platform has no executable bit**, announced; the content row beside it runs
  everywhere.
- **No gloss for prikk 0.42's interrupted-materialization refusal** (A2).
- **prikk 0.46's `tree`, `cat` and `diff` unblock `FR-030` and `FR-033`, and `UD-10` retires** — each its own RFC,
  deliberately not built here.

## Amendment A2 — 2026-09-22, from Handoff B's review: the 0.42 gloss rested on a stale premise

**F7 and Handoff A's §7.4 said stikk *"presents [prikk 0.42's interrupted-materialization refusal] as an integrity
finding, though nothing is corrupt"*. That has not been true since RFC 017.**

`is_integrity_finding` was **removed** there — *"removed rather than kept on a guess"* — and **nothing in the seam
constructs `StikkError::IntegrityFinding` from a prikk message any more**. `classify` returns a plain `Refusal`
carrying prikk's words verbatim, which is what a user sees today.

**The premise came from a roadmap note written before RFC 017 landed**, and the architect carried it into RFC 033's
F7, then into this RFC, then into Handoff A without re-checking it. **The dev team measured prikk 0.42's refusal,
found it already names its own way out, and declined to write the gloss rather than inventing one.** That was right.

**Ruled: no gloss is built.** *"Nothing is corrupt"* is a claim about prikk's data that stikk has no standing to make,
and prikk's own sentence is both accurate and actionable. **Decision 4 loses its 0.42 clause; nothing else changes.**

## Delivery

- **Handoff A — the repair**, issued on acceptance: decisions 1 and 2, with the suite legs that pin them at 0.44,
  0.45 and 0.46. **Not a patch release** (Q1 ruled (b)); it lands on `main` for 0.9.0.
- **Handoff B — the re-baseline**, after A: decisions 3 and 4.
- Patch detail, Compare and the per-file diff follow as their own RFCs (decision 5).

## What this RFC does not do

- **No new view**, and no Patch detail, Compare or per-file diff.
- **No `format`, `bundle`, `sync` or `checkout` verb** in stikk.
- **No claim that prikk 0.43's FIFO hang exists** — prikk states it; stikk did not reproduce it (F5).
- **No reading of an unpublished ref that is not prikk's current branch** — prikk offers no way, and stikk says so
  rather than showing an empty view.

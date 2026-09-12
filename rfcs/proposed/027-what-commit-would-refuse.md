# RFC 027 — What commit would refuse: prikk's verdict, and the unsupported paths stikk never listed

**Status.** **Proposed 2026-09-13** by the architect, the first increment of 0.7.0. One open question
(Q1). Every finding below was measured against real prikk **0.28.0** and **0.41.0** binaries built from
their tags the same day, not read from prikk's changelog or letters.
**Tracks.** `FR-034`, `FR-050`, `UD-06`, `UD-02`, `C-T4d`, `C-T2b`, `C-T2c′`, `ER-02`, `ASM-2`, and
RFC 014 decisions 1 and 5b.
**Touches.** `stikk-prikk` (the `worktree-status` readers, `WorktreeStatus`, `WorktreeEntry`),
`stikk-core` (`changes`, the commit preview), `stikk-tui` (the Changes view), `stikk-real-binary`,
`requirements.md`.

## Summary

Two things, found while measuring one.

**First, the question 0.7.0 opened with.** Since 0.39, prikk reports for every worktree entry whether
`commit` would author it or refuse it, with the reason `commit` would print — computed by the same
classifier `commit` uses. And **one refused entry refuses the whole commit**. So at ≥ 0.39 commit's
preview can know, before arming anything, that the commit will fail. That is exactly the condition under
which RFC 014 made a clean worktree *unavailable-with-a-reason* rather than offered-then-refused.
**`FR-050`'s carried question gets decided here.**

**Second, a defect in every release stikk has made.** prikk names an unrepresentable path
`unsupported-path`, and has since at least the 0.28 floor. stikk matches the word `unsupported`, and its
parser discards any line whose first word it does not recognize. **Since 0.1.0, the Changes view has
counted unsupported paths in its header and listed none of them** — and those paths block commit too.

## Findings

### F0 — stikk has never shown an unsupported path

**prikk's word.** `UnsupportedPath => "unsupported-path"` at every tag checked — 0.28.0, 0.33.0, 0.36.0,
0.38.0, 0.39.0 — and measured in the prose output of both built binaries:

```
unsupported paths: 1
worktree: changed against baseline
  unsupported-path <…>/back\slash.txt — worktree path is not representable as a safe Prikk path: invalid name: …
```

**stikk's word.** `WORKTREE_KINDS = ["modified", "missing", "untracked", "unsupported"]`
(`cli_backend/parse.rs:35`), and `parse_worktree_entry` returns `None` for any line whose first word is
not in that list — deliberately, its doc says, so a wrapped note is not mistaken for an entry.
`ChangeKind::from_label` in `stikk-core` also maps `"unsupported"`.

**The result.** Measured at 0.41 with a backslash name and a non-UTF-8 name: prikk reports
`unsupported paths: 2` and two `unsupported-path` entries. stikk keeps the count and drops both entries.
The header says `unsupported 2`; the list below it is empty.

**Two documented guarantees have never been reachable.** `WorktreeEntry::kind` says the kind is *"kept
as text so a future kind renders rather than breaks parsing"*, and `ChangeKind::Other` *"preserves any
future kind rather than dropping it"*. The parser drops an unrecognized kind before either can see it.
The unit test at `changes/tests.rs:91` proves `from_label("typechange")` — a call the parser can never
make.

**Why nothing caught it.** Every captured `worktree-status` fixture in `parse/tests.rs` says
`unsupported paths: 0`. A fixture set that never contains a shape never tests it; grep is a floor.

**And those paths block commit.** At 0.41 the commit fails with
`invalid name: backslashes are not allowed in repository paths` or
`invalid name: worktree path is not valid UTF-8: …`; at 0.28, the same backslash message.

**One more thing about them**, for upstream rather than for stikk: an `unsupported-path` entry carries
an **absolute filesystem path**, in prose and in JSON, at both 0.28 and 0.41 — where every other kind is
repo-relative. stikk renders it inert, but it is a user's home directory on a screen that otherwise shows
repository paths.

### F1 — prikk ≥ 0.39 reports commit's verdict per entry, from commit's own classifier

prikk 0.39's changelog: every change entry carries an authoring verdict. Prose gains `refused paths: N`
and a trailing `[refused: <reason>]`; `--format json` gains `refused_count` and, per change,
`"authoring": "authored" | "refused"` with `"refusal": null` or the reason — additive within
`worktree-status-report-v1`. *"The verdict is not a second opinion. `commit`'s own refusal paths and
`worktree-status` call one shared classifier."*

**Measured at 0.41, and it holds.** An untracked symlink beside an ordinary modification:

```json
{"path": "b.txt",    "kind": "modified",  "authoring": "authored", "refusal": null}
{"path": "link.txt", "kind": "untracked", "authoring": "refused",
 "refusal": "precondition not met: link.txt: worktree symlink authoring is out of scope"}
```

`prikk commit` then prints **exactly that string**.

**`refused` is orthogonal to `kind`**, as prikk says: a tracked file replaced by a symlink is `modified`
*and* refused, and so is a dangling one; an untracked FIFO is `untracked` and refused with
`invalid name: pipe0: worktree entry is not a regular file`.

**The reasons do not share a class word** — `precondition not met:` for a symlink, `invalid name:` for a
FIFO. stikk carries them verbatim and never classifies them.

### F2 — one refused entry refuses the whole commit

The same worktree — `b.txt` authorable, `link.txt` refused — commits nothing: exit 1, no patch recorded,
the error is the refusal. Commit is whole-worktree (`UD-06`); there is no partial commit to fall back to.

**So at ≥ 0.39 stikk knows before arming that commit will fail**, from prikk's own verdict. That is RFC
014 decision 5b's condition — *"stikk knows before arming anything; `C-T4d` requires the affordance be
visibly disabled with its reason rather than failing on use"* — which today covers only a clean worktree.

### F3 — `.prikkignore` is a real way out, and it is itself committed

With `link.txt` listed in `.prikkignore`: `refused_count 0`, and the commit succeeds. **But
`.prikkignore` appears as an `untracked`, authored entry — it is part of that commit.** A next step that
suggests it must say so; `C-T2b` requires stikk-authored next steps to be true, and "ignore it" without
that clause would not be.

### F4 — below 0.39 there is no verdict, and 0.28's refusal reads differently

At 0.28: no `refused paths:` line, no suffix, and no `--format json` at all
(`error: unknown worktree-status argument: --format`). The same symlink refuses the commit with
`integrity error: worktree authoring: unsupported symlink authoring: link.txt: …` — the `integrity error:`
class that 0.39 reclassified to `precondition not met:`.

`worktree-status-report-v1` exists from **0.38** (with rename declarations); the verdict fields from
**0.39**. **stikk's JSON gate is already ≥ 0.39** (`CliBackend::reads_json`), so the verdict arrives
under the one number stikk already has.

**Below 0.39, "no entry is refused" is not knowable, only unreported.** Rendering it as zero would be
`C-T2c′`'s failure a fourth time.

### F5 — prikk's verdict says "authored" for paths commit refuses

At 0.41, every `unsupported-path` entry measured — a backslash name, a non-UTF-8 name, one inside a
subdirectory — is `"authoring": "authored"`, `"refusal": null`, with `refused_count: 0`. **And `commit`
refuses each one.**

prikk closed this exact disagreement for dangling symlinks in RFC 147 (its own source comment:
*"`worktree-status` said `refused paths: 0` for a tree `commit` refuses"*). It survives for
unrepresentable names. prikk deliberately keeps `refused` apart from `unsupported-path` — a name is not
an entry — and that distinction is sound; the narrower question is whether `authored` is the intended
verdict for an entry `commit` refuses. That question is prikk's, and it is what Q1 turns on.

## Decisions

1. **F0 is fixed first, in its own handoff.** The prose reader recognizes `unsupported-path`, and stops
   discarding unrecognized kinds, so `ChangeKind::Other` is reachable at last. Whether an indented line
   is an entry is decided by the entries region and the line's shape, not by a closed list of words; the
   handoff measures what else prikk indents in that region. Fixtures with unsupported entries are
   captured at both ends. The changelog's `### Fixed` says it affected every release since 0.1.0.
2. **`worktree-status` is read as JSON at ≥ 0.39**, under the existing `reads_json` gate, and as prose
   below it. The schema name is checked first, as for the other JSON reports. Per-kind counts are
   derived from `changes`: prikk computes its prose counts from that same list, so the two are equal by
   construction. **Paths are not validated as safe repository paths** — an `unsupported-path` entry's
   path is by definition not one, and refusing it would drop F0's entries a second way. The path is
   carried as reported and rendered inert.
3. **The model carries the verdict three-valued.** Per entry: `Authored`, `Refused(reason)` with the
   reason verbatim, or `Unreported` below 0.39. `WorktreeStatus` carries `refused: Option<u64>` —
   `None` below 0.39, **never `Some(0)`**.
4. **The Changes view shows a refused entry as refused**: a text tag, not colour alone, with prikk's
   reason verbatim and inert beside it. The header counts refused entries at ≥ 0.39. Below 0.39 it says
   in one line that this prikk does not report which paths commit would refuse, and never shows
   `refused 0`.
5. **`FR-050`, decided: at ≥ 0.39, commit is unavailable-with-a-reason when prikk's verdict refuses any
   entry** (F1, F2; RFC 014 decision 5b; `C-T4d`). The reason is stikk's own wording (`C-T2b`), listing
   each refused path, inert, with prikk's reason verbatim. Its next steps are the measured ones: remove
   or replace the path, or list it in `.prikkignore` — *which is then part of the commit* (F3). The check
   runs after the cross-ref and clean-worktree checks, since a clean tree has no entries.
   **Below 0.39 commit is offered exactly as today**, and prikk's refusal comes back verbatim; stikk does
   not infer a refusal prikk has not stated. If the tree changes between preview and commit, prikk's
   refusal reaches the seam and is presented verbatim, as RFC 014 decision 2 already handles for the
   cross-ref race.
6. **The real-binary suite drives both findings at both ends**: an untracked symlink (at 0.41 `Refused`
   with commit's own string and the preview blocked; at 0.28 `Unreported`, with commit refused verbatim),
   and an unsupported name (F0's entries listed at both ends). **Platform limits are announced skips, not
   silent ones**: creating a symlink on `windows-latest` needs privileges, a backslash cannot be part of a
   Windows filename, and a non-UTF-8 name is expected to be refused by macOS's filesystem. The handoff
   measures each on the matrix rather than trusting this paragraph.
7. **This is breaking, and 0.6.0 is published.** `WorktreeEntry`, `WorktreeStatus`, `ChangeEntry` and
   `ChangesView` gain fields and are not `#[non_exhaustive]`. The 023-B ruling's condition holds again —
   the workspace version names a published release — so the first breaking commit bumps the workspace to
   0.7.0, as its own commit.

## Open question

**Q1 — should commit's preview also be unavailable when an `unsupported-path` entry is present, given
that prikk marks those entries `authored` but `commit` refuses them (F5)?**

- **(a) Prevent on the kind.** stikk infers "commit will refuse" from `unsupported-path`. That is
  measured true for backslash and non-UTF-8 names at both 0.28 and 0.41. But it is stikk's inference
  over an explicit prikk field saying the opposite, and it drifts silently if prikk changes either side.
- **(b) Prevent only on prikk's verdict** (decision 5). After F0 those entries are finally on screen
  with prikk's own detail — *"worktree path is not representable as a safe Prikk path"* — and commit is
  offered, then refused verbatim. stikk writes to prikk asking whether they should carry
  `authoring: refused`. If prikk agrees, decision 5 covers them with no change here.
- **(c) Defer Q1 alone** until prikk answers; everything else proceeds.

**My lean is (b).** Every mechanism in this RFC rests on prikk stating the verdict, and (a) would make
the one exception the case prikk has just marked the other way. The cost is real — a user with an
unrepresentable name is offered a commit that fails — but after F0 the reason is on screen above the
action, and the refusal comes back in prikk's words. That is honest; (a) would be confident. **It is
yours because it trades a failure the user can see for a claim stikk cannot source.**

## Delivery

- **Handoff A — F0.** The prose reader, the two unreachable documented guarantees, fixtures at both ends,
  and its `### Fixed` entry. Not breaking, and small; it is a defect in every release, so it goes first.
- **Handoff B — decisions 2–7.** The JSON reader, the three-valued verdict, the Changes view, commit's
  prevention, the suite, and the 0.7.0 bump. Built on A's reader.

`FR-034` and `FR-050` are amended when this is accepted.

## What this RFC does not do

- **No partial commit.** `UD-06` stands.
- **stikk writes nothing into a repository.** `.prikkignore` is suggested, never edited (`CON-1`,
  `C-E2`).
- **No classification of prikk's refusal reasons.** They are carried verbatim.
- **No change to how an unsupported path's absolute location is displayed** beyond rendering it inert;
  that question goes to prikk.
- **Not the Queue view**, which is 0.7.0's second item.

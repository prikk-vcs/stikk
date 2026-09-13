# Handoff A — the prikk 0.42 re-baseline (v1)

**Companion to:** [RFC 029](../../accepted/029-prikk-0-42-rebaseline-and-the-current-branch.md) (Accepted
2026-09-13; **Q1 still open**).
**This handoff is decisions 1–3 and 5 of RFC 029: the mechanical re-baseline.** It does not depend on Q1, and
landing it unblocks RFC 028's Queue view.
**Handoff B**, whatever Q1 decides about stikk's focus and prikk's current branch, is not issued yet.
**Design items:** `ASM-2`, `NFR-R03`, `FR-055`, `TU-03`, RFC 027 decision 5.

> **This should be the quietest re-baseline yet, and that is a claim to verify, not to inherit.** The architect
> ran the whole real-binary suite at 0.28 and 0.42 in a scratch copy with only the ceiling raised: 18 passed, 0
> failed, confirmed to drive 0.42. **Your matrix run is the evidence; that local run is only why this handoff is
> short.**
>
> **No behaviour that Q1 decides.** stikk does not read prikk's current branch here, does not change where focus
> starts, and does not add `branch switch`. Where stikk's words now say something false about prikk, you correct
> the words.

---

## 1. Scope

**In, in this order:**
1. The ceiling, 41 → 42, committed alone (§2).
2. The suite at 0.28 and 0.42 — report the run first, whatever it shows (§3).
3. Fixtures: new 0.42 captures, and re-verification of what did not move (§4).
4. The unsupported-path tests, split by the prikk they describe (§5).
5. The claims that became false (§6).
6. The `0.41` sweep and the changelog (§7).

**Out:** reading `current branch:` or `current_branch` into any stikk type; where focus starts; the header;
`branch switch`; the Queue view (RFC 028); `show` on a queued patch.

## 2. The ceiling

`VALIDATED_MAX_MINOR = 42`, **committed on its own**, before the suite runs. The suite's version guard refuses a
0.42 binary until it moves. prikk 0.42.0 is on crates.io, so the workflow's install step can fetch it.

## 3. The suite — report the run before changing anything else

Run at **0.28 and 0.42** locally on the MSRV toolchain, **then on the full platform matrix**. Report the result
verbatim — per leg, per test — before any fixture moves.

**Expect it to pass.** If it does not, that failure is this handoff's most important finding, and it outranks
everything below. If it passes on the matrix too, say what that proves and what it does not: it covers the
eleven seam methods at two versions on three platforms, not every prose line stikk reads.

## 4. Fixtures

**Measured on 0.42 by the architect — verify each, do not copy:**

| Surface | 0.42 shape | What stikk reads at 0.42 |
|---|---|---|
| `status` prose | a new unindented `current branch: heads/main` line, between `heads/main RefState:` and `queued patches:` | **prose, always** — Orientation's reader looks lines up by label |
| `status` prose, pointer malformed or naming a missing or closed branch | `current branch: <unresolved; run `prikk doctor`>` | the same reader |
| `worktree-status`, `log` JSON | `current_branch`: a string, or `null` when unresolved | JSON at ≥ 0.39; the field is not read |
| `branch-list-v1` | `current` on each entry | JSON at ≥ 0.39; the field is not read |
| `worktree-status`, `log`, `branch list` prose | `current branch:` lines; `*` beside the current branch | prose only below 0.39 — **not read at 0.42** |
| `worktree-status` JSON, `unsupported-path` | `authoring: "refused"`, `commit`'s text, counted in `refused_count`; `path` relative to the worktree | JSON at ≥ 0.39 |

**Capture, with 0.42.0 provenance:**

1. **`status` prose** beside the five existing prose fixtures (`STATUS_EMPTY_FIXTURE`, `STATUS_QUEUED_FIXTURE`,
   `STATUS_CLEAN_PUBLISHED_FIXTURE`, `STATUS_QUEUED_WITH_WARNING_FIXTURE`, `STATUS_QUEUED_AT_HARD_LIMIT_FIXTURE`):
   **one** 0.42 capture with a queue and the `current branch:` line, and one with the unresolved form. Assert
   Orientation parses each to exactly what the equivalent older fixture gives. **That test is what makes "the
   reader ignores the new line" a fact rather than a reason.**
2. **`worktree-status --format json`** with the backslash, non-UTF-8 and subdirectory names (§5), from a neutral
   directory, as RFC 027's fixtures were.
3. **One JSON report with `current_branch: null`** — `worktree-status` or `log` — so a reader that someday
   requires the field fails a test, not a user.

**Re-verify, not re-capture**, every other fixture at 0.42, and keep the two claims distinct.
*"Re-verified unchanged"* is the weaker claim and must not be dressed as the stronger.

## 5. The unsupported-path tests, split by the prikk they describe

Two tests encode 0.41's `authored` verdict, and both remain true of 0.41:

- `an_unsupported_path_prikk_marks_authored_does_not_block` in `stikk-core/src/commit/tests.rs`;
- the unsupported-path fixture near `parse_json/tests.rs:685`.

**Keep them, and rename or document each as a 0.41 fact.** Beside them add:

- **A 0.42 fixture test**: every `unsupported-path` entry reads as refused with prikk's reason, and `refused`
  counts them.
- **A preview test**: commit is unavailable on a 0.42 worktree holding a refused `unsupported-path`. This is RFC
  027's Q1 ruling (b) doing what it waited for, and it needs no production change. **If it does need one, stop
  and report.**
- **The `U+FFFD` caution rendered.** It was unreachable until 0.42 made a non-UTF-8 name a refused path. An
  80-column `TestBackend` capture of the would-refuse overlay with `bad�name.txt`, showing the third next step.

## 6. The claims that became false

**Words only; no behaviour.** Found by the architect's sweep — re-run it, since grep is a floor:

| Where | Says | Becomes, in substance |
|---|---|---|
| `stikk-core/src/glossary.rs:45–46`, **shown to users** | *"(none — there is no HEAD)"*; *"prikk has no current-branch pointer; stikk focuses a named ref as a client-side preference."* | still no HEAD; **below 0.42 no pointer, from 0.42 a `.prikk/current-branch` default for `--ref` that prikk calls "never an authority"**; stikk focuses a named ref client-side |
| `stikk-tui/src/app.rs:38`, doc comment | *"prikk has no HEAD, so stikk focuses a named ref explicitly (FR-055)"* | version-accurate, pointing at RFC 029 |

**Do not describe how stikk relates to prikk's pointer.** That is Q1's decision; the glossary says what prikk
has, not what stikk does about it. The requirements' terminology row, `FR-055` and `TU-03` were already
amended on acceptance.

## 7. The sweep and the changelog

**Sweep:** `git ls-files | xargs grep -ln "0\.41"`. 44 files matched when this was written. Every **live**
statement of the validated range moves to 0.42; released changelog sections, RFC history, and fixtures'
provenance stay. Read comments and examples as claims.

**Changelog, `## Unreleased`:**
- **`### Changed`**: the validated prikk range is `>= 0.28`, through `0.42.0`.
- **`### Changed`**: on prikk ≥ 0.42, commit is unavailable, with prikk's reason, when a file's name cannot be a
  repository path (a backslash, or not UTF-8). prikk 0.42 began reporting those as refused, and stikk already
  prevents a commit prikk would refuse. It is the first time a user sees this, so say it.
- **`### Fixed`**: the Glossary no longer says prikk has no current-branch pointer.

## 8. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the
MSRV; gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite
and supply-chain run ids at one SHA.

## 9. Acceptance criteria

1. `VALIDATED_MAX_MINOR = 42`, in its own commit.
2. The suite's first run at 0.28 and 0.42 reported verbatim, before any fixture changed; then the matrix run
   named.
3. The two 0.42 `status` prose captures, with Orientation shown to parse them identically to their older
   equivalents.
4. The 0.42 unsupported-path capture, and a JSON report with `current_branch: null`.
5. Every other fixture re-verified, with re-captured and re-verified kept distinct.
6. The 0.41 unsupported-path tests kept and labelled as 0.41. The 0.42 fixture test, the preview-blocks test, and
   the rendered `U+FFFD` caution, all with no production change.
7. The Glossary entry and `app.rs`'s doc made version-accurate, without deciding Q1.
8. The `0.41` sweep done, and the changelog as §7.
9. Eight gates under the toolchain rule; three run ids at one SHA.
10. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/029-a-rebaseline/review-request-v1.md`.

**Lead with §3's first run and the matrix**, leg by leg. **Then the Orientation parse test over the 0.42 `status`
captures**, which is the one place stikk still reads prose that 0.42 changed. **Then the rendered `U+FFFD`
caution.**

**And tell me what 0.42's binary shows that neither this handoff nor prikk's changelog predicted.** The
architect measured a lot of it; that is exactly why a second pair of eyes on the binary matters.

**Push once approved.**

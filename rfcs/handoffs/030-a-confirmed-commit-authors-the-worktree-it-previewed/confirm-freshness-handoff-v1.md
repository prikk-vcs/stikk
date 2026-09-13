# Handoff — a confirmed commit authors the worktree its preview showed (v1)

**Companion to:** [RFC 030](../../accepted/030-a-confirmed-commit-authors-the-worktree-it-previewed.md) (Accepted
2026-09-13; no open question).
**This handoff is all six decisions of RFC 030.**
**Sequencing:** after [RFC 029 Handoff A](../029-prikk-0-42-rebaseline-and-the-current-branch/a-rebaseline-handoff-v1.md)
is approved, because the suite's 0.42 leg needs the raised ceiling. Core and seam work may start before that, but
nothing lands on `main` ahead of A. **RFC 029 Handoff B follows this one** and builds on §2's field.
**Design items:** `OPL-02`, `FR-050`, `FR-052`, `FR-106`, `T-T4`, `C-T2b`, `C-T2c′`, `ER-02`.

> **The failure this closes is measured, not hypothetical.** At prikk 0.42 the architect previewed a commit onto
> `heads/main` whose only change was one untracked file, switched branches in a terminal, and ran the commit
> stikk would have run. prikk recorded `create-file dev-only.txt` and `edit-text shared.txt` onto `heads/main`.
> The preview had shown one untracked file. stikk's change token did not move.
>
> **Your first job is to reproduce that with the suite on today's code, red, before you change anything.** A
> safeguard whose test never failed has not been shown to guard.

---

## 1. Scope

**In, in this order:**
1. The suite tests of §6, written first and shown **failing** on the unchanged code.
2. Orientation reads prikk's current branch (§2).
3. The change token composes it (§3).
4. Commit's confirmation re-reads the worktree (§4).
5. Core tests (§5), then the suite tests going green.
6. `FR-050`, the changelog and the Breaking table (§7).

**Out:** anything rendered from the current branch (the header, where focus starts, and the safeguard-3 notice
all belong to RFC 029 Handoff B); `branch switch` from stikk; a worktree check on seal; content digests.

## 2. Measured at prikk 0.42.0 — verify each, do not copy

From the architect's scratch probes against the 0.42.0 binary:

| # | Scenario | prikk 0.42 does |
|---|---|---|
| M1 | Clean on `heads/main`, one **untracked** file added | `worktree-status --ref heads/main` lists it (`untracked`, `authored`), so commit's preview is Ready |
| M2 | M1, then `prikk branch switch heads/dev` | **Switches.** Writes dev's files, keeps the untracked file, and `status` prints `current branch: heads/dev` |
| M3 | M2, then `worktree-status --ref heads/main` | lists dev's `dev-only.txt` (`untracked`) and `shared.txt` (`modified`) **beside** the untracked file |
| M4 | M2, then `commit --from-worktree --ref heads/main` | **Records dev's files onto `heads/main`**, exit 0 |
| M5 | A **tracked** file modified on `heads/main`, then `branch switch heads/dev` | **Refuses:** `error: precondition not met: the worktree is not clean against heads/main: shared.txt (modified); commit it, or restore those files, before switching (…)` |
| M6 | Clean on `heads/dev`; preview onto `heads/main` (dev's tree lists as changes); `branch switch heads/main` | Switches; `worktree-status --ref heads/main` is now clean |

**What this means for the tests:**
- **RFC 030 decision 5's switch test must preview a change made only of untracked files.** A tracked edit makes
  prikk refuse the switch (M5), and a refused switch tests nothing. The RFC is amended to say so.
- **M6 is the token's case alone.** The worktree re-read would also catch it, since the tree went from changed
  to clean, so a test aimed only at the token must use a NullBackend (§5).

**Also verify:** the exact unresolved text in `status` prose. RFC 029 Handoff A's table records
``current branch: <unresolved; run `prikk doctor`>``, with backticks. **Carry whatever the binary prints,
byte for byte.**

## 3. Orientation reads prikk's current branch (decision 1)

- **`stikk_prikk::Orientation` gains the current branch, as three states kept distinct in the type.** An enum,
  not an `Option<String>`:
  - **not reported** — prikk below 0.42 prints no `current branch:` line;
  - **unresolved** — prikk's text after the label, **verbatim** (`ER-02`);
  - **a branch** — validated as a `stikk_model::RefName`.

  Name the type and the variants as the crate's idiom reads.
- **Read from the `status` prose Orientation already parses**, by label, as its other lines are. No new spawn,
  and **never `.prikk/current-branch`** (`CON-1`, RFC 029 decision 3).
- **Parse failures are failures, never a fallback** (`C-T2c′`):
  - at prikk ≥ 0.42, a missing `current branch:` line is a parse error, not "not reported";
  - a value that is neither prikk's unresolved form nor a valid `RefName` is a parse error.

  How the parser learns the version is yours: pass it in, or check at the call site. Say which, and why.
- **`OrientationView` carries it through, unrendered.** Handoff B renders it.
- **Fixtures:** the 0.42 `status` captures from RFC 029 Handoff A — branch form and unresolved form — now
  assert the new field too. The older prose fixtures assert "not reported". **The unresolved text is asserted
  byte for byte against the capture.**
- **`NullBackend`'s default Orientation is "not reported"**, so existing tests keep their meaning.

## 4. The change token composes it (decision 2)

- **`ChangeToken::compose` takes the current branch as a further input**, hashed with a **discriminant per
  state**, so "unresolved" with text `heads/x` can never equal the branch `heads/x`.
- **`CliBackend::change_token`** passes the Orientation it already reads, so there is no new spawn. **Rewrite
  its comment on the excluded worktree marker:** the exclusion stands for the token, and RFC 030 moved the
  worktree check to commit's confirmation.
- **Model tests:** each pair of the three states composes distinct tokens, the same state composes equal ones,
  and two different branch names compose distinct tokens.
- **Consequence to state, not suppress:** the token also feeds `staleness_notice` (`FR-106`), so a terminal
  `branch switch` now produces *"repository changed outside stikk — refreshed"*. That sentence is true of a
  switch. Say in your review request that you checked it, and where it renders.

## 5. Commit's confirmation re-reads the worktree (decision 3)

**The previewed `ChangesView` must travel with the token, and a caller must not be able to substitute one.**

- **`CommitPreviewOutcome::Ready`'s `token` becomes a commit-specific token** that owns the `PreviewToken`, the
  previewed ref, and the previewed `ChangesView`, all with **private fields**.
- **`commit_confirm_and_execute` takes that token, and commits to the ref inside it.** Remove the separate `reff`
  parameter: a ref that could differ from the preview's has no correct meaning. If you find a reason to keep
  it, **stop and report** instead of reconciling the two.
- **Where the re-read runs:** inside the closure `confirm::execute` runs, **after** `execute`'s own token check
  and **immediately before** `prikk.commit`. That narrows the F4 window as far as stikk can.
  1. Call `worktree_status(repo, previewed_ref)`.
  2. Build the view **through the same function the preview used** (`changes::from_status`), not a second
     construction that could drift from it.
  3. **Unequal** → `StikkError::Stale { operation }`, and `prikk commit` does not run.
  4. **The re-read fails** → propagate that error, and `prikk commit` does not run.
- **Seal changes only through §4.** Seal authors no worktree.
- **The TUI:** `PendingCommit::Confirming` holds the new token, and the worker passes it through. `Stale` already
  routes to `Overlay::Stale` through `present()`.
  - **Quote that overlay's words in your review request** and say whether they are true of a worktree that
    changed while refs did not (`C-T2b`).
  - If they are not, **stop and report**; do not reword. The words are a design decision.
  - Add a `TestBackend` capture of a commit confirmation that went stale because the worktree changed.

**Core tests, through `NullBackend`:**
1. **Equal re-read:** the commit runs.
2. **An entry added:** `Stale`, and the commit is **provably not called**.
3. **Only a count differs:** `Stale`, commit not called.
4. **The re-read refused:** that refusal propagates, commit not called.
5. **The token-only case (M6):** refs and queue unchanged, current branch moved, worktree unchanged → `Stale` from
   the token, before any re-read.

`NullBackend` needs a way to answer `worktree_status` differently on its second call, and to record whether
`commit` ran. **Add what is missing as test support, in the style of its existing `with_*` knobs.** If one
already exists, use it.

## 6. The suite (decision 5)

**Write these first and run them on today's code.** Report each red result verbatim, per version.

1. **At 0.42 only — the branch switch.**
   1. Seal `heads/main`, and publish and seal `heads/dev` with a file main lacks and a `shared.txt` that
      differs. Switch back to main, clean.
   2. Add one **untracked** file. `commit_preview` on `heads/main` is `Ready`, listing only that file.
   3. Raw `prikk branch switch heads/dev`, and assert it exits 0.
   4. **Assert `worktree_status` for `heads/main` now differs from the preview.** Without this assertion the test
      could pass on an unchanged tree and prove nothing.
   5. `commit_confirm_and_execute` → `Stale`, and **nothing is queued**, asserted from prikk's own `status`.
   6. **Below 0.42, skip with an announcement** (prikk has no `branch switch`), as the suite announces other
      version skips.
2. **At 0.28 and 0.42 — a file added.**
   1. Preview with one untracked file.
   2. Add a second file.
   3. Confirm → `Stale`, nothing queued.
   4. **Then the positive control:** preview again, confirm at once, and the commit **is recorded**, with both
      files in prikk's queue. **A safeguard that blocks every commit also passes step 3**; this step is what rules
      that out.
3. **The existing full-lifecycle test stays green, unmodified,** which is the second positive control. If it
   needs a change beyond the new signature, report why.

## 7. Docs, changelog, Breaking

- **`FR-050`** gains RFC 030 F4's limits, in substance:
  - commit's confirmation re-checks the worktree at path level, and a further edit to a file already shown as
    modified is not detected;
  - a window remains between that re-check and `prikk commit`, and prikk authors what is there.

  **State the limits, not a guarantee.**
- **Changelog, `## Unreleased`:**
  - **`### Fixed`**: a commit confirmed after the worktree changed, a terminal `prikk branch switch` included,
    no longer commits the changed worktree. stikk reports the preview stale and commits nothing.
  - **`### Changed`**: on prikk ≥ 0.42, switching branches outside stikk counts as a repository change (the
    staleness notice, and stale commit and seal confirmations).
- **Breaking table: build it by API diff, not grep** (signatures, variants, fields, re-exports). Expect at least:
  - `Orientation`'s new field;
  - `ChangeToken::compose`;
  - `CommitPreviewOutcome::Ready`'s token type;
  - `commit_confirm_and_execute`'s parameters.

  All are inside unreleased 0.7.0 (decision 6).

## 8. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite,
supply-chain and Docs run ids at one SHA.

## 9. Acceptance criteria

1. §6's tests shown **red on the unchanged code**, verbatim, per version, and the switch test's worktree-differs
   assertion shown holding.
2. Orientation's three-state current branch, parsed by label from the `status` read already made. A missing line
   at ≥ 0.42 and an invalid value are parse errors. The unresolved text is byte-exact to the 0.42 capture.
   Nothing reads `.prikk/`.
3. The token composes the three states distinctly, with model tests for each pair.
4. The commit token owns the previewed ref and view, with private fields. `commit_confirm_and_execute` takes no
   separate ref. The re-read runs after `execute`'s token check and immediately before `prikk.commit`, through
   the preview's own construction.
5. §5's five core tests, including commit **not called** on every `Stale` and refusal path.
6. The stale overlay's words quoted and judged against `C-T2b`, with a `TestBackend` capture.
7. §6's tests green at 0.28 and 0.42, including the positive control, and the below-0.42 skip announced.
8. `FR-050`, the changelog, and a Breaking table built by API diff.
9. Eight gates under the toolchain rule; four run ids at one SHA.
10. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/030-a-confirm-freshness/review-request-v1.md`.

**Lead with the red run**, then the same tests green, including the positive control. **Then the stale
overlay's words**, and whether they are true of a worktree-only change. **Then the Breaking table.**

**And tell me whether anything in 0.42 lets the worktree change between preview and confirmation in a way §6
does not cover.** RFC 030 F4 names what re-reading cannot see; if you find something it can see but this
handoff does not test, that is a finding.

**Push once approved.**

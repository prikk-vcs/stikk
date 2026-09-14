# Handoff — a confirmed commit authors the worktree its preview showed (v2)

**Companion to:** [RFC 030](../../accepted/030-a-confirmed-commit-authors-the-worktree-it-previewed.md) (Accepted
2026-09-13; **amended 2026-09-15**, see its *Amendments*).
**Supersedes v1.** This handoff is RFC 030's decisions 1–7, including amendments A1–A3.
**Starting point:** `origin/main` at `7417738` (RFC 029 Handoff A landed), plus the two suite tests you wrote
and ran red in v1's first review. Keep them; §6 extends them.
**RFC 029 Handoff B follows this one** and builds on §3's field.
**Design items:** `OPL-02`, `FR-050`, `FR-052`, `FR-106`, `T-T4`, `C-T2b`, `C-T2c′`, `ER-02`, `ER-03`.

> **What changed since v1, and why.** Your first review stopped at the two points v1 told you to stop at, and
> both were right.
>
> - **§5 now compares prikk's rename declarations.** At prikk 0.42 a `prikk mv` made after the preview leaves
>   `worktree-status`'s path list **identical** and changes what `commit` authors (§2, M7). Your suspicion was
>   correct.
> - **`Stale` carries a cause, with true words for each (§5b).** The v1 words were not true of a worktree
>   change.
> - **§6 gains the declaration test**, which must also be shown red on the unchanged code.

---

## 1. Scope

**In, in this order:**
1. **§6's third test**, written and shown **red** on the unchanged code. Tests 1 and 2 are already red; keep
   that evidence.
2. Orientation reads prikk's current branch (§3).
3. The change token composes it (§4).
4. `WorktreeStatus` and `ChangesView` carry rename declarations (§5a).
5. `Stale` carries its cause, and the words (§5b).
6. Commit's confirmation re-reads the worktree (§5c).
7. Core and TUI tests (§5d), then the suite tests green (§6).
8. `FR-050`, the doc sentence RFC 030 makes false, the changelog and the Breaking table (§7).

**Out:**
- **Showing declarations anywhere.** That is roadmap item 11, and its own RFC.
- Anything rendered from the current branch: the header, where focus starts, and the safeguard-3 notice all
  belong to RFC 029 Handoff B.
- `branch switch` from stikk.
- A worktree check on seal.
- Content digests.

## 2. Measured at prikk 0.42.0 — verify each, do not copy

| # | Scenario | prikk 0.42 does |
|---|---|---|
| M1 | Clean on `heads/main`, one **untracked** file added | `worktree-status --ref heads/main` lists it (`untracked`, `authored`), so commit's preview is Ready |
| M2 | M1, then `prikk branch switch heads/dev` | **Switches.** Writes dev's files, keeps the untracked file; `status` prints `current branch: heads/dev` |
| M3 | M2, then `worktree-status --ref heads/main` | lists dev's `dev-only.txt` and `shared.txt` **beside** the untracked file |
| M4 | M2, then `commit --from-worktree --ref heads/main` | **Records dev's files onto `heads/main`**, exit 0. You reproduced this through stikk's confirm path. |
| M5 | A **tracked** file modified, then `branch switch heads/dev` | **Refuses:** `error: precondition not met: the worktree is not clean against heads/main: …` |
| M6 | Clean on `heads/dev`; preview onto `heads/main`; `branch switch heads/main` | Switches; `worktree-status --ref heads/main` is now clean |
| **M7** | `a.txt` sealed; shell `mv a.txt b.txt`; then `prikk mv a.txt b.txt` | **Before `prikk mv`:** `changes` = `missing a.txt`, `untracked b.txt`; `declarations` = `[]`. **After:** prikk prints `declared a.txt -> b.txt (already moved on disk; no bytes touched)`; `changes` **identical**; `declarations` = `[{"old_path": "a.txt", "new_path": "b.txt"}]`; `prikk commit` authors **`rename-path a.txt -> b.txt`** |
| M8 | Clean tree; `prikk mv a.txt b.txt` | prikk moves the file itself; `changes` gains `missing a.txt` and `untracked b.txt`, so the path list changes too |

**What M7 means.** A comparison over paths alone passes, and the commit authors an operation the preview never
held. M8 is caught by paths, but M7 is only caught by declarations.

## 3. Orientation reads prikk's current branch (decision 1)

Unchanged from v1:

- **`stikk_prikk::Orientation` gains the current branch as three states, in an enum**, not an
  `Option<String>`:
  - **not reported** — below 0.42, no line;
  - **unresolved** — prikk's text after the label, **verbatim** (`ER-02`);
  - **a branch** — a validated `stikk_model::RefName`.

  Name the type and variants in the crate's idiom.
- **Read by label from the `status` prose Orientation already parses.** No new spawn, and **never
  `.prikk/current-branch`** (`CON-1`).
- **Parse failures are failures** (`C-T2c′`):
  - at prikk ≥ 0.42, a missing line is a parse error;
  - a value that is neither prikk's unresolved form nor a valid `RefName` is a parse error.

  How the parser learns the version is yours; say which way and why.
- **`OrientationView` carries it through, unrendered.**
- **Fixtures.** Extend Handoff A's `a_0_42_status_parses_exactly_as_its_older_equivalent_under_both_pointer_forms`
  — it compares whole `Orientation` values, so it must now expect the field:
  - the 0.42 captures carry "a branch" and "unresolved" (the latter byte-exact to
    `STATUS_QUEUED_UNRESOLVED_0_42_FIXTURE`);
  - the older fixtures carry "not reported".

  **Do not capture the same surface again.**
- **`NullBackend`'s default Orientation is "not reported".**

## 4. The change token composes it (decision 2)

Unchanged from v1:

- **`ChangeToken::compose` takes the current branch**, hashed with a **discriminant per state**. The unresolved
  text `heads/x` must never equal the branch `heads/x`.
- **`CliBackend::change_token`** passes the Orientation it already reads. **Rewrite its comment on the excluded
  worktree marker:** the exclusion stands for the token, and RFC 030 moved the worktree check to commit's
  confirmation.
- **Model tests:** each pair of states distinct, the same state equal, two branch names distinct.
- **State the consequence:** `staleness_notice` (`FR-106`) now fires on a terminal `branch switch`. Its
  sentence, *"repository changed outside stikk — refreshed"*, is true of that. Say you checked it, and where it
  renders.

## 5. The confirmation

### 5a. Rename declarations (amendment A1)

- **`WorktreeStatus` gains the declarations**, as a list of `(old path, new path)` in a small named type.
  - **At prikk ≥ 0.39**, read the JSON `declarations` array. It must be **present**, since its absence is not
    "none". Each entry needs string `old_path` and `new_path`. Carry the paths as reported and render them inert
    wherever they are ever shown, as the entries' paths are.
  - **At 0.38**, read the prose `live rename declarations: N` section. Its shape is in
    `WORKTREE_RENAME_0_38_FIXTURE` and `WORKTREE_RENAME_BARE_KIND_0_38_FIXTURE`: `  <old> -> <new>` lines.
    The F0 parser (`parse.rs` near line 379) already locates the section.
    - **Check `N` against the lines.**
    - **If a line cannot be split unambiguously**, for example because a name contains ` -> `, **fail the
      parse**; do not guess. Report what you find there.
  - **Below 0.38 the list is empty, as a version fact**: `prikk mv` does not exist there, so nothing can be
    declared. **Say so in the field's doc**, because that is what makes an empty list true rather than an
    unreported zero.
- **Update `parse_json.rs`'s doc**, which says *"`declarations` is not read"*.
- **`ChangesView` carries them through `changes::from_status`**, so equality covers them.
- **Nothing renders them.** The Changes view and the commit preview are unchanged on screen.

**Fixtures:** the existing 0.38 prose fixtures and the ≥ 0.39 JSON fixtures now assert the declarations they
hold. `STATUS_JSON_UNRESOLVED_NODE_0_38_FIXTURE` and the JSON worktree fixtures show `[]` or real entries.
**Capture one 0.42 JSON report with a declaration** (M7's tree after `prikk mv`), from a neutral directory, with
provenance.

### 5b. `Stale` carries its cause, and the words (amendment A3)

- **`stikk_model::StikkError::Stale { operation, cause }`**, where `cause` is a new `StaleCause` enum:
  `Repository` or `Worktree`. `Presentation::Stale` and `Overlay::Stale` carry it through.
- **The token comparisons in `confirm` and `execute` raise `Repository`**, and §5c's re-read raises
  `Worktree`.
- **The words live in `stikk-core`** (`present.rs`), headline and gloss, so both frontends say the same. The
  TUI's `render_stale` takes the headline from core instead of its own literal.

**The words, exactly** — the headline follows the bold operation name:

| Cause | Headline | Gloss |
|---|---|---|
| `Repository` | `: the repository changed since this was last previewed.` | `Something in this repository changed between your preview and now: a branch or a tag, the queue, or, on prikk 0.42 and later, prikk's current branch. This is not a retry: previewing again re-reads the repository's current state, which is the only safe way forward.` |
| `Worktree` | `: the worktree changed since this was last previewed.` | `What prikk reports about the worktree no longer matches what this preview listed: a path was added or removed, a path's status or prikk's verdict on it changed, or its rename declarations changed. Nothing was committed. Previewing again shows what a commit would author now, which is the only safe way forward.` |

**The Display form:** `stale: {operation}'s preview no longer matches the repository` for `Repository`, and `…
the worktree` for `Worktree`.

**If you find either sentence untrue for a case this handoff creates, stop and report again.** That is the
same rule as v1, and it worked.

### 5c. Commit's confirmation re-reads the worktree (decision 3)

Unchanged from v1 except for what is compared:

- **`CommitPreviewOutcome::Ready`'s `token` becomes a commit-specific token.** It owns the `PreviewToken`, the
  previewed ref and the previewed `ChangesView`, all in **private fields**.
- **`commit_confirm_and_execute` takes that token and commits to the ref inside it.** Remove the separate `reff`
  parameter; if you find a reason to keep it, **stop and report**.
- **The re-read runs inside the closure `confirm::execute` runs**, **after** `execute`'s token check and
  **immediately before** `prikk.commit`:
  1. Call `worktree_status(repo, previewed_ref)`.
  2. Build the view through **`changes::from_status`**, the preview's own construction.
  3. **Unequal**, declarations included → `Stale { cause: Worktree }`, and `prikk commit` does not run.
  4. **The re-read fails** → that error propagates, and `prikk commit` does not run.
- **Seal changes only through §4 and §5b.**
- **The TUI:** `PendingCommit::Confirming` holds the new token, and the worker passes it through.

### 5d. Core and TUI tests

Through `NullBackend`, adding the test support it lacks (a second `worktree_status` answer, a record of
whether `commit` ran) in the style of its `with_*` knobs:

1. **Equal re-read:** the commit runs.
2. **An entry added:** `Stale { Worktree }`, and commit **provably not called**.
3. **Only a count differs:** `Stale { Worktree }`, not called.
4. **Only the declarations differ**, M7's shape: `Stale { Worktree }`, not called.
5. **The re-read refused:** that refusal propagates, not called.
6. **The token-only case (M6):** `Stale { Repository }`, before any re-read.
7. **`present()` for each cause** yields exactly §5b's headline and gloss, and one next step, `Preview again`.
8. **Two `TestBackend` captures of the stale overlay at 80 columns**, one per cause. **Quote both in your review
   request.**

## 6. The suite (decision 5, amendment A2)

**Tests 1 and 2 are yours from v1, and already red.** Extend each:

1. **The branch switch, at 0.42:**
   - assert the cause is `Repository`, because the token catches it first;
   - replace the `reff` argument with the new token.
2. **A file added, at both ends:**
   - assert the cause is `Worktree`;
   - keep the positive control as written.
3. **New — the declaration, at prikk ≥ 0.38:**
   1. Seal `a.txt` on `heads/main`, then shell-move it to `b.txt`.
   2. `commit_preview` is Ready. Record `worktree_status`'s entries and declarations.
   3. Run raw `prikk mv a.txt b.txt`.
   4. **Assert the entries are unchanged and the declarations differ.** Without this assertion the test would not
      show why declarations must be compared.
   5. Confirm → `Stale { Worktree }`; **nothing queued**, by prikk's own `status`.
   6. **Positive control:** preview again, confirm at once, and prikk's recorded changes include
      `rename-path` (or whatever `CommitResult` names it).
   7. **Below 0.38, skip with an announcement.**

   **Write this one first and run it red on the unchanged code**, like the other two, and report it verbatim.
4. **The full-lifecycle test stays green**, changed only for the new signature.

## 7. Docs, changelog, Breaking

- **`FR-050`** gains RFC 030 F4's limits:
  - commit's confirmation re-checks what prikk reports about the worktree (paths, statuses, verdicts and rename
    declarations), and a further edit to a file already shown as modified is not detected;
  - a window remains before `prikk commit`, and prikk authors what is there.

  **State the limits, not a guarantee.**
- **`crates/stikk-prikk/src/version.rs`'s module doc** says `status` gained *"a `current branch:` line
  Orientation does not read"*. **§3 makes that false; correct it.**
- **Changelog, `## Unreleased`:**
  - **`### Fixed`**: a commit confirmed after the worktree changed no longer commits the changed worktree,
    whether the change was a terminal `prikk branch switch`, a file added or removed, or a `prikk mv`. stikk
    says the worktree changed and commits nothing.
  - **`### Changed`**: on prikk ≥ 0.42, switching branches outside stikk counts as a repository change.
  - **`### Changed`**: a stale confirmation now says whether the repository or the worktree changed, and no longer
    attributes the change to *"another writer"*.
- **Breaking table, built by API diff** (signatures, variants, fields, re-exports). Expect at least:
  - `Orientation`'s field;
  - `WorktreeStatus`' and `ChangesView`'s declarations;
  - `ChangeToken::compose`;
  - `StikkError::Stale`'s `cause`, and the new `StaleCause`;
  - `Presentation::Stale`;
  - `CommitPreviewOutcome::Ready`'s token type;
  - `commit_confirm_and_execute`'s parameters.

## 8. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite,
supply-chain and Docs run ids at one SHA. Docs runs on `main` only, so name it after the push.

## 9. Acceptance criteria

1. The **declaration test red on the unchanged code**, verbatim, beside v1's red run of tests 1 and 2.
2. **Orientation:** the three-state current branch, parsed by label; parse errors as §3 says; Handoff A's test
   extended, not duplicated; nothing reads `.prikk/`.
3. **The token:** composes the three states distinctly, with model tests.
4. **Declarations:** carried at ≥ 0.39 (JSON, required present) and 0.38 (prose, count-checked, ambiguity a
   parse error); empty below 0.38 as a documented version fact; the 0.42 declaration capture.
5. **`Stale { operation, cause }`:** the words byte-exact to §5b, in core; the TUI takes its headline from core.
6. **The commit token:** owns the previewed ref and view, in private fields; no separate ref parameter; the
   re-read after `execute`'s token check and immediately before `prikk.commit`, through `changes::from_status`.
7. **§5d:** the eight tests, including commit **not called** on every `Stale` and refusal path, and both overlay
   captures quoted.
8. **§6:** the three tests green at 0.28 and 0.42, with causes asserted, both positive controls, and both skips
   announced.
9. **§7:** `FR-050`, `version.rs`'s sentence, the changelog, and a Breaking table built by API diff.
10. **Gates:** the eight under the toolchain rule; the run ids at one SHA.
11. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/030-a-confirm-freshness/review-request-v3.md`.

**In this order:**
1. **The declaration test's red run.**
2. **The three suite tests green,** with both positive controls.
3. **The two stale overlay captures.**
4. **The Breaking table.**

**§10's question from v1 stands:** does anything in 0.42 let what `commit` authors change between preview and
confirmation in a way §6 still does not cover? Your v2 list named three candidates:
- **`checkout --patch-materialize`:** measure it and report.
- **`prikk mv`:** M7 settles it.
- **`.prikkignore`:** it changes the path list, but confirm it by measuring.

**Push once approved.**

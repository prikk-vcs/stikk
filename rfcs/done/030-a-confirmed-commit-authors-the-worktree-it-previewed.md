# RFC 030 — A confirmed commit authors the worktree its preview showed

**Status.** **Done 2026-09-15** — delivered on `main` (`1420e45`, `40321f7`); a **0.7.0 candidate**. Accepted by the
project owner 2026-09-13; **amended 2026-09-15** by the architect after the handoff's first review (see *Amendments*):
decision 3 widened to prikk's rename declarations, decision 5 gained a test, and decision 7 added. Proposed 2026-09-13
by the architect, from RFC 029 F7, and delivered before RFC 029's Handoff B.
**Tracks.** `OPL-02` (the change token), RFC 003 decision 3, RFC 013's confirm primitive, RFC 014, `FR-050`,
`FR-052`, `T-T4`, RFC 029 F7 and F8.
**Touches.** `stikk-prikk` (Orientation reads prikk's current branch), `stikk-model` (the change token), `stikk-core`
(commit's confirmation), `stikk-real-binary`.

## Delivered

**One handoff, reissued as v2 after its first review** (`1420e45`, then `40321f7` for the review's two conditions).

- **Orientation reads prikk's current branch** as `CurrentBranch::{NotReported, Unresolved, Branch}`, with the version
  passed in. A missing line at ≥ 0.42 is a parse error, and so is any `<…>` sentinel stikk has not seen.
- **The change token composes it**, hashed with a discriminant per state.
- **`WorktreeStatus` and `ChangesView` carry prikk's rename declarations:**
  - as JSON at ≥ 0.39, where the array must be present;
  - as prose at 0.38, where the count is checked and a missing section or an ambiguous line is a parse error;
  - below 0.38 the list is empty, as a version fact.
- **Commit's confirmation owns its previewed ref and view** in a private `CommitToken`. The re-read runs after
  `execute`'s token check and immediately before `prikk commit`.
- **`Stale` carries its cause**, and its words live in `stikk-core`.

**Red first, measured through stikk's own confirm path on the unchanged code** (`7417738`, and `3fdcf96` for the
declaration test):
- at 0.42, a branch switch committed `heads/dev`'s files onto `heads/main`;
- a file added after the preview was committed, at 0.28 and at 0.42;
- a `prikk mv` committed `rename-path`.

**Green:**
- 21 of 21 at 0.28 and 0.42 on Linux, macOS and Windows (suite `34905701292`, CI `34905702476`, supply chain
  `34905703517` at `1420e45`);
- re-run by the architect at both commits;
- CI `34907596129` and Docs `34907596091` at `40321f7`.

### Carried forward

- **The commit preview does not show the rename a declaration authors** (roadmap item 11); its own RFC.
- **`FR-106`'s passive notice has no caller** (roadmap item 12). Handoffs v1 and v2 wrongly said decision 2 made it
  fire; the dev team's review found it.
- **A prose-path parse error (prikk < 0.39) reaches the refusal classifier** (roadmap item 7). That now includes
  0.38's missing declarations section: it fails, as it should, but reads as prikk's refusal.
- **`checkout --patch-materialize` writes before it refuses**, and neither `status` nor `doctor` reports it. prikk's
  behaviour; letter 012 is drafted for the owner.

## Summary

**A commit preview in stikk does not bind the commit to what the preview showed.** Between the preview and the
user's confirmation the worktree can change — by an edit, a `prikk checkout`, or, since prikk 0.42, a single
`prikk branch switch` that replaces the whole tree — and **stikk commits the new worktree onto the ref it previewed,
with nothing on screen saying so.**

Two safeguards close most of it, both cheap, and neither depends on how RFC 029's Q1 was ruled:

1. **prikk's current branch joins the change token**, so a branch switch between any preview and its confirmation
   is stale.
2. **Commit's confirmation re-reads the worktree** and requires it to match the preview.

## Findings

### F1 — commit's confirmation checks refs and the queue, and then authors the worktree

`confirm::confirm` re-checks the capability and compares the change token, which composes refs, tags and the queue
(`CliBackend::change_token`). `commit_confirm_and_execute` then runs `prikk commit`, which authors **the worktree as
it is at that moment**. The preview's `ChangesView` is never read again.

### F2 — RFC 003 excluded the worktree on a premise that holds for views, not for confirmation

RFC 003: *"the worktree marker is deliberately excluded: it would need a `worktree-status` spawn per token … a
preview built from it is self-freshening."* **That is true while a view is on screen** — the Changes view re-reads
on refresh. **It is not true at confirmation**, where nothing re-reads. The objection was to a spawn per *token*;
this is one spawn per *confirmed commit*, a human act, which is the price RFC 026 B already accepted for re-checking
readiness at the seam.

### F3 — at prikk ≥ 0.42 the token cannot see a branch switch

Measured (RFC 029 F7): `prikk branch switch heads/dev` wrote two files and deleted one, and moved the pointer, while
refs, tags and the queue — every input the token has — stayed as they were. **prikk's `current branch:` line is in
the `status` report the token already reads**, so adding it costs no spawn.

### F4 — what re-reading can detect, and what it cannot

prikk's worktree report is path-level: kind, path, detail and authoring verdict, never a content digest.

- **Detectable:** any change in which paths are listed, their kinds, or prikk's verdict — a branch switch, a
  checkout, an added, deleted or reverted file. *(Amended: and, once decision 3 compares them, prikk's rename declarations.)*
- **Not detectable:** a further edit to a file already shown as `modified`. It stays `modified`, with the same path.
- **And a window remains** between the re-read and `prikk commit` itself. prikk is the last authority, and it
  authors what is there.

**These limits are stated, not implied away.**

### F5 — the pointer alone is not evidence

RFC 029 F8: `checkout --patch-materialize` wrote another branch's files and left the pointer alone. So safeguard 2
compares **what prikk reports about the worktree**, not the pointer.

## Decisions

1. **Orientation reads prikk's current branch** from the `status` report it already parses, as **three states
   kept distinct**:
   - **absent** — prikk < 0.42 prints no line;
   - **unresolved** — prikk's ``<unresolved; run `prikk doctor`>``, carried verbatim;
   - **a branch** — validated as a `RefName`.

   stikk never reads `.prikk/current-branch` itself. RFC 029's Handoff B builds its focus and header on this field.
2. **The change token composes the current branch**, with the three states distinct, so a switch between any
   preview and its confirmation is `Stale` — for commit and seal alike.
3. **Commit's confirmation re-reads `worktree-status` for the previewed ref** and requires the resulting
   `ChangesView` to equal the one the preview showed. Any difference is `StikkError::Stale`, presented through the
   existing stale overlay, and `prikk commit` is not run. It is commit-specific: seal authors no worktree.
4. **The limits in F4 are recorded in `FR-050`** on delivery. The confirmation is not dressed as a guarantee it
   cannot make.
5. **The suite drives it against real binaries:**
   - **at 0.42:** a preview on `heads/main` whose only change is an untracked file, then a raw `prikk branch
     switch heads/dev`, then confirmation — the result is `Stale` and nothing is queued. *(Amended on acceptance,
     measured at 0.42.0: prikk refuses to switch over a modified tracked file, so a preview holding one cannot
     reach the race. An untracked file is carried across the switch, and the worktree then also lists dev's
     files against `heads/main`; committing records them there.)*
   - **at both ends:** a preview, then a file added, then confirmation — the result is `Stale` and nothing is
     queued;
   - **below 0.42** there is no branch switch, and safeguard 2 still holds. That skip is announced.
6. **Breaking, inside 0.7.0**, which is unreleased and already breaking: the token's composition and Orientation's
   fields change.

## Amendments — 2026-09-15, from the handoff's first review

The dev team stopped at two points the handoff told them to stop at, and both were right.

### A1 — decision 3 widened: prikk's rename declarations are part of what commit authors

**Measured at prikk 0.42.0** by the architect, reproducing the dev team's suspicion:

1. `a.txt` is sealed on `heads/main`. The shell runs `mv a.txt b.txt`.
2. `worktree-status --format json` lists `missing a.txt` and `untracked b.txt`, with `"declarations": []`. This
   is what a preview would show.
3. Then `prikk mv a.txt b.txt` prints *"declared a.txt -> b.txt (already moved on disk; no bytes touched)"*.
4. `worktree-status` lists **the identical `changes`**, and `"declarations"` now holds `a.txt -> b.txt`.
5. `prikk commit` authors **`rename-path a.txt -> b.txt`**, one operation. The same tree committed without the
   declaration authors two, measured separately: `delete-file a.txt` and `create-file b.txt` in prikk's output,
   `delete-node` and `create-file` in its queue.

**stikk reads no declarations** (`parse_json.rs`: *"`declarations` is not read"*), so a `ChangesView` comparison
passes, and the commit authors an operation the preview never showed. **Decision 3 therefore compares prikk's
rename declarations too:**

- `WorktreeStatus` carries them — from the JSON `declarations` array at prikk ≥ 0.39, and from the `live rename
  declarations:` prose section at 0.38;
- below 0.38 the list is empty, **as a version fact**: `prikk mv` does not exist there, so none can be declared;
- `ChangesView` carries them through, so equality covers them.

**Showing declarations in the commit preview is not this RFC.** stikk's preview has never shown the rename a
declaration makes `commit` author, race or no race. That is a separate display gap, carried to the roadmap for its
own RFC.

### A2 — decision 5 gains a test

At prikk ≥ 0.38: the tree from A1's step 1 is previewed; `prikk mv` declares the rename; the test asserts
`worktree_status`'s entries are **unchanged** and its declarations differ; confirmation → `Stale`; nothing is
queued. **Below 0.38 the skip is announced.**

### A3 — decision 7: the stale words say what changed, and name no writer

The existing words — *"the repository changed since this was last previewed"* and *"Another writer moved something
in this repository"* — are **not true of a worktree change** (`C-T2b`). Refs, tags and the queue did not move,
and the writer is most often the user. *"Another writer"* is also doubtful for the causes decision 2 adds.

**`StikkError::Stale` carries a cause**, because only the check that failed knows which it was:

| Cause | Raised by | Headline, after the operation name | Gloss |
|---|---|---|---|
| **Repository** | the change-token comparison, in `confirm` and in `execute` | *the repository changed since this was last previewed.* | *Something in this repository changed between your preview and now: a branch or a tag, the queue, or, on prikk 0.42 and later, prikk's current branch. This is not a retry: previewing again re-reads the repository's current state, which is the only safe way forward.* |
| **Worktree** | commit's worktree re-read (decision 3) | *the worktree changed since this was last previewed.* | *What prikk reports about the worktree no longer matches what this preview listed: a path was added or removed, a path's status or prikk's verdict on it changed, or its rename declarations changed. Nothing was committed. Previewing again shows what a commit would author now, which is the only safe way forward.* |

- **The words live in `stikk-core`**, headline and gloss alike, so both frontends say the same thing. The TUI
  renders them.
- **A branch switch at ≥ 0.42 reads as Repository**, because the token check runs before the re-read. The
  Repository gloss names prikk's current branch, so the words are true of it.
- **The Display form** follows the cause: *"stale: {operation}'s preview no longer matches the repository"* or
  *"… the worktree"*.
- **Breaking, inside unreleased 0.7.0** (decision 6).

## Delivery

**One handoff**, issued on acceptance, ahead of RFC 029's Handoff B. **Reissued as v2** with the amendments
above, after its first review.

## What this RFC does not do

- **It does not block committing to a ref other than prikk's current branch.** That is legitimate, and RFC 029
  safeguard 3 names both refs on the confirmation instead.
- **No content digests**, because prikk reports none (F4).
- **No worktree check on seal**, because seal authors no worktree.
- **It does not close the last window** between the re-read and `prikk commit`.

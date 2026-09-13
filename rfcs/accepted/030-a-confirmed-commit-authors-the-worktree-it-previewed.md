# RFC 030 — A confirmed commit authors the worktree its preview showed

**Status.** **Accepted by the project owner 2026-09-13.** Proposed the same day by the architect, from RFC 029 F7. Scheduled **before** RFC 029's Handoff B,
because the risk is live for anyone running stikk against prikk 0.42 today. No open question.
**Tracks.** `OPL-02` (the change token), RFC 003 decision 3, RFC 013's confirm primitive, RFC 014, `FR-050`,
`FR-052`, `T-T4`, RFC 029 F7 and F8.
**Touches.** `stikk-prikk` (Orientation reads prikk's current branch), `stikk-model` (the change token), `stikk-core`
(commit's confirmation), `stikk-real-binary`.

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
  checkout, an added, deleted or reverted file.
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
   - **unresolved** — prikk's `<unresolved; run prikk doctor>`, carried verbatim;
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

## Delivery

**One handoff**, issued on acceptance, ahead of RFC 029's Handoff B.

## What this RFC does not do

- **It does not block committing to a ref other than prikk's current branch.** That is legitimate, and RFC 029
  safeguard 3 names both refs on the confirmation instead.
- **No content digests**, because prikk reports none (F4).
- **No worktree check on seal**, because seal authors no worktree.
- **It does not close the last window** between the re-read and `prikk commit`.

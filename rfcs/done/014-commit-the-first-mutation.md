# RFC 014 — Commit: the first mutation

**Status.** Implemented (0.4.0 candidate; on `main` 2026-09-06, reviewed and approved with no
corrections) — handoff:
[`../handoffs/014-commit-the-first-mutation/commit-handoff-v1.md`](../handoffs/014-commit-the-first-mutation/commit-handoff-v1.md).
Originally proposed 2026-09-05 — the first operation stikk performs that changes a repository, and
the first consumer of RFC 013's gate. Also closes the `capability_gate`/palette divergence RFC 013
deferred with a deadline of exactly this increment.
**Tracks.** `FR-050` (commit), `FL-05` (the flow), `UD-01`/`UD-06`/`UD-08` (message, whole-worktree,
ignore honesty), `OPL-04`'s seam-side half, and `C-D2a` (surface the limit before it is hit).
**Deferred, carried forward (not built by this RFC):** the **AUTHOR key id in the confirmation**
(`FL-05` step 5 / `FR-104`) — the id is public and *should* be shown, but reading it must not weaken
`env.rs`'s unqualified no-value-materialization guard, so it belongs in its own module and lands with
the seal ceremony's MAINTAINER key id; the **pre-commit `UD-01` copy**, now false for released prikk
0.32 (see below), corrected in the 0.32 re-baseline where 0.32 is actually validated; and
`StikkError::Declined`'s inline path, unreachable at tier 2 by construction and inherited by
tier-3-typed.

> **Superseded in part, 2026-09-06.** `UD-01`'s framing ("the message is validated, then discarded")
> is **falsified for prikk ≥ 0.32**: upstream RFC 123 landed message storage and removed the very
> `note:` line F4 quotes. Separately, upstream RFC 132 — prompted by
> `.git-exclude/upstream/001-commit-precondition-error-classes.md` and landed within a day —
> reclassifies both F2's and F6's messages to `precondition not met:`, unreleased as of the `0.32.0`
> tag. Neither costs this increment a follow-up: the classifier matches the messages' stable semantic
> part rather than their prefixes, and `CommitResult.notes` transports whatever prikk printed rather
> than assuming two fixed notes. Both are carried into the **prikk 0.32 re-baseline**.

**Touches.** `stikk-prikk` (a `commit` seam method — **the first that writes**), `stikk-model` (one new
error class), `stikk-core` (the commit operation and its preview), `stikk-tui` (the message input, and
the palette's tier-aware affordance).

## Summary

Everything stikk has shipped reads. This changes that. The gate is already built (RFC 013), so this
increment is about what commit specifically must be honest about — and checking prikk's real commit
surface turned up five things, two of which change the design.

## The findings

Verified against prikk 0.31 with probe repositories, not read from documentation.

### F1 — `--text-edits` is a no-op; stikk must not offer it

`prikk commit --from-worktree [--text-edits] …` accepts the flag, and it does nothing:
`worktree_patch.rs` documents `prefer_text_edits` as *"a no-op, see the field… retained for API
compatibility"*, and prikk's own guide says *"`--text-edits` is accepted for compatibility; text nodes
author `EditText` either way."* Offering it as a stikk option would be a control that changes nothing —
a small dishonesty of exactly the kind this project does not ship. **Do not surface it.**

### F2 — a cross-ref commit refuses, and prikk words it as a lock conflict *[changes the design]*

Committing to a ref other than the one the active WAL owns is refused. Reproduced:

```
$ prikk commit --from-worktree --ref heads/other -m "x"     # WAL owns heads/main
error: lock conflict: active WAL is owned by heads/main; requested ref heads/other   (exit 1)
```

The queue is unchanged afterwards — prikk fails closed, correctly. **But stikk's classifier matches
`"lock"` + `"conflict"` and files it as `LockConflict`**, whose whole meaning is *"another writer is
active"* (`FR-106`). No other writer exists. This is the **RFC 012 F-b defect shape again**: a message
classified into a bucket whose presentation does not fit — there, a version gate telling users to check
their signing keys.

It is latent rather than live today, because `present()` currently shows prikk's verbatim message in a
banner with no jump. It becomes a real confident-but-wrong picture the moment `FR-102`'s lock inspector
lands and `LockConflict` gains `jump: Some(LockInspector)` — offering a jump to an inspector that will
show no lock.

**And stikk can prevent it entirely rather than merely classify it well.** RFC 009 F1 added
`Orientation::queued_target` — the ref the queue belongs to — precisely because prikk reports it. So
stikk knows, *before* arming a commit, that the focused ref and the queue's target disagree.

### F3 — commit has no dry-run, so its preview is stikk's own (RFC 013 F1, Class B)

Recorded already; restated because it is this increment's obligation. The preview is the Changes view,
and it is **well-founded rather than a guess**: prikk's own guide says `worktree-status` *"answers
'what would the next commit author?', not merely 'what differs from the last seal.'"* — the same replay
baseline `commit` authors against. stikk may say so, and must say **whose** derivation it is.

### F4 — prikk now states the message's fate itself

`UD-01` still holds: the message is required, validated, and discarded. prikk's commit output now says
so in its own words — *"note: the message is validated but not stored -- it will not appear in
`prikk log`; persisting it is a later increment."* stikk should **transport that note** rather than
paraphrase it (`ER-02`), which is better copy than stikk's own and costs nothing.

### F5 — the active-patch thresholds are knowable before the commit, not only after

prikk warns at `PRIKK_ACTIVE_PATCH_WARN` (800) and refuses at `_LIMIT` (1000). stikk already reads the
queue count at every orientation. `C-D2a` is explicit that stikk *"surfaces the limit it is about to
hit, not just the failure after"* — so the warning belongs in the preview, not in the refusal.

### F6 — a clean-worktree commit also refuses, and also under an unrelated error class

Verified alongside F2. A commit with nothing to author is refused — exit 1, queue unchanged, fails
closed — with `error: invalid name: worktree has no node-addressed changes to commit`. Nothing about
it involves a name.

stikk's classifier matches none of its patterns, so it degrades to a verbatim `Refusal`, which is the
honest outcome and needs no fix. But F2 and F6 together are **the same shape twice** on the same
command: a precondition surfaced through whichever error variant sat nearest the call site, which
becomes the only signal a message-classifying consumer has. That pattern — not either instance — is
what we have written up for the prikk team
(`.git-exclude/upstream/001-commit-precondition-error-classes.md`).

## Decisions

1. **Refuse to arm a commit whose focused ref is not the queue's target** (F2), with a preview that
   explains it: the queue belongs to `<queued_target>`, you are focused on `<focused_ref>`, and the
   next steps that exist are to seal first, or to focus the queue's ref. Both ref names come from
   **stikk's own authoritative sources** — the intent's ref and `Orientation::queued_target` — never
   from parsing prikk's refusal (`C-T2b`).
2. **Classify the cross-ref refusal distinctly** if it reaches the seam anyway (a race: the queue moved
   between preview and commit). A new `StikkError` class, not `LockConflict` — RFC 012 F-b's lesson is
   that overloading a class to avoid adding one produces exactly this. The class carries prikk's
   verbatim message; the next-steps come from `(class, operation)`.
3. **The preview is the Changes view, labelled as stikk's derivation** (F3), citing prikk's own
   statement about what `worktree-status` answers. It carries the `UD-06` whole-worktree reminder and,
   when untracked entries are hidden, RFC 012's corrected `UD-08` caveat.
4. **Transport prikk's message-fate note verbatim** (F4) into the commit result, alongside the patch id
   and operation counts `FR-050` requires. Do not paraphrase and do not suppress it.
5. **Warn about the active-patch threshold in the preview** (F5), not after a refusal.
5b. **A clean worktree makes commit unavailable-with-a-reason, not offered-then-refused** (F6). stikk
   knows before arming anything; `C-T4d` requires the affordance be visibly disabled with its reason
   rather than failing on use.
6. **Do not offer `--text-edits`** (F1).
7. **Unify `capability_gate` and the palette's affordance check** — RFC 013 deferred this with a
   deadline of "before the first mutating command enters the palette registry", and commit *is* that
   command. The palette must grey out on the same tier-aware check `confirm` enforces, or it will offer
   what `confirm` refuses under read-only.
8. **The seam-side half of `OPL-04`'s double check lands here** — and is stated honestly. For AUTHOR
   readiness it is **constant within a process**: presence of `PRIKK_AUTHOR_SEED` cannot change
   underneath a running session. It is built because a future readiness source (a trust-policy read for
   MAINTAINER, `FR-104`) genuinely can change, and because the seam is the right place for it — not
   because it catches something today. Saying otherwise would be security theatre.

## Upstream dependency

**No new blocking one**, but one letter sent (F2 + F6): both commit preconditions are reported under
error classes that name a different condition, which matters because `UD-05`'s coarse exit codes leave
message text as a consumer's only classification signal. The constructive ask is a machine-readable
error surface on `commit`/`seal` — the `verify --format json` pattern — which would retire this class
rather than narrow it. Recorded at
`.git-exclude/upstream/001-commit-precondition-error-classes.md`; stikk is **not blocked** either way,
since the classifier degrades to prikk's verbatim words and F2 is preventable client-side.

Otherwise: no new one. `UD-01` (messages discarded) and `UD-06` (whole-worktree only) are surfaced honestly, as
they have been since 0.1.0. RFC 013's queued-patch enumeration ask stands for the *seal* ceremony and
is not needed here.

## Open questions — settled 2026-09-05

- ~~**What does stikk do when the worktree is clean?**~~ **Verified: prikk refuses** —
  `error: invalid name: worktree has no node-addressed changes to commit`, exit 1, queue unchanged,
  failing closed. stikk's decision is therefore only about *when to say so*: the preview knows before
  anything is armed, so commit is **unavailable with a reason** rather than offered and then refused
  (`C-T4d`). See F6 and decision 5b.

- **Does the commit preview re-read the worktree, or reuse the Changes view on screen?**
  **Ruled: re-read inside `preview()`.** RFC 013's token stamps the change token *at preview time*, so
  the preview and the stamp must describe the same instant. Reusing a Changes view the user loaded
  minutes ago while stamping freshness now would make the token assert something the view does not
  have — the confident-but-wrong picture the token exists to prevent. The cost is one extra
  `worktree-status` read at commit time, which is trivial and buys the token its meaning.

- ~~**Where does the message input live?**~~ **The design set already answered this and I should have
  checked before listing it as open.** `FL-05` steps 4 and 5 are explicit: the message prompt
  ("required non-empty, with the notice that core does not yet persist it") comes *first*, and the
  confirmation ("summarizes the whole-worktree capture and the AUTHOR key id") comes after. So the
  message is its own step **before** the confirmation, which also keeps `TU-09`'s confirmation a
  restatement rather than a form — a thing you are confirming should not change while you confirm it.
  Build `FL-05`'s ordering as written.

## Consequences

- stikk writes to a repository for the first time, through a gate that structurally cannot be bypassed.
- The `LockConflict` overload is caught **before** `FR-102` makes it a wrong picture rather than after.
- `queued_target`, added in 0.2.0 to make Orientation honest, turns out to be what lets stikk prevent
  a refusal rather than explain one — a read surface paying for itself in the mutation surface.

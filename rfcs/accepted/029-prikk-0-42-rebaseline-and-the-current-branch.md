# RFC 029 — The prikk 0.42 re-baseline, and prikk's current branch

**Status.** **Accepted by the project owner 2026-09-13; Q1 ruled (b)** the same day, with safeguard 3 and the
fallback folded into Handoff B. Handoff A landed (`9b0a6e4`, `0d3903e`, `7417738`); Handoff B is issued, after RFC 030 landed. Proposed the same day by the architect. Measured against real prikk
**0.28.0** and **0.42.0** binaries built from their tags, and by running stikk's own real-binary suite at both
ends with only the validated ceiling raised, in a scratch copy of `b1460cc`.
**Tracks.** `ASM-2`, `NFR-R03`, `FR-055`, `FR-002`, `TU-02`, `TU-03`, the requirements' terminology table,
RFC 027 decision 5, RFC 028.
**Touches.** `stikk-prikk` (the ceiling; Orientation's reading of `status`), `stikk-tui` (where focus starts; the
header), `stikk-real-binary`, the captured fixtures, and on acceptance `requirements.md` and `external-design.md`.

## Summary

prikk 0.42.0 shipped three things stikk asked for, in letters 010 and 011, and one it did not: **a current
branch**.

**Mechanically this is the smallest re-baseline stikk has had.** Its whole real-binary suite passes at 0.28 and
0.42 with nothing changed except the ceiling.

**The real change is in stikk's design, not its parsers.** stikk's requirements say prikk has *"no
current-branch pointer"*, and `FR-055` says *"no such thing exists"*. At 0.42 one does. Q1 decides how stikk's
client-side focused ref relates to it.

## Findings

### F1 — nothing breaks

Scratch copy of `b1460cc`, `VALIDATED_MAX_MINOR` set to 42, suite run with prikk 0.28.0 and 0.42.0:
**18 passed, 0 failed**. A `--nocapture` run confirms the ceiling fixture really was built by prikk 0.42.

It holds for four reasons:

- **stikk passes `--ref` explicitly** everywhere, so prikk's new default never applies to it.
- **Its JSON readers ignore fields they do not read**, and every 0.42 JSON addition is additive.
- **Its prose Orientation reader looks lines up by label**, so a new `current branch:` line is invisible to it.
- **The new prose lines are unindented** and sit outside `worktree-status`'s scoped entry region. `branch
  list`'s new `*` marker is prose that stikk reads only below 0.39.

### F2 — letter 010, answered in the binary

At 0.42 an `unsupported-path` entry — a backslash name, a non-UTF-8 name, one inside a subdirectory — is
`"authoring": "refused"`, with `commit`'s exact text, counted in `refused_count`, and its `path` is relative to
the worktree:

```
kind=unsupported-path authoring=refused path='back\\slash.txt' refusal='invalid name: backslashes are not allowed in repository paths'
kind=unsupported-path authoring=refused path='bad�name.txt'    refusal='invalid name: worktree path is not valid UTF-8: bad�name.txt'
```

**So RFC 027 decision 5 now blocks commit on them, with no stikk change** — RFC 027 Q1's ruling working as
designed. The would-refuse overlay's `U+FFFD` caution, which could not be reached before, now can be:
`bad�name.txt` is a refused path.

**Two tests encode 0.41's `authored` verdict**: `an_unsupported_path_prikk_marks_authored_does_not_block` in
`stikk-core/src/commit/tests.rs`, and an unsupported-path fixture in `parse_json/tests.rs`. Both are still true
of 0.41. 0.42's shape needs its own capture beside them, and each test's name should say which prikk it
describes.

### F3 — letter 011, answered in the binary

This finding is for RFC 028, which is amended with it.

- **Each queue entry now carries `message`.** `commit` requires `-m` at 0.42, so `null` means an older patch.
- **`show` renders a queued patch.** Every patch in `show-report-v1` carries `queued` — `true` or `false`,
  always present — and **no message**.
- **An id found nowhere** is `precondition not met: no object … in the object store or the active WAL`.

### F4 — prikk has a current branch

**The pointer.**
- It lives in `.prikk/current-branch`. `init` writes `heads/main`, and a repository without the file reads as
  `heads/main`.
- `--ref` defaults to it for `commit`, `seal`, `log`, `checkout`, `worktree-status` and the rest. An explicit
  `--ref` is unchanged.
- prikk calls it *"a default, never an authority"*: `verify`, trust, signing, `bundle` and `sync` do not read it.

**Switching.** `prikk branch switch heads/<name>` moves the worktree and the pointer. Measured, it refuses
three things, each as `precondition not met:`:
- a dirty tree;
- a closed branch;
- unsealed work queued for another branch.

**The reports.**
- `status`, `worktree-status` and `log` print `current branch:` and carry `current_branch` in their JSON.
- `branch-list-v1` entries carry `current`.

**When the pointer is malformed, or names a branch that is missing or closed:**
- the prose reads `current branch: <unresolved; run prikk doctor>`, and every JSON `current_branch` is `null`;
- the defaults refuse;
- an explicit `--ref` still works.

### F5 — stikk's design now says something false

- The requirements' terminology row: *"HEAD / checkout a branch — **does not exist.** No current-branch pointer"*.
- `FR-055`: *"the UI must never imply a HEAD moved (no such thing exists)"*.
- `app.rs`'s default-focus doc: *"prikk has no HEAD"*.

The first two remain true below 0.42.

### F6 — two defaults can disagree in silence

stikk opens focused on `heads/main`, always (`app.rs:261`).

At 0.42, a user who ran `prikk branch switch heads/dev` opens stikk focused on `heads/main`. stikk's commit then
targets `heads/main` explicitly, while prikk's own default is `heads/dev`. **Nothing is unsafe** — every stikk
call names its ref. But the two defaults disagree, and nothing on screen says so. This is the roadmap's
default-focus item, in a sharper form.

### F7 — a branch switch lets stikk commit another branch's files onto the focused ref

*(Measured 2026-09-13 on 0.42.0, answering the owner's question about data safety before ruling Q1.)* `heads/main`
holds `shared.txt` and `main-only.txt`; `heads/dev` holds a different `shared.txt` and `dev-only.txt`. stikk is
focused on `heads/main`, and the user runs `prikk branch switch heads/dev` in a terminal:

```
worktree-status --ref heads/main      current branch: heads/dev
                                      untracked dev-only.txt · missing main-only.txt · modified shared.txt
commit --ref heads/main -m …          recorded worktree patch in active WAL
                                      delete-file main-only.txt · create-file dev-only.txt · edit-text shared.txt
```

**prikk accepts it** — an explicit `--ref` is the authority, by prikk's design — **and stikk does not catch it.**
Before executing, `confirm::confirm` re-checks the capability and the change token, and the token composes refs,
tags and the queue. **Neither prikk's current branch nor the worktree is in it**, and `prikk commit` authors the
worktree as it is when the user confirms, not as the preview showed it. So a switch *between* preview and confirm
also passes.

The gap predates 0.42 for any worktree change between preview and confirm. `branch switch` makes it one command
that replaces the whole tree. **It exists under every option in Q1**: (a) leaves it silent, (b) makes it unlikely
at startup but not while stikk is open, (c) would have stikk move the worktree itself.

### F8 — prikk's pointer is not evidence of whose files are in the worktree

`prikk checkout --patch-materialize --ref heads/dev` wrote `dev-only.txt` and then refused to overwrite
`shared.txt` (`integrity error: refusing to overwrite existing file with different content`) — **and left
`.prikk/current-branch` on `heads/main`.** Only `init`, `setup` and `branch switch` write the pointer. So "focus
equals prikk's current branch" does not guarantee the worktree holds that branch's files, and no safeguard may
rest on the pointer alone. *(That the materialization stopped part-way, having written one file, is prikk's
behaviour and a question for prikk, not a stikk finding.)*

## Decisions

1. **The ceiling moves 41 → 42**, and the suite runs on the full matrix at 0.28 and 0.42. Fixtures are
   re-captured where 0.42's shape differs — the unsupported-path entries, and the new `current branch:` lines in
   prose stikk captures — and re-verified where it does not. The two claims are kept distinct, as in every
   re-baseline before.
2. **The 0.41-pinned unsupported-path tests stay, named for 0.41.** Beside them go 0.42 fixtures and a test that
   commit's preview blocks on a refused `unsupported-path`, with the `U+FFFD` caution rendered.
3. **stikk never reads `.prikk/current-branch` itself** (`CON-1`). prikk's reports are its only source.
4. **Whatever Q1 decides, stikk never calls prikk's current branch HEAD**, and never presents it as an authority.
   prikk itself says it is not one.
5. **On acceptance**, the terminology row and `FR-055` become version-accurate: no pointer below 0.42; from 0.42,
   a default pointer that is not HEAD.
6. **RFC 028's handoffs are issued after this re-baseline lands.**

## Open question

**Q1 — at prikk ≥ 0.42, how does stikk's focused ref relate to prikk's current branch?**

- **(a) Ignore it.** Focus stays client-side and starts at `heads/main`, and prikk's current branch is not shown.
  This is the cheapest option, and F6 stays silent.
- **(b) Start focus at prikk's current branch; keep switching client-side; show prikk's when the two differ.**
  - On open, stikk focuses what prikk reports.
  - The picker still moves stikk's focus without touching the worktree, so `FR-055` holds.
  - When focus and prikk's current branch differ, the header shows prikk's beside it, labelled as prikk's
    default.
  - With an unresolved pointer, stikk focuses `heads/main` and shows prikk's
    `<unresolved; run prikk doctor>` verbatim.
  - Below 0.42, nothing changes.

  This also settles the roadmap's default-focus item.
- **(c) Make stikk's focus prikk's current branch.** Switching focus runs `prikk branch switch`, which moves the
  worktree. That contradicts `FR-055`'s *"switching focus never touches the worktree"*, and a worktree-moving
  mutation needs its own tiered confirmation and its own RFC.

**My lean is (b).** stikk starts where prikk would, keeps the safety `FR-055` bought, and says out loud when
the two defaults part. A `branch switch` action can be its own RFC later, if it is wanted.

### RULED by the project owner, 2026-09-13: (b), with safeguard 3 and the fallback

**Handoff B builds this:**

- **At prikk ≥ 0.42, stikk opens focused on prikk's current branch**, read from the `status` report Orientation
  already makes — no new read, and stikk never reads `.prikk/current-branch` itself (decision 3).
- **Switching focus stays client-side** and never touches the worktree (`FR-055`).
- **When focus and prikk's current branch differ, the header shows prikk's beside it**, labelled as prikk's
  default, and it refreshes with every Orientation read, so it cannot go stale behind a terminal switch.
- **The fallback, at every prikk version.** stikk focuses `heads/main` only if it is published — Orientation's
  `heads/main RefState:` already says so. Otherwise no ref is focused until the user picks one, and the ref picker
  opens. An unresolved pointer shows prikk's `<unresolved; run prikk doctor>` verbatim. This also settles the
  roadmap's default-focus item.
- **Safeguard 3: at prikk ≥ 0.42, commit's and seal's confirmations name both refs when the target differs from
  prikk's current branch**, in stikk's words, as a notice rather than a block. prikk allows it, and deliberately
  committing to another branch is legitimate.
- **Never HEAD, never an authority** (decision 4).

**Recorded by the architect with Handoff B, 2026-09-15 — three readings of the ruling, and one correction:**

- **"The header" is the status bar.** stikk renders its focus in the status bar (`TU-03`), not the header, so
  prikk's current branch goes beside it there.
- **A branch prikk names is followed even when it is unpublished.** A fresh 0.42 repository reports `current branch:
  heads/main` beside `heads/main RefState: <not published>`, and prikk would author a first commit there. The
  fallback governs when prikk names no branch: below 0.42, or when the pointer is unresolved.
- **An empty ref picker offers `heads/main (not published)`.** Below 0.42 a new repository has nothing published,
  so the fallback would otherwise leave stikk unable to make a first commit.
- **Correction: the status bar can lag a terminal switch.** stikk reads Orientation on open, on `r`, and after a
  commit or seal. It does not poll, and `FR-106`'s notice is not wired. *"It cannot go stale"* was too strong.
  What protects a confirmation armed before a switch is RFC 030.

**Safeguards 1 and 2 are not this RFC's.** Putting prikk's current branch in the change token, and re-checking the
worktree when a commit is confirmed, answer F7 whichever way Q1 went, and change the confirmation primitive every
mutation relies on. They are **RFC 030**, scheduled before Handoff B, because the risk is live against prikk 0.42
today.

## Delivery

- **Handoff A — decisions 1–3 and 5.** The mechanical re-baseline. It does not depend on Q1, and RFC 028
  unblocks when it lands.
- **Handoff B — Q1's outcome, ruled (b).** Issued after A lands, and after RFC 030's safeguards.

## What this RFC does not do

- **No `branch switch` action in stikk.**
- **No reading of files inside `.prikk/`.**
- **Not the Queue view** — that is RFC 028.

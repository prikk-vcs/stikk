# RFC 044 — Branches and tags: create a branch, create a tag, close a branch

**Status.** **Done 2026-10-17**: Handoff A delivered on `main` (`88bb7d1`, `f938b58`, `126d4a5`); a **0.13.0** candidate. **Accepted by the project owner 2026-10-17.** **Proposed 2026-10-17** by the architect, as 0.13.0's increment 2 (`.git-exclude/release/0-13-0-scope.md`,
authorized). **No open question.**
Measured against a real prikk **0.52.0**, plus `--help` at **0.28.0** for the floor. Evidence:
`.git-exclude/reports/044-branches-and-tags/` (`measure044.sh`, `measure044-0.52.txt`).
**Tracks.** `FR-070`, `FR-071`, `FR-120/121` (preview-first, tiers), `TU-09`, `C-T4d`, `ER-02`, `C-T2b`; RFC 013 (the
confirmation machinery), RFC 016 (seal: the same tier and capability), RFC 036 (names), RFC 037 B (the current branch),
RFC 041 (the picker).
**Touches.** `stikk-prikk` (three mutating seam methods), `stikk-core` (three previews and executions), `stikk-tui` (the
ref picker, History, a name-entry prompt), `stikk-real-binary`, `requirements.md`, and the changelog.

## Summary

stikk shows branches and tags but cannot make one. **This RFC adds the three operations that write only refs**:
pointers, never the files in the worktree. **Create a branch** at a branch's or tag's tip, **create a tag** at any
block, and **close a branch**. All three go through the confirmation machinery commit and seal already use, at seal's
tier. Switching branch, which rewrites the worktree, is RFC 045's.

## Findings (measured at 0.52)

### F1 — the commands, and what they print

| Command | On success (exit 0) |
|---|---|
| `branch create heads/<n> [--from <ref>]` | `created branch heads/<n>`, `target block: <id>`, `RefState: <id>`, `update_seq: 1` |
| `tag create tags/<n> --target <ref\|block-id> [-m <msg>]` | `created tag tags/<n>`, `tag object: <id>`, `target block: <id>`, `RefState: <id>` |
| `branch close heads/<n>` | `closed branch heads/<n>`, `RefState: <id>`, *"nothing was reclaimed; the pointer, its history, and every object remain, and the branch is recoverable"* |

**Prose only; there is no `--format json` for these.** The result is read back from `branch list --format json` and
`tag list --format json`, which stikk already parses. A closed branch shows `closed: true`.

### F2 — what each refuses, verbatim

- **Nothing sealed yet**: *"ref heads/main does not exist in this repository"*, for both creates.
- **A name that exists**: *"branch heads/feature already exists"*; *"integrity error: tag tags/v1 already exists"*.
- **A missing prefix**: *"invalid name: ref feature2 is not a local branch ref; expected heads/<name>"* (and the tags
  equivalent).
- **`--from` a block id**: *"--from ref <id> does not resolve to a published ref"*. **A branch starts at a ref's tip,
  never at an arbitrary block.** `tag create --target` accepts a block id.
- **Closing a closed branch**: *"branch heads/from-ref is already closed"*. Closing a missing one gives *"does not
  exist"*.
- **No maintainer key**: all three refuse with *"invalid signature: maintainer signer public key does not match trusted
  key maintainer"*. **The author key is not needed.**
- **prikk accepts `heads/bad name`**, a space in a branch name.

### F3 — closing the current branch is allowed, and it strands the repository

With no queued work, `branch close heads/main` **succeeds while `heads/main` is the current branch**. After that, a
default-source `branch create` refuses: *".prikk/current-branch names heads/main, which is closed; run `prikk branch
switch heads/<name>` to a branch that exists and is open, or `prikk branch create` it"*. With queued work, prikk refuses
the close itself (*"it owns a non-empty active WAL; seal it before closing"*).

### F4 — the floor

prikk 0.28 has `branch create`, `branch close` and `tag create` (with names not yet prefixed), but **no current-branch
pointer**. stikk reads the current branch from prikk 0.42 (`CURRENT_BRANCH_FROM_MINOR`), and F3 needs it.

## Decisions

1. **Three seam methods, each one prikk call**: `create_branch(name, from_ref)`, `create_tag(name, target, message)`
   and `close_branch(name)`. They run the F1 commands with **no `--format json`**, which does not exist. **Exit 0 is
   success**, and prikk's printed lines are kept verbatim for the result screen. Every other exit is classified as
   today, with prikk's refusal verbatim (`ER-02`).
2. **Category `Publication`, so Tier Three and the publish capability**, exactly as seal is. A user without a ready
   maintainer key is stopped by stikk's own readiness check **before prikk runs**, with stikk's reason. F2's signature
   refusal is the fallback, never the first thing a user sees.
3. **Banded at prikk ≥ 0.42**, reusing `CURRENT_BRANCH_FROM_MINOR` and not a new literal. Below it, the three actions
   are unavailable with stikk's reason (`C-T4d`).
4. **Where each action lives:**
   - **The ref picker**: **create a branch from the selected ref** (a branch or tag tip, as `--from`), **close the
     selected branch**, and **tag the selected ref's tip**.
   - **History and Block detail**: **tag this block** (by block id).
   - **No "branch from this block"** anywhere (F2: prikk does not take a block). stikk does not imitate it by making a
     tag and branching from that.
   The keys are the handoff's to choose from the free ones, listed in the Glossary.
5. **The name is typed by the user**, with the prefix (`heads/` or `tags/`) shown fixed in front of it. **stikk adds no
   naming rules of its own**; prikk's refusal is shown verbatim. An empty name is not sent. In the preview, the name
   renders as an identifier (RFC 036), so `heads/bad name` is visibly quoted. **A tag's message** is optional and
   prompted after the name; an empty message sends no `-m`.
6. **The preview restates the facts before anything runs** (`TU-09`, `ConfirmationSummary`): the operation, the new
   name, the source or target (ref name and short block id, from data already read), the message if any, and for a
   close, prikk's own promise from F1 that nothing is reclaimed.
7. **Closing the current branch is not offered** (F3): disabled with stikk's reason, *"this is the branch you are on;
   closing it would leave the repository on a closed branch. Switch to another branch first."* Until RFC 045 lands, the
   reason names `prikk branch switch`. **A branch with queued commits** is left to prikk's own refusal, which is already
   clear (F3).
8. **After success**: prikk's printed lines are shown verbatim in the result screen, and the ref lists are re-read. A
   new ref appears in the picker; a closed branch moves to the closed rows that `--all` already marks.

## Changelog

`### Added`: create a branch, create a tag and close a branch, preview-first and with prikk's own words, at prikk ≥ 0.42.

## Delivery

**One handoff, issued on acceptance** ([Handoff A](../handoffs/044-branches-and-tags/a-branches-and-tags-handoff-v1.md)), **in two commits**: the seam methods with their tests, then the previews, prompts, keys and
views. It re-measures F1–F3 at 0.52 first. **The 0.42 floor is not measured** (the owner's rule: no effort on bands no user
is on). It is safe because every refusal is prikk's, shown verbatim, and success is read back from the JSON lists. An
older prikk can produce an error, never a wrong picture.

## What this RFC does not do

- **No branch switch** (RFC 045).
- **No deleting** a branch or tag: prikk has no delete, and closing reclaims nothing.
- **No branch from an arbitrary block.**

## Delivered

**One handoff, three commits, green on `main` at `126d4a5`**: CI, Docs, the real-binary suite (53 cases at 0.28.0 and
0.52.0), supply chain and `installers.yml`.

- **F3 re-measured on its own.** The committed `measure044.sh` could only reach the *queued* case of closing the current
  branch; `measure044-f3.sh` measures the empty-WAL case, which reproduces F3 exactly. Nothing in the RFC changed.
- **The seam**: three `Publication` methods sharing one helper; `--from` always; `-m` only for a message that is not blank
  (prikk refuses a blank one as a usage error, measured); one argument per name; prikk's lines kept verbatim, never
  parsed; `CURRENT_BRANCH_FROM_MINOR` promoted to `version.rs` rather than duplicated.
- **The previews** name no ref in stikk's unescaped prose (review v1 C1): every name is in `target_ids`, through the
  identifier rule. **The source's or target's block id is read fresh inside the preview** (`history(…, 1)`), because
  `branch list` reports a RefState id, not a block id (`measure044-c2-0.52.txt`), and an id from the picker's own read
  could never show a move. A close restates prikk's word, *recoverable*.
- **The current branch is refused in core**, before anything is spawned; the suite proves it on an empty WAL. Its reason
  names `prikk branch switch` until RFC 045 lands.
- **Beyond the handoff, accepted in review**: the shell banner wraps instead of clipping (RFC 024's rule), and `t` is
  declined on the unpublished current branch as `n` and `x` are.
- **Carried**: the Glossary's Keys section lacks `V`; the overlay clipping gate has no `CommitWouldRefuseDeclarations`
  case; one closure in `refs.rs` needs a hand re-indent.

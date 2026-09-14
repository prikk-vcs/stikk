# Handoff B — stikk opens on prikk's current branch (v1)

**Companion to:** [RFC 029](../../accepted/029-prikk-0-42-rebaseline-and-the-current-branch.md), Q1 ruled (b) with
safeguard 3 and the fallback.
**This handoff is the ruling:** where focus starts, how prikk's current branch is shown, the fallback, and
safeguard 3.
**Starting point:** `origin/main` with RFC 030 delivered. You build on `stikk_model::CurrentBranch` and
`OrientationView.current_branch` as RFC 030 shipped them, and on its `StaleCause`.
**Design items:** `FR-055`, `TU-03`, `FR-050`, `FR-052`, `C-T2a`, `C-T2b`, `C-T2c′`, `C-T4d`, `NFR-A03`.

> **One sentence to hold on to: stikk follows where prikk would start, and never moves prikk.** Nothing here runs
> `branch switch`, writes `.prikk/current-branch`, reads it, or calls anything HEAD. Focus is still stikk's own,
> and switching it still never touches the worktree (`FR-055`).

---

## 1. Scope

**In, in this order:**
1. **Focus has three states**, resolved once from the first Orientation read (§2).
2. **The status bar** shows prikk's current branch beside the focus when they differ (§3).
3. **The fallback and the empty picker** (§4).
4. **Safeguard 3:** commit's and seal's confirmations name both refs (§5).
5. **Tests** (§6), docs, changelog and Breaking (§7).

**Out:**
- a `branch switch` action in stikk;
- polling, or any new Orientation read (see §3's limit);
- wiring `FR-106`'s passive notice (roadmap item 12);
- the Changes view's words for an unpublished baseline (roadmap item 8's other half).

## 2. Focus: three states, resolved once

Today `App.focused_ref` is a `String` initialised to `DEFAULT_REF` (`heads/main`) before anything is read. It
becomes a small enum. Name it in the crate's idiom:

| State | Meaning |
|---|---|
| **pending** | nothing is focused yet, and the first Orientation read has not arrived |
| **unfocused** | the first read arrived, and no ref qualifies (§4); the ref picker has been opened |
| **a ref** | a ref name, which the user can change with the picker at any time |

**Resolution happens once, on the first successful Orientation read, and only while focus is still pending:**

| `current_branch` | `main_ref_state` | Focus becomes |
|---|---|---|
| a branch `b` (prikk ≥ 0.42) | either | **`b`** |
| unresolved (≥ 0.42) | `Some` (published) | `heads/main` |
| unresolved | `None` | **unfocused**, and the ref picker opens |
| not reported (below 0.42) | `Some` | `heads/main` |
| not reported | `None` | **unfocused**, and the ref picker opens |

**The rules around it:**
- **A later Orientation read never moves focus.** That includes `r`, the read after a commit or seal, and a read
  after the user picked a ref. Focus is the user's from the first read on (`FR-055`).
- **A pick made while focus is still pending wins.** If the user opens the picker and chooses before the first
  read arrives, that read resolves nothing.
- **If the first read fails**, focus stays pending. The picker still works, and `r` retries; the first
  *successful* read resolves.
- **A branch `b` is followed even when it is unpublished.** On a fresh 0.42 repository prikk reports `current
  branch: heads/main` beside `heads/main RefState: <not published>`, as `STATUS_QUEUED_0_42_FIXTURE` shows.
  prikk would author a first commit there, so stikk focuses it. **This is the architect's reading of the ruling**:
  the fallback governs when prikk names no branch. It is recorded on RFC 029.

**Without a focused ref** (pending or unfocused), every action that needs one is unavailable with a reason
(`C-T4d`) and dispatches nothing:
- History, Changes, commit and seal, from their keys and from the palette;
- Block detail is unreachable without History, so it needs nothing.

**The reason, exactly:** `No ref is focused. Press b to choose one.` It is shown as the existing banner for a
key, and as the palette entry's disabled reason. If the palette's width makes that line too long, as RFC 018
once found, **report it rather than shortening the words**.

`App::focused_ref()` returns the state, or `Option<&str>`, as reads best at its callers. That is breaking
(§7).

## 3. The status bar

The focus renders in the status bar (`status_bar.rs`), not the header, so **"beside the focus" means the status
bar**. The header is unchanged.

| Focus | Orientation's `current_branch` | Rendered where the focus is today |
|---|---|---|
| pending | — | nothing for focus; the existing `(loading)` or `(error)` stays |
| unfocused | any | `no ref focused`, in `warn` |
| a ref `r` | a branch equal to `r` | `r`, as today |
| a ref `r` | a branch `b` ≠ `r` | `r` · `prikk's default: b` |
| a ref `r` | unresolved `t` | `r` · `prikk's default: t` — `t` is prikk's text, verbatim |
| a ref `r` | not reported | `r`, as today |

- **The label is exactly `prikk's default: `**, as the ruling says. Never `HEAD`, `current` alone, or anything
  that reads as authority (decision 4).
- **Every name and prikk's text go through `inert`** (`C-T2a`).
- **It is computed from the loaded `OrientationView` on every render**, so it follows every Orientation read.

**The limit, stated.** The ruling says this *"refreshes with every Orientation read, so it cannot go stale behind
a terminal switch."* **The first half is true; the second is not.** stikk reads Orientation on open, on `r`, and
after a commit or seal. Between reads a terminal `prikk branch switch` is not seen, and `FR-106`'s notice is not
wired (roadmap item 12). What protects the user is RFC 030: a confirmation armed before the switch is stale. **No
stikk text may claim the status bar is live.** RFC 029 is amended to say this.

**Update `status_bar.rs`'s module doc**, which still says *"this increment shows the literal `heads/main`"*.

## 4. The fallback, and the empty picker

**When focus resolves to unfocused, the ref picker opens**, exactly as `b` opens it, and focus is set when the
user picks. Escaping the picker leaves focus unfocused, and `b` opens it again.

**The empty picker.** Today an empty ref list renders `no refs reported`, and nothing can be picked. That is the
state of a brand-new repository **below 0.42**, where nothing is published: the fallback leaves focus unfocused,
and the picker has nothing to offer, so stikk could no longer make a first commit. (At ≥ 0.42 a new repository
resolves to `heads/main` through §2's first row and never gets here.)

**So the empty picker offers one row:** `heads/main (not published)`, pickable. Picking it focuses `heads/main`.
- The dim line above it reads `no published refs`.
- **Only when the list is empty.** A repository with other branches does not get an unpublished `heads/main`
  offered, because that is the confusion roadmap item 8 describes.
- **The row states a fact** — nothing named `heads/main` is published — and makes no claim about what prikk
  would do.

## 5. Safeguard 3: confirmations name both refs

**At prikk ≥ 0.42**, when a commit's or seal's target differs from prikk's current branch, the confirmation says
so. **It is a notice, not a block**: committing to another branch is legitimate, and prikk allows it.

- **Computed in `stikk-core`, from the Orientation each preview already reads**, not from the TUI's possibly
  older copy. `commit::compute` and `seal::compute` both read it before building their summary.
- **`ConfirmationSummary` gains the notice**: an `Option` of stikk's words, `None` when there is nothing to say.

**The words, exactly** (`{target}` is the previewed ref; names and prikk's text rendered through `inert`):

| `current_branch` | Notice |
|---|---|
| a branch `b` ≠ target | `This targets {target}. prikk's current branch is {b}, the ref prikk uses when no --ref is given.` |
| a branch equal to the target | none |
| unresolved `t` | `This targets {target}. prikk reports its current branch as {t}.` |
| not reported (below 0.42) | none |

- **Where it renders.** On the confirmation card, **directly under the target ids, in `warn`**, with the text
  carrying the meaning in a monochrome terminal (`NFR-A03`). It goes above the counts and above `C-S2`'s
  published-example line, since it is about where the change goes.
- **Check the words against the binary before using them** (`C-T2b`). *"The ref prikk uses when no --ref is
  given"* is what RFC 029 measured of 0.42's `--ref` default. **Re-measure it**: a raw `prikk commit` without
  `--ref`, on a tree whose pointer names `heads/dev`, queues for `heads/dev`. If it does not, stop and report.

## 6. Tests

**`App`, through `from_state` and `apply`:**
1. The first read with branch `heads/dev` focuses `heads/dev`.
2. A second read with branch `heads/x` leaves focus on `heads/dev`, and the status bar shows `prikk's default:
   heads/x`.
3. Not reported with `heads/main` published focuses `heads/main`; with it unpublished, focus is unfocused and a
   `Refs` request is dispatched.
4. Unresolved: the same two outcomes, and the status bar shows prikk's text verbatim beside a focused
   `heads/main`.
5. A pick made while pending survives the first read.
6. A failed first read leaves focus pending; a later successful read resolves it.
7. With no focus, the History, Changes, commit and seal keys dispatch nothing and set exactly the reason in §2.
   The palette lists them disabled with that reason.
8. An empty `Refs` answer renders `no published refs` and a pickable `heads/main (not published)`, and picking it
   focuses `heads/main`. A non-empty answer offers no such row.

**Status-bar captures at 80 columns**, in `status_bar/tests.rs`'s style: equal (no label), differing, unresolved,
unfocused, and pending. Each asserts `HEAD` is absent.

**Core:** the notice for commit and for seal, in each row of §5's table, byte-exact. Plus one confirmation-card
capture at 80 columns with the branch notice.

**Real-binary suite:**
- **At 0.42:**
  1. Seal `heads/main`, create `heads/dev` from it, and run raw `prikk branch switch heads/dev`.
  2. `orient()` reports branch `heads/dev`.
  3. Add an untracked file; `commit_preview` on `heads/main` is Ready, and its summary's notice is §5's first
     row, byte-exact.
  4. Confirm the commit, which is legitimate. Then `seal_preview` on `heads/main` carries the same notice.
  5. Also §5's `--ref` default re-measurement: a raw `prikk commit` without `--ref` queues for `heads/dev`.
- **At 0.28:** not reported, and no notice on either preview.

## 7. Docs, changelog, Breaking

- **`FR-055`**: replace *"how stikk's focus relates to prikk's pointer is RFC 029 Q1"* with the ruling in
  substance. At ≥ 0.42 stikk opens on prikk's current branch. Below 0.42, or when prikk's is unresolved, it opens
  on `heads/main` only if published, and otherwise with no focus and the picker open. Switching stikk's focus
  still never moves prikk's, and never touches the worktree.
- **The terminology `HEAD` row**: the same replacement for *"how the two relate is RFC 029 Q1"*.
- **`TU-03`**: the status bar's focus segment gains `prikk's default: <branch>` when the two differ, and `no ref
  focused`.
- **The Glossary's HEAD entry** (shown to users) keeps what prikk has, and gains what stikk now does: from prikk
  0.42 stikk opens focused on prikk's current branch, and switching focus in stikk never moves prikk's.
- **`app.rs`'s `DEFAULT_REF` doc**, and the constant itself if it is now used only by the fallback and the empty
  picker, say so.
- **Changelog, `## Unreleased`, `### Changed`:**
  - on prikk ≥ 0.42, stikk opens focused on prikk's current branch, and the status bar shows prikk's default
    beside stikk's focus when they differ;
  - commit and seal confirmations name both refs when they differ;
  - below 0.42, or with an unresolved pointer, stikk opens on `heads/main` only when it is published, and
    otherwise opens the ref picker; an empty repository's picker offers `heads/main (not published)`.
- **Breaking, by API diff.** Expect `ConfirmationSummary`'s new field, and `App::focused_ref`'s return type if it is
  public API. Anything else, you find.

## 8. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite and
supply-chain run ids at one SHA, and Docs after the push.

## 9. Acceptance criteria

1. Focus has three states. Resolution happens once, on the first successful read, per §2's table; later reads
   never move it; a pick while pending wins; a branch is followed even when unpublished.
2. With no focus, every focus-needing action is unavailable with §2's exact reason, from keys and palette alike.
3. The status bar shows §3's table: `prikk's default: ` exactly, names and prikk's text inert, and no claim of
   liveness anywhere.
4. The fallback opens the picker; the empty picker offers only `heads/main (not published)`.
5. Safeguard 3 is computed in core from each preview's own Orientation read, with §5's words byte-exact and the
   `--ref` default re-measured.
6. §6's tests, captures and suite legs.
7. §7's docs, the Glossary, the changelog, and a Breaking table built by API diff.
8. Eight gates under the toolchain rule; run ids at one SHA.
9. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/029-b-current-branch/review-request-v1.md`.

**In this order:**
1. **The `--ref` default re-measurement** (§5), because the notice's words rest on it.
2. **The five status-bar captures.**
3. **The confirmation card with the notice.**
4. **The empty picker.**
5. **The Breaking table.**

**And tell me where the ruling, as this handoff reads it, surprised you in use:** opening a repository you know,
at 0.28 and at 0.42. Three readings above are the architect's:
- a branch is followed even when unpublished;
- the status bar rather than the header;
- the empty picker's row.

If one feels wrong at the keyboard, that is a finding.

**Push once approved.**

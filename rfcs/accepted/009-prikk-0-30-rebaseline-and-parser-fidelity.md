# RFC 009 — The prikk 0.30 re-baseline and the parser-fidelity defects

**Status.** Accepted (2026-09-04) — re-baseline stikk on **prikk 0.30**, and correct four
**parser-fidelity defects** found by running shipped stikk against the real binary. One of them
(**F1**) breaks the Orientation view — stikk's entry point — on *any* repository with queued work, at
*every* prikk version stikk claims to support. This is a correction increment, proposed ahead of
roadmap increment 6, and a **0.1.1 patch release** candidate. Handoff:
[`../handoffs/009-prikk-0-30-rebaseline-and-parser-fidelity/parser-fidelity-handoff-v1.md`](../handoffs/009-prikk-0-30-rebaseline-and-parser-fidelity/parser-fidelity-handoff-v1.md).
**Decision 6 was ruled by the owner on 2026-09-04: dropping prikk 0.27.x is accepted, and 0.30.0 is
the version to develop and validate against.** No decision in this RFC is now outstanding; all seven
are accepted and implementable.
**Tracks.** The standing obligation to re-validate against the real prikk (`ASM-2`, `NFR-R03`), and the
correction of `UD-05`/`UD-08` after prikk 0.28–0.30.
**Touches.** `stikk-prikk` (`cli_backend/parse.rs`, `classify.rs`, `version.rs`, and every golden
fixture), `stikk-core` (`changes.rs` view-model), `stikk-tui` (`view/changes.rs`), and the design set's
UD table. Nothing mutates; the seam's trait shape is unchanged.

## Summary

RFC 006 and RFC 008 each re-scoped an increment by checking the real `prikk` binary. This RFC applies
the same habit to work already shipped, and finds that **stikk 0.1.0 does not work against a real
repository**. The defects are not exotic: they are what happens when a golden fixture is *written*
rather than *captured*.

The re-baseline itself is the smaller half. prikk moved 0.28 → 0.30 while stikk shipped 0.1.0, and two
of stikk's recorded upstream dependencies have changed underneath it. The larger half is that three
parsers accept shapes prikk has never emitted, and one drops a warning prikk emits precisely so a
front-end cannot mislead a user into deleting real work.

Everything below was **verified by running prikk 0.30.0 and shipped stikk against probe repositories**
on 2026-09-04, not inferred from documentation. The reproductions are recorded so the next reader can
re-run them.

## The findings that scope this increment

### F1 — Orientation is broken on every non-empty repository *[blocking]*

`prikk status` reports the queue as `queued patches: <n> targeting <ref>`, not `queued patches: <n>`.
stikk's `parse::orientation` calls `required_u64` on that field, so it refuses:

```
$ stikk /path/to/repo            # one queued patch, prikk 0.30.0
error: environment: prikk field "queued patches:" is not a number: "1 targeting heads/main"
```

**This is not version drift.** `git log -S` in prikk dates the `targeting <ref>` suffix to prikk
**0.18.0**, and it is present in `0.27.1` — the audited baseline stikk was built against. Orientation,
the view every session opens with and the launcher's one-shot output, has therefore **never** worked
against a supported prikk on a repository that has been committed to. An empty repository parses
cleanly, which is why every demo, example, and manual check passed.

**Root cause — an invented golden fixture.** `parse/tests.rs::parses_queued_and_partial_counts` pins:

```
queued patches: 3
trailing partial WAL bytes: 7
heads/main RefState: <none>
```

prikk has never emitted either line in that form. The *clean* status fixture beside it is genuine
(a real 64-hex id, prikk's real trailing `status:` line); the queued variant was written by analogy
from `log`'s output and never captured. The handoff rule — "golden fixtures captured **verbatim** from
the real prikk" — was stated correctly in every handoff and not followed here, and review did not catch
it. The test suite is green *because of* the defect, not in spite of it.

### F2 — a sentinel is parsed as an object id

`prikk status` prints `heads/main RefState: <not published>` for an unpublished ref. stikk's
`optional_object_id` treats only the literal `<none>` as absent, so `OrientationView.main_ref_state`
becomes `Some("<not published>")` — a fabricated value in a field the data model types as an object id.
Verified against a freshly-`init`ed repository. prikk uses **two** sentinels (`<none>` in `log`'s
`previous-ref-state`, `<not published>` in `status`); stikk assumed one. It renders acceptably by luck
in today's one-shot print, and would not survive being abbreviated, copied (FR-124), or folded into a
change token (RFC 003).

### F3 — an empty repository yields a phantom ref

`prikk branch list --all` prints `no branches` when there are none (`tag list` prints `no tags`).
`parse::refs` accepts any line with two or more whitespace-separated tokens, so it produces
`RefEntry { name: "no", id: "branches" }`. It is the one parser that does **not** refuse on an
unrecognized shape — the others anchor on a required field, `refs` anchors on nothing.

### F4 — stikk drops the warning that exists to stop a user deleting real work *[serious]*

`worktree-status` emits an extra note when the active WAL holds queued patches for a **different** ref
than the one asked about. Reproduced exactly:

```
$ prikk commit --from-worktree -m "…"        # queues on heads/main
$ prikk worktree-status --ref heads/other
…
  untracked readme.txt — worktree file is not in the baseline
  untracked src/main.rs — worktree file is not in the baseline
note: the active WAL has queued (unsealed) patches for heads/main, not heads/other -- that is real,
committed work, not shown above; any "untracked" file here may be exactly that work seen from this
ref's own baseline, so do not delete based on this report alone (see `prikk status`)
```

stikk's parser reads only the counts, the headline, and indented entry lines; this flush-left `note:`
is discarded. The Changes view then presents those paths as ordinary untracked files and, if the user
hides them, adds stikk's own banner: **"a commit still captures them."** In this state that is
misleading in the opposite direction — the files are *already committed*, queued on another ref.

A user reading stikk's Changes view here can conclude that tracked-looking work is disposable junk.
That is `A-UND` harmed through a correct tool, the **T-T4** "confident-but-wrong picture" the threat
model names as the project's worst failure, and it is data-loss-adjacent. prikk did the honest thing
and stikk silently removed it — a direct **ER-02** (verbatim truth) breach.

This is not 0.30 drift either: prikk's 0.28 notes already record that `worktree-status` "says when the
active queue belongs to a different ref." RFC 008's fixtures missed it because the scratch repository
used to capture them never had queued work on another ref.

### F5 — `.prikkignore` shipped in prikk 0.29.0; **UD-08 is retired**

Verified: with `target` as a rule, `target/out.bin` leaves the untracked list entirely. It binds at
**discovery only** — `commit`'s worktree walk and `worktree-status`'s untracked scan, nothing else — so
it can never change what sealed history means. A malformed file **fails closed**, verified:

```
$ prikk worktree-status --ref heads/main      # .prikkignore line 1 = "/absolute/bad"
error: invalid name: .prikkignore line 1: invalid name: absolute paths are not allowed   (exit 1, stdout empty)
```

stikk degrades that correctly to a verbatim `Refusal` (the RR-5 path works), but offers next-steps
("Choose another ref", "Refresh") that cannot resolve it.

**One nuance worth stating precisely, because it bounds the fix:** prikk filters ignored paths *before*
reporting, so they never reach stikk. stikk's existing "a commit still captures them" banner is
therefore **not false** for what it displays. What is stale is the *rationale* — the requirement's
"prikk has no ignore mechanism" — and the absent surface, not the banner text.

### F6 — prikk's exit codes are now 0/1/2, and stikk misreads `2`

prikk 0.28 replaced the 0/1 collapse with `0` success, `1` operational failure, `2` **usage error**
(unknown/duplicate/malformed argument, detected before any repository work). `CliBackend::run` treats
every non-zero exit as a failure to classify by message text, so an exit `2` — which means **stikk
built a bad argument list**, i.e. a stikk bug — reaches the user as prikk's semantic `Refusal`.
`UD-05`'s premise is half-obsolete: the coarseness that justified message-classification is gone for
the usage case.

### F7 — the ceilings that still hold (re-verified at 0.30)

Confirmed unchanged, so the deferrals resting on them stand:

- **No `show`, `diff`, or `compare`; `checkout` is ref-tip-only** (no `--block`) — **UD-09** and RFC
  008's Compare deferral hold.
- **`log` has no `--format json`** — the UD-09 upstream ask stands.
- **Commit messages are still discarded** — **UD-01** holds, and prikk now says so itself in `commit`'s
  own output ("the message is validated but not stored -- it will not appear in `prikk log`"), which is
  better copy than stikk's for the commit increment.
- The **`log` and `worktree-status` report shapes are unchanged**, so those parsers and their captured
  fixtures remain valid.

## Decisions

1. **Capture, never write, a golden fixture.** Every fixture is copied verbatim from a real `prikk`
   run and carries a `captured at prikk <version>` provenance line. A hand-written fixture is a defect,
   not a shortcut — F1 is the proof. Re-capture **all** existing fixtures against 0.30 under this rule.
2. **Fix F1–F3 by anchoring each parser on a required shape and refusing everything else.** Concretely:
   `queued patches:` parses a leading integer and *preserves the `targeting <ref>` tail* as data (the
   queue's target ref is genuinely useful — it is what F4's warning is about); sentinels are an explicit
   set (`<none>`, `<not published>`, `<missing metadata>`, `<malformed metadata>`) that map to `None`,
   and **any other non-object-id value refuses** rather than passing through; `refs` anchors on prikk's
   real line shape and treats `no branches` / `no tags` as the empty list, refusing unrecognized text.
3. **Carry prikk's `queued_elsewhere` note verbatim into the Changes view** as a first-class warning
   band, above the entries (ER-02: stikk carries prikk's warning, it does not paraphrase it). While it
   is present, the UD-08 untracked filter's "a commit still captures them" banner is **suppressed and
   replaced** by prikk's warning — the two statements contradict each other, and prikk's is the true
   one. The seam surfaces it as a typed `Option<String>` on `WorktreeStatus`, not as free text the view
   re-derives.
4. **Retire UD-08 and stop claiming prikk has no ignore mechanism.** Scope for this increment is the
   honest minimum: correct the design set and the Changes copy, and add a glossary entry plus a
   next-step for the malformed-`.prikkignore` refusal (which is *not* "choose another ref"). A richer
   ignore surface — showing rules, or offering to add one — is a later increment, and stikk must not
   report a count of ignored paths, because prikk does not report one (T-T4).
5. **Revise UD-05 for the 0/1/2 contract and map exit `2` to `StikkError::Internal`** (fault screen:
   "the repository was not touched"). A stikk argument bug must never wear prikk's voice.
6. **Raise the validated range to prikk ≥ 0.28, validated through 0.30.0**, and state it per release
   (NFR-R03). 0.27.x is dropped because its `worktree-status` is the UD-03 defect and stikk already
   refuses to run it. **Ruled by the owner 2026-09-04:** accepted — 0.27.x is dropped and 0.30.0 is the
   development and validation target.

7. **An unbounded upper range is how this class of defect goes unnoticed, so give it a soft ceiling.**
   `is_supported()` currently returns true for *any* `0.x` at or above the floor, so a future prikk is
   silently declared supported — which is precisely the posture that let F1 survive three releases.
   prikk is pre-1.0 and has changed output shapes between minors twice in the window this RFC covers.
   Therefore: a prikk **above** the validated ceiling still runs (refusing it would break users the day
   prikk ships a minor), but stikk **says so** — Orientation states "validated through 0.30; this prikk
   is newer, and its output shapes have not been checked against stikk" — and the confined refusing
   parsers remain the real guard. This is the "where stikk cannot, stikk says so" stance applied to
   stikk's own knowledge of prikk, and it costs one comparison. *(Concretization within decision 6's
   scope, not a new product decision.)*

## Upstream dependency

**No new UD.** `UD-08` is **retired** (prikk 0.29.0); `UD-05` is **revised** (0/1/2 since 0.28);
`UD-01` and `UD-09` are **re-affirmed** against 0.30.

The standing ask is sharpened, because this is the third time parsing human output has cost real
correctness (UD-02, then RFC 006's scope, now F1–F4):

- **`--format json` on `status`, `log`, and `branch list`.** `verify` already has it. Each of F1, F2 and
  F3 is a direct consequence of its absence, and no amount of parser discipline removes the class —
  it only narrows it. This is now stikk's highest-value upstream ask, above the content surface.

## Open questions

- ~~**Does the owner accept dropping prikk 0.27.x?**~~ **Ruled 2026-09-04: yes.** The floor becomes
  **≥ 0.28**, validated through **0.30.0**. The owner also ruled **no yank of 0.1.0** — it was never in
  production use — with the correction shipping as **0.1.1**. Recorded here because the release
  disposition is what makes this RFC's severity actionable rather than alarming.
- **Should Orientation display the queue's target ref** (now that F1's fix parses it)? *Ruled yes* —
  it is the same fact F4's warning turns on, and showing "3 queued targeting heads/main" is strictly
  more honest than a bare count. It costs no extra call.
- **Should `status`'s active-patch threshold warnings be parsed now?** *Ruled deferred* to the commit
  increment, where FR-051's thresholds are the surface that consumes them.

## Consequences

- **stikk becomes usable against real repositories.** That sentence is the measure of this RFC.
- The seam grows no method and no category; `WorktreeStatus` gains one optional field, `Orientation`
  gains one, and three parsers get stricter. The trait shape, the layering, and the security
  invariants are untouched.
- The Changes view stops contradicting prikk in the one state where the contradiction could cost a
  user their work.
- **A process correction outlasts the code fix:** fixtures are captured, and the capture is proven by a
  provenance line a reviewer can check. This RFC is the evidence for why that rule is not ceremony.
- RFC 008 is amended by decision 3 (its untracked-filter copy) and by F5 (its UD-08 rationale); it stays
  `done/` with this RFC recorded as the amendment.

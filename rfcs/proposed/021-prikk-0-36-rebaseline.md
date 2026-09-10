# RFC 021 — The prikk 0.38 re-baseline: `UD-09` retires, and a fabricated worktree entry

**Status.** Proposed (2026-09-08). **Five** prikk releases land at once — **0.34 through 0.38** — against a validated ceiling of 0.33. **The largest re-baseline in this project's history**, and the first one that
retires the dependency stikk has carried since 0.1.0.
**Tracks.** `UD-09` (retires, in part), `FR-030`, `FR-033`, `FR-034`, `FR-051`, `FR-052`, `FR-103`,
`ASM-2`/`NFR-R03`, `C-T2c′`, `T-T4`, `TS-03`, `TS-07`.
**Touches.** `stikk-prikk` (version ceiling, new seam reads, fixtures), `docs/src/reference/`.
**Depends on.** [RFC 019](../done/019-the-real-binary-integration-suite.md)'s suite — this is its first
real job, and its version guard fixes the order: **raise the ceiling first, then run.**

## Summary

prikk answered both outstanding letters across three releases and shipped the content surface it had
previously ranked third with no date. **Everything below was verified against a real prikk 0.36.0
binary**, not read from the reply — the shapes, the exit codes, and both degraded states, each reached
through ordinary use.

| Release | What arrived | What it unblocks in stikk |
|---|---|---|
| **0.34.0** | `trust maintainer list` / `check --key-id`, `--format json`, `check` exits `0` either way | `MaintainerReadiness::Ready`; `FR-103` |
| **0.35.0** | `status --format json` (`status-report-v1`) with the queue enumerated; six preconditions reclassified | `FR-051`'s Queue view; `FR-052`'s consent naming *which* patches |
| **0.36.0** | **`prikk show <block-id\|patch-id> [--format json]`** (`show-report-v1`) | **`UD-09`'s content half — `FR-030`, `FR-034`'s per-file diffs** |

## F0 — 0.38 makes stikk fabricate a worktree entry *[live above the ceiling; verified end-to-end]*

**prikk read our parser, told us where it would break, and they were right.** 0.38.0's
`worktree-status` prints `live rename declarations: N` unconditionally, followed when `N > 0` by
indented `  <old> -> <new>` lines. Our entry scan takes **every** indented line in the document and
keeps it if its first whitespace-delimited token is one of `modified`/`missing`/`untracked`/
`unsupported`.

**A renamed path whose first token is one of those four therefore parses as a change entry.** Captured
from a real 0.38.0 binary after `prikk mv "modified draft.txt" renamed.txt`, then run through our own
`parse::worktree_status`:

```
prikk reported 2 changes; stikk reports 3 entries
  kind="missing"   path="modified draft.txt"     ← real
  kind="untracked" path="renamed.txt"            ← real
  kind="modified"  path="draft.txt -> renamed.txt"  ← FABRICATED
```

**A file that does not exist, in a state prikk never reported.** `T-T4`, manufactured by stikk out of
correct prikk output — the RFC 015 F4 shape again, and the third time this project has produced a wrong
picture from a right answer.

**It is reachable in shipped 0.4.1 today**, because stikk deliberately runs above its validated ceiling
and says so rather than refusing. It needs prikk ≥ 0.38, a path whose first token is a kind word, and a
`prikk mv` — narrow, but none of those is exotic.

**The root cause is not the kind list; it is that the entry scan has no section.** It reads indented
lines from the whole document rather than from the region that holds entries. Any future indented
section breaks it the same way. **Fix by scoping the scan to the entries region, not by adding `->` to a
reject list** — a heuristic against today's one collision would leave the next section to find.

*`worktree-status --format json` exists at 0.38 and reports declarations as a field. It is the right
long-term answer and it is **not** the fix here: our floor is 0.28, where it does not exist.*

## Findings

### F1 — `UD-09` retires, and it is the oldest thing on the board

Verified at 0.36.0. A sealed block's patches render their operations, paths, **and content**:

```json
{"kind": "edit-text", "paths": [{"path": "a.txt"}],
 "content": {"kind": "edit-text", "old_span_text": "", "replacement_text": " world"}}
```

`create-file` carries `{"kind": "text", "text": …}` plus `mode`. **This is the surface `FR-030` (Patch
detail) and `FR-034`'s per-file diffs have waited on since 0.1.0.**

**What does *not* arrive: `prikk diff`.** prikk states it as **a refusal, not a deferral** — comparing
two arbitrary points is a materially more expensive question they have declined to answer. See F3.

### F2 — spans are not lines, and rendering them as lines would be invention

The capture above is the whole edit: `old_span_text` is **empty** and `replacement_text` is `" world"`.
prikk's edits are **content-anchored spans**, not line ranges.

**A renderer that synthesizes hunks, line numbers, or `+`/`-` gutters from this would be manufacturing
structure prikk did not report** — `T-T4`, and the same shape as RFC 015 F4's patch-count discrepancy.
Whatever Patch detail renders, it renders a span replacing a span.

### F3 — `FR-033` asks for the one thing prikk refuses *[needs a ruling]*

`FR-033` reads: *"Compare two blocks (same or different refs)… each entry expandable to content diff.
This is the range-diff equivalent."* **That is arbitrary-point comparison**, and prikk's reply asks us
directly to say if we need it.

**The requirement splits cleanly, and only one half is now impossible:**

- **The state-level half is buildable.** Which paths were added / removed / content-changed /
  mode-changed between two points is **foldable from operations prikk reports** — every operation
  carries its kind and its paths. Folding reported facts is arithmetic, not invention.
- **The content half is not.** For a path changed by several blocks in the range, an A-vs-B content
  diff needs exactly the comparison prikk declined. **stikk can show each block's own spans; it cannot
  synthesize a combined before/after without inventing one.**

See Q1.

### F4 — `Ready` becomes reachable, and RFC 016's robustness claim gets its test

`trust maintainer check --key-id <ID>` answers adoption, exits `0` either way. `MaintainerReadiness`
has had a `Ready` variant no code could construct since RFC 016 — **it is now constructible at prikk
≥ 0.34, and `Unknown` becomes version-conditional rather than universal.**

**RFC 016 Q1 claimed this would resolve with no change to the type or the UI's shape.** That claim is
now testable, and this re-baseline should say plainly whether it held.

### F5 — the queue's thresholds arrive machine-readable

`status --format json`'s queue carries more than the reply mentioned — verified:

```json
{"count": 2, "target_ref": "heads/main", "target_ref_status": null,
 "threshold_status": "none", "warn_threshold": 800, "hard_limit": 1000, "patches": [...]}
```

**`FR-051` asks for "the active-patch warn/limit thresholds as they apply."** stikk currently carries
prikk's *prose* warning through the commit preview (RFC 014). These are the same facts as data.

### F6 — three degraded states, and two of them are ordinary *[the renderer's real work]*

**All three reached against the real binary**, not taken from the reply:

| State | Meaning | Reached by |
|---|---|---|
| `paths: [{"unresolved_node_id": …}]` | the node no longer resolves to a path | **edit a file, then delete it, without sealing between** |
| `content: {"kind": "unavailable", "blob_id": …}` | the pre-edit identity is correct but unbacked | **the same sequence, viewed after sealing** |
| `target_ref_status: "missing-metadata"` / `"malformed-metadata"` | **repository damage** — the WAL has records but cannot say whose | not reached; prikk names it |

**The first two are ordinary and exit `0`.** I produced both by editing a file and deleting it — the
most unremarkable sequence a user can perform.

**So the rule is prikk's, and stikk must not get it backwards: absence degrades, an error propagates.**
prikk shipped this wrong twice internally before settling it, and told us so. **A renderer that warns
about corruption on `unavailable` would cry wolf on a normal workflow**; one that stays silent on
`malformed-metadata` would hide real damage. `C-T2c′`'s three-valued discipline, in a third place.

### F7 — the six reclassified preconditions cost nothing, and the fixtures know it

0.35.0 moved all six from `lock conflict:` to `precondition not met:`, **text after the prefix
unchanged.** RFC 017 narrowed `is_lock_conflict` to four semantic clauses that name a real lock, so
**none of the six ever matched the prefix and none matches it now** — the classifier is unaffected.

**The fixtures are not.** `classify/tests.rs` encodes `lock conflict:` for these six; they are stale
captures of messages prikk no longer emits. Re-capture, do not re-word.

*This is the third consecutive re-baseline where matching the semantic clause rather than the class word
cost nothing to a change that would otherwise have broken the classifier.*

## Decisions

0. **Fix F0 before anything else, and separately.** It is the only finding that is wrong in a shipped
   release rather than merely unvalidated.
1. **Ceiling 33 → 38.** `VALIDATED_MAX_MINOR = 38`, after RFC 019's suite runs green at both ends —
   **ceiling first, then run**, as its version guard enforces. The suite's matrix becomes 0.28 and 0.38.
2. **`UD-09` retires its content half** and the entry records precisely what remains: no arbitrary-point
   comparison, and no `log --format json`.
3. **Absence degrades; an error propagates.** Adopt prikk's rule verbatim as stikk's rendering rule for
   `unavailable` and `unresolved_node_id`, and render `*-metadata` as damage pointing at `doctor`.
4. **Spans render as spans** (F2). No synthesized hunks, no line numbers, no gutters.
5. **This RFC re-baselines; it does not build the views.** Patch detail (`FR-030`), the Queue view
   (`FR-051`), three-valued `Ready` (`FR-103`), and whatever `FR-033` becomes are **each their own
   increment.** Three releases of unblocked surface is exactly the situation where one increment
   quietly becomes four.
6. **Amend `FR-051`, `FR-052`, `FR-103` and the `UD-09` row** to what is now knowable — as `FR-051`
   and `FR-052` were amended *downward* when the surface was absent. **The same discipline in the
   other direction.**

## Open question

**Q1 — what becomes of `FR-033`?** prikk has refused arbitrary-point comparison as out of scope, and
asked us to say whether we need it.

- **(a) Amend it to what F3 says is buildable** — state-level difference folded from reported
  operations, with each entry expandable to *that block's own spans*, never a synthesized A-vs-B diff.
- **(b) Withdraw it**, and tell prikk we do not need what they declined.
- **(c) Ask for arbitrary-point comparison anyway**, knowing they have called it materially expensive.

### Owner asked 2026-09-08 whether (a) is sufficient for production use. **Answer: no — in one case, and it is a common one.**

**Where (a) is complete**, and it is most of the view: *what did this block change?* (`show`, verbatim);
*which paths differ between A and B?* (folded from reported operations); *what did each block do to
this path?* (each block's own spans, in order).

**Where it breaks: a path changed more than once in the range.** The user gets N separate spans and has
to compose them in their head. `a.txt` edited in block 1, reverted in 3, edited again in 7 has a net
change of one line — and (a) shows three fragments and no answer. **In a range of any length, a file
touched repeatedly is the normal shape of development, not an edge case.** For a view whose entire
purpose is *what is different between these two points*, that is the central question, not a peripheral
one.

**stikk cannot close it itself, and should not try.** Reconstructing content by replaying spans means
reimplementing prikk's replay engine — `CON-1` (repository semantics are prikk's) and `T-T4` (a
one-byte disagreement would be a confidently wrong file). **prikk's own replay is incomplete today**:
`checkout --patch-plan` prints *"renames, conflicts, and full patch algebra remain later increments."*
Reimplementing an unfinished engine is worse than not having the view.

### But the ask is not `prikk diff` — it is much smaller, and this is the part worth taking upstream

**prikk already computes the answer and reports a byte count instead of the bytes.** Verified at 0.36.0:

```
$ prikk checkout --patch-plan --ref heads/main
blocks replayed: 2 · patches replayed: 4 · operations applied: 5
result files: 1 · result content bytes: 238
```

**The replay engine produces the state at a point. The CLI prints its size.** And `checkout` is the one
read command with **no `--format json`** — `status`, `show`, `verify`, `trust maintainer list` and
`check` all have one.

**So the request is: expose, read-only and machine-readable, the replay result prikk already computes.**
Content-at-a-point for a path. stikk would then diff **two contents prikk handed it** — that is
rendering, not inventing; the `T-T4` line is about fabricating facts, not about running a diff over
authoritative data.

Three properties make it a far likelier ask than `diff`:

1. **It exposes existing computation** rather than commissioning new computation — the objection prikk
   actually raised.
2. **It fits their own pattern**: a `--format json` read surface, on the one read command missing it.
3. **It commits them to no comparison semantics.** prikk owns content; stikk owns the comparison.

**Two things to say honestly if we ask:** their replay's own gaps (renames, conflicts, algebra) would
be inherited and must be named rather than papered over; and `--patch-materialize` is no substitute —
it *writes files*, which `C-E2` forbids stikk from using.

### Amended 2026-09-09 — prikk 0.37.0 shipped it, and it reaches less far than the reply believes

**`prikk checkout --patch-plan --format json --content-path <path> [--ref REF]` shipped**
(`patch-plan-content-v1`), exactly the framing letter 005 asked for. Verified at 0.37.0: `coverage`
carries `applied_operation_kinds` + `walk`, `not_found` is a list, absent paths degrade at exit `0`.

**But `--ref` resolves a published *branch* only, and I could not reach an older block by any route:**

| Attempt | Result |
|---|---|
| `--ref <block-id>` | `error: integrity error: ref <id> is not published` |
| `--ref tags/<name>` (tag created at that block) | `error: object type mismatch: expected block, got tag` — **exit 1** |
| `branch create --from <block-id>` | `error: --from ref <id> does not resolve to a published ref` |

**So "content at a point" is content at a branch tip.** `FR-033` splits differently than the reply
assumes:

- **Two branch tips** (`heads/main` vs `heads/feature`) — **fully answerable now.** Request the path at
  each, diff two authoritative contents.
- **Two blocks on one ref** — the range case, and the one `FR-033` names first — **not answerable, and
  there is no workaround.** Not even a mutating one: tagging the older block does not help, because
  `checkout` refuses to dereference a tag.

**The reply says the case we named "is now answerable: request that path at both points."** For the
case letter 005 actually described — a path edited across blocks *within a range on one ref* — both
"points" are blocks, not branch tips. **The claim does not hold there**, and we should not amend a
**[M]** requirement on the strength of it.

**Also found, and it is their own strongest kind of argument:** `tag create --target <ref|block>`
publishes a tag *at a block*, and `checkout` then refuses to read at that tag. prikk lets a caller name
a block and then declines to resolve the name it just created — an internal inconsistency, not a
request from us.

**Revised position: do not un-narrow yet.** Amend `FR-033` to *branch-tip comparison, fully; same-ref
range comparison, state-level plus per-block spans*, and carry block-addressability as the named
dependency. **Report the gap first** (letter 006) — the reply invited exactly this, and asked to know
before we amended rather than after.

*(Superseded lean, before 0.37.0 shipped:)* **(a) as the amendment, plus a narrow upstream ask — not a withdrawal.** Ship the half
that is buildable and honest now, and file content-at-a-point as a named dependency that would restore
the rest, the way `UD-09` itself was carried. **Do not ask for `diff`**: they have costed and refused it
with reasoning we would make ourselves.

*(Original lean, before probing the binary:)* **(a), and a letter saying so.** It keeps the half a user actually asks for — *what changed
between these two points* — and it is honest about the half that would require inventing a combined
diff. **(c) would be asking a project that has shipped every one of our last four requests to build the
one thing they have explicitly reasoned themselves out of**, on a requirement we wrote before their
content surface existed. But the requirement is the owner's, and withdrawing or narrowing a **[M]** is
not mine to do silently.

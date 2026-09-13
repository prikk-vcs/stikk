# RFC 028 — The Queue view: what a seal will freeze, and what prikk cannot yet say about it

**Status.** **Accepted by the project owner 2026-09-13, Q1 ruled (b)** — the work waits for the prikk release that
answers letter 011. Proposed the same day by the architect, 0.7.0's second increment. Every
finding below was measured against real prikk **0.28.0** and **0.41.0** binaries built from their tags the
same day.
**Tracks.** `FR-051`, `FR-052`, `FR-010`, `FR-030`, `TU-01`, `C-T2b`, `C-T2c′`, `ER-02`, `UD-02`, `ASM-2`,
RFC 014 decision 5, RFC 016.
**Touches.** `stikk-prikk` (a queue seam method and a `status-report-v1` reader), `stikk-core` (a queue
view-model, seal's read path, History's queued tier), `stikk-tui` (a Queue screen, the History tier),
`stikk-real-binary`, and on acceptance `requirements.md` and `external-design.md`.

## Summary

**prikk has enumerated its queue since 0.35. stikk still says only how many patches are queued, and for
which ref** — in Orientation, in History, and in the seal ceremony, which asks a maintainer to freeze patches
into signed history without naming one of them.

This RFC builds the **Queue view** that `TU-01` parked on 2026-09-06 for want of exactly that enumeration,
and lets **seal's confirmation name what it freezes**.

It also records **two limits that are prikk's to lift**, both asked in letter 011: a queued patch's message is
not reported, and `prikk show` refuses a queued patch as an integrity error. And **one defect that is stikk's**:
History puts the whole queue's count at the top of any ref's lineage, including a ref the queue does not
belong to.

## Findings

### F1 — the enumeration, measured

At 0.41, `status --format json` (`status-report-v1`, present from 0.35 by source) carries the queue. Two patches
committed to `heads/main`, the second touching two files:

```json
"queue": {
  "count": 2, "target_ref": "heads/main", "target_ref_status": null,
  "threshold_status": "none", "warn_threshold": 800, "hard_limit": 1000,
  "patches": [
    {"patch_id": "a7d52f31…", "operations": [{"kind": "create-file", "paths": [{"path": "a.txt"}]}]},
    {"patch_id": "ea9de5d2…", "operations": [{"kind": "create-file", "paths": [{"path": "b.txt"}]},
                                             {"kind": "edit-text",   "paths": [{"path": "a.txt"}]}]}
  ]
}
```

**The vocabulary, from prikk's emitter and its binary:**

| field | values |
|---|---|
| `target_ref_status` | `null`, `missing-metadata`, `malformed-metadata` |
| `threshold_status` | `null` for an empty queue; otherwise `none`, `warn`, `hard-limit` — with `warn_threshold` and `hard_limit` null only when the queue is empty |
| operation `kind` | `create-file`, `delete-node`, `edit-text`, `rename-path`, `change-perm`, `create-symlink`, `replace-binary` |
| a path | `{"path": …}` or `{"unresolved_node_id": …}` — a node no longer live in the baseline |
| `author_key_id` | on `rename-path` only, and always there: prikk cannot construct a rename without its asserting author |

`count` is `patches.len()` in prikk's emitter, so the two agree by construction.

**At 0.28 there is no JSON** — `error: unknown status argument: --format` — only the prose line
`queued patches: 2 targeting heads/main`.

### F2 — what a queued patch cannot say

**No message.** `QueuedPatchEntry` is `{patch_id, operations}` and nothing else. A patch committed with
`-m "queued with a message"` reports no message while queued; `log` shows it once sealed.

**No content.** `prikk show` refuses a queued patch, with an id taken from `status --format json` a moment
before:

```
error: integrity error: no object 8ca10f1dd47e3201874c99738795d79a9693d19a2d12e36973b6172c3f9bb7e8
```

**So a Queue view can show what each patch touches, not why, and not its content.** An absent message must
not read as *"this patch has no message"* (`C-T2c′`). Letter 011 asks prikk for the message, and whether the
integrity class is intended for a patch that is merely queued.

### F3 — the JSON report drops the threshold sentence

With `PRIKK_ACTIVE_PATCH_WARN=1`:

```
prose  warning: active patches (1) at or above the recommended threshold (1); consider running `prikk seal`
json   "threshold_status": "warn", "warn_threshold": 1, "hard_limit": 5
```

RFC 014 decision 5 carries prikk's sentence verbatim into the commit preview, from the prose `status` read
Orientation makes. **Moving Orientation to JSON would drop it** — RFC 027 F6's shape again. So Orientation stays
on prose, and the Queue view makes its own read.

### F4 — seal's confirmation names nothing

Seal's `compute` takes the count and target from `orientation` and confirms `("patches", N)`. `FR-052` was
re-amended on 2026-09-12 (RFC 021 §5): naming *which* patches has been satisfiable since 0.35.

### F5 — History shows the whole queue on any ref's lineage

`history_view` reads `orientation.queued_patches` with no check against `queued_target`, and History renders
it as the ref's "not yet history" tier:

```
  queued   2 patch(es) in the active WAL — not yet sealed
```

**Open History on `heads/other` while two patches are queued for `heads/main`, and that line tops
`heads/other`'s lineage.** The sentence is true of the WAL and wrong about the ref it sits on. Orientation
already carries `queued_target`, so the fix needs no new read.

## Decisions

1. **One seam method, `queue`, reading `status --format json` at ≥ 0.39** under `reads_json`. Below 0.39 the
   count and target come from the prose read Orientation already makes, and the enumeration is *unreported*.
   - The schema name is checked first; `count` must equal the patches listed, or it is a schema error.
   - `target_ref` is validated as a `RefName` when present; `patch_id` as an `ObjectId`.
   - `target_ref_status` and `threshold_status` are held to their vocabularies.
   - An operation's `kind` is kept as text, so a future kind renders — RFC 027 A's lesson.
   - Paths are carried as reported, and an unresolved node is labelled as one.
   - A report that parses as JSON but breaks a rule is stikk's environment error (RFC 027 B, C1).

   **Why 0.39 and not 0.35:** 0.39 is the JSON band stikk already reads and the suite's ceiling binary
   exercises. stikk has never measured `status-report-v1` at 0.35–0.38, and a band nobody runs is a band nobody
   tests. **What this gives up:** on 0.35–0.38 a user sees the count, not the list.

2. **The Queue view** — `TU-01`'s Queue row, built:
   - the target ref, or prikk's metadata status named when there is none;
   - the depth, and the thresholds as they apply, in stikk's words with prikk's numbers;
   - each patch's id and its operations: the kind word verbatim, paths inert, a rename as *from → to* with its
     asserting key id, an unresolved node shown as one.

   It says **once and plainly** that prikk does not report a queued patch's message or content, and that both
   appear in History once sealed. **Below 0.39** it shows the depth and target and one line saying this prikk
   does not enumerate queued patches — never an empty list. An empty queue says nothing is queued. It is
   reachable from the palette (`TU-07`) and from History's tier.

3. **Orientation stays on prose `status`** (F3). RFC 014 decision 5's verbatim warning is untouched; the Queue
   view may show threshold numbers, and the sentence stays prikk's, on the commit preview.

4. **Seal's confirmation names what it freezes, at ≥ 0.39** (`FR-052`). The count, the target and the list come
   from **one** queue read inside seal's `compute`, not Orientation plus a second read, so the count and the
   list cannot describe two different moments. Below 0.39, unchanged.

5. **History's tier claims the queue only when it is this ref's** (F5). Otherwise it says the active queue
   belongs to that other ref, and how many patches it holds. Built on the Orientation read History already makes.

6. **The suite drives it at both ends**, with two queued patches including a `prikk mv` rename:
   - at 0.41 the queue's patch ids **equal the ids `log` reports for the block after sealing** — a queued patch
     is the patch that seals, asserted rather than assumed — and the rename carries both paths and its key id;
   - at 0.28 the enumeration is unreported and the count equals prose. `prikk mv` does not exist below 0.33, so
     the rename case is an announced skip there, in the suite's existing idiom;
   - F5 at both ends: a queue for `heads/main`, and History for another ref that does not claim it.

7. **Breaking, inside the release that is already breaking.** `Prikk` gains a method and two view-models change.
   0.7.0 is unreleased and already the breaking position, so there is no further bump.

8. **On delivery** — not on acceptance, since nothing is built until prikk's release — `FR-051` records the Queue view as built, `FR-052` records that the ceremony names the
   patches at ≥ 0.39, and `TU-01`'s Queue row is un-parked.

## Open question

**Q1 — ship the Queue view without messages, or wait for prikk's answer to letter 011?**

- **(a) Ship now, stating the gap** (decision 2). Which files, which renames, by which key — what a seal
  freezes is what `FR-052` asks for. The message is the part a maintainer would read first, and it is missing,
  and the view says so.
- **(b) Wait for prikk.** The view arrives complete, later, on a schedule that is prikk's.
- **(c) Ship decisions 4 and 5 now** — seal names its patches, History stops misattributing — and hold the view.

**My lean is (a).** The enumeration has existed since 0.35, and stikk's ceremony still names nothing. If prikk
says yes, the message arrives as an additive field and the view gains a line. **It is yours because it is a
schedule call that rests on another project's answer.**

### RULED by the project owner, 2026-09-13: (b) — and prikk has already answered

**The Queue view waits for prikk.** prikk's reply 012 to letter 011 came the same day:

- **A queued entry gains `message`** — `null` exactly where `log` would show none, additive within
  `status-report-v1`.
- **`show` renders a queued patch** from the active WAL, as the same `show-report-v1` with `"queued": true`.
- **An id that resolves nowhere becomes `precondition not met:`**, naming both places prikk looked. An object a
  ref names and the store lacks stays an integrity error.

prikk's `main` already carries it (`93eb49ea`); its latest tag is still 0.41.0.

**What this changes.** F2's two limits lift in prikk's next release. When that release is published, stikk
re-baselines to it, **measures the message and a queued `show` on the binary**, and revises decision 2 so the view
shows each patch's message where prikk reports one. How far the view goes with a queued patch's content through
`show` is decided then, against the measured report, not from this letter. Below that release, decision 2's
statement of what prikk does not report stands as written.

**Held with it.** Option (c) was the one that shipped decisions 4 and 5 first, so under (b) they wait too:
seal naming what it freezes, and History's tier (F5). **F5 is stikk's own defect and depends on nothing from
prikk**; it can be split out and delivered alone on the owner's word.

## Delivery

**Both handoffs are issued after prikk's next release is published and stikk has re-baselined to it** (Q1
ruled (b)).

- **Handoff A** — decisions 1, 2, 3, 5 and 6: the reader, the seam method, the Queue view, History's tier and
  the suite.
- **Handoff B** — decision 4: seal names what it freezes. It is separate because it changes a signing
  confirmation (RFC 016), which deserves its own review rather than a paragraph inside a view.

## What this RFC does not do

- **No content or Patch detail for a queued patch** — prikk cannot show one (F2). `FR-030` stays its own
  increment.
- **No messages** until prikk reports them (letter 011).
- **No seal action in the Queue view.** Sealing happens in the ceremony.
- **Orientation does not move to JSON** (F3).
- **No enumeration on prikk 0.35–0.38** (decision 1).

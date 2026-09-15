# Handoff A — the Queue view, and History's tier (v1)

**Companion to:** [RFC 028](../../done/028-the-queue-view.md). Accepted 2026-09-13, Q1 ruled (b); decision 2
revised at prikk 0.42.0.
**This handoff is decisions 1, 2, 3, 5 and 6:**
- the reader and its seam method;
- the Queue view;
- Orientation staying on prose;
- History's tier;
- the suite.

**Handoff B**, in which seal names what it freezes (decision 4), follows this one.
**Sequencing:** **after RFC 029 Handoff B lands.** Both change `stikk-tui`'s `app.rs` and the palette registry,
and B also changes seal's confirmation summary, which this handoff's Handoff B touches next.
**Design items:** `FR-051`, `FR-010`, `TU-01`, `TU-07`, `C-T2a`, `C-T2b`, `C-T2c′`, `ER-02`, `UD-02`, `ASM-2`.

> **The shape of the work: prikk has enumerated its queue since 0.35, and stikk has only ever said how many.**
> This handoff reads the enumeration at ≥ 0.39, shows it, and stops History from claiming another ref's
> queue. **Every absence stays distinct from a zero, a `null`, or an empty list** — the reader is where that is
> won or lost.

---

## 1. Scope

**In, in this order:**
1. Captures (§2).
2. The reader and the seam method (§3).
3. The core view-model (§4).
4. History's tier (§5).
5. The Queue screen (§6).
6. The suite (§7).
7. Docs, changelog and Breaking (§8).

**Out:**
- seal naming the patches (Handoff B);
- a queued patch's content (Patch detail, `FR-030`, its own increment);
- a seal action in the Queue view;
- moving Orientation to JSON (decision 3);
- enumeration at prikk 0.35–0.38 (decision 1).

## 2. Captures — measured by the architect at prikk 0.42.0; verify each, do not copy

`prikk status --format json` is `status-report-v1`. Its top-level keys at 0.42 are `active_wal_records`,
`current_branch`, `heads_main_ref_state`, `queue`, `repository`, `schema_version` and `trailing_partial_wal_bytes`.
The queue, with two patches committed onto a sealed `heads/main` (`add b`, then `prikk mv a.txt c.txt` and
`rename a to c`):

```json
"queue": {
  "count": 2, "target_ref": "heads/main", "target_ref_status": null,
  "threshold_status": "none", "warn_threshold": 800, "hard_limit": 1000,
  "patches": [
    {"patch_id": "cfc488a1…", "message": "add b",
     "operations": [{"kind": "create-file", "paths": [{"path": "b.txt"}]}]},
    {"patch_id": "34ed9e61…", "message": "rename a to c",
     "operations": [{"kind": "rename-path", "paths": [{"path": "a.txt"}, {"path": "c.txt"}],
                     "author_key_id": "author"}]}
  ]
}
```

- **`message` follows `patch_id`**, and is absent below 0.42.
- **`prikk commit` requires `-m` at 0.42**: `error: commit requires -m <message> …`. A `null` message therefore
  means a patch an older prikk committed.

**Capture, with provenance, from a neutral directory:**
1. **The two-patch queue above**, at 0.42.
2. **The unresolved-node case at 0.42**, using `STATUS_JSON_UNRESOLVED_NODE_0_38_FIXTURE`'s own recipe: commit and
   seal `doc.txt`, edit and commit, delete and commit. It should show one operation carrying
   `unresolved_node_id`, now with messages.
3. **The same two-patch queue at 0.41.0**, with no `message` field. **Install 0.41.0 for it.** The 0.38 capture
   has the right shape but sits below the band this reader serves, and the absent state deserves a capture from
   the band it describes.
4. **A `warn` threshold at 0.42**, with `PRIKK_ACTIVE_PATCH_WARN=1` (RFC 028 F3's recipe).
5. **A `null` message**, only if you can produce one honestly — a queue written by 0.41 and read by 0.42. If you
   cannot, say so. **Do not hand-edit a capture to make one**; that case is then tested with a literal marked as
   constructed, and named as such.

## 3. The reader and the seam method (decision 1)

**`Prikk::queue(&self, repo) -> Result<…>`**, read from `status --format json` when `reads_json()` holds, which is
at ≥ 0.39.

**Below 0.39 it does not spawn anything new.** It answers from the prose Orientation read with the count and
target, and **says the enumeration is unreported**. Make that a distinct variant, not an empty list.

**The model**, in `stikk-prikk`, named in the crate's idiom. It must be able to hold every state below
distinctly:

| Field | States |
|---|---|
| target | a `RefName`, or absent with prikk's `target_ref_status`: `missing-metadata` or `malformed-metadata`; `null` status with a present target |
| threshold | absent for an empty queue; otherwise `none`, `warn` or `hard-limit`, with `warn_threshold` and `hard_limit` |
| patch id | an `ObjectId` |
| message | **not reported** (field absent, prikk < 0.42) · **none** (`null`) · **text** |
| operation | `kind` as text, kept verbatim so a future kind still renders; paths; `author_key_id` |
| path | a path as reported, **or** an unresolved node id — exactly one of the two |

**Rules. Breaking any one is stikk's environment error, never prikk's refusal (RFC 027 B, C1):**
- The schema is checked first, with `report(text, "status-report-v1")`.
- **`count` equals the patches listed.**
- `target_ref` parses as a `RefName` when present; `target_ref_status` and `threshold_status` are held to their
  vocabularies.
- **`threshold_status` is `null` exactly when the queue is empty**, and `warn_threshold`/`hard_limit` are `null`
  only then.
- `patch_id` parses as an `ObjectId`.
- **Each path object carries exactly one of `path` and `unresolved_node_id`.**
- **`rename-path` carries `author_key_id`** (prikk cannot construct one without it). On any other kind, carry it
  if present.
- **The message, by version, passed in as RFC 030 did for Orientation:**
  - at ≥ 0.42 the field must be present, as `null` or a string;
  - below 0.42 it must be absent, and reads as not reported.

  **A message present below 0.42 is also an error**, since it is a shape stikk has not seen at that version.

**`NullBackend`** gains a knob in its `with_*` style, and defaults to an empty, unreported-safe answer. Say which
you chose.

**Tests:** each §2 capture parses to what it shows, and each rule above has one failing case.

## 4. The core view-model

- **`stikk_core::queue_view(prikk, repo)`** returns a view-model the screen renders without further decisions.
- **Its words are in core**, so a GUI says the same. §6 gives them.
- **It makes one `queue` read.** At ≥ 0.39 the count and the list come from that one report, so they describe the
  same moment.

## 5. History's tier (decision 5)

**Today** `history_view` puts `orientation.queued_patches` on any ref's lineage (RFC 028 F5).

**`HistoryView` gains the queue's target** from the same Orientation read, and the tier says whose queue it is:

| Queue | Tier line, exactly |
|---|---|
| empty | no line, as today |
| this ref's | `{n} patch(es) in the active WAL — not yet sealed · Q: Queue` |
| another ref's | `the active WAL holds {n} patch(es) for {target} — not this ref's history · Q: Queue` |
| a count with no target reported | `the active WAL holds {n} patch(es); prikk reports no target ref · Q: Queue` |

`{target}` is rendered through `inert`.

**Check the third row against prose `status`** before relying on it. If prikk's prose never reports a count
without a target, **say so, and the row stays as a guard, not as a claim you measured.**

## 6. The Queue screen (decision 2, as revised)

- **`Screen::Queue`**, opened by **`Q`** and by a palette entry: `view.queue`, *"Open Queue"*, binding `Q`, tier
  one, opening `Target::Queue`.
- **The queue is repository-wide, so it needs no focused ref.**
- **Refreshed by `r`** like the other screens, keeping its view visible while the refresh is in flight (RFC 010
  §5).

**What it shows, in this order, with these words** (numbers and names are prikk's; every name inert):

1. **The heading:**
   - with a target: `{n} patch(es) queued for {target}`;
   - with none: `{n} patch(es) queued; prikk reports the target ref's metadata as missing` — or `malformed`;
   - empty: `Nothing is queued.`, and nothing else below.
2. **The thresholds**, when present:
   `warning at {warn} · hard limit at {hard}`, then `below the warning threshold`, `at or above the warning
   threshold`, or `at or above the hard limit`.
   **Do not add what prikk does at the limit**; that is not measured here.
3. **Each patch:**
   - its id, full, then its message line:
     - text: the message, first line only if it has several, with `…` marking the rest;
     - none: `(no message)`;
     - not reported: no line (item 4 says why once).
   - Then each operation, indented:
     - `{kind} {path}` for a path;
     - `rename-path {from} → {to} · asserted by {key id}` for a rename;
     - `{kind} unresolved node {id} (no longer in the baseline)` for an unresolved node.
4. **Once, at the foot**, whichever applies:
   - at 0.39–0.41: `prikk 0.{minor} does not report a queued patch's message or content; both appear in History once
     it is sealed.`
   - at ≥ 0.42: `A queued patch's content is not shown here; stikk's Patch detail view is not built yet.`
5. **Below 0.39**, instead of items 2–4:
   `{n} patch(es) queued for {target}`, then `prikk 0.{minor} does not list queued patches.` — **never an empty
   list.**

**Captures at 80 columns:** 0.42 two-patch with the rename; 0.41 without messages; 0.42 unresolved; a `warn`
threshold; below 0.39; empty.

**The id is 64 characters, and with the indent it fits 80.** If a message or path does not, **wrap, do not clip**
(RFC 024). Report any row that does not fit.

## 7. The suite (decision 6)

**Corrections to RFC 028 decision 6, recorded on the RFC:**
- **The ceiling is now 0.42, not 0.41.**
- **`prikk mv` does not exist below 0.38**, as the suite's existing F0 skip already announces — not 0.33.

**The tests:**
1. **At 0.42, two queued patches:** one ordinary, and one a `prikk mv` rename, each committed with a message.
   - The queue's patch ids **equal the ids `log` reports for the block after sealing**. A queued patch is the
     patch that seals, asserted rather than assumed.
   - The rename carries both paths and its key id.
   - Each message equals what was committed.
2. **At 0.28:** the enumeration is unreported, and the count equals Orientation's. The rename case is an
   **announced skip** below 0.38.
3. **F5 at both ends:** a queue for `heads/main`, and History for a second published ref that does **not** claim
   it. That ref's tier reads as another ref's queue.

## 8. Docs, changelog, Breaking

**On delivery (decision 8):**
- **`FR-051`** records the Queue view as built, at ≥ 0.39, with what it does not show;
- **`TU-01`'s Queue row** is un-parked;
- **`FL-06`'s note** that *"there is no dedicated Queue view yet"* is corrected;
- **the view list in `external-design.md` §6** gains Queue.

**`FR-052` is Handoff B's.**

**Changelog, `## Unreleased`:**
- **`### Added`**: the Queue view, which lists each queued patch's operations and, on prikk ≥ 0.42, its message
  (`Q`, or the palette).
- **`### Fixed`**: History no longer shows another ref's queued patches as this ref's "not yet sealed" tier.

**Breaking, by API diff.** Expect at least:
- `Prikk::queue`, breaking for any implementor;
- `HistoryView`'s new field;
- the new types and their re-exports.

## 9. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule: gates 1–5, 7 and 8 on the MSRV;
gate 6 on stable with a fresh `CARGO_TARGET_DIR`. **The suite on the full matrix.** Name the `CI`, suite and
supply-chain run ids at one SHA, and Docs after the push.

## 10. Acceptance criteria

1. **§2's captures**, with provenance, and a `null` message either captured honestly or tested as a constructed
   literal named as such.
2. **`Prikk::queue`:** reads `status --format json` at ≥ 0.39 and answers "unreported" below it with no new
   spawn. Every §3 rule is enforced as stikk's environment error, with a failing test for each. The message's
   three states are distinct, by version.
3. **`queue_view`:** one read, words in core.
4. **History's tier:** §5's table exactly; the third row measured or marked as a guard.
5. **The Queue screen:** §6's words exactly, reachable by `Q` and the palette, refreshable, and the six captures at
   80 columns with nothing clipped.
6. **§7's suite tests at 0.28 and 0.42**, with the ids equal to `log`'s after sealing and the skip announced.
7. **§8's docs, changelog, and a Breaking table built by API diff.**
8. **Eight gates** under the toolchain rule; run ids at one SHA.
9. **Nothing tagged or published.**

## 11. Submit

Package to `.git-exclude/review-request/028-a-queue-view/review-request-v1.md`.

**In this order:**
1. **The ids-equal-`log` assertion's output at 0.42.**
2. **The six Queue captures.**
3. **History's tier for another ref.**
4. **The reader's rule tests.**
5. **The Breaking table.**

**And tell me what the 0.41 and 0.42 binaries show that §2 does not.** In particular, whether a queue can report
a count with no `target_ref` and a `null` `target_ref_status`, which §3's model would have to hold and §5's third
row assumes.

**Push once approved.**

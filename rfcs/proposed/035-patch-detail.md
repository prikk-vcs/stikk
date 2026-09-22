# RFC 035 — Patch detail: what one patch changed, in prikk's own terms

**Status.** **Proposed 2026-09-22** by the architect, the day RFC 034 closed. **Revised the same day after prikk's
reply 019: F2's central claim was wrong, and the correction shrinks this RFC's only real gap** (see F2). **One open
question (Q1), now much narrower.**
`FR-030` has been blocked since **0.1.0** — first on `UD-09`, then on its own increment — and prikk 0.46 leaves
nothing else in the way.
Measured against a real prikk **0.46.0** binary (`cargo install --locked`), one repository exercising **every
operation kind prikk emits**: `edit-text`, `create-file`, `delete-node`, `replace-binary`, `change-perm`,
`rename-path`. Evidence: `.git-exclude/reports/035-patch-detail/`.
**Tracks.** `FR-030`, `FR-013`, `FR-031`, `UD-09` (retired), `C-T2b`, `C-T2c′`, `ER-02`, `T-T4`, `P-2`/`P-4`,
RFC 006 (3b), RFC 021 (F1, F2, decision 5), RFC 028 (the Queue view), RFC 034.
**Touches.** `stikk-prikk` (a new `show` seam method and its JSON reader), `stikk-core` (a patch-detail operation),
`stikk-tui` (the view, reached from History, Block detail and the Queue), `stikk-real-binary`, and on delivery
`requirements.md`.

## Summary

**prikk 0.36 made a patch's content readable; 0.46 made everything around it readable too. What was missing was the
view.**

`prikk show <block-id|patch-id> --format json` gives, per operation, exactly what `FR-030` asks for: a text edit's
**spans**, a create's **content and mode**, a delete's **preimage**, a binary replacement's **blob ids and sizes**, a
permission change's **old and new mode**, and a rename's **two paths and the key that asserted it**.

**Paths resolve when `show` is given a block id** — in both formats. They are unresolved for a **bare patch id**,
and therefore for a **queued patch**, which has no block to resolve against (F1, F2). **prikk 0.47 resolves the
queued case too**, against the folded baseline, and stikk must not key on a path being absent.

**So sealed history is complete today**, and the only gap is a queued patch on prikk ≤ 0.46. **Q1 is whether to ship
with that gap** or wait one prikk release.

## Findings

### F0 — `show --format json` answers `FR-030`, operation by operation

Measured at 0.46.0, one patch holding all six kinds (`show-report-v1`):

| Operation | What prikk's JSON carries |
|---|---|
| `edit-text` | `old_span_text`, `replacement_text` — **spans, not line ranges** (RFC 021 F2, now measured live) |
| `create-file` | `content` (`kind: text`, with the text) and `mode` |
| `delete-node` | **`preimage`** — the whole content the delete removed |
| `replace-binary` | `old`/`new`, each `blob_id` + `size` |
| `change-perm` | `old_mode`, `new_mode` (e.g. `33188` → `33261`) |
| `rename-path` | both paths, plus `author_key_id`/`author_key_ids` |

**A patch's shape:** `{patch_id, queued, operations[]}`, and **`show <block-id>` returns every patch in that block**,
which is how stikk enumerates them — `log` reports `patch_count` and `patch_messages` but **no patch ids**.

**`queued: true|false` is prikk's own word for whether the patch is still in the WAL**, so Patch detail serves the
Queue view and History from one read.

### F1 — three operations name no path **when `show` is given a patch id**

`edit-text`, `replace-binary` and `change-perm` report `paths: [{"unresolved_node_id": "<64 hex>"}]` when `show` is
asked about a **patch id**. Sealing the patch does not change that — measured before and after `prikk seal`,
byte-identical — because a patch id carries no block context to resolve against.

**Given a block id, the same three operations carry their paths** (F2). **stikk cannot resolve a node id itself:**
`prikk tree` lists `path`, `kind`, `encoding`, `mode`, `size` and `content_id` — **no node id** — so there is nothing
to join on.

### F2 — the axis is the id `show` is given, not the output format — **my letter 016 was wrong**

Measured as a 2×2, one patch, one repository, prikk 0.46.0:

| | `--format json` | prose |
|---|---|---|
| **`show <block-id>`** | `"path": "a.txt"` | `path: a.txt` |
| **`show <patch-id>`** | `"unresolved_node_id": "be664e1a…"` | `path: <unresolved node be664e1a…>` |

**The two formats agree in both directions.** Letter 016 claimed prikk's JSON withheld what its prose printed; it
does not. **The architect compared a prose run against a block id with a JSON run against a patch id, and attributed
the difference to the visible axis rather than the one that changed in the input.** prikk's reply 019 corrected it.

**The evidence to catch this was already in `measure035-0.46.txt`** — its prose section runs a patch id and prints
`<unresolved node …>` — and it was grepped for a few patterns rather than read. **Measuring is not the same as
reading what was measured.**

**What this changes here:** for a sealed patch, Patch detail asks `show <block-id>` and gets every path. The
unresolved case is a **queued** patch alone, which has no block by definition — and prikk 0.47 resolves that against
the folded baseline (reply 019 §2), rendering a queued patch identically to a sealed one apart from `"queued": true`.

**prikk files that as `### Changed`, not `### Fixed`:** a `path` will appear where one was absent. **stikk must not
key on absence** (decision 3).

### F3 — prikk's own diff names every path, for a sealed block

`prikk diff --from <parent-block> --to <block> --format json` on the same block:

```
modified a.txt | hunks: 1        added   created.txt | hunks: 1
binary   b.bin | hunks: 0        deleted del.txt     | hunks: 1
renamed  moved.sh | hunks: 0
```

**This is path-level and block-level.** It is *this patch's* effect **only when the block holds exactly one patch**
(`patch_count == 1`), which prikk reports. With more, it is the block's combined change and nothing may attribute it
to one patch.

**It cannot serve a queued patch at all** — there is no block yet.

### F4 — what a reader gets today

The Queue view (RFC 028) lists a queued patch's id, message and operation **kinds and paths** — where prikk resolves
them — and says plainly that it does not show content. Block detail lists a block's state files. **Neither shows what
an operation did**, and no view opens a patch. `FR-030` is the oldest unbuilt requirement stikk has.

### F5 — `UD-09` is fully retired

Its content half retired at prikk 0.36 (RFC 021 F1); its remaining half — *"no view"* — is this RFC. **`UD-10`
retired with RFC 034.** Nothing in the design set now blocks patch-level inspection.

### F6 — the raw view `FR-030` requires is cheap here

`FR-030` asks for a **raw operation view (exact fields, hashes, anchors) for `P-2`/`P-4`**. prikk's JSON *is* that
view: node ids, blob ids, modes as integers, spans as text. **Rendering it needs no extra read** — only a toggle
between stikk's rendering and prikk's fields, which is also the honest place to show an `unresolved_node_id`.

## Decisions

1. **Patch detail reads `show --format json`, and only that.** One new seam method (`show`), one reader, at prikk
   ≥ 0.36 where `show-report-v1` exists. **Below 0.36 the view says prikk does not report a patch's content** and
   shows what the Queue and History already know — never an empty patch.
   **It asks with the id that resolves the most:** a **block id** for a sealed patch, rendering the one patch the
   user opened from the block's report; a **patch id** only for a queued patch, which has no block (F2).

2. **Every operation is rendered from prikk's own fields**, in prikk's kind word, verbatim (`ER-02`):
   - **`edit-text`** shows `old_span_text` → `replacement_text` **as spans**, labelled as spans, with **no invented
     line numbers or context** (RFC 021 F2, `T-T4`). Either may be empty, and an empty one is shown as *empty*, not
     as absent.
   - **`create-file`** shows its content and mode; **`delete-node`** shows its **preimage**, labelled as what was
     removed.
   - **`replace-binary`** shows both blob ids and sizes, and **never renders bytes**.
   - **`change-perm`** shows both modes.
   - **`rename-path`** shows both paths and the asserting AUTHOR key id.

3. **An unresolved path is named as unresolved** — prikk's node id, inert, with one sentence saying prikk does not
   resolve it *for a patch with no block yet*. **stikk never guesses which file it is**, from `diff`, from the
   Changes view, or from anything else. This is RFC 028's rule for the Queue view, applied to the same value.
   **It is reachable only for a queued patch on prikk ≤ 0.46**, and stikk **must not key on the absence of `path`**:
   0.47 adds one there, as a `### Changed`. The reader takes a path when prikk gives one and says so when it does
   not, at every version.

4. **Prikk's block diff is a separate, labelled section, and only where it is honest** (F3): shown when the patch's
   block holds exactly one patch, labelled *"what this block changed, from `prikk diff`"*; **offered in Block detail
   rather than Patch detail when the block holds more**, where it belongs to the block. **Never merged into the
   operation list**, and never for a queued patch.

5. **A raw view is a toggle, not a second read** (F6) — prikk's fields as prikk gives them, which is also where an
   unresolved node id is legible rather than apologised for.

6. **Patch detail opens from three places**: the Queue view (a queued patch), Block detail (a sealed patch, via
   `show <block-id>`), and History's block rows. **One view, one operation, one read.**

7. **Content is bounded before it is rendered.** A create or preimage carries whole file content; stikk shows a
   bounded prefix with an explicit *"N more bytes not shown"*, **never a silent truncation** (`C-T2c′`). The bound is
   stikk's own display limit, stated where it bites.

8. **Letter 016 is withdrawn and corrected by letter 017** (F2). Nothing is asked of prikk for the sealed case;
   the queued case is already ruled and shipping in 0.47.

## Open question

### Q1 — does Patch detail ship before prikk 0.47 resolves a queued patch's paths?

**The facts, after F2's correction.** **Sealed history is complete today**: every operation carries its path when
`show` is asked about the block. **A queued patch** — committed, not yet sealed — reports a node id for `edit-text`,
`replace-binary` and `change-perm` on prikk ≤ 0.46, in both formats, because it has no block to resolve against.
**prikk 0.47 fixes exactly that**, and it is ruled and handed off upstream.

- **(a) Ship it — recommended.** Sealed patches, which are the history a user browses, are complete. A queued patch
  shows its spans, preimages, modes and blob ids, and says plainly that prikk does not resolve those three paths
  until the patch is sealed — which is **true, short-lived and self-correcting**: the same patch renders completely
  the moment it seals, and completely at any age once prikk 0.47 lands.
- **(b) Wait for prikk 0.47.** The view arrives with no caveat at all. *Cost:* `FR-030` waits again — since 0.1.0 —
  for a gap that affects only unsealed work, on a release prikk has already handed off.

**My recommendation is (a).** The caveat is one sentence, it is true, and it disappears twice over.

## Delivery

**One handoff**, issued on acceptance, in two commits: the seam method and reader, then the view. **Letter 016 goes
out independently** — the design does not wait on it.

## What this RFC does not do

- **No Compare** (`FR-033`). It shares the new `show` reader and follows next.
- **No diff-aware search** (`FR-013`), which needs content across many patches.
- **No worktree reads.** Everything here comes from prikk's report of a patch (`CON-1`).
- **No line-diff rendering of a span edit**, at any prikk version (RFC 021 F2).
- **No prose `show` parsing**, ever, for the reason in F2.

# RFC 035 — Patch detail: what one patch changed, in prikk's own terms

**Status.** **Proposed 2026-09-22** by the architect, the day RFC 034 closed. **One open question (Q1).**
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

**One thing is missing, and it is the one a reader needs most: the file's name.** For `edit-text`, `replace-binary`
and `change-perm`, the JSON reports an **`unresolved_node_id`** instead of a path — while **prikk's own prose `show`
prints the path for all three** (F2). So prikk knows it; the machine-readable form does not carry it. **Letter 016
asks for it.**

**Q1 is whether that gap is worth shipping around**, or whether the view waits for prikk.

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

### F1 — three operations name no path

`edit-text`, `replace-binary` and `change-perm` report `paths: [{"unresolved_node_id": "<64 hex>"}]`. **Sealing does
not resolve them** — measured before and after `prikk seal`, byte-identical.

**stikk cannot resolve them either:** `prikk tree` lists `path`, `kind`, `encoding`, `mode`, `size` and `content_id`
— **no node id** — so there is nothing to join on. The id is opaque to every surface stikk can read.

### F2 — prikk's prose `show` prints the paths its JSON withholds

The same command, same patch, without `--format json`:

```
  operation 3: change-perm
    path: moved.sh
    mode: 644 -> 755
  operation 5: edit-text
    path: a.txt
    old:
two
    new:
TWO
```

**prikk resolves the path and prints it.** The JSON does not carry it.

**And the prose is not a safe fallback.** Content is printed raw and unframed — the `old:`/`new:`/`content:` blocks
are file bytes with no delimiter — so a file containing a line like `  operation 7: create-file` would be read as
structure. **Parsing that would be stikk inventing a patch from a user's file content**, which is the `T-T4` failure
in its purest form. **stikk reads the JSON and asks prikk for the path** (letter 016).

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
   resolve it in this report. **stikk never guesses which file it is**, from `diff`, from the Changes view, or from
   anything else. This is RFC 028's rule for the Queue view, applied to the same value.

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

8. **Letter 016 asks prikk to carry the path in `show --format json`** for the three operations whose prose already
   names it.

## Open question

### Q1 — does Patch detail ship before prikk resolves those paths?

**The facts.** Three of six operation kinds — including `edit-text`, the one a reader opens a patch *for* — arrive
with a node id instead of a path. prikk's prose has the path; its JSON does not; parsing the prose is unsafe (F2). A
single-operation patch is unambiguous in practice (the Queue view's row names the file), but a patch touching four
files shows three edits whose targets stikk cannot name.

- **(a) Ship it, with unresolved paths named as unresolved — recommended.** Everything else in the patch is exact:
  the spans, the preimage, the modes, the blob ids. A reader sees *what changed* and, for three kinds, not *where*
  until prikk answers. **stikk says so plainly**, and the gap closes with a prikk release rather than a stikk one.
- **(b) Wait for prikk.** The view arrives complete. *Cost:* `FR-030` stays unbuilt on an upstream timeline we do not
  control, having already waited since 0.1.0 — and prikk has answered every letter so far in days.
- **(c) Ship it, and resolve paths from `prikk diff` where the mapping is unambiguous** (one modified path, one
  `edit-text`). *Cost:* **it is inference**, and the moment it is wrong it is wrong on a confirmation-free screen a
  user trusts. It also cannot serve a queued patch, where there is no diff at all.

**My recommendation is (a).** stikk's whole discipline is to show what prikk reports and name what it does not.
**(c) is the option this project exists to refuse.**

## Delivery

**One handoff**, issued on acceptance, in two commits: the seam method and reader, then the view. **Letter 016 goes
out independently** — the design does not wait on it.

## What this RFC does not do

- **No Compare** (`FR-033`). It shares the new `show` reader and follows next.
- **No diff-aware search** (`FR-013`), which needs content across many patches.
- **No worktree reads.** Everything here comes from prikk's report of a patch (`CON-1`).
- **No line-diff rendering of a span edit**, at any prikk version (RFC 021 F2).
- **No prose `show` parsing**, ever, for the reason in F2.

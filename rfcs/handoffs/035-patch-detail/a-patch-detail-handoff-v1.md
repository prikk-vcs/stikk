# Handoff A — RFC 035: Patch detail, and the 0.51 ceiling (v1)

**Companion to:** [RFC 035](../../accepted/035-patch-detail.md), **accepted 2026-10-13, Q1 ruled (a), amended on
acceptance (A1)**. Read **A1** before the body of the RFC: where they differ, A1 governs.
**Covers:** roadmap item 2, the 0.51 ceiling; RFC 035 decisions 1, 2, 3, 3a, 5, 6 and 7 as amended. **Decision 4 is
not in this handoff**: A1 moved it to Compare.
**Not a release.** 0.9.0 stays unreleased; nothing is tagged.

> **What this increment is.** `FR-030` has been unbuilt since 0.1.0. A reader can see that a block holds three
> patches and what their messages say, but not what any of them changed. prikk's `show --format json` says exactly
> that, per operation. This builds the view.
>
> **The floor:** every sentence must be true of the prikk that produced the report it describes. A1 exists because
> the RFC's own first draft broke that in two places (F7). Each band is named below. Do not widen one without
> measuring it.

---

## 1. Commit 1 — raise the validated ceiling to 0.51, first

RFC 034 A1, and `VALIDATED_MAX_MINOR`'s own documentation: the suite refuses binaries outside the range, so the raise
comes before anything is built on 0.51.

- `crates/stikk-prikk/src/version.rs`: `VALIDATED_MAX_MINOR` 46 → **51**, plus whatever fixture or test pins the old
  number.
- `docs/src/reference/requirements.md` `ASM-2`: *"validated through 0.51.0"*, with RFC 035 added to the raise history.
  `README.md`: the same.
- **I measured this already** (2026-10-13, prikk letter 021): on a scratch copy with only the constant raised, the
  suite passed **37 of 37 at 0.28.0 and 0.51.0** (`.git-exclude/reports/036-prikk-0-51-recheck/`). **If the suite is
  not green at this commit, stop and report**: something between that copy and `main` differs, and I need to know
  what.
- Grep for `0.46` across `docs/`, `README.md` and `crates/` doc comments, and change every one that states the
  ceiling. **I left two stale ceiling claims behind last time**, and the sweep is part of this commit.

## 2. Measured at prikk 0.46.0 and 0.51.0. Do not re-derive this

Scripts and output: `.git-exclude/reports/035-patch-detail/`. **If a row differs when you re-run it, stop and
report.**

**`show-report-v1`, identical at 0.46 and 0.51.** Top level: `{schema_version, patches[]}`. Per patch:
`{patch_id, queued, operations[]}`. Per operation: `kind`, `paths[]`, and `content{}`:

| `kind` | `paths[]` | `content` |
|---|---|---|
| `edit-text` | one, **node-addressed** | `old_span_text`, `replacement_text`, both strings, either may be `""` |
| `create-file` | one, its own path | `content: {kind: "text", text}`, `mode` (integer, e.g. `33188`) |
| `delete-node` | one, its own path | `preimage: {kind: "file", content: {kind: "text", text}}` |
| `replace-binary` | one, **node-addressed** | `old`/`new`, each `{kind: "binary", blob_id, size}` |
| `change-perm` | one, **node-addressed** | `old_mode`, `new_mode` (integers) |
| `rename-path` | two, old then new | `author_key_id`, `author_key_ids[]` |

Each element of `paths[]` is **either** `{"path": "…"}` **or** `{"unresolved_node_id": "<64 hex>"}`.

**Where a node-addressed path resolves (RFC 035 A1 F7).** Patch 1 edits `a.txt`, and patch 2 renames it to `z.txt`.
Both are then sealed into one block:

| Patch 1's `edit-text` | 0.46 | 0.51 |
|---|---|---|
| queued, `show <patch-id>` | unresolved | `a.txt` |
| sealed, `show <patch-id>` | unresolved | unresolved |
| sealed, `show <block-id>` | **`z.txt`** | **`z.txt`** |

Edit then delete in one block, at 0.51: queued gives `d.txt`; sealed `show <block-id>` gives **unresolved**.

**Messages.** `show` carries none. `log --format json` gives each block's `patch_messages: [{patch_id, message}]`.
stikk already reads them into `BlockRow.messages`. A patch with no message has **no entry**; it does not have an
empty one.

**Not measured: 0.39**, stikk's JSON floor and this view's band floor. See §7.

## 3. Commit 2 — the seam: `show`, and its reader

**One new `Prikk` method.** It takes a target: a sealed block (by block id) or a queued patch (by patch id). Its
category is `ReadHistory`, and it runs `show <id> --format json`.

**Version-banded behind this one method**, in `Prikk::queue`'s pattern. At prikk ≥ 0.39 (`reads_json()`, the one
place that number lives) it reads. **Below 0.39 it spawns nothing** and returns an *unreported* variant, never an
empty list.

**The reader's rules:**
1. `schema_version` must be `show-report-v1`; otherwise stikk's environment error, as for every other JSON reader.
2. `patch_id` is validated as a 64-hex object id. `queued` is a boolean and is **prikk's word for sealed or queued**.
   stikk never infers it from which id it asked with.
3. **`kind` is kept as prikk's text.** A kind stikk does not know does **not** fail the report. It becomes an
   operation stikk does not render, which keeps prikk's word and its raw fields.
4. **`paths[]` elements become a two-way value: a path, or an unresolved node id.** An element with neither, or
   with both, is an environment error. **Never key on a field's absence.** 0.47 started putting `path` where there
   was none, prikk filed that as `### Changed`, and A1 asks them to change it again.
5. **Content is read per kind, as §2's table gives it.** A `content`/`preimage` whose own `kind` is not one stikk
   knows (only `text`, `binary` and `file` were measured) becomes "not rendered" with prikk's kind word. It is not
   an error and **not empty** (`C-T2c′`).
6. **Keep each operation's JSON as prikk gave it**, for the raw view (decision 5). It costs no second read.
7. **Order is prikk's**, for both the patches and their operations.

**Seam tests:** fixtures captured from the 0.51 runs above for each kind, for both `paths` shapes, and for an unknown
operation kind and an unknown content kind. Use the existing `variant(fixture, from, to)` helper for the
negatives: wrong schema, a malformed id, a `paths` element with neither field or with both.

## 4. Commit 3 — the view

### 4.1 Block detail gains the block's patches (≥ 0.39)

- `stikk_core::block_detail` reads `show <block-id>` for **every** block, not only the tip. The tip's state-file read
  is unchanged and still tip-only. That read moves to `tree` later, as roadmap item 6.
- A **Patches** section: one row per patch in prikk's order, with the short patch id, the message **joined by
  `patch_id`** from `BlockRow.messages` (or *no message reported*), and the operation count. It gets a cursor, and
  **Enter opens Patch detail** for that patch **from the report already read**. There is no second read
  (decision 6).
- Below 0.39 the section says, in stikk's voice, that stikk reads a patch's content from prikk 0.39 on (`C-T4d`).
  **Not** *"prikk does not report it"*: that is false at 0.36–0.38.
- The existing patches summary (`patches_summary`) stays as it is.

### 4.2 The Queue view gains a selection (≥ 0.39)

It scrolls today, with no cursor. Give it one, over patches. Scrolling follows the cursor, and wrapping stays as it
is. **Enter opens Patch detail with one `show <patch-id>` read.** Below 0.39 the Queue is already `Unreported`, so
there is nothing to select.

### 4.3 Patch detail

**Heading:** the patch id; *queued* or *sealed*, from prikk's `queued`; for a sealed patch, the block it is in; and
the message (from the Block detail join, or `QueuedPatch.message`, with its three states intact). Message text is
repository content, so it goes through `inert` (`C-T2a`).

**Each operation**, under prikk's kind word verbatim (`ER-02`):
- **`edit-text`**: `old_span_text` → `replacement_text`, **labelled as spans**. No line numbers, no context, no
  line-diff (RFC 021 F2, `T-T4`). An empty span reads as *empty*, never as a blank line.
- **`create-file`**: its content and mode. **`delete-node`**: the preimage, labelled as what was removed.
- **`replace-binary`**: both blob ids and sizes. **Never bytes.**
- **`change-perm`**: both modes. Show them as octal (`100644` → `100755`) **next to** prikk's integers in the raw
  view, never instead of them.
- **`rename-path`**: both paths and the asserting AUTHOR key id.
- **An unresolved path** (decision 3 as amended): **exactly** *prikk names no path for this operation*, with the
  node id inert. **No reason.** stikk cannot tell which reason applies (A1 F7).
- **The block-end label** (decision 3a): when the patch's block holds **more than one patch**, the path of each
  `edit-text`, `replace-binary` and `change-perm` operation is labelled *as at the end of this block*. The count is
  the number of patches in the same `show <block-id>` report. **Never correct a name from a later patch's
  rename.** It never applies to a queued patch or a block of one.

**Bounded content** (decision 7): **one** constant in `stikk-core` bounds every content field shown. Cut at a
`char` boundary and say *N more bytes not shown*, where N counts the UTF-8 bytes withheld. **A silent cut is the
defect this rule exists to prevent** (`C-T2c′`). Wrap long lines, never clip them (RFC 024).

**`v` toggles the raw view** (decision 5): each operation's JSON exactly as prikk gave it, and nothing else. List it
wherever the other keys are listed (help, palette, glossary), in the pattern the existing toggles use.

**`r` re-reads.** A queued patch can be sealed while it is open. Re-read the same patch id, and **render the result
as prikk now reports it**: `queued: false`, and a path that is now unresolved, because a sealed patch id resolves
nothing (§2). **Do not carry a path over from the earlier read.** The resulting screen tells the truth, and is
worse than the one before it. That is A1 F7, which letter 020 asks prikk to fix.

## 5. The traps

1. **The id decides the path, not the format** (RFC 035 F2). That was my error in letter 016. Do not repeat it in a
   test by comparing a block-id run against a patch-id run.
2. **Absence is not information.** `path` appears or disappears between releases. Read the element's shape.
3. **The block-end label is about the block's patch count**, not about whether a later patch renames something.
   stikk does not inspect later patches to decide.
4. **Messages join by id, never by position.** `patch_messages` omits a patch with no message, so positions drift.
5. **`queued` is prikk's word.** Asking with a patch id does not make a patch queued; F7's middle row is a sealed
   patch asked about by patch id.
6. **Bound before you render**, not after layout. A preimage can be a whole file.
7. **One read per screen.** Patch detail opened from Block detail spawns nothing.

## 6. Captures at 80 columns

Patch detail for each of the six kinds. An unresolved path. The block-end label in a two-patch block. A bounded
preimage with its *N more bytes not shown*. The raw view. Block detail's Patches section at 0.51, and its
below-0.39 sentence. The Queue with its selection.

## 7. Tests and the bands

- **Suite legs at 0.51:** a repository with all six kinds in one sealed patch, where every path resolves and the
  rendered words match prikk's fields. **F7's two two-patch cases**: the rename gives `z.txt` **with** the block-end
  label; edit-then-delete gives *prikk names no path for this operation*. A queued patch opened from the Queue.
- **Suite leg at 0.28:** Block detail's below-0.39 sentence; the Queue unchanged.
- **0.39 is the band floor and not in the suite's matrix. Measure it by hand through the suite's harness**, which
  already supplies keys to old prikks the way 0.39 needs (the architect does not read seeds, which is why it was not
  measured). Run the six-kind repository and F7's rename case. **If `show-report-v1` at 0.39 differs from §2's
  table in any field stikk reads, stop and report**: the band floor would move.
- **Core tests with a stand-in backend:** the message join with a missing entry; the label at one patch (absent) and
  at two (present); the bound at, under and over the limit, including a multi-byte character at the cut; an
  unknown kind; a re-read that turns a resolved path unresolved.
- **On delivery, `requirements.md`:** `FR-030` delivered at prikk ≥ 0.39, with A1 F7's label named. `UD-09`'s note
  points here. **`BlockRow`'s doc comment** still says per-patch content is beyond prikk's ceiling; correct it.

## 8. Gates, runs, submit

The eight gates as always: 1–5, 7 and 8 on the MSRV; gate 6 on stable with a fresh `CARGO_TARGET_DIR`. Then, from a
`prep/035-patch-detail` branch: `CI`, the real-binary suite with `full_platform_matrix=true`, and the supply-chain
gate, **all at one SHA**, every job and leg read, and the branch deleted before submitting.

Package to `.git-exclude/review-request/035-patch-detail/review-request-v1.md`.

**Lead with the suite at the ceiling commit**, then the 0.39 hand measurement, then the captures. If §2's table or
F7's rows differ from what you measure, put that first of all.

**Push your own commits once approved.** After this, Compare (`FR-033`) is next, with A1's decision 4 in it.

# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## 0.10.0 — 2026-10-14

**stikk compares any two points in history, and every string it shows from a repository is shown as it
is.** **Compare** (`FR-033`) is the last named view that was unbuilt: from History, Block detail and
Changes, it shows what changed between two points in **prikk's own `diff` hunks**, quoted line by line
so that a whitespace or line-ending edit is visible — stikk computes no difference itself (RFC 038).
The other half is a **security fix**: bidirectional overrides, zero-width characters and look-alike
spaces in paths, branch names and messages no longer reach the terminal in any view, and two different
names never look alike (RFC 036). 0.9.0's own release notes named that gap. Beside them, **Block detail
now lists any block's files, with modes**, where it listed only the ref tip's, and **the ref picker
offers the branch you are on before its first seal** (RFC 037). The validated prikk range is unchanged
— **≥ 0.28, through 0.51.0** — and the real-binary suite drives **47 cases** against real prikk
binaries at 0.28.0 and 0.51.0, on Linux, macOS and Windows.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position. These are grouped by crate
in dependency order. **`stikk-model`, `stikk-state` and the `stikk` launcher are unchanged.**

| Crate | Change |
|---|---|
| `stikk-prikk` | **the `Prikk` trait gains `diff(&self, repo, &Comparison) -> Result<DiffReport>`**, with no default — every implementor must add it |
| `stikk-prikk` | **`Prikk::block_state` takes the block and whether it is the tip** — `block_state(&self, repo, reff, block_id, is_tip)` — and returns `StateReport`, which is `Listed` or `Unreported`. It asks prikk with the **block's own id**, so the answer is about the block the reader opened rather than wherever the ref now points |
| `stikk-prikk` | **`Prikk::refs` returns `BranchList`** — the ref pointers **and** the current branch before its first seal, beside the list and never in it. A `RefEntry` for an unpublished branch would make every caller of the *published* test (membership in that list) call it published |
| `stikk-prikk` | **`StateFiles::files` becomes `Vec<StateFile>`** — a path, prikk's `kind`, and the file's `mode`. The mode is `Option<u32>`: `None` is *not reported*, never `0`, which would be a mode (`000000`) prikk never claimed |
| `stikk-core` | **`OperationView::title` becomes `Vec<Piece>`** — prikk's kind word and each path are names the repository chose, and only a frontend can decide how a name is rendered. A `String` forced one rule on the whole sentence |
| `stikk-core` | **`OperationRow` gains `Named(Vec<Piece>)`** — breaking for an exhaustive `match` — and **`OperationRow::text` returns `Option<&str>`**, `None` for that variant: a caller that flattened one would have lost the distinction the row exists to carry |
| `stikk-core` | **`BlockPatchRow::summary`, `HistoryView::queued_tier` and `RenameHalf::annotation` return pieces** (`Vec<Piece>`, and `Option<Vec<Piece>>` for the tier) instead of a composed `String`, for the same reason |
| `stikk-core` | **`BlockDetailView::state` becomes `BlockState`** (`Listed` / `Unreported(minor)` / `Failed`) instead of `Option<StateFiles>`. `None` used to mean "an older block, which prikk cannot replay to"; from prikk 0.46 every block has an answer, and `None` would now mean two things, one of them false |
| `stikk-core` | **`list_refs` returns `RefList`**, for the same reason `Prikk::refs` does |
| `stikk-tui` | **`Screen` gains `Compare { … }`** and **`Focus` gains `Compare(…)`** — breaking for an exhaustive `match` |
| `stikk-tui` | **`Overlay::RefPicker` gains `unpublished_current: bool`** — breaking for a struct-variant literal or a pattern that names every field |

**Additive:** `stikk_prikk::StateFile`, `StateReport`, `UnpublishedBranch`, `BranchList`,
`Comparison`, `Diff`, `DiffEntry`, `DiffFile`, `DiffReport`, `DiffSide`, `DiffTarget`,
`QueuedOnSide` and `UnsupportedPath`, with `TREE_FROM_MINOR`, `UNPUBLISHED_BRANCH_FROM_MINOR` and
`DIFF_FROM_MINOR`, and `NullBackend`'s `with_diff`, `with_diff_refusal`, `with_state_unreported`,
`with_state_refusal` and `with_unpublished_current`; `stikk_prikk::escape::escape_invisible_into`.
`stikk_core::piece` with `Piece` and `flatten`; `stikk_core::compare` with `CompareView`, `Compared`,
`ComparedEntry`, `ComparedSide`, `HunkRow`, `compare`, `compared` and `COMPARE_FROM_MINOR`;
`stikk_core::BlockState` and `RefList`; `OperationContext::Compare`, on an enum that is already
`#[non_exhaustive]`. `stikk_tui::app::CompareSource`, `CompareFocus` and `MarkedBlock`, with
`App::mark_block`, `App::compare` and `App::marked_block`; `stikk_tui::text::identifier` and
`compose`. **`stikk_tui::text::inert` keeps its signature** and changes only what it produces (see
*Changed*). **No public item was removed.** The MSRV is unchanged at 1.88.

### Security

- **Every string stikk did not write is now rendered so that what you read is what prikk reported.**
  Until this release only Patch detail's *content* was (0.9.0).
  **In the TUI** — paths, ref and branch names, key ids, patch messages, prikk's refusals, notes and
  version line, in every view — the rest passed through `inert`, which replaced Unicode category
  `Cc` with the replacement character and let everything else through. **In the one-shot printout it
  was worse: the repository root and prikk's version line were printed with no filtering at all**, so
  a control character in either reached stdout as itself; only the queued target was filtered.
  (`heads/main`'s id is a validated hex id, so it was never a carrier.) What `inert` let through was
  three kinds of character:
  - **bidirectional controls** (`U+202A`–`U+202E`, `U+2066`–`U+2069`, `U+200E`, `U+200F`, `U+061C`),
    which **reorder the displayed text** — the "Trojan Source" class, CVE-2021-42574. A branch name or
    a patch message could make a line read in an order its bytes do not have;
  - **zero-width and other default-ignorable characters**, which display nothing, so two different
    names looked identical;
  - **whitespace that is not an ordinary space** (`U+00A0`, `U+2000`–`U+200A`, `U+3000`, …), which
    looks like a space or breaks a row.

  These are reachable: prikk 0.51 reports an on-disk filename carrying any of them, and **accepts a
  branch name carrying a bidirectional override or a zero-width space** (measured before this was
  built). 0.9.0's release notes named this gap; this closes it.

  **Two rules now, from the one escape table 0.9.0 built for Patch detail.**
  - **Prose** — messages, refusals, notes, reasons, routes, prikk's version line: every character that
    does not show as itself is written as its escape, in place. `"` and `\` are untouched and nothing
    is quoted, so a refusal still reads as prikk wrote it.
  - **Identifiers** — paths, ref and branch names, key ids, prikk's kind words and status words where
    they tag a row, and **every name in a view added this release**, the ref picker's unpublished
    branch and Compare's paths among them: the same, **and two different identifiers never look
    alike**. An identifier is shown plainly when nothing in it needs an escape and its whitespace is
    unremarkable, which is nearly always; otherwise it is quoted, with every space written as
    `\u0020`, so that no wrapping can collapse, trim or break it. `"a\u0020\u0020b.txt"` is a path
    prikk itself accepts — prikk's own validator does, and a commit would author it.

  **All four values the printout writes now take the same two rules** as the TUI: the repository
  root, `heads/main`'s id and the queued target as identifiers, prikk's version line as prose.

  **The limit, stated:** this is about what reaches the terminal. A terminal that renders an escape
  sequence stikk never sent is outside stikk's reach, and so is a font that draws two different
  characters identically.

### Added

- **Compare: what changed between two points, in prikk's own hunks** (prikk ≥ 0.46) — `FR-033`,
  the last named view stikk had not built. The roadmap said arbitrary-point comparison was refused
  rather than deferred; that predates `prikk diff`, which compares any two points, bare block ids
  included. **Three ways in, each a comparison that is honest where it is offered.** In **History**,
  `m` marks a block and `d` compares it with the selected one, ordered **older → newer** by prikk's
  own publication sequence whichever was marked first. In **History and Block detail** with nothing
  marked, `d` compares the block with **the previous block in this ref's history** — the next older
  row in the loaded lineage, named as exactly that and never as "the parent", because a merge block
  has two parents and neither need be the row above it. On the oldest loaded block `d` says so and
  reads nothing. In **Changes**, `d` compares the worktree with what `commit` would author against,
  naming the queued commits folded into that side.

  **stikk computes no part of a comparison** — no renumbering, no re-hunking, no rename inference.
  prikk's unified hunks are shown as prikk wrote them: the `@@` header and the
  `\ No newline at end of file` note in prikk's words, and every body line as its marker plus its
  content **quoted, with the line's terminator inside the quotes**. So a trailing-space edit, a CRLF
  change and a missing final newline are each visible, and **two lines differing in any byte never
  render alike** — in the one view built to show exactly those edits. A rename names both paths, a
  `mode` entry both modes in octal, a `binary` entry both sizes and content ids and **never bytes**,
  and a path `commit` would refuse carries prikk's refusal verbatim. Where prikk says the hunks are
  not the shortest edit script, it says so. Each entry is bounded, with the withheld byte count said
  out loud, and `v` shows prikk's own fields for the selected entry — bounded too. Below 0.46, stikk
  says it compares from 0.46 on and spawns nothing. **Comparing two different refs is not built**:
  prikk can do it, but the picker that would choose two refs is its own design.
- **The ref picker offers the branch you are on, before its first seal** (prikk ≥ 0.47). Until now
  a brand-new repository had nothing to choose: a branch has no ref pointer until its first `seal`,
  so the picker was empty and stikk offered a guessed `heads/main (not published)` row only when it
  was *completely* empty. prikk 0.47 names that branch in `branch list --format json`, in a field
  built for stikk, and the picker now offers it as its own row — **first**, marked in stikk's own
  words. Selecting it focuses that ref like any other: History shows its queued tier and no sealed
  blocks, and Changes works as it does today. **It is never added to the ref list**, so "has this ref
  published history?" — which stikk answers by membership in that list — still says *no* for it,
  and Changes still opens a fresh repository with *no published history*. Below 0.47 the picker is
  exactly as it was, the older guessed row included; where prikk reports there is no such branch,
  nothing says so.

### Changed

- **Block detail lists any block's files, with each file's mode, at prikk ≥ 0.46.** It listed only
  the ref tip's, and said *"prikk replays only to the ref tip — not shown for an older block"* for
  every older one. That stopped being true when prikk 0.46 added `tree`, which lists the files at a
  bare block id and needs no replay. stikk now asks with the **block's own id**, so the answer is
  about the block you opened rather than wherever the ref has since moved, and the heading says
  *state at this block* where it is not the tip. Each row carries its mode in the same octal Patch
  detail shows (`100644`). **Below 0.46 nothing changes**, and the old sentence is shown only there,
  where it is still true. A file whose mode prikk did not report says so; it is never shown as `0`.
  **A failed read costs Block detail its file list alone** — the block's fields and its patch list
  stay.
- **A control character is shown as its escape, not as `�`.** `\u001b`, `\n`, `\t` say which
  character was there; `�` said only that something was. **`U+FFFD` is now prikk's alone**: prikk
  substitutes it for filename bytes it cannot show, so a `�` on screen means prikk put it there —
  which is what makes commit's *"a name shown with `�` is not the file's real name"* step exact.
- **The Glossary** (`?`) names the quoted-name convention beside the quoted-content one, and lists
  `m` and `d`.
- **The workflows run on node24.** Every GitHub action stikk uses was moved off the deprecated
  node20 runtime; mdBook is installed from a release archive pinned by **version and SHA-256**
  rather than resolved as `latest`, and the Pages deploy is guarded to `main` so a dispatch from
  another branch cannot publish it. Nothing a user of the crates or the binaries can observe
  changes; the attestation keeps its format.

### Fixed

- **A long path in a Changes row is no longer cut at the border.** The tag keeps the first row and the
  path continues on further rows at the entry's indent, hard-split so that nothing is lost, with
  prikk's note after it. A 150-character path at 80 columns now appears whole, in order.
- **A double space in a path no longer collapses**, and neither does a leading or trailing one. Word
  wrapping rebuilt every row from whitespace-separated words, which silently rewrote the three path
  shapes prikk accepts — `a  b.txt`, ` lead.txt` and `say"hi.txt`.
- **History's queued tier no longer clips.** It was one unwrapped row, so at 80 columns its sentence
  was cut at the border mid-word — *"…not this ref's hist"* — and a long ref name in it was cut with
  it. It now wraps, with continuation rows at the tier's own indent; a 60-character ref name appears
  whole, in order. (The same class of defect as the Changes row above, in the view beside it.)
- **The Changes view no longer says a per-file content diff awaits prikk.** It did, and at prikk
  0.46 that stopped being true: `prikk diff` with no arguments *is* that diff. The foot now names
  the `d` key that opens it, or — below 0.46 — says in stikk's own voice that stikk compares from
  0.46 on.

## 0.9.0 — 2026-10-14

**stikk now shows what each patch changed, in prikk's own terms and byte for byte.** `FR-030` has been
unbuilt since 0.1.0: a reader could see that a block held three patches and what their messages said, but
not what any of them did. **Patch detail** opens from Block detail — which now lists every block's patches
— and from the Queue, and renders each operation from prikk's own `show` report: a text edit's spans, a
created file's content and mode, a deleted file's preimage, both blobs of a binary replacement, both
modes of a permission change, and a rename's two paths with the key that asserted it. `v` shows prikk's
own fields instead. **Content is quoted and escaped**, so a whitespace-only edit is visible and nothing
invisible or text-reordering reaches the terminal from it (RFC 035). The other half of this release is
the **prikk 0.46 re-baseline**: a ref with no published history reads again on prikk 0.45 and 0.46, a
rename declaration's fate is **prikk's own verdict** rather than stikk's inference at prikk ≥ 0.44, and a
checkout that stopped part-way is shown (RFC 034). **The validated prikk range is now ≥ 0.28, through
0.51.0**, and the real-binary suite drives 42 cases against real prikk binaries at 0.28.0 and 0.51.0, on
Linux, macOS and Windows.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; these land in 0.9.0, grouped by
crate in dependency order. `stikk-model`, `stikk-state` and the `stikk` launcher are unchanged.

| Crate | Change |
|---|---|
| `stikk-prikk` | the `Prikk` trait gains `show(&self, repo: &Path, id: &str) -> Result<ShowReport>`, **with no default** — every implementor must add it |
| `stikk-prikk` | `Orientation` gains `interrupted_materialization: Option<String>` — prikk's own sentence for a checkout that stopped part-way, or `None` |
| `stikk-prikk` | `WorktreeStatus` gains `refused_declarations: Option<u64>` — `None`, never `Some(0)`, where prikk states no declaration verdict stikk reads |
| `stikk-prikk` | `RenameDeclaration` gains `resolution: Option<DeclarationResolution>`, `refusal: Option<String>`, `content_changed: Option<bool>` and `mode_changed: Option<bool>` — all `None` below prikk 0.44 |
| `stikk-core` | `BlockDetailView` gains `patches: BlockPatches` — the block's own patches, listed, unreported below prikk 0.39, or a failed read |
| `stikk-core` | `ChangesView` gains `refused_declarations: Option<u64>`, and `DeclaredRename` gains `resolution`, `refusal`, `content_changed` and `mode_changed` |
| `stikk-core` | `OrientationView` gains `interrupted_materialization: Option<String>` |
| `stikk-core` | `QueuedPatchView` gains `detail_message: PatchMessageState` — the message in the three states Patch detail needs, beside the line the Queue already showed |
| `stikk-core` | `CommitPreviewOutcome` gains a variant, `WouldRefuseDeclarations(Vec<RefusedDeclaration>)` — breaking for an exhaustive `match` |
| `stikk-tui` | `Screen` gains a variant, `PatchDetail { view, raw, source, refreshing, offset }`, and `Screen::BlockDetail` and `Screen::Queue` each gain `cursor: usize` |
| `stikk-tui` | `Focus` gains a variant, `PatchDetail(&PatchDetailView, bool, &Cell<u16>)`; `Focus::BlockDetail` gains a second field and `Focus::Queue` a third, both the `usize` cursor |
| `stikk-tui` | `Overlay` gains a variant, `CommitWouldRefuseDeclarations { declarations }` — breaking for an exhaustive `match` |

Every struct above is constructed with struct literals by anyone scripting a `NullBackend` or rendering a
view, and none is `#[non_exhaustive]`; a pattern that names every field without `..`, or matches a tuple
variant, no longer compiles. **No public signature changed, and no public function, trait method, type or
re-export was removed.** The MSRV is unchanged at 1.88.

**Additive**, and not listed above:

- **`stikk-prikk`:** the `escape` module (`needs_unicode_escape`, `escape_into`, `quoted`); the constant
  `JSON_REPORTS_FROM_MINOR`; the `show-report-v1` types `ShowReport`, `ShownPatch`, `PatchOperation`,
  `OperationContent`, `FileContent` and `BinaryBlob`; `DeclarationResolution` and its `label`; and
  `NullBackend::with_show`, `NullBackend::with_show_refusal` and `NullBackend::show_calls`.
- **`stikk-core`:** the `patch` module — `PatchDetailView`, `OperationView`, `OperationRow`,
  `PatchMessageState`, `patch_detail` and `patch_detail_in_block` — plus `BlockPatches`, `BlockPatchRow`,
  `RefusedDeclaration`, `Notation`, `glossary::notations`, `DeclaredRename::is_rename`,
  `DeclaredRename::content_sentence`, the constants `CONTENT_DISPLAY_BYTES`, `NO_PATH_REPORTED` and
  `AS_AT_BLOCK_END`, and the re-exports `DeclarationResolution` and `CONTENT_FROM_MINOR`.
- **`stikk-tui`:** `PatchSource`, `App::toggle_raw`, and `text::wrap_verbatim`.

### Added

- **Patch detail** (`FR-030`), at prikk ≥ 0.39: one patch's operations, each under prikk's own kind word
  with prikk's own fields — the span a text edit replaced and its replacement, **as spans**, never line
  numbers or invented context; a created file's content and mode; a deleted file's preimage; a binary
  replacement's two blob ids and sizes, **never its bytes**; a permission change's two modes; and a
  rename's two paths with the AUTHOR key that asserted it. `v` toggles prikk's own fields for the
  operation. It opens from **Block detail**, which now lists
  every block's patches with each patch's message joined by patch id, and from the **Queue**, which gained
  a selection; opening from a block spawns no second read (RFC 035).
  **Limit:** Patch detail starts at prikk 0.39, stikk's JSON floor. Below that it says so in stikk's own
  voice, rather than showing an empty patch.
  **Limit:** in a block of more than one patch, a node-addressed path is labelled *as at the end of this
  block*, because prikk resolves a sealed patch's path against the state after **all** of the block's
  patches — so patch 1 can be reported under a name a later patch gave the file. stikk renders prikk's
  value and labels it; it never corrects the name from a later rename. prikk has said it will resolve each
  patch at that patch in a future release.
- **Content is shown byte for byte.** Every content row is quoted, one row per source line with the line's
  own terminator inside the quotes, and wrapped by hard-splitting rather than on whitespace, so a
  re-indent, a tab turned into spaces, a double space, trailing whitespace and a removed trailing newline
  are all visible — each of which used to render as *no change at all*. Inside the quotes the spelling is
  prikk's own JSON: `\n`, `\t`, `\"`, `\\`, and `\uXXXX` for **every character that does not show as
  itself** — control characters, bidirectional overrides ("Trojan Source"), zero-width and other
  default-ignorable characters, and any whitespace that is not an ordinary space. Visible text stays as
  itself. Two lines that differ in any byte never look alike. The Glossary (`?`) names the notation.
  **Limit:** this is Patch detail's content only. **Paths, messages and ref names in every view still pass
  bidirectional and zero-width characters through** — stikk's `inert` neutralises control characters only.
  That gap predates this release and is not closed by it.
- **A binary file's create or delete shows its blob id and byte count**, the same as a binary replacement,
  and never its bytes. Where prikk reports content it does not hold, stikk says so and names the blob id
  it could not read.
- **A checkout that stopped part-way is shown** in Orientation, in prikk's own words, at prikk ≥ 0.43. It
  blocks commit's preview, while Changes and History keep working (RFC 034).

### Changed

- **The validated prikk range is `>= 0.28`, through `0.51.0`** (was `0.46.0` on `main`, `0.42.0` in the
  published 0.8.0; `ASM-2`). A newer prikk still runs, and stikk says the range is unvalidated rather than
  pretending to know it.
- **A rename declaration's fate is prikk's own verdict at prikk ≥ 0.44**, not stikk's inference: prikk's
  `resolution` — a rename, a deletion, an ignored deletion, a never-tracked destination, or a refusal —
  each with its own sentence. Below that band stikk infers as 0.8.0 did. **A renamed file's content and
  mode changes also come from prikk** where prikk reports them; where it reports nothing, stikk says
  nothing, because unknown is not "unchanged" (RFC 034).
- **A declaration prikk would refuse makes commit unavailable**, with prikk's refusal verbatim and no way
  out in stikk's words — checked before the clean-worktree case, so prikk's verdict stays visible on a
  worktree prikk itself calls clean. In 0.8.0 this was named but never prevented (RFC 034).
- **prikk's no-repository and too-old-binary answers are environment errors**, matched on prikk's own
  clauses with no format number parsed — not refusals of something the user did (RFC 034).
- **A failed read of a block's patches costs the patch list only.** The block's own fields, and the tip's
  replayed state files, do not start depending on a read that did not exist before this release; the
  section says what failed, in the usual classes (RFC 035).

### Fixed

- **On prikk 0.45 and 0.46, a ref with no published history refused** the Changes view, History and
  commit's preview — the state every new repository is in, measured against the published 0.8.0. stikk now
  re-runs the command without `--ref` and **accepts the report only when it names the ref that was asked
  for**; every other outcome returns prikk's original refusal, and an absent ref still refuses. Nothing
  consults the current branch, so one ref's name can never leak into another ref's answer (RFC 034).
  **prikk fixed this upstream in 0.47.0**, and stikk measured it again at 0.51, so the retry now matters
  on those two releases alone.

## 0.8.0 — 2026-09-16

**stikk stops describing a repository it has stopped watching.** 0.7.0 made stikk show what a change will do
before it does it; 0.8.0 is about the screen staying true — while another terminal works in the same repository,
and where prikk's own report has a shape stikk was reading too simply. **stikk now notices a commit, seal or
branch switch made outside it**, refreshes what is on screen, says so, and makes an open confirmation stale at
once (RFC 031). **A `prikk mv` is shown as the rename it will author**, counted beside the other changes, and a
declaration prikk will *not* author as a rename is named instead of counted (RFC 032). **A ref with no published
history is named as one**, in place of the "against baseline" it has no baseline for. `r` refreshes in place, and
every refresh re-reads the ref its own screen shows. The real-binary suite now drives 32 cases against real prikk
binaries at 0.28.0 and 0.42.0, on Linux, macOS and Windows.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; these land in 0.8.0, grouped by crate in
dependency order. `stikk-model`, `stikk-state` and the `stikk` launcher are unchanged, and `stikk-prikk` is additive
only.

| Crate | Change |
|---|---|
| `stikk-core` | `changes_view` returns `ChangesRead { view, history }` instead of `ChangesView`: whether the ref has published history travels **beside** the view, never inside it, so a commit's re-read at Enter still builds an equal view. `ChangesView` itself gains `declared_renames: Vec<DeclaredRename>` and `renames: u64`, both derived from prikk's worktree report alone |
| `stikk-core` | `ChangeEntry` gains `rename: Option<RenameHalf>` — which half of a paired declared rename this entry is, or `None` |
| `stikk-core` | `ConfirmationSummary` gains `history_notice`, `rename_note` and `declaration_notices` |
| `stikk-core` | `CommitPreviewOutcome::Ready`'s `preview` becomes `Box<CommitPreview>`, as its `token` already was (`clippy::large_enum_variant`, after the view grew) |
| `stikk-tui` | `Screen::Changes` gains `refreshing: Option<u64>` and `history: RefHistory` — the in-place refresh slot, and the publication state kept beside the view |
| `stikk-tui` | `Screen::BlockDetail(BlockDetailView)` becomes `Screen::BlockDetail { reff, view, refreshing }`: the ref the detail was read for, which a refresh re-reads, and its own refresh slot |
| `stikk-tui` | `Focus::Changes` gains a third field, `&RefHistory` |
| `stikk-tui` | `Overlay::Confirmation`'s `summary` becomes `Box<ConfirmationSummary>` (`clippy::large_enum_variant`, after the summary grew) |

Every struct above is constructed with struct literals by anyone rendering a view or driving the app, and none is
`#[non_exhaustive]`; a pattern that names every field without `..`, or matches a tuple variant, no longer compiles.
**One public signature changed** — `changes_view` — and **no public function, trait method or re-export was
removed.** The `Prikk` trait is unchanged. The MSRV is unchanged at 1.88.

**Additive**, and not listed above:

- **`stikk-core`:** the `ChangesRead`, `DeclaredRename`, `DeclarationState`, `RenameHalf`, `RefHistory` and
  `UnpublishedQueue` types; `ChangesView::content_note`, `DeclaredRename::notice`, `RenameHalf::annotation`,
  `RenameHalf::is_destination`, `RefHistory::from_facts`, `RefHistory::changes_headline` and `RefHistory::card_line`;
  and the `RENAME_CONTENT_NOTE` and `RENAMES_ALSO_COUNTED` sentences.
- **`stikk-prikk`:** `NullBackend::with_refs_refusal`.
- **`stikk-tui`:** `App::tick` and `App::focus_gained`, which take the current `Instant`.

### Added

- **stikk notices a change made outside it** — a commit, seal or branch switch in another terminal — when the
  terminal regains focus, or within 5 seconds while stikk is idle. It refreshes what is on screen, in place, and says
  *"repository changed outside stikk — refreshed"*. A commit or seal confirmation open at that moment becomes stale
  at once, with one next step: preview again. **The check is silent** — never counted in `⟳ n`, never in the
  Background Operations list, and it says nothing when it fails — and it is never sent while a request of stikk's own
  is running, so stikk can never take its own commit for an outside change (RFC 031).
  **Limit:** an edit to a file in the worktree is not a repository change and is not noticed this way. It is caught at
  Enter (RFC 030), by `r`, or by opening Changes.
- **A file renamed with `prikk mv` is shown as the one rename prikk will author**, on the Changes view and in
  commit's confirmation, whenever prikk lists both halves: prikk's `missing` and `untracked` rows both stay, each
  annotated as half of one declared rename, and the counts gain `renames N` at prikk ≥ 0.38. The untracked filter
  never hides a rename's destination (RFC 032).
  **Limit:** prikk does not report whether a renamed file's content also changed, and stikk says so — **except while
  prikk reports that the commit would be refused, when stikk says nothing at all, because prikk would author
  nothing.**
- **A declaration prikk will not author as a rename is named, and never counted** (RFC 032): its destination is not a
  file in the worktree, so prikk records a deletion; or its source is present again, and prikk refuses the commit
  until the declaration is resolved. Where the destination is gone too, stikk names the way out it measured at prikk
  0.42.0 — `prikk mv {new} {old}` in a terminal drops the declaration — and offers none where it measured none. A
  clean worktree holding such a declaration says that, rather than only "nothing to commit". **This is a notice, not a
  prevention:** stikk makes commit unavailable only on prikk's own verdict.

### Changed

- **`r` refreshes in place, and refreshes more.** The Changes view and the tip's Block detail now refresh too, beside
  Orientation, History and the Queue; each view stays on screen while its read runs, and Changes keeps its untracked
  filter. An older block's detail cannot change and is not re-read (RFC 031).
- **Every in-place refresh re-reads the ref its own screen shows**, not the focused ref: a History or Block-detail
  screen opened for one ref stays that ref's after the ref picker moves focus elsewhere (RFC 031).
- **The Changes operation reads more of prikk in one go**: `worktree-status`, then `refs()` for whether the ref is
  published, then `status` for the queue those words depend on; commit's preview reads `refs()` beside the reads it
  already made. **Three reads are not one instant**, so a terminal `prikk seal` landing between them can show the
  no-published-history words once for a ref just published — **stated rather than hidden**: RFC 031's check notices
  and refreshes within five seconds, and RFC 030's change token stops a confirmation armed on the old state (RFC 032).

### Fixed

- **A ref with no published history no longer reads as untracked files "against baseline"** — there is no baseline
  (RFC 032). stikk names it, on the Changes view and under commit's targets, in the words prikk's own facts support:
  **that a commit would be the ref's first**, or — once patches are queued for that ref and nothing is sealed — **that
  those queued patches are its baseline**. A queue belonging to another ref claims nothing about this one.

## 0.7.0 — 2026-09-15

**stikk shows what a change will do — and no longer makes a change it did not show.** 0.6.0 made stikk
say what it knows; 0.7.0 is about the moment you decide. **A confirmed commit authors the worktree its
preview showed**: if the repository or the worktree changes between preview and Enter, stikk says which,
and commits nothing (RFC 030). **Commit is unavailable, with prikk's reasons, when prikk says it would
refuse**, and the unsupported paths stikk has counted and never listed since 0.1.0 are listed (RFC 027).
**The Queue view** (`Q`) lists each queued patch, **the seal confirmation names the patches it freezes**,
and History no longer puts another ref's queue on this ref's lineage (RFC 028). **The validated prikk
range moves to 0.42**, and stikk opens on prikk's current branch, shows prikk's default beside its own
focus when the two differ, and names both refs on a confirmation (RFC 029). The real-binary suite drives
all twelve seam methods, at prikk 0.28 and 0.42 on Linux, macOS and Windows.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; these land in 0.7.0, grouped by
crate in dependency order:

| Crate | Change |
|---|---|
| `stikk-model` | `ChangeToken::compose` gains a fourth parameter, `current_branch: &CurrentBranch` |
| `stikk-model` | `StikkError::Stale` gains a field, `cause: StaleCause` — breaking for anyone constructing it or destructuring it without `..`; its `Display` now names the repository or the worktree |
| `stikk-prikk` | the `Prikk` trait gains `queue(&self, repo) -> Result<QueueReport>` — every implementor must add it |
| `stikk-prikk` | `WorktreeEntry` gains `authoring: Authoring` — prikk's per-entry commit verdict: authored, refused with prikk's reason, or unreported below prikk 0.39 |
| `stikk-prikk` | `WorktreeStatus` gains `refused: Option<u64>` (`None` below prikk 0.39, never `Some(0)`) and `declarations: Vec<RenameDeclaration>`, and its `queued_elsewhere` changes type from `Option<String>` to `Option<QueuedElsewhere>` — prikk's sentence (`Note`) below 0.39, or prikk's queued ref (`Ref`) at ≥ 0.39 |
| `stikk-prikk` | `Orientation` gains `current_branch: CurrentBranch` |
| `stikk-core` | `commit_confirm_and_execute` takes a `CommitToken` instead of a `PreviewToken`, and no longer takes `reff: &str` — the commit is authored onto the ref the preview was built for |
| `stikk-core` | `CommitPreviewOutcome` gains a variant, `WouldRefuse(Vec<RefusedPath>)` — breaking for an exhaustive `match` — and its `Ready` variant's `token` changes type from `Box<PreviewToken>` to `Box<CommitToken>` |
| `stikk-core` | `ChangeEntry` gains `authoring`; `ChangesView` gains `refused` and `declarations`, and its `queued_elsewhere` changes type as `WorktreeStatus`'s does |
| `stikk-core` | `ConfirmationSummary` gains `branch_notice: Option<String>` and `freezes: Option<FrozenPatches>` |
| `stikk-core` | `OrientationView` gains `current_branch`, and `HistoryView` gains `queued_target: Option<String>` |
| `stikk-core` | `Presentation::Stale` gains `cause` and `headline` |
| `stikk-core` | `Command` gains `needs_focused_ref: bool` |
| `stikk-tui` | `App::focused_ref` returns `Option<&str>` instead of `&str` — `None` until a ref is focused, and `App::open` no longer focuses `heads/main` before the first Orientation read |
| `stikk-tui` | `Overlay` gains a variant, `CommitWouldRefuse { paths }` — breaking for an exhaustive `match` — and `Overlay::Stale` gains `cause` and `headline`, `Overlay::RefPicker` gains `unpublished_main`, and `Overlay::Palette` gains `ref_focused` |
| `stikk-tui` | `Screen` gains a variant, `Queue { view, refreshing, offset }`, and `Focus` gains `Queue(&QueueView, &Cell<u16>)` — breaking for an exhaustive `match` |

Every struct above is constructed with struct literals by anyone scripting a `NullBackend` or rendering a
view; none is `#[non_exhaustive]`. **Three public signatures changed** — `ChangeToken::compose`,
`commit_confirm_and_execute` and `App::focused_ref` — and **the `Prikk` trait gained one method**, `queue`.
No public function, trait method or re-export was removed. The MSRV is unchanged at 1.88.

**Additive**, and not listed above:

- **`stikk-model`:** the `CurrentBranch` and `StaleCause` types.
- **`stikk-prikk`:** the `Authoring`, `QueuedElsewhere` and `RenameDeclaration` types; the queue's
  `QueueReport`, `Queue`, `QueueTarget`, `QueueThreshold`, `ThresholdStatus`, `QueuedPatch`,
  `QueuedMessage`, `QueuedOperation` and `QueuedPath`; and `NullBackend::with_queued_elsewhere_ref`,
  `with_worktree_status_on_reread`, `with_worktree_status_refusal_on_reread`, `commit_calls`, `with_queue`
  and `with_queue_refusal`.
- **`stikk-core`:** the `RefusedPath`, `CommitToken`, `FrozenPatches`, `QueueView` and `QueuedPatchView`
  types; `queue_view`, `queued_elsewhere_clauses`, `would_refuse_next_steps`, `stale_headline`,
  `stale_gloss`, `branch_notice` and `unshown_patches_line`; `ChangeKind::label`, `HistoryView::queued_tier`,
  `Command::available` and `Command::unavailable_reason`; `NO_FOCUSED_REF_REASON` and `SHORT_ID_CHARS`;
  `Target::Queue` and `OperationContext::LoadQueue`, each on a `#[non_exhaustive]` enum; and re-exports of
  `Authoring`, `QueuedElsewhere` and `RenameDeclaration`.
- **`stikk-tui`:** the `RefFocus` type, `App::ref_focus` and `App::open_queue`.

### Added

- **An entry commit would refuse is marked refused in the Changes view**, with prikk's reason verbatim
  beside it and its kind still shown, and the header counts refusals. prikk ≥ 0.39 reports this per
  entry, from the same classifier `commit` uses. Below 0.39 the view says the verdict is not reported —
  never "0" (RFC 027).
- **Commit is unavailable, with prikk's reasons, when prikk says it would refuse.** One refused entry
  refuses the whole commit, so at prikk ≥ 0.39 stikk no longer offers a commit prikk has already said it
  will refuse. It lists each refused path with prikk's reason, and says what to do: remove or replace the
  path, or list it in `.prikkignore` — which is then part of that commit. Below 0.39 commit is offered as
  before, and prikk's refusal comes back verbatim (RFC 027).
- **The Queue view** (`Q`, or the palette) lists each queued patch: its id, its operations — a rename as
  *from → to* with the key that asserted it, and a node no longer in the baseline named as one — and, on
  prikk ≥ 0.42, its message. It shows the queue's target ref and thresholds, scrolls when the queue is
  taller than the screen, and needs no focused ref. On prikk 0.39–0.41 it says once that prikk does not
  report a queued patch's message; below 0.39 it shows the count and target and says prikk does not list
  queued patches, never an empty list (RFC 028).

### Changed

- **`worktree-status` is read as JSON at prikk ≥ 0.39**, like `log`, `branch` and `tag` already are. The
  report is held to its schema: an authoring verdict prikk never emits, or a refused count that
  disagrees with the entries, is refused rather than guessed at (RFC 027).
- **At prikk ≥ 0.39 the queued-elsewhere warning is in stikk's words**, labelled as stikk's, because
  prikk's JSON report carries only the queued ref. It keeps every claim prikk's own sentence makes — the
  queue holds unsealed patches for that ref, that is real committed work not shown here, an untracked
  entry may be exactly that work, and nothing should be deleted on this view alone — and a test holds it
  to that sentence clause by clause. Below 0.39 prikk's sentence is still shown verbatim (RFC 027 F6).
- **The validated prikk range is `>= 0.28`, through `0.42.0`.** The real-binary suite passed unchanged at
  0.28 and 0.42 on Linux, macOS and Windows, and the captured fixtures were re-verified against the 0.42
  binary, except the few that record a state 0.42 cannot produce (two pre-0.33 log histories, and RFC
  027's captures of 0.28 and 0.41) (RFC 029).
- **On prikk ≥ 0.42, commit is unavailable, with prikk's reason, when a file's name cannot be a
  repository path** — a backslash in it, or bytes that are not UTF-8. prikk 0.42 began reporting those
  paths as ones `commit` would refuse, and stikk already declines to offer a commit prikk has said it
  will refuse. This is the first release in which a user sees it happen (RFC 029).
- **Where stikk opens.** On prikk ≥ 0.42, stikk opens focused on prikk's current branch. Below 0.42, or
  when prikk reports its pointer unresolved, it opens on `heads/main` only when that is published, and
  otherwise opens the ref picker with no ref focused; until a ref is focused, History, Changes, commit and
  seal say why they are unavailable, and an empty repository's picker offers `heads/main (not published)`.
  Choosing another ref in stikk still never moves prikk's current branch or touches the worktree (RFC 029).
- **The status bar shows `prikk's default: <branch>` beside stikk's focus when the two differ**, and the
  Orientation view shows prikk's current branch whole. When the status bar is too narrow it gives up, in
  order, its key hint, then the repository name (shortened with `…`), then prikk's default (shortened with
  `…`), then that segment — and never cuts the focus, the queue count or the signing badges (RFC 029).
- **Commit and seal confirmations name both refs when they differ**: the ref being targeted, and prikk's
  current branch — the ref prikk uses when no `--ref` is given. It is a notice, not a block (RFC 029).
- **On prikk ≥ 0.42, switching branches outside stikk counts as a repository change.** A terminal
  `prikk branch switch` moves no branch, tag or queue, so stikk used not to notice it; its current branch
  is now part of what stikk compares, and a commit or seal confirmation armed before the switch is stale
  (RFC 030).
- **A stale confirmation now says whether the repository or the worktree changed**, and no longer
  attributes the change to "another writer". The worktree wording names what stikk compares: which paths
  prikk lists, their status, prikk's verdict on them, and rename declarations (RFC 030).
- **On prikk ≥ 0.39, the seal confirmation names each patch it will freeze**, by short id, with its
  message on prikk ≥ 0.42. When they do not all fit, it shows as many as fit and says how many more are
  not shown, and that the Queue view lists them all; nothing else on the card gives up room for the list.
  Below 0.39 it says prikk does not list queued patches (RFC 028).

### Fixed

- **The Changes view lists unsupported paths.** Since 0.1.0 it counted them in its header and showed
  none of them: prikk names a path it cannot represent `unsupported-path`, and stikk matched
  `unsupported`, a word prikk has never printed, so every such line was dropped. Those paths also block
  commit, and prikk's reason is now visible beside each one. An entry of any kind stikk does not model
  is now listed too, rather than dropped (RFC 027 F0).
- **Every count in the Changes header is on screen at 80 columns.** The counts line used to clip
  `unsupported N` — the one number F0 is about — and now takes two rows. The headline counts the entries
  listed, so it matches the list (RFC 027).
- **An entry of a kind stikk does not model shows prikk's own word** in its tag, rather than stikk's
  `changed` (RFC 027).
- **prikk's queued-elsewhere warning is readable in full at 80 columns.** Below prikk 0.39 stikk shows
  prikk's sentence verbatim, and it was one unwrapped row: at 80 columns it stopped after the first
  clause, so "real, committed work" and "do not delete" were never on screen. It now wraps, prikk's
  words unchanged, with the quote bar on every row (RFC 027).
- **History no longer shows another ref's queued patches as this ref's "not yet sealed" tier.** The queue
  belongs to one ref, and History put its count on every ref's lineage. The tier now says whose queue it
  is — this ref's, another ref's by name, or one prikk reports no target for — and shows no line when
  nothing is queued (RFC 028 F5).
- **The Glossary no longer says prikk has no current-branch pointer.** That was true below prikk 0.42.
  From 0.42, `.prikk/current-branch` names a default for `--ref`, which prikk calls "a default, never an
  authority" — still not a HEAD (RFC 029).

### Security

- **A confirmed commit no longer authors content its preview did not show.** Before 0.7.0, a commit
  could be authored — and signed with your AUTHOR key — holding content the preview never showed, which
  is threat model `T-T4` exactly. Between a commit's preview and its confirmation, a terminal
  `prikk branch switch`, a file added or removed, or a `prikk mv` went straight into the commit, onto the
  ref the preview showed, with nothing on screen saying so. A file added was measured at prikk 0.28 and
  0.42; a branch switch and a `prikk mv`, which need prikk 0.42 and 0.38, were measured at 0.42. stikk now
  re-reads what prikk reports about the worktree immediately before committing; if it no longer matches
  the preview, stikk says the worktree changed and commits nothing. **A change made outside stikk between
  preview and Enter — an editor's autosave, a build writing a file that is not ignored — now makes the
  confirmation stale, and you preview again.** **What this cannot see is stated rather than implied
  away:** a further edit to a file the preview already listed as modified, and anything that changes in
  the moment between that re-read and prikk's own commit (RFC 030).

## 0.6.0 — 2026-09-13

**stikk says what it knows.** 0.5.0 made stikk checked against a real prikk; 0.6.0 makes it honest about
what it can and cannot tell you. **Signing readiness is read from prikk itself** instead of guessed from
your environment — which fixes a picture that was wrong in both directions on the prikk that is current
(RFC 026). **Confirmations name the key that will sign** and say whether it is already bound to this
repository, and a key prikk publishes as an example is flagged every time (RFC 023, RFC 026). **Overlays
size themselves by measurement**, so seal's confirmation shows how to confirm for the first time and no
list hides its own selection (RFC 024). **The validated prikk range moves to 0.41**, three releases, one
of which changed how prikk is configured at all (RFC 026). And **the real-binary suite now drives all
eleven seam methods**, correcting an undercount 0.5.0 shipped (RFC 022, and `readiness` in this
release's preparation).

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; these land in 0.6.0:

| Crate | Change |
|---|---|
| `stikk-model` | `Readiness`'s `author_ready: bool` and `maintainer_readiness` become `author` and `maintainer`, each a `RoleReadiness` mirroring prikk's own `binding` vocabulary |
| `stikk-model` | `MaintainerReadiness` is removed; `RoleReadiness` replaces it |
| `stikk-prikk` | the `Prikk` trait gains `readiness(&self, repo)` — every implementor must add it |
| `stikk-core` | `present()` gains a third parameter, `prikk_minor: Option<u32>` |
| `stikk-core` | `ConfirmationSummary` gains `signing_key_id`, `signing_key_claim` and `signing_key_is_published_example`; `OrientationView` gains `prikk_minor` and `stale_seed_variables` — breaking for anyone constructing either with a struct literal |
| `stikk-tui` | `Overlay::Glossary` becomes `Overlay::Glossary { offset }` — the Glossary scrolls now, and its scroll position lives on the variant |

Everything else new is additive: `RoleReadiness` and `Binding`; `ReadinessReport` and `RoleDetail`;
`stikk_prikk::key_id`; `stikk_prikk::env::stale_seed_variables` and `StaleSeedVariables`;
`stikk_model::example_keys` with `ExampleKey` and `published_example`; `KeyClaim` and
`signing_key_claim`; `NullBackend::with_readiness`; and `stikk_tui::text::wrap_indented`.

### Added

- **Commit's and seal's confirmations name the key id that will sign — and say how far that can be
  trusted.** On prikk ≥ 0.41 stikk reads the id from `prikk key status`, which always has one (it falls
  back to the role's own name when `PRIKK_<ROLE>_KEY_ID` is unset). A key this repository already
  records is stated plainly; a key that has never signed here says *this signature binds it*; on an
  older prikk that cannot report which key will sign, stikk says it is naming the id it will pass rather
  than one prikk confirmed. With no key configured, no id line is shown at all.

- **The glossary's code explanations are reachable.** Seven explanations — for schema skew, a full
  queue, a trust refusal, the Windows path refusal and others — were written and rendered nowhere: a
  refusal card named a code with no way to read it. The Glossary & Help overlay now lists them, and it
  **scrolls and wraps** (`↑`/`↓` or `j`/`k`; the panel's own Keys section says so). Wrapping without
  scrolling was tried in 0.4.1 and reverted, because it hid content rather than showing it.

- **An explanation for prikk's backslash-path refusal**, shown beside prikk's own words. `invalid name:
  backslashes are not allowed in repository paths` has two causes that need opposite fixes: on Windows
  with prikk 0.28, prikk built the path itself and the fix is upgrading prikk; anywhere else, a file
  really does have a backslash in its name. stikk knows the platform and your prikk version, so it shows
  only the cause that can apply — both only on Windows at prikk 0.28, where both genuinely can.

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.41.0`** (was `0.38.0`) — prikk 0.39, 0.40 and
  0.41. prikk 0.40 moved signing seeds out of the environment and into a key directory; see `### Fixed`
  for what that did to stikk.

- **`log`, `branch` and `tag` are read as JSON on prikk ≥ 0.39**, and as text below it. These were the
  last of stikk's text parsing, and each had cost a re-baseline. Each report's schema name is checked
  before anything is read from it, so an unknown version is refused rather than guessed at.

- **Overlays size themselves by measurement rather than estimate**, and say when a terminal is too short
  to show everything (`lines 19–40 of 40`) instead of clipping in silence. A test asserts this over every
  overlay, and a new overlay does not compile until it is covered.

- **The real-binary suite drives all eleven `Prikk` seam methods** at both ends of the supported range,
  asserting what the repository is rather than that a string parsed — including `readiness`, checked
  against prikk's own `key status` output. It also provokes a genuinely held lock and the full-queue
  precondition against real binaries, so the next re-baseline catches a class-word change by itself.

  **Correcting 0.5.0's entry:** that release said the suite covered *"four of the nine surfaces"*. There
  were ten seam methods then, not nine, and `handshake` was already driven by every test — so 0.5.0
  shipped covering **five of ten**. That section is left as released.

- **A failed suite run says in one line** whether the harness could not build a repository or stikk got
  an answer wrong.

- **Release workflow actions moved off Node 20**: `actions/checkout` to v5 in every workflow, and
  `softprops/action-gh-release` to v3 in both release steps (its only change is the Node 24 runtime).

- **Recorded, not yet acted on: prikk 0.41 marks paths a commit would refuse.** `worktree-status` now
  reports `refused paths: N` and tags each such entry `[refused: <reason>]`, which is how a worktree
  symlink finally explains why it blocks a commit. stikk shows prikk's per-path text verbatim; it does
  not yet count those paths or treat them differently.

### Fixed

- **Signing readiness was wrong on prikk ≥ 0.40, in both directions.** stikk decided readiness from
  whether `PRIKK_*_SEED` was set in your environment, and prikk 0.40 stopped reading it. So stikk
  **hid commit and seal from users who could perform them** — anyone using the key directory, which is
  what `prikk setup` produces — and **offered them to users prikk would refuse**, anyone with an old
  exported seed. On prikk ≥ 0.41 stikk asks `prikk key status`, which answers from the same computation
  `commit` and `seal` use, and withholds an action prikk would refuse — a key that does not match this
  repository's record, or a maintainer key the repository has not adopted — instead of offering a
  failure. On **prikk 0.40 exactly**, which has no way to ask, stikk reports readiness as **unknown**,
  says why, and points at 0.41. A `PRIKK_*_SEED` still set on a prikk that ignores it is called out.

- **Seal's confirmation shows `Enter to confirm · Esc to cancel`.** It never had — at any terminal size,
  on the path every user takes — since seal shipped in 0.4.0: the confirmation's own explanation wrapped
  to more rows than the overlay had reserved, and the instruction fell outside the box.

- **The commit message prompt shows its footer and the text being typed** at 80×24, for the same reason.

- **Lists keep their selection on screen.** The ref picker rendered from the top and clipped, so holding
  `↓` past the fold moved the selection somewhere invisible — since 0.1.0. `Recent refusals` had the
  same defect (it holds fifty entries and about twenty fit at 80×24). Both now scroll with the cursor,
  and the command palette, whose ten commands fit today, is covered for when they stop fitting.

- **A long refusal keeps its quote bar on every wrapped row**, so you can tell prikk's words from
  stikk's all the way down, not only on the first line.

- **prikk 0.28 cannot commit a file in a subdirectory on Windows**, and Getting Started now says so. The
  defect is prikk's and was fixed in prikk 0.29.0; it is documented because 0.28 is still supported.

- **`StikkError::LockConflict`'s documentation** described a presentation stikk stopped using in 0.4.0.
  Documentation only.

### Security

- **`C-S2` is implemented: prikk's published example keys are flagged.** A signature made with a key
  from prikk's own documentation is forgeable by anyone who has read it. stikk compares the public key
  that would sign against the two example keys prikk publishes in its user-facing documentation and
  warns on **every** confirmation that would use one — persistently, not a dismissable notice. It covers
  the documentation examples, which is where a person following a guide copies a key from; it cannot
  cover values in prikk's tests or issue comments, which prikk's own rule also names and no list can
  enumerate. The threat model states both halves. No secret is stored: the values compared are public
  keys.

- **stikk now invokes `prikk key status`, and it is the only `prikk key` subcommand it may invoke.** The
  rule (`C-I1e`) forbids stikk creating key material or reading a seed; `key status` does neither — prikk
  describes it as *"reads only, signs nothing"*. The source-level guard that enforces the rule was
  narrowed to exactly that subcommand and still refuses `key generate`, `key public` and any other,
  however the call is formatted.

- **No seed value is read anywhere in stikk**, on any prikk version. The two modules that touch signing
  variables — presence in `stikk_prikk::env`, key ids in `stikk_prikk::key_id` — are each held to that by
  a source-level test, and no other shipped code names a seed variable except in text shown to you.

## 0.5.0 — 2026-09-12

**stikk is checked.** 0.4.0 was the release where stikk writes; this is the one where what it writes is
verified against a real prikk rather than against captured strings and a person's memory. Four
increments: a **real-binary integration suite** drives commit and seal against actual prikk binaries at
both ends of the supported range, asserting what the repository *became* rather than what stikk parsed
(RFC 019); a **supply-chain gate** runs advisories and licences over the dependency tree, because the
advisory that forced this release's MSRV raise was found by a person reading a report and nothing in CI
would have caught it (RFC 020); the **validated prikk range moves 0.33 → 0.38**, five releases at once
and the first re-baseline a machine performed (RFC 021); and a **fabricated worktree entry** — stikk
showing a file that does not exist — is fixed (RFC 021 F0). The security-relevant half of that list is
under `### Security`, including what the suite does *not* cover.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; these land in 0.5.0:

- **The minimum supported Rust version is now 1.88** (was 1.85). Raised to clear
  [`RUSTSEC-2026-0009`](https://rustsec.org/advisories/RUSTSEC-2026-0009.html), a parsing DoS in the
  `time` crate that stikk carries transitively through ratatui's calendar widget (never rendered, but
  not optional in the facade stikk depends on) — the fix needs `time >= 0.3.47`, which needs Rust 1.88.
  A consumer on an older toolchain can see plainly what they're being asked to trade for.
- **ratatui 0.29 → 0.30**, the dependency the MSRV raise rode in on. **stikk declares no API change of
  its own from it** — no struct gained or lost a field, no signature changed shape — **but ratatui is a
  *public* dependency of `stikk-tui`, so the major propagates**: `Palette`'s five colour fields are
  `ratatui::style::Color`, and `stikk_tui::shell::render` takes a `&mut ratatui::Frame`. A consumer of
  `stikk-tui` must therefore move to ratatui 0.30 as well — recompiling against 0.29 will not work, and
  that is the concrete reason this row forces the minor rather than merely accompanying it. (Internally
  the migration was `Alignment` → `HorizontalAlignment` and nothing else.) Consumers of the other five
  crates — `stikk-model`, `stikk-prikk`, `stikk-state`, `stikk-core` — are unaffected: none depends on
  ratatui at all.

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.38.0`** (was `0.33.0`) — five prikk releases
  at once, the widest re-baseline this project has done, and **the first one a machine ran rather than
  a person** (RFC 021, using the real-binary suite RFC 019 built for exactly this). What the run found
  across 0.34–0.38, each verified against a real binary rather than taken from a changelog:
  - **Six error messages changed their class word at 0.35**, `lock conflict:` → `precondition not met:`,
    with the message text itself byte-identical. stikk's classifier was unaffected — it has matched each
    message's own semantic clause rather than its class prefix since RFC 017 — and both wordings are now
    pinned, because both are still reachable: a user on prikk 0.34 sees the first, one on 0.35+ the
    second, and stikk supports the whole range.
  - **`worktree-status` gained an unconditional `live rename declarations: N` section at 0.38.** This is
    the one that was already fixed, under Fixed below: stikk read those lines as worktree changes.
  - **`prikk trust maintainer add` stopped printing `policy: required=1` and now reports `adopted
    maintainer keys: N`** (0.34). stikk never invokes that command — it is outside the `C-I1e` boundary
    — so nothing here depended on either wording.
  - **Everything else stikk parses is unchanged**: `status`, `log`, `commit`, `seal`, `branch list`, `tag
    list`, and `checkout --patch-plan` are byte-identical at 0.38 to their 0.30–0.32 captures. That was
    claimed upstream and is now checked.
  - prikk also answered three long-standing gaps in this window — queue enumeration (0.35), per-patch
    content (0.36), and trust enumeration (0.34). **No view in stikk uses them yet**; the requirements
    they unblock are amended to say so, and each names the release that changed the answer.

### Fixed

- **On prikk ≥ 0.38, a renamed path whose name began with a change-kind word appeared in Changes as a
  modified file that does not exist.** prikk 0.38 prints a `live rename declarations:` section, and
  stikk's worktree-status parser scanned *every* indented line in the report rather than only the ones
  under the `worktree:` headline — so after `prikk mv "modified draft.txt" renamed.txt`, the
  declaration line `modified draft.txt -> renamed.txt` was read as a fourth kind of change entry.
  stikk showed three changes where prikk reported two, the third a file in a state prikk never named.
  The entry scan is now bounded by its section — the headline and the next flush-left line — rather
  than by indentation alone, so the next section prikk adds cannot do this again. Reachable in 0.4.1
  on prikk ≥ 0.38, which stikk runs against deliberately (saying the range is unvalidated rather than
  refusing) — it needed that version, a `prikk mv`, and a path whose first word was a change kind
  (RFC 021 F0).

  **prikk's maintainers found this by reading stikk's parser and predicted the exact line before we
  reproduced it** — the second time an upstream reading of this project's code has been exactly right,
  and worth recording where a user can see it.

### Security

- **stikk's mutations are now exercised against a real prikk binary, not only against captured
  strings.** Until this release, every one of stikk's tests ran against a scripted backend or a
  recorded fixture: four releases shipped — including the one that writes to repositories — verified
  by a person running commands by hand and pasting the output into a review. The **real-binary
  integration suite** (RFC 019) makes that repeatable. It drives the same `CliBackend` the product
  uses against real prikk **0.28.0 and 0.38.0** — both ends of the supported range — through a
  throwaway repository, and asserts **what the repository became**, re-read afterwards, rather than
  what stikk parsed out of prikk's reply: after a commit the patch is actually queued; after a seal
  the queue is empty and the new block exists; the repository still verifies clean. It also confirms
  that the two refusals stikk *prevents* client-side — committing against the wrong ref, sealing an
  empty queue — are refusals a real prikk genuinely gives, which until now was an inference from
  reading prikk's source in two separate increments.

  **What it does not cover, stated because a Security entry is read to decide what to trust:** the
  suite exercises **four** of the nine surfaces stikk parses — `commit`, `seal`, `orientation` and
  `history`. It does not touch `worktree-status`, `refs`, `tags`, `block_state` or `change_token`, and
  it does not reach the failure-classifier fixtures at all; those remain covered by captured strings
  and by reading, as before. Widening it is the next infrastructure increment. A green run here means
  four surfaces at two prikk versions on three platforms — a real result, and not the whole product.

- **A supply-chain gate now runs over the dependency tree** (RFC 020): `cargo-deny` across both
  advisories and licences, in one report. It exists because
  [`RUSTSEC-2026-0009`](https://rustsec.org/advisories/RUSTSEC-2026-0009.html) — the `time` parsing DoS
  that forced this release's MSRV raise — **was found by a person reading an advisory feed, and
  nothing in CI would have caught it.** The licence half is there because stikk publishes six
  Apache-2.0 crates and a copyleft dependency arriving transitively is a thing nobody here was
  positioned to notice. By the owner's ruling it is **non-blocking on a pull request** — an advisory
  published overnight must not redden unrelated work — and **required, and read by a human, before a
  release**. Every ignore entry must carry a dated reason and a removal condition; the list is
  currently empty.

- **The minimum supported Rust version now has one source.** It was stated in five places, three of
  which went stale the day it moved — including the workflow that cuts releases, which would have
  failed *after* a tag was pushed, mid-publish, with the version number already spent. Every workflow
  now derives it from `Cargo.toml`, and the rule for finding such copies is a command over every
  tracked file rather than a list of directories to remember (RFC 020 F1/F3).

## 0.4.1 — 2026-09-06

### Fixed

- **The Glossary told users stikk never writes their repository, four lines above the keybindings for
  `commit` and `seal`.** 0.4.0 published this exact contradiction — one released version, hours — and
  it was found the same day, before a user reported it. The line was anchored to a feature set (true
  while stikk had no mutations, false the moment it gained one) rather than to anything that couldn't
  change; it is now anchored to the architecture instead: every repository write happens inside prikk
  itself, never as a direct write stikk performs, which was true before commit/seal existed and stays
  true of whatever mutation lands next (RFC 018 F1).
- **A signing-readiness refusal pointed at "Glossary → Trust & Keys" — a section that does not exist.**
  The Glossary's only relevant section is titled `Keys`, and it is the keyboard-shortcut table; a user
  following the old pointer landed on what `j` and `k` do. The pointer is removed rather than
  redirected: there is nowhere real to send it until the Trust & Keys view (`FR-104`) exists (RFC 018
  F2).
- **The Glossary's Git→prikk terminology still said commit messages "are not yet persisted."** That
  stopped being true at prikk 0.32 (RFC 015); the entry now states the version boundary and that author
  name/email/date remain permanently absent by design, not "not yet" (found sweeping the same panel for
  RFC 018 F1).
- **The same table's two longest terms ran straight into their own descriptions, with no separator**
  (`checkout / switch branch`, `merge conflict / resolve` — both 24 characters against a hardcoded
  22-character column pad). The column width is now computed from the longest term. **The table's notes
  still truncate at the box edge** — wrapping them was tried and reverted: without a way to scroll past
  what wrapping pushes down, it turned truncated-but-present into absent, leaving as few as one of eleven
  terms reachable at all on an ordinary 80×24 terminal. Wrap and scroll are filed together for 0.5.0, not
  shipped separately (RFC 018 C1/C2).
- **`ROADMAP.md` held a validated-ceiling claim three releases stale** (`0.30.0`, current since RFC 015
  raised it to 0.31 and RFC 017 to 0.33) **and two un-retired `UD-` rows** (`UD-01`'s messages, retired
  at prikk 0.32; `UD-09`'s per-patch enumeration, narrowed at the same version) — outside every version
  grep this project has run, because `ROADMAP.md` was outside every grep's scope (RFC 018 F3). The file
  is now current with 0.4.0, and the grep scope is a standing exclusion-list rule rather than an
  inclusion list widened again after the fact.

## 0.4.0 — 2026-09-06

**stikk writes.** 0.1.0 through 0.3.0 were read-only by design; this release is where that changes.
Six increments: **commit** authors a worktree capture into prikk's active queue (RFC 014); **seal**
freezes that queue into permanent, MAINTAINER-signed history (RFC 016); both mutations sit behind
machinery that makes skipping a step a compile error, not a review finding (RFC 013), stamped with a
change token that refuses to execute against a repository that moved since preview (RFC 003); two
re-baselines (prikk 0.32, RFC 015; prikk 0.33, RFC 017) keep every parsed shape and classified error
grounded in what a real prikk binary actually emits, never in what stikk assumed it would. Preview-first
is not a convention this release's reviewers enforced — it is a property of the type system.

It also corrects four things stikk had been getting wrong, three of them since 0.1.0, all found by
review rather than by a user — see Fixed.

### Breaking

Per RFC 011, for a `0.x` crate the minor version is the breaking position; 0.4.0 carries these:

| Crate | Change | Who it breaks |
|---|---|---|
| `stikk-prikk` | `Prikk` gained **three required methods**: `change_token`, `commit`, `seal` | anyone implementing `Prikk` outside the crate |
| `stikk-prikk` | `Orientation` gained the field `active_patch_warning: Option<String>` | anyone constructing it |
| `stikk-prikk` | `BlockRow` gained the field `messages: Vec<PatchMessage>` | anyone constructing it |
| `stikk-model` | `Readiness::maintainer_ready: bool` → `maintainer_readiness: MaintainerReadiness` (a new three-valued enum) | anyone constructing or reading it |
| `stikk-core` | The palette's `Command` struct: `min_capability: Capability` removed; `operation: &'static str` and `tier: Tier` added | anyone constructing a `Command` or driving the palette |
| `stikk-core` | `Command::available_to`/`unmet_reason` now take `Readiness`, not `Capability` | anyone calling them |
| `stikk-core` | `OrientationView` gained the fields `validated_through: String`, `prikk_persists_messages: bool` | anyone constructing it |

`StikkError`, `Presentation`, `OperationContext`, and `Target` all gained variants in this release
(`Stale`, `Declined`, `CrossRef`; `Presentation::Stale`; `OperationContext::Commit`; `Target::Seal`) —
**not breaking**: all four are `#[non_exhaustive]`, which is exactly what that attribute is for.

### Added

- **Commit** — the first mutation stikk performs. `Prikk::commit` authors the whole worktree capture
  (there is no staging) as a new patch in the active WAL, previewed against a fresh read of the real
  worktree and queue — never a stale on-screen view — and gated behind an explicit AUTHOR-tier
  confirmation restating what changes. Two refusals prikk would otherwise give are **prevented**
  client-side, before anything is offered, rather than classified after the fact: a commit whose focused
  ref does not match the active WAL's queue target, and a commit against a clean worktree with nothing to
  author. Every note prikk prints — the perpetual diff-minimization caveat, and, on a prikk below 0.32,
  that the message is validated but not stored — is carried through verbatim as a list, never fixed
  fields, since which notes a given prikk prints is not stable across versions (RFC 014).
- **Preview-first and tiered confirmation** — the gate every mutation in this release sits behind,
  enforced by the type system rather than by convention: `preview() → PreviewToken → confirm() →
  ConfirmedToken → execute()`, where neither token has a public constructor and `execute` takes
  `ConfirmedToken` by value, so code that skips the preview or the confirmation does not compile. The
  confirmation tier is derived from the request's category alone, never declared per operation, so a new
  mutating operation cannot be added un-gated. A repository that changed between preview and
  confirmation, or between confirmation and execution, refuses as its own distinct outcome — attributed
  to stikk, never rendered under prikk's own "reported" label — with exactly one next step: preview
  again, never a retry of the execution (RFC 013).
- **The repository change token** — the primitive the machinery above sits on. A preview is stamped with
  a token composed from every ref's current pointer plus the active queue's depth and target; confirming
  or executing re-reads that same signal set and refuses on any difference, so a preview computed under
  one repository state cannot execute against another. A repository **fingerprint** — a coarser,
  persistent identity for session/cache keying — was proposed alongside the token and deliberately not
  built: prikk states as a documented security property that repositories are anonymous, deriving one
  client-side would mean walking a repository's entire history on every open, and it would be absent for
  exactly the repositories most likely to be newly created (RFC 003).
- **`UD-01` retires.** Commit messages are validated and discarded below prikk 0.32; at 0.32 and above,
  prikk persists them, and `prikk log` names a patch id and message for every patch that carries one.
  stikk supports both sides of that boundary rather than asserting either blanket claim: the
  commit-message prompt's copy reads this session's own handshake, and Block detail shows the id/message
  list a messaged patch carries, always beside the block's own patch count — so a pre-0.32 patch's
  absence from the message list is never mistaken for its absence from history (RFC 015).
- **Seal.** Freezes the active WAL's queued patches into new, MAINTAINER-signed history — behind a
  preview that prevents an empty queue or a wrong-ref target client-side, a tier-3 confirmation stating
  plainly that a trust refusal is possible and that success is never promised, and a separate, unchecked,
  undefaultable no-audit acknowledgement. Two deliberate acts, never collapsed into one keypress: the
  confirmation and the acknowledgement are distinct screens, and the second cannot be skipped by a
  reflexive `Enter` carried over from the first (RFC 016).
- Maintainer-trust refusals now classify `NotReady` (classifier only; Trust & Keys presentation lands
  with the seal ceremony) (RFC 017).

### Fixed

- **The `[MNT]` badge has been claiming a readiness it could not verify since 0.1.0.** It showed
  MAINTAINER as ready from signing-key presence alone, never checking whether the repository's trust
  policy had actually adopted that key — a fact no currently supported prikk exposes a way to check
  either. The badge is now three-valued: `✓` (adopted), `–` (no key material), `?` (key material
  present, adoption unverifiable on any supported prikk). `?` never renders as a pass (RFC 016).
- **stikk no longer claims another writer is active when the queue is simply full** (RFC 017 F4). The
  commit path's full-queue precondition was classified as a lock conflict, rendering "another writer is
  active" directly above prikk's own "run `prikk seal`"; it now reaches its own honest explanation
  instead — nothing is locked, seal the queue — and now also offers to seal directly.
- **Five classifier arms matched text prikk has never emitted**, at any version, written from prikk's
  prose rather than captured from its output — dead since 0.1.0. All five are removed (`FR-003`'s
  invented retired-format string, asserting a `prikk migrate` command that does not exist, among them,
  replaced with one grounded on a live-captured migration message); each degrades safely to a verbatim
  refusal, exactly as designed. A foreign directory's `Environment` classification is now grounded on
  the arm that actually catches it, not the differently-worded arm originally written for it (RFC 017).
- **A refusal card's next-step could be silently clipped off screen.** Schema skew's "Upgrade prikk" has
  been invisible at ordinary terminal widths since 0.3.0 — the card's box height was sized from a count
  of logical lines, not the rows a long gloss or verbatim message actually wraps to, found while building
  the seal ceremony's own trust-refusal card. Every refusal card's next-step list now has its own region,
  sized exactly and never squeezed out by wrapped prose (RFC 016).
- A trust refusal reaching the seal ceremony now names what adoption actually requires (`prikk trust
  maintainer add`, done outside stikk) and admits stikk cannot verify it afterwards — rather than
  degrading to a bare, unexplained refusal (RFC 016; RFC 017 F5).
- **Both the TUI's Orientation view and the launcher's one-shot print stated a stale validated ceiling**
  ("0.30") through all of 0.3.0, after RFC 012 had already raised it to 0.31 within that same release —
  the notice built specifically for version honesty was itself stale. Both now read
  `validated_ceiling_display()`, the single source, with a regression test at each call site asserting
  against an arbitrary sentinel rather than against whatever the ceiling currently is — the pattern that
  let the original drift go unnoticed for a release (RFC 015).

### Security

- The `prikk key` / `prikk setup` boundary is declared in the threat model and enforced by test: stikk
  never invokes either, and never quotes `setup`'s hard-coded `policy: required=1` line as if it were
  read policy (RFC 016/017).

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.33.0`** (was `0.31.0`). No output shape
  changed between 0.31 and 0.33 for any command stikk parses; every parser fixture was re-run against
  real 0.32.0 and 0.33.0 binaries and confirmed unchanged, and independently confirmed unchanged at the
  source level for 0.33 (RFC 015, RFC 017).

## 0.3.0 — 2026-09-05

Responsiveness and correctness (RFC 010 + RFC 012). The UI no longer blocks on a seam call, and five
correctness/honesty defects found by review — not by test — are closed: read-only mode now actually
denies recovery, version skew stops pointing users at their signing keys, config and state resolve on
every platform stikk ships binaries for, ref names are validated at the seam boundary, and the ref
picker finally shows tags. Still **no mutations**; cancellation stays deferred to `FR-100`.

### Breaking

| Crate | Change | Who it breaks |
|---|---|---|
| `stikk-prikk` | `Prikk` gained the `Send + Sync` supertrait | anyone implementing `Prikk` outside the crate |
| `stikk-prikk` | `Prikk::tags` added (a required method) | same |
| `stikk-model` | **`Capability::may_operate` removed**; `Readiness::may_operate` added in its place | anyone calling it — and the *semantics* changed too: read-only now denies recovery |
| `stikk-tui` | `App`'s navigation methods no longer take `&impl Prikk`; results arrive via `App::apply` | anyone driving `App` directly |

### Added

- **The UI no longer blocks** (RFC 010; `NFR-P01` was a Must and was unmet). Every seam-driven read now
  runs on a worker thread via `std::thread::scope`, one worker, no thread pool, no async runtime; the
  render/input loop never waits on it. Load states that existed in the design but could never actually
  be seen — `Loading` — are now real and observable, shown honestly rather than as a frozen or blank
  pane. A response for a view the user has since navigated away from is discarded by sequence number
  rather than surfacing over an unrelated screen — the correctness risk this increment exists to close.
  The status bar's `⟳ n` indicator and the Background Operations overlay (a listing; no cancel action)
  show what is in flight.
- **Config and state now resolve on macOS and Windows** (RFC 012 F-c) — platforms 0.1.0 and 0.2.0
  shipped binaries for without ever resolving paths for. macOS uses `~/Library/Application Support`;
  Windows uses `%APPDATA%`/`%LOCALAPPDATA%`. Linux paths are byte-identical to before this release.
  `STIKK_CONFIG`/`STIKK_STATE_DIR` still win outright. `stikk-state` stays dependency-free.
- **Tags appear in the ref picker** (`FR-014` completed). A real `prikk tag list` read joins the
  existing branch listing, merged and de-duplicated by name.

### Fixed

- **Read-only mode now actually denies recovery actions** (RFC 012 F-a; `FR-121`). The external design
  had said Operator was "always tier 3" regardless of read-only mode, which the code could not even
  implement correctly and, if it had, would have been a read-only mode that mutates — exactly the
  confident-but-wrong picture this project refuses. `Readiness::may_operate` is `!read_only`; external
  design `AC-04` is corrected to match.
- **Version skew stops pointing users at their signing keys** (RFC 012 F-b). A too-old prikk opening
  Changes now names the prikk-version gate as the target, not Trust & Keys.
- **A repository written by a newer prikk gets an explanation, not a bare refusal** (RFC 012 F-e). prikk
  0.31 is forward-incompatible (no CLI change, but its repositories cannot be read by 0.30 or earlier);
  the resulting envelope-schema-skew refusal now resolves to a glossary entry and an "upgrade prikk"
  next-step, recognized by the refusal's stable message shape rather than by widening the integrity
  classifier.

### Security

- Every golden fixture in `stikk-prikk`'s parser tests was re-captured against a real prikk **0.31.0**
  binary and diffed byte-for-byte against the committed 0.30.0 captures — identical in every case
  (RFC 012 F-e).
- **Ref names are now validated at the seam's parse boundary** (`INV-9`; RFC 012 F-d), the same
  discipline 0.2.0 applied to object ids: an empty or control-character-bearing name refuses rather than
  travelling further as an unvalidated string. Display was already inert (`C-T2a`) and remains so —
  validation and inert rendering are complementary, not alternatives.
- **RFC 009's "`branch list` cannot emit a tag" claim was found and corrected** while building the tag
  list read above: prikk's own `branch list --all` does not filter by ref namespace, so a tag can appear
  there too. This was an untested inference, not a checked fact — the same failure class RFC 009 itself
  exists to prevent, just the opposite sign (assuming an absence rather than a fabricated presence).
  stikk does not depend on `branch list` either including or excluding tags either way: the ref-list
  merge de-duplicates by name so the result is correct regardless.

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.31.0`** (RFC 012 F-e). A prikk above the
  ceiling still runs; Orientation says its output shapes have not been checked, rather than assuming
  they have.

## 0.2.0 — 2026-09-04

The prikk 0.30 re-baseline and parser-fidelity corrections (RFC 009). Running shipped 0.1.0 against a
real prikk 0.30.0 repository found that Orientation refused to open **any repository with queued
work**, at every prikk version stikk claimed to support — not new drift, but a golden fixture that was
written rather than captured. This release fixes that and three related parser defects, closes a live
threat-model violation, and re-baselines the validated prikk range.

### Breaking

For a `0.x` crate the minor version is the breaking position: `^0.1.0` resolves `0.1.1`, so a public
field added on what looked like a patch would break any downstream code constructing these structs with
struct-literal syntax. That is why this release is `0.2.0`, not `0.1.1` (RFC 011). Five structs gained
fields; none is `#[non_exhaustive]` (RFC 011 decides against adding it before 1.0 — see that RFC for the
reasoning):

| Crate | Struct | Field added |
|---|---|---|
| `stikk-prikk` | `Handshake` | `validated: bool` |
| `stikk-prikk` | `Orientation` | `queued_target: Option<String>` |
| `stikk-prikk` | `WorktreeStatus` | `queued_elsewhere: Option<String>` |
| `stikk-core` | `OrientationView` | `queued_target: Option<String>`, `prikk_validated: bool` |
| `stikk-core` | `ChangesView` | `queued_elsewhere: Option<String>` |

If you construct any of these five with struct-literal syntax, add the new field(s) before upgrading.

### Fixed

- **Orientation no longer refuses on queued work** (F1). `prikk status`'s `queued patches: N targeting
  <ref>` line — present since prikk 0.18.0 — is now parsed correctly, and `Orientation`/
  `OrientationView` carry the queue's target ref (`queued_target`), shown as "N queued · targeting
  `<ref>`".
- **An unpublished `heads/main` no longer becomes a fabricated object id** (F2). `status`'s
  `<not published>` sentinel — and any future unrecognized sentinel — is now recognized (or refused,
  never guessed) alongside `log`'s `<none>`.
- **An empty repository no longer produces a phantom ref** (F3). `prikk branch list --all`'s `no
  branches` line used to parse as `RefEntry { name: "no", id: "branches" }`; it is now the empty list.
  `branch list` cannot emit a tag — the seam's doc comments and the ref-picker gap are corrected to say
  so; a real `tag list` read is tracked, not built here.
- **A prikk usage error (exit `2`) no longer wears prikk's voice** (F6). prikk 0.28 split its exit
  contract into success / operational failure / usage error; a bad argument list stikk assembled now
  surfaces as a stikk-internal fault, never as one of prikk's own refusals.

### Security

- **Closed a live confident-but-wrong-picture violation** (F4; threat model `T-T4`/`C-T4c`, recorded in
  the threat model's §5.1, *Closed violations*). When the active WAL holds queued patches for a ref
  other than the one being reviewed, prikk's `worktree-status` says so explicitly — those "untracked"
  paths may already be committed, unsealed work. Shipped 0.1.0 silently dropped that warning and then
  showed its own contradicting "a commit still captures them" banner. The Changes view now carries
  prikk's warning verbatim into a distinct band above the entries, and suppresses the contradicting
  claim while the warning is present.
- **`UD-08` retired.** `.prikkignore` shipped in prikk 0.29; the design set, the Changes view's copy,
  and the glossary no longer say prikk has no ignore mechanism. The malformed-`.prikkignore` refusal now
  has a glossary entry and a next-step that can actually resolve it.
- Every golden fixture in `stikk-prikk`'s parser tests is re-captured verbatim from a real prikk 0.30.0
  binary and carries a provenance comment naming the command and version; a regression test enforces
  the rule going forward.

### Changed

- **The validated prikk range is now `>= 0.28`, through `0.30.0`** (owner-ruled 2026-09-04; `0.27.x`
  dropped — its `worktree-status` was already the `UD-03` defect stikk refused to run). A prikk above
  the validated ceiling still runs; Orientation states that its output shapes have not been checked,
  rather than silently assuming they have (`Handshake`/`OrientationView` gain a `validated` field
  alongside `supported`).

## 0.1.0 — 2026-09-03

The first public release: a **read-only preview** of stikk — orientation, ref history, worktree
changes, and the refusal-explanation / glossary surfaces over the prikk version control system. It was
built increment by increment, design- and security-first, under prikk-grade gates (`fmt` /
`clippy -D warnings` / `test`). It performs **no repository mutations** yet; the working cycle
(commit → seal → merge, always preview-first) lands in later releases.

stikk drives the external `prikk` binary at runtime (not a Cargo dependency): this release is validated
against **prikk ≥ 0.27**, and the worktree-changes view needs **prikk ≥ 0.28**.

### Added

#### Foundation — workspace, kernel, seam, state, operation layer

- **Workspace and lint discipline.** Six crates (Rust 2024, MSRV 1.85) — `stikk-model`, `stikk-prikk`,
  `stikk-state`, `stikk-core`, `stikk-tui`, and the `stikk` launcher — under a virtual-manifest
  workspace. `unsafe` is forbidden; public items must be documented; the panic-prone clippy lints warn.
- **`stikk-model`** — the shared kernel: the `StikkError` taxonomy (seven presentation classes,
  `#[non_exhaustive]`, `source()` implemented — a lesson from the prikk audit); validated
  `ObjectId`/`RefName` newtypes; the nine `RequestCategory` values carrying their policy as data; and
  the `Capability`/`Readiness` derivation.
- **`stikk-prikk` — the prikk seam.** The `Prikk` trait; a `CliBackend` driving the `prikk` binary
  (draining output fully before classifying the exit — the EPIPE guard); a version handshake with a
  validated-range gate; a scripted `NullBackend` for offline testing; and the presence-only
  key-readiness reader.
- **`stikk-state`** — user-scope config, session, and handle stores: a forgiving config parser that
  preserves unknown keys and never blocks launch; repository discovery; and the path resolver's
  repository-internal write refusal.
- **`stikk-core`** — the operation layer both frontends drive, starting with the read-only `orient`.
- **The `stikk` launcher** — `--version`, `--help`, `config check`, `config path`, and opening a
  repository (the TUI on a TTY, a one-shot orientation print off one).

#### Interactive TUI — shell & Orientation (RFC 001)

- **`stikk-tui`** on `ratatui` + `crossterm`: the shell (header, active view, status bar, overlay
  layer), the Orientation view (`VW-01`/`FR-002`), the status bar (`TU-03` — repo, focused ref, queue,
  capability/readiness badges; never a "HEAD"), global key dispatch through a single `Action` seam, the
  light/dark/mono palette (fixed-RGB text so labels stay legible on any terminal theme — NFR-A03), and
  the panic-safe terminal guard.
- **The inert-text primitive** (`C-T2a`): every repository-sourced string is stripped of control
  characters before it reaches a cell.

#### History & inspection (RFC 006)

- The seam grew `history`, `block_state`, and `refs`; `stikk-core` gained `history_view`,
  `block_detail`, and `list_refs`; the app became a **view stack**.
- **History view** (the unsealed queue tier above the sealed block lineage) and **Block detail** (a
  block's metadata + the replayed tip state), with a **ref picker** for the client-side focused ref.
- Block granularity is prikk's ceiling: **Patch detail is deferred behind UD-09** — prikk exposes no
  per-patch content and no `show`/`diff`, so stikk shows lineage and a block's state file list and
  names the gap rather than faking a diff.

#### Explanation & discovery (RFC 007)

- **One class → presentation mapping** in `stikk-core` (`present`), so the TUI and a future GUI cannot
  diverge (ER-03), and a **confined, version-gated failure classifier** in the seam that maps prikk's
  collapsed 0/1 exit to an error class and **degrades an unknown message to a verbatim refusal** (UD-05).
- **The refusal overlay** (`TU-08`): prikk's message verbatim and inert, a plain-language gloss, and
  **stikk-authored next-steps** (never parsed from the message — `C-T2b`), plus glossary links.
- **The glossary asset** (`DM-09`): the Git→prikk terminology mapping, with a missing-code degradation
  that shows prikk's message rather than hiding it.
- **The in-memory session refusal history** (`DM-06`/`FR-112`) and the **command palette** backed by an
  operation registry (`TU-07`/`FR-125`), with below-capability entries shown disabled with a reason.

#### Worktree changes (RFC 008)

- The seam grew `worktree_status`; `stikk-core` gained `changes_view` (version-gated at **prikk ≥ 0.28**,
  with honest guidance below it rather than the pre-fix command).
- **The Changes view** — worktree-vs-baseline at the path level prikk reports
  (modified / missing / untracked / unsupported), with the **UD-08** display-only untracked filter (it
  always says a commit still captures the hidden files), the **UD-06** whole-worktree reminder, and the
  **UD-09** per-file-content-diff note.
- Verified against the live binary that prikk's `worktree-status` is **fixed as of prikk 0.28** (the
  audit's `UD-03` was a 0.27.x defect); a dirty worktree's non-zero exit is treated as a normal status,
  not a refusal. **Compare is deferred** — no honest two-tree command exists, and a partial one would
  mislabel differing files as identical (`T-T4`).

#### Distribution & docs

- **CI** for `fmt` / `clippy` / `test`; a **release** workflow (crates.io via Trusted Publishing, plus
  checksummed, build-provenance-attested binaries for six targets) and a **docs** workflow (mdBook + the
  rustdoc API → GitHub Pages), both least-privilege and tag/branch-gated.
- The workspace is laid out as **six peer crates under `crates/`** with a virtual root manifest, so bare
  `cargo` commands cover every crate by default.

### Security

- The seam reads `PRIKK_*_SEED` **presence only, never their values** — enforced by a source-level
  guard test.
- The path resolver **refuses any repository-internal write target** — the primary boundary control,
  since prikk has no foreign-file backstop.
- Repository/prikk-sourced strings are rendered **inert** (`C-T2a`); refusal next-steps are
  **stikk-authored, never parsed from prikk's message** (`C-T2b`); prikk's message is **preserved
  verbatim** (`ER-02`); and stikk **never fakes a diff** where prikk exposes no content (`T-T4`).

### Notes

- 164 tests pass; `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features
  -D warnings` are clean.
- Deferred behind upstream gaps, recorded as stated properties rather than surprises: Patch detail and
  Compare's content view (`UD-09`), and a two-tree compare command.
- RFCs accepted: 001 (frontend toolkit), 006 (history & inspection), 007 (explanation surface), 008
  (worktree changes). The GUI toolkit and several Program-Design decisions remain deferred by design.

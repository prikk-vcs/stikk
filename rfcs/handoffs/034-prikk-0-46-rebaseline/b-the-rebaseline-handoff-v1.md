# Handoff B — RFC 034: the re-baseline, on prikk's own verdict (v1)

**Companion to:** [RFC 034](../../accepted/034-prikk-0-46-rebaseline.md), **accepted 2026-09-22, Q1 ruled (b)**.
**Follows** [Handoff A](./a-the-repair-handoff-v1.md), on `main` at `ad95152`, green.
**Covers:** RFC 034 **decision 4** — the work RFC 033 measured and carried (its F7), **re-measured at 0.46 by the
architect** and reproduced below so nothing is re-derived.
**Not a release.** 0.9.0 stays unreleased; nothing is tagged.

> **What this increment is.** RFC 032 had stikk *infer* what a commit would do with a rename declaration, because
> prikk would not say. **prikk now says.** This replaces stikk's inference with prikk's own resolution, turns a
> declaration prikk will refuse into a prevention rather than a notice, and shows a checkout that stopped part-way —
> which stikk has never shown at all.
>
> **The floor:** every sentence must be true of the prikk that produced the report it describes. Bands are named
> below; do not widen one without measuring it.

---

## 1. The band, and why it is 0.44

**Declaration work (§3–§6) takes prikk's verdict at ≥ 0.44, and keeps RFC 032's inference below it.**

**0.43 is not used, deliberately**, although its fields exist:
- **its classifier is wrong in a measured state** — a directory at the destination resolved `rename` while `commit`
  recorded a deletion (stikk letter 014; fixed in 0.44);
- **prikk reports 0.43 hangs on a FIFO destination** (their letter 017 §2), which stikk would meet through
  `worktree-status`;
- **`content_changed`/`mode_changed` changed meaning in 0.44** for a destination that is not a regular file.

**One band, ≥ 0.44.** Splitting a band for a single superseded release buys nothing and costs a test matrix.

**The marker (§7) is a separate band, ≥ 0.43**, where `status-report-v1` first carries it — measured at 0.43 and at
0.46, identical.

## 2. Measured at prikk 0.46.0 — do not re-derive this

One fresh repository per row, `a.txt` sealed on `heads/main`, then `prikk mv a.txt b.txt`
(`.git-exclude/reports/034-prikk-0-46-rebaseline/measure046-rows.txt`, `…-declarations.txt`):

| State | `resolution` | `content_changed` / `mode_changed` | counts | `prikk commit` |
|---|---|---|---|---|
| nothing else | `rename` | `false` / `false` | — | `rename-path` |
| `b.txt` edited | `rename` | **`true`** / `false` | — | `edit-text`, `rename-path` |
| `b.txt` made executable | `rename` | `false` / **`true`** | — | `change-perm`, `rename-path` |
| `b.txt` deleted | `deletion` | `null` | — | `delete-file`, *"destination is gone"* |
| **`b.txt` is a directory** | **`deletion`** | `null` | — | `delete-file`, ***"destination is a directory"*** |
| `b.txt` in `.prikkignore` | **`deletion-ignored`** | `null` | — | `delete-file`, *"destination is ignored"* |
| an untracked file moved | **`never-tracked`** | `null` | — | `create-file`, *"source was never a tracked node"* |
| `a.txt` recreated (both present) | **`refused`** | `null` | `refused_declaration_count: 1` | refuses with the declaration's `refusal` |
| shell `mv b.txt a.txt` (source back) | **`refused`** | `null` | **`clean: true`**, `refused_declaration_count: 1`, **exit 0** | refuses with the declaration's `refusal` |
| `b.txt` is a symlink | `rename` | **`null`** / **`null`** | **`refused_count: 1`** | refuses the whole commit, over the **path** |

**The two refusal texts, verbatim at 0.43–0.46** (byte-identical to what `commit` prints after
`error: precondition not met: `):
- **both present:** `a.txt -> b.txt: both paths exist in the worktree, so which one is the tracked node is not prikk's to guess. Set one copy aside first -- ``prikk mv`` refuses while both are there: delete a.txt and commit to author the rename, or delete b.txt and run ``prikk mv b.txt a.txt`` to drop the declaration`
- **source back:** `a.txt -> b.txt: the source is present in the worktree again, so the declared move is not what the worktree holds. Run ``prikk mv b.txt a.txt`` to drop the declaration, or ``prikk mv a.txt b.txt`` to make the move again`

## 3. Declarations, from prikk's resolution (≥ 0.44)

`ChangesView`'s declaration analysis stops inferring and reads `resolution`:

| `resolution` | stikk shows |
|---|---|
| `rename` | the pair, as RFC 032 ships it: both rows annotated, counted in `renames N` — **plus §4's content words** |
| `deletion` | *"declared rename {old} → {new}: {new} is not a file in the worktree, so prikk will record a deletion, not a rename"* |
| `deletion-ignored` | *"declared rename {old} → {new}: {new} is ignored, so prikk will record a deletion, not a rename"* |
| `never-tracked` | *"declared rename {old} → {new}: {old} was never tracked, so prikk will create {new} and drop the declaration"* |
| `refused` | **§5's prevention.** No sentence of stikk's own beyond it |

- **`renames N` counts `resolution == "rename"`** at ≥ 0.44, and RFC 032's pairing below it.
- **The pairing marks still need both halves listed** to have two rows to annotate — where prikk resolves `rename` and
  only one row exists, annotate the row that exists and **do not invent the other**.
- **RFC 032 A6's sentence is retired at ≥ 0.44**, replaced by the two rows above that know *why*.

## 4. What a rename does to content and mode (≥ 0.44)

**`RENAME_CONTENT_NOTE` — *"prikk does not report whether its content also changed"* — is said only below 0.44.** It
is false at ≥ 0.44, where prikk reports exactly that.

At ≥ 0.44, per paired rename, from `content_changed`/`mode_changed`:

| `content` / `mode` | the sentence |
|---|---|
| `false` / `false` | **nothing** — silence is the true statement |
| `true` / `false` | *"declared rename {old} → {new}: its content changed too"* |
| `false` / `true` | *"declared rename {old} → {new}: its mode changed too"* |
| `true` / `true` | *"declared rename {old} → {new}: its content and mode changed too"* |
| **`null`** | **nothing about content or mode** — unknown is not "unchanged" (`C-T2c′`) |

**RFC 032 A7 stands unchanged:** while prikk reports **any** refusal (`refused_count ≥ 1`), stikk promises no outcome
at all — the symlink row is exactly that state, and it now also reports `null`.

## 5. A declaration prikk will refuse makes commit unavailable (≥ 0.44)

**This is RFC 027 Q1 (b)'s rule finally applied to declarations** — prevention only on prikk's own verdict, which is
why RFC 032 could only warn.

- **Trigger:** `refused_declaration_count ≥ 1`, **beside** RFC 027's `refused_count ≥ 1`. prikk's own instruction is
  to read a commit's prospects as *both* being zero. **They are different facts:** the symlink row has
  `refused_count: 1, refused_declaration_count: 0`; the source-back row has the reverse.
- **The reason carries each refused declaration's `refusal` verbatim**, attributed to prikk (`ER-02`). It already
  names prikk's measured ways out, so **stikk adds no way out of its own at ≥ 0.44** — RFC 032 A3's suffix is for
  below the band.
- **The source-back row reports `clean: true` and exits 0.** So **the refused-declaration check must run before the
  clean-worktree block**, or stikk hides prikk's verdict behind its own *"nothing to commit"*. RFC 032 A3 put the
  notice in that blocked reason; at ≥ 0.44 it becomes a prevention instead.
- **Where prikk's refusal advises deleting a file, that is prikk's advice**, shown as prikk's words. **stikk never
  phrases it as its own.**

## 6. The sentence that gives a reason stikk cannot know (< 0.44)

RFC 032 A6 ships *"{new} is not a file in the worktree"*. **With the destination ignored, that is false** — the file
is there, prikk simply does not list it, and below 0.44 the report cannot tell the two apart.

**Below 0.44 the sentence states only what the report shows:**
*"declared rename {old} → {new}: prikk's report does not list {new}, so prikk will not author it as a rename"*

**Measure it at 0.42 and 0.43 before using it.**

## 7. A checkout that stopped part-way (≥ 0.43), and the gloss below it

**Measured**, marker set by a refused `checkout --patch-materialize` on prikk **0.42**, repository then read by
**0.46** (`measure046-marker.txt`) — and identically at 0.43 when RFC 033 measured it:

| Surface | What prikk gives |
|---|---|
| `status` prose | `interrupted materialization: a checkout or branch switch stopped part-way; move aside any file it named, then run prikk checkout --patch-materialize --ref heads/main or prikk branch switch heads/main` |
| `status --format json` | `"interrupted_materialization": {"routes": ["prikk checkout --patch-materialize --ref heads/main", "prikk branch switch heads/main"]}`; **`null`** otherwise |
| `doctor` | `warning [PRIKK-DOCTOR-INTERRUPTED-MATERIALIZATION]` … *"and `commit` refuses"* |
| `commit` | **`precondition not met:`** *"worktree materialization was interrupted, so the worktree is not verified against its baseline and nothing can be committed; run …"* |

**What stikk does:**
1. **Orientation says so at ≥ 0.43**, from the JSON field stikk already reads for the queue, **with prikk's routes
   verbatim**. It is a state of the repository, not an error.
2. **Commit's preview is unavailable with that reason** — prikk's own verdict, since prikk's `commit` and `doctor`
   both say commit refuses.
3. **Reads are not blocked, and stikk must not block them.** Measured at 0.46 with the marker set: `log` exits 0, and
   `worktree-status` answers normally (its exit 1 there is the ordinary dirty-tree code). **Changes and History keep
   working.**
4. **At ≤ 0.42 the state is unreportable, and the refusal is glossed.** 0.42's `commit` refuses with
   `integrity error: worktree materialization was interrupted …`, which stikk presents as an integrity finding
   although **nothing is corrupt**. The gloss says what it is and gives the measured ways out (prikk's reply 013).
   **Measure 0.42's exact commit refusal before writing the gloss** — this handoff measured 0.42's *checkout*
   refusal, not its commit one.

## 8. The traps

1. **Two counts, two meanings.** `refused_count` is paths; `refused_declaration_count` is declarations. Neither
   implies the other (§2's last two rows prove it).
2. **`rename` does not mean the commit succeeds** (the symlink row). A7's rule is what keeps stikk honest there.
3. **`null` is not `false`.** A non-regular destination reports `null` for content and mode; stikk says nothing about
   them rather than "unchanged" (`C-T2c′`).
4. **The clean check must not run first** (§5), or the source-back row loses prikk's verdict.
5. **The marker blocks commit, not reading** (§7.3).
6. **Nothing below 0.44 changes** except §6's sentence and §7.4's gloss.
7. **RFC 030's re-read at Enter compares views for equality.** These fields ride inside `ChangesView`'s declarations,
   which that comparison already covers — **check that a commit on a repository with a declared rename is still not
   stale**, at both ends; the suite already has that leg.

## 9. Tests, captures and the bands

- **Suite legs at the ceiling (0.46) and floor (0.28)** for each `resolution` row in §2, asserting stikk's words and
  `prikk commit`'s printed output together, naming the surface (RFC 032 A4).
- **prikk 0.44 is the band boundary and is not in the suite's matrix** — its two ends are 0.28 and the ceiling.
  **Measure 0.44 by hand**, as Handoff A did for 0.45, with the script in the review-request folder.
- **Core tests with a stand-in backend** for each band: a 0.43 report keeps RFC 032's inference; a 0.44 report uses
  `resolution`; `null` content/mode renders nothing; `refused_declaration_count` with `clean: true` prevents.
- **Captures at 80 columns**: the four §3 sentences, §4's content words, §5's prevention reason (prikk's refusal
  wrapped, not clipped), and §7's Orientation line with prikk's routes.
- **`requirements.md`** on delivery: `FR-034` and `FR-050` gain a note that at prikk ≥ 0.44 the declaration analysis
  is prikk's resolution rather than stikk's inference, and that a refused declaration prevents a commit.

## 10. Gates, runs, submit

The eight gates as always — 1–5, 7, 8 on the MSRV; gate 6 on stable with a fresh `CARGO_TARGET_DIR`. Then, from a
`prep/034-b-the-rebaseline` branch: `CI`, the real-binary suite with `full_platform_matrix=true`, and the
supply-chain gate, **all at one SHA**, every job and leg read, the branch deleted before submitting.

Package to `.git-exclude/review-request/034-b-the-rebaseline/review-request-v1.md`.

**Lead with the 0.44 hand measurement and the captures.** Then the suite numbers at both ends, and anything §2's
table got wrong when you re-measured it — **if a row differs from what I measured, stop and report it**; that is the
second time this increment a row like that would have mattered.

**Push your own commits once approved.** After this, Patch detail (`FR-030`) and Compare (`FR-033`) are the next
RFCs, on prikk 0.46's `tree`, `cat` and `diff`.

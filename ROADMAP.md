# Roadmap

How stikk gets from the current foundation to a usable history browser and workbench for prikk. This
is a direction, not a dated schedule: increments ship when they are correct and tested, and the order
can change as prikk evolves. Requirement and design ids (e.g. `FR-050`, `TU-01`, `UD-03`) refer to the
design set in [`docs/src/reference`](docs/src/reference/).

Guiding rules that do not change across the roadmap:

- **stikk owns no repository authority and no secrets** — every increment preserves this.
- **Preview-first, and *where prikk refuses, stikk explains*** — read and explanation surfaces come
  before, and stay ahead of, mutation surfaces.
- **One operation layer, two frontends** — the TUI is built first; the GUI reaches parity through the
  same operations, never a parallel implementation.
- **Design before implementation** — each milestone below is gated on its design items already
  existing in the reference set; new decisions are recorded as RFCs first.

## Shipped (0.1.0 foundation · 0.2.0 read surface, corrected)

The security-critical layers, built and tested under the project gates:

- The workspace and lint discipline; `stikk-model` kernel; the `stikk-prikk` seam (CLI backend with
  the EPIPE guard, version gate, scripted backend, presence-only key reader); `stikk-state`
  (config, discovery, the repository-internal write refusal); the `stikk-core` `orient` operation;
  and the launcher (`--version`, `config check/path`, one-shot orientation).
- The two security invariants are enforced by test. 57 tests pass; clippy is clean under `-D
  warnings`.

## Shipped — the interactive read surface (0.2.0)

_Increments 1–5 shipped in 0.1.0; 5b corrected them in 0.2.0. Kept in sequence because each records why
its scope is what it is._

The goal: a running TUI you can browse a repository with. Nothing here needs a mutation.

1. **Pick the TUI toolkit** — ✅ decided: **`ratatui` + `crossterm`** (RFC 001, accepted 2026-09-01;
   GUI toolkit spun out to a future RFC). It gates everything interactive.
2. **The TUI shell and Orientation view** (`TU-01/02/03`, `FR-002`) — ✅ shipped (`stikk-tui`): the
   header/status-bar/overlay layout, the view stack, global keys, and the live Orientation. `stikk
   <repo>` opens the TUI on a terminal; piped/CI keeps the one-shot print. Built per the
   [handoff](rfcs/handoffs/001-frontend-toolkit-selection/tui-shell-and-orientation-handoff-v1.md).
3. **History** (`FR-010…017`) with the unsealed queue tier + **Block detail** (`FR-031/032` at block
   granularity) — ✅ **3a shipped** (RFC 006): the seam grew `history`/`block_state`/`refs`, the
   operation layer gained `history_view`/`block_detail`/`list_refs`, and the TUI gained the view
   stack, the History and Block-detail views, and a ref picker — the first heavy use of the
   inert-text primitive and the overlay layer. `cargo run -p stikk-tui --example history_demo` drives
   all of it against a scripted backend. Built per the
   [handoff](rfcs/handoffs/006-history-and-inspection-seam/history-view-handoff-v1.md).
   - **3b — Patch detail** (`FR-030`), patch-id enumeration, and diff-aware search (`FR-013`) are
     **split out, blocked on UD-09**: prikk exposes no per-patch content and no `show`/`diff`, only
     block-level `log`. stikk shows block lineage + a block's state file list now, and names the gap
     where a user would open a patch — never a faked diff. UD-09 (a `log --format json` + a
     patch-content surface) is filed upstream, mirroring UD-01…08.
4. **Refusal explanation overlay + the witness/finding glossary** (`FR-110/111/112`) and the **command
   palette** (`FR-125`) — ✅ **shipped** (RFC 007): one class→presentation mapping in `stikk-core`
   (ER-03, `present()`), a version-gated seam classifier for prikk's 0/1 exit (UD-05, degrading an
   unknown message to a verbatim refusal), the TU-08 refusal overlay (verbatim + gloss + stikk-authored
   next-steps + glossary links), the glossary asset (DM-09, terminology seeded in full / witness+finding
   codes as their sources land), the in-memory session refusal history (DM-06), and the palette's
   operation registry (TU-07, capability-gated with visible-but-disabled reasons). `cargo run -p
   stikk-tui --example explanation_demo` drives all of it. Built per the
   [handoff](rfcs/handoffs/007-explanation-and-discovery-surface/explanation-surface-handoff-v1.md).
5. **Changes** — worktree-vs-baseline (`FR-034`) — ✅ **shipped** (RFC 008); **Compare (`FR-033`)
   deferred**. A check of the live binary found `worktree-status` **fixed as of prikk 0.28** (UD-03 was
   a 0.27.x defect), so Changes uses it directly (version-gated; below 0.28 it explains rather than runs
   the broken command). It is path-level (modified/missing/untracked/unsupported) with the UD-08
   untracked filter and the UD-06/UD-09 honesty notes; a dirty tree's non-zero exit is treated as a
   normal status, not a refusal (UD-05). `cargo run -p stikk-tui --example changes_demo` drives it.
   **Compare has no honest command** (no `diff`/`compare`/`show`; `checkout` is ref-tip-only; plan
   output carries no per-file content), so a partial Compare would mislabel differing files as identical
   (T-T4) — split out with a concrete future route (materialize two ref tips to temp dirs). Built per
   the [handoff](rfcs/handoffs/008-worktree-changes-and-the-compare-ceiling/changes-view-handoff-v1.md).
5b. **Correction — prikk 0.30 re-baseline and parser fidelity** (RFC 009) — ✅ **shipped in 0.2.0.**
   Running shipped stikk against the real binary found that **Orientation fails on any repository with
   queued patches** (prikk reports `queued patches: N targeting <ref>`; stikk's parser refused it), that
   two other parsers accept shapes prikk never emits, and that stikk **drops prikk's own warning** that
   paths shown "untracked" may be committed-but-unsealed work on another ref — the `T-T4` picture the
   project exists to prevent. Root cause in each case: a golden fixture that was *written* rather than
   *captured*. Targeted at a **0.1.1** patch release. Built per the
   [handoff](rfcs/handoffs/009-prikk-0-30-rebaseline-and-parser-fidelity/parser-fidelity-handoff-v1.md).

---

## Shipped — responsive & correct (0.3.0, breaking)

Two increments, in a load-bearing order: RFC 010 reshapes the seam trait, so anything landing after it
would otherwise be re-touched. Recorded in
[RFC 012](rfcs/done/012-post-0-2-0-correctness-sweep.md).

1. ✅ **[RFC 010](rfcs/done/010-off-thread-seam-and-ui-responsiveness.md) — the off-thread seam.**
   Shipped to `main` 2026-09-04. `NFR-P01` was a **Must** and was **unmet**: every seam call blocked
   the render loop, and `OrientationState::Loading` was a state no user could observe. Gives the trait
   its `Send + Sync` bound, a cached handshake, per-view load states with stale-response discarding,
   and the background-operation surfaces. `NFR-P02` (true cancellation) is deliberately deferred to
   `FR-100`, where it can be measured.
2. ✅ **[RFC 012](rfcs/done/012-post-0-2-0-correctness-sweep.md) — the correctness sweep.** Shipped to
   `main` 2026-09-05. Read-only
   must lock out recovery (`FR-121` over `AC-04`); version skew must stop pointing users at their
   signing keys; per-platform config/state paths on the platforms we already ship binaries for;
   `RefName` adopted so ref names are validated and not merely rendered inert; a gloss for prikk
   0.31's forward-incompatible schema error; and the `tag list` read that completes `FR-014`.

**Then 0.3.0 is cut.** RFC 003 moves to 0.4.0, where its consumers actually live — see the
release-boundary note in RFC 012.

## Shipped — the working cycle (0.4.0)

_Re-sequenced 2026-09-04: session persistence no longer precedes this. It was placed first when stikk
could not open a real repository; now that it can, being able to commit matters more than resuming a
view — and after RFC 003 it is also cheaper to build._

**[RFC 003](rfcs/done/003-repository-change-token.md) — the change token — opens this release**,
because `OPL-02`'s preview↔execute binding is built on it: a preview computed under one change token
must refuse to execute if the repository moved underneath it. *(Moved here from 0.3.0 on 2026-09-05: it
delivers nothing user-visible alone, and 0.4.0 breaks the seam trait anyway for the mutating methods.
**Its fingerprint half was split off and deferred at acceptance** — prikk deliberately has no
repository identity and states that as a security property, deriving one would mean walking the entire
log, and `INV-5`'s re-resolution already carries the protection it was meant to add.)*

✅ Shipped to `main` 2026-09-05.

**Then the preview + tiered-confirmation machinery**
([RFC 013](rfcs/done/013-preview-and-confirmation-machinery.md), `FR-120`/`FR-121`, `OPL-01…05`) —
not an afterthought inside the commit flow: it is what every mutation below is gated on, and the first
thing to consume the change token. Preview-first becomes **structural** — `execute` takes a token only
a preview can produce — rather than a rule reviewers must remember.

> Drafting it already found that **`FR-052` is not satisfiable as written**: the seal ceremony is
> required to show "exactly which patches will seal", and prikk exposes the queued *count* and target
> ref, never the patch ids. The requirement will be amended to what is honest, and the enumeration
> surface is filed as an upstream ask — found before the seal ceremony was built against it, which is
> why the machinery precedes the mutations it gates. **`FR-052` and `FL-06` were amended 2026-09-05** to
> require what is knowable: how many patches, which ref, which resulting block, and that the ceremony
> says it cannot enumerate.

✅ Shipped to `main` 2026-09-05.

**Then commit** ([RFC 014](rfcs/done/014-commit-the-first-mutation.md), `FR-050`/`FL-05`) — ✅ shipped
to `main` 2026-09-06. **The first operation stikk performs that writes.** Two prikk refusals it
*prevents* rather than classifies (a cross-ref commit, and a clean worktree — both verified, both
failing closed upstream), the message step before the confirmation per `FL-05`'s own order, prikk's
result and both `note:` lines transported verbatim, and the `capability_gate`/palette unification
RFC 013 deferred with a deadline of exactly this increment.

**The prikk 0.32 re-baseline** ([RFC 015](rfcs/done/015-prikk-0-32-rebaseline.md)) — ✅ shipped to
`main` 2026-09-06. Not a chore; `UD-01` is retiring, and drafting it found more: **`prikk log` now prints a line per patch
carrying its id and message**, so RFC 006's founding finding — that `log` has no per-patch detail and
no patch ids — has stopped being true, and stikk is silently discarding both. `UD-09` therefore
**narrows** rather than retires: patch *ids* are now enumerable, patch *content* still is not, so
Patch detail stays deferred. Verifying it also turned up a version-honesty defect that had
survived a release *inside the feature built for version honesty*: both renderings of the
"validated through N" notice carried a literal that RFC 012 left behind when it raised the ceiling, and
the tests asserted the same stale number. Both now read one shared source, guarded by tests that assert
against a sentinel rather than the answer.

**Then the seal ceremony** ([RFC 016](rfcs/done/016-the-seal-ceremony.md), `FR-052` as amended) — ✅
shipped to `main` 2026-09-06. **0.4.0 is content-complete on its landing.**

> **Queue review (`FR-051`) is not a separate increment**, and checking rather than assuming is why.
> Its core clause — *"list queued patches with full patch detail"* — is **not satisfiable**: prikk
> reports the active WAL only as `queued patches: N targeting <ref>` and exposes no queued-patch ids
> (verified at 0.32.0). RFC 015's `log` enumeration covers **sealed** patches only, never the WAL. Its
> knowable half is already delivered, distributed rather than in a dedicated view — depth and target
> ref in Orientation, the "not yet history" tier in History, prikk's threshold warning in the commit
> preview — so `FR-051` was **amended 2026-09-06** and the Queue view waits for an upstream
> enumeration surface rather than shipping a view that lists nothing.

Drafting the seal RFC then found a **shipped defect unrelated to seal**: stikk's `[MNT ✓]` badge means
"key material is present", but sealing also requires the maintainer key to be **adopted in the
repository's trust policy** — and prikk offers `trust maintainer add`/`remove` with **no `list`**, so
stikk cannot check it. The badge has been claiming a capability stikk cannot verify. RFC 016 stops the
over-claim; `FR-103`'s "adopted maintainer keys list" is the third requirement now amended against
prikk's real surface. Upstream RFC 123 landed
**commit-message storage** in prikk 0.32, which falsifies the dependency stikk has carried since
0.1.0 and makes one line of stikk's pre-commit copy false for anyone on that release. Separately,
upstream **RFC 132 — prompted by a report we sent them, and landed within a day** — reclassifies both of RFC 014's refusal messages to `precondition not met:`; that is unreleased
as of the `0.32.0` tag, so the two changes reach users at different times. Neither breaks stikk today,
because the classifier matches what the messages *mean* rather than what they *say*, and commit
results transport whatever notes prikk actually printed. The re-baseline validates 0.32, retires
`UD-01`, and corrects the copy.

`FR-051` (queue review) did **not** become a fourth mutation: its core clause was amended (above) to
what prikk can actually report, and that knowable half is already delivered, distributed rather than in
a dedicated view — a Queue view waits on upstream queued-patch enumeration.

**The prikk 0.33 re-baseline & classifier provenance** ([RFC 017](rfcs/done/017-prikk-0-33-rebaseline-and-classifier-provenance.md))
— ✅ shipped to `main` 2026-09-06. Re-grounds five classifier arms that matched text prikk had never
actually emitted, at any version, on live-captured output instead; corrects the commit path's
full-queue precondition, previously misclassified as a lock conflict when the queue was simply full;
and raises the validated ceiling to **0.33.0**.

Also carried into 0.4.0's release prep, both found during review rather than planned: **23 rustdoc
warnings** — several were public docs linking to private items, which render as dead references in the
published API docs, grown from 19 while nothing gated them — fixed, and a **rustdoc lint gate added to
CI**, which prikk's own CI has and stikk's previously did not.

**0.4.0 shipped.** What it does not have, named rather than left implicit: a Trust & Keys view, the
AUTHOR/MAINTAINER key-id display module (deferred out of RFC 014 into RFC 016 and carried past it;
built in 0.6.0), a Queue view, Compare, Patch detail, and verify/branches/merge — see "Later".

## Shipped — stikk is checked (0.5.0)

A **real-binary integration suite** drives stikk against actual prikk binaries at both ends of the
supported range, asserting what the repository became rather than what stikk parsed
([RFC 019](rfcs/done/019-the-real-binary-integration-suite.md)); a **supply-chain gate** runs advisories
and licences over the dependency tree ([RFC 020](rfcs/done/020-msrv-1-88-consistency-and-a-supply-chain-gate.md));
the validated prikk range moved **0.33 → 0.38** ([RFC 021](rfcs/done/021-prikk-0-38-rebaseline.md)); and
a fabricated worktree entry was fixed.

## Shipped — stikk says what it knows (0.6.0, breaking)

- **Signing readiness is read from prikk**, not inferred from the environment: `prikk key status` at
  prikk ≥ 0.41, *unknown and said to be* at 0.40, presence at ≤ 0.39
  ([RFC 026](rfcs/done/026-readiness-after-the-key-directory.md)). The validated range was then **0.28
  through 0.41**, and `log`/`branch`/`tag` are read as JSON at ≥ 0.39.
- **Confirmations name the key that will sign**, and say whether it is bound, unbound, or unchecked;
  prikk's published example keys are flagged (`C-S2`, implemented)
  ([RFC 023](rfcs/done/023-what-stikk-has-and-does-not-show.md), RFC 026).
- **Overlays size themselves by measurement** and say when they are clipped; seal's confirm affordance
  is on screen for the first time ([RFC 024](rfcs/done/024-overlay-sizing-and-the-fix-that-did-not-travel.md)).
- **The real-binary suite drives all eleven `Prikk` seam methods**
  ([RFC 022](rfcs/done/022-widening-the-real-binary-suite.md), and `readiness` in 0.6.0's prep).

## Shipped — stikk shows what a change will do (0.7.0, breaking)

- **prikk's own commit verdict, shown and enforced**: commit is unavailable, with prikk's reasons, when prikk
  says it would refuse, and the unsupported paths stikk had counted and never listed since 0.1.0 are listed
  ([RFC 027](rfcs/done/027-what-commit-would-refuse.md)).
- **The Queue view** (`Q`) lists each queued patch, with its message on prikk ≥ 0.42; History's tier says whose
  queue it is; and the seal confirmation names the patches it freezes
  ([RFC 028](rfcs/done/028-the-queue-view.md)).
- **The prikk 0.42 re-baseline**: the validated range is **0.28 through 0.42**, and stikk opens on prikk's
  current branch, names prikk's default beside its focus when they differ, names both refs on a confirmation,
  and falls back to the ref picker when `heads/main` is not published
  ([RFC 029](rfcs/done/029-prikk-0-42-rebaseline-and-the-current-branch.md)).
- **A confirmed commit authors the worktree its preview showed**: the worktree is re-read immediately before
  `prikk commit`, prikk's rename declarations included; prikk's current branch joins the change token; and a
  stale confirmation says whether the repository or the worktree changed
  ([RFC 030](rfcs/done/030-a-confirmed-commit-authors-the-worktree-it-previewed.md)).

## Shipped — stikk stops describing a repository it has stopped watching (0.8.0, breaking)

- **stikk notices a change made outside it**: a silent change-token check when the terminal regains focus and every
  five seconds while idle — never in `⟳ n`, never in the Operations list, and never sent while stikk's own work is
  running, so its own commit is never mistaken for one. A detected change refreshes what is on screen, says
  *"repository changed outside stikk — refreshed"*, and makes an open confirmation stale at once; `r` now refreshes
  Changes and the tip's Block detail too, and every refresh re-reads the ref its own screen shows
  ([RFC 031](rfcs/done/031-seeing-a-change-made-outside-stikk.md)).
- **What a commit will author**: a `prikk mv` whose two halves prikk lists is annotated on both rows as one declared
  rename and counted `renames N` at prikk ≥ 0.38, on the Changes view and commit's confirmation; a declaration prikk
  will not author as a rename is named and never counted, with the measured way out where one exists; **a ref with no
  published history is named as one**, with words chosen from prikk's queue for that ref; and **stikk promises no
  outcome while prikk reports the commit would be refused**
  ([RFC 032](rfcs/done/032-what-a-commit-will-author.md)).

## Shipped — stikk shows what each patch changed (0.9.0, breaking)

- **Patch detail** (`FR-030`), unbuilt since 0.1.0: one patch's operations, each under prikk's own kind word with
  prikk's own fields — a text edit's spans **as spans**, a create's content and mode, a delete's preimage, a binary
  replacement's two blobs and sizes, a permission change's two modes, and a rename's two paths with the key that
  asserted it — opened from Block detail, which now lists every block's patches, or from the Queue, which gained a
  selection; `v` shows prikk's own fields. **Content is byte-faithful**: quoted, one row per source line, with
  everything that does not show as itself escaped from one table, so a whitespace-only edit is visible and a
  bidirectional override cannot reorder a row. In a block of several patches a node-addressed path is labelled *as
  at the end of this block*, because prikk resolves it there
  ([RFC 035](rfcs/done/035-patch-detail.md)).
- **The prikk 0.46 re-baseline, and the repair the published line needed**: a ref with no published history reads
  again on prikk 0.45 and 0.46 — stikk re-runs without `--ref` and accepts the answer only when it names the ref
  asked for; a declaration's fate is **prikk's own `resolution`** at prikk ≥ 0.44, one prikk would refuse makes
  commit unavailable, a checkout that stopped part-way is shown, and prikk's no-repository and too-old-binary
  answers are environment errors
  ([RFC 034](rfcs/done/034-prikk-0-46-rebaseline.md)).
- **The validated prikk range is `>= 0.28`, through `0.51.0`** (was `0.46.0` on `main`, `0.42.0` published), raised
  first and alone each time; the real-binary suite drives 42 cases at 0.28.0 and 0.51.0 on Linux, macOS and Windows.

## Shipped — stikk shares work as a bundle file, and installs from the AUR (0.16.0, breaking)

- **The panels' prose gets a right gutter** ([RFC 013 Handoff B](rfcs/handoffs/013-preview-and-confirmation-machinery/b-panel-prose-gutter-handoff-v1.md)):
  no word stikk wrote sits against a card's border and reads as cut off, on the confirmation, seal and refusal
  cards alike. Identifiers and prikk's own lines keep the full inner width, so a 64-hex block id stays on one row.
- **Exchange, read-only** (`FR-090/091`, [RFC 050](rfcs/done/050-exchange-bundles.md)): `i` opens a bundle file and
  shows **prikk's own verification** of it, word for word, beside what prikk says importing it would change against
  your branch of the bundle's own name — the files and their sizes, who sealed it with prikk's continuity note, and
  prikk's own reason where it conflicts. **Nothing is written** by opening one.
- **Exchange, the writes** (RFC 050): `I` imports a bundle as `remotes/<name>`, read-only history beside your own,
  and `E` writes a branch out to a bundle file. **stikk declines two imports prikk allows** — one that changes
  nothing, and a stale file that would move the received branch *back* and hide newer work — each without asking
  prikk at all. An export never writes inside the repository and never over a file that is there. Adopting received
  history into a branch of your own is a merge and stays out.
  *(Rollback was scoped here first and withdrawn the same day, re-authorized as exchange: measured, prikk 0.53's
  rollback inverts the whole chain to the empty state, not one change. It waits on prikk.)*
- **The prikk 0.54 re-baseline** ([RFC 052](rfcs/done/052-prikk-0-54-rebaseline.md)): **on Windows the installer
  prints prikk's own install command**, so both programs install in two plain steps on every platform; the version
  report names the minor it validated. prikk 0.54 also fixes a read stikk shows — Block and Patch detail open where
  0.53 refused after consecutive edits of one file.
- **Arch Linux packages on the AUR** ([RFC 051](rfcs/done/051-arch-linux-packages.md)): `stikk`, built from the
  published crate, and `stikk-bin`, the prebuilt static build left unstripped so it stays byte-identical to the
  attested archive. Both are built and linted in Arch's own container on every change, and published by this
  release's run.
- **The validated prikk range is `>= 0.28`, through `0.54.1`** (was `0.53.0`); the real-binary suite drives 69 cases
  at 0.28.0 and 0.54.1 on Linux, macOS and Windows.

## Shipped — stikk merges one branch into another, and installs in two plain steps (0.15.0, breaking)

- **The declarations card fits** ([RFC 034 Handoff C](rfcs/handoffs/034-prikk-0-46-rebaseline/c-declarations-card-fits-handoff-v1.md)):
  *"what commit would refuse"* shows whole declarations and says how many more there are, never a heading over an
  empty quote and never a row the reader cannot reach on a card that does not scroll.
- **Merge, read-only** (`FR-080/081`, [RFC 047](rfcs/done/047-merge.md)): mark a branch, select another, `M` — and
  prikk's own plan for the pair, word for word, against the baseline block stikk found. **stikk names a baseline
  only where it can name the true one**: prikk reports no parent ids, so past a merge there is nothing to follow,
  and where stikk cannot it says so and names the prikk command that takes a baseline you choose. Four merges are
  declined before prikk is asked, each in stikk's own words.
- **Merge, execution** (`FR-082`, RFC 047): `Enter` merges, at seal's tier, behind a card that names the branch
  merged from, the branch merged into, the baseline and the tip being adopted, with prikk's plan beneath it.
  **Every guard is re-asked at the moment you confirm**, not reused from when the view opened, and nothing is
  written to your worktree.
- **Delivery for users new to all of this** ([RFC 048](rfcs/done/048-delivery-for-new-users.md)): **each program
  installs itself** — stikk's installers install stikk, check prikk and print **prikk's own install command**, so
  where prikk lives and how it is verified stay prikk's decisions; an install page that starts from opening a
  terminal; and a **static build for 64-bit ARM Linux**, because the glibc one did not start below glibc 2.39 —
  found by the new ARM CI job, fixed, and proven on real hardware at glibc 2.35. The Homebrew tap (Handoff C) and
  SignPath signing wait on the owner and are not in 0.15.0.
- **The prikk 0.53.0 re-baseline** ([RFC 049](rfcs/done/049-prikk-0-53-rebaseline.md)): nothing stikk reads changed
  — 0.53's one output change is to `checkout`'s refusals, which stikk calls only below prikk 0.46.
- **The validated prikk range is `>= 0.28`, through `0.53.0`** (was `0.52.0`); the real-binary suite drives 63 cases
  at 0.28.0 and 0.53.0 on Linux, macOS and Windows.

## Shipped — stikk reopens each repository where you left it, and never stores anything inside one (0.14.0, breaking)

- **Session persistence** (`FR-122`, [RFC 046](rfcs/done/046-session-persistence.md)): each repository reopens on
  **the ref you had focused, which of Orientation, History, Changes and the Queue was open, and the untracked
  filter** — names and flags, no content and no ids — **re-checked against prikk before any of it is shown**. A
  branch that has been closed or is no longer listed is not restored, and a one-line note says so. Verify never
  reopens: it scans the whole repository, and a launch does not start that on anyone's behalf. The records live
  outside every repository, one small file each, and deleting them is always safe. **`STIKK_PRIVATE=1`** remembers
  nothing.
- **`C-E2` resolves a path before it judges it.** stikk's first file of its own is the first write that refusal ever
  guarded, and it had always compared the text it was handed: a relative path, a `..` leading back in, or a symlink
  into a repository reached past it. Both sides are resolved now. **No released stikk ever wrote through it**, so
  there was nothing to disclose — the check simply became the control it was documented as.
- **A workspace-wide seed-read guard** (RFC 023 Handoff C): *"No key material ever enters stikk … enforced by
  test"* holds over all six shipped crates rather than two files, with a three-file allowlist that each carries its
  reason, and `SECURITY.md` states what is now enforced.
- **Two carried items** (RFC 040 Handoff C): the Glossary lists `V`, which had shipped unlisted since 0.11.0; and the
  overlay clipping gate builds every `Overlay` variant, with the count read from the enum rather than written down —
  the literal was how the gap lasted an increment.
- **The validated prikk range is unchanged at `>= 0.28`, through `0.52.0`**; the real-binary suite drives 58 cases
  at 0.28.0 and 0.52.0 on Linux, macOS and Windows.

## Shipped — stikk manages branches and tags, not only browses them (0.13.0, breaking)

- **Block detail's `said` onto `view::wrap_hanging`** (RFC 040's carry,
  [Handoff B](rfcs/handoffs/040-verify/b-wrap-hanging-tidy-handoff-v1.md)): a pure refactor, folding the third
  hand-written copy of the opening-row/hanging-row wrap into the shared helper. No row of any capture moved.
- **Branches and tags** (`FR-070`/`FR-071`, [RFC 044](rfcs/done/044-branches-and-tags.md)): **create a branch**
  (`n`), **create a tag** (`t`, from the ref picker or at any block in History) and **close a branch** (`x`), each
  preview-first at seal's tier and capability, running exactly the one prikk command the preview restated and
  showing prikk's refusals word for word. **stikk adds no naming rules of its own** — an empty name is not sent,
  and that is the only one — and a refusal hands the typed name back. **The branch you are standing on is never
  offered for closing**: prikk allows it with an empty WAL and the repository is then on a closed branch where even
  a default `branch create` refuses, measured at 0.52.0. prikk ≥ 0.42. A long banner now wraps instead of losing
  its tail.
- **Switch branch** (`FR-053`, [RFC 045](rfcs/done/045-switch-branch.md)): **`s` in the ref picker**, stikk's first
  operation that changes files on disk, **previewed as the exact files it will write and delete** — stikk's own
  comparison of the two branches, measured as an exact prediction of what the switch then does. Tier Three and
  **no signing key**, because prikk needs none: the confirmation gate is keyed on the request category now rather
  than on its tier. A plan that is not whole blocks rather than shortening itself, and a branch the queue holds
  unsealed work for is not offered — the one case prikk allows and gets wrong. prikk ≥ 0.46.
- **The validated prikk range is unchanged at `>= 0.28`, through `0.52.0`**; the real-binary suite drives 55 cases
  at 0.28.0 and 0.52.0 on Linux, macOS and Windows.

## Shipped — stikk works with prikk 0.52.0, and the README shows stikk at work (0.12.0, breaking)

- **The prikk 0.52 re-baseline** ([RFC 043](rfcs/done/043-prikk-0-52-rebaseline.md)): **validated through 0.52.0**,
  raised the day prikk published it and measured the same day against real 0.28.0 and 0.52.0 binaries. **prikk 0.52
  fixed the defect stikk reported in letter 020**: in a sealed block of several patches, `show <block-id>` resolves
  each patch's path **at that patch** rather than at the block's end, so a patch whose file a later patch renamed is
  reported under the name that patch saw, and one whose file a later patch deleted has a path at all. **Patch
  detail's *as at the end of this block* label is therefore banded below 0.52**, where it stays true, rather than
  deleted. The raise came first and alone, red on exactly the one leg that pinned 0.51's answer. The installers pin
  prikk 0.52.0 by themselves, deriving it from the same constant.
- **README key visuals** ([RFC 042](rfcs/done/042-readme-key-visuals.md)): three images — History, Compare and
  Verify — under the README's opening paragraph, **rendered by stikk's own views from invented data rather than
  photographed**, so nothing of a maintainer's machine can be in them: there is nothing real in a scene to leak. A
  test regenerates them and fails if they have drifted from the views; a second fails if an image carries a
  home-directory path, a machine's user or host name, an email address, an undeclared object id or the words `seed`
  or `secret`; a third plants a fault per rule and fails if the rule stays quiet. **Making them found five view
  titles that ratatui was cutting at 80 columns** — Verify in all three states, and Patch detail's and Compare's
  raw views — which lost the keys at the end of each title; those are fixed and tested.

## Shipped — stikk installs in one command, prikk included, and tells you whether your repository is sound (0.11.0, breaking)

- **One-command installation, on Linux, macOS and Windows** ([RFC 039](rfcs/done/039-installation-for-everyone.md)).
  `curl … install.sh | sh`, or `irm … install.ps1 | iex`: each script downloads the release it was published
  with, **checks it against the checksum published beside it and installs nothing if that check fails**, installs
  into a directory you own, adds it to your own `PATH` once, and says what to type next. **When prikk is missing it
  offers to install it — through prikk's own installer**, whose download prikk verifies itself, pinned to the prikk
  version this stikk was tested against; where prikk has no installer, stikk says so and prints the alternative
  rather than improvising one. Matching uninstallers remove the binary and the `PATH` entry and nothing else, and
  never touch prikk. `cargo binstall stikk` fetches the same prebuilt binary. **The README and the guide lead with
  installing, not `cargo build`** — every release since 0.1.0 had shipped binaries and nothing said so — and all
  six archive checksum files now share one format.
- **Verify (`V`): prikk's verdict on whether the repository is sound** (`FR-100`, in part;
  [RFC 040](rfcs/done/040-verify.md)), at prikk ≥ 0.39, on demand. The verdict, every condition prikk currently
  fails in prikk's own words, and every stage it ran — **no stage count or name is written down anywhere in
  stikk**, because prikk 0.39 reports 14 and 0.51 reports 17. **A damaged repository reads as a verdict, never as a
  refusal**: prikk exits non-zero *with* a complete report when it finds damage, and this is the one command whose
  seam reads that exit as the report it carries. What prikk's JSON leaves out — which objects or refs failed, and
  the unverifiable-signature count — is named on every report, sound or not, with where to find it; **stikk parses
  no prose**. Verify repairs nothing and never runs on its own.
- **Compare reaches any two refs** (`FR-033`, now delivered in full;
  [RFC 041](rfcs/done/041-compare-two-refs.md)): `m` marks a ref in the picker and `d` compares it with the
  selected one — branches, tags and received refs alike. **Two refs have no lineage order**, so stikk states the
  one you chose rather than inventing one, and names the **block** each ref resolved to beside it, because a ref
  can move and a block cannot. The branch you are on before its first seal cannot be a side, and says so in stikk's
  own words: prikk answers its name with *"does not exist in this repository"*, the same sentence a never-created
  ref gets. **No seam change** — prikk could always compare two refs.
- **Long rows wrap under their own value** (RFC 037's carried polish, with RFC 036's). Block detail continued a
  wrapped row at column 0, under the labels; **History wrapped nothing and cut instead** — a row carrying every
  marker is 85 columns, so at 80 its `← tip` was off screen entirely and the tip of a lineage read as an ordinary
  block — and its update-seq and kind columns ran together when either value filled its width.
  `queued_elsewhere_clauses` returns `Piece`s, the last place core took text a frontend had already rendered.

## Shipped — stikk compares any two points in history, and shows every repository string as it is (0.10.0, breaking)

- **Compare** (`FR-033`), the last named view unbuilt: what changed between two points, in **prikk's own `diff`
  hunks** — two marked blocks (`m`, then `d`) or a block and the previous block in its lineage, from History and
  Block detail, and the worktree against **what `commit` would author** from Changes, at prikk ≥ 0.46. Every body
  line is quoted with its own terminator inside the quotes, so a trailing space, a CRLF and a missing final newline
  are each visible and two lines differing in any byte never render alike; prikk's `@@` header and its no-newline
  note are prikk's words and are not quoted. **stikk computes no part of a comparison.** `UD-10` is retired, and
  the Changes foot no longer says a per-file content diff *awaits prikk support*
  ([RFC 038](rfcs/done/038-compare.md)).
- **Repository text renders as it is, everywhere — a security fix.** Bidirectional overrides, zero-width
  characters and look-alike spaces in paths, branch names, key ids and messages no longer reach the terminal in
  any view, and **two different names never look alike**. Prose is escaped in place; an identifier is plain, or
  quoted with every space as ` `. The one-shot printout wrote the repository root and prikk's version line
  with **no filtering at all**; all four of its values now take the same two rules. 0.9.0's own notes named this
  gap ([RFC 036](rfcs/done/036-repository-text-renders-as-it-is.md)).
- **prikk's JSON, read where it now answers**: Block detail lists **any** block's files, with their modes, from
  `tree` (prikk ≥ 0.46), where it listed only the ref tip's; and the ref picker offers **the branch you are on
  before its first seal** (prikk ≥ 0.47), beside the refs and never among them, so the *published* test keeps its
  meaning. History's queued tier wraps instead of clipping
  ([RFC 037](rfcs/done/037-read-prikk-json-where-it-now-answers.md)).
- **The workflows run on node24**, including the actions inside composites; mdBook is pinned by version and
  SHA-256 where it was `latest`, and the Pages deploy runs from `main` only (under
  [RFC 020](rfcs/done/020-msrv-1-88-consistency-and-a-supply-chain-gate.md)).
- **The validated prikk range is unchanged — `>= 0.28`, through `0.51.0`** — and the real-binary suite drives 47
  cases at 0.28.0 and 0.51.0 on Linux, macOS and Windows.

## Next — 0.17.0: stikk gets an earlier version of your files back, and tells you whose work you trust

Scoped and authorized 2026-10-29, in delivery order:

1. **Getting files back** ([RFC 053](rfcs/proposed/053-get-files-back.md), `FR-053`'s non-destructive half): from
   Block detail, save one file as it was at that block, or all of its files into a new folder — outside the
   repository, never over an existing path, previewed by prikk's own plan.
2. **Trust, read-only** ([RFC 054](rfcs/proposed/054-trust-read-only.md), `FR-103`'s read half): a Trust view of the
   keys this repository trusts, yours marked, and on the bundle cards whether each sealer is trusted.
3. **A prikk re-baseline**, only if prikk publishes a new minor before the cut; the cut does not wait for it.

**Decision points, carried:** **SignPath signing** — re-entered only together with prikk's own signing decision,
since signing stikk alone leaves `prikk.exe` blocked; **the Homebrew tap keeps waiting** on the owner's repository and
token (RFC 048 Handoff C), and can join any release when they exist. **The AUR packages** publish by a dispatch of
`aur.yml` the day the AUR grants the adoption of the restored `stikk` and `stikk-bin`; no release waits for it.

## Later — verification, merge, session, exchange, trust, and the GUI

- **Orientation on `status --format json`** (prikk ≥ 0.35). A refactor with no visible change: it would unify
  Orientation's read with the queue's and retire a prose parser. RFC 034's interrupted-materialization marker is read
  from prikk's prose line, by ruling (2026-09-22: prose carries prikk's whole sentence, `ER-02`). Nothing waits on it.
- *Dropped 2026-10-14 (scope 0.10.0; no user is on those bands):* the prose `worktree-status` parse-failure
  attribution (prikk < 0.39), and the would-refuse overlay's absolute-path case (prikk 0.39–0.41).
- **Verify report browser and doctor/recovery** (`FR-100/101/102`), with the three-valued
  author-signature outcome rendered precisely (Sound / Unverifiable / a blocking failure) and locks
  never auto-cleared.
- **Checkout / deletion planning and materialization** (`FR-053`'s remainder, `FR-054`): a block or
  a path, not only a branch. *(Branches and tags — `FR-070`/`FR-071` — and branch switch shipped in
  0.13.0; the focused-ref model shipped in 0.4.0 and 0.11.0.)*
- **The rollback flow** (`FR-083`). *(Merge evidence → plan → execution — `FR-080…082` — shipped
  in 0.15.0, RFC 047: the merge refusal path is where the explanation surface earned its place,
  with four merges declined in stikk's own words before prikk is asked and prikk's own refusal
  shown verbatim where it is prikk's.)*
- **Progressive disclosure** (`TU-12`): default vs. advanced depth. *(Session persistence —
  `FR-122`, the focused ref, view and filter — shipped in 0.14.0, and with it `C-E2`'s first
  production caller. What is left of `DM-04` is **deep restore**: Block detail, Patch detail and
  Compare, and the scroll and selection anchors they need.)*
- **Exchange**: bundle export/verify/import and the **sync assistant** (`FR-090…094`), with the input
  ceilings surfaced before an operation runs.
- **Trust & keys** (`FR-103/104`): the Trust & Keys view; adopted maintainer keys;
  TOFU-conflict-as-security-event. *(The key-id display shipped in 0.6.0 — RFC 023 B — and signing
  readiness asks prikk's `key status` at ≥ 0.41 since RFC 026 B; still no seed is ever stored.)*
- **The GUI** (`GU-01…09`): the same operations rendered natively, reaching TUI parity through the
  shared operation layer, with drag-and-drop constrained to prikk-legal targets.
- **Internationalization** (en / ja / nb) and accessibility hardening across both frontends
  (`NFR-I01`, `NFR-A01…04`).
- **Report export** (`CT-02`) and the versioned `stikk-export` schema.

## Deferred design decisions (the near-term RFC queue)

From the internal design, recorded in [`rfcs/README.md`](rfcs/README.md):

| Decision | Gates |
|---|---|
| TUI/GUI toolkit | everything interactive |
| `stikk-export` schema | report export |
| action-id catalog | the keybinding config |
| change-token signal set | cache validity and external-change refresh |
| linked-library prikk backend | performance, once prikk's crates stabilize (behind the existing seam trait) |

## prikk-side dependencies stikk is waiting on

Several stikk features are gated on prikk-core changes, not on stikk effort. stikk ships degraded,
honest behavior until they land, and adopts the clean path when they do. These double as upstream
issues for the prikk project (requirement `UD-01…UD-05`):

| Dependency | prikk gap | stikk behavior meanwhile |
|---|---|---|
| `UD-01` | **retired at prikk 0.32** — messages are stored (schema 4, tag 6) and `log` enumerates a patch id and message for every patch that carries one; no author display name either way (permanent, no-clock design) | stikk supports prikk on both sides of the boundary: the commit-message prompt's copy is version-conditional, and Block detail shows the id/message list a messaged patch carries, beside the block's own patch count |
| `UD-02` | **narrowing release by release** — `verify` first; `status --format json` at prikk 0.35 and `show --format json` at 0.36 (`ASM-2`); `log`/`branch`/`tag --format json` at 0.39 and `key status --format json` at 0.41 (RFC 026) | the seam parses confined, version-gated output and refuses rather than guesses; never screen-scrapes unpinned prose |
| `UD-03` | **resolved at prikk 0.28** (was a 0.27.x defect) | Changes uses `worktree-status` directly, version-gated at ≥ 0.28; below it stikk explains rather than runs it |
| `UD-04` | the CLI panics on EPIPE | the seam drains output fully (already implemented) |
| `UD-08` | **retired at prikk 0.29** — `.prikkignore` excludes matching paths from `commit`'s walk and `worktree-status`'s untracked scan | prikk filters ignored paths before reporting them; stikk keeps its display-only untracked filter for what remains, and no longer claims files cannot be excluded |
| `UD-05` | **revised at prikk 0.28**: `0`/`1`/`2` (2 = usage error); exit 1 still covers refusal, dirty worktree and integrity failure alike | the seam classifies exit 1 by message + context; exit 2 is a stikk argument bug, surfaced as `stikk-internal` (RFC 009) |
| `UD-09` | **narrowed at prikk 0.32** (a messaged patch's *id* became enumerable via `log`), and its **content half retired at 0.36** — `prikk show <block-id\|patch-id> [--format json]` renders a patch's operations and content (RFC 021 F1; `FR-034`) | History shows block lineage + a block's state file list; Block detail lists every block's patches; **Patch detail is built** (0.9.0, RFC 035), rendering each operation from `show-report-v1` at prikk ≥ 0.39 |

## Releases and versioning

stikk versions independently of prikk and declares, per release, the prikk range it was validated
against — currently **`>= 0.28`, validated through `0.54.1`** (`NFR-R03`; 0.27.x dropped by owner
ruling 2026-09-04, RFC 009). A prikk newer than the validated ceiling still runs, and stikk says the
range is unvalidated rather than pretending to know it. Before a 1.0, the repository format and command surface of
prikk are still moving, so stikk stays pre-1.0 too and treats its own APIs as unstable. Changes are
recorded in [`CHANGELOG.md`](CHANGELOG.md); design decisions in `rfcs/`.

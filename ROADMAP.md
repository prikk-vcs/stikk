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
  ([RFC 026](rfcs/done/026-readiness-after-the-key-directory.md)). The validated range is now **0.28
  through 0.41**, and `log`/`branch`/`tag` are read as JSON at ≥ 0.39.
- **Confirmations name the key that will sign**, and say whether it is bound, unbound, or unchecked;
  prikk's published example keys are flagged (`C-S2`, implemented)
  ([RFC 023](rfcs/done/023-what-stikk-has-and-does-not-show.md), RFC 026).
- **Overlays size themselves by measurement** and say when they are clipped; seal's confirm affordance
  is on screen for the first time ([RFC 024](rfcs/done/024-overlay-sizing-and-the-fix-that-did-not-travel.md)).
- **The real-binary suite drives all eleven `Prikk` seam methods**
  ([RFC 022](rfcs/done/022-widening-the-real-binary-suite.md), and `readiness` in 0.6.0's prep).

## Next — carried into 0.7.0, in order

1. **`refused paths:` — landed on `main`** ([RFC 027](rfcs/done/027-what-commit-would-refuse.md)): commit is
   unavailable, with prikk's reasons, when prikk would refuse a path; and the unsupported paths stikk had
   never listed are listed. Ships in 0.7.0.
2. **The Queue view** (`FR-051`) — **[RFC 028](rfcs/accepted/028-the-queue-view.md), accepted; it waits for
   prikk's next release**, which carries each queued patch's message and lets `show` render a queued patch
   (prikk's reply 012). Until then the seal ceremony still says how many patches it will freeze but not which.
3. **Patch detail** (`FR-030`), then **Compare** (`FR-033`), each its own RFC.
4. `docs.yml`'s three node20 actions — including `peaceiris/actions-mdbook`, which has no node24 release
   to move to.
5. **A workspace-wide seed-read guard.** 0.6.0's notes say no seed value is read anywhere in stikk,
   and a source-level test holds that only in the two modules that name those variables. A guard
   over every shipped crate would let the sentence say *enforced by test* without a qualifier.
6. **The would-refuse overlay scrolls, and long paths get a row budget** — carried from RFC 027: a path past
   about 64 characters clips on an entry row, and prikk reports unsupported paths absolute.
7. **The prose `worktree-status` path's parse failures** (prikk < 0.39) still reach the refusal classifier,
   where the JSON path now reports stikk's own error.
8. **stikk opens focused on `heads/main`, which a repository may not have.** Measured at prikk 0.28 and 0.41,
   a ref with no published history reads as an empty baseline everywhere — `worktree-status` lists every
   file untracked, `log` reports empty history, and `commit` authors onto the new name. **That is prikk's
   deliberate model, not a defect**: it is exactly how a first commit is previewed, so nothing is asked of
   prikk. stikk has no free-text ref entry, so the only way to reach it is the default focus; Orientation
   already shows `<unpublished>`, and the Changes view should say the same rather than show a whole tree as
   untracked without comment.
9. **The next re-baseline is not routine.** prikk's unreleased `main` adds a **current branch**: `prikk branch
   switch`, `--ref` defaulting to it, a `current branch:` line in `status`, `worktree-status` and `log`, and
   `"current"` on each `branch-list-v1` entry. stikk's design has no HEAD and keeps a client-side focused ref
   (`FR-055`, `TU-03`), so whether stikk adopts prikk's current branch is an RFC-level decision, taken when that
   release is measured — the same release RFC 028 waits for. stikk always passes `--ref` explicitly, so nothing
   breaks silently meanwhile; where the new prose lines fall relative to the scoped entry region is measured then.

## Later — verification, branches/tags, merge, session, exchange, trust, and the GUI

- **Verify report browser and doctor/recovery** (`FR-100/101/102`), with the three-valued
  author-signature outcome rendered precisely (Sound / Unverifiable / a blocking failure) and locks
  never auto-cleared.
- **Branches and tags** (`FR-070/071`), the **focused-ref** model, and **checkout / deletion**
  planning and materialization (`FR-053/054`).
- **Merge evidence → plan → execution** (`FR-080…082`) and the **rollback flow** (`FR-083`) — the
  merge refusal path is where the explanation surface earns its place.
- **Session persistence and progressive disclosure** (`FR-122`, `TU-12`): resume the focused ref, view
  and filters; default vs. advanced depth. Cheap now that RFC 003's change token exists — and the
  increment that finally gives `C-E2` (the *primary* control against writing inside a repository) a
  production caller, which it does not have today.
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
| `UD-09` | **narrowed at prikk 0.32** (a messaged patch's *id* became enumerable via `log`), and its **content half retired at 0.36** — `prikk show <block-id\|patch-id> [--format json]` renders a patch's operations and content (RFC 021 F1; `FR-034`) | History shows block lineage + a block's state file list; Block detail lists a messaged patch's id/message; Patch detail as a rendered diff (3b) still waits; the gap is named where a user would open a patch |

## Releases and versioning

stikk versions independently of prikk and declares, per release, the prikk range it was validated
against — currently **`>= 0.28`, validated through `0.41.0`** (`NFR-R03`; 0.27.x dropped by owner
ruling 2026-09-04, RFC 009). A prikk newer than the validated ceiling still runs, and stikk says the
range is unvalidated rather than pretending to know it. Before a 1.0, the repository format and command surface of
prikk are still moving, so stikk stays pre-1.0 too and treats its own APIs as unstable. Changes are
recorded in [`CHANGELOG.md`](CHANGELOG.md); design decisions in `rfcs/`.

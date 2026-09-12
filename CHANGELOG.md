# Changelog

All notable changes to stikk are recorded here. Dates are ISO-8601.

## Unreleased

### Added

- **Commit's and seal's confirmations now name the key id that will sign.** `FL-05` step 5 has required
  this since before 0.4.0 — the confirmation showed `Consumes: AUTHOR`, a capability, where the
  requirement asks for the key. `FL-06` is amended to ask the same of seal: RFC 016 removed a typed
  key-id **act**, not the **information**, and a ceremony that freezes patches into permanent signed
  history should say which key is about to sign. Absent renders as nothing — there is no placeholder,
  because a confirmation is unreachable without the readiness the id accompanies.

  The id is read by a **new module**, `stikk_prikk::key_id`, deliberately separate from the
  presence-only `stikk_prikk::env`: that module is forbidden by a source-level test from materializing
  any environment value, and reading an id requires exactly the calls it forbids. The new module has the
  mirror-image guard — it may read `PRIKK_*_KEY_ID` values and may not so much as name a `*_SEED`
  variable. **No seed is read anywhere, and `env.rs` is untouched.** A key id is a label prikk prints
  itself, not key material.

- **The glossary's code explanations are reachable.** stikk has shipped **six** authored, reviewed,
  test-covered `GlossaryEntry` explanations whose text was rendered **nowhere** (a seventh arrives with
  the Windows gloss below): a refusal card's
  `glossary: <code>` line named a code with no way to read it. The Glossary & Help overlay now lists
  them — code, title, explanation, see-also — and **scrolls**, and **wraps**. Those three are one change
  on purpose: 0.4.1 shipped wrapping on its own and had to revert it, because without somewhere to
  scroll to, wrapping turned *truncated-but-present* into *absent* and left one of eleven terminology
  entries readable at 80×24. The scroll keys are `↑/↓` (or `j`/`k`), and the panel's own Keys section
  lists them.

- **A gloss for prikk 0.28's Windows path refusal.** `invalid name: backslashes are not allowed in
  repository paths` is verbatim, honest, and baffling to the user who typed no backslash — prikk 0.28
  built that path itself while committing a subdirectory on Windows (found by the widened suite; fixed
  upstream in prikk 0.29.0). stikk now says so beside prikk's own words, naming the version and the fix,
  and **without overstating it**: a top-level file commits normally at 0.28 on Windows. Off Windows the
  same message has the other cause entirely — a file whose name really does contain a backslash — and
  gets that gloss instead, because telling that user to upgrade prikk would be a stikk-authored claim
  contradicting the evidence beside it.

### Changed

- **The real-binary suite now covers ten of the ten `Prikk` seam methods** (RFC 022), up from five. The
  five it did not drive — `worktree_status`, `refs`, `tags`, `block_state`, `change_token` — are each
  now driven against real prikk binaries at both ends of the supported range, asserting what the
  repository *is* (re-read after the fact) rather than that a string parsed. `change_token` is asserted
  in both directions, because a token that never changes and a token that always changes both pass a
  one-sided test. Two failure-classifier arms are now provoked live rather than cited: a genuinely held
  lock, and the full-queue precondition that prikk 0.35 silently reclassified — so the next re-baseline
  finds a class-word change without a person looking for it.

  **Correcting 0.5.0's own entry:** that release's Security note said the suite covered *"four of the
  nine surfaces"*. It understated itself twice — there are **ten** seam methods, not nine, and
  `handshake` was driven all along by the version guard every test runs at both ends, so 0.5.0 shipped
  with **five of ten**. The released section is left as it stands; the number to trust is this one.

- **The fabricated-worktree-entry fix (0.5.0's `### Fixed`) is now pinned by a real binary**, not only
  by a captured fixture: the suite provokes it at 0.38 with a real `prikk mv` of a path whose first
  token is a change kind. It announces its skip at 0.28, where `prikk mv` does not exist — a test that
  quietly does nothing at one end is the inert-suite failure this project has warned itself about twice.

- **A failed suite run now says in one line whether the harness could not build a repository or stikk
  got an answer wrong.** Every test builds a fixture first, so one broken precondition previously
  produced one identical panic per test — the shape RFC 021's Windows break had.

- **`actions/checkout` is pinned past the Node 20 deprecation** in every workflow.

- **Documented: prikk 0.28 cannot commit a file in a subdirectory on Windows.** **Found by the widened
  suite on its first matrix run**, not by a person reading source: prikk 0.28's commit-side worktree
  scan built the repository path with the platform separator, and its own validator then refused the
  backslash. It is prikk's defect, fixed upstream in **0.29.0**, and affects only the floor of stikk's
  supported range and only on Windows — but it is a supported configuration, so it is now stated in
  Getting Started rather than left for a user to hit. The suite skips that one combination with an
  announced message and still asserts the rest.

### Security

- **`C-S2` is now marked *not implemented* in the threat model.** The control — recognizing prikk's
  published example keys and flagging them as unsafe — **has no implementation in stikk and never had
  one**, but §6's coverage table listed it beside `C-I1a–d` and `SEAM-06` as though all three were in
  force. Two are. The bullet, the coverage table and the attack-surface map all say so now, along with
  what it was waiting on and which increment ships it. **Nothing about stikk's behaviour changed**; what
  changed is that the document most responsible for telling a reader what protects them no longer makes
  a claim it cannot support.

### Fixed

- **`StikkError::LockConflict`'s documentation said it is presented as `FR-106`'s "another writer is
  active".** It is presented as prikk's verbatim message in a banner, with no gloss, and has been since
  RFC 017 narrowed the classifier. Documentation only; no behaviour changed.

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

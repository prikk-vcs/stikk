# stikk — Requirements Specification

| | |
|---|---|
| Document | stikk Requirements Specification (business-function level) |
| Version | v0.2 (draft for review) — v0.1 + project-rules alignment: added NFR-U (progressive disclosure, per "Less is more"), NFR-S07 (threat-model maintenance), CON-6/CON-7 (development workflow, project structure), ASM-4 (rules adoption). No existing IDs changed meaning |
| Date | 2026-08-31 |
| Basis | prikk **0.27.1** @ `3eb3101`, as established by the 2026-08-31 audit (`audit-2026-08-31-task-{1a,1b,2,3,4}.md`); project rules in `.git-exclude/rules/` (RFC lifecycle policy; general/Rust/GUI project instructions) |
| Authority | This document defines WHAT stikk does. Technology, architecture, and UI layout belong to the External Design (task 02) and later documents. Where stikk's needs exceed prikk 0.27.1, the gap is a numbered **Upstream Dependency (UD-xxx)**, never a silent assumption |
| ID scheme | `UC-` use case · `OS-` out of scope · `FR-` functional req · `NFR-` non-functional req · `CON-` constraint · `ASM-` assumption · `UD-` upstream dependency. Priorities: **M**ust / **S**hould / **C**ould (for v1) |

**stikk** (Norwegian: *to set a course, to take a bearing*) is the user-facing tool of the prikk ecosystem: a terminal UI (TUI) and a graphical UI (GUI) over one shared operation layer, for navigating, inspecting, and operating on prikk repositories. Its founding stance mirrors prikk's own: **where prikk refuses, stikk explains.** Evidence, witnesses, and refusal reasons are first-class content, not error noise.

## 0. Terminology: arriving from Git

Readers will bring Git mental models. The mapping below is normative for this document's vocabulary.

| Git concept | prikk / stikk reality |
|---|---|
| commit | **patch** (signed atomic change, carries its preimages) queued in the active **WAL**; `commit` appends to the queue |
| push to publish | **seal** — a MAINTAINER-signed **block** freezes the queue into immutable history |
| commit hash | patch id / block id (SHA-256 object ids) |
| author name/email/date | AUTHOR **key id** + signature. **No names, no emails, no dates anywhere** (no-clock design). Messages are not yet persisted by core (UD-01) |
| branch | ref `heads/<name>`: a signed RefState chain. Created, listed, **closed** (never deleted) |
| HEAD / checkout a branch | **does not exist.** No current-branch pointer; checkout materializes an explicit ref's state into the worktree. stikk keeps a client-side **focused ref** instead |
| staging area / index | **does not exist.** Whole-worktree commit; the queue (WAL) is the pre-publication holding area |
| diff | a patch **is** the diff: typed operations (create/delete/edit-text-span/replace-binary/change-perm) with preimages |
| merge | evidence → plan → execute, **only when proven confluent**; otherwise a refusal with typed conflict witnesses. No conflict markers, ever |
| revert | **rollback flow**: preview → inverse draft → verify → seal (preimage-exact) |
| rebase, cherry-pick, amend, squash | **do not exist**; sealed history is immutable (see OS-1) |
| remote / fetch / push | **no network.** File artifacts: bundles (whole verifiable subsets) and the 9-step sync negotiation loop; received history lands read-only under `remotes/` |
| fsck | `verify` — 14 stages; author-signature outcome is **Sound** or **Unverifiable** (both values; Unverifiable is non-blocking and must never read as a pass), or a **blocking failure** (an error, not a value) — see FR-035 |
| reflog | signed RefState/RefUpdate chain per ref (stronger; append-only) |
| gc / prune | **do not exist** (append-only, no-GC store) |

---

## 1. Stakeholders & Use Cases

### 1.1 Personas

- **P-1 — Solo developer / evaluator ("Astrid").** Tries prikk on a real side project. Needs the daily loop (change → commit → seal → history) to be legible without memorizing CLI flags, and needs refusals explained in plain language.
- **P-2 — Patch-theory reviewer ("Kenji").** Studies commutation, confluence, and merge evidence. Needs deep, faithful rendering of patches, witnesses, state roots, and lineage — the tool as a microscope, not a simplifier.
- **P-3 — Small-team maintainer ("Marta").** Coordinates 2–8 authors exchanging history over files (shared drive, mail, USB). Owns the MAINTAINER key, the trust policy, sealing, tag adoption, and the sync loop. Release-engineer analog.
- **P-4 — Auditor / security reviewer ("Sam").** Reviews someone else's repository or bundle. Cares about verify outcomes, signature continuity (TOFU), trust-policy state, and whether received history is safe to adopt. Often works read-only.

### 1.2 Primary use cases

| ID | Name | Narrative (condensed) |
|---|---|---|
| UC-01 | Open & orient | User opens a local repository; stikk shows at a glance: health (verify summary), refs (local / received / closed), queue depth, worktree state, signing readiness |
| UC-02 | Browse history | User walks a ref's lineage: blocks in sequence order, patches within blocks, the unsealed queue on top; filters narrow the view |
| UC-03 | Inspect a patch | User opens one patch and reads it as a diff: operations, preimages, touched paths, author key, signature status |
| UC-04 | Compare two blocks | User selects two blocks (any refs) and sees the state-level difference between their trees |
| UC-05 | Review & commit worktree changes | User reviews worktree-vs-baseline changes, then commits the whole worktree into the queue with an AUTHOR signature |
| UC-06 | Review queue & seal | User reviews queued patches, then performs the seal ceremony (MAINTAINER key; explicit no-audit acknowledgement) |
| UC-07 | Materialize a checkout | User plans a checkout of a ref (dry-run first), reviews the plan incl. any refusals, then materializes; separately plans/executes deletions |
| UC-08 | Manage branches | Create a branch from a ref; close a branch; set the client-side focused ref |
| UC-09 | Manage tags | Create a tag (with persisted message) targeting a ref or block; list tags |
| UC-10 | Roll back sealed history | User previews a rollback, appends the inverse draft, verifies it, and seals — guided end-to-end |
| UC-11 | Merge with evidence | User picks baseline + two sides, reads the evidence/plan, and executes when confluent |
| UC-12 | Understand a refusal | Any refused operation opens an explanation: the witness(es), what they mean, what the user can and cannot do next |
| UC-13 | Exchange via bundle | Export a ref to a bundle; verify a bundle offline; import one and see what arrived (received ref, objects, author keys) |
| UC-14 | Run the sync loop | stikk walks both sides of summary → compare → have → build → accept → pending → seal, telling the user at each step which file to hand to whom |
| UC-15 | Review received history & adopt | Inspect `remotes/` refs and received tags; adopt a maintainer key into trust (TOFU conflicts surfaced); adopt tags; merge received work |
| UC-16 | Verify & audit | Run verify; browse the 14 stages, per-item findings, and three-valued signature outcomes; export the machine-readable report |
| UC-17 | Recover | Surface doctor findings; run the safe WAL-tail repair; inspect held locks with liveness info and clear one after explicit confirmation |
| UC-18 | Signing readiness | See which roles (AUTHOR / MAINTAINER) are usable in this session, which key ids are active, and whether the trust policy knows the maintainer key — without stikk ever storing a seed |
| UC-19 | Resume a session | Reopen stikk and land where they left off: repository, focused ref, view, filters |
| UC-20 | Inspect an artifact offline | Open a bundle file with no repository present and browse what it claims to contain, with verify-level caveats displayed |

### 1.3 Explicitly out of scope (v1)

| ID | Excluded | Rationale (prikk reality) |
|---|---|---|
| OS-1 | rebase, cherry-pick, amend, squash, reorder, any history rewriting | Sealed blocks are immutable; no core operations exist; contradicts prikk's model rather than merely missing |
| OS-2 | In-tool conflict resolution, "mark resolved", conflict-marker editing | Core non-goal: merges refuse with evidence; resolution is a future core increment, not a UI feature |
| OS-3 | Hunk- or file-level staging | No staging area exists; commit is whole-worktree (partial commit is UD-06, a core change) |
| OS-4 | Remote URIs, network transport, hosting/forge features | prikk moves no bytes; exchange is file-artifact based by design |
| OS-5 | Git interoperability (import/export, `.git` reading) | Not provided by core (history import is a proposed RFC 113 track) |
| OS-6 | GC / prune surfaces | Append-only no-GC store; nothing to expose beyond `compact` (which stikk surfaces under recovery, FR-105) |
| OS-7 | Displaying wall-clock times for history | prikk history carries no time. stikk must not fabricate chronology from file mtimes or import times as if it were history metadata |
| OS-8 | Direct `.prikk/` reading/writing, repairs beyond doctor's safe surface | The store's layout is prikk's authority; bypassing it voids every core guarantee |
| OS-9 | Multiple worktrees per repository | One worktree per repository in core |
| OS-10 | Telemetry / usage analytics | Trust-sensitive audience; see NFR-S05 |

---

## 2. Functional Requirements

Notation: each requirement is "stikk shall …". *[M/S/C]* = v1 priority. *(→ UD-x)* marks a dependency on a prikk-core change; the FR text states the shipped degraded behaviour until it lands.

### 2.1 Repository discovery & opening (FR-001 …)

- **FR-001** *[M]* Open a prikk repository from a local filesystem path (explicit path, or upward discovery of `.prikk/` from the current directory). No remote URIs exist to support.
- **FR-002** *[M]* On open, present an orientation summary: format acceptance, ref counts (local / received / closed), queue depth per active session, worktree dirty marker, last verify status if known, signing readiness (FR-104).
- **FR-003** *[M]* Refuse-and-explain unopenable targets: retired repository formats (surface prikk's own migration message verbatim), missing/foreign directories, and version skew between stikk and the prikk core in use (ASM-2).
- **FR-004** *[S]* Open a **bundle file** as a first-class read-only object with no repository present: show its declared ref, tip, object/author-key counts, and offline-verify result, always displaying prikk's caveat that offline verification proves structure, not trust (UC-20).
- **FR-005** *[S]* Maintain a recent-repositories list (stored outside any repository; CON-4).
- **FR-006** *[C]* Open multiple repositories side by side (required by the sync loop's two-sided narrative UC-14; degraded alternative: guided single-side mode).

### 2.2 History browsing (FR-010 …)

- **FR-010** *[M]* List a ref's history in **lineage order** (block sequence via the RefState chain; patches in sealed order within each block), with the unsealed queue rendered as a visually distinct "not yet history" tier above the tip. Ordering is topological/sequential only (OS-7).
- **FR-011** *[M]* Present, per block row: block id (abbreviated, expandable), kind (Root/Normal/Merge/Repair/Import), patch count, sealing maintainer key id, update-seq; per patch row: patch id, author key id, purpose (normal/rollback-draft), operation summary (counts by type, touched paths).
- **FR-012** *[M]* Filter the history view by: ref; author key id; block kind; patch purpose; path touched (exact or prefix). *(→ UD-01)* Message filtering ships only when patch messages exist in core; until then the filter UI must not advertise it.
- **FR-013** *[M]* Free-text find over what exists: object ids (prefix), path names, tag names and tag messages, ref names. Content search inside blob bytes is *[C]* and must be labelled by cost (walks blob data).
- **FR-014** *[M]* Show received refs (`remotes/…`) in the same browser, visually distinct, read-only, with their trust standing (blocks signed by adopted keys or not) summarized at the ref level.
- **FR-015** *[S]* Show closed branches on demand (default hidden), rendered as closed, with the closure RefState visible in the chain.
- **FR-016** *[S]* From any block, show its ref-chain context: the RefState/RefUpdate entries that published it (the reflog-equivalent), including recovery-relevant fields (previous state id, update seq, publishing key id).
- **FR-017** *[C]* Graph view across refs sharing lineage (merge blocks connect two parents; mainline parent distinguished from adopted parent).

### 2.3 Inspection (FR-030 …)

- **FR-030** *[M]* Patch detail: every operation rendered as a human-readable diff — text-span edits shown as before/after with surrounding context (the preimage travels in the patch, so no worktree is needed); creates/deletes with content summary; binary replacements as old/new blob identity + size; permission changes as old/new mode. Raw operation view (exact fields, hashes, anchors) available for P-2/P-4.
- **FR-031** *[M]* Block detail: parents (mainline vs adopted for merge blocks), baseline claim for merges, state Merkle root, patch list, seal signature(s) and their verification standing.
- **FR-032** *[M]* File tree at a block: browse the replayed state (paths, kinds, modes, blob identities); open file content at that state.
- **FR-033** *[M]* Compare two blocks (same or different refs): state-level difference (added / removed / content-changed / mode-changed paths), each entry expandable to content diff. This is the range-diff equivalent.
- **FR-034** *[M]* Worktree-vs-baseline view for the focused ref: changed / missing / untracked / unsupported paths with per-file diffs. *(→ UD-03, **resolved**; → UD-09 for the per-file diffs)* Core's `worktree-status` was broken on ordinary repositories at 0.27.x and is **verified fixed at prikk 0.28** (RFC 008), so stikk uses it directly, version-gated at ≥ 0.28 and explaining rather than running it below that. The imagined replay/plan workaround is **superseded**: it was never feasible (it needs per-file baseline content prikk does not expose, and would require stikk to read worktree bytes directly, against CON-1). The view is **path-level**; per-file content diffs wait on UD-09. When prikk reports that the active WAL holds queued patches for a *different* ref, stikk must carry prikk's own warning verbatim — paths listed "untracked" there may be committed-but-unsealed work (RFC 009 F4).
- **FR-035** *[S]* Signature inspector on any signed object: role, key id, algorithm, and the outcome with a plain-language explanation of what it means and does not mean (TOFU continuity, not identity). The outcome has three user-facing states that stikk must keep distinct: **Sound** (verifies against recorded key material), **Unverifiable** (no key material recorded — *non-blocking; verify still passes; must never be shown as a pass/green state* — prikk's `AuthorSignatureVerification::Unverifiable`), and a **verification failure** (blocking — surfaced by prikk as an error/refusal, not a value). stikk must not collapse Unverifiable into either of the other two.
- **FR-036** *[C]* Blob detail: kind (text/binary/snapshot), size, referencing patches/blocks ("where is this content used?").

### 2.4 Working cycle: commit, queue, seal, checkout (FR-050 …)

- **FR-050** *[M]* Commit: review the whole-worktree change set (FR-034), require a message input (see UD-01 for its fate), require AUTHOR signing readiness, then queue the patch. Surface prikk's own result summary (patch id, operation counts, threshold warnings) faithfully.
- **FR-051** *[M]* Queue review: list queued patches with full patch detail (FR-030); show active-patch warn/limit thresholds as they apply; make "queued, not yet history" status unmistakable.
- **FR-052** *[M]* Seal ceremony: an explicit, multi-step confirmation that (a) shows exactly which patches will seal into one block on which ref, (b) requires MAINTAINER signing readiness, (c) requires an explicit no-audit acknowledgement while core requires `--allow-no-audit`, presented as informed consent, not a pre-ticked box.
- **FR-053** *[M]* Checkout, plan-first: every materialization is previewed (files to write, conflicts that will refuse, snapshot-vs-patch route) before any write; the user confirms the plan, not the idea. Refused overwrites are explained per file (UC-12).
- **FR-054** *[M]* Deletion flow mirrors core's separation: a distinct deletion plan (with per-path safety verdicts) and a distinct confirmed execution; unsafe candidates are shown and never auto-resolved.
- **FR-055** *[S]* Focused-ref model: stikk maintains a per-repository client-side focused ref (baseline for FR-034, default target for commit/seal/checkout). Switching focus never touches the worktree by itself; the UI must never imply a HEAD moved (no such thing exists).
- **FR-056** *[C]* Partial-scope commit (pathspec). *(→ UD-06)* Absent core support, out of v1 (OS-3); the commit view must make "commits are whole-worktree" explicit.

### 2.5 Refs: branches & tags (FR-070 …)

- **FR-070** *[M]* Branch create (name validation feedback incl. case-collision refusals surfaced with prikk's reason; `--from` any ref), branch list (FR-014/015 presentation), branch close with an explanation of closure semantics (pointer retained, history retained, reopenable only by a future core verb — none exists today).
- **FR-071** *[M]* Tag create targeting a ref or block, with message (persisted by core); tag list. Tag deletion does not exist and must not be offered.
- **FR-072** *[S]* Received-tag review and adoption flow (`sync tags` / `adopt-tag`): show the sender's signature standing, require MAINTAINER confirmation, explain that adoption re-signs under the local key.

### 2.6 Merge & rollback (FR-080 …)

- **FR-080** *[M]* Merge evidence browser: for baseline + left + right (blocks or refs, incl. received refs), render the evidence report — outcome, per-pair classifications, and every conflict witness with kind, side/op identification, and a plain-language gloss per witness kind (12 kinds; the glossary is a stikk asset).
- **FR-081** *[M]* Merge plan view: what would seal (adopted patches, resulting block shape) when confluent.
- **FR-082** *[M]* Merge execution: allowed only from a currently-confluent evidence state; requires MAINTAINER readiness + explicit confirmation; on core refusal (e.g., ref advanced meanwhile), re-run evidence and re-present rather than retrying silently.
- **FR-083** *[M]* Rollback, guided end-to-end (UC-10): preview (what the inverse will do) → append draft (AUTHOR-signed, purpose-tagged) → draft verification result → seal ceremony. Each stage shows core's own artifacts (inverse operation summary, draft patch id).
- **FR-084** *[S]* Merge-base assistance: since core requires an explicit baseline block, stikk computes and proposes common-ancestor candidates from lineage, clearly labelled as proposals the user confirms (core re-derives and checks the claim at verify time).

### 2.7 Exchange: bundles & sync (FR-090 …)

- **FR-090** *[M]* Bundle export (ref → file) with destination-collision and durability semantics delegated to core; show the export report (objects, author keys).
- **FR-091** *[M]* Bundle verify (offline) and import, rendering core's reports and its trust caveats verbatim; after import, land the user on the received ref's history (FR-014).
- **FR-092** *[M]* Sync loop guidance (UC-14): a stateful checklist walking the user through the full negotiation as the sender or receiver, generating/consuming the right artifact files in the right order, and stating at each hand-off which file goes to the other side. Artifact size/count limits surfaced before building.
- **FR-093** *[M]* Pending-claims view (`sync pending`): accepted-but-unsealed patches with their claims; seal-from-claim flow with the same ceremony rigor as FR-052; batch behaviour (stops at first failure) explained before execution.
- **FR-094** *[S]* Exchange dashboard per peer-ref: what we have vs. what the last summary/have-list said, so Marta can see divergence at a glance without decoding files by hand.

### 2.8 Integrity, trust & recovery (FR-100 …)

- **FR-100** *[M]* Verify runner & report browser: all 14 stages with status; per-item findings grouped by object/ref; warnings vs errors distinguished exactly as core does; three-valued author-signature outcomes explained (FR-035). Machine-readable export passes through core's `verify --format json`.
- **FR-101** *[M]* Doctor view: findings with core's codes and recommendations; the one safe repair (WAL-tail truncation) executable after confirmation showing exactly what will be truncated and which patch ids survive.
- **FR-102** *[M]* Lock inspector: held locks with kind, recorded PID, and advisory liveness exactly as core reports it (positive = reliable refusal, negative = *not* authorization); clearing a lock requires typed confirmation and shows core's warning; stikk never auto-clears (NFR-S04).
- **FR-103** *[M]* Trust management: adopted maintainer keys list; add/remove with TOFU semantics surfaced — a changed key under a known id is refused by core and stikk must present this as the security event it is, not a generic error.
- **FR-104** *[M]* Signing readiness (UC-18): per-role status (key id present? seed material available to the session? maintainer key adopted in trust policy?) with guidance drawn from the security-setup doc; stikk reads key state from the environment/session and never persists or displays seed material (NFR-S03). Example/tutorial seeds are recognized and flagged as unsafe for real use.
- **FR-105** *[S]* Compaction surface: show compactable containers with plan-only preview; execute per target after confirmation.
- **FR-106** *[S]* Concurrent-change awareness: stikk detects external repository changes (another CLI/process) and refreshes views rather than acting on stale state; lock conflicts from core are presented as "another writer is active", never as corruption.

### 2.9 Refusal explanation (FR-110 …)

- **FR-110** *[M]* Every core refusal reaching the user carries: (a) prikk's original message verbatim (support-grade fidelity, NFR-I03), (b) a plain-language explanation, (c) next-step options that actually exist (e.g., for a non-confluent merge: inspect evidence, choose a different baseline, wait for the other side — never "resolve conflicts", which does not exist).
- **FR-111** *[M]* A witness glossary: every conflict-witness kind and verify finding code has a maintained explanation page reachable from wherever it appears.
- **FR-112** *[S]* Refusal history for the session, so a user can revisit an explanation after closing it.

### 2.10 Session & cross-cutting UX (FR-120 …)

- **FR-120** *[M]* Preview-first: every mutating operation offers a dry-run view before execution wherever core provides one (checkout plans, deletion plans, merge plan, rollback preview, compaction plan) — and stikk must not add mutating operations that bypass an available preview.
- **FR-121** *[M]* Confirmation tiers: read operations free; queue-affecting operations confirmed; history-publishing (seal, merge, rollback-seal, tag/branch publication, trust changes, lock clearing) confirmed with operation-specific summaries. A global read-only mode locks tier 2–3 out entirely (NFR-S01).
- **FR-122** *[M]* Session persistence: per-repository focused ref, last view, filters, and layout persist across restarts; stored outside the repository (CON-4); absence of stored state degrades to defaults without error.
- **FR-123** *[M]* TUI/GUI parity: both frontends expose the same operation set via the shared operation layer (CON-2); any operation shipped in one and not the other is a defect, not a variant.
- **FR-124** *[S]* Everything addressable: ids, paths, and report lines copyable; deep-link identity (repo + object id / ref) shareable between stikk sessions as text.
- **FR-125** *[C]* Command palette exposing every operation by name with its keybinding (external design will specify bindings; the requirement is discoverability parity with menus/keys).

---

## 3. Non-Functional Requirements

### 3.1 Performance & responsiveness (NFR-P…)

- **NFR-P01** *[M]* The UI never blocks input: any operation that can exceed 100 ms runs asynchronously with visible progress.
- **NFR-P02** *[M]* Long-running core operations (verify, deep history walks, bundle build/import, sync accept) are cancellable from the UI; cancellation never leaves stikk's own state inconsistent (core operations are themselves crash-safe; stikk simply re-reads).
- **NFR-P03** *[M]* Capacity tiers, aligned with prikk's measured envelope rather than aspirational Git-scale numbers (core verify cost is linear in history; core benchmarks exist to ~160-block / 10k-file scale):
  - **Tier 1 (comfort), must:** ≤ 2,500 blocks and ≤ 25k tracked files — navigation interactions < 100 ms after initial load; open-to-oriented < 2 s excluding a full verify.
  - **Tier 2 (supported), should:** ≤ 25k blocks / ≤ 250k files — progressive loading; interactions < 500 ms; verify runs in background with staged results.
  - **Beyond tier 2:** functional with honest degradation (no hangs, no unbounded memory), no latency promises.
- **NFR-P04** *[S]* Expensive views (block-state trees, comparisons) are computed lazily and cached per session; cache invalidation keys off repository change detection (FR-106), never off wall-clock.
- **NFR-P05** *[M]* Verify is never run implicitly on open beyond what orientation needs; full verify is user-initiated or explicitly scheduled (its linear cost belongs to the user's decision).

### 3.2 Accessibility (NFR-A…)

- **NFR-A01** *[M]* TUI: fully keyboard-operable; every view reachable and operable without a pointer.
- **NFR-A02** *[M]* GUI: exposes platform accessibility APIs (names, roles, focus order, keyboard operability for every action).
- **NFR-A03** *[M]* Status and severity are never encoded by color alone (shape/label always present); palettes meet WCAG AA contrast in both light and dark presentation.
- **NFR-A04** *[S]* Reduced-motion preference respected; no information conveyed solely by animation.

### 3.3 Internationalization (NFR-I…)

- **NFR-I01** *[M]* All stikk-authored UI strings externalized; initial locales **en, ja, nb**; locale switch without restart *[S]*.
- **NFR-I02** *[M]* Ids, paths, ref names, and key ids are never translated or reshaped.
- **NFR-I03** *[M]* prikk's own diagnostics are English and are the support-grade ground truth: stikk may add localized explanation (FR-110) but always preserves and can show the original message.

### 3.4 Security & safety (NFR-S…)

- **NFR-S01** *[M]* Read-only by default posture: a fresh session performs no mutation until the user acts; a global read-only mode exists (FR-121) and is the default when signing readiness is absent.
- **NFR-S02** *[M]* stikk writes nothing inside `.prikk/` and never modifies repository content except through prikk operations (CON-1). stikk's own files (session state, config) live in user-scope locations.
- **NFR-S03** *[M]* Key hygiene: seed material is read from the session environment for the duration of an operation, never persisted, logged, or displayed; UI shows key **ids** only. Recognized example/tutorial seeds trigger a persistent warning (FR-104).
- **NFR-S04** *[M]* stikk never auto-clears locks, never bypasses a core refusal, and never retries a refused mutation without user action.
- **NFR-S05** *[M]* No telemetry, no network calls of any kind (the tool of a no-network VCS is itself no-network).
- **NFR-S06** *[S]* Untrusted-content discipline: bundle/received content is rendered inert (no execution, no link-following, no format-string interpretation of history content).
- **NFR-S07** *[M]* stikk maintains a **threat model** as a first-class document (`stikk-03-threat-model`). Per project rules, every release whose changes involve new data flows, external integrations, or auth/signing logic updates the threat model; other releases verify existing controls remain valid. (Grounding: `.git-exclude/rules/project-instructions-general-common.md` §Release Deliverables.)

### 3.7 Usability & progressive disclosure (NFR-U…)

Grounding: project rule "**Less is more** — sophisticated UI/UX comes from limited information and considered workflows; users start immature; advanced views can be added for matured users."

- **NFR-U01** *[M]* Default views present the minimum information the current task needs; deep or expert detail (raw operation fields, ref-chain internals, per-item verify findings, exchange internals) is opt-in via explicit expanders or an **advanced mode**, never shown by default.
- **NFR-U02** *[M]* The primary workflow (open → review changes → commit → seal → history) must be completable end-to-end using only default-view information plus the confirmations of FR-121; nothing on that path may require advanced mode, the glossary, or prior prikk expertise beyond the §0 mapping.
- **NFR-U03** *[S]* Advanced mode is a persistent per-user preference (CF), switchable at runtime; switching it never changes semantics, only depth of display.

### 3.5 Portability (NFR-T…)

- **NFR-T01** *[M]* Mutating stikk runs where mutating prikk runs: Linux, macOS, Windows. Read-only stikk degrades gracefully on read-only prikk platforms, with mutation affordances visibly disabled and explained.
- **NFR-T02** *[M]* TUI: standard terminal environments on the three platforms (POSIX terminals; Windows Terminal-class on Windows), degrading legibly on limited color/Unicode support.
- **NFR-T03** *[S]* GUI: cross-platform desktop on the same three platforms, native-conventional (menus, shortcuts, file dialogs) per platform.

### 3.6 Robustness (NFR-R…)

- **NFR-R01** *[M]* stikk must be safe to kill at any moment: repository safety is core's (crash-safe by design); stikk's own state files are written atomically and tolerate absence/corruption by resetting to defaults.
- **NFR-R02** *[M]* stikk holds no repository lock across user think-time; every core operation is bracketed (acquire-act-release semantics live in core's own operations).
- **NFR-R03** *[M]* Version honesty: stikk states which prikk version range it was validated against (ASM-2) and refuses gracefully (read-only where possible) outside it, rather than misrendering unknown formats. The range has **two ends and they behave differently**: below the floor stikk degrades or refuses; **above the validated ceiling stikk still runs but says the range is unvalidated**, because refusing every prikk newer than the last stikk release would break users on the day prikk ships a minor. An *unbounded* upper range is not honesty — it silently asserts knowledge stikk does not have, and is how a shape change goes unnoticed (RFC 009 decisions 6–7).

---

## 4. Constraints & Assumptions

- **CON-1** *[M]* stikk drives prikk exclusively through prikk's public surfaces and never reads or writes `.prikk/` content directly. The *current* public surface is: the `prikk` CLI (stable-ish command set; machine-readable output only on `verify`; 0/1 exit codes) and pre-1.0 library crates whose APIs may change without notice. The External Design must choose the integration seam; **this document constrains the choice** by requiring: no direct storage access; behaviour identical to what the CLI would do; and the UD-list below as the honest gap record for whichever seam is chosen.
- **CON-2** *[M]* One shared operation layer beneath both frontends; TUI and GUI differ only in presentation (FR-123).
- **CON-3** *[M]* Reversibility is expressed in prikk's model, not a mutable undo log: sealed history is never rewritten; compensation is the rollback flow (FR-083); the signed ref chain is the audit trail (FR-016); and every mutating operation is preview-first (FR-120) so "undo" pressure is minimized at the source. stikk must not present an "undo" affordance for published history.
- **CON-4** *[M]* All stikk-owned persistent state (config, sessions, caches) lives outside every repository, in user-scope app locations; a repository must remain byte-identical whether or not stikk ever opened it (except through explicit prikk operations).
- **CON-5** *[S]* stikk is versioned and released independently of prikk; compatibility is declared per release (NFR-R03).
- **CON-6** *[M]* Development follows the project workflow: Requirements → External Design → Internal Design → Program Design → Implementation → Testing, with **design specifications as the source of truth for test design** — test cases validate the specs, not merely the written code. (Grounding: rules §Feature Development.)
- **CON-7** *[M]* stikk's own project structure follows the rules: English throughout; concise README in the rules' six-section shape; full documentation under `docs/src` (mdBook-compatible) organized by the three documentation personas; Apache-2.0 with `LICENSE`/`NOTICE`, author **nabbisen**; development plans and change tracking via the stikk repository's own `rfcs/` (five-folder variant of the RFC lifecycle policy, adopted to keep "accepted for implementation" a distinct event from "shipped") and `CHANGELOG.md`.
- **ASM-1** prikk-core evolution continues along its RFC process; stikk requirements name dependencies (UD-xxx) rather than forking core semantics.
- **ASM-2** v1 targets prikk **≥ 0.28**, **validated through 0.31.0** (owner ruling 2026-09-04, RFC 009 decision 6; ceiling raised to 0.31.0 by RFC 012 F-e, 2026-09-05, on an empirical fixture re-capture with no shape difference found). **0.27.x is dropped**: its `worktree-status` is the UD-03 defect, which stikk already refuses to run, so carrying 0.27 in the range promised what stikk could not serve. Format changes before stikk v1 are absorbed by re-validation, not by stikk reading old formats itself (core refuses retired formats with migration guidance — FR-003 surfaces it). **Re-validation is a standing obligation, not a release chore:** prikk is pre-1.0 and changed its output shapes twice between 0.28 and 0.30 (RFC 009) and, separately, its on-disk schema — with no CLI shape change — at 0.31 (RFC 012 F-e, re-verified empirically rather than trusted from prikk's own changelog); this project checks every time rather than assuming either way.
- **ASM-3** Users hold their own key material and channel for artifact exchange; stikk manages neither.
- **ASM-4** The project rules in `.git-exclude/rules/` (RFC lifecycle policy `000-rfc-lifecycle-policy.md`; general, Rust, and GUI project instructions) apply to stikk as written. Language and edition are therefore fixed upstream of internal design (Rust, 2024 edition — a fact the internal design consumes; this business document still specifies no technology itself).

---

## 5. Upstream Dependencies on prikk (UD-…)

Each names the core gap, the stikk requirements it blocks or degrades, and the shipped behaviour until it lands. These should be filed against prikk as issues/RFC input; several already appear in the 2026-08-31 audit.

| ID | prikk gap (audit ref) | Blocks / degrades | stikk behaviour until landed |
|---|---|---|---|
| **UD-01** | Patch messages validated then discarded; no author display name (audit 1A-High-2) | FR-012 (message filter), FR-030/011 (message display), UC-02/03 richness | Commit UI still collects a message (FR-050) and states plainly that core does not yet persist it; history shows ids/keys/paths only; no fabricated titles |
| **UD-02** | Machine-readable output exists only on `verify` (audit CLI findings); library API pre-1.0 | Nearly all read FRs if the CLI seam is chosen; report exports (FR-100, FR-124) | External Design must pick: parse human output (fragile — rejected), drive libraries behind a pinned version (CON-5 discipline), or await `--format json` on log/status/branch/tag/merge-evidence. Requirement: stikk never screen-scrapes prose it can't pin |
| **UD-03** | ~~`worktree-status` broken on normally-created repositories (audit 1A-High-1)~~ — **RESOLVED at prikk 0.28**, verified against the live binary (RFC 008) | FR-034, UC-05 | stikk uses `worktree-status` directly, version-gated at ≥ 0.28; below it stikk explains rather than running the pre-fix command |
| **UD-04** | CLI panics on EPIPE (audit 1B-Medium) | Any CLI-seam integration that closes pipes early | stikk drains subprocess output fully; upstream fix filed |
| **UD-05** | **REVISED at prikk 0.28**: exit codes are no longer 0/1 but `0` success / `1` operational failure / `2` usage error. The 1-vs-semantics coarseness remains (a refusal, a dirty worktree and an integrity failure all exit 1); the usage case is now distinguishable | Scripted error classification behind FR-110's explanations | stikk classifies exit 1 by message + context, degrading to a generic explanation on unknown messages and always showing the original (NFR-I03). **Exit 2 means stikk built a bad argument list** — a stikk bug — and is surfaced as `stikk-internal`, never as prikk's refusal (RFC 009 F6) |
| **UD-06** | No pathspec/partial commit; whole-worktree only | FR-056 (C-priority), OS-3 | Whole-worktree commit presented honestly; no staging metaphor anywhere in UI copy |
| **UD-07** | Merge-base discovery is manual in core (README known limit) | FR-084 quality | stikk computes candidate ancestors client-side from lineage it reads through the public seam; labelled as proposals |
| **UD-08** | ~~No ignore mechanism (audit 1A-Medium)~~ — **RETIRED at prikk 0.29**: `.prikkignore` at the repository root excludes matching paths from `commit`'s worktree walk and `worktree-status`'s untracked scan (verified against the live binary, RFC 009 F5). It binds at **discovery only**, so it never changes what sealed history means; a malformed file **fails closed** | FR-034/FR-050 signal-to-noise on real projects | prikk filters ignored paths before reporting, so they never reach stikk. stikk still offers its **view-level** untracked filter, marked display-only, for the paths that *are* reported — and must not claim there is no way to exclude files, nor report a count of ignored paths, which prikk does not expose |

---

## 6. Traceability Matrix (use cases → functional requirements)

| Use case | Primary FRs | Supporting FRs / NFRs |
|---|---|---|
| UC-01 Open & orient | FR-001, FR-002, FR-003 | FR-005, FR-104, NFR-P03, NFR-R03 |
| UC-02 Browse history | FR-010, FR-011, FR-012, FR-013 | FR-014, FR-015, FR-016, FR-017, UD-01 |
| UC-03 Inspect a patch | FR-030 | FR-035, FR-036, UD-01 |
| UC-04 Compare blocks | FR-033 | FR-032, NFR-P04 |
| UC-05 Review & commit | FR-034, FR-050 | FR-055, UD-03, UD-06, UD-08 |
| UC-06 Queue & seal | FR-051, FR-052 | FR-121, FR-104 |
| UC-07 Checkout | FR-053, FR-054 | FR-120, FR-110 |
| UC-08 Branches | FR-070, FR-055 | FR-015, FR-110 |
| UC-09 Tags | FR-071 | FR-072 |
| UC-10 Rollback | FR-083 | FR-120, FR-121, FR-052 |
| UC-11 Merge | FR-080, FR-081, FR-082 | FR-084, FR-110, UD-07 |
| UC-12 Refusals | FR-110, FR-111 | FR-112, NFR-I03 |
| UC-13 Bundles | FR-090, FR-091 | FR-004, FR-014 |
| UC-14 Sync loop | FR-092, FR-093 | FR-094, FR-006, FR-121 |
| UC-15 Received & adopt | FR-014, FR-072, FR-103 | FR-082, FR-091 |
| UC-16 Verify & audit | FR-100 | FR-035, FR-111, NFR-P05, UD-02 |
| UC-17 Recover | FR-101, FR-102 | FR-105, FR-106, NFR-S04 |
| UC-18 Signing readiness | FR-104 | NFR-S01, NFR-S03 |
| UC-19 Resume | FR-122 | FR-005, CON-4, NFR-R01 |
| UC-20 Offline artifact | FR-004 | FR-091, NFR-S06 |

Persona coverage check: P-1 lives in UC-01–08, 12, 19; P-2 in UC-02–04, 11–12, 16, 20; P-3 in UC-06, 08–11, 13–15, 17–18; P-4 in UC-15–16, 20 with NFR-S posture. Every FR traces to at least one UC; UD items trace to the FRs they unblock.

Rules-derived items (v0.2): NFR-U01–U03 cross-cut every UC's default views (P-1 is the calibration persona; P-2/P-4 are the advanced-mode audience); NFR-S07 and CON-6/CON-7/ASM-4 are process/structure constraints realized by the threat model (`stikk-03`), the internal design (`stikk-04`), and the stikk repository skeleton rather than by individual use cases.

---

*End of Requirements Specification v0.1. Next document: External Design (task 02) — note that its current draft also assumes Git semantics (rebase flows, staging views, drag-to-cherry-pick, HEAD in the status bar) and should be revised against this document's OS-1…OS-10 and the terminology mapping before drafting begins.*

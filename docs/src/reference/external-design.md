# stikk — External Design Specification

| | |
|---|---|
| Document | stikk External Design Specification (black-box view) |
| Version | v0.2 (draft for review) — v0.1 + project-rules alignment: added TU-12/GU-09 (progressive disclosure per NFR-U01–U03), advanced-mode entry in CF-03, size-guarded rendering note in TU-11. No existing IDs changed meaning |
| Date | 2026-08-31 |
| Inputs | Requirements Specification **v0.2** (`stikk-01-requirements-spec-v0.1.md`) — cited as UC/FR/NFR/CON/ASM/UD; prikk 0.27.1 reality per the 2026-08-31 audit; project rules in `.git-exclude/rules/` |
| Authority | Defines WHAT the user sees and does at every stikk boundary. Internal architecture, module decomposition, libraries, languages, and serialization are out of scope (internal design). |
| ID scheme | `BD-` boundary · `AC-` actor · `CL-` launcher contract · `TU-` TUI · `GU-` GUI · `CF-` configuration · `FL-` flow · `CT-` external data contract · `OP-` operational behaviour. (`DC-` deliberately avoided — it collides with prikk's RFC numbering.) |

Design stance carried from requirements: **where prikk refuses, stikk explains** (FR-110/111); **preview-first** (FR-120); **tiered confirmation** (FR-121); **TUI/GUI parity through one operation set** (FR-123, CON-2). No UI exists for operations prikk does not have (OS-1…OS-10); Git-shaped expectations are redirected in copy ("revert" → rollback flow, "switch branch" → focused ref + checkout plan).

---

## 1. System Boundary & Actors

### 1.1 Boundary

- **BD-01 — Inside stikk:** the TUI frontend, the GUI frontend, the shared operation layer they both call (CON-2), the explanation content (refusal glosses, witness/finding glossary — FR-111), and stikk's own state store (config, sessions, caches — CON-4).
- **BD-02 — Outside stikk:** prikk core (the only path to any repository byte — CON-1); the repository (`.prikk/` and the worktree); bundle and sync artifact files; the operating system (terminal, desktop shell, accessibility APIs, user-scope config locations); the user's exchange channel (ASM-3); the human actors.
- **BD-03 — The single write rule:** repository and worktree mutation happens only by asking prikk (NFR-S02). stikk itself writes only BD-01 state files, in user scope, atomically (NFR-R01). A repository remains byte-identical whether or not stikk ever opened it, except through explicit prikk operations (CON-4).
- **BD-04 — One client among many:** stikk assumes concurrent prikk CLI use and other stikk instances. It detects external repository change and refreshes rather than acting on stale state (FR-106); it never holds a repository lock across user think-time (NFR-R02).
- **BD-05 — No network boundary exists.** stikk makes no network calls of any kind (NFR-S05). File dialogs and paths are the only way content enters or leaves.

### 1.2 Actors

stikk has no accounts. Capability is derived per session from prikk-side facts — signing readiness (FR-104) and the read-only mode (NFR-S01) — and is displayed, never stored.

| ID | Actor | Derived from | May do (via stikk) |
|---|---|---|---|
| AC-01 | **Viewer** | no signing readiness, or read-only mode on | Every read surface: browse, inspect, compare, verify, evidence, exchange inspection (FR-121 tier 1) |
| AC-02 | **Author** | AUTHOR readiness present | Viewer + queue-affecting: commit (FR-050), rollback draft (FR-083 steps 1–3) (tier 2) |
| AC-03 | **Maintainer** | MAINTAINER readiness present | Author + history-publishing: seal, merge execution, branch/tag publication, trust changes, tag adoption, sync build/seal (tier 3) |
| AC-04 | **Operator** | any human at the machine, explicit confirmations | Recovery surfaces: doctor's safe repair, lock clearing, compaction (FR-101/102/105, NFR-S04). Orthogonal to the *mutating* axis (AC-02/03) — no signing readiness is required — but **not** an exemption from the global read-only mode: `FR-121` locks tier 2–3, lock clearing included, out entirely when read-only is on (RFC 012 F-a; corrected 2026-09-05 — this row previously said "always tier 3", which contradicted FR-121) |

Persona mapping (informative): P-1 ≈ Author, P-2/P-4 ≈ Viewer, P-3 ≈ Maintainer + Operator.

---

## 2. External Interfaces

### 2.1 Launcher command-line contract (CL-…)

**stikk is not a second prikk CLI** (revised task ruling; CON-1, UD-02). VCS verbs on a command line belong to `prikk`. stikk's command line only launches and aims the frontends, plus launcher-scope utilities.

- **CL-01 — Launch TUI:** `stikk [path]` opens the TUI on the repository at `path` (default: upward `.prikk/` discovery from the current directory — FR-001; if none, the recents/picker screen — FR-005).
- **CL-02 — Launch GUI:** `stikk --gui [path]` (and the desktop launcher) opens the GUI identically. All other options below apply to both frontends.
- **CL-03 — Aiming (deep links):** `--ref <name>` sets the focused ref (FR-055) and opens History; `--object <id>` opens the matching detail view (patch/block/tag) (FR-124); `--view <orientation|history|changes|queue|refs|evidence|exchange|trust|verify|recovery>` opens a named view. Invalid aims degrade to Orientation with a notice, never an error exit.
- **CL-04 — Headless utilities** (no UI): `stikk --version`; `stikk config check [file]` (validates CF-02, prints findings); `stikk config path` (prints CF-01 locations). *Requirement backing: none direct — justified as CF-02/OP-01 support; flagged per task rule.*
- **CL-05 — Launcher exit codes** (distinct from prikk's; scripts must not conflate): `0` launched/utility ok · `2` usage error · `3` config invalid (`config check` failure) · `4` environment unsuitable (e.g., TUI without a TTY, unsupported terminal). The TUI/GUI session itself always exits `0` once interactive; operation outcomes are never encoded in the process exit code.
- **CL-06 — Terminal discipline:** the TUI owns the terminal only while attached to a TTY; otherwise it refuses with guidance (exit 4) rather than emit control sequences into a pipe. Headless utilities write plain text to stdout, diagnostics to stderr, and honour `NO_COLOR` (CF-04).
- **CL-07 — No VCS verbs, stated in-product:** `stikk log`, `stikk commit`, etc. print one line redirecting to `prikk <verb>` and exit `2` — the vocabulary stays prikk's.

### 2.2 Terminal UI layout model (TU-…)

- **TU-01 — View inventory** (each maps to requirements; all reachable from the palette TU-07 and listed in §6):

| View | Purpose | Grounding |
|---|---|---|
| Orientation (root) | health, refs summary, queue depth, worktree marker, signing readiness | UC-01, FR-002 |
| History | lineage of the focused/selected ref; queue tier on top; filters | UC-02, FR-010–017 |
| Patch Detail | one patch as a diff + raw operation view | UC-03, FR-030 |
| Block Detail & State Tree | block metadata, signatures, file tree at block, file content at state | FR-031/032 |
| Compare | two-block state difference, expandable to content diffs | UC-04, FR-033 |
| Changes | worktree-vs-baseline for the focused ref, per-file diffs | UC-05, FR-034 |
| Queue | queued patches, thresholds, entry to Seal ceremony | UC-06, FR-051 |
| Refs | branches (open/closed/received) and tags; create/close/tag actions | UC-08/09, FR-070/071 |
| Merge Evidence | baseline/left/right slots; evidence report; plan; execute entry | UC-11, FR-080–082 |
| Exchange | bundle export/verify/import; sync-loop checklist; pending claims; peer dashboard | UC-13/14, FR-090–094 |
| Trust & Keys | adopted maintainer keys; add/remove; signing readiness detail | UC-15/18, FR-103/104 |
| Verify Report | stage list, per-item findings, signature outcomes, export | UC-16, FR-100 |
| Recovery | doctor findings + safe repair; lock inspector; compaction | UC-17, FR-101/102/105 |
| Refusal Explanation (overlay) | verbatim message, gloss, next steps, glossary links, session history | UC-12, FR-110/112 |
| Glossary / Help (overlay) | witness kinds, verify codes, terminology mapping, key reference | FR-111 |
| Command Palette (overlay) | fuzzy access to every operation/view/ref/recent object | FR-125 |
| Background Operations (overlay) | running/finished long operations, progress, cancel | NFR-P01/P02 |

- **TU-02 — Shell layout:** a fixed header line (repository name · focused ref), the active view, a one-line status bar, and a transient overlay layer (explanations, confirmations, palette, glossary, background ops). Overlays never destroy view state underneath.
- **TU-03 — Status bar contents** (FR-002/055; deliberately no "HEAD" — none exists): repository short name · focused ref · queue depth (`●n queued`, hidden at 0) · worktree marker (clean/dirty/unknown) · capability badges `[RO]` `[AUT ✓/–]` `[MNT ✓/–]` (NFR-S01, FR-104) · background-operation indicator (`⟳ n`) · palette hint. Every badge has a text form (NFR-A03) and is focusable for its explanation.
- **TU-04 — Navigation model:** a view stack. Drill-in pushes (History → Patch Detail → Block Detail…), back pops; Orientation is the stack root. Slots-based views (Compare, Merge Evidence) accept picks initiated elsewhere ("send to Compare A/B", "send to Merge left/…") and become the top view when armed. State transitions:

| From | User action | To |
|---|---|---|
| any | palette selection / deep link | named view (stack reset to Orientation + target) |
| Orientation | choose ref | History |
| History | open patch / open block | Patch Detail / Block Detail |
| History / Refs | send block to Compare slot | Compare (armed when both slots filled) |
| History / Refs | send ref-or-block to Merge slot | Merge Evidence |
| Changes | commit action | Commit confirmation → Queue |
| Queue | seal action | Seal ceremony → History (on success) |
| Merge Evidence | execute (confluent only) | Confirmation → History |
| Merge Evidence | outcome is a refusal | Refusal Explanation overlay |
| any refused operation | automatic | Refusal Explanation overlay (FR-110) |
| Exchange (sync step) | step complete | next checklist step (FL-12) |
| Recovery | clear lock / repair | typed confirmation → Recovery (refreshed) |

- **TU-05 — Global key bindings** (defaults; all rebindable — CF-03; shown in Help): `?` glossary/help · `:` command palette · `Esc`/`q` close overlay / back / quit at root (confirm if operations are running) · `Tab`/`Shift-Tab` pane cycle · `/` find within view (FR-013) · `r` refresh view (FR-106) · `b` background operations · arrows/`j k` movement · `Enter` open/drill-in · `y` copy focused id/path (FR-124).
- **TU-06 — Context keys (notable defaults):** design rule — **mutating actions are uppercase** and always confirm (FR-120/121). History: `f` filters, `a` send to Compare A, `s` send to Compare B, `m` send to Merge slot chooser, `T` tag this block. Changes: `Enter` file diff, `e` open in external editor (CF-03), `C` commit. Queue: `S` seal. Refs: `N` new branch, `X` close branch, `T` new tag. Merge Evidence: `p` plan, `M` execute merge. Exchange: `E` export bundle, `I` import, `v` verify bundle, `n` next sync step. Recovery: `R` repair WAL tail, `U` clear selected lock. Verify: `V` run verify, `x` export report.
- **TU-07 — Command palette:** fuzzy-matches operations, views, refs, and recent object ids; each entry shows its binding and required capability; entries the session cannot perform stay visible but disabled with the reason ("MAINTAINER key not ready — see Trust & Keys") (FR-104, FR-125).
- **TU-08 — Refusal overlay pattern** (uniform for every refusal — FR-110): ① prikk's message verbatim (copyable) ② plain-language explanation ③ next steps that exist, as actionable entries (e.g., "Inspect evidence", "Choose another baseline", "Open Trust & Keys") ④ glossary links for every witness/code named ⑤ kept in session refusal history (FR-112).
- **TU-09 — Confirmation overlay pattern** (FR-121): shows exactly-what (operation, target ids, counts — e.g., "Seal 3 patches → new block on heads/main"), the capability consumed, and the consequence class. Tier-2 = explicit yes. Tier-3 (seal, merge execute, rollback-seal, ref publication, trust change, lock clear, tag adoption) = restate + explicit yes; lock clearing and trust changes additionally require typing the target name (NFR-S04, FR-102/103). The seal ceremony's no-audit acknowledgement is its own unchecked-by-default step (FR-052).
- **TU-10 — Degraded terminals:** minimum 80×24 (below: a legible "terminal too small" screen); monochrome fallback renders every status by text/shape (NFR-A03); ASCII-only mode when Unicode is unavailable (CF-03); reduced-motion disables spinners in favour of counters (NFR-A04).
- **TU-11 — Long-content discipline:** wide content (diffs, trees, reports) scrolls within its pane, never breaks the shell; digits align in tables; ids abbreviate with expand-on-focus and full-copy (FR-124). Very large renderables (oversized diffs, huge trees) render a size summary with an explicit "show anyway" expander instead of freezing the pane (NFR-P01; threat model T-DO1).
- **TU-12 — Progressive disclosure (NFR-U01–U03):** every view has a **default depth** showing only what its task needs, and an **advanced depth** behind explicit expanders or the global advanced mode: History default hides update-seq/state-root columns; Patch Detail default is the rendered diff (raw operation fields are the expander); Block Detail default hides the ref-chain internals (FR-016 content is advanced); Verify default shows stage statuses and counts (per-item findings expand); Exchange default shows the current checklist step only. The primary loop — open → Changes → commit → seal → History — is fully operable at default depth (NFR-U02). Advanced mode is a persistent preference (CF-03), toggled at runtime, changing display depth only, never semantics.

### 2.3 Graphical UI layout model (GU-…)

- **GU-01 — Window structure:** left sidebar (repository switcher + view list mirroring TU-01 + refs tree with open/closed/received groups — FR-014/015), main pane (active view), right inspector pane (detail of current selection: signature inspector FR-035, blob detail FR-036), bottom status strip mirroring TU-03. Panes collapsible; layout persists per repository (FR-122).
- **GU-02 — Menu bar inventory:** **App/File** Open Repository… · Open Recent ▸ · Open Bundle… (FR-004) · Read-only Mode ✓ · Quit. **Repository** Verify · Doctor · Locks… · Compact… · Refresh. **History** Focus Ref… · Filters… · Compare… · Find. **Work** Commit… · Seal… · Checkout Plan… · Deletion Plan… · Rollback… **Exchange** Export Bundle… · Verify Bundle… · Import Bundle… · Sync Assistant… · Pending Claims. **Trust** Adopted Keys · Add Maintainer Key… · Signing Readiness. **View** Theme ▸ (System/Light/Dark) · Font… · Panes ▸ · Locale ▸. **Help** Glossary · Terminology (Git → prikk) · prikk Documentation · About.
- **GU-03 — Toolbar:** Open · focused-ref selector (never labelled "branch switcher"; tooltip explains FR-055) · Commit · Seal · Verify · Sync Assistant · Read-only toggle (turning read-only **on** is always allowed; turning **off** requires signing readiness — NFR-S01) · Palette.
- **GU-04 — Context menus (per view, notable):** History block row — Inspect · Send to Compare A/B · Use as Merge baseline/left/right · Tag this block… · Copy id. History patch row — Inspect · Copy id. Changes row — Open diff · Open in editor · Copy path. Refs: branch — Focus · History · Close… ; received ref — History · Merge from… · Copy; tag — Inspect target · Copy. Received tag — Adopt… (FR-072). Verify finding — Explain · Copy · Go to object. Lock row — Explain liveness · Clear… (typed, FR-102).
- **GU-05 — Drag-and-drop inventory (prikk-legal only):** drag a block → Compare slot A/B (FR-033); drag a block or ref → Merge baseline/left/right slot (FR-080); drag a bundle file from the OS onto the window → Inspect/Import chooser (FR-004/091); drag a ref onto the Changes pane → opens the **checkout plan** for it — never an immediate write (FR-053, FR-120). **Deliberately absent:** dragging a commit onto a branch (cherry-pick/rebase gestures — OS-1); dragging files into a "stage" (OS-3). Drop targets appear only while a legal drag is in progress; every drop lands in a preview or chooser, never directly in a mutation.
- **GU-06 — Dialog parity:** refusal, confirmation, and the three ceremonies (seal, rollback, sync assistant) are the same patterns as TU-08/09 rendered as dialogs/wizards; every dialog action is also palette-reachable and keyboard-operable (FR-123, NFR-A02).
- **GU-07 — Theming & fonts:** theme System/Light/Dark; UI font and monospace content font (family + size); high-contrast option; reduced motion follows the OS setting with a manual override (NFR-A03/A04, CF-03).
- **GU-08 — Accessibility:** full keyboard operability with visible focus; accessibility tree exposes names/roles/values for every control; status-strip badges readable by screen readers as text (NFR-A02/A03).
- **GU-09 — Progressive disclosure in the GUI (NFR-U01–U03):** TU-12's default/advanced depths apply identically; the inspector pane is the natural home of advanced detail (collapsed sections by default); advanced mode lives in the View menu and is the same persisted preference as the TUI's (CF-03) — one setting, both frontends (FR-123).

### 2.4 Configuration interface (CF-…)

- **CF-01 — Locations** (user scope, per platform convention; **never inside a repository** — CON-4): a config file at the platform user-config directory under `stikk/`, and a state directory (sessions, caches, refusal history) under the platform user-state/data directory. `stikk config path` prints both (CL-04). Deleting the state directory is always safe (NFR-R01: defaults restored).
- **CF-02 — Format:** one human-editable, line-oriented declarative text file with sections and `key = value` entries; documented and versioned with the product; unknown keys warn (named in the notice) and are ignored — they never block launch; syntactic errors fall back to full defaults with a visible notice (OP-01) and a pointer to `stikk config check`.
- **CF-03 — Configurable items:** key bindings (per stable action id, TUI and GUI); theme; locale (`en`/`ja`/`nb` — NFR-I01); **advanced mode** (default off — TU-12/GU-09, NFR-U03); confirmation strictness (`default` | `strict` — may only tighten over FR-121, never below it); external editor command template for opening worktree files (Changes view; the only external program stikk ever launches, and only on explicit user action); diff context lines; show-closed-branches default (FR-015); accessibility: `reduced-motion`, `high-contrast`, `ascii-only`; recents length (FR-005); per-repository override section (e.g., opt out of focused-ref memory). *No merge-tool integration exists to configure (OS-2).*
- **CF-04 — Environment variable catalogue** (precedence: environment > config file > default):

| Variable | Owner | Effect |
|---|---|---|
| `STIKK_CONFIG` | stikk | Absolute path of the config file to use |
| `STIKK_STATE_DIR` | stikk | Override state directory |
| `STIKK_READ_ONLY=1` | stikk | Forces read-only mode for the session (NFR-S01); cannot be overridden from inside the UI |
| `STIKK_LOCALE` | stikk | Locale override (NFR-I01) |
| `NO_COLOR` | convention | Disables color in TUI and headless output (CL-06) |
| `TERM`, platform a11y/motion settings | OS | Respected as found (TU-10, NFR-A04) |
| `PRIKK_AUTHOR_KEY_ID` / `PRIKK_AUTHOR_SEED` / `PRIKK_MAINTAINER_KEY_ID` / `PRIKK_MAINTAINER_SEED` and other `PRIKK_*` | prikk | **Pass-through, read-only.** stikk reads presence (never seed values) for readiness display (FR-104); never stores, logs, or displays material (NFR-S03); never sets or modifies them |

- **CF-05 — Application of changes:** config is read at launch; theme and locale apply immediately when changed from the UI (NFR-I01-S); everything else applies on restart, stated in the settings surface.

---

## 3. User Interaction Flows (FL-…)

Numbered as *user action → system response*. Every mutation passes a TU-09 confirmation; every refusal lands in TU-08. Flows cite the requirement they realize.

**FL-01 — Open & orient (UC-01).** 1. User launches `stikk [path]`. 2. Orientation renders: format acceptance, refs summary (local/received/closed counts), queue depth, worktree marker, last known verify status, signing readiness badges (FR-002/104). 3. Unopenable target → full-screen refusal with prikk's message verbatim (retired format guidance included) (FR-003). 4. User picks a ref → History (FR-055 sets focus).

**FL-02 — Browse with filters (UC-02).** 1. History shows the queue tier above the sealed lineage (FR-010). 2. User opens filters (`f`): ref, author key id, block kind, patch purpose, path (FR-012; message filter absent until UD-01). 3. Applying filters re-renders with an active-filters line; filters persist in the session (FR-122). 4. `/` finds ids/paths/tag text within the view (FR-013).

**FL-03 — Inspect a patch (UC-03).** 1. User opens a patch row. 2. Patch Detail renders operations as diffs (text spans with context from carried preimages; creates/deletes; binary old/new; mode changes) (FR-030). 3. `Tab` switches to the raw operation view (fields, hashes, anchors). 4. Inspector shows the signature standing with its three-valued gloss (FR-035).

**FL-04 — Compare two blocks (UC-04).** 1. From any History/Refs context, user sends block → Compare A, then block → Compare B (TU-04/GU-05). 2. Compare renders the state difference: added/removed/content-changed/mode-changed (FR-033). 3. Opening an entry shows the content diff; `y` copies ids/paths (FR-124).

**FL-05 — Commit (UC-05).** 1. User opens Changes; stikk shows worktree-vs-baseline with per-file diffs (FR-034, via UD-03 workaround). 2. Untracked noise may be view-filtered — banner states a commit still captures those files (UD-08). 3. User invokes Commit (`C`). 4. Message prompt (required non-empty), with the notice that core does not yet persist it (UD-01). 5. Confirmation summarizes the whole-worktree capture (file counts) and the AUTHOR key id to be used (FR-121 tier 2). 6. On yes → prikk commit runs; result (patch id, operation counts, threshold warnings) shown verbatim; Queue badge increments (FR-050/051). 7. Refusal (e.g., lock held) → TU-08.

**FL-06 — Seal ceremony (UC-06).** 1. From Queue, user invokes Seal (`S`). 2. Ceremony step 1: exactly-what — how many patches will seal into one block, on which ref, and the block that results (FR-052, amended 2026-09-05: prikk exposes the queued count and target ref, never the patch ids, so the ceremony names what it knows and says it cannot enumerate). 3. Step 2: the no-audit acknowledgement, unchecked, with its meaning explained. 4. Step 3: MAINTAINER key id confirmation (readiness re-checked; not ready → guidance to Trust & Keys, FR-104). 5. Execute → success shows new block id and RefState; History refreshes with the queue tier emptied. 6. Any refusal (incomplete publication, lock) → TU-08 with recovery pointers (FR-110).

**FL-07 — Checkout, plan then materialize (UC-07).** 1. User drops/aims a ref at Changes or invokes Checkout Plan. 2. Plan view: files to write, unchanged files, per-file conflicts that will refuse, route (snapshot/patch) (FR-053). 3. User confirms the plan → materialization runs; report summarizes written/unchanged. 4. Deletions are a separate flow: Deletion Plan lists candidates with per-path safety verdicts; unsafe candidates are visible and inert (FR-054); a distinct confirmation executes only the safe set.

**FL-08 — Merge, confluent path (UC-11).** 1. User fills Merge Evidence slots: baseline block (or accepts a proposed ancestor, labelled proposal — FR-084/UD-07), left = focused ref, right = ref/received ref. 2. Evidence renders: outcome, per-pair classifications, witnesses if any (FR-080). 3. Outcome confluent → user opens Plan (adopted patch ids, resulting block shape) (FR-081). 4. Execute (`M`) → tier-3 confirmation naming into-ref, from-ref, baseline, patch count. 5. Success → History shows the merge block (two parents, mainline marked). 6. If the ref advanced meanwhile, prikk's refusal is shown and evidence re-runs automatically once (FR-082).

**FL-09 — Merge, refusal path (UC-12).** 1–2 as FL-08. 3. Outcome not confluent → Refusal overlay: verbatim reason, witness list; each witness expands to its glossary entry (kind, the two operations, paths/nodes involved) (FR-080/110/111). 4. Next-step entries offered: inspect the conflicting patches (→ FL-03), try a different baseline (→ FL-08 step 1), open the sync assistant if the right side is stale. 5. The explanation is retained in session refusal history (FR-112). *No resolution affordance exists or is implied (OS-2).*

**FL-10 — Rollback (UC-10).** 1. User invokes Rollback on the focused ref. 2. Step 1 Preview: what the inverse will change (FR-083, plan-first FR-120). 3. Step 2 Draft: message prompt (UD-01 notice), AUTHOR confirmation → inverse draft appended; draft patch id shown. 4. Step 3 Verify: draft verification result rendered; failure → TU-08 and the flow halts with the draft visible in Queue. 5. Step 4 Seal: as FL-06. 6. History then shows the rollback block flagged as such.

**FL-11 — Bundle exchange (UC-13, UC-20).** *Export:* 1. Export Bundle → ref chooser + destination file dialog. 2. Confirmation; core writes; report (objects, author keys) shown (FR-090). *Verify/Inspect:* 3. Open/drop a bundle → offline report with the structure-not-trust caveat verbatim (FR-004/091). *Import:* 4. Import → tier-2 confirmation; result names the received ref; view lands on its history with the "no local ref advanced, no key trusted" notice (FR-091, FR-014).

**FL-12 — Sync loop (UC-14).** 1. Sync Assistant asks the role: receiver or sender. 2. The checklist renders all steps with the current one armed; each step names the artifact file to produce or consume and **who it goes to** (FR-092). Receiver: produce summary → hand over → consume peer's build → accept (limits shown first) → review pending claims (FR-093) → seal-from-claim (ceremony as FL-06). Sender: consume summary → compare → consume have-list → build (MAINTAINER) → hand over. 3. Each completed step advances the checklist; the assistant is resumable across sessions (FR-122). 4. The peer dashboard summarizes local-vs-last-known divergence (FR-094).

**FL-13 — Trust adoption with TOFU conflict (UC-15).** 1. Trust & Keys lists adopted keys; user invokes Add (key id + public key) or Adopt from a received tag (FR-072/103). 2. Tier-3 typed confirmation explains what trusting the key means. 3. If prikk refuses because the key id is known with different material, the refusal overlay presents it **as a security event**: what TOFU continuity means, why this is refused, and that no override exists in stikk (FR-103, NFR-S04).

**FL-14 — Verify & report (UC-16).** 1. User invokes Verify (`V`); it runs in background with progress (NFR-P02/P05). 2. Report renders stages → per-item findings, warnings vs errors as core distinguishes them; signature outcomes grouped Sound/Unverifiable/Failed with glosses (FR-100/035). 3. Each finding links to its object view and glossary entry (FR-111). 4. Export writes core's JSON verbatim or stikk's versioned export (CT-02) via a file dialog.

**FL-15 — Stale-lock recovery (UC-17).** 1. A lock-conflict refusal or Recovery view shows held locks: path, kind, recorded PID, advisory liveness with core's exact asymmetry explained (positive = refuse; negative ≠ authorization) (FR-102). 2. User selects a lock → Clear. 3. Typed confirmation (lock name) with the two-writers warning. 4. Result shown; the originating operation is **not** auto-retried (NFR-S04).

**FL-16 — Resume (UC-19).** 1. User relaunches stikk. 2. The last repository opens at the persisted view/focused ref/filters (FR-122). 3. Missing/corrupt session state → defaults silently (NFR-R01); external changes since last run are reflected, not replayed (FR-106).

---

## 4. Data Contracts (External) (CT-…)

- **CT-01 — Inputs stikk accepts:**

| Input | Form | Notes |
|---|---|---|
| Repository | local filesystem path | FR-001; no URI schemes exist (OS-4) |
| Bundle file | file path (dialog, argument, or drop) | FR-004/091; content handed to prikk, never parsed by stikk |
| Sync artifacts (summary, have-list, exchange, claims) | file paths via the assistant | FR-092/093; opaque to stikk |
| stikk config file | CF-01/CF-02 | validated by CL-04 |
| stikk session state | CF-01 state dir | stikk-owned; safe to delete |
| Environment | CF-04 catalogue | `PRIKK_*` read-only pass-through |

- **CT-02 — Outputs stikk produces:** rendered screens; stikk-owned config/session files (atomic writes — NFR-R01); **report exports** on explicit user action only: (a) prikk's `verify --format json` passed through byte-verbatim, labelled as prikk's `verify-report-v1` schema (FR-100); (b) stikk-authored exports of evidence/report/refusal views as plain text and as JSON under a versioned `stikk-export-v1` label (FR-124) — schema contents are internal-design scope, the versioning and stability promise are external. Repository bytes, bundles, and sync artifacts are **always written by prikk** at stikk's request (CON-1, NFR-S02).
- **CT-03 — Inter-process contract with prikk core, as request/response categories** (serialization and transport are internal design, bounded by UD-02; behaviourally each category commits to the properties below):

| Category | Covers (examples) | Mutates | Cancellable | Lock behaviour | Surfaced error classes |
|---|---|---|---|---|---|
| read-history | ref lists, lineage, patch/block reads, ref chains | no | yes | none held by stikk | integrity-finding, environment |
| read-state | state trees, content at block, comparisons | no | yes | none | integrity-finding, environment |
| worktree-analysis | changes vs baseline (UD-03 route) | no | yes | none | environment |
| queue-mutation | commit, rollback draft | yes | between prikk calls only | prikk-internal, per call | refusal, lock-conflict, not-ready |
| publication | seal, merge execute, branch/tag publish | yes | before execution only | prikk-internal, per call | refusal, lock-conflict, not-ready, integrity-finding |
| exchange | bundle export/verify/import, sync build/accept/seal | import/build/seal yes | before execution; long imports abortable per core semantics | prikk-internal | refusal, limits, integrity-finding |
| integrity | verify, doctor (read) | no | yes | none | integrity-finding |
| trust | key add/remove, tag adoption, readiness probe | yes | before execution | prikk-internal | refusal (TOFU), not-ready |
| recovery | doctor repair, lock clear, compaction | yes | before execution | prikk-internal / explicit | refusal, environment |

  Idempotency note: stikk treats every mutating category as **not idempotent** and never auto-retries (NFR-S04); re-invocation is always a fresh user decision with fresh preconditions (FR-106, FR-082).
- **CT-04 — Error classes** (the vocabulary OP-03 presents): `refusal` (prikk's semantic no — FR-110 pattern) · `lock-conflict` ("another writer is active" — FR-106) · `not-ready` (signing/trust prerequisites — FR-104) · `integrity-finding` (verify/doctor content — FR-100/101) · `limits` (artifact ceilings surfaced pre-execution — FR-092) · `environment` (I/O, permissions, version skew — NFR-R03) · `stikk-internal` (OP-03).
- **CT-05 — Concurrency contract:** at most one mutating request in flight per repository from a stikk instance; reads may run concurrently; after any external-change detection, armed previews are marked stale and must be re-generated before their execute action re-enables (FR-106, FR-120).

---

## 5. Operational Behaviours (OP-…)

- **OP-01 — Startup:** 1. launcher parses arguments (CL-01…07); 2. config loads — invalid syntax → defaults + a visible notice naming the problem and `stikk config check` (CF-02); 3. repository selection (argument → discovery → recents/picker); 4. Orientation renders within NFR-P03 budgets (< 2 s tier-1, excluding verify); 5. **no implicit full verify** — orientation uses cheap reads and the last known verify status with its age by repository-state, never wall-clock (NFR-P05, OS-7).
- **OP-02 — Long-running operations:** run in background (NFR-P01) with determinate progress where core reports counts, otherwise indeterminate with an elapsed counter; cancellable where CT-03 allows (NFR-P02) — cancelling a multi-step ceremony stops before the next prikk call and reports exactly which steps completed (core calls themselves are crash-safe; stikk refreshes state after cancellation). Completed operations post results to the Background Operations overlay (TU-11) and, where relevant, refresh affected views.
- **OP-03 — Error presentation by class (CT-04):** `refusal` → TU-08 overlay; `lock-conflict` → non-modal banner with a jump to the Lock inspector (FR-102); `not-ready` → inline guidance toward Trust & Keys (FR-104); `integrity-finding` → routed into the Verify/Doctor views, never a popup; `limits` → shown in the pre-execution confirmation, not after failure; `environment` → plain statement with the failing path and the original message (NFR-I03); `stikk-internal` → a fault screen stating that the repository was not touched, session state is preserved, and where the log/state files are — the session may continue read-only.
- **OP-04 — External change & multi-writer behaviour:** on detected repository change, a passive "repository changed outside stikk — refreshed" notice; armed plans/previews are invalidated per CT-05. stikk never blocks another writer: it holds no locks between operations (NFR-R02, BD-04).
- **OP-05 — Shutdown & cleanup guarantees:** quitting with operations running prompts once (cancel-and-quit / wait); session state is flushed atomically on every transition, not only at exit, so `kill -9` at any instant loses at most the last view transition and never repository state (NFR-R01); no locks survive stikk exit because none are held (NFR-R02); temporary export files are written to their final names only on completion.
- **OP-06 — Read-only platforms & degraded capability:** on platforms where prikk is read-only, every mutating affordance renders disabled with the platform reason (NFR-T01); the same disabled-with-reason presentation applies to missing signing readiness (FR-104) and read-only mode (NFR-S01) — capability is always visible, never silently absent.

---

## 6. Traceability (design items → requirements)

| Design items | Realize |
|---|---|
| BD-01…05 | CON-1/2/4, NFR-S02/S05, NFR-R02, FR-106, ASM-3 |
| AC-01…04 | FR-104, FR-121, NFR-S01, FR-101/102/105 |
| CL-01/02 | FR-001, FR-005, UC-01 |
| CL-03 | FR-055, FR-124 |
| CL-04 | *(no direct FR — flagged; supports CF-02, OP-01, NFR-R03)* |
| CL-05/06/07 | CON-1, UD-02, UD-05 (non-conflation), NFR-A03 |
| TU-01 | UC-01…20 view coverage; FR-002, 010–017, 030–036, 050–054, 070–072, 080–084, 090–094, 100–105, 110–112, 125 |
| TU-02/03 | FR-002, FR-055, FR-104, NFR-S01, NFR-A03, NFR-P01 |
| TU-04 | FR-120 (armed previews), FR-110 (auto-overlay), UC navigation |
| TU-05/06/07 | NFR-A01, FR-125, FR-121 (uppercase-mutate rule), FR-013, FR-124, CF-03 |
| TU-08 | FR-110/111/112, NFR-I03 |
| TU-09 | FR-121, FR-052, FR-102/103, NFR-S04 |
| TU-10/11 | NFR-A03/A04, NFR-T02, FR-124, NFR-P01 |
| TU-12 / GU-09 | NFR-U01/U02/U03, FR-123, CF-03 |
| GU-01 | FR-014/015, FR-035/036, FR-122, FR-123 |
| GU-02/03/04 | FR surface parity (FR-123); FR-004, FR-053, FR-070–072, FR-090–094, FR-100–105; NFR-S01 |
| GU-05 | FR-033, FR-080, FR-004/091, FR-053, FR-120; OS-1/OS-3 (deliberate absences) |
| GU-06/07/08 | FR-123, NFR-A02/A03/A04, CF-03 |
| CF-01 | CON-4, NFR-R01, CL-04 |
| CF-02 | OP-01, NFR-R01 |
| CF-03 | FR-121 (tighten-only), FR-015, FR-005, NFR-A03/A04, NFR-I01; OS-2 (absence) |
| CF-04 | NFR-S01/S03, NFR-I01, CL-06 |
| CF-05 | NFR-I01 |
| FL-01 | UC-01; FR-001/002/003/055/104 |
| FL-02 | UC-02; FR-010/012/013/122; UD-01 |
| FL-03 | UC-03; FR-030/035 |
| FL-04 | UC-04; FR-033/124 |
| FL-05 | UC-05; FR-034/050/051/121; UD-01/03/08 |
| FL-06 | UC-06; FR-052/104/110 |
| FL-07 | UC-07; FR-053/054/120 |
| FL-08 | UC-11; FR-080/081/082/084; UD-07 |
| FL-09 | UC-12; FR-080/110/111/112; OS-2 |
| FL-10 | UC-10; FR-083/120/121; UD-01 |
| FL-11 | UC-13/20; FR-004/090/091/014 |
| FL-12 | UC-14; FR-092/093/094/122 |
| FL-13 | UC-15/18; FR-072/103/104; NFR-S04 |
| FL-14 | UC-16; FR-100/035/111; NFR-P02/P05 |
| FL-15 | UC-17; FR-102; NFR-S04 |
| FL-16 | UC-19; FR-122/106; NFR-R01 |
| CT-01 | FR-001/004/091/092; OS-4; CF-01/04 |
| CT-02 | FR-100/124; CON-1; NFR-S02; NFR-R01 |
| CT-03 | CON-1; UD-02 (seam deferred); FR-106/110; NFR-S04; NFR-P02 |
| CT-04 | FR-110/100/101/104/092; NFR-R03 |
| CT-05 | FR-106/120; NFR-S04 |
| OP-01 | NFR-P03/P05; CF-02; FR-001/002/005; OS-7 |
| OP-02 | NFR-P01/P02; TU-11 |
| OP-03 | FR-110/102/104/100/101; NFR-I03; CT-04 |
| OP-04 | FR-106; NFR-R02; CT-05 |
| OP-05 | NFR-R01/R02 |
| OP-06 | NFR-T01; NFR-S01; FR-104 |

Unbacked items: **CL-04 only** (flagged above). Every OS-excluded capability is represented solely by deliberate absences (GU-05, CF-03) and redirecting copy (design stance), never by controls.

---

*End of External Design v0.1. Next document (internal design) must decide the prikk integration seam within CON-1/UD-02, define the `stikk-export-v1` schema promised by CT-02, and bind the action-id catalogue referenced by CF-03/TU-05.*

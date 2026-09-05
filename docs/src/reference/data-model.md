# stikk — Data Model & Lifecycle Specification

| | |
|---|---|
| Document | stikk Data Model & Lifecycle (design-level; feeds Internal Design and Threat Model) |
| Version | v0.1 (draft for review) |
| Date | 2026-08-31 |
| Inputs | Requirements v0.2, External Design v0.2, prikk 0.27.1 reality, project rules (`.git-exclude/rules/`) |
| Scope | The data **stikk itself owns** and its lifecycle. prikk's repository objects (patches, blocks, refs, tags, trust policy, WAL) are **not** stikk's data — stikk reads them through prikk and never persists copies as authority. This document is deliberately explicit about that line because it is the project's central safety property (CON-1, NFR-S02). |
| ID scheme | `DM-` data entity · `LC-` lifecycle rule · `VW-` view-model (ephemeral) · `INV-` invariant |

Design stance from the rules: *design data structures for long-term safety, maintainability, and simplicity; balance feature-fit against general flexibility; avoid both rigid feature-coupling and vague over-abstraction* (`project-instructions-general-common.md`).

---

## 1. The two-world model

**INV-1 — stikk owns no repository truth.** Every durable datum stikk writes is a *convenience* — a pointer, a preference, a cache, or an export snapshot — that can be deleted with no effect on any repository (CON-4, NFR-R01). Nothing stikk stores is ever read back *as authority* about repository state; on every use, repository facts are re-derived from prikk. This single rule is what makes stikk safe to run against a repository another tool is also using (BD-04, FR-106).

Two data worlds, never mixed:

| World | Owner | Authority | Where it lives | stikk's relationship |
|---|---|---|---|---|
| **Repository world** | prikk | prikk | `.prikk/` + worktree | reads via prikk; **never writes**; never caches as truth |
| **stikk world** | stikk | stikk | user-scope config + state dirs (CF-01) | full read/write, atomic, deletable |

**INV-2 — Nothing stikk-owned lives inside a repository.** No `.prikk/`, no worktree dotfile, no sidecar next to repository files (CON-4). stikk's state is keyed *by* repository identity but stored *away from* it. **This is stikk's own responsibility, not something prikk will catch:** prikk's `.prikk/` has a *design invariant* — "No name under `.prikk/` is created after `init`" (`repository-layout.md:77-79`) — but there is **no general foreign-file scan**; `verify` inspects only `refs/tmp/` and a retired object tree, so a stikk file written into `containers/`, `active/`, `trust/`, `cache/`, or `.prikk/` root would go undetected (and a stray worktree file would additionally risk being captured by the next `commit`: `.prikkignore`, prikk 0.29+, can exclude a matching path, but only if a rule already covers it — **UD-08 retired**, RFC 009 F5 — so stikk cannot rely on it as a backstop for its own state). The control is therefore in `stikk-state`: the path resolver refuses any repository-internal target before every write, enforced by test (threat model C-E2, internal design MOD-03 `paths.rs`).

---

## 2. Entities stikk owns (DM-…)

All stikk-owned data is small, human-comprehensible, and non-secret (INV-3, below). Grouped by store.

### 2.1 Configuration store (one file, user-authored — CF-01/CF-02)

- **DM-01 — Config.** The single declarative config file. Fields (from CF-03): keybindings (per stable action id), theme, locale, advanced-mode default, confirmation strictness (`default`|`strict`, tighten-only), external-editor command template, diff context, show-closed-branches default, accessibility flags (`reduced-motion`/`high-contrast`/`ascii-only`), recents length, and a per-repository override section. **Human-owned:** stikk reads it, validates it (CL-04), and may write it *only* through an explicit in-app settings action that the user confirms; stikk never silently rewrites the user's config file.
  - **INV-4** Unknown keys are preserved on any stikk-initiated rewrite, never dropped (a user's newer key must survive an older stikk writing the file back).

### 2.2 State store (stikk-managed, user-scope — CF-01)

- **DM-02 — RepositoryHandle.** The identity stikk uses to recognize a repository across sessions and to key everything below. Fields: a canonical absolute path (the key, for now — see LC-9); a display name; last-opened marker. **No repository content** — a handle names a repository, it does not describe it.
- **DM-03 — RecentRepositories.** An ordered, bounded (CF-03 length) list of RepositoryHandles (FR-005). Pure convenience; losing it costs nothing.
- **DM-04 — SessionState (per repository).** What UC-19/FR-122 resumes: focused ref name (a string, re-validated against prikk on load — LC-6), last active view id, active History filters (FR-012 predicate values only: ref/author-key/kind/purpose/path — all re-applied against fresh data), pane layout / advanced-mode override, and scroll/selection anchors expressed as **stable object ids**, not indices. **INV-5:** every stored reference into repository content is an id or a name that stikk re-resolves on load; a resolution miss degrades to a default, never an error (FR-122, NFR-R01).
- **DM-05 — SyncAssistantProgress (per repository, per peer-exchange).** The resumable checklist state for FL-12: role (sender/receiver), current step, and the **paths** of the artifact files produced/consumed so far — never their contents (the bytes are prikk's/the peer's; stikk holds only where they are). A missing/moved artifact path invalidates the step with a re-pick prompt, never a crash.
- **DM-06 — RefusalHistory (per session, optionally persisted).** The session's refusal explanations (FR-112): the verbatim prikk message, the classified error class (CT-04), the operation attempted, and timestamps. Persisted only if the user has not enabled a private/ephemeral session (LC-8). Content is diagnostic text, not repository authority.
- **DM-07 — UIPreferencesRuntime.** Ephemeral-but-persisted-on-change runtime toggles that aren't "settings" proper: last window size/position (GUI), last-used export directory, advanced-mode current value. Flushed atomically on change (OP-05).

### 2.3 Cache store (derived, disposable — CF-01 state dir)

- **DM-08 — DerivedViewCache (per repository).** Memoized results of expensive read-only derivations (block-state trees, comparisons, evidence reports) keyed by **(canonical repository path, prikk version, input object ids, derivation kind)** — the repository component follows `DM-02`'s own key; the fingerprint originally proposed here was deferred (LC-9) — and stamped with the **repository-change token** current when computed (LC-4). Never a source of truth: a cache hit is used only after the change token is confirmed still current; any mismatch discards it and re-derives (FR-106, NFR-P04). **INV-6:** deleting the entire cache store is always safe and only costs recomputation.
- **DM-09 — GlossaryContent.** The witness-kind and verify-finding explanations (FR-111) plus the Git→prikk mapping. Ships *with* stikk (a product asset, versioned with the release), not user data; listed here because views bind to it. It is keyed by prikk finding/witness codes, so a prikk version introducing a new code that the glossary lacks must degrade to "no gloss yet — showing prikk's message only" (NFR-I03), never hide the message.

### 2.4 Export outputs (stikk-authored, user-directed — CT-02)

- **DM-10 — ReportExport.** Two shapes, both written only on explicit user action to a user-chosen path: (a) prikk's `verify --format json` passed through **byte-verbatim** under prikk's own `verify-report-v1` label; (b) stikk-authored `stikk-export-v1` (evidence/report/refusal snapshots) as text and versioned JSON. **INV-7:** a stikk-authored export is a *labelled snapshot* — it carries the repository's canonical path, prikk version, and capture time, and states in-band that it is a point-in-time view, so it can never be mistaken for live repository authority when read back later or elsewhere.

### 2.5 What stikk deliberately does NOT store (the negative model)

- **DM-N1 — No key material, ever** (INV-3, NFR-S03). Not the seed, not a derived key, not a hash of a seed. stikk reads `PRIKK_*_SEED` *presence* from the environment for readiness display and holds nothing. Key **ids** (public identifiers) may appear in views and logs; secret material may not exist in any stikk-owned datum, in memory beyond the moment of use, or in any log.
- **DM-N2 — No repository object copies as authority.** Rendered content (a diff, a tree) may sit in a view-model for the moment it is displayed and in DerivedViewCache under the change-token guard; it is never persisted as a durable record of what the repository contains.
- **DM-N3 — No shadow trust store.** stikk never records its own notion of which keys are trusted; trust is prikk's `MaintainerTrustPolicy` alone, read live every time it matters (FR-103, FR-104).
- **DM-N4 — No shadow ref/branch state.** The focused ref (DM-04) is a *client-side pointer preference*, not a HEAD; it stores a name, and prikk remains the authority on whether that ref exists and where it points (FR-055).

---

## 3. View-models (ephemeral — VW-…)

View-models exist only while a view is open; none are persisted (except as the id-only anchors in DM-04). They are listed so the internal design has a stable vocabulary and so the "nothing persisted here" property is explicit.

| ID | View-model | Sourced live from prikk each open/refresh | Notes |
|---|---|---|---|
| VW-01 | OrientationSummary | ref counts, queue depth, worktree marker, readiness, last-verify status | FR-002; readiness from env presence + prikk trust read |
| VW-02 | HistoryList | ref lineage + queue tier | FR-010; paged/progressive at Tier-2 scale (NFR-P03) |
| VW-03 | PatchView / BlockView | one patch/block detail + rendered diff/tree | FR-030/031/032 |
| VW-04 | CompareResult | two-block state diff | FR-033; cacheable (DM-08) |
| VW-05 | ChangesView | worktree-vs-baseline | FR-034 via UD-03 route |
| VW-06 | QueueView | queued patches + thresholds | FR-051 |
| VW-07 | EvidenceReport | merge outcome + witnesses | FR-080; cacheable |
| VW-08 | VerifyReportView | 14 stages + items + signature outcomes | FR-100; passthrough export source |
| VW-09 | TrustView / ReadinessView | adopted keys + per-role readiness | FR-103/104; **no secrets** |
| VW-10 | ExchangeView | bundle reports, sync checklist, pending claims | FR-090–094 |
| VW-11 | RecoveryView | doctor findings, lock inspector rows | FR-101/102 |

**INV-8 — View-models are write-through-nothing.** A view-model may trigger a prikk mutation (via the operation layer), but the view-model itself is never the record of that mutation's result; on completion the affected view-models are re-sourced from prikk (OP-02, FR-106).

---

## 4. Lifecycles (LC-…)

### 4.1 Config lifecycle

- **LC-1 — Load.** At launch: read DM-01 → validate → on syntax error, fall back to full defaults with a visible notice and a pointer to `stikk config check` (OP-01, CF-02). Unknown keys warn (named) and are ignored, never block launch, never dropped (INV-4).
- **LC-2 — Change.** Theme/locale/advanced-mode apply immediately; other changes apply on restart, stated in the settings surface (CF-05). A stikk-initiated write of DM-01 is atomic and preserves unknown keys and comments where the format allows.

### 4.2 Session lifecycle

- **LC-3 — Open repository → SessionState.** Resolve DM-02 (canonical path); locate DM-04 for that path; re-validate every stored reference against prikk (focused ref exists? filter targets resolvable?) — misses degrade to defaults (INV-5). Compute VW-01 within the NFR-P03 budget with **no implicit full verify** (NFR-P05).
- **LC-4 — Repository-change token.** On open, and on each refresh, stikk obtains a cheap change indicator from prikk's observable state (e.g., ref-pointer/index/WAL extents — the internal design binds the exact signals within CON-1) and stamps it as the current token. Any view or cache computed under an older token is stale: reads refresh passively with a notice (OP-04); armed mutation previews invalidate and their execute action disables until regenerated (CT-05, FR-120).
- **LC-5 — Flush.** SessionState and UIPreferencesRuntime flush atomically on every meaningful transition (view change, focus change, filter change), not only at exit, so `kill -9` loses at most the last transition and never repository state (OP-05, NFR-R01).
- **LC-6 — Focused-ref reconciliation.** The stored focused ref (DM-04) is re-checked against prikk on every load and after every external-change refresh; if the ref was closed or removed, stikk falls back to a present ref and notes it — it never acts on a focused ref prikk no longer has (FR-055, INV-5).
- **LC-7 — Close.** No locks are held between operations, so close holds none (NFR-R02); final flush (LC-5); running operations prompt once (OP-05).
- **LC-8 — Private/ephemeral session.** A user may open a repository in a mode that persists no SessionState, RefusalHistory, or Recents entry for that session (rules' GUI-notes "private mode not to store at current session"). Caches under this mode are memory-only and dropped on close. This is a lifecycle *mode*, not a separate entity.

### 4.3 Identity & cache lifecycle

- **LC-9 — Repository fingerprint: deferred (RFC 003 decision 4).** `DM-02` keys session state by **canonical path** for now, not a content-derived fingerprint. Three reasons, checked against prikk rather than assumed: prikk deliberately has **no repository identity** — "repositories are anonymous... that is a security property this design has, not a gap in it" (`trust-threat-model.md`) — so deriving one would work against prikk's own design intent, not merely be extra effort; the cheapest honest derivation (the genesis/root block) requires walking the entire sealed history, since `prikk log` has no oldest-first or "give me the root" query; and it would be `None` for exactly the repositories a user is most likely to be creating — one with no sealed blocks has no root block at all. **It is not load-bearing either way**: the "same path, different repository" risk it was meant to catch is already covered by `INV-5` (every stored reference is re-resolved against prikk on load; a miss degrades to a default) — a fingerprint would have been defence in depth behind a control that already exists, not the control itself. Revisit only if a future need cannot be met by path-keying plus `INV-5`.
- **LC-10 — Cache validity.** A DerivedViewCache entry (DM-08) is usable only if its key matches **and** its stamped change token equals the current token (LC-4) **and** the prikk version is unchanged; otherwise discard and re-derive (INV-6). Caches are size-bounded with LRU eviction; eviction never affects correctness.
- **LC-11 — Version skew.** On open, stikk records prikk's version; outside the validated range it enters read-only degradation (NFR-R03, OP-06) and invalidates all caches (their derivations may no longer match).

### 4.4 Export lifecycle

- **LC-12 — Export.** Explicit user action → derive live (never from a stale cache without a fresh token) → write to a temp path → atomically rename to the user's chosen final path on completion (OP-05). A stikk-authored export is stamped per INV-7. prikk's JSON is passed byte-verbatim (no stikk reserialization) so it remains valid against prikk's own `verify-report-v1` schema (CT-02).

### 4.5 Secret-material lifecycle (the critical one)

- **LC-13 — Key material never enters stikk's lifecycle.** stikk observes `PRIKK_*_SEED` **presence** to compute readiness and then forgets it; it never copies, derives from, persists, logs, or displays seed bytes (DM-N1, NFR-S03). When a mutation needs signing, prikk reads the environment itself — stikk hands prikk no key material because it holds none. Recognized example/tutorial seeds are flagged unsafe by *pattern of the public inputs*, never by storing the seed (FR-104). This is a threat-model anchor (see `stikk-03`, asset A-KEY).

---

## 5. Entity relationships

```
Config (DM-01, one, user-authored)
  └─ defaults & overrides consulted by every session

RepositoryHandle (DM-02) ──1:N── SessionState (DM-04)          [keyed by canonical path — LC-9]
        │                          ├─ focused ref name (re-resolved: LC-6)
        │                          ├─ filters (re-applied)
        │                          └─ id-only anchors (re-resolved: INV-5)
        ├──1:N── SyncAssistantProgress (DM-05)  [artifact PATHS only]
        ├──1:N── DerivedViewCache (DM-08)       [token-guarded: LC-10]
        └──1:N── RefusalHistory (DM-06)         [unless private: LC-8]

RecentRepositories (DM-03) ──N:1── RepositoryHandle

ReportExport (DM-10)  ── produced from ── a live VW-08/VW-07/refusal   [stamped: INV-7]

GlossaryContent (DM-09, product asset) ── bound by ── VW-07/VW-08/refusal overlays

[prikk world — NOT stikk entities, shown for the boundary]
  Repository objects: Patch, Block, RefState/RefUpdate, Tag, Blob,
  MaintainerTrustPolicy, WAL queue    ── read live via prikk; never persisted by stikk
```

**INV-9 — Every stikk→repository edge is a re-resolvable reference (id or name), never an embedded copy.** Cutting every stikk-owned store leaves every repository byte-identical and every future session correct after re-derivation. This is the machine-checkable statement of CON-1/CON-4/NFR-S02.

---

## 6. Design-rule conformance (self-check)

- **Long-term safety:** the two-world split (INV-1) and the negative model (DM-N1…N4) mean stikk cannot corrupt a repository or leak a secret through its own data, structurally.
- **Maintainability:** ten owned entities, each single-purpose; view-models named once; one identity key (canonical path — the fingerprint originally proposed here was deferred, LC-9) threads sessions, caches, and exports.
- **Simplicity vs flexibility (the rules' explicit balance):** the model is not over-abstracted (no generic "object store" mirroring prikk — that would invite treating stikk data as authority) and not rigidly feature-coupled (SessionState carries predicate *values*, not view-specific widget state, so a new view reuses it). The one deliberate generality is the change-token guard (LC-4), which every derived/cached datum shares rather than each inventing its own staleness rule.

*End of Data Model & Lifecycle v0.1. Consumed by the Threat Model (`stikk-03`, asset inventory) and the Internal Design (`stikk-04`, store components and the operation layer that re-derives from prikk).*

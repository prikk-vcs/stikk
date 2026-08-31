# stikk — Threat Model

| | |
|---|---|
| Document | stikk Threat Model (security) |
| Version | v0.1 (draft for review) |
| Date | 2026-08-31 |
| Inputs | Requirements v0.2 (NFR-S01…S07), External Design v0.2, Data Model & Lifecycle v0.1, prikk 0.27.1 security posture (audit `audit-2026-08-31-task-1b`, prikk's own `docs/src/reference/trust-threat-model.md`, `path-safety.md`, `concurrency-locking.md`, `security-setup.md`), project rules |
| Method | STRIDE-per-asset, scoped to what a **front-end** can affect. Trust boundaries first, then assets, then threats with controls, then a residual-risk register. |
| ID scheme | `TB-` trust boundary · `A-` asset · `T-` threat · `C-` control · `RR-` residual risk. Threats tagged with STRIDE: **S**poofing **T**ampering **R**epudiation **I**nfo-disclosure **D**oS **E**levation. |
| Maintenance | Per NFR-S07 / rules §Release Deliverables: any release introducing new data flows, external integrations, or auth/signing logic updates this document; other releases verify these controls still hold. |

**Framing.** stikk is a front-end. It holds **no repository authority and no secrets** (Data Model INV-1, DM-N1). The largest security fact about stikk is therefore a *negative*: most classic VCS-tool threats (corrupting history, forging signatures, leaking keys through the tool) are **out of reach by construction**, because stikk cannot write repository bytes (only prikk can — CON-1) and never possesses key material (prikk reads the environment itself — LC-13). This document's job is to prove those negatives hold and to cover what remains: the seam to prikk, untrusted repository/bundle **content rendered as UI**, stikk's own state files, and the ways a front-end can *mislead* a user into an unsafe prikk action.

---

## 1. Trust boundaries (TB-…)

```
        ┌──────────── the user (trusted operator) ────────────┐
        │                                                      │
   [TB-1 UI surface]                                     [TB-4 environment]
        │  keystrokes, clicks, drops                     PRIKK_* vars, config file,
        ▼                                                OS locale/theme
  ┌───────────── stikk process ─────────────┐
  │  frontends │ operation layer │ view      │
  │  ──────────────────────────────────────  │
  │  stikk-state files   [TB-3 state store]   │◀── user-scope disk (attacker-of-account)
  │           │                               │
  │      [TB-2 the prikk seam]                │
  └───────────┼───────────────────────────────┘
              ▼
        prikk core  ──────────────  .prikk/ + worktree   [prikk's own trust boundary]
              ▲
        [TB-5 untrusted content]  bundles, received refs, artifact files
              (authored by other parties, arriving as files)
```

| ID | Boundary | What crosses | Trust change |
|---|---|---|---|
| **TB-1** | User → stikk UI | intents, file selections, drag payloads | trusted input, but the *targets* it names may be untrusted content (TB-5) |
| **TB-2** | stikk → prikk (the seam) | typed requests / responses; verbatim messages | stikk trusts prikk as the authority; prikk does not trust stikk with keys or storage |
| **TB-3** | stikk ↔ its own state files | config, sessions, caches, exports | same-user disk; an attacker with the user's account already owns everything (see assumptions) |
| **TB-4** | Environment → stikk | `PRIKK_*` presence, config, OS settings | `PRIKK_*_SEED` values are secrets stikk must never read/store (LC-13) |
| **TB-5** | Other parties → repository | bundles, received refs, sync artifacts | **untrusted**; prikk keeps them read-only until adopted; stikk must render them as inert data |

**Assumptions (inherited from prikk's threat model and stated for stikk).**
- **ASSUME-1 — Local same-user trust.** The user's account is the trust base (`trust-threat-model.md:123-148`: "Repositories are anonymous. Identity lives in signer keys and in patch ids — never in a repository."). An attacker who already has the user's local privileges can read the environment, the config, and the worktree directly; defending stikk against that attacker is out of scope, exactly as it is for prikk and for Git. stikk's job is not to *add* exposure beyond what the user already has.
- **ASSUME-2 — prikk is correct within its *stated* posture — including where that posture has stated gaps.** stikk relies on prikk's audited guarantees (TOFU continuity, fail-closed decoders with input ceilings, `verify_strict` signatures, crash-safe mutation, no lock auto-steal) but must not present them as stronger than prikk itself claims. Three prikk-stated limits stikk must not paper over:
  - **Worktree containment is check-then-write, not race-free.** prikk's own `path-safety.md:37-38,117-120`: "not an `openat`/`O_NOFOLLOW` design, not a canonical realpath proof… A concurrent process that mutates the worktree between checks and writes is outside the current guarantee." (stikk's lower-level fsutil primitives are openat-anchored, but the *materialization* containment check is lexical — the weaker claim is the one stikk must honour.)
  - **`import_bundle` object writes are not lock-protected** (`concurrency-locking.md:186-198`: "Known and accepted, not fixed here."). A front-end must not run a bundle import concurrently with other mutation and imply it is safe.
  - **The gated-operation set is not provably complete** (`trust-threat-model.md:89-94`: "no enumeration of gated operations can catch an operation that is absent from it… a standing, unenumerable risk"). stikk must not assert "this operation is trust-gated" beyond what prikk actually gates → RR-7.
- **ASSUME-3 — The exchange channel is the user's responsibility.** prikk moves no bytes and does not encrypt them (`sync.md:56-62`: a `sync build` file "contains repository content **in the clear**… never secrecy. The channel that moves the file is the operator's choice and the operator's responsibility."). stikk neither adds nor can add transport security (NFR-S05).

---

## 2. Assets (A-…)

| ID | Asset | Owner | Why it matters | Worst case if harmed |
|---|---|---|---|---|
| **A-KEY** | AUTHOR/MAINTAINER seed material (`PRIKK_*_SEED`) | user's environment | signing authority for all history | forged patches/seals under the user's identity |
| **A-HIST** | Repository history integrity (patches, blocks, refs) | prikk | the product's whole value | corrupted/rewritten history, false lineage |
| **A-TRUST** | The local maintainer trust policy (TOFU) | prikk | decides whose sealed history is "Sound" | trusting an attacker's key → laundering hostile history as sound |
| **A-UND** | The user's *understanding* of repository state | the user, mediated by stikk | every safe action depends on a correct mental model | user authorizes a destructive/wrong prikk action believing it is safe |
| **A-STATE** | stikk's own state (sessions, recents, refusal log) | stikk | convenience + a small privacy surface (what/where you work) | disclosure of activity; misdirection if trusted as authority |
| **A-AVAIL** | stikk's availability/responsiveness | stikk | a hung tool blocks work and can mask real state | denial of the tool; user acts blind |

**A-UND is the asset unique to a front-end.** stikk cannot forge a signature or corrupt a block — but it *can* render prikk's truth misleadingly and thereby induce the user to take a real, prikk-executed action they would not have chosen. Much of this model is about protecting A-UND.

---

## 3. Threats & controls (T-…, C-…)

### 3.1 The seam (TB-2)

- **T-S1 (Spoofing/Tampering) — a wrong or hostile `prikk` binary.** If stikk invokes a `prikk` on `PATH` that is not the real one, every response is attacker-controlled.
  - **C-S1a** The seam records the resolved prikk path and version at handshake (SEAM-05) and surfaces them in Orientation/About; a version outside the validated range degrades to read-only (NFR-R03, OP-06).
  - **C-S1b** stikk does not *elevate* this risk beyond the user's own shell: it invokes `prikk` the same way the user would. Pinning a specific binary path is offered via config (CF) for users who want it. Residual → RR-1.
- **T-T1 (Tampering) — output-parsing confusion.** A malformed or unexpected prikk output shape causes stikk to misrender state (feeds A-UND).
  - **C-T1a** Parsing is confined to `cli_backend/parse/`, version-gated, golden-fixture-tested (SEAM-03, TS-03); an unrecognized shape returns `StikkError::Environment` and refuses to fabricate a partial result (UD-02).
  - **C-T1b** `verify --format json` is consumed as prikk's own schema and passed through byte-verbatim on export (INV-7), never reserialized.
- **T-D1 (DoS) — EPIPE / hung child.** prikk's audited EPIPE panic (UD-04) or a stuck child could hang or crash stikk.
  - **C-D1** The seam drains child stdout/stderr fully before inspecting exit (UD-04 guard) and runs the child off the UI thread with cancellation (CC-01, OP-02); a child that exceeds a bound is cancellable by the user.
- **T-E1 (Elevation) — stikk auto-retrying or bypassing a refusal.** A front-end that "helpfully" retries a refused mutation, or clears a lock to proceed, would defeat prikk's safety.
  - **C-E1** Structural: the seam never retries a mutating call (SEAM-04); the operation layer never converts a refusal into a retry (OPL-05, NFR-S04); lock clearing is Operator-only, typed-confirmed, and never auto-invoked (FR-102).

### 3.2 Key material (TB-4, A-KEY) — the highest-value asset

- **T-I1 (Info-disclosure) — leaking a seed.** The catastrophic front-end failure would be a seed reaching a config file, a session file, a cache, a log, an export, a crash dump, or the screen.
  - **C-I1a** stikk never reads a `*_SEED` **value** (the exact secret vars: `PRIKK_AUTHOR_SEED`, `PRIKK_MAINTAINER_SEED`, `security-setup.md:41-52`): `stikk-prikk/env.rs` is the only module touching those variables and reads **presence only** (LC-13, DM-N1). prikk, not stikk, reads seeds when signing (SEAM-06); prikk "does not provide local secret storage, key generation, or public-key derivation" (`trust-threat-model.md:150-157`), and stikk adds none.
  - **C-I1b** No stikk-owned datum has a field capable of holding key material (Data Model §2.5); the UI shows key **ids** (public) only (NFR-S03).
  - **C-I1c** Enforced by test: a build-time check greps the seam for any `*_SEED` value read (TS-04); an export is proven stamped-or-verbatim, never carrying environment (INV-7).
  - **C-I1d** Crash containment: stikk-internal fault screens and logs never dump the environment (ER-04); the process does not write core dumps of secrets because it holds none.
- **T-I3 (Info-disclosure) — leaking sensitive *content* through diagnostics/exports/logs.** prikk states a redaction rule for its own diagnostics (`trust-threat-model.md:210-211`): avoid "raw text spans, replacement text, blob bytes, absolute host paths, `.prikk` private paths, signer secrets, key material, and arbitrary object debug dumps." A front-end that logs or exports freely could re-leak exactly this.
  - **C-I3** stikk inherits the redaction rule (NFR-S03 extended): its own logs and stikk-authored exports (`stikk-export-v1`) never contain blob bytes, raw span/replacement text beyond what the user is actively viewing, absolute host paths, or `.prikk` private paths; where such content is shown in a view it is inert (C-T2a) and is not copied into a durable log. prikk's `verify --format json` passthrough is the operator's explicit act to a chosen path (CT-02), not a background log.
- **T-S2 (Spoofing) — inducing use of an unsafe key.** Rendering the public example/tutorial seeds as if they were real signing keys would invite users to "sign" with a publicly-known key.
  - **C-S2** stikk recognizes the documented example public inputs and **flags them as unsafe** persistently (FR-104), by pattern of the public value — never by storing the secret.

### 3.3 Untrusted content rendered as UI (TB-5, A-UND) — the front-end's own surface

- **T-T2 (Tampering/Spoofing of the display) — malicious content in a bundle/received ref/patch.** A hostile party crafts patch paths, tag messages, ref names, or blob content containing terminal control sequences, spoofed UI framing, homoglyph ref names, or misleading structure, aiming to corrupt the terminal, forge stikk's own chrome, or mislead the user about what they are about to adopt/merge (A-UND).
  - **C-T2a** All repository/bundle content is rendered **inert** (NFR-S06): control characters are escaped/stripped before display in the TUI (no raw passthrough of patch/tag/ref bytes to the terminal), and the GUI renders them as text, never as markup or links.
  - **C-T2b** stikk chrome is visually distinct from rendered content (a content pane cannot forge a confirmation dialog or a capability badge); confirmations restate the *operation and target ids* (TU-09), which are prikk-authoritative values, not attacker-supplied display strings.
  - **C-T2c** Received/untrusted refs are always labelled as such (FR-014) and are read-only until an explicit Operator adoption (TB-5); prikk guarantees import "never touches `refs/by-id/`, never advances a local ref, and never adopts a MAINTAINER key" (`bundle.rs:4-8,382-388`) and that offline verify checks structure only — "no signature is cryptographically checked… A verified bundle is not yet a *trusted* one" (`bundle.rs:482-490`). stikk shows the sender's signature standing and prikk's caveats **verbatim** (FL-13, FL-11), so the named adversary — "an attacker who re-signs a Patch with their own key and ships that key in the bundle produces a bundle that verifies perfectly" (`trust-threat-model.md:69-71`) — cannot have their bundle mis-presented as trusted. Adoption is the receiver's own signed act, and "does not verify who the sender is" (`trust-threat-model.md:111-115`); stikk's adopt confirmation states exactly that.
  - **C-T2c′** The three-valued author-signature outcome is rendered precisely (FR-035): **Sound** = verifies against recorded key material; **Unverifiable** = no key material recorded — *non-blocking, must never render as a pass/green state* (`verify.rs:361-369`: "Not a failure: `verify` still passes, but this must be visible, not silent"); a genuine verification **failure blocks and is surfaced as a refusal** (it is a propagated `Err`, not a value — `verify.rs:350-355`). stikk must not collapse Unverifiable into either Sound or failure, and must state TOFU's meaning at the point it matters: "the same `key_id` has always signed under this name here — not that this author's claimed identity is genuine" (`trust-threat-model.md:57-64`).
  - **C-T2d** Homoglyph/case-collision ref names: stikk relies on prikk's own case-collision refusal at creation (audit: DC-72) and additionally surfaces the raw bytes of any ref/key id on focus (TU-11), so a visually-confusing name can be inspected.
- **T-D2 (DoS) — resource exhaustion via oversized content.** A bundle or history with a pathological diff, a huge tree, or a deep structure could hang or OOM the front-end even though prikk's own ceilings bound what prikk will process.
  - **C-D2a** stikk knows and surfaces prikk's input ceilings before an operation runs and never asks prikk to exceed them silently (FR-092). The concrete defaults it displays (all CLI-boundary-only, never persisted, fail-closed on a malformed override): bundle **100,000** objects / **256 MiB** (`PRIKK_BUNDLE_MAX_OBJECTS`/`_MAX_BYTES`); exchange **100,000** each of five counts / **256 MiB** (`PRIKK_EXCHANGE_MAX_OBJECTS`/`_MAX_BYTES`); sync summary **100,000** refs / **16 MiB** (`PRIKK_SYNC_SUMMARY_MAX_REFS`/`_MAX_BYTES`); active-patch warn **800** / hard limit **1000** (`PRIKK_ACTIVE_PATCH_WARN`/`_LIMIT`). stikk surfaces the *limit* it is about to hit, not just the failure after.
  - **C-D2b** Oversized *renderables* get a size summary with an explicit "show anyway" (size_guard, TU-11) rather than an unbounded render; expensive derivations run off-thread, cancellable, and are cache-bounded (LC-10) — the front-end never blocks input (NFR-P01).
- **T-R1 (Repudiation) — misattributed history.** prikk has no clocks and does not yet persist commit messages/author names (audit UD-01); a front-end could paper over this by fabricating times/authors, creating false attribution.
  - **C-R1** stikk **never fabricates** time, author name, or message (OS-7, UD-01 behaviour): it shows key ids and lineage order only, and states plainly that messages are not yet persisted. Refusal history and exports are stamped with repository fingerprint + prikk version + capture time as *stikk's* record, clearly not repository authority (INV-7).

### 3.4 stikk's own state (TB-3, A-STATE)

- **T-I2 (Info-disclosure) — activity leakage.** Sessions, recents, and refusal logs reveal which repositories the user works in and what failed. On a shared or backed-up machine this is a minor privacy surface.
  - **C-I2a** State lives in user-scope locations with the platform's default user-only permissions (paths.rs); stikk stores no secrets there (DM-N1) and no repository content as authority (DM-N2).
  - **C-I2b** A **private/ephemeral session** (LC-8, from the rules' GUI notes) persists no session/recents/refusal state — the user's control for sensitive work.
- **T-T3 (Tampering) — poisoned state file.** An attacker (or a corruption) alters a stikk state file to point the user at a wrong repository or replay stale filters.
  - **C-T3a** State is never authority (INV-1): every stored reference is re-resolved against prikk on load (INV-5); a focused ref prikk no longer has is discarded (LC-6); a fingerprint mismatch discards stale session/cache for a moved-or-different repository (LC-9/10). A poisoned state file thus cannot make stikk act on a false repository fact — at worst it opens the wrong path, which the user sees.
  - **C-T3b** A corrupt state file resets to defaults with a notice (NFR-R01), never a crash.
- **T-E2 (Elevation) — writing into a repository via state paths.** A bug or crafted config that resolved a stikk state path *inside* `.prikk/` or the worktree would violate the boundary. **prikk would not catch it:** there is no general foreign-file scan of `.prikk/` — `verify` inspects only `refs/tmp/` (a non-blocking debris finding) and the retired loose-object tree; a stikk file written into `containers/`, `active/`, `trust/`, `cache/`, or `.prikk/` root would go **undetected** by `verify` (agent-confirmed against `refs/verify.rs`, `verify/objects.rs`). The "never write foreign files there" rule therefore rests on prikk's *design invariant* — "No name under `.prikk/` is created after `init`" (`repository-layout.md:77-79`) — and on **stikk's own control**, not on detection.
  - **C-E2** `stikk-state::paths` refuses any repository-internal path — checked before every state/cache/export write (INV-2), enforced by test (TS-04); the config's per-repository override cannot relocate state into a repository or worktree. Because prikk provides no backstop here, this control is *primary*, not defence-in-depth: it is the only thing preventing the boundary violation. A stray foreign file in the worktree would additionally be swept into the next `commit` (prikk has no ignore mechanism — audit UD-08), a second reason the refusal must hold.

### 3.5 Misleading the user into an unsafe prikk action (A-UND, cross-cutting)

- **T-T4 — the front-end's signature risk: a confident-but-wrong picture.** Every mutating prikk action stikk offers is gated on the user's understanding. If stikk shows a merge as confluent when prikk would refuse, a checkout as safe when it will overwrite, or a received ref as trusted when it is not, the user authorizes real harm through a correct tool.
  - **C-T4a Preview-first, always** (FR-120, OPL-01): stikk never offers a mutation without prikk's own plan/preview where one exists; the *user confirms prikk's plan*, not stikk's summary of it.
  - **C-T4b Preview↔execute binding** (OPL-02): a preview computed under an old change-token cannot be executed after external change; the user re-previews (defeats "the world moved under the preview").
  - **C-T4c Verbatim truth** (ER-02, NFR-I03): prikk's refusal/warning text is shown unmodified; stikk's gloss is additive, never a replacement, so stikk cannot soften a refusal into an apparent success.
  - **C-T4d Capability honesty** (OP-06): unavailable actions are disabled-with-reason, never hidden and never silently no-op — the user always knows why they cannot do a thing.
  - **C-T4e Confirmation restates authoritative values** (TU-09): ids and counts come from prikk, so attacker-supplied display content cannot dress up a different operation than the one that will run.

---

## 4. STRIDE coverage matrix

| Asset ↓ / STRIDE → | S | T | R | I | D | E |
|---|---|---|---|---|---|---|
| A-KEY | T-S2 / C-S2 | — (stikk can't tamper keys) | — | **T-I1 / C-I1a–d** | — | — |
| A-HIST | C-E1 (no bypass) | prikk-owned; C-E1/C-T4 | T-R1 / C-R1 | — | — | T-E1 / C-E1 |
| A-TRUST | T-T2c (adoption clarity) | C-T2c | — | — | — | C-E1 (no auto-adopt) |
| A-UND | T-T4a | **T-T2 / C-T2**, **T-T4 / C-T4** | T-R1 / C-R1 | — | T-D2 / C-D2 | — |
| A-STATE | — | T-T3 / C-T3 | — | T-I2 / C-I2 | — | T-E2 / C-E2 |
| A-AVAIL | — | — | — | — | **T-D1 / C-D1, T-D2 / C-D2** | — |
| Seam | T-S1 / C-S1 | T-T1 / C-T1 | — | — | T-D1 / C-D1 | T-E1 / C-E1 |

Cells that are blank or "prikk-owned" are threats a front-end structurally cannot realize (it holds no keys, writes no history) or that belong to prikk's own threat model (which stikk assumes, ASSUME-2). The concentration is exactly where a front-end's real surface is: **A-UND (misleading the user) and A-KEY info-disclosure.**

---

## 5. Residual risks (RR-…)

| ID | Residual risk | Why it remains | Disposition |
|---|---|---|---|
| **RR-1** | A hostile `prikk` on `PATH` | stikk deliberately invokes prikk as the user's shell would; pinning is opt-in | Accepted (ASSUME-1); surfaced via version/path display + optional pinning (C-S1) |
| **RR-2** | Local same-user attacker reads env/config/worktree | stikk cannot exceed the OS trust model | Accepted (ASSUME-1); stikk adds no exposure (DM-N1, C-I2) |
| **RR-3** | Exchange-channel confidentiality/authenticity | prikk moves no bytes; the channel is the user's | Accepted (ASSUME-3); stikk shows the "structure ≠ trust" caveat (C-T2c) |
| **RR-4** | prikk-core defects stikk inherits (e.g. the audited `worktree-status` break, exit-code coarseness) | stikk depends on prikk's correctness | Mitigated by the seam's version gate + degraded routes (UD-03/05); tracked as upstream dependencies |
| **RR-5** | A brand-new prikk finding/witness code with no glossary entry | glossary is a product asset lagging prikk releases | Mitigated: degrade to prikk's verbatim message, never hide it (C-T1/NFR-I03); glossary updated per release (NFR-S07) |
| **RR-6** | User fatigue defeating confirmations | tiered confirmations can be click-through | Mitigated by tiering (only tier-3 uses typed confirmation) and progressive disclosure keeping the common path simple (NFR-U02); not fully eliminable |
| **RR-7** | An operation that *should* be trust-gated but is not (prikk's unenumerable gating gap, `trust-threat-model.md:89-94`) | prikk cannot prove its gated set complete; `tag create` shipped ungated for months | Inherited from prikk (ASSUME-2); stikk never *claims* an operation is gated beyond what prikk enforces, and shows the maintainer key id that actually sealed each object (FR-035) so a user can judge provenance directly rather than trusting a "gated" label |
| **RR-8** | Repo-path case-collision surfaces at `seal`, not at `commit` (`path-safety.md:28-32`); NFC/NFD ref-name collisions are un-rejected | prikk's stated timing/validation gap | stikk surfaces the seal-time refusal with its FL-06/TU-08 explanation and, where it can, warns at commit time that a colliding path was queued; ref-name NFC/NFD collisions are shown by raw-byte inspection on focus (C-T2d) |

---

## 6. Control-to-requirement traceability

| Control theme | Controls | Backing |
|---|---|---|
| No key material in stikk | C-I1a–d, C-S2, SEAM-06 | NFR-S03, LC-13, DM-N1, TS-04 |
| Diagnostic/export redaction | C-I3 | NFR-S03, `trust-threat-model.md:210-211` |
| No repository authority in stikk | C-T3a, C-E2, INV-1/2/5 | CON-1, CON-4, NFR-S02 |
| No bypass of prikk safety | C-E1, C-T4a–e | NFR-S04, FR-120/121, OPL-01/02/05 |
| Untrusted content inert | C-T2a–d, C-D2b | NFR-S06, FR-014, TU-11, size_guard |
| Verbatim truth to the user | C-T1b, C-T4c, C-R1 | NFR-I03, FR-110, OS-7, INV-7 |
| Availability under hostile input | C-D1, C-D2a/b | NFR-P01/P02, FR-092 |
| Privacy of activity | C-I2a/b | LC-8, CF-01 |
| Threat-model upkeep | this document | NFR-S07, rules §Release Deliverables |

---

## 7. prikk-side source citations

Ground-truth anchors for the facts this model rests on (prikk 0.27.1 working tree; cross-checked against the 2026-08-31 audit):

- **Trust boundaries / actors / non-goals:** `docs/src/reference/trust-threat-model.md` — anonymity of repositories `:123-148`; local-store sole authority `:140-141,159-164`; threat boundaries + diagnostic redaction rule `:205-211`; explicitly-not-defended `:213-223`; AUTHOR key pinning permanence `:19-22`; unenumerable gating gap `:89-94`; named bundle adversary `:69-71`; TOFU-is-continuity-not-identity `:57-64`.
- **Path/worktree safety:** `docs/src/reference/path-safety.md` — accepted/rejected shapes `:49-79`; the `.prikk`-first-component-only limit `:85`; check-then-write, not race-free `:37-38,117-120`; case-collision surfaces at seal `:28-32`.
- **Locks:** `docs/src/reference/concurrency-locking.md` — no auto-steal + rationale `:22-24,207-212`; asymmetric liveness `:213-223`; `LockConflict` ≠ CAS mismatch `:109-120,229-230`; unlocked bundle-import object writes `:175-204`.
- **Key supply:** `docs/src/guide/security-setup.md` — env var names `:41-52`; public-example-seeds-are-compromised + never-commit warnings `:106-114`; deferred key management `:134-141`. (Note: this page is stale on AUTHOR trust policy vs. `trust-threat-model.md:52-54` + `verify.rs` — cite the trust-threat-model, not this page, for AUTHOR trust.)
- **Signature outcomes:** `crates/prikk-store/src/verify.rs:350-369` (`Sound`/`Unverifiable` values, failure is `Err`); non-gating siblings `tag_travel.rs:37-57`, `recognition_claim.rs:113-136`.
- **Bundle/received/sync:** `crates/prikk-store/src/bundle.rs:4-10,382-388` (import never advances a ref/trusts a key), `:482-501` (offline verify = structure not trust); `docs/src/guide/sync.md:56-93` (in-the-clear, untrusted-input, receiver-signs-under-own-key); `docs/src/reference/data-model.md:138-152`, `data-model-lifecycle.md:161-171` (received refs read-only).
- **Ceilings:** `bundle.rs:105,111`; `patch_exchange/artifact.rs:58,63`; `sync_negotiation/summary.rs:33,36`; `worktree_patch.rs:79`; `crates/prikk-cli/src/main.rs:355`.
- **Layout / no foreign-file scan:** `docs/src/reference/repository-layout.md:45-79` (tree + "no name created after `init`"); confirmed absence of a general foreign-file scan via `refs/verify.rs:277-304` and `verify/objects.rs:346-353`.

*End of Threat Model v0.1. This document is a release deliverable per NFR-S07: revisit whenever the prikk seam, a data flow, or the signing/trust surface changes. Companion documents: `stikk-04` (Internal Design — where these controls are implemented), `stikk-05` (Data Model — the asset/negative-model source).*

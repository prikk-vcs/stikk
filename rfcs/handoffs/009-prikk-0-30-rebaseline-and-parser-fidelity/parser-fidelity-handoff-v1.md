# Handoff — prikk 0.30 re-baseline and parser fidelity (v1)

**Companion to:** [RFC 009](../../accepted/009-prikk-0-30-rebaseline-and-parser-fidelity.md) (Accepted 2026-09-04). Inherits its state.
**Realizes:** the corrective increment ahead of roadmap increment 6 — a **0.1.1 patch release**
candidate. Fixes four parser-fidelity defects (RFC 009 F1–F4), retires `UD-08` (F5), and revises
`UD-05` (F6).
**Design items:** `FR-002`/`VW-01` (Orientation), `FR-014` (ref listing), `FR-034` (worktree changes),
`SEAM-03`/`UD-02` (confined, refusing parsers), `UD-05` (exit-code classification), `UD-08` (retired),
`ER-02`/`C-T4c` (verbatim truth), `T-T4`/`A-UND` (no confident-but-wrong picture), `NFR-R03`/`ASM-2`
(version honesty), `TS-03` (golden fixtures).

This is the program design and decision record for the increment. **Implementation, tests, and the
example follow it.** Where this handoff and RFC 009 or the design set disagree, the RFC/design wins and
this handoff is corrected first.

> **Read RFC 009 first, especially F1.** The headline is not "prikk moved". It is that shipped stikk
> cannot open a repository anyone has committed to, at *any* prikk version it claims to support, and
> that the test which should have caught it passes because its fixture was **written rather than
> captured**. That is the defect class this increment exists to close — the code fix is the smaller
> half.

---

## 0. The rule this increment establishes

**A golden fixture is captured from a real `prikk` run, never composed by hand or by analogy.** Every
fixture constant carries a provenance line naming the command and the prikk version it came from:

```rust
// Captured verbatim from `prikk status` on a repository with one queued patch, prikk 0.30.0.
```

A fixture whose provenance line cannot be reproduced by running that command is a defect. Three of
stikk's existing fixtures fail this rule today (RFC 009 F1, F3, and the `branch list` fixture in §2.3),
and one of them **carries a false provenance comment** claiming it was captured when it was not. Fix
the fixtures first; the parser changes then follow from real data rather than from this document's
prose.

---

## 1. Scope

**In:**
- **Four parser corrections** in `stikk-prikk/src/cli_backend/parse.rs` — the `queued patches:` tail
  (F1), the sentinel set (F2), `refs`' empty-and-unrecognized handling plus the tag gap (F3), and the
  `worktree-status` `queued_elsewhere` note (F4).
- **Re-capture of every golden fixture** against prikk 0.30.0, with provenance lines (§0).
- **Carrying prikk's `queued_elsewhere` warning verbatim** into `ChangesView` and the Changes view,
  and **suppressing the contradicting UD-08 banner** while it is present (RFC 009 decision 3).
- **Exit-code 2 → `StikkError::Internal`** in the seam (F6), so a stikk argument bug never wears
  prikk's voice.
- **`UD-08` retirement** in the design set and in Changes copy, plus a glossary entry and a usable
  next-step for the malformed-`.prikkignore` refusal (F5).
- **Orientation shows the queue's target ref** now that F1's fix parses it (RFC 009 open question,
  ruled yes).
- **The version floor and ceiling** (RFC 009 decisions 6 and 7, both ruled by the owner 2026-09-04):
  raise the floor to prikk **≥ 0.28**, declare the range validated through **0.30.0**, and add the soft
  ceiling that says so for anything newer. See §2.6.

**Out (do not build here):**
- **Moving seam calls off the UI thread** — that is RFC 010, proposed separately. Do not restructure
  the run loop here.
- *(No longer out of scope: the version floor was carved out for the owner and has since been ruled —
  see §2.6.)*
- **A richer `.prikkignore` surface** (showing rules, offering to add one) — later increment. And
  **never** a count of ignored paths: prikk does not report one, so stikk cannot show one (T-T4).
- **Calling `prikk tag list`** to complete the ref picker — see §2.3; record the gap, do not build the
  second seam read in a patch release.
- Any mutation, any new seam category, any change to the seam trait's shape.

---

## 2. The seam corrections (`stikk-prikk`)

### 2.1 F1 — `queued patches: <n> targeting <ref>`

**Captured verbatim from `prikk status` on a repository with one queued patch, prikk 0.30.0:**

```text
prikk repository: /tmp/probe/.prikk
active WAL records: 1
trailing partial WAL bytes: 0
heads/main RefState: <not published>
queued patches: 1 targeting heads/main
status: multi-operation text diff minimization and plugins not yet implemented
```

**Captured verbatim from `prikk status` on a freshly-`init`ed repository, prikk 0.30.0:**

```text
prikk repository: /tmp/empty/.prikk
active WAL records: 0
trailing partial WAL bytes: 0
heads/main RefState: <not published>
queued patches: 0
status: multi-operation text diff minimization and plugins not yet implemented
```

prikk emits the bare form only when the queue is empty (`prikk-cli/src/main.rs`: `queued patches: 0`,
else `queued patches: {n} targeting {target}`). Parse a **leading integer** followed by an optional
` targeting <ref>` tail, and **keep the tail** — add `queued_target: Option<String>` to `Orientation`.
A trailing shape that is neither empty nor `targeting <ref>` **refuses** (`Environment`, UD-02).

`target` itself has sentinel forms when the active-ref metadata is unreadable — prikk emits
`<missing metadata>` or `<malformed metadata>` — which must map to `None`, not to a ref name. They are
in the sentinel set of §2.2.

> prikk also appends a threshold warning line when the queue reaches
> `PRIKK_ACTIVE_PATCH_WARN`/`_LIMIT`. **Do not parse it this increment** (RFC 009 open question, ruled
> deferred to the commit increment) — but the parser must **tolerate** its presence, so anchor on the
> `queued patches:` line rather than on the file ending where you expect it to.

### 2.2 F2 — sentinels are a set, and a non-id refuses

`optional_object_id` currently treats only `<none>` as absent, so `heads/main RefState: <not
published>` becomes `Some("<not published>")` — a fabricated object id. Replace it with:

- a **sentinel set** → `None`: `<none>`, `<not published>`, `<missing metadata>`, `<malformed
  metadata>`;
- an **object-id shape check** on anything else (64 lowercase hex — `stikk-model::ObjectId` already
  validates; use it rather than re-implementing);
- **anything else refuses** with `Environment` naming the field and the value. A value stikk cannot
  recognize as either an id or a known sentinel is exactly the "unrecognized shape" UD-02 is about.

This is the general fix, not a second literal. A future prikk sentinel then refuses loudly instead of
becoming a fake id.

### 2.3 F3 — `no branches`, and the tag gap

**Captured verbatim from `prikk branch list --all` on an empty repository, prikk 0.30.0:**

```text
no branches
```

`parse::refs` accepts any line with two or more tokens, so this becomes `RefEntry { name: "no", id:
"branches" }` — a phantom ref in the picker. It is the only parser anchored on nothing.

Fix: treat the exact line `no branches` (and `no tags`, for when `tag list` is added) as the **empty
list**; parse a ref line as `<name> <64-hex-id>` with an optional trailing `(closed)` / `(received)`
marker; **refuse anything else** (`Environment`). Verified against prikk's own printer
(`prikk-cli/src/branch.rs`): the three emitted shapes are exactly

```text
<name> <id>
<name> <id> (closed)      # only with --all
<name> <id> (received)
```

**The tag gap — record it, do not close it here.** The existing `BRANCH_LIST_FIXTURE` contains a
`tags/v1` line and a comment claiming it was "captured verbatim from `prikk branch list --all`
(branches, a closed branch, and a tag)". **`branch list` cannot emit a tag** — prikk lists tags through
the separate `prikk tag list`. So the fixture is invented *and* falsely labelled, and it concealed a
real gap: `RefEntry::is_tag()`, and the `Prikk::refs` doc comment ("lists every ref — branches, tags,
received"), promise tags the ref picker will never show.

For this increment: **capture a real fixture** (see below), keep `is_tag()` (it is correct for a
`tags/…` name, it just has no source yet), and **correct the doc comment** to say what `refs` actually
returns. Add a one-line note where the ref picker would show tags. Completing `FR-014` with a `tag
list` seam read is queued in §10.

**Capturing the non-empty fixture requires a sealed repository**, which needs a MAINTAINER keypair
(prikk does not derive the public key from the seed — generate a real Ed25519 pair, `trust maintainer
add` it, then `commit` + `seal` + `branch create` + `tag create`). Do that and paste the real output.
**Do not** adapt the existing fixture — it is the artifact that caused this defect.

### 2.4 F4 — the `queued_elsewhere` warning *[the serious one]*

**Captured verbatim from `prikk worktree-status --ref heads/other`, prikk 0.30.0, on a repository whose
active WAL holds a patch queued for `heads/main`:**

```text
worktree-status repository: /tmp/probe/.prikk
ref: heads/other
tracked files: 0
unchanged files: 0
missing files: 0
modified files: 0
untracked files: 2
unsupported paths: 0
worktree: changed against baseline
  untracked readme.txt — worktree file is not in the baseline
  untracked src/main.rs — worktree file is not in the baseline
note: the active WAL has queued (unsealed) patches for heads/main, not heads/other -- that is real, committed work, not shown above; any "untracked" file here may be exactly that work seen from this ref's own baseline, so do not delete based on this report alone (see `prikk status`)
note: use `prikk commit -m <message>` to author node-addressed worktree changes; text nodes use deterministic arbitrary-span EditText
```

(stderr carries `error: worktree has changes against the baseline`; exit `1` — the existing dirty-exit
rule already handles that correctly.)

The parser reads counts, the headline, and **indented** entry lines; both `note:` lines are flush-left
and are discarded. Add to `WorktreeStatus`:

```rust
/// prikk's own warning, verbatim, when the active WAL holds queued patches for a *different* ref
/// than the one asked about: paths listed "untracked" here may be committed-but-unsealed work.
/// `None` when prikk did not emit it. Never paraphrased (ER-02).
pub queued_elsewhere: Option<String>,
```

Capture the **whole note line verbatim**, from `note: ` to end of line, and carry it unchanged. Do not
reconstruct it from the ref names — prikk's wording is the deliverable (ER-02/C-T4c); stikk's job is to
transport it, not to restate it.

The second `note: use \`prikk commit …\`` line is generic and always present; ignore it (it is
guidance stikk's own UD-06 copy already covers). Distinguish the two by prefix, and let an unrecognized
`note:` line pass without refusing — notes are additive prose, not a shape contract, and refusing on a
new one would take the whole view down for a line stikk does not need.

### 2.5 F6 — exit `2` is a stikk bug, not prikk's refusal

prikk 0.28 split the exit contract: `0` success, `1` operational failure, `2` **usage error** detected
before any repository work. `CliBackend::run` classifies every non-zero exit by message text, so a `2`
surfaces as `Refusal` — prikk's semantic no — when it actually means stikk assembled a bad argument
list.

Fix in `run`/`run_capturing`: when the exit code is exactly `2`, return

```rust
StikkError::Internal { detail: /* the command stikk ran + prikk's verbatim message */ }
```

which `present()` already routes to the `FaultScreen` ("the repository was not touched; continue
read-only" — true here, since a usage error runs before any repository work). Do **not** route it
through `classify`. Keep prikk's message in the detail: it names the bad argument.

### 2.6 The version floor and the soft ceiling (owner-ruled)

`version.rs` today accepts any `0.x` with `minor >= 27` and has **no upper bound**, so every future
prikk is silently "supported". Change both ends:

- **Floor → `0.28`.** 0.27.x is dropped: its `worktree-status` is the UD-03 defect, and `changes_view`
  already refuses to run there, so the floor merely stops promising what stikk cannot serve.
- **Ceiling → `0.30` as the *validated* ceiling, not a refusal.** A prikk above it still runs — refusing
  it would break every user the day prikk ships a minor — but `Handshake` gains enough for the frontend
  to say so. Suggested shape: keep `supported: bool` meaning "at or above the floor", and add
  `validated: bool` meaning "at or below the validated ceiling". Orientation renders the newer case as
  *"validated through 0.30 — this prikk is newer; its output shapes have not been checked against
  stikk"*, text-first (`NFR-A03`).
- The confined refusing parsers remain the real guard (`UD-02`). The ceiling is an honesty signal, not a
  security control — do not gate behaviour on it beyond the notice.

**Two consequences you must handle, or a lot of tests will lie:**

1. **`NullBackend`'s default handshake is `0.27.1`** — below the new floor. Move the default to
   **`0.30.0`** so the scripted backend represents a supported prikk, and audit every test and example
   that relies on the default (`app/tests.rs`, `shell/tests.rs`, `status_bar/tests.rs`,
   `changes/tests.rs`, and the four demos all mention `0.27.1`).
2. **`changes/tests.rs`'s below-gate test currently relies on that default** ("The default NullBackend
   reports 0.27.1 — below the worktree-status fix"). It must **explicitly** script an old version via
   `with_version` instead of inheriting it, or it will silently stop testing the gate.

Also update the user-facing range statements in the same change, so docs and code ship together:
`README.md`, `docs/src/guide/getting-started.md`, and the `CHANGELOG.md` entry for 0.1.1. The design set
(`requirements.md` ASM-2/NFR-R03, `ROADMAP.md`) has already been corrected — do not re-edit it.

---

## 3. The operation layer (`stikk-core`)

- **`OrientationView`** gains `queued_target: Option<String>`, passed through from the seam. No
  computation.
- **`ChangesView`** gains `queued_elsewhere: Option<String>`, passed through unchanged.
- `changes_view` keeps its `≥ 0.28` version gate exactly as RFC 008 specified. **Do not** cache the
  handshake here — that is RFC 010's decision 5, and doing it piecemeal would leave two mechanisms.
- Nothing else in the operation layer changes. It still computes nothing prikk did not report.

---

## 4. The frontend (`stikk-tui`)

### 4.1 Changes view — the warning band

When `queued_elsewhere.is_some()`, render **above the entries** a visually distinct warning band
carrying prikk's note **verbatim and inert** (`C-T2a`), in the same quoted-content style the refusal
overlay uses for prikk's own text (`C-T2b`: prikk's words are content, not stikk's chrome).

While that band is present:

- the UD-08 untracked-filter banner's **"a commit still captures them" text is suppressed** and
  replaced by a pointer to the warning. The two statements contradict each other and prikk's is the
  true one (RFC 009 decision 3). The filter itself still works; only the claim changes.
- the UD-06 whole-worktree footer stays — it remains true.

This is the acceptance-critical behaviour of the increment. A user must not be able to read stikk's
Changes view in this state and conclude that already-committed work is disposable.

### 4.2 Orientation — the queue's target

Where the queue depth renders, show the target when known: `3 queued · targeting heads/main`. Text
first, never colour-only (`NFR-A03`); the ref name goes through `inert`.

### 4.3 The malformed-`.prikkignore` refusal

**Captured verbatim, prikk 0.30.0** (stdout empty, stderr only, exit `1`):

```text
error: invalid name: .prikkignore line 1: invalid name: absolute paths are not allowed
```

The existing path already degrades this correctly to a verbatim `Refusal` (the RR-5 behaviour works —
verify it still does). What is wrong is the **next-steps**: `LoadChanges` currently offers "Choose
another ref" and "Refresh", neither of which resolves it. Add a glossary entry for `.prikkignore` and,
for the `LoadChanges` context, a next-step whose label points at the real fix (edit or remove the
malformed `.prikkignore`) as a `DismissAndResolveExternally` step — **stikk must not edit a repository
file** (`CON-1`, `INV-1`), so this is guidance, never an action.

Next-steps stay stikk-authored from `(class, operation)` and are **never** derived from the message
(`C-T2b`) — do not pattern-match `.prikkignore` out of prikk's text to decide.

---

## 5. Security surface (threat model `stikk-03`)

- **T-T4 / A-UND — the reason this increment exists.** F4 is a live instance of the project's named
  worst failure: a confident-but-wrong picture that could lead a user to delete real work. The fix is
  not cosmetic and its test (§7) is the acceptance gate.
- **ER-02 / C-T4c — verbatim truth.** prikk's warning is transported unchanged. A test asserts the
  rendered band contains prikk's note byte-for-byte (modulo `inert`), and that stikk never substitutes
  its own paraphrase.
- **C-T2a — inert.** The new warning band and the queue target ref are prikk-sourced and go through
  `inert`. Test with a hostile ref name.
- **UD-02 — refuse, never guess.** F1–F3 all *widen* what refuses. Every new refusal path gets a test;
  no fix may be implemented by making a parser more permissive.
- **No new asset, boundary, or data flow.** Per `NFR-S07` this increment does not require a threat-model
  document update — but it **does** require a threat-model note that `C-T4c`/`T-T4` had a live
  violation and how it was closed. Add it to the threat model's residual-risk discussion rather than
  leaving the document implying the control always held.

---

## 6. Decision notes (program-level; RFC 009 has the rationale)

1. **Fixtures are captured, with provenance.** The rule, not the individual fixes, is the durable
   output of this increment (§0).
2. **Keep the `targeting` tail rather than discarding it.** It is the same fact F4's warning turns on,
   and Orientation is more honest for showing it. Discarding a field prikk gives us and then warning
   about our own ignorance would be the wrong trade.
3. **Sentinels as a set with an id-shape check**, not a second literal — so the *next* sentinel refuses
   instead of becoming a fake id.
4. **Notes are prose, not shape.** Refuse on a malformed *count* or a missing *headline*; never refuse
   on an unrecognized `note:` line. Getting this backwards would make every future prikk note a
   user-visible outage.
5. **prikk's warning is transported, not restated.** stikk's paraphrase would be a second source of
   truth for a safety-critical claim, and ER-02 exists to prevent exactly that.
6. **Exit 2 is ours, not prikk's.** Presenting a stikk argument bug as prikk's refusal teaches the user
   to distrust prikk for stikk's mistake.

---

## 7. Test plan

- **F1 (TS-03):** the two captured `status` fixtures parse — `0` bare, and `1 targeting heads/main`
  yielding `queued_patches == 1` and `queued_target == Some("heads/main")`; a threshold-warning line
  appended does **not** break the parse; a malformed tail (`queued patches: 1 wat heads/main`)
  **refuses**.
- **F2:** each sentinel maps to `None`; a real 64-hex id parses; a non-id, non-sentinel value
  **refuses**.
- **F3:** `no branches` yields an **empty list, not a phantom ref** (this is the regression test);
  the newly-captured non-empty fixture parses with `(closed)`/`(received)` markers; an unrecognized
  line refuses. Assert explicitly that **no `RefEntry` named `no` is ever produced**.
- **F4 (the acceptance-critical test):** the captured `queued_elsewhere` fixture parses with the note
  **byte-identical** to prikk's; the Changes view (`TestBackend`) renders it; and — the one that
  matters — **with `hide_untracked` set and `queued_elsewhere` present, the string "a commit still
  captures them" does not appear anywhere in the rendered buffer.**
- **F6:** a scripted exit-2 invocation yields `StikkError::Internal` (→ `FaultScreen`), not `Refusal`.
  Cover it in `cli_backend/tests.rs` with the existing `sh`-based harness.
- **Hostile input (C-T2a):** a ref name and a note carrying control sequences render inert.
- **Regression guard for the fixture rule:** a test asserting every fixture constant in
  `parse/tests.rs` is preceded by a provenance comment naming a prikk version — cheap, and it makes §0
  enforceable rather than aspirational.
- **Gates:** `fmt` / `clippy --all-targets --all-features -D warnings` / `test`, all green.

---

## 8. Example

Extend `changes_demo` with a scripted `NullBackend` state carrying `queued_elsewhere`, so the warning
band and the suppressed UD-08 banner are drivable with no prikk and no repository — the state that
caused the defect becomes the state the demo shows. Add a `NullBackend::with_queued_elsewhere` builder
beside the existing `with_worktree_status`.

**Also verify against the real binary**, not only the scripted one: reproduce RFC 009's F1 and F4 by
the commands in §2.1 and §2.4 and confirm stikk now opens the repository and shows the warning. A
scripted test alone cannot prove this increment, because a scripted test is what missed it.

---

## 9. Acceptance criteria

1. `stikk <repo>` opens a repository **with queued patches** and renders Orientation, showing the
   queue count and its target ref. Verified against the real prikk binary, not only `NullBackend`.
2. An unpublished `heads/main` yields `main_ref_state == None`; no sentinel is ever carried as an
   object id; an unrecognized value refuses.
3. `branch list` on an empty repository yields an empty ref list; no phantom `no` ref exists; the
   `Prikk::refs` doc comment states what it actually returns, and the tag gap is recorded.
4. `WorktreeStatus`/`ChangesView` carry prikk's `queued_elsewhere` note **verbatim**; the Changes view
   renders it as a distinct inert warning band; and **the "a commit still captures them" claim is
   absent whenever that note is present** (tested).
5. A prikk exit code of `2` produces `StikkError::Internal` → fault screen, never a `Refusal`.
6. Every fixture in `parse/tests.rs` is captured from a real prikk run and carries a provenance line
   naming the command and version; the false `branch list` provenance comment is gone.
6b. The floor is **≥ 0.28** and a prikk newer than the validated **0.30** runs while Orientation states
   that it is unvalidated; `NullBackend`'s default is `0.30.0`, and the below-gate Changes test scripts
   its old version explicitly rather than inheriting the default. `README.md` and
   `docs/src/guide/getting-started.md` state the new range.
7. `UD-08` is retired in the design set and in Changes copy; the malformed-`.prikkignore` refusal has a
   glossary entry and a next-step that could actually resolve it.
8. `fmt` / `clippy -D warnings` / `test` green; `changes_demo` builds and drives the new state.

---

## 10. Out of this increment, queued next

- **RFC 010** — off-thread seam and UI responsiveness (`NFR-P01`/`NFR-P02` are currently unmet).
- **Completing `FR-014`'s ref surface** with a `tag list` seam read, so the ref picker shows tags it
  currently only claims to.
- **Increment 6** — session persistence and progressive disclosure, which needs **RFC 003**'s
  repository fingerprint (`DM-02`/`LC-9`) before `SessionState` can be keyed correctly.
- A **richer `.prikkignore` surface**, and the **`C` commit action** that this Changes view is the
  preview for.

---

## 11. Delivered — 2026-09-04

Built and verified against a live prikk binary (0.30.0, then rebuilt to 0.31.0 mid-implementation —
which turned out to be a live exercise of decision 7's soft ceiling: stikk kept working and said so):

- **Seam** (`stikk-prikk`): `parse::orientation` now parses `queued patches: <n>[ targeting <ref>]`,
  preserving the target as `Orientation::queued_target` and tolerating an interposed active-patch
  threshold `warning:` line (F1); `optional_object_id` replaced with a sentinel-set check
  (`<none>`/`<not published>`/`<missing metadata>`/`<malformed metadata>`) plus an `ObjectId`-shape
  check on anything else, refusing rather than fabricating (F2); `parse::refs` anchors on the id's
  shape, treats `no branches`/`no tags` as the empty list, and refuses an unrecognized marker or
  trailing content — the `no branches` phantom-ref regression is directly tested (F3);
  `WorktreeStatus::queued_elsewhere` carries prikk's queued-elsewhere note verbatim, matched by prefix
  so the generic commit-hint note is ignored without refusing (F4); `CliBackend::run`/`run_capturing`
  route a `2` exit to `StikkError::Internal` via a shared `usage_error` helper, never through
  `classify` (F6); `Version::is_validated` adds the 0.30 ceiling alongside the 0.28 floor, and
  `Handshake`/`NullBackend::supported` carry `validated` (decisions 6–7).
- **Operations** (`stikk-core`): `OrientationView` gains `prikk_validated`/`queued_target`;
  `ChangesView` gains `queued_elsewhere`, passed through unmodified; `present()`'s `LoadChanges`
  next-steps gain an unconditional `.prikkignore` pointer (never derived from the message — C-T2b) and
  the glossary gains a `.prikkignore` entry linked via the existing `codes_in` mechanism (F5).
- **TUI** (`stikk-tui`): Orientation shows "`N` queued · targeting `<ref>`" and, above the validated
  ceiling, "validated through 0.30 — this prikk is newer…" (both `Wrap`-enabled, inert); the Changes
  view renders `queued_elsewhere` as a distinct quoted band above the entries and suppresses the
  UD-08 filter's "a commit still captures them" claim while it is present, replacing it with a pointer
  to the warning (the acceptance-critical behaviour); `changes_demo` drives the new state with a
  `NullBackend::with_queued_elsewhere` builder.
- **Fixtures**: every constant in `parse/tests.rs` re-captured verbatim from the live prikk binary
  (init → commit → seal → branch create/close → bundle export/import for a received ref → dirtied
  worktree → queued-elsewhere reproduction → malformed `.prikkignore` → exit-2 usage error), each with
  a provenance comment; a new test (`every_fixture_constant_carries_a_provenance_comment`) enforces the
  rule mechanically. The false `branch list` "…and a tag" provenance comment is gone.
- **Design set**: `docs/src/reference/data-model.md` and `threat-model.md` no longer claim prikk has no
  ignore mechanism (`requirements.md`/`ROADMAP.md` were already corrected before this handoff);
  `threat-model.md` gains **RR-9**, recording F4 as a live `T-T4`/`C-T4c` violation in shipped 0.1.0 and
  how it was closed, per `NFR-S07`. `README.md`, `crates/stikk/README.md`, and
  `docs/src/guide/getting-started.md` state the new `>= 0.28`, validated-through-`0.30.0` range.
- **Verified against the real binary, not only `NullBackend`** (the discipline this increment exists to
  enforce): reproduced F1 (`stikk` now opens a repository with one queued patch and shows
  "1 · targeting heads/main" where it previously errored `prikk field "queued patches:" is not a
  number`) and F4 (`worktree_status` against a live `heads/other` returns
  `queued_elsewhere: Some(...)` byte-identical to the fixture) end-to-end through the built launcher and
  the `CliBackend`, against both 0.30.0 and, after a mid-session rebuild, 0.31.0 — the latter run is
  live evidence for decision 7's ceiling notice.
- **Gates**: `fmt` / `clippy --workspace --all-targets --all-features -D warnings` / `test` green — 194
  tests (up from 164). `cargo build --examples -p stikk-tui` succeeds.

Known limitation carried forward: the `.prikkignore` next-step and glossary entry address the
malformed-file refusal (F5); a richer ignore surface (showing rules, offering to add one) remains
out of scope, as scoped. Items in §10 remain queued; RFC 009 stays **accepted**.

# Handoff — the post-0.2.0 correctness sweep (v1)

**Companion to:** [RFC 012](../../accepted/012-post-0-2-0-correctness-sweep.md) (Accepted 2026-09-05).
Inherits its state.
**Realizes:** the second increment of **0.3.0**, after
[RFC 010](../../done/010-off-thread-seam-and-ui-responsiveness.md) (shipped to `main`).
**Design items:** `FR-121`/`AC-04`/`NFR-S01` (read-only vs recovery), `FR-003`/`FR-110`/`C-T2b`
(refuse-and-explain, stikk-authored next-steps), `NFR-T01`/`CF-01`/`CF-04` (per-platform user-scope
paths), `INV-9` (never fabricate an identifier), `FR-014` (ref listing), `NFR-R03`/`ASM-2` (version
honesty), `C-T2a` (inert rendering).

Five independent findings plus one upstream re-validation. **They share a release, not a design** —
build them in the order below, but each is separable, and a problem in one must not stall the others.
Say in the review request if you land them as separate commits (preferred) or one.

**This is a breaking increment** (F-a and F-d change public API). That is expected in 0.3.0 per
RFC 011; do not contort a design to avoid it.

---

## 1. Scope

**In**, in build order:

1. **F-e (upstream first)** — re-validate against prikk 0.31 *empirically*, then the schema-skew gloss.
2. **F-b** — a distinct guidance target for version skew. Non-breaking, self-contained.
3. **F-c** — per-platform config/state paths, hand-rolled. Non-breaking.
4. **F-a** — `may_operate` moves to `Readiness`. Breaking.
5. **F-d** — adopt `RefName` across the seam's ref-bearing fields. Breaking, and the largest.
6. **`FR-014` completion** — a `tag list` seam read, so the ref picker shows tags it currently only
   claims to.

**Out (do not build here):**
- **RFC 003** (change token / fingerprint) — the next increment.
- Anything from `NFR-P02` (cancellation) — still deferred to `FR-100`, per RFC 010.
- Any change to `Command::output()` or the EPIPE guard.
- Session persistence, the working cycle, Compare, Patch detail.

---

## 2. F-e — re-validate against prikk 0.31, then explain its skew

**Do this first**, because if 0.31 changed an output shape, everything else in this increment is being
built against a stale baseline.

**Step 1 — verify, do not trust.** prikk 0.31's changelog says it changes *no* CLI surface. RFC 009
exists because a claim like that went unchecked. So: capture every fixture in
`stikk-prikk/src/cli_backend/parse/tests.rs` again from a real prikk **0.31** binary, and diff against
the committed ones. **Report the diff (or its absence) explicitly in the review request** — that
sentence is the deliverable, not the assumption behind it.

- If identical: update each provenance comment to name 0.31 as re-verified, and raise
  `VALIDATED_MAX_MINOR` from `30` to `31`.
- If **anything** differs: **stop and report it.** A shape change is a new RFC 009, not a line to fix
  inside a sweep.

**Step 2 — gloss the skew refusal.** prikk 0.31 writes repositories that prikk ≤ 0.30 cannot read. An
older prikk emits, verbatim:

```
error: integrity error: format-2 patch does not accept envelope schema 3 (accepted: [1, 2])
```

Today stikk degrades this to a verbatim `Refusal` with next-steps ("Choose another ref", "Refresh")
that cannot help. `FR-003` requires stikk to refuse-and-explain version skew. Add:

- a **glossary entry** (`DM-09`) keyed so the message resolves to it, explaining that the repository was
  written by a newer prikk and that prikk's compatibility guarantee is **backward, not forward** —
  older prikk reading newer history is the direction that is *not* promised;
- a **next-step** from `(class, operation)` — "upgrade prikk" as a
  `DismissAndResolveExternally` step. **Never** pattern-matched out of prikk's message (`C-T2b`), and
  never an action stikk performs.

Be careful with the classifier: do **not** widen `is_integrity_finding` to catch this. It would route
schema skew into the (nonexistent) Verify view instead of an explanation, and it would catch genuine
integrity findings in read contexts. Leave the classification as `Refusal` and let the gloss do the
work — the RR-5 degradation is behaving correctly here.

---

## 3. F-b — version skew must stop pointing at signing keys

`changes_view` reports a too-old prikk as `StikkError::NotReady`; `present()` maps every `NotReady` to
`InlineGuidance { toward: Target::TrustKeys }`. A user on prikk 0.27 opening Changes is told to check
their signing keys.

`Target` is `#[non_exhaustive]`, so **add a variant** (e.g. `Target::PrikkVersion`) and route the
version-skew case to it. `present()` must be able to tell the two `NotReady` causes apart —
**use `OperationContext`, not the message text** (`C-T2b`): `LoadChanges` + `NotReady` is a version
gate today. If you find that too coarse, say so in the review request rather than reaching for the
string.

The banner text in `App::surface` is currently hard-coded to append
"— see Glossary → Trust & Keys" for *every* `InlineGuidance`. That must become target-dependent.

---

## 4. F-c — per-platform paths, hand-rolled

`config_file()` and `state_dir()` resolve XDG/`HOME` only. `NFR-T01` claims Linux, macOS and Windows;
0.2.0 ships binaries for all three; on Windows `HOME` is typically unset, so `stikk config path` fails.

**No dependency** (RFC 012 F-c). `stikk-state`'s dependency graph stays at exactly `stikk-model` — it
holds the primary write-boundary control and that is worth preserving. Target behaviour:

| | config | state |
|---|---|---|
| Linux | `$XDG_CONFIG_HOME/stikk` else `$HOME/.config/stikk` | `$XDG_STATE_HOME/stikk` else `$HOME/.local/state/stikk` |
| macOS | `$HOME/Library/Application Support/stikk` | `$HOME/Library/Application Support/stikk` |
| Windows | `%APPDATA%\stikk` | `%LOCALAPPDATA%\stikk` |

**Two things that must not regress:**

1. **`STIKK_CONFIG` / `STIKK_STATE_DIR` still win outright** (`CF-04` precedence: environment > config >
   default). They are checked before any platform logic, as today.
2. **Existing Linux paths must be byte-identical.** A user upgrading from 0.2.0 must not silently get a
   new, empty config location. Assert this in a test.

**Test it the way `stikk-prikk::env` does**: resolve through an **injected environment lookup** so the
rules are exercised hermetically for all three platforms without touching process-global state or
requiring the test to run on that OS. That pattern is already in this codebase and is the reason a
hand-roll is defensible here — it gives better coverage than a dependency would. `#[cfg]` only where
the real lookup is wired.

Keep the existing "no home at all" environment error as the final fallback.

---

## 5. F-a — `may_operate` moves to `Readiness`

**Ruled (RFC 012):** `FR-121` governs — a read-only session may not clear a lock. `AC-04`'s
"orthogonal" describes how Operator is *derived*, not an exemption from the global switch.

- **Delete `Capability::may_operate`.** It is unimplementable there: `Capability::derive` discards
  `read_only`, so a read-only session and a no-keys session are indistinguishable by the time a
  `Capability` exists.
- **Add `Readiness::may_operate(self) -> bool`**, returning `!self.read_only`.
- Update `capability.rs`'s docs: Operator is orthogonal to the signing ladder *and* subject to
  read-only, and each recovery action still carries its own typed confirmation (`FR-102`).
- **Correct external design `AC-04`** in `docs/src/reference/external-design.md` to say so. The design
  set is the source of truth for tests; leaving it contradicting `FR-121` is the actual bug.

There is no caller today — that is *why* this lands now, before `FR-102` exists to inherit the wrong
semantics. Test the ruling directly: read-only ⇒ `!may_operate()`, regardless of key presence.

---

## 6. F-d — adopt `RefName`

`stikk_model::RefName` rejects empty and control-character-bearing names, and is referenced by **no**
code. Every ref name from the seam travels as a plain `String`: `History.reff`, `RefEntry.name`,
`WorktreeStatus.reff`, `Orientation.queued_target`.

Adopt it at the **parse boundary**, exactly as F2 (RFC 009) adopted `ObjectId` for ids: the parser
validates and refuses (`StikkError::Environment`, `UD-02`) on a shape prikk would never emit. Above the
seam the type carries the guarantee.

**Judgment call for you, and say which you chose:** change the struct fields to `RefName`, or keep
`String` fields and validate at parse time without changing the types. The first is stronger (the
guarantee is in the type and cannot be bypassed later); the second is a smaller diff. **I lean to the
first** — a newtype nobody holds is how we got here — but the field change ripples into the frontends
and demos, so make the call with the diff in front of you and justify it.

Display stays inert regardless (`C-T2a`) — validation and inert rendering are complementary, not
alternatives. Do not remove any `inert()` call on the grounds that a value is now validated.

---

## 7. `FR-014` — the ref picker's missing tags

`prikk branch list --all` emits branches only (open, `(closed)`, `(received)`); tags come from
`prikk tag list`. RFC 009 corrected the *documentation* of this and recorded the gap; close it now.

Add a `tag list` read to the seam (category `read-history`), fixtures **captured** per RFC 009 §0, and
merge its results into `list_refs` so the picker shows `tags/…` entries. `RefEntry::is_tag()` finally
has a source. Note prikk prints `no tags` for an empty list — the same empty-list shape F3 handled.

**This adds a trait method**, which is breaking for any outside implementor — fine in 0.3.0, and it is
the last thing in this increment to touch the trait.

---

## 8. Security surface

- **`C-T2b`** — every new next-step (F-e, F-b) comes from `(class, operation)`. A test must show that a
  hostile message containing "upgrade prikk" or ".prikkignore" produces no actionable entry.
- **`C-T2a`** — F-d does not replace inert rendering. Assert both still hold on a hostile ref name.
- **`C-E2`** — F-c changes where stikk resolves its paths. `ensure_outside_repository` must still be the
  gate, and a test must show a repository-internal target is refused on **every** platform branch, not
  just the one the test host runs on. This is the primary control; do not let a platform refactor
  quietly narrow it.
- **`INV-9`/`UD-02`** — F-d and F-e widen what refuses. Every new refusal path gets a test.

No new asset, flow or trust boundary; no threat-model edit expected. Record that the review happened
(`NFR-S07`).

---

## 9. Test plan

Per finding, plus: **the gates, with the count called out.** Specific asks beyond the obvious:

- **F-e**: the fixture re-capture diff result, stated explicitly. Plus: an older-prikk schema error
  resolves to the new glossary entry and offers the upgrade next-step.
- **F-b**: a version-gated Changes refusal points at the prikk-version target, **not** Trust & Keys —
  assert the rendered banner does not contain "Trust & Keys".
- **F-c**: all three platform branches resolved through the injected lookup; overrides still win;
  **Linux paths byte-identical to 0.2.0**; the no-home error survives.
- **F-a**: read-only ⇒ no operator capability, with and without keys present.
- **F-d**: a control-character-bearing ref name **refuses at the parser**; display stays inert.
- **F-014**: `no tags` yields an empty list; tags appear in the picker alongside branches.

---

## 10. Acceptance criteria

1. Fixtures re-captured against prikk **0.31** and the diff reported; `VALIDATED_MAX_MINOR` raised only
   if they matched. Any difference was escalated, not absorbed.
2. An older prikk's schema-3 refusal resolves to a glossary entry and an "upgrade prikk" next-step,
   stikk-authored (`C-T2b`), and the classifier was **not** widened.
3. Version skew no longer routes to Trust & Keys.
4. Config and state resolve per platform convention on Linux, macOS and Windows; overrides still win;
   **existing Linux paths unchanged**; `stikk-state` still depends only on `stikk-model`.
5. `Capability::may_operate` is gone; `Readiness::may_operate` exists and honours read-only; external
   design `AC-04` corrected.
6. Ref names are validated at the parse boundary and still rendered inert.
7. The ref picker lists tags; `no tags` is the empty list.
8. Gates green; test count delta stated; all four demos build.
9. Nothing tagged, pushed, or published.

---

## 11. Submit

Package to `.git-exclude/review-request/012-post-0-2-0-correctness-sweep/review-request-v1.md`.

Call out: **the F-e fixture diff result** (the single most important sentence in the package), your
F-d type choice and why, whether you landed one commit or six, and anything in §2's "stop and report"
path that you hit.

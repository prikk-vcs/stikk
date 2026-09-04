# RFC 012 — Post-0.2.0 correctness sweep, and the re-sequenced roadmap

**Status.** Proposed (2026-09-04) — collect the findings left over from the RFC 009 review plus one new
upstream fact, rule the two that carry design questions, and record the **re-sequenced roadmap** the
owner delegated (2026-09-04: no 0.2.1 — these ride 0.3.0; and increment 6 no longer precedes the
working cycle).
**Tracks.** Correctness and honesty defects found by review rather than by test, and the release
sequencing that follows from which of them are breaking.
**Touches.** `stikk-model` (`Capability`), `stikk-core` (`present`, glossary), `stikk-prikk`
(`RefName` adoption, a `tag list` read), `stikk-state` (`paths`), and the design set (`CL-05`,
`releasing.md`). Plus `ROADMAP.md`, which this RFC re-sequences.

## Summary

Five findings, none of which fit RFC 009's scope. Two are implementation gaps against a design that is
already clear; two are places where the **design set contradicts itself** and a ruling is needed before
code can be right; one is new upstream ground truth from prikk 0.31, released the same day as stikk
0.2.0.

The sequencing consequence is the useful part: **four of the five are breaking**, so they group into
one release rather than trickling out. The owner ruled against a 0.2.1, so everything here lands in
**0.3.0**.

## The findings

### F-a — `Capability::may_operate()` cannot honour read-only, and the design set disagrees with itself

`may_operate()` returns `true` unconditionally. `FR-121` says a global read-only mode "locks tier 2–3
out entirely" and lists **lock clearing** as tier 3; external design `AC-04` says Operator is
"orthogonal to the above; always tier 3." Those cannot both be applied.

The code cannot currently implement the `FR-121` reading even if we chose it: `Capability::derive()`
collapses `read_only` into `Viewer`, so by the time a `Capability` exists, a read-only session and a
no-keys session are **indistinguishable**. `may_operate(self)` has no access to the fact it would need.

**Ruling — `FR-121` wins; read-only locks out recovery too.** `AC-04`'s "orthogonal" describes how the
Operator role is *derived* (any human at the machine, not a signing role), not an exemption from the
global read-only switch. `STIKK_READ_ONLY=1` is documented as a control the UI cannot lift
(`CF-04`, `NFR-S01`); a mode that still permits clearing another writer's lock would be a read-only
mode that mutates, which is exactly the "confident-but-wrong picture" this project refuses. External
design `AC-04` is corrected to say so.

Fixing it is a **public API change** (`may_operate` must see readiness, not a collapsed capability), so
it is breaking — a bigger change than it looks, and one that must land **before `FR-102`** (the lock
inspector) exists to consume the wrong semantics.

### F-b — a version-skew message points the user at their signing keys

`changes_view` reports a too-old prikk as `StikkError::NotReady`, and `present()` maps every `NotReady`
to `InlineGuidance { toward: Target::TrustKeys }`. So a user on prikk 0.27 opening Changes is told
*"Worktree review needs prikk ≥ 0.28 … — see Glossary → Trust & Keys."* Their keys are not the problem.

Root cause: `NotReady` is overloaded for two unrelated conditions — absent signing readiness, and
version skew — and one mapping cannot serve both. **Ruled:** add a distinct target for the environment
case rather than a new error class; `Target` is `#[non_exhaustive]`, so this is **not breaking**.

### F-c — `paths.rs` resolves XDG/`HOME` only, on a release that ships macOS and Windows binaries

`config_file()`/`state_dir()` follow `XDG_CONFIG_HOME`/`XDG_STATE_HOME` else `$HOME`, with a source
comment calling a per-platform resolver "a later increment." `NFR-T01` already claims Linux, macOS and
Windows, `CF-01` already says "user scope, **per platform convention**", and 0.2.0 ships binaries for
all three. On Windows `HOME` is typically unset, so `stikk config path` fails unless the user sets
`STIKK_CONFIG`/`STIKK_STATE_DIR` themselves.

This is a gap against a clear design, not a design question. The only open call is whether to take a
dependency (e.g. `directories`) or resolve by hand; **ruled: take the dependency** — platform
conventions are a moving target that a hand-rolled resolver gets subtly wrong, and this is precisely
the kind of well-audited, narrow crate the *Less is more* rule permits. Non-breaking.

### F-d — `RefName` exists to reject control characters and is used nowhere

`stikk_model::RefName` validates that a ref name is non-empty and control-character-free. It is
referenced by **no** code in `stikk-prikk` or `stikk-core`: `History.reff`, `RefEntry.name`,
`WorktreeStatus.reff` and `Orientation.queued_target` are all plain `String`. Display is inert at every
call site (that was review finding M1), but nothing *validates*.

`INV-9` says every stikk→repository reference is a re-resolvable identifier, and `stikk-model`'s own
docs say stikk "never fabricates an identifier." F2 in RFC 009 applied exactly this discipline to
object ids via `ObjectId::parse`; the ref-name half was never done. Adopting it changes public struct
field types, so it is **breaking**.

### F-e — prikk 0.31.0 is forward-incompatible, and stikk has nothing to say about it

Released 2026-09-04, hours after stikk 0.2.0. It changes **no CLI surface** — no command, flag, exit
code or message — but repositories it writes **cannot be read by prikk 0.30 or earlier**, which refuse
with:

```
error: integrity error: format-2 patch does not accept envelope schema 3 (accepted: [1, 2])
```

Two consequences:

1. **The soft ceiling already works.** A user on 0.31 sees stikk's "validated through 0.30 — this prikk
   is newer; its output shapes have not been checked" notice. RFC 009 decision 7 earned its keep one
   day after shipping, which is worth recording as evidence that an unbounded upper range would have
   been the wrong default.
2. **stikk cannot explain the failure it will now see.** A user whose prikk is *older* than the
   repository hits that error on every command. stikk's classifier does not match it, so it degrades to
   a verbatim `Refusal` — safe (`RR-5` working as designed) but unglossed, with next-steps that cannot
   help. `FR-003` explicitly requires stikk to refuse-and-explain "version skew between stikk and the
   prikk core in use." **Ruled:** add the glossary entry and the `(class, operation)` next-step —
   "this repository was written by a newer prikk; upgrade prikk" — never parsed from the message
   (`C-T2b`).

**Also ruled: re-validate against 0.31 and raise the ceiling — by *running* the fixtures, not by
reading this changelog entry.** 0.31 claims no CLI change; that claim is exactly the sort of thing
RFC 009 exists to distrust. Capture the fixtures against 0.31, diff them against the committed ones,
and only then move `VALIDATED_MAX_MINOR`.

## Decisions

1. **`FR-121` governs read-only; `AC-04` is corrected** (F-a). `may_operate` takes readiness. Breaking.
2. **A distinct guidance target for environment/version skew** (F-b), not a new error class.
3. **Per-platform path resolution via a dependency** (F-c).
4. **Adopt `RefName` across the seam's ref-bearing fields** (F-d). Breaking.
5. **Gloss the schema-skew refusal, and re-validate the ceiling against 0.31 empirically** (F-e).
6. **All of it lands in 0.3.0** — the owner ruled out a 0.2.1 (2026-09-04). Four of the five are
   breaking, so grouping them costs one breaking release instead of three.

## The re-sequenced roadmap

Delegated by the owner on 2026-09-04 ("no need to keep it original — make it reasonable and effective").
Two changes from the roadmap as written:

- **Session persistence (increment 6) no longer precedes the working cycle.** It was sequenced there
  when stikk could not open a real repository; now that it can, resuming your last view matters less
  than being able to commit. It moves after, where it is also *cheaper* — RFC 003's fingerprint, which
  it needs, will already exist.
- **The foundations are grouped by breakage, not by theme**, so one release absorbs the churn.

| Release | Theme | Contents |
|---|---|---|
| **0.3.0** | Foundations & correctness (all breaking) | **RFC 010** (off-thread seam) → **RFC 012** (this sweep) → **RFC 003** (fingerprint + change token). Ships: a UI that never blocks, cancellable reads, correct paths on every platform we ship, honest version-skew guidance, validated ref names, resolved capability semantics, staleness detection. Plus the `tag list` read that completes `FR-014`. |
| **0.4.0** | The working cycle | The `FR-120`/`FR-121` preview + tiered-confirmation machinery (`OPL-01…05`), then **commit → queue review → seal ceremony**. The first mutations. |
| **Later** | | Session persistence (`FR-122`, cheap once RFC 003 lands, and the increment that finally gives `C-E2` a production caller); verify/doctor browser; branches, tags and checkout planning; merge evidence and rollback; Compare and Patch detail when `UD-09` allows. |

**Order within 0.3.0 is not arbitrary.** RFC 010 reshapes the seam trait, so it goes first or everything
after it gets re-touched. RFC 012's F-a/F-d change the same public surface, so they follow immediately.
RFC 003 adds `change_token()` to that trait, so it goes last — after the shape has settled.

## Open questions

- **Does `may_operate` survive at all?** If read-only locks out recovery, Operator may be better
  expressed as a check over `Readiness` than as a method on `Capability`. Settle in the F-a handoff;
  the ruling above fixes the semantics, not the shape.
- **Which platform-dirs crate** (F-c) — evaluate at handoff time against dependency weight, not now.

## Consequences

- One breaking release instead of three, and 0.4.0 arrives at the working cycle with responsiveness,
  staleness detection and capability semantics already correct — all three of which a mutation needs
  and none of which is pleasant to retrofit underneath one.
- `C-E2`, described in the threat model as the *primary* control against writing inside a repository,
  still has **no production caller**. Session persistence is what gives it one; noting it here so its
  demotion to "Later" is a considered risk rather than an oversight.
- The re-validation obligation is now demonstrably recurring: prikk shipped 0.29, 0.30 and 0.31 inside
  the window this project took to find and fix RFC 009. Treat "check the real binary" as a standing
  cost of every increment, not a phase.

# RFC 020 — The MSRV raise's other six sites, and the gate that would have found the advisory

**Status.** **Accepted by the project owner 2026-09-08**, Q1 ruled with it. **F1 shipped** the same day (`b76ea38`). Proposed 2026-09-08. Opened on reviewing the owner's own security commit `e9b8c70`
(ratatui 0.29 → 0.30, MSRV 1.85 → 1.88, clearing RUSTSEC-2026-0009). **The upgrade itself is correct
and I verified its central claim** (below). What it did not carry with it is the MSRV number, which
still reads **1.85** in **ten places across eight files** — three of them workflow pins that will now
fail. *(This RFC first said "six places", from a grep over the paths I happened to think of. Applying
the fix found `docs/src/guide/getting-started.md` and `CONTRIBUTING.md` as well — **the finding had the
same scope error as the thing it was reporting**, which is F3's own point landing on its author.)*
**Tracks.** `NFR-R03` (version honesty), RFC 011 (breaking position), RFC 018 (the grep-scope rule).
**Touches.** `.github/workflows/{release,real-binary}.yml`, `README.md`,
`docs/src/contributing/development.md`, `.git-exclude/specs/`, and a new supply-chain workflow.

## Summary

The security fix is right. The MSRV it required is stated in `Cargo.toml` and `ci.yml` and **nowhere
else that needs it** — including the workflow that cuts releases.

This is RFC 018's finding for the fourth time, in a directory RFC 018's own remedy does not cover.

## The claim I checked before writing anything else

`e9b8c70`'s message says `time` arrives through `ratatui-widgets`' calendar and *"the facade does not
make optional"* — the premise the whole MSRV raise rests on. **If the calendar were droppable, the
advisory would be avoidable without raising MSRV at all**, and a 1.88 floor is a breaking change for
consumers.

**It looked droppable and it is not.** `ratatui-widgets` does gate it (`calendar = ["dep:time"]`,
optional), and the facade does expose `widget-calendar` — but the facade's own dependency is:

```toml
[dependencies.ratatui-widgets]
version = "0.3.0"          # ← no default-features = false
```

so `ratatui-widgets`' `default = ["all-widgets"]` enables `calendar` before any facade feature has a
say. **Verified empirically, not read off a manifest**: with `default-features = false` and the
feature set stikk actually uses, with and without `macros`, `cargo tree -i time` still resolves
`time v0.3.55`.

**The owner's diagnosis is correct and the MSRV raise was necessary.** Recorded because the next
person to look at a `time` advisory here will have the same idea I did, and should be able to find out
in one paragraph that it does not work.

## Findings

### F1 — two workflows still pin 1.85, and the code no longer compiles under it *[live break]*

`e9b8c70` applied clippy's let-chain suggestion, which the new MSRV unlocks:

```rust
if let Some(target) = orientation.queued_target.as_deref()
    && target != reff
{
```

**Let-chains are stable in 1.88.** So the code genuinely requires it, and:

| Workflow | Pins | Consequence |
|---|---|---|
| `.github/workflows/ci.yml` | **1.88** | correct |
| `.github/workflows/real-binary.yml` | **1.85** ×2 | **broken now** — RFC 019's suite cannot build |
| `.github/workflows/release.yml` | **1.85** | **the next release fails at tag time** |

**The release one is the serious half.** It fails at exactly the moment 0.4.0's prep added a
`cargo package` check to avoid — after the tag is pushed, mid-publish, with a version number already
spent.

### F2 — six documentation sites still say MSRV 1.85

`README.md`, `CONTRIBUTING.md`, `docs/src/guide/getting-started.md`,
`docs/src/contributing/development.md`, and **both spec files the dev team treats as their operating
manual** (`.git-exclude/specs/00-project-overview.md`, `02-implementer-handoff.md` §3). An
implementer following the spec would install a toolchain that cannot build the workspace.

### F3 — the grep scope RFC 018 wrote excludes `.github/`

RFC 018 replaced an inclusion list with an exclusion list — *every tracked `.md` and every `crates/**`
source file* — after an inclusion list had failed three times. **`.github/workflows/` is in neither**,
which is exactly why F1 exists.

The rule was right in shape and still drawn too small. **A version or capability sweep covers every
tracked file that states a fact about the project**, with named exceptions (`CHANGELOG.md`'s released
sections, `Cargo.lock`), not a list of blessed directories.

### F4 — nothing in CI would have found the advisory

There is no `deny.toml`, no `cargo audit`, no `cargo deny` workflow. **RUSTSEC-2026-0009 was found by
the owner, by hand.** Five gates cover formatting, lints, tests, examples and rustdoc; none covers a
dependency advisory, and this project now ships six crates and six platform binaries.

That is the RFC 019 shape again — *nothing was checking* — one supply chain over.

## Decisions

1. **Fix F1 first and separately.** Two workflow files, one number. It is the only live break.
2. **One source for the MSRV.** The number appears in `Cargo.toml`'s `rust-version` and is repeated in
   four workflows and four documents. Where a workflow can read it rather than restate it, it should —
   the same discipline `validated_ceiling_display()` already holds for the prikk ceiling, and the same
   one RFC 019's version matrix holds for the suite.
3. **Widen the sweep rule to every tracked file** (F3), exceptions named rather than inclusions listed.
4. **Add a supply-chain gate**, advisory-only at first: it must not redden a PR for an advisory
   published upstream overnight, the same reasoning that put RFC 019's suite outside `ci.yml`. **A
   green run is required in release prep**, where a human reads it.
5. **The MSRV raise and the ratatui major are `### Breaking` entries** for 0.5.0 (RFC 011: for 0.x the
   minor is the breaking position). Neither is currently recorded anywhere a user will read.

## Q1 — RULED by the owner, 2026-09-08

**Non-blocking. Required to be read in release prep. Every allowlist entry carries a dated reason.**

The reasoning stands as written below: a gate that cannot be satisfied gets bypassed, and a bypassed
gate is worse than none — RFC 019 Q1's own argument, one supply chain over. What makes this version
honest rather than decorative is the *release-prep* half: the run is not advisory at the moment it
matters, it is a thing a human has to look at before a tag exists.

**Deliberately not ruled, and not to be added quietly:** whether it also runs on a schedule. An
advisory published overnight affects users who already installed a binary, which is a stronger argument
for a cron than RFC 019's suite had — and a muted cron is the same failure either way. **It is a
separate decision with a separate owner question**, and this increment does not pre-empt it.

### The original question, for the record

**Q1 — does the supply-chain gate block a release, or inform it?** `cargo deny` fails on any advisory
in the tree, including ones with no fix available and ones in a dev-dependency that never ships. A gate
that cannot be satisfied gets bypassed, and a bypassed gate is worse than none (RFC 019 Q1's own
reasoning). **My lean: advisory-only, required-to-be-read in release prep, with an explicit
`deny.toml` allowlist whose every entry carries a dated reason.** Wants a ruling — it is a policy about
what stops a release.

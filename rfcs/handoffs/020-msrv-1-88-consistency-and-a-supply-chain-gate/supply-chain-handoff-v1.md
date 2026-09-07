# Handoff — the sweep rule and the supply-chain gate (v1)

**Companion to:** [RFC 020](../../accepted/020-msrv-1-88-consistency-and-a-supply-chain-gate.md)
(Accepted 2026-09-08; **Q1 ruled by the owner** with the acceptance). Inherits its state.
**Realizes:** F3 and F4. **F1 and F2 already shipped** in `b76ea38` — the MSRV number now reads 1.88 in
all ten places; do not redo that.
**Design items:** `NFR-R03`, RFC 011 (breaking position), RFC 018 (the sweep rule this replaces).

> **The advisory that started this was found by a person reading a report, not by anything in CI.** The
> owner fixed it, correctly, and the fix's own MSRV number then failed to reach three workflow pins —
> one of which would have broken a release after the tag was pushed.
>
> **Both halves of this increment are about the same thing: a fact that lives in more than one place,
> and nothing checking that the copies agree.**

---

## 1. Scope

**In:** the sweep rule as a command (§2); the supply-chain gate (§3); MSRV derived rather than restated
in workflows (§4); the changelog's `### Breaking` entries (§5).

**Out:** F1/F2 (shipped). Any schedule for the gate — **explicitly deferred by the owner's ruling, not
an oversight**; do not add a `schedule:` trigger. The prikk 0.34/0.35 re-baseline. Anything that
changes product behaviour.

---

## 2. F3 — the sweep rule becomes a command, not a path list

RFC 018 replaced an inclusion list with an exclusion list — *every tracked `.md` and every `crates/**`
source file*. **That is still a path list**, `.github/` was in neither, and F1 is the result.

**Replace it with the command that actually worked.** When I fixed F1/F2 I found two more sites than my
own RFC named, and this is what found them:

```sh
git ls-files | xargs grep -ln "<the value being changed>"
```

**Exhaustive by construction** — every tracked file, no directory anyone has to remember. Write it into
`.git-exclude/specs/02-implementer-handoff.md` as *the* way to run a version or capability sweep,
replacing the path-list rule, with:

- **Named exceptions, not blessed directories.** `CHANGELOG.md`'s already-released sections are dated
  historical claims, not current ones; `Cargo.lock` is generated. Name those two and nothing else.
- **RFC 018's other half kept: grep is a floor, not a ceiling.** The Glossary's false safety claim was
  not a version string and no grep would have matched it. The command finds copies of a *known* value;
  reading finds claims that quietly became false. Both, every time.

## 3. F4 — the supply-chain gate

**`cargo-deny`, not `cargo-audit`.** Advisories are the reason this exists, but the same run covers
**licenses** — stikk publishes six Apache-2.0 crates, and a copyleft dependency arriving transitively is
a problem nobody here is currently positioned to notice. One tool, one config, one report.

**Q1 is ruled — build exactly this, and no more:**

- **Its own workflow**, `workflow_dispatch` only. **No `push`, no `pull_request`, no `schedule`.** The
  schedule question is deliberately open (RFC 020's Q1 note); adding one here pre-empts a decision that
  is the owner's.
- **Non-blocking**: it never reddens an unrelated PR. The same reasoning that put RFC 019's suite
  outside `ci.yml`.
- **A green run, read by a human, is required in release prep.** Add it to the release-prep checklist
  in `.git-exclude/specs/` beside the existing gates and RFC 019's suite. That is the half that makes
  this real rather than decorative.

**`deny.toml`, and the one rule that keeps it from rotting:** every `[advisories] ignore` entry carries
**a dated reason and what would let it be removed**. An ignore list without dates becomes permanent by
default, and a permanently-ignored advisory is indistinguishable from an unnoticed one.

Seed it against the tree as it stands and **report what the first run says** — including anything you
end up allowlisting and why. If it is clean, say so plainly; that is a useful fact and this project has
learned to state it rather than imply it.

## 4. Decision 2 — derive the MSRV, do not restate it

Four workflows now each carry `toolchain: "1.88"`, and `Cargo.toml` carries `rust-version = "1.88"`.
**Five copies of one number is what F1 was.**

Make the workflows read it:

```yaml
- id: msrv
  run: echo "v=$(grep '^rust-version' Cargo.toml | cut -d'"' -f2)" >> "$GITHUB_OUTPUT"
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: ${{ steps.msrv.outputs.v }}
```

Same discipline as `validated_ceiling_display()` for the prikk ceiling and RFC 019's
`print_version_matrix` for the version matrix: **one source, read by everything that needs it.**

**If some workflow genuinely cannot do this**, say which and why rather than half-applying it — four
derived and one hardcoded is worse than five hardcoded, because it looks solved.

**One more copy, found reviewing `b0064b6`:** the root `Cargo.toml` now explains why MSRV is 1.88 in
**two** places — the `rust-version` block at lines 8–11, and again in the ratatui comment that commit
moved up from `stikk-tui`. Both are true; neither is load-bearing; they will drift the next time the
number moves. **Have the ratatui comment point at the `rust-version` rationale rather than restate it.**
Prose, not a functional copy — but it is this increment's own subject, in the file this increment is
about.

## 5. The changelog

`## Unreleased` currently has no entry for either of the two changes already on `main`. Both are
user-facing and both are **`### Breaking`** (RFC 011: for a 0.x crate the minor is the breaking
position, so these land in **0.5.0**):

- **MSRV raised 1.85 → 1.88.** Say *why* — clearing RUSTSEC-2026-0009, a parsing DoS in `time`, whose
  fix needs 1.88 — so a consumer pinned to an older toolchain can see what they are being asked to
  trade for.
- **ratatui 0.29 → 0.30.** A rendering-layer major. No stikk API changes from it, but say that
  explicitly rather than leaving a reader to infer it.

## 6. Gates

The five, plus the packaging check with its exclusion:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
```

## 7. Acceptance criteria

1. The sweep rule in the spec is the `git ls-files | xargs grep -ln` command, with exactly two named
   exceptions, and keeps RFC 018's "grep is a floor, not a ceiling" half.
2. `cargo-deny` runs in its own `workflow_dispatch`-only workflow — **no schedule, no PR trigger** —
   and is non-blocking.
3. `deny.toml` exists; every ignore entry has a dated reason and a removal condition.
4. The first run's result is reported, allowlist entries included, or reported clean.
5. Release prep requires a green, human-read supply-chain run; recorded in the spec.
6. Every workflow derives the MSRV from `Cargo.toml`, or names the one that cannot and why.
7. `## Unreleased` has `### Breaking` entries for the MSRV raise and the ratatui major, the MSRV one
   naming the advisory.
8. All gates green; no product behaviour changed.

## 8. Submit

Package to `.git-exclude/review-request/020-msrv-and-supply-chain/review-request-v1.md`.

**Lead with the first `cargo deny` run**, verbatim — clean or not.

**And tell me whether the derived-MSRV change actually removed a copy or just moved it.** If a workflow
ends up with a hardcoded fallback, that is a fifth copy wearing a disguise, and I would rather know than
find it the next time the number moves.

**Push once approved** (`.git-exclude/specs/02-implementer-handoff.md` §6).

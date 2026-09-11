# Handoff — 0.5.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The fourth release
prepared under it.
**Authorized by the project owner** 2026-09-12, on
[the 0.5.0 release proposal](../../../.git-exclude/release/0-5-0-proposal.md).
**Covers:** RFCs [019](../../done/019-the-real-binary-integration-suite.md),
[020](../../done/020-msrv-1-88-consistency-and-a-supply-chain-gate.md),
[021](../../accepted/021-prikk-0-38-rebaseline.md) A and B.

> **This release is the first whose prep depends on workflows that had never run.** `real-binary.yml`
> and `supply-chain.yml` are dispatch-only by ruling, and until today neither had executed on GitHub.
> **The architect dispatched both on current `main` when the proposal was authorized** — run
> `34655423803` (suite, full platform matrix) and `34655425444` (supply chain) — so that whatever the
> CI plumbing does, it is known before you start rather than discovered by you mid-prep. **Read those
> two runs first.** Their outcome decides whether §4 is a confirmation or a fix.
>
> **No code behaviour changes.** If a workflow leg fails, fixing the *harness or the workflow* is in
> scope; fixing *stikk* is not — stop and report.

---

## 1. Scope

**In**, in this order: the two first runs (§2); the changelog (§3); the platform-matrix finding, if
any (§4); the Breaking table (§5); version and lockfile (§6); final verification (§7).

**Out:** any product behaviour change; widening the suite (0.6.0's first increment); a `schedule:`
trigger on either workflow (the owner's, deliberately open); the key-id module and Glossary
wrap+scroll (moved to 0.6.0 — the proposal says so); the tag and publish.

## 2. The two first runs — read before anything else

```sh
gh run view 34655423803   # real-binary suite, full_platform_matrix: true
gh run view 34655425444   # supply-chain gate
```

**Report both verbatim in the review request — every job, every leg, the conclusion of each.** The
spec (§5b, §5c) requires a green run *read by a human*; you are that human for the prep, and I am for
the review. A pass/fail badge is not a reading.

**What each run exercises for the first time**, so you know what a failure would mean:

| First-time thing | Where it fails if it fails |
|---|---|
| the derived-MSRV step (`grep '^rust-version' Cargo.toml`) | every job's toolchain install — a wrong or empty value fails loudly at `dtolnay/rust-toolchain` |
| `cargo install prikk --version 0.28.0` / `0.38.0` on a cold runner | the suite's install step; slow, not wrong, unless crates.io is unreachable |
| `print_version_matrix` read inside a workflow | `determine-matrix`'s output — an empty `floor`/`ceiling` |
| **`openssl` on `windows-latest`** | the < 0.33 fixture path (`support.rs:266`, `:300`) — **unverified; the dev team flagged it in RFC 019 and nothing has run there since** |
| `EmbarkStudios/cargo-deny-action@v2` | the whole supply-chain job |

## 3. The changelog — two `### Security` entries, mine to have caught

`## Unreleased` has the MSRV/ratatui `### Breaking`, the range `### Changed`, and F0 under
`### Fixed`. **RFC 019 and RFC 020's gate have no entry.** Add a `### Security` section, the way 0.3.0
recorded fixture re-capture, with two entries written from what shipped:

- **The real-binary integration suite** (RFC 019). Say what it drives — commit, seal, orientation,
  history against real prikk 0.28 and 0.38 — that it asserts repository *state* after a mutation, that
  it confirms the two client-side refusals stikk prevents are refusals prikk gives, and **that it covers
  those four surfaces and not the rest.** A consumer reading a Security entry to decide what to trust
  needs the boundary as much as the claim.
- **The supply-chain gate** (RFC 020 F4) — `cargo-deny` over advisories and licenses, non-blocking by
  ruling, required-and-read at release. Name `RUSTSEC-2026-0009` as the advisory that motivated it and
  say it was found by a person. And the sweep-as-command and derived-MSRV changes belong here too, as
  one line: the number now has one source.

**Then re-read the top of `## Unreleased`.** It currently has no summary paragraph. Write one — the
release is *stikk is checked*, and the four increments in one sentence each. It becomes
`## 0.5.0 — <date>` in §6.

## 4. If the platform matrix found something

**Most likely: `openssl` absent or differently named on `windows-latest`.** If so, the fix is in the
harness (`stikk-real-binary`), not in stikk, and two shapes are acceptable:

- an explicit install/setup step in the workflow's Windows leg, **or**
- deriving the Ed25519 public key in Rust rather than shelling out — but only if a dependency is not
  needed for it; this project has taken no external crate for test convenience and that decision
  stays with the architect.

**Say which, and why.** Then re-dispatch the suite with the full matrix and report the second run.

**If the failure is anything else** — the MSRV derivation, the matrix output, the install — report it
before fixing: those are RFC 019/020's workflow design, and a fix there is a review conversation, not a
prep task.

**If both runs were green: say so, and say what that proves and does not.** Green on four surfaces at
two versions on three platforms is a real result; it is not coverage of the other five surfaces.

## 5. The Breaking table

From `git diff 0.4.1..HEAD -- 'crates/*/src'`, per RFC 011's rules. **My reading: one additive
function (`supported_minor_range`), zero struct or trait changes.** Verify it; report what you rejected
and why, as 0.4.0's prep did. The MSRV raise and the ratatui major stay as the two rows that force the
minor.

## 6. Version and lockfile

- `[workspace.package] version` and the **five** `[workspace.dependencies]` pins: `0.4.1` → `0.5.0`.
- `cargo update --workspace`. Not a `--locked` build.
- `## Unreleased` → `## 0.5.0 — <the date you land it>` (em dash; the file's convention).
- **The sweep**: `git ls-files | xargs grep -ln "0\.4\.1"`. Every live statement moves; `CHANGELOG.md`'s
  released sections and `Cargo.lock` are the two exceptions. Read comments and examples as claims.

## 7. Final verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
cargo deny check
```

**Plus both workflows green on the final prep commit** — if §4 changed the harness or the workflow,
re-dispatch after the version bump so the run that satisfies §5b/§5c is on the commit that gets tagged.
State the run IDs.

## 8. Acceptance criteria

1. Both first runs reported verbatim, per job and per leg.
2. `### Security` with the suite and the gate, the suite's coverage boundary stated.
3. A release summary paragraph at the top of the section.
4. Any matrix failure diagnosed, fixed in harness/workflow only, re-run, and reported — or both runs
   green and their scope stated plainly.
5. Breaking table verified from the diff; rejected candidates listed.
6. Version `0.5.0` in all six places; lockfile refreshed; heading moved; `0.4.1` sweep clean.
7. All eight commands green; both workflows green on the final commit, run IDs named.
8. **No stikk behaviour changed.**
9. Nothing tagged or published.

## 9. Submit

Package to `.git-exclude/review-request/011-release-0-5-0-preparation/review-request-v1.md`.

**Lead with the two first runs** — they are the thing this release's prep exists to produce. Then the
changelog in full.

**And tell me what running on three platforms found that running on one never had.** The Windows leg
is the obvious candidate, but the macOS leg has never run either, and "it passed" on a platform nothing
had exercised is a fact worth stating on its own.

The tag and the publish are the owner's to authorize and mine to perform.

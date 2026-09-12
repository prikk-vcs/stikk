# Handoff — 0.6.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The fifth release
prepared under it.
**Authorized by the project owner** 2026-09-13, on
[the 0.6.0 release proposal](../../../.git-exclude/release/0-6-0-proposal.md) — read it first; this
handoff does not repeat its reasoning.
**Covers:** RFCs [022](../../done/022-widening-the-real-binary-suite.md),
[023](../../done/023-what-stikk-has-and-does-not-show.md) A and B,
[024](../../done/024-overlay-sizing-and-the-fix-that-did-not-travel.md), and
[026](../../done/026-readiness-after-the-key-directory.md) A, B and C.

> **This release's prep is mostly text, and the text is what ships.** `release.yml` extracts the
> `## 0.6.0` section of `CHANGELOG.md` — everything from that heading to the next `## ` — verbatim into
> the GitHub Release body. **Whatever §9 produces is the first thing a user reads.** Today that section
> describes six increments in the order they landed, and later entries contradict earlier ones.
> **§9 matters most; it comes late only because it describes what §3–§8 leave behind.**
>
> **Both release-required workflows are already known green on current `main`:** the real-binary suite
> `34708246818` at `9d01866` (full matrix, 14/14), and the supply-chain gate `34718879108` at `883e849`,
> which I dispatched at authorization — advisories, bans, licenses and sources all ok. Nothing is known
> to be broken. §3–§5 change a test, an API and a workflow, so both re-run at your final commit.
>
> **Sanctioned beyond text, and only these:** one real-binary test (§3), one dead public type removed
> (§4), one action version (§5). Anything else that changes product behaviour — stop and report.

---

## 1. Scope

**In**, in the order to do it: the gate rule (§2); the `readiness` test (§3); removing
`MaintainerReadiness` (§4); the release action (§5); a guard's stale documentation (§6); `ROADMAP.md` and
`releasing.md` (§7); the Breaking table (§8); **the changelog (§9)**; heading and sweeps (§10); final
verification (§11).

**Out:** any product behaviour change; the four unbuilt views; `refused paths:`; `docs.yml`'s three
node20 actions (carried to 0.7.0); **`#[non_exhaustive]` on any type** — RFC 011 defers that as a
1.0-readiness task, and a release-prep commit is not where it changes; a `schedule:` trigger (the
owner's); the tag and the publish.

## 2. The gates, and where the runs come from

**This is the first release prepared under the toolchain rule RFC 026 B produced** — read
`.git-exclude/specs/02-implementer-handoff.md`, *"The gate set is the MSRV toolchain's, and
`cargo package` can lie"*, §1–§4, before running anything.

Gates 1–5, 7 and 8 on the MSRV, which CI enforces — read from `Cargo.toml`, never restated:

```sh
MSRV=$(sed -n 's/^rust-version *= *"\(.*\)"$/\1/p' Cargo.toml)
cargo +"$MSRV" fmt --all -- --check
cargo +"$MSRV" clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +"$MSRV" test --workspace --locked
cargo +"$MSRV" build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo +"$MSRV" doc --no-deps --workspace --locked
mdbook build docs
cargo deny check
```

Gate 6 on **stable**, with a fresh target directory:

```sh
CARGO_TARGET_DIR="$(mktemp -d)" cargo package --workspace --exclude stikk-real-binary --locked
```

It cannot run on the MSRV while sibling versions are unpublished. That is expected; do not report it
as a failure.

**Where the runs come from.** `main` is not yours to push until review approves. Use the sanctioned
practice in the same spec's §6: push the final commit to `prep/release-0-6-0` (CI triggers on every
push), then

```sh
gh workflow run real-binary.yml  --ref prep/release-0-6-0 -f full_platform_matrix=true
gh workflow run supply-chain.yml --ref prep/release-0-6-0
```

**Read every job and every leg**, delete the branch before submitting, and say so. **The runs must be
on the byte-identical tree that lands** — land by fast-forward, so the SHA the three runs name is the
SHA that gets tagged. If anything changes after the runs, they run again.

## 3. The `readiness` test

RFC 026 B added an eleventh `Prikk` method. The suite calls ten of eleven directly:

```
handshake 1 · orientation 4 · history 2 · block_state 1 · refs 2 · tags 2
worktree_status 3 · change_token 4 · commit 14 · seal 9 · readiness 0
```

`readiness` is exercised only through the re-check inside `commit` and `seal` — a gate, never an answer
anyone asserts. **It is the method this release is about.** Add one test to `tests/real_binary.rs`, in
the shape of `the_three_reports_parse_at_both_ends_json_above_prose_below`: under `ENV_LOCK`,
`Fixture::clear_env()` at entry and exit, both binaries, seeding through raw prikk.

| binary | configured as | assert |
|---|---|---|
| floor (≤ 0.39 band) | `set_author_env` + `set_maintainer_env` | both roles `RoleReadiness::Unknown` |
| floor | after `clear_env` | both `RoleReadiness::NotReady` — deterministic here: this band has no key directory |
| ceiling (≥ 0.41 band) | both set, fresh fixture | `author` is `Known(Binding::Unrecorded)`, `maintainer` is `Known(Binding::Matches)`; `report.author.key_id` is `Some("author")` and `key_id_source` is `Some("environment")` |
| ceiling | after one raw `prikk commit` as the author | `author` is `Known(Binding::Matches)` |

**Those expectations come from prikk's letter 007 measurement and RFC 026 F4, not from a run of mine.**
If the binary answers differently, **report it — do not adjust the assertion to match.**

**At the ceiling, cross-check against prikk itself**: run `prikk key status --format json` raw in the
same test and assert its `binding` strings agree with what stikk read. The suite's doctrine is that it
asserts what prikk says; a constant stikk agrees with itself about is not that.

### The trap: never assert the unconfigured state at 0.41 against a real key directory

prikk 0.41 resolves its key directory as `$XDG_CONFIG_HOME/prikk`, else `$HOME/.config/prikk` on Unix,
and `%APPDATA%\prikk` on Windows (`crates/prikk-cli/src/key_material.rs` at the `0.41.0` tag).
**`clear_env` touches none of those.** On a CI runner the directory is absent and a "not ready"
assertion passes; on a developer machine that has run `prikk setup`, the test reads that developer's
real keys and passes or fails by laptop. That is not a test.

**A `NotReady` at the ceiling is still worth having** — it is the only place `reason` would be asserted
verbatim against a real binary. Make it deterministic with prikk's own rule from that same file: **an
override that is set always wins, even when the file is missing.** Point `PRIKK_AUTHOR_SEED_FILE` at a
path under the fixture's temp root that does not exist (nothing is written), and assert `author` is
`NotReady` with `report.author.reason.as_deref() == Some("override-missing")`. No key directory is
consulted.

That is one more environment write, so one more `#[allow(unsafe_code)]` site. Follow the existing
pattern at `the_full_queue_precondition_classifies_refusal_at_both_ends` (`tests/real_binary.rs:856–867`)
— a `SAFETY:` comment naming `ENV_LOCK`, the variable removed before the lock is released — and read §6
before you write it.

**Result:** the suite drives eleven of eleven, and §9 says so because it is true.

## 4. Remove `MaintainerReadiness`

I measured zero uses in code or tests: it is defined at `crates/stikk-model/src/capability.rs:120` and
re-exported at `crates/stikk-model/src/lib.rs:30`, and nothing else touches it. **Re-measure before
deleting** — `git ls-files | xargs grep -n MaintainerReadiness`.

- Delete the enum and its doc comment; drop it from the re-export.
- **`capability.rs:143` is an intra-doc link** — `[`MaintainerReadiness`]` in `Readiness`'s doc. Gate 5
  fails if it is left. `capability.rs:16` names it in prose as history; keep or reword it, but it must
  not read as though the type still exists.
- **`requirements.md`'s `FR-103`** names it inside an amendment chain, as the record of what was true at
  0.28–0.33. **Add one clause** saying RFC 026 replaced it with `RoleReadiness` in 0.6.0. Do not rewrite
  the history.
- Leave `rfcs/` and the released changelog sections alone.

Why now, in one line: the `### Breaking` entry already says it "becomes `RoleReadiness`"; removal is free
in a breaking release and forces 0.7.0 later. It is a row in §8.

## 5. The release action

`softprops/action-gh-release@v2` → `@v3`, at both uses in `release.yml`: the `release` job's *Create or
update the release*, and the `binaries` job's *Upload to the release*.

**What I have checked:** v3.0.0's release notes say the runtime moving from Node 20 to Node 24 is the
only change, and that `v2` stays on the latest 2.x line (they name `v2.6.2`). Every input `release.yml`
sets — `tag_name`, `name`, `body_path`, `prerelease`, `generate_release_notes`, `files` — exists in
v3.0.3's `action.yml`.

**What I have not checked, and you do:** the defaults of the inputs `release.yml` does *not* set
(`make_latest`, `draft`, `overwrite_files`, `fail_on_unmatched_files`, `target_commitish`, …). Diff them:

```sh
diff <(gh api 'repos/softprops/action-gh-release/contents/action.yml?ref=v2.6.2' --jq .content | base64 -d) \
     <(gh api 'repos/softprops/action-gh-release/contents/action.yml?ref=v3.0.3' --jq .content | base64 -d)
```

**This change cannot be exercised before the tag, and you must not try.** `release.yml` runs only on a
version tag, and tags are the owner's. The review evidence is that comparison, not a run. **If a default
`release.yml` relies on has changed, stay on v2 and report why** — the proposal's fallback.

Why it matters: the `release` and `crates` jobs both need only `guard` and `verify`, and run in
parallel. A failure in this step at tag time still publishes to crates.io and produces no GitHub Release
and no binaries — recoverable by re-running, not by undoing.

## 6. A guard's documentation that stopped being true

`crates/stikk-real-binary/src/lib.rs:26–27`:

> *"`#[allow(unsafe_code)]` on exactly the three `set_var`/`remove_var` call sites in [`support`] that
> need it — a fourth `unsafe` anywhere else in this crate still fails the build."*

**There are five**: `src/support.rs:524`, `:539`, `:560`, and `tests/real_binary.rs:858`, `:864`. The
last two landed in `8221c15` (RFC 022, 2026-09-12). The sentence was written in `561c05a` (2026-09-07)
and was true then. **Both newer sites are sound** — each carries its `SAFETY:` comment, runs under
`ENV_LOCK`, and removes what it set — so nothing is wrong with the code. What is wrong is a count that
drifted, and "a fourth still fails the build" describes the mechanism slightly wrong: review C2 chose
`deny` precisely because, unlike `forbid`, it can be overridden, so what fails is an `unsafe` **without**
its own `#[allow]`. **I reviewed RFC 022 and did not catch it.**

**State the property instead of a count**: every `unsafe` in this crate carries its own
`#[allow(unsafe_code)]` and a `SAFETY:` comment naming `ENV_LOCK`. Then give the command that
enumerates the sites — `grep -rn 'allow(unsafe_code)' crates/stikk-real-binary` — so the doc cannot drift
again. `Cargo.toml:24`'s comment says the same thing ("in `src/support.rs`") and moves with it.

**It must be true after §3.** If you take the missing-override case, that is a sixth site.

## 7. `ROADMAP.md` and `releasing.md`

**`ROADMAP.md` is two releases stale**, and some of it is now false:

- *"Next — responsive & correct (0.3.0, breaking)"* and *"Then — the working cycle (0.4.0)"* present
  shipped releases as future; there is no 0.5.0 or 0.6.0 section.
- *Later → Trust & keys* says the key-id display module is **"still unbuilt"** (RFC 023 B built it) and
  readiness is **"still presence-only"** (at ≥ 0.41 it asks `key status`, RFC 026 B).
- The upstream table's `UD-02` row says machine-readable output exists **"only on `verify`"**. Cite the
  real history from the design record rather than from me: `requirements.md`'s `ASM-2` records
  `status --format json` at 0.35 and `show --format json` at 0.36, and RFC 026 adds `log`/`branch`/`tag`
  at 0.39 and `key status` at 0.41.
- Its `UD-09` row says **"narrowed, not retired, at prikk 0.32"**. Its content half retired at 0.36
  (RFC 021 F1; `FR-034` records it).

**Fix what is false and record what shipped.** Add 0.7.0's carried order, from the proposal's last
section, as the forward section — the roadmap is where order lives. Do not re-plan *Later*.

**`docs/src/contributing/releasing.md`:** *"What a v0.4.x release is (and is not)"* still describes 0.4.x
as current — bring it to 0.6.x, and re-verify its *"Merge, sync, tag create, and branch create/close
remain unbuilt"* rather than carrying it. Its *"match the file's own three released sections"* — there
are six; drop the count.

## 8. The Breaking table

**My reading**, from `git diff 0.5.0..HEAD` over non-test sources, filtered to `pub` lines:

| Crate | Change |
|---|---|
| `stikk-model` | `Readiness`'s `author_ready`/`maintainer_readiness` become `author`/`maintainer: RoleReadiness` |
| `stikk-model` | `MaintainerReadiness` removed (§4) |
| `stikk-prikk` | `Prikk` gains `readiness(&self, repo)` |
| `stikk-core` | `present()` gains `prikk_minor: Option<u32>` |
| `stikk-core` | `ConfirmationSummary` gains `signing_key_id`, `signing_key_claim`, `signing_key_is_published_example`; `OrientationView` gains `prikk_minor`, `stale_seed_variables` — breaking for struct-literal construction |

**A line grep catches signatures and fields, not a changed trait bound or a behaviour change behind an
unchanged signature.** Verify against the diff, and **list the candidates you rejected and why**, as
0.4.0's and 0.5.0's prep did. The ones I read as additive, for you to confirm or overturn:
`stikk_tui::text::wrap_indented` (newly public), `NullBackend::with_readiness`, the `key_id` module,
`ReadinessReport`, `RoleDetail`, `StaleSeedVariables` and `stale_seed_variables`, `example_keys` with
`ExampleKey` and `published_example`, `KeyClaim` and `signing_key_claim`. `RoleReadiness` and `Binding`
are new types; the break is `Readiness`'s fields changing to use them.

**§9's `### Breaking` matches this table exactly.**

## 9. The changelog — one release, not six increments

**Structure.** A bold summary paragraph first, the way 0.5.0's section opens (*"**stikk is checked.**"*):
the release is *stikk says what it knows*, with each increment named once. Then **one heading of each
kind**, in this order: `### Breaking`, `### Added`, `### Changed`, `### Fixed`, `### Security` — 0.5.0's
order, with `### Added` after `### Breaking` where 0.4.0 put it. **No `## ` line inside the section** —
extraction stops at the first one.

**Every entry describes 0.6.0's end state.** The intermediate states belong to the RFCs, which already
record them. These are the entries I found superseded while writing the proposal:

| Entry today (from) | Says | What 0.6.0 actually does |
|---|---|---|
| `### Security` (023) | `C-S2` is **marked not implemented** | implemented (026 B) — **one entry**, with its scope limit: documentation examples, not prikk's tests or issue comments |
| `### Added` key id (023 B) | id read from `PRIKK_*_KEY_ID`; **"`env.rs` is untouched"**; absent renders as nothing | at ≥ 0.41 read from `key status`; tells a bound key from an unbound one from an unchecked one; flags a published example key |
| `### Added` backslash gloss (023) | chosen by platform | chosen by platform **and** prikk version (026 C) |
| `### Fixed` seal affordance (024) | `MaintainerReadiness::Unknown` is "the only value any supported prikk can produce" | false at ≥ 0.41, and the type is gone after §4 |
| `### Changed` suite (022) | "ten of the ten" seam methods | **eleven of eleven** after §3 |

**The paragraph correcting 0.5.0's released coverage claim stays** — it corrects something users already
have. It says 0.5.0 shipped five of ten, which is true; it must not end up beside a "ten of the ten".

**Placement of the new items:** §4's removal under `### Breaking`; §5's action bump under `### Changed`,
where 0.5.0 recorded the `actions/checkout` pin. **§6 gets no entry** — `stikk-real-binary` is not
published.

**Re-check every number against the tree, not against the old entries:** seam methods, example keys
(two), the validated range (`>= 0.28`, through `0.41.0`), and the prikk version named beside each
feature.

**Then read it from the top as a user who has 0.5.0 and is deciding whether to upgrade.** That is who
reads the GitHub Release, and none of the six reviews that approved these entries read it that way —
mine included.

## 10. Heading and sweeps

- `## Unreleased` → `## 0.6.0 — <the date you land it>` (em dash, the file's convention). The extraction
  step matches `## 0.6.0` followed by a non-digit.
- **No version bump.** `[workspace.package] version` and the five pins have been `0.6.0` since `8acfed6`.
  Do not run `cargo update`; the `--locked` gates prove the lockfile agrees.
- **Sweeps**, each `git ls-files | xargs grep -ln "<pattern>"` — grep is a floor, not a ceiling; read
  comments and examples as claims:
  - `0\.5\.0` — I ran it for the proposal and every hit was history or third-party (`heck 0.5.0`).
    **Re-run it**, because §7 and §9 add text.
  - `MaintainerReadiness` — after §4, only `rfcs/`, released changelog sections, and `FR-103`'s clause.
  - `validated through` — every live statement says `0.41.0`.
  - `ten of the ten` / `ten seam` — no live claim of ten after §3.

## 11. Final verification

1. §2's gates at the final commit.
2. That commit pushed to `prep/release-0-6-0`; `CI` runs; both workflows dispatched; every job and leg
   read; the branch deleted, and the request says so.
3. **Three run ids at one SHA** in the gate table — `CI`, the real-binary suite (full matrix), the
   supply-chain gate — and that SHA is the one that lands on `main` by fast-forward.

## 12. Acceptance criteria

1. Gates 1–5, 7 and 8 green on the MSRV; gate 6 green on stable with a fresh target directory.
2. `CI`, the suite (full matrix) and the supply-chain gate green at one SHA, every job and leg read, run
   ids named; the `prep/` branch deleted and said so.
3. A real-binary test drives `Prikk::readiness` at both ends and asserts the band at each, cross-checked
   against prikk's raw `key status` at the ceiling. **No assertion depends on a real key directory.**
4. `MaintainerReadiness` removed; rustdoc clean; `FR-103` records what replaced it.
5. `action-gh-release` at v3 in both uses, with the `action.yml` default diff reported — or kept at v2
   with the reason.
6. `lib.rs` and `Cargo.toml` state the `unsafe` property and the command that enumerates the sites, true
   after §3.
7. `ROADMAP.md` and `releasing.md` make no false statement about what has shipped; 0.7.0's order is
   recorded.
8. Breaking table verified from the diff, rejected candidates listed, matching `### Breaking`.
9. `## 0.6.0 — <date>`: one summary paragraph, one heading of each kind, every entry the end state,
   every number re-checked.
10. Sweeps clean; no version bump.
11. **No product behaviour changed** beyond §3–§5's sanctioned items.
12. Nothing tagged or published.

## 13. Submit

Package to `.git-exclude/review-request/011-release-0-6-0-preparation/review-request-v1.md`.

**Lead with the `## 0.6.0` section in full, exactly as `release.yml` will extract it.** I will read it
as a user who has 0.5.0 before I read it as a reviewer.

Then the `readiness` test's assertions at both ends, with prikk's raw `key status` output beside them.
Then the three run ids at one SHA.

**And tell me what reading the section as a whole found that six separate reviews did not.** I found five
superseded entries while writing the proposal; I would be surprised if that is all of them.

**Push your own commits once approved.** The tag and the publish are the owner's to authorize and mine to
perform.

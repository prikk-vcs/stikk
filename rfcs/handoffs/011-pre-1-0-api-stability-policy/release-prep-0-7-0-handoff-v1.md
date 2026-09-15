# Handoff — 0.7.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The sixth release prepared under it.
**Issued** 2026-09-15, on the project owner's word that the dev team may help with release prep. The tag and
publish still wait for the owner's authorization, on
[the 0.7.0 release proposal](../../../.git-exclude/release/0-7-0-proposal.md). **Read the proposal first**; this
handoff does not repeat its reasoning.
**Covers:** RFCs [027](../../done/027-what-commit-would-refuse.md) A and B,
[028](../../done/028-the-queue-view.md) A and B,
[029](../../done/029-prikk-0-42-rebaseline-and-the-current-branch.md) A and B, and
[030](../../done/030-a-confirmed-commit-authors-the-worktree-it-previewed.md).

> **This release's prep is text, and the text is what ships.** `release.yml` extracts the `## 0.7.0` section of
> `CHANGELOG.md` — everything from that heading to the next `## ` — verbatim into the GitHub Release body. **Today
> that section is seven increments in the order they landed, with no summary, and its lead fix buried in
> `### Fixed`.** §3 matters most.
>
> **Two pages users see first say false things about stikk today.** `crates/stikk/README.md` is the page
> crates.io shows, and it says *"v0.4.0 is where stikk writes"*. The docs site's landing page says the same, and
> lists the Queue view as unbuilt. §4.
>
> **Sanctioned: text only.** Anything that changes product behaviour, a test's assertion, or a workflow —
> **stop and report.**

---

## 1. Scope

**In, in the order to do it:**
1. The gates and where the runs come from (§2).
2. **The changelog (§3).**
3. User-facing text 0.7.0 makes false (§4).
4. `ROADMAP.md` (§5).
5. The Breaking table, verified once for the whole release (§6).
6. The heading and sweeps (§7).
7. Final verification (§8).

**Out:**
- any product behaviour change;
- Patch detail, Compare, and every other *carried* item in §5;
- **`#[non_exhaustive]` on any type** (RFC 011 defers it as a 1.0-readiness task);
- `docs.yml`'s node20 actions (carried);
- a `schedule:` trigger (the owner's);
- letter 012 (the owner's);
- the tag and the publish.

## 2. The gates, and where the runs come from

**Unchanged from 0.6.0's prep.** Read `.git-exclude/specs/02-implementer-handoff.md`, *"The gate set is the MSRV
toolchain's, and `cargo package` can lie"*, §1–§4, before running anything.

Gates 1–5, 7 and 8 on the MSRV, which is read from `Cargo.toml` and never restated:

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

**Where the runs come from.** Push the final commit to `prep/release-0-7-0`, where `CI` triggers, then:

```sh
gh workflow run real-binary.yml  --ref prep/release-0-7-0 -f full_platform_matrix=true
gh workflow run supply-chain.yml --ref prep/release-0-7-0
```

- **Read every job and every leg.**
- **Delete the branch before submitting, and say so.**
- **The three runs must be on the byte-identical tree that lands**, landed by fast-forward, so the SHA they name
  is the SHA that gets tagged.
- **If anything changes after the runs, they run again.**

**Supply chain matters here even with no dependency change**: its last run was on a prep branch today, and the
advisory database moves.

## 3. The changelog — one release, not seven increments

**Structure.**
1. **A bold summary paragraph first**, as 0.6.0's opens (*"**stikk says what it knows.**"*). The proposal's line is
   *"stikk shows what a change will do — and no longer makes a change it did not show"*. Name each RFC once, and
   say what a user will notice.
2. **One heading of each kind, in this order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`,
   `### Security`, which is 0.6.0's order.
3. **No `## ` line inside the section.** Extraction stops at the first one.

**`### Security` — RFC 030's fix moves here**, out of `### Fixed`:
- **Why:** it is threat model `T-T4` exactly. Before 0.7.0, a commit could be authored and signed with the user's
  AUTHOR key holding content the preview never showed.
- **Keep its measured basis:** a file added, at prikk 0.28 and 0.42; a branch switch and a `prikk mv`, at 0.42.
- **Keep its two stated limits.**
- **Add one clause:** a change outside stikk between preview and Enter now makes the confirmation stale, so a user
  previews again.

**Every entry describes 0.7.0's end state.** Each was approved on its own terms in a separate review, and none of
those reviews read the section as a whole. Things to check, not a complete list:
- the RFC 027 entries say *"at prikk ≥ 0.39"* and *"below 0.39"* consistently with RFC 029's 0.42 entries;
- the three RFC 029 `### Changed` entries do not repeat each other;
- the status-bar entry's shedding description matches what shipped (hint, then repository name, then prikk's
  value, then the segment). It currently names only the hint;
- nothing says *"another writer"* except where it quotes what changed.

**If the section states suite coverage**, it is **twelve** `Prikk` methods, counted:
- `queue` is called directly;
- `handshake` is driven by the version guard every test runs through, as 0.6.0 recorded.

The 0.6.0 section's *"eleven"* stays, because it is released.

**Then read it from the top as a user who has 0.6.0 and is deciding whether to upgrade.** That is who reads the
GitHub Release.

## 4. User-facing text that 0.7.0 makes false

**Fix what is false; do not re-plan or add marketing.** Found by the architect; grep is a floor.

| Where | Says today | Becomes, in substance |
|---|---|---|
| **`crates/stikk/README.md`, `## Status`** (crates.io shows this) | *"**v0.4.0 is where stikk writes.**"* | a 0.7.x statement: what it reads and does, including the Queue view, and the confirmation that commits only what it showed |
| **`docs/src/index.md`, `## Status`** | *"**0.4.0 is where stikk writes.**"*; *"a **Queue view** … named gaps"*; Patch detail *"deferred behind `UD-09` — prikk exposes no per-patch content yet"* | 0.7.x; the Queue view is built; **Patch detail is unbuilt, and prikk ≥ 0.36 has `show`**, so the reason must not be prikk's |
| **`README.md`**, the TUI paragraph | *"more views (History, Patch detail) follow"*; *"stikk reads `PRIKK_*_SEED` **presence only**"* | History shipped long ago, so name what exists; signing readiness comes from `prikk key status` at ≥ 0.41 and presence below, as `getting-started.md`'s table already says |
| **`docs/src/guide/getting-started.md`** | the key reference has no `Q`; nothing says where stikk opens; *"**Patch detail** is deferred behind `UD-09` (prikk exposes no per-patch content yet)"* | add `Q` (the Queue view); say where stikk opens by prikk version (RFC 029's `FR-055` amendment); correct Patch detail's reason |
| **`docs/src/contributing/releasing.md`** | *"What a v0.6.x release is (and is not)"* | v0.7.x. **Re-verify** *"Merge, sync, tag create, and branch create/close remain unbuilt"* rather than carrying it |

**Read `README.md` and the guide end to end** for anything else a 0.7.0 user would find untrue.

## 5. `ROADMAP.md`

**Record what shipped, and set the forward order the proposal gives.**

1. **Add `## Shipped — <0.7.0's summary line> (0.7.0, breaking)`** after the 0.6.0 section, in its style: one bullet
   each for RFCs 027, 028, 029 and 030, linking `rfcs/done/`.
2. **Rename `## Next — carried into 0.7.0, in order` to `… 0.8.0 …`.**
   - Remove items 1, 2, 9 and 10, which are shipped.
   - Re-order what remains **exactly as the proposal's last section**: `FR-106`'s notice; the preview showing
     renames; Patch detail then Compare; the unpublished-baseline words; prose parse-failure attribution; the
     overlay scroll and row budget; the seed-read guard; `docs.yml`'s actions.
   - **Keep each item's existing text**, trimming only what §1–§4 of this handoff made false.
3. **Do not re-plan `Later`.**

## 6. The Breaking table, verified once for the whole release

The 23 rows were each built by API diff in their own review. **Read `git diff 0.6.0..HEAD` over every non-test
source file once, as one release**, by the spec's five categories: signatures, enum variants, trait methods,
struct fields and `pub use`, and removed items.

- **Group the table by crate**, in dependency order: `stikk-model`, `stikk-prikk`, `stikk-core`, `stikk-tui`.
- **Merge rows that describe one type across increments.** For example:
  - `ChangesView` (RFC 027 and 030);
  - `ConfirmationSummary` (RFC 029 and 028);
  - `CommitPreviewOutcome` (RFC 027's variant and RFC 030's token);
  - `Overlay` (RFC 027, 029 and 030);
  - `WorktreeStatus` (RFC 027 and 030).
- **Where one increment's change was superseded by a later one** in this release, the table states the end
  state, not both. Example: `Focus::Queue` first carried a view, then a view and an offset.
- **List the candidates you rejected and why**, as every prep has. Check `NO_FOCUSED_REF_REASON` (value changed, type
  not) and `OperationContext::LoadQueue` (`#[non_exhaustive]`).
- **The additive paragraph** keeps every new public item, grouped the same way.

**§3's `### Breaking` matches this table exactly.**

## 7. Heading and sweeps

- **`## Unreleased` → `## 0.7.0 — <the date you land it>`** (em dash, the file's convention).
- **No version bump.** `[workspace.package] version` and the five pins have been `0.7.0` since `94a7315`. Do not run
  `cargo update`.
- **Sweeps**, each `git ls-files | xargs grep -ln "<pattern>"`. **Read comments and examples as claims.**
  - `0\.6\.0` — live statements of the *current* version move. History, released sections and RFCs stay.
  - `0\.4\.0 is where` and `v0\.4` — after §4, none live.
  - `validated through` — every live statement says `0.42.0`.
  - `no per-patch content` — none live.
  - `Queue view` — nothing live calls it unbuilt.
  - `eleven` — only history (the 0.6.0 section and the roadmap's shipped 0.6.0 bullet).

## 8. Final verification

1. §2's gates at the final commit.
2. That commit pushed to `prep/release-0-7-0`; `CI` runs; both workflows dispatched; every job and leg read; the
   branch deleted, and the request says so.
3. **Three run ids at one SHA** — `CI`, the real-binary suite (full matrix) and the supply-chain gate — and that
   SHA is the one that lands on `main` by fast-forward.
4. **`release.yml`'s own `awk`**, run locally on the final `CHANGELOG.md`, extracts the section. Report its line
   count and its headings.

## 9. Acceptance criteria

1. Gates 1–5, 7 and 8 green on the MSRV; gate 6 green on stable with a fresh target directory.
2. `CI`, the suite (full matrix) and the supply-chain gate green at one SHA, every job and leg read, run ids
   named; the `prep/` branch deleted, and said so.
3. **`## 0.7.0 — <date>`:** a summary paragraph, one heading of each kind in §3's order, RFC 030's fix under
   `### Security` with its limits, every entry the end state.
4. **§4's five files** say nothing false about 0.7.0; crates.io's README and the docs landing page describe 0.7.x.
5. **`ROADMAP.md`** records 0.7.0 as shipped and 0.8.0's order exactly as the proposal gives it.
6. **The Breaking table** verified once over `0.6.0..HEAD`, grouped by crate, rows merged, rejected candidates
   listed, matching `### Breaking`.
7. **Sweeps clean; no version bump.**
8. **No product behaviour, test assertion or workflow changed.**
9. **Nothing tagged or published.**

## 10. Submit

Package to `.git-exclude/review-request/011-release-0-7-0-preparation/review-request-v1.md`.

**Lead with the `## 0.7.0` section in full, exactly as `release.yml` will extract it.** I will read it as a user
who has 0.6.0 before I read it as a reviewer.

**Then:**
1. **`crates/stikk/README.md` as crates.io will show it.**
2. **The Breaking table's rejected candidates.**
3. **The three run ids at one SHA.**

**And tell me what reading the section as a whole found that seven separate reviews did not.**

**Push your own commits once approved.** The tag and the publish are the owner's to authorize and mine to
perform.

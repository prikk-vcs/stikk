# Handoff — 0.8.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The seventh release prepared under it.
**Issued** 2026-09-16, on the project owner's authorization of the cut and their word that the dev team may do the
prep. **The tag and the publish still wait for the owner**, on
[the 0.8.0 release proposal](../../../.git-exclude/release/0-8-0-proposal.md). **Read the proposal first**; this
handoff does not repeat its reasoning.
**Covers:** RFCs [031](../../done/031-seeing-a-change-made-outside-stikk.md) and
[032](../../done/032-what-a-commit-will-author.md).

> **This release's prep is text, and the text is what ships.** `release.yml` extracts the `## 0.8.0` section of
> `CHANGELOG.md` — everything from that heading to the next `## ` — verbatim into the GitHub Release body. **Today
> that section is two increments in landing order, with no summary.** §3 matters most.
>
> **The page crates.io serves still describes v0.7.x**, as do the docs landing page and the releasing guide. §4.
>
> **Sanctioned: text only.** Anything that changes product behaviour, a test's assertion, or a workflow —
> **stop and report.**

---

## 1. Scope

**In, in the order to do it:**
1. The gates and where the runs come from (§2).
2. **The changelog (§3).**
3. User-facing text 0.8.0 makes false (§4).
4. `ROADMAP.md` (§5).
5. The Breaking table, verified once for the whole release (§6).
6. The heading and sweeps (§7).
7. Final verification (§8).

**Out:**
- any product behaviour change;
- Patch detail, Compare, and every other *carried* item in §5;
- **the prikk 0.43.0 re-baseline** — 0.43 is not published, and **no 0.43 field may be read or named as present**;
- **`#[non_exhaustive]` on any type** (RFC 011 defers it as a 1.0-readiness task);
- `docs.yml`'s node20 actions (carried);
- a `schedule:` trigger (the owner's);
- letter 013, which is the owner's to send;
- the tag and the publish.

## 2. The gates, and where the runs come from

**Unchanged from 0.7.0's prep.** Read `.git-exclude/specs/02-implementer-handoff.md`, *"The gate set is the MSRV
toolchain's, and `cargo package` can lie"*, §1–§4, before running anything.

Gates 1–5, 7 and 8 on the MSRV, read from `Cargo.toml` and never restated:

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

**Where the runs come from.** Push the final commit to `prep/release-0-8-0`, where `CI` triggers, then:

```sh
gh workflow run real-binary.yml  --ref prep/release-0-8-0 -f full_platform_matrix=true
gh workflow run supply-chain.yml --ref prep/release-0-8-0
```

- **Read every job and every leg.**
- **Delete the branch before submitting, and say so.**
- **The three runs must be on the byte-identical tree that lands**, landed by fast-forward, so the SHA they name is
  the SHA that gets tagged.
- **If anything changes after the runs, they run again.**

**Supply chain matters even with no dependency change**: the advisory database moves.

## 3. The changelog — one release, not two increments

**Structure.**
1. **A bold summary paragraph first**, as 0.7.0's opens. The proposal's line is *"stikk stops describing a repository
   it has stopped watching"*. Name each RFC once, and say what a user will notice.
2. **One heading of each kind, in this order:** `### Breaking`, `### Added`, `### Changed`, `### Fixed`.
3. **No `## ` line inside the section.** Extraction stops at the first one.

**No `### Security` section this release, and this is a ruling, not an omission.** 0.7.0's RFC 030 closed the hole:
a confirmed commit already re-reads the worktree and commits nothing if it changed. **RFC 031's staling of an open
confirmation is defence in depth**, narrowing the window in which a user looks at a stale screen — **it is not a fix
for a live vulnerability, and must not be dressed as one.** If prep believes otherwise, stop and say why rather than
adding the heading.

**Every entry describes 0.8.0's end state.** Each was approved in its own review, and no review read the section as a
whole. Things to check, not a complete list:

- **The rename entry's `Limit:` clause** was amended late (A7). It must read as the shipped behaviour: prikk does not
  report whether a renamed file's content changed, **and stikk says nothing at all while prikk reports the commit
  would be refused, because prikk would author nothing.**
- **The `### Changed` entry about extra reads** currently says the Changes operation *"reads prikk's refs and status
  beside `worktree-status`"*. **That is not what shipped**: it reads `worktree-status`, `refs()` **and Orientation**,
  the last for the queue that A2's words depend on. Correct it, and say the race is accepted and stated, not hidden.
- **RFC 031's limit stays**: a worktree edit is not noticed by the check; it is caught at Enter, by `r`, or by opening
  Changes.
- **The unpublished-history entry** must cover both cases — a first commit, and a ref whose queued patches are its
  baseline with nothing sealed — because shipping only the first is the bug the dev team caught.
- **Nothing claims prevention** of a refused rename declaration. stikk prevents only on prikk's own verdict.

**If the section states suite coverage**, count it rather than copying 0.7.0's *"twelve"*.

**Then read it from the top as a user who has 0.7.0 and is deciding whether to upgrade.** That is who reads the
GitHub Release.

## 4. User-facing text that 0.8.0 makes false

**Fix what is false; do not re-plan or add marketing.** Found by the architect; grep is a floor.

| Where | Says today | Becomes, in substance |
|---|---|---|
| **`crates/stikk/README.md:44`, `## Status`** (crates.io serves this) | *"**v0.7.x shows what a change will do — and makes no change it did not show.**"* | a 0.8.x statement: it also **notices a change made outside it**, shows a declared rename as a rename, and says when a ref has no published history |
| **`docs/src/index.md:23`, `## Status`** | *"**0.7.x shows what a change will do…**"*, then the built/unbuilt list | 0.8.x, naming RFC 031 and RFC 032. **Keep prikk ≥ 0.28, validated through 0.42.0**, and keep Patch detail and Compare listed as **stikk's own work, not prikk's limit** |
| **`docs/src/contributing/releasing.md:87–89`** | *"What a **v0.7.x** release is (and is not)"* | v0.8.x. **Re-verify** the unbuilt list (merge, sync, tag create, branch create/close) rather than carrying it |
| **`docs/src/guide/getting-started.md:58`** | *"`r` refreshes the current view from prikk"* | `r` refreshes **in place**, and **re-reads the ref its own screen shows**, not the focused ref |
| **`docs/src/guide/getting-started.md`**, the same section | says nothing about outside changes | **one short paragraph**: stikk checks when the terminal regains focus and every few seconds while idle, refreshes what is on screen, says *"repository changed outside stikk — refreshed"*, and stales an open confirmation. **State the limit**: a worktree edit is not caught that way |

**A user meeting the Changes view for the first time should find the two new things there explained** — a declared
rename shown across its two rows, and what *"has no published history"* means. **One or two sentences in the guide,
not a tour.**

**Read `README.md` and the guide end to end** for anything else a 0.8.0 user would find untrue.

## 5. `ROADMAP.md`

1. **Add `## Shipped — <0.8.0's summary line> (0.8.0, breaking)`** after the 0.7.0 section, in its style: one bullet
   each for RFC 031 and RFC 032, linking `rfcs/done/`.
2. **Rename `## Next — carried into 0.8.0, in order` to `… 0.9.0 …`.**
   - **Remove items 1, 2 and 4**, which shipped.
   - **Order the rest as the proposal's last section gives it:** the **prikk 0.43.0 re-baseline first** — it retires
     what this release just shipped — **then Patch detail (`FR-030`), then Compare (`FR-033`)**, then the prose
     parse-failure attribution, the would-refuse overlay scroll and row budget, the seed-read guard, and `docs.yml`'s
     node20 actions.
   - **Keep each item's existing text**, trimming only what this release made false.
   - **The 0.43 item keeps its measured basis and prikk's reply 014 field names**, still marked as unpublished and
     to be measured when 0.43 ships.
3. **Do not re-plan `Later`.**

## 6. The Breaking table, verified once for the whole release

The 8 rows were each built by API diff in their own review. **Read `git diff 0.7.0..HEAD` over every non-test source
file once, as one release**, by the spec's five categories: signatures, enum variants, trait methods, struct fields
and `pub use`, and removed items.

- **Group by crate**, in dependency order: `stikk-model`, `stikk-prikk`, `stikk-core`, `stikk-tui`.
- **Merge rows that describe one type across increments.** At least:
  - **`Screen::Changes` has two rows** — `refreshing` from RFC 031 and `history` from RFC 032. **One row, stating the
    end state.**
  - `ChangesView` and `changes_view`'s return type belong together as one story.
- **Where one increment's change was superseded by a later one**, state the end state, not both.
- **List the candidates you rejected and why**, as every prep has. Check `Focus::Changes`'s arity, `RefHistory`'s
  placement, and the two `Box` rows — the boxings **are** breaking for a caller who pattern-matches, and stay.
- **The additive paragraph** keeps every new public item, grouped the same way.

**§3's `### Breaking` matches this table exactly.**

## 7. Heading and sweeps

- **`## Unreleased` → `## 0.8.0 — <the date you land it>`** (em dash, the file's convention).
- **No version bump.** `[workspace.package] version` and the pins have been `0.8.0` since `357e8d0`. Do not run
  `cargo update`.
- **Sweeps**, each `git ls-files | xargs grep -ln "<pattern>"`. **Read comments and examples as claims.**
  - `0\.7\.x` and `v0\.7` — after §4, none live. History, released sections and RFCs stay.
  - `0\.7\.0` — live statements of the *current* version move.
  - `validated through` — every live statement says **`0.42.0`**. **prikk 0.43 is not published; nothing may imply it
    is.**
  - `twelve` — the seam-method count is stated only where it was counted.
  - `against baseline` — nothing live claims it for a ref with no published history.

## 8. Final verification

1. §2's gates at the final commit.
2. That commit pushed to `prep/release-0-8-0`; `CI` runs; both workflows dispatched; every job and leg read; the
   branch deleted, and the request says so.
3. **Three run ids at one SHA** — `CI`, the real-binary suite (full matrix) and the supply-chain gate — and that SHA
   is the one that lands on `main` by fast-forward.
4. **`release.yml`'s own `awk`**, run locally on the final `CHANGELOG.md`, extracts the section. Report its line count
   and its headings.

## 9. Acceptance criteria

1. Gates 1–5, 7 and 8 green on the MSRV; gate 6 green on stable with a fresh target directory.
2. `CI`, the suite (full matrix) and the supply-chain gate green at one SHA, every job and leg read, run ids named;
   the `prep/` branch deleted, and said so.
3. **`## 0.8.0 — <date>`:** a summary paragraph, one heading of each kind in §3's order, **no `### Security`**, every
   entry the end state, and §3's four corrections made.
4. **§4's four files** say nothing false about 0.8.0; crates.io's README and the docs landing page describe 0.8.x, and
   the guide covers `r`'s new behaviour and the outside-change notice with its limit.
5. **`ROADMAP.md`** records 0.8.0 as shipped and 0.9.0's order as the proposal gives it.
6. **The Breaking table** verified once over `0.7.0..HEAD`, grouped by crate, the two `Screen::Changes` rows merged,
   rejected candidates listed, matching `### Breaking`.
7. **Sweeps clean; no version bump; nothing implies prikk 0.43 exists.**
8. **No product behaviour, test assertion or workflow changed.**
9. **Nothing tagged or published.**

## 10. Submit

Package to `.git-exclude/review-request/011-release-0-8-0-preparation/review-request-v1.md`.

**Lead with the `## 0.8.0` section in full, exactly as `release.yml` will extract it.** I will read it as a user who
has 0.7.0 before I read it as a reviewer.

**Then:**
1. **`crates/stikk/README.md` as crates.io will show it.**
2. **The Breaking table's rejected candidates, and the merged `Screen::Changes` row.**
3. **The three run ids at one SHA.**

**And tell me what reading the section as a whole found that two separate reviews did not.**

**Push your own commits once approved.** The tag and the publish are the owner's to authorize and mine to perform.

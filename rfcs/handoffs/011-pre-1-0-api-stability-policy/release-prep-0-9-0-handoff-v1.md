# Handoff — 0.9.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The eighth release prepared under it.
**Issued** 2026-10-14. The project owner authorized the cut and said the dev team may do the prep. **The tag and
the publish still wait for the owner**, on
[the 0.9.0 release proposal](../../../.git-exclude/release/0-9-0-proposal.md). **Read the proposal first**; this
handoff does not repeat its reasoning.
**Covers:** RFCs [034](../../done/034-prikk-0-46-rebaseline.md) and [035](../../done/035-patch-detail.md), and the
validated ceiling 0.46 → 0.51 (`dd90d4f`).
**Follows** [0.8.0's prep handoff](./release-prep-0-8-0-handoff-v1.md). **Read its §2 for the gates and runs**, which
are unchanged; this handoff only says what differs.

> **Two things make this prep bigger than 0.8.0's.**
> 1. **There is no changelog entry for any of 0.9.0's work.** 35 commits since `0.8.0` and not one touched
>    `CHANGELOG.md`, because my handoffs for RFCs 034 and 035 did not ask for entries. **§3 writes the section whole**,
>    from the RFCs' *Delivered* sections and the code.
> 2. **The version is still `0.8.0`.** §2 bumps it.
>
> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything that changes product
> behaviour, a test's assertion, or a workflow: **stop and report.**

---

## 1. Scope

**In, in this order:** the bump (§2), the changelog (§3), user-facing text (§4), `ROADMAP.md` (§5), the RFC stamps
(§6), the Breaking table (§7), the sweeps (§8), final verification (§9).

**Out:**
- any product behaviour change, **including `inert`'s escaping**: the proposal states it as a limit;
- Compare (`FR-033`) and every other carried roadmap item;
- **prikk 0.52.0**: not published (crates.io's latest is 0.51.0 as of 2026-10-14). **Nothing may imply it is**, and
  the block-end label's removal is not this release's;
- `#[non_exhaustive]` on any type (RFC 011 defers it to 1.0 readiness);
- the tag and the publish.

## 2. The version bump

`[workspace.package] version` and the five `[workspace.dependencies]` pins in `Cargo.toml`: `0.8.0` → `0.9.0`. Then
update **only the workspace members' entries** in `Cargo.lock` (`cargo update --workspace`, which touches nothing
else), and **show the lock diff in the request: only `stikk*` version lines may move.** If any third-party entry
moves, stop and report. `357e8d0` is the precedent: 3 files, 15 lines.

## 3. The changelog: `## 0.9.0 — <the date you land it>`

**Structure, as 0.8.0's:** a bold summary paragraph first, then `### Breaking`, `### Added`, `### Changed`,
`### Fixed`, in that order, and **no `## ` line inside the section**.

**The summary:** the proposal's line, *stikk now shows what each patch changed, in prikk's own terms and byte for
byte*, then what a user of 0.8.0 will notice, naming each RFC once. **Count the suite's cases; do not copy a number**
(42 at `c533947`, on Linux, macOS and Windows, at prikk 0.28.0 and 0.51.0).

**What goes where.** This is a floor, not the text. Write the entries from each RFC's *Delivered* section and the
code, and describe the **end state** only.

| Heading | Entries |
|---|---|
| `### Added` | **Patch detail** (`FR-030`): what it shows per operation kind; opened from Block detail and the Queue; `v`; content quoted byte-faithfully with prikk's JSON escapes, and every character that does not show as itself escaped (RFC 035 A3); binary content as blob id and size; `unavailable` named; the block-end label and why. **Block detail lists every block's patches**, with messages joined by id. **The Queue has a selection.** A checkout that stopped part-way is shown (prikk ≥ 0.43, RFC 034 B §7). |
| `### Changed` | **Validated through prikk 0.51.0** (was 0.46.0; `ASM-2`). At prikk ≥ 0.44 a rename declaration's fate is **prikk's `resolution`**, not stikk's inference, and **a declaration prikk will refuse makes commit unavailable** (RFC 034 B). prikk's no-repository and too-old-binary answers are environment errors (RFC 034 A). A failed patch read costs Block detail its patch list only. |
| `### Fixed` | **On prikk 0.45 and 0.46, a ref with no published history refused** Changes, History and the commit preview. stikk retries without `--ref` and accepts the answer only when it names the ref asked for (RFC 034 A). **Say that prikk ≥ 0.51 reads it directly** (measured), so the fix matters on those two releases. Plus anything else RFC 034's *Delivered* names as a defect fixed. |
| `### Breaking` | §7's table, exactly. |

**Limits the entries must state, not bury:**
- In a block of several patches, a node-addressed path is **as at the end of the block** until prikk resolves each
  patch at that patch. prikk has said that will be 0.52.0; **do not name a version as shipped.**
- **Patch detail's content is escaped; paths, messages and ref names elsewhere still pass bidi and zero-width
  characters through** (`inert` handles control characters only). Roadmap item 11.
- Patch detail starts at prikk 0.39 (stikk's JSON floor). Below that it says so in stikk's own voice.

**No `### Security` section, and this is a ruling.** Patch detail is new in this release, so its escaping fixes
nothing that shipped. The `inert` gap predates 0.9.0 and is **not** fixed by it, so a Security heading would claim a
fix that is not there. If prep believes otherwise, stop and say why.

**Then read the section from the top as a user who has 0.8.0** and is deciding whether to upgrade.

## 4. User-facing text that 0.9.0 makes false

Found by the architect; grep is a floor. **Fix what is false; add no marketing.**

| Where | Says today | Becomes, in substance |
|---|---|---|
| **`crates/stikk/README.md:44`**, `## Status` (crates.io serves this) | *"**v0.8.x keeps the screen true to the repository.**"* | a 0.9.x statement that leads with Patch detail |
| **`crates/stikk/README.md:55`** | *"Patch detail, Compare, … are not built yet"* | Patch detail is built; Compare and the rest are not |
| **`README.md:58`** | *"Patch detail and Compare are not built yet"* | the same correction |
| **`docs/src/index.md`**, `## Status` | *"**0.8.x keeps the screen true…**"* | 0.9.x, naming RFC 034 and RFC 035; **prikk ≥ 0.28, validated through 0.51.0** |
| **`docs/src/contributing/releasing.md:87`** | *"What a **v0.8.x** release is (and is not)"* | v0.9.x; **re-verify** the unbuilt list rather than carrying it |
| **`ROADMAP.md:371`**, *Releases and versioning* | *"validated through **`0.42.0`**"* | **`0.51.0`**. This one escaped two ceiling sweeps (RFC 034's and RFC 035's), so **grep `ROADMAP.md` itself** in §8 |

The guide already covers Patch detail (`2874657`, `c533947`). **Read `README.md`, `crates/stikk/README.md` and the
guide end to end** for anything else a 0.9.0 user would find untrue.

## 5. `ROADMAP.md`

1. **Add `## Shipped — <0.9.0's summary line> (0.9.0, breaking)`** after the 0.8.0 section, in its style: one bullet
   each for RFC 034, RFC 035 and the 0.51 ceiling, linking `rfcs/done/`.
2. **Rename `## Next — carried into 0.9.0, in order` to `… 0.10.0 …`.** Remove items 1–3, which shipped. **Order
   the rest:**
   - **Compare (`FR-033`)** first, with RFC 035's decision 4 (lift its text from item 3);
   - **then item 11, `inert`'s escaping**;
   - then the rest in their current order (4, 5, 6, 7, 8, 9, 10, 12).
   Keep each item's text, trimming only what this release made false.
3. **Do not re-plan `Later`.**

## 6. RFC stamps

`rfcs/README.md`'s Done table and the Status lines of `rfcs/done/034-*.md` and `rfcs/done/035-*.md`: *"0.9.0
candidate (on `main`)"* → **`0.9.0`**, in the form 032's row uses (`**0.8.0**`). **Run `rfc_index` after**: its
Status-line rule reads the first physical line, which is what caught RFC 035 last time.

## 7. The Breaking table, built once for the whole release

**Nothing is written yet**, so build it: `git diff 0.8.0..HEAD` over every non-test source file, by the spec's five
categories (signatures, enum variants, trait methods, struct fields and `pub use`, removed items), **grouped by crate
in dependency order**: `stikk-model`, `stikk-prikk`, `stikk-state`, `stikk-core`, `stikk-tui`.

Known to be there; check them, do not just copy them:
- **`Prikk` gained `show`, with no default**: breaking for any implementor;
- **`Screen` gained `PatchDetail`**; `BlockDetailView` gained `patches`; `QueuedPath` is now shared with
  `show-report-v1`; and whatever RFC 034 B did to `ChangesView`'s declarations.

**List the candidates you rejected and why**, as every prep has. **The additive paragraph** keeps every new public
item (`stikk_prikk::escape`, `JSON_REPORTS_FROM_MINOR`, the `show-report-v1` types, …), grouped the same way.
`### Breaking` matches this table exactly.

## 8. Heading and sweeps

- **`## 0.9.0 — <date>`**, inserted above `## 0.8.0` (there is no `## Unreleased` heading to rename).
- **Sweeps**, each `git ls-files | xargs grep -ln "<pattern>"`. **Read comments and examples as claims.**
  - `0\.8\.x`, `v0\.8`: none live after §4. History, released sections and RFCs stay.
  - `0\.8\.0`: live statements of the *current* version move.
  - `validated through`: **every** live statement says `0.51.0`. **Include `ROADMAP.md`.**
  - `0\.52`: nothing live says prikk 0.52 exists or ships anything.
  - `not built yet` / `unbuilt`: Patch detail appears in none of them.

## 9. Final verification

As 0.8.0's §8: the gates at the final commit; the commit pushed to `prep/release-0-9-0`; CI, the real-binary suite
(`full_platform_matrix=true`) and the supply-chain gate, **three run ids at one SHA**, every job and leg read; the
branch deleted; that SHA lands on `main` by fast-forward. **`release.yml`'s own `awk`**, run locally on the final
`CHANGELOG.md`, extracts the section: report its line count and its headings.

## 10. Acceptance criteria

1. The gates green (1–5, 7, 8 on the MSRV; 6 on stable, fresh target).
2. Three runs green at one SHA, every job and leg read, run ids named, the `prep/` branch deleted.
3. **The version is `0.9.0`**, and `Cargo.lock`'s diff moves only `stikk*` lines.
4. **`## 0.9.0 — <date>`** has a summary, one heading of each kind in §3's order, **no `### Security`**, end-state
   entries, and §3's three limits stated.
5. §4's six places say nothing false about 0.9.0.
6. `ROADMAP.md` records 0.9.0 as shipped and 0.10.0's order as §5 gives it.
7. RFCs 034 and 035 stamped `0.9.0`; `rfc_index` green.
8. The Breaking table built over `0.8.0..HEAD`, grouped by crate, rejected candidates listed, matching
   `### Breaking`.
9. Sweeps clean; nothing implies prikk 0.52 exists.
10. **No product behaviour, test assertion or workflow changed. Nothing tagged or published.**

## 11. Submit

Package to `.git-exclude/review-request/011-release-0-9-0-preparation/review-request-v1.md`.

**Lead with the `## 0.9.0` section in full, exactly as `release.yml` will extract it.** I will read it as a user who
has 0.8.0 before I read it as a reviewer. Then `crates/stikk/README.md` as crates.io will show it, the Breaking
table's rejected candidates, the lock diff, and the three run ids.

**Paste every capture and extract exactly as the file holds it** (review v3 of RFC 035, §2).

**Push your own commits once approved.** The tag and the publish are the owner's to authorize and mine to perform.

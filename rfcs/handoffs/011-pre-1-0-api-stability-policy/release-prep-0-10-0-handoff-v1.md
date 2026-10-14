# Handoff — 0.10.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md). The ninth release prepared under it.
**Issued** 2026-10-14, **effective on the project owner's authorization of the cut.** Do not start before the owner
hands it to you. The tag and the publish wait for the owner after review, on
[the 0.10.0 release proposal](../../../.git-exclude/release/0-10-0-proposal.md). **Read the proposal first.**
**Covers:** RFCs [036](../../done/036-repository-text-renders-as-it-is.md),
[037](../../done/037-read-prikk-json-where-it-now-answers.md) and [038](../../done/038-compare.md), and the node20 chore
(`b3dcade`).
**Follows** [0.9.0's prep handoff](./release-prep-0-9-0-handoff-v1.md), whose §2–§9 hold except where this one says
otherwise.

> **Two things differ from 0.9.0.**
> 1. **`## Unreleased` exists** (146 lines, written as the work landed). The prep **edits** it into one release: it
>    does not write it.
> 2. **No `prep/` branch** (owner, 2026-10-14). The runs are on `main` after approval (§8).
>
> **Sanctioned: text, the version bump, and `Cargo.lock`'s workspace entries.** Anything that changes behaviour, a
> test assertion or a workflow: **stop and report.**

---

## 1. The version bump

`[workspace.package] version` and the five pins: `0.9.0` → `0.10.0`. `cargo update --workspace`, and **show the lock
diff: only `stikk*` lines may move.**

## 2. `## Unreleased` → `## 0.10.0 — <date>`

- **A bold summary paragraph first**: the proposal's line, then what a user of 0.9.0 will notice, naming each RFC
  once. **Count the suite's cases** (47 at `44cc947`, on Linux, macOS and Windows, at prikk 0.28.0 and 0.51.0).
- **One heading of each kind, in this order:** `### Breaking`, `### Security`, `### Added`, `### Changed`,
  `### Fixed`. Security leads after Breaking this time, because this release *does* fix a shipped weakness.
- **Every entry describes the end state.** Each increment wrote its own entries; no review read them together.
  **Where two increments touched one thing, merge them into one entry.** For example, RFC 036's quoting of names,
  and RFC 037's picker row rendering a name as an identifier.
- **Read it from the top as a user who has 0.9.0.**

## 3. The Breaking table, verified once over `0.9.0..HEAD`

The twelve rows were each written in their own increment. **Verify the whole set once**, by the same two methods
0.9.0's prep used (the five-category diff, and a set-diff of public items), **grouped by crate in dependency order**.

**At least two rows are not breaking, and must go:**
- **`keys::Action` gains `MarkBlock` and `Compare`**: `mod keys` is private in `stikk-tui/src/lib.rs`. Your own 0.9.0
  prep rejected `Action::ToggleRaw` for exactly this reason.
- **`stikk_tui::worker::ResponseKind::Refs`** (inside the `list_refs` row): `mod worker` is private too. Keep the
  rest of that row.

**List the rejected candidates and why**, as every prep has. **The Additive paragraph** keeps every new public item,
merged across the three increments.

## 4. User-facing text that 0.10.0 makes false

Found by the architect; grep is a floor.

| Where | Says today | Becomes |
|---|---|---|
| **`crates/stikk/README.md:44`**, `## Status` (crates.io) | *"**v0.9.x shows what each patch changed.**"* | a 0.10.x statement leading with Compare |
| **`crates/stikk/README.md`**, the same section | *"Compare … not built yet"*, and *"outside Patch detail, a path or a message is not escaped"* | Compare built (two-ref comparison not); **the escaping limit removed**, since RFC 036 closed it |
| **`README.md:59`** | *"Compare is not built yet"* | Compare built, at prikk ≥ 0.46 |
| **`docs/src/index.md:23`**, `## Status` | *"**0.9.x shows what each patch changed.**"*, and its escaping limit | 0.10.x, naming RFCs 036–038; the limit removed |
| **`docs/src/contributing/releasing.md:87–89`** | *"What a **v0.9.x** release is"* | v0.10.x; **re-verify** the unbuilt list against the seam's methods, as you did for 0.9.0 (now fourteen, with `diff`) |
| **`docs/src/guide/getting-started.md:113`** | *"**Compare** is not built yet"* | how to open it: `m` and `d` in History, `d` in Block detail and in Changes, what each compares, and two-ref comparison as not built |

**Read `README.md`, `crates/stikk/README.md` and the guide end to end** for anything else a 0.10.0 user would find
untrue.

## 5. `ROADMAP.md`

1. **Add `## Shipped — <0.10.0's summary line> (0.10.0, breaking)`** after 0.9.0's section: one bullet each for RFCs
   036, 037, 038 and the node20 chore.
2. **Rename `## Next — 0.10.0 …` to `## Next — carried into 0.11.0, in order`.** Remove items 1–4, which shipped.
   Keep, in this order:
   - **the prikk 0.52.0 re-baseline**, unconditional now: it waits only on prikk publishing;
   - **History's and Block detail's column-0 wrap** (RFC 037's carry);
   - **`queued_elsewhere_clauses` to `Piece`s** (RFC 036's carry);
   - **comparing two different refs** (RFC 038's carry).
   Keep each item's text, trimming only what this release made false.
3. **Do not re-plan `Later`.**

## 6. RFC stamps

`rfcs/README.md`'s Done rows for 036, 037 and 038, and their Status lines: *"0.10.0 candidate (on `main`)"* →
**`0.10.0`**. Run `rfc_index` after.

## 7. Sweeps

- `0\.9\.x`, `v0\.9`: none live after §4.
- `0\.9\.0`: live statements of the *current* version move.
- `validated through`: every live statement still says `0.51.0`.
- `0\.52`: nothing says prikk 0.52 is published.
- `not built yet` / `unbuilt` / `awaits prikk`: Compare appears in none, except two-ref comparison as not built.
- **The pre-submit grep for invisible characters and decoded escapes**, over every file touched and the request.

## 8. Gates, runs, submit (no branch)

**Before submitting:** the eight gates and the real-binary suite **locally** at 0.28.0 and 0.51.0, and
**`release.yml`'s own `awk`** run locally on the final `CHANGELOG.md`. Report its line count and headings. Package
to `.git-exclude/review-request/011-release-0-10-0-preparation/review-request-v1.md`. **Lead with the `## 0.10.0`
section exactly as the `awk` extracts it**, then `crates/stikk/README.md` as crates.io will show it, the Breaking
table's rejected candidates, and the lock diff.

**After approval:** push to `main`. CI and Docs run on push. Then dispatch the full-matrix suite and the supply-chain
gate on `main`, and report every run id at that one SHA, **which is the SHA to be tagged.** If anything changes after
those runs, they run again.

**Push your own commit once approved.** The tag and the publish are the owner's to authorize and the architect's to
perform.

# RFC 050 — Exchange: sharing work as a bundle file

**Status.** **Accepted by the project owner 2026-10-21**; [Handoff A](../handoffs/050-exchange-bundles/a-open-a-bundle-handoff-v1.md) and [Handoff B](../handoffs/050-exchange-bundles/b-import-and-export-handoff-v1.md) issued. **Proposed 2026-10-21** by the architect, as 0.16.0's increments 2 and 3 (`.git-exclude/release/0-16-0-scope.md`,
authorized; re-authorized as exchange the same day after rollback was withdrawn). **No open question.**
Measured against a real prikk **0.53.0**, one fresh repository per role: `.git-exclude/reports/050-exchange/`
(`measure050.sh` → `measure050-0.53.txt`; `measure050b.sh` → `measure050b-0.53.txt`, a receiver that shares the sender's
history). prikk's source read in the clone (`crates/prikk-store/src/bundle.rs`).
**Tracks.** `FR-090`, `FR-091` (and `FR-014` for where an import lands), `FL-11`, `TU-06`, `CT-02`, `C-E2`, `C-T2a`,
`UD-02`; RFC 013 (confirmation), RFC 026/037 (received refs in the picker), RFC 047 (one pinned line read exactly).
**Touches.** `stikk-model` (the Exchange category's tier and signing requirement), `stikk-prikk` (four seam methods),
`stikk-core` (a bundle read, two previews and two writes), `stikk-tui` (a path prompt, a Bundle view, `E` in the ref
picker), `stikk-real-binary`, `requirements.md`, the guide, the changelog.

## Summary

Two people share work in prikk by **bundle files**: one exports a branch to a file, the other checks it, sees what it
would change, and imports it as a **received** branch (`remotes/…`) — read-only, trusting nobody, advancing nothing of
their own. prikk 0.53 does all of it safely; stikk lets a user do it without leaving stikk, **seeing what a bundle is
and what it would change before anything is written**.

## Findings (measured at 0.53)

### F1 — four commands; one speaks JSON

- **`bundle export --ref REF --output FILE [--force]`**: writes one ref's closure; prose report (tip block, object
  count, author keys *"continuity only, not a trust decision"*). **Refuses an existing file** unless `--force`, a missing
  directory, and an absent ref. **Needs no key.** **Writes happily inside the worktree**, where the bundle then sits as
  an untracked file. No `--format json`.
- **`bundle verify --input FILE`**: offline, **needs no repository**; prose, ending with prikk's caveat that it *"checks
  structural and internal consistency only … A verified bundle is not yet a trusted one"*. A truncated, foreign or
  bit-flipped file is refused as *"malformed persisted data: …"*. No `--format json`.
- **`bundle preview --input FILE [--ref LOCAL] --format json`** (`bundle-preview-v1`): **needs a repository**, writes
  nothing, and compares the bundle with a **local** ref (default: the bundle's own ref name). Fields: `bundle_ref`,
  `local_ref`, `connectivity` (`no-local-history`, `does-not-connect`, `already-included`, `fast-forward`, `diverged`),
  `conflict` (`applies-cleanly`, `conflict`, `undetermined`, or `null` when connectivity has nothing to apply),
  `conflict_detail` (prikk's replay error, verbatim, e.g. *"integrity error: EditText failed during localization …"*),
  `sealed_by` (key ids) with a continuity note, and `effects` (`path`, `kind` ∈ `created`/`deleted`/`edited`/
  `permission-changed`, `current_bytes`, `after_bytes`). **All five connectivities measured**, and `diverged` both
  cleanly and in conflict.
- **`bundle import --input FILE`**: stores the bundle as **`remotes/<bundle_ref>`**, *"no local ref was created or
  advanced, and no MAINTAINER key was trusted"*. **Needs no key.** A corrupted bundle is refused. Re-importing the same
  file reports *"new objects: 0"*. No `--format json`.

### F2 — importing an older bundle moves the received branch back, silently

Imported `v2` then `v1`: `remotes/heads/main` went from `v2`'s tip back to `v1`'s, exit 0, the same message. Nothing
is lost — the objects stay — but the received branch no longer shows the newer work, and nothing says so. **A stale
file in a download folder is the ordinary way to do this.**

### F3 — the preview cannot detect F2

Previewing against the received ref (`--ref remotes/heads/main`) answers `no-local-history` whatever it holds: prikk
compares only against local history. The bundle's tip is in `bundle verify`'s prose, as one line,
**`tip block: <64 hex>`**; the received ref's history is in `log --ref remotes/… --format json`.

### F4 — what adoption would take is out of reach here

A received ref cannot seed a branch (`branch create --from remotes/…` is refused by prikk, which names `merge --from`
as the way), and `merge-plan` from a received ref into a local branch answers as for any branch. **Adoption is a merge**,
a later increment on RFC 047's surface.

## Decisions

1. **Four seam methods**: `bundle_export(repo, ref, output)`, `bundle_verify(path)` (no repository), `bundle_preview(repo,
   path, local_ref) -> BundlePreview` (JSON), `bundle_import(repo, path)`. The three without JSON **keep prikk's lines
   verbatim**; refusals classify as today. **Never `--force`**: stikk does not overwrite a file (decision 5).

2. **The Exchange category: Tier Two, and no key** (`stikk-model`). Measured, neither export nor import signs anything,
   and `FL-11` already gives import a tier-2 confirmation; today the category says Tier Three and MAINTAINER. Change
   both, keyed on the category as RFC 045 made possible, with a test over `ALL` that every other category keeps its
   requirement. **Breaking** (`tier()` and `signing_requirement()` answers).

3. **Opening a bundle is read-only, and shows two things.** `i` (anywhere a repository is open) asks for a path; the
   **Bundle view** shows:
   - **prikk's verification**, verbatim, with its caveat — or prikk's refusal, verbatim, and nothing else;
   - **prikk's preview against the local branch of the bundle's own name** (prikk's default), restated in stikk's words
     per connectivity and conflict, with `conflict_detail` quoted verbatim beneath and every path through `inert`:
     *new to this repository*, *nothing in common with your branch*, *your branch already has all of it*, *would bring
     your branch forward*, *both sides have moved on — it applies cleanly / it conflicts / prikk cannot tell*;
   - who sealed it, as prikk's key ids through `identifier`, **with prikk's continuity note** — never as a trust claim.
   The path is resolved against the directory stikk was started in and **shown resolved**. Nothing is written.

4. **Import, from the Bundle view with `I`**, at Tier Two. The card names **the received branch it will write**
   (`remotes/<bundle_ref>`), the bundle's tip, the preview's sentence, and prikk's *not a trust decision* caveat. **Two
   guards stikk adds** (F2, F3), each asked again inside the preview:
   - **the bundle's tip is the received branch's tip** → *"Already received: importing changes nothing."*, declined;
   - **the bundle's tip is an older block of the received branch** → declined, in stikk's words: importing would move
     `remotes/…` back to an older block, hiding newer work you already received.
   The tip is read from **exactly one `tip block: <64 hex>` line** of `bundle verify`'s output, matched exactly, under
   RFC 047 decision 2's rule: a missing, repeated or malformed line is a shape error (`UD-02`) and import is not offered.
   Nothing else of that prose is parsed. After success: prikk's report verbatim (its *"no local ref was created or
   advanced"* note included); refs re-read; **focus moves to the received branch's history** (`FR-091`).

5. **Export, from the ref picker with `E` on a published branch**, at Tier Two. The path prompt suggests
   `<the repository's parent>/<branch's last segment>-<short tip>.bundle`. **stikk refuses a destination inside the
   repository** (`C-E2`'s check, resolved — the bundle would otherwise sit in the worktree as an untracked file, and
   could be committed) and **an existing file** (prikk refuses it too; stikk says so first, naming another name as the
   way). The card names the branch, its tip, and the resolved destination; the result is prikk's report verbatim.
   Declined in the picker, without dispatching: tags (not measured), received refs, closed branches, the unpublished
   row.

6. **Band: prikk ≥ 0.53** (`EXCHANGE_FROM_MINOR`). Only 0.53 is measured; **no user is on an older band**. Below it the
   three keys answer *unavailable*, spawning nothing.

7. **Not in this RFC**: adopting a received branch (a merge, F4), trusting its sealer (trust management), sync, a bundle
   opened with no repository (`FR-004` — stikk needs one open today), choosing another local branch to preview against,
   and overwriting with `--force`.

8. **For prikk, held for the next necessary letter**: F2 (an import that moves a received ref back says nothing),
   `--format json` on `bundle verify` and `bundle import` (stikk reads one pinned line where a field would do), and F3
   (`preview --ref remotes/…` answering `no-local-history`).

## Changelog

`### Added`: open a bundle file to see prikk's verification and what it would change, import it as a received branch,
and export a branch to a file — each previewed, at prikk ≥ 0.53. `### Breaking`: the Exchange category's tier and
signing requirement, and whatever `cargo public-api` shows.

## Delivery

**Two handoffs**, as for merge. **A, read-only** (increment 2): `bundle_verify`, `bundle_preview`, the category change,
the path prompt, the Bundle view, the band. **B, the writes** (increment 3): `bundle_import`, `bundle_export`, the two
guards and the pinned line, the export destination rules, the cards and results. Each re-runs the two measurement
scripts at 0.53 first. **Suite legs at 0.53**: the five connectivities and a conflict rendered from prikk's real JSON;
an import lands and the received branch's history is read; **importing an older bundle is declined with no `bundle
import` spawned** (the received tip unchanged by prikk's own `log`); an export **inside** the repository is declined and
the worktree byte-identical; an exported file verifies and imports in a second repository. **At 0.28**, unavailable.

## What this RFC does not do

- **No adoption, no trust, no sync** (decision 7).
- **No overwrite** of an existing file, and **no file inside a repository**.
- **No parsing of prikk's prose** beyond the one pinned line (decision 4).

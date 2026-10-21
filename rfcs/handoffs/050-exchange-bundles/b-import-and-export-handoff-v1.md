# Handoff B — RFC 050: importing and exporting a bundle (v1)

**Companion to:** [RFC 050](../../done/050-exchange-bundles.md), **accepted 2026-10-21**. **Covers:** decisions 1
(`bundle_import`, `bundle_export`), 4 and 5. 0.16.0's increment 3. **Follows [Handoff A](./a-open-a-bundle-handoff-v1.md);
start when A is pushed.** **The first to move to 0.17.0** if the cycle runs long.
**Not a release.**

> **What this increment is.** Two writes, each previewed at Tier Two with no key (A's category change): **import** a
> bundle as a received branch, and **export** a branch to a file. stikk adds **two guards prikk does not have** for
> import (RFC 050 F2) and **two destination rules** for export (decision 5).
>
> **No branch.**

---

## 1. Measure first

Re-run both measurement scripts at 0.53 (A's §1). If any row differs, stop and report.

## 2. Commit 1: two seam methods and one pinned line

- **`bundle_import(&self, repo, path) -> Result<BundleImport>`** — `bundle import --input <path>`, at `Exchange`.
  Exit 0 keeps prikk's lines verbatim; refusals as today. **Never parsed for state.**
- **`bundle_export(&self, repo, ref_name, output) -> Result<BundleExport>`** — `bundle export --ref <ref> --output
  <path>`, at `Exchange`. **Never `--force`.** Lines verbatim.
- **The pinned line**: from A's `bundle_verify` output, read **exactly one line matching `tip block: <64 lowercase
  hex>`** — matched exactly, as RFC 047 decision 2 reads `evidence outcome:`. **A missing line, a second one, or a
  value that is not 64 hex is a shape error (`UD-02`)**, and import is not offered. Nothing else of that prose is read.
- Both banded at `EXCHANGE_FROM_MINOR`.

**Tests:** argument vectors (one argument per path, a space intact; **no `--force` ever**); the pinned line found, absent,
repeated and malformed.

## 3. Commit 2: import

**a. The preview** (`stikk-core`, inside `confirm::preview`, re-reading everything as merge does):
1. verify the bundle (A); refused → blocked with prikk's text;
2. read the tip (§2's line); shape error → blocked;
3. preview (A) for `bundle_ref`; the received branch is **`remotes/<bundle_ref>`**;
4. if that received branch exists, read its lineage with `history` up to a bound (**`BUNDLE_LINEAGE_LIMIT`**, documented
   like `MERGE_LINEAGE_LIMIT`), and apply **the two guards**, each in stikk's words, without spawning `import`:
   - **the tip is the received branch's tip** → *already received: importing changes nothing* — blocked;
   - **the tip is an older block of the received branch** → blocked: importing would move the received branch back to
     an older block and hide newer work already received;
   - **the lineage is longer than the bound and the tip is not within it** → blocked: stikk cannot tell whether this
     bundle is older; name `prikk bundle import` as the way out.
**The card** (Tier Two): `target_ids` labelled (`TargetId::named`) — **`received as`** `remotes/<bundle_ref>`,
**`tip`** short id; the resolved path through `inert` in its own row; A's sentence for the local branch; prikk's
*"not a trust decision"* caveat verbatim. Consequence: stikk's words, no names (RFC 044 C1) — the bundle is stored as a
received branch; no branch of yours moves; nobody's key is trusted.

**b. The key**: **`I` on the Bundle view**, only when verify succeeded and the tip line was read; otherwise the view
does not advertise it.

**c. After success**: prikk's report verbatim (its *"no local ref was created or advanced"* note included); refs
re-read; **focus moves to the received branch and History opens on it** (`FR-091`; an explicit user action, so the
focus moves — say so in the delivery note).

## 4. Commit 3: export

**a. The key**: **`E` in the ref picker** on a **published, open, local branch**. Declined without dispatching, in
stikk's words: a tag (not measured), a received ref, a closed branch, the unpublished row; below the band.

**b. The prompt**, prefilled with **`<the repository's parent>/<the branch's last segment>-<short tip>.bundle`**. The
path resolves against the directory stikk was started in.

**c. The preview**, inside `confirm::preview`:
- **inside the repository** → blocked: `C-E2`'s resolved check (`ensure_outside_repository` with `Some(root)`), with
  stikk's reason (it would sit in your worktree as an untracked file, and could be committed);
- **an existing file** → blocked: stikk does not overwrite; choose another name;
- otherwise the card (Tier Two): **`branch`** and **`tip`** labelled, the resolved destination through `inert`.
A missing directory is **prikk's** refusal, shown verbatim; do not pre-check it.

**d. After success**: prikk's report verbatim.

## 5. Tests and the suite

- **Core:** each import guard, **with `import` never spawned** (a recording fake); the bound; the export rules, **with
  `export` never spawned** for the two blocked cases; Tier Two; no key needed.
- **TUI:** `I` advertised only when importable; `E` declines each row kind; success moves focus to the received branch.
- **Suite legs at 0.53:** an import lands — `branch list` shows the received ref and its history reads; **importing an
  older bundle is declined, no `import` spawned, and the received tip is unchanged by prikk's own `log`**; importing
  the same bundle twice declined; **an export inside the repository declined and the worktree byte-identical** (hash it
  before and after); an existing destination declined and the file byte-identical; **an exported file verifies and
  imports in a second repository**. **At 0.28**, unavailable.
- **Captures at 80 columns:** the import card; the older-bundle refusal; the export card; the inside-the-repository
  refusal; both results.

## 6. Text

`requirements.md`: `FR-090` and `FR-091` delivered at prikk ≥ 0.53 (adoption, trust and sync not). The guide: a short
**Sharing work** section — export, open, import, what a received branch is and that importing trusts nobody, and that
taking received work into your branch is not in stikk yet (`prikk merge --from`). `## Unreleased`: `### Added`.

## 7. The traps

1. **Never `--force`**, never a file inside the repository.
2. **One pinned line, matched exactly**; nothing else of verify's prose.
3. **The guards run inside the preview**, re-read at confirm time, and decline without spawning `import`.
4. **No key is required**, and none is asked for.
5. **Importing trusts nobody**; nothing on screen may suggest otherwise.
6. **Paste captures as the file holds them; grep before committing.**

## 8. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.53.0. Package to
`.git-exclude/review-request/050-exchange-b/review-request-v1.md`, leading with §1's measurement and the older-bundle
leg.

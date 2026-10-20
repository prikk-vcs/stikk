# Handoff A — RFC 050: opening a bundle, read-only (v1)

**Companion to:** [RFC 050](../../accepted/050-exchange-bundles.md), **accepted 2026-10-21**, no open question. **Read the
RFC first**, F1–F3 and decisions 1–3 and 6. **Covers:** decisions 1 (`bundle_verify`, `bundle_preview`), 2, 3 and 6.
0.16.0's increment 2. [Handoff B](./b-import-and-export-handoff-v1.md) follows with the writes.
**Not a release.**

> **What this increment is.** A user opens a bundle file and sees **what it is** (prikk's verification, verbatim) and
> **what it would change** (prikk's preview against their branch, in stikk's words). Nothing is written, by stikk or by
> prikk.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main` and dispatch the
> suite and supply chain there.

---

## 1. Measure first, at 0.53

Re-run `.git-exclude/reports/050-exchange/measure050.sh` and `measure050b.sh` against the pinned 0.53.0 (each takes
the binary and a scratch directory; seeds stay in 0600 files and are never printed). **Confirm F1–F3 row for row; if
any differs, stop and report.** The fixtures below are taken from these outputs, **as the file holds them**.

## 2. Commit 1: the category and the band

- **`RequestCategory::Exchange`** (`stikk-model/src/category.rs:114`, `:129`): **`Tier::Two`** and
  **`SigningRequirement::ReadWrite`** (decision 2: measured, neither export nor import signs; `FL-11` gives import a
  tier-2 confirmation). Update the doc comments that explain each match arm, and the test over `ALL` that every *other*
  category keeps its tier and requirement. **Breaking** — `### Breaking` names both answers.
- **`EXCHANGE_FROM_MINOR = 53`** in `stikk-prikk/src/version.rs`, documented beside the others: only 0.53 is
  measured, and no user is on an older band.

## 3. Commit 2: two seam methods

- **`bundle_verify(&self, path: &Path) -> Result<BundleVerify>`** — `bundle verify --input <path>`, **run with no
  repository** (`run(None, …)`, as the handshake does), at `ReadState`. Exit 0 keeps **prikk's lines verbatim**; a
  refusal classifies as today and keeps prikk's text. **No line is parsed in this handoff** (Handoff B reads one).
- **`bundle_preview(&self, repo: &Path, path: &Path, local_ref: Option<&str>) -> Result<BundlePreview>`** —
  `bundle preview --input <path> [--ref <local>] --format json`, at `ReadState`. Parse `bundle-preview-v1` **strictly**:
  an unknown `schema_version`, an unknown `connectivity`, `conflict` or effect `kind` is a shape error (`UD-02`), never
  a guess. Types are yours; carry `bundle_ref`, `local_ref`, `connectivity`, `conflict` (an enum with `None` for
  `null`), `conflict_detail` (verbatim), `sealed_by` (ids), `sealed_by_note` (verbatim), `effects` (path, kind,
  `current_bytes`, `after_bytes`), `rename_note` (verbatim).
- **Both banded at `EXCHANGE_FROM_MINOR`**: below it, spawn nothing and answer *unavailable*, as RFC 044's
  `RefWrite::Unavailable`.
- Arguments: **one argument per path and name**; a path with a space stays one argument.

**Tests:** the argument vectors; the JSON fixtures from §1 for **all five connectivities**, a `conflict` and an
`applies-cleanly`, parsed; each unknown-value case refused; a verify refusal (*"malformed persisted data: …"*) kept
verbatim; nothing spawned below the band.

## 4. Commit 3: the core read and the Bundle view

**a. `stikk-core`, a `bundle` module.** `open_bundle(prikk, repo, path) -> BundleOpened`: verify first; **if verify
refuses, do not preview** (a malformed file has nothing to preview) and carry prikk's refusal. Otherwise preview with
`local_ref: None` (prikk's default: the bundle's own ref name) and carry both. **stikk's sentences**, one per case, with
no name in them (names go through `identifier`/`inert` beside them):

| `connectivity` | `conflict` | stikk's sentence (yours to word, this is the meaning) |
|---|---|---|
| `no-local-history` | — | new to this repository: you have no branch of this name |
| `does-not-connect` | — | it has nothing in common with your branch of this name |
| `already-included` | — | your branch already has all of it |
| `fast-forward` | `applies-cleanly` | it would bring your branch forward |
| `diverged` | `applies-cleanly` | both sides have moved on; prikk's preview applies it cleanly |
| `diverged` | `conflict` | both sides have moved on, and it conflicts — prikk's reason below, verbatim |
| `diverged` | `undetermined` | both sides have moved on; prikk cannot tell whether it conflicts |

`fast-forward` with `conflict` other than `applies-cleanly` is impossible by prikk's own construction; **treat it as a
shape error**, not a sentence. Put the sentences in `stikk-core` so the suite reads the same words the TUI draws.

**b. The path prompt.** `i`, wherever a repository is open, opens a text prompt for a file path (reuse RFC 044's
`RefName` prompt machinery if it fits; a separate overlay if it does not — say which). The path resolves against the
directory stikk was started in; **the view shows the resolved absolute path**, through `inert`. Empty input declines
without dispatching. `Esc` cancels. **Below the band**, `i` answers *unavailable* in stikk's words, spawning nothing.

**c. The Bundle view** (a screen pushed like `MergePlan`, scrollable like Verify):
- the resolved path;
- **prikk's verification**, verbatim, under prikk's bar — including its trust caveat — or **prikk's refusal**, verbatim,
  and nothing else;
- the **bundle's ref** and the **local branch it was compared with** (`identifier`), the sentence (§4a), and under it
  `conflict_detail` verbatim when present;
- **the effects** — path (`inert`), kind, sizes — as many as fit with *"and N more"*, never a silent cut;
- **who sealed it**: the key ids (`identifier`) **with `sealed_by_note` verbatim**, never as a trust claim; and
  `rename_note` verbatim when there are effects.
`Esc` leaves. **Nothing on the view acts in this handoff**, and it advertises no key that does nothing (`I` arrives
with Handoff B).

## 5. Tests and the suite

- **Core:** each row of §4a's table from a real fixture; the impossible combination refused; verify refused → no
  preview read.
- **TUI:** `i` → prompt → view; empty and `Esc`; below the band; the view's overflow row; scroll.
- **Suite legs at 0.53** (`measure050b.sh`'s shape: a sender, and a **copy** of the sender's repository as the receiver
  so history is shared): through `open_bundle`, **`fast-forward`, `already-included`, `diverged` applying cleanly and
  `diverged` in conflict** from prikk's real JSON; `no-local-history` in a fresh repository; `does-not-connect` against
  an independent one; a **truncated** bundle refused verbatim with no preview spawned. **At 0.28**, unavailable, nothing
  spawned.
- **Captures at exactly 80 columns:** a fast-forward view; a conflict view; a refused file.

## 6. Text

`requirements.md` `FR-091`: partly delivered (verify and preview; import is Handoff B). `## Unreleased`: `### Added`
(opening a bundle), `### Breaking` (the category, and what `cargo public-api` shows).

## 7. The traps

1. **Nothing is written.** No `import`, no `export`, no file of stikk's.
2. **No prose is parsed.** The preview is JSON; verify's lines are shown, not read.
3. **Strict JSON**: an unknown value refuses, never guesses.
4. **"Sealed by" is never a trust claim.** prikk's note is shown with it.
5. **Paths and prikk's lines through `inert`**; ids through `identifier`.
6. **Paste fixtures and captures as the file holds them; grep before committing**, including for repeated sections.

## 8. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.53.0. Package to
`.git-exclude/review-request/050-exchange-a/review-request-v1.md`, leading with §1's measurement.

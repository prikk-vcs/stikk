# RFC 055 — The prikk 0.55 re-baseline

**Status.** **Done 2026-10-29**: Handoff A delivered on `main` (`62e2ad0`, `051378e`); 0.17.0 candidate. **Accepted by the project owner 2026-10-29**; [Handoff A](../handoffs/055-prikk-0-55-rebaseline/a-rebaseline-handoff-v1.md). **Proposed 2026-10-29** by the architect, as 0.17.0's conditional increment 3
(`.git-exclude/release/0-17-0-scope.md`: *"if prikk publishes a new minor before the cut"*). prikk **0.55.0** published
2026-10-28 (GitHub release *Latest*, crates.io). **No open question.**
Measured against real prikk **0.55.0** and **0.28.0** binaries. Evidence: `.git-exclude/reports/053-get-files-back/`
(`suite-0.28-0.55.0-ceiling-raised.txt`, `measure053-0.55.txt`) and `.git-exclude/reports/054-trust-read/`
(`measure054-0.55.txt`).
**Tracks.** `ASM-2`, `NFR-R03`; RFC 052 (the last re-baseline, and its shape).
**Touches.** `stikk-prikk/src/version.rs`, the READMEs, the docs, the install page's sample output, `ASM-2`, the
roadmap, the changelog.

## Summary

prikk 0.55.0 changes nothing stikk reads except one thing in stikk's favour. **The suite passes 69 of 69 at 0.28.0
and 0.55.0 with only `VALIDATED_MAX_MINOR` moved**, and the unit tests pass. Raise the ceiling, say so everywhere the
range is stated, and nothing else.

## Findings

### F1 — the suite, with only the constant moved

A copy of `main` at `4d33e19`, made outside the repository (`git archive`), with `VALIDATED_MAX_MINOR` set to 55: `cargo
+1.88 test --workspace --locked` green, and the real-binary suite **69 passed, 0 failed**, 40 cases at 0.28.0 and 29
at 0.55.0 (`suite-0.28-0.55.0-ceiling-raised.txt`). Linux only; macOS and Windows run in CI after the change, as for
every re-baseline.

### F2 — what 0.55 changed, against what stikk reads

From prikk's 0.55.0 release notes, each checked against stikk:

- **`extract` refuses a destination inside the repository** (`refusal-v1`: `destination-inside-repository`). stikk does
  not call `extract` yet; RFC 053 does, and refuses that destination itself first. Measured (`measure053-0.55.txt`).
- **A damaged bundle's refusal reads differently** (*"unexpected end of bundle: failed to fill whole buffer"* where 0.54
  said *"unexpected end of record"*, and a size named for an oversized field). stikk shows prikk's refusal verbatim
  and parses none of it; the suite's truncated-bundle leg asserts only *"malformed persisted data"*, which both say.
  Unit-test fixtures quote 0.54's wording as a fixture, which stays true of 0.54.
- `extract --claim`'s new refusal, the library's `BundleSource` and `#[non_exhaustive]` changes, faster `verify` and
  `bundle import`, fewer disk syncs, streamed bundle reads, the Windows uninstaller's `Path` fix: **nothing stikk
  calls or parses**.

### F3 — RFC 054's reads are unchanged

`trust maintainer list/check` and `key status` give the same reports at 0.55.0 as at 0.54.1 (`measure054-0.55.txt`).

## Decisions

1. **`VALIDATED_MAX_MINOR = 55`**, its doc comment saying what 0.55 changed for stikk (F2) and that the suite was green
   with only the constant moved. CI installs `0.55.0` itself (`print_version_matrix`).
2. **The range is stated as `0.55.0`** wherever it says `0.54.1`: `README.md`, `crates/stikk/README.md`,
   `docs/src/index.md` (both mentions), `docs/src/guide/getting-started.md`, the install page's sample output
   (`prikk 0.55.0`, *0.28 through 0.55*), `ASM-2` (appended to its history, in its shape), and the roadmap's
   *Releases and versioning*. The roadmap's 0.16.0 section keeps 0.54.1: that is what 0.16.0 shipped.
3. **No leg changes**, and no new leg: F2's only change stikk will meet is tested by RFC 053's legs.
4. **Order:** land this **before RFC 053's commit 1** if it can, so 053's and 054's legs run at 0.55 from their first
   run; nothing breaks if it lands after.

## Changelog

`### Changed`: validated through prikk 0.55.0.

## Delivery

**One handoff**, issued on acceptance. Its gate is F1 reproduced: the suite at 0.28.0 and 0.55.0 with the constant
moved, then the full platform matrix in CI after the push.

## Delivered

**One handoff, two commits, green on `main` at `051378e`**: CI, Docs, supply chain, `installers.yml` (dispatched) and
the real-binary suite on the full platform matrix (69 cases at 0.28.0 and 0.55.0). Commit 1 moved only
`VALIDATED_MAX_MINOR`, and the suite at that commit is the evidence; commit 2 says `0.55.0` in the seven places stikk
states the range. Two sentences true then and false once RFC 053 called `extract` were carried to RFC 053 and reworded
there (`6b14260`).

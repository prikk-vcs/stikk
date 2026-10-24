# RFC 052 — The prikk 0.54 re-baseline, and prikk's own Windows installer

**Status.** **Accepted by the project owner 2026-10-24**; [Handoff A](../handoffs/052-prikk-0-54-rebaseline/a-rebaseline-handoff-v1.md) issued. **Proposed 2026-10-24** by the architect, as 0.16.0's conditional increment 4
(`.git-exclude/release/0-16-0-scope.md`: *"if prikk publishes it before the cut"*). prikk **0.54.1** published
2026-10-24 (GitHub release *Latest*, crates.io; prikk's letter 027). **No open question.**
Measured against real prikk **0.54.1**, **0.53.0** and **0.28.0** binaries. Evidence:
`.git-exclude/reports/052-prikk-0-54-rebaseline/` (`suite-0.28-0.54.1-ceiling-raised.txt`, `measure-show.sh` →
`measure-show.txt`).
**Tracks.** `ASM-2`, `NFR-R03`; RFC 049 (the last re-baseline, and its shape); **RFC 048 decision 8**, which said this
re-baseline switches stikk's Windows message to prikk's own installer.
**Touches.** `stikk-prikk/src/version.rs`, `scripts/install.ps1`, `scripts/uninstall.ps1`, `installers.yml`'s Windows
assertions, the install page, the READMEs, `ASM-2`, the changelog.

## Summary

**prikk 0.54.1 changes nothing stikk reads in a way stikk must handle**, fixes one read stikk shows, and — the reason
this RFC is more than a ceiling — **ships its own Windows installer**, which RFC 048 has been waiting for. So the
validated ceiling becomes 54, and **on Windows stikk's installer stops pointing at prikk's guide and prints prikk's own
one line**, as it already does on Linux and macOS. Both platforms are then two plain steps.

## Findings

### F1 — the suite, with only the ceiling raised

On a scratch copy of `main` at `2240b78` with `VALIDATED_MAX_MINOR` set to 54 and nothing else, at prikk 0.28.0 and
0.54.1: **69 passed, 0 failed**; the workspace's own tests, **1260 passed, 0 failed**. **No leg moved.** (The harness
confirms each binary's version before every leg, so the run is at 0.54.1, not a stale binary.)

### F2 — what 0.54 changed, and what reaches stikk

- **`show <block-id>` no longer fails after consecutive text edits of one file.** Measured: at 0.53.0 the third of three
  sealed edits to `a.txt` refuses with *"integrity error: lifecycle replay: blob … required for a state effect is
  missing"*, exit 1; at 0.54.1 all three show, exit 0. **stikk's Block and Patch detail read `show`**, and at 0.53
  showed prikk's refusal verbatim there; at 0.54.1 they open. Nothing in stikk changes; the user-visible difference is
  prikk's fix.
- **`verify` prints a new count line, `unreached objects: N`, and `--format json` gains an additive `unreached_objects`
  array**; an unreached object is now a warning, and `verify`'s exit stays 0 on it alone (letter 027 §3). **stikk reads
  `verify-report-v1` for its verdict, failing conditions and stages only** (`parse_json.rs:393`), so the new field
  reaches the raw view untouched and the verdict is prikk's, as before. stikk's Verify does not yet show prikk's
  warnings at all (`FR-100`, RFC 040's deferral) — unchanged.
- **`extract` is new**, with `extract-report-v1` and a `refusal-v1` document for its refusals. stikk does not call it.
  No other command's output changed (letter 027 §3).

### F3 — prikk's Windows installer exists, and its uninstaller

At `releases/latest/download/`: `install.ps1`, `install.ps1.sha256` and `uninstall.ps1` answer **200** (measured
2026-10-24). prikk's letter 027: it verifies the checksum, installs into `%LOCALAPPDATA%\Programs\prikk` (the folder
letter 025 named), adds it to the user `PATH` once, says what the checksum does and does not prove; measured by prikk on
x64 and ARM64 Windows under PowerShell 5.1 and 7. **0.54.0's `install.ps1` refused some ordinary x64 machines**; 0.54.1
fixed it, and the `latest` link serves 0.54.1's. **stikk pins no prikk version** (RFC 048 decision 2), so nothing of
stikk's can fetch the broken one.

### F4 — `cargo binstall prikk` on Windows

`cargo binstall --dry-run --disable-strategies compile,quick-install --targets x86_64-pc-windows-msvc prikk@0.54.1`
resolves (measured), where 0.53.0 did not (RFC 048 M4).

## Decisions

1. **The validated ceiling is 54**, raised in one commit, with the pinned 0.54.1 in the suite. The suite at 0.28.0 +
   0.54.1 at that commit is F1's: green.
2. **Windows: prikk's own command.** Where `install.ps1` today prints prikk's guide and release page with *"a Windows
   installer is coming from prikk"*, it prints **`irm https://github.com/prikk-vcs/prikk/releases/latest/download/install.ps1 | iex`**,
   with prikk's guide on the next line, as `install.sh` does — for **missing** and **too old** alike. `uninstall.ps1`
   prints prikk's `uninstall.ps1` line. Both still run nothing of prikk's (RFC 048 decision 2). The comment in
   `install.ps1` that named this re-baseline as the place to change goes with it.
3. **The install page and READMEs**: Windows step 1 becomes prikk's one line, linked to prikk's guide as the
   authority; *"prikk's own Windows installer is on its way"* goes. `installers.yml`'s Windows assertions follow the
   new text.
4. **`validated through 0.54.1`** in prose — the patch named, because 0.54.0's Windows installer is the one prikk asks
   people not to use. **The installers' version report names the minor**: it says *"0.28 through 0.54"*, not *"through
   0.54.0"* — the ceiling is a minor (`VALIDATED_MAX_MINOR`), the filled `@PRIKK_VERSION@` has printed `0.<n>.0`, a
   version stikk did not test this time. `release.yml` and `installers.yml` fill it the same way, so the assertions
   move together.

## Changelog

`### Changed`: validated through prikk 0.54.1; on Windows the installer prints prikk's own install command, so both
programs install in two plain steps there too. (With 0.54.1, Block and Patch detail open where prikk 0.53 refused after
consecutive edits of one file — prikk's fix.)

## Delivery

**One handoff** ([Handoff A](../handoffs/052-prikk-0-54-rebaseline/a-rebaseline-handoff-v1.md)): the ceiling, the two
Windows scripts and their `installers.yml` assertions, the page and READMEs, `ASM-2`. Measured before review: the suite
at 0.28.0 + 0.54.1; `installers.yml`'s Windows leg on the push. The release prep sweeps `validated through`.

## What this RFC does not do

- **Show prikk's verify warnings** (`FR-100`; unchanged by this RFC).
- **Use `extract`.**
- **Pin any prikk version** in stikk's installers.

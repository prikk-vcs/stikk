# RFC 057 — Orientation on `status --format json`

**Status.** **Done 2026-10-30**: Handoff A delivered on `main` (`86e7c34`..`5322ecc`); **released in 0.19.0**. **Accepted by the project owner 2026-10-30**; [Handoff A](../handoffs/057-orientation-on-status-json/a-orientation-json-handoff-v1.md). **Proposed 2026-10-30** by the architect, as 0.19.0's increment 1 (`.git-exclude/release/0-19-0-scope.md`,
authorized 2026-10-30). **No open question.**
**Measured** against real prikk **0.53.0, 0.54.1 and 0.55.0**: `.git-exclude/reports/057-orientation-json/`
(`measure057.sh` → `measure057-0.5{3,4,5}.txt`, identical across the three once ids and paths are normalised). Five
states were measured, each with prose and JSON side by side: a repository with nothing published, one sealed block,
one queued patch, the warn threshold, and the hard limit. **prikk's source was read** for the two states no script
can reach: an unresolved current branch and an interrupted materialization (`prikk-cli/src/main.rs` `run_status`,
`prikk-cli/src/output/status.rs` `print_status_json`).
**Tracks.** `UD-02`, `ER-02`, `C-T2a`; RFC 009 (the queue field), RFC 014 F5 (the active-patch warning), RFC 029/030
(the current branch), RFC 034 §7 (the interrupted materialization).
**Touches.** `stikk-prikk` (the `orientation` seam call, `Orientation`), `stikk-core` (`OrientationView` and commit's two
uses), `stikk-tui` (Orientation's two lines), `stikk-real-binary`, the changelog.

## Summary

Orientation, the screen every session opens on, is built from `prikk status`'s **prose**, which prikk itself calls
*"current CLI vocabulary, not a stable machine-readable schema"*. `status --format json` (`status-report-v1`) has carried
everything the prose carries since prikk 0.35.0, and everything Orientation reads since 0.43.0. stikk reads it there,
and keeps the prose parser only for the floor's band.

**What a user sees does not change, with two exceptions.** prikk's JSON states the active-patch threshold and an
interrupted materialization **as data, not sentences**. So stikk says those two things **in its own words**, with
prikk's numbers and prikk's commands carried exactly.

## Findings

### F1 — every field Orientation reads has a JSON counterpart (measured)

| Orientation reads (prose) | `status-report-v1` |
|---|---|
| `queued patches: N` / `N targeting REF` | `queue.count`, `queue.target_ref` |
| …`targeting <missing metadata>` / `<malformed metadata>` | `queue.target_ref: null` with `queue.target_ref_status: "missing-metadata"` / `"malformed-metadata"` (source) |
| `trailing partial WAL bytes: N` | `trailing_partial_wal_bytes` |
| `heads/main RefState: ID` / `<not published>` | `heads_main_ref_state`: the id, or `null` |
| `current branch: REF` | `current_branch` |
| `current branch: <unresolved; run `prikk doctor`>` | `current_branch: null` (source) |
| `warning: active patches (N) at or above the recommended threshold (W); consider running `prikk seal`` | `queue.threshold_status: "warn"`, `queue.warn_threshold: W`, with `queue.count: N` |
| `warning: … at or above the configured hard limit (L); commit is blocked until you run `prikk seal`` | `queue.threshold_status: "hard-limit"`, `queue.hard_limit: L` |
| `interrupted materialization: a checkout or branch switch stopped part-way; move aside any file it named, then run prikk checkout --patch-materialize --ref B or prikk branch switch B` | `interrupted_materialization: {"routes": ["prikk checkout --patch-materialize --ref B", "prikk branch switch B"]}`, or `null` (source) |

- **Every key is always written**, with `null` where the prose would omit a line or print a sentinel. With an empty
  queue, `threshold_status`, `warn_threshold` and `hard_limit` are all `null`. With a non-empty queue below both
  thresholds, `threshold_status` is `"none"` and the two numbers are present (measured).
- **Versions.** `current_branch` arrived in 0.42.0, and `interrupted_materialization` in **0.43.0** (prikk's
  CHANGELOG). From 0.43 every row above is in the JSON.

### F2 — two of those rows are data in JSON and a sentence in prose

The active-patch warning and the interrupted materialization are prikk's **sentences** in the prose. In the JSON they
are a vocabulary word with numbers, and a list of two commands. stikk shows prikk's sentence verbatim today (`ER-02`,
RFC 014 F5, RFC 034 §7). That cannot continue on the JSON path, because the JSON has no sentence to show.

## Decisions

1. **The seam call changes, not the trait.** At **prikk ≥ 0.43** (`ORIENTATION_JSON_FROM_MINOR = 43`), the
   `orientation` call runs `status --format json` and builds `Orientation` from `status-report-v1`. Below 0.43 it runs
   `status` and the prose parser, unchanged.
   - **One read either way.**
   - **Closed vocabularies:** `threshold_status` ∈ {`none`, `warn`, `hard-limit`, `null`} and `target_ref_status` ∈
     {`null`, `missing-metadata`, `malformed-metadata`}. An unknown value, a missing key or a wrong type is a shape
     error (`UD-02`).
   - **Unknown extra keys are ignored**, which is how stikk reads every other `-v1` document.
   - **The prose parser's interrupted-materialization reading is removed.** prikk prints that line only at ≥ 0.43,
     which is now the JSON band, so the prose path can never meet it.
2. **The two sentences become typed, and stikk words them.**
   - `active_patch_warning` becomes an enum:
     - **`Said(String)`** below the band: prikk's line, verbatim, as today.
     - **`Threshold { blocking: bool, count: u64, threshold: u64 }`** at ≥ 0.43. stikk's words:
       - warn: *"N patches are queued, at or above prikk's recommended threshold of W. Consider sealing."*
       - hard limit: *"N patches are queued, at prikk's limit of L. prikk refuses to commit until you seal."*
   - `interrupted_materialization` becomes **`Option<Vec<String>>`**, the routes, which are only ever set at ≥ 0.43.
     stikk's words: *"A checkout or branch switch stopped part-way. Move aside any file it named, then run one of:"*,
     followed by prikk's routes **verbatim**, one per row, each through `inert`.
   - **Neither line says *"prikk reports"* any more**, because the words are stikk's. The commands and numbers are
     prikk's, carried exactly.
   - The commit card's two uses (`commit.rs`: the consequence's appended warning, and the interrupted block) take the
     same sentences. Every other piece of RFC 014's and RFC 034's behaviour is unchanged: commit is still blocked on an
     interrupted materialization, and reads are still not.
3. **`current_branch: null` is `CurrentBranch::Unresolved`**, carrying stikk's sentence: *"unresolved — `prikk
   doctor` can say why"*. Below the band, `Unresolved` keeps carrying prikk's sentinel verbatim, as today.
4. **`provisional_worktree` is not read here.** It is in the JSON and not in Orientation today, so reading it would be a
   visible change of its own and a separate decision.
5. **Breaking:** whatever `cargo public-api` shows (`Orientation`'s and `OrientationView`'s two field types).

## Delivery

**One handoff.**

- **Measure first:** re-run `measure057.sh` at the ceiling.
- **The proof of "no visible change": an equivalence leg.** At the ceiling, for each of the five measured states,
  `orientation` through JSON (the new path) and through prose (the prose parser, called directly on `status`'s
  output) **agree on every field**, except that the two typed fields carry the same numbers and routes.
- **The two source-only states** get unit fixtures in `status-report-v1`'s exact shape, quoting the writer
  (`status.rs`). An unresolved branch, and an interrupted materialization with both routes, are each drawn on
  Orientation and on the commit card.
- **Captures at 80 columns:** Orientation and the commit card, with the warn line, the hard-limit line and the
  interrupted line.
- **At 0.28**, the prose path, with nothing about the JSON spawned.

## What this RFC does not do

- **Read `provisional_worktree`** (decision 4).
- **Drop the prose parser**: the floor needs it.
- **Change what Orientation shows**, except the wording of decision 2's two lines and decision 3's one.

## Delivered

**One handoff, six commits, green on `main` at `5322ecc`**: CI, Docs, supply chain, and the real-binary suite on the
full platform matrix (87 cases at 0.28.0 and 0.55.0 on Linux and macOS; 86 on Windows).

- **The seam**: `ORIENTATION_JSON_FROM_MINOR = 43`, with one read either way. It reuses the Queue view's own readers
  of `status-report-v1`, so the two cannot disagree. Every key is required and both vocabularies are closed. The prose
  parser's interrupted reading is removed, because the JSON band now covers it.
- **The words**:
  - `active_patch_sentence` is prikk's line below the band and stikk's around prikk's numbers above it;
  - Orientation lists each route on its own row;
  - the commit card puts the routes in backticks, joined by *"or"* (`774e9e8`, review v1). Bare commands run
    together once the text wraps.
- **The proof**: an equivalence leg reads five states through JSON and through prose. Every field agrees, and both
  warning states are counted as reached. The floor leg is proved by an argv shim and by prikk 0.28 refusing
  `--format`.
- **Found by the dev team**:
  - Orientation never showed the active-patch warning; only the commit card does.
  - stikk's *"unresolved"* sentence was drawn as repository text and attributed to prikk in two places. Both fixed.
- **One red interval**, at `774e9e8`. The review's no-resubmission instruction named the unit tests but not the
  real-binary suite, and RFC 034's leg still asserted the old lead-in. Fixed in `5322ecc`.

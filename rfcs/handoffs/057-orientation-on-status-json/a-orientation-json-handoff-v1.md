# Handoff A — RFC 057: Orientation on `status --format json` (v1)

**Companion to:** [RFC 057](../../accepted/057-orientation-on-status-json.md), **accepted 2026-10-30**. **Read the RFC
first**: F1, F2 and decisions 1–5. **Covers:** all of it. **0.19.0's increment 1.**
**Land it before RFC 056's [Handoff B](../056-trust-adopt-and-remove/b-typed-card-layout-handoff-v1.md)** if both are in
flight, because both re-capture the commit card. It is independent of RFC 053's Handoff C.
**Not a release.**

> **What this increment is.** At prikk ≥ 0.43, the Orientation read parses `status-report-v1`; below that it keeps the
> prose parser. **Nothing a user sees changes except three lines' wording**, which RFC 057's decisions 2 and 3 give
> exactly.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite with `full_platform_matrix: true` and supply chain there.

---

## 1. Measure first

Run `.git-exclude/reports/057-orientation-json/measure057.sh <prikk> <empty scratch dir>` at **0.55.0**, and diff it
against `measure057-0.55.txt`, ignoring ids and paths. If anything else differs, stop and report.

## 2. Commit 1: the seam (`stikk-prikk`)

1. **`ORIENTATION_JSON_FROM_MINOR = 43`** goes in `version.rs`, beside the other bands. Its doc names both reasons:
   - `interrupted_materialization` is in `status-report-v1` from 0.43.0;
   - `current_branch` is in it from 0.42.0.
2. **`orientation`** (`cli_backend.rs`, around `:515`):
   - at ≥ the band it runs `status --format json`, parsed by a new `parse_json::orientation`;
   - below the band it runs `status` and `parse::orientation`, as today.
3. **`parse_json::orientation`** reads exactly the rows of RFC 057's F1 table:
   - `queue.count`;
   - `queue.target_ref` together with `queue.target_ref_status`. Both missing-metadata forms map to `None`, as the
     prose path maps them;
   - `trailing_partial_wal_bytes`;
   - `heads_main_ref_state`;
   - `current_branch`;
   - `queue.threshold_status` with `warn_threshold` and `hard_limit`;
   - `interrupted_materialization.routes`.

   **The two vocabularies are closed** (decision 1), and an unknown value is a shape error. **Ignore `queue.patches`
   here**: the Queue view already reads it through `parse_json::queue`. **Ignore `provisional_worktree`** (decision 4).
4. **The types** (decision 2):
   - `active_patch_warning: Option<ActivePatchWarning>`, with `Said(String)` and
     `Threshold { blocking, count, threshold }`;
   - `interrupted_materialization: Option<Vec<String>>`.

   **Remove** the prose parser's interrupted-materialization reading, which can no longer be reached, along with its
   test, and say so in the commit message.
5. **`current_branch: null`** maps to `CurrentBranch::Unresolved` carrying decision 3's sentence. A non-null value that
   is not a ref name is a shape error, the same rule as the prose path.

**Tests:**

- the five measured states' JSON, as fixtures pasted from `measure057-0.55.txt`, each parsed to the expected
  `Orientation`;
- the two source-only states, built in `print_status_json`'s exact shape and quoted in the test's doc:
  - an unresolved branch;
  - an interrupted materialization, with both routes;
- each vocabulary's unknown value refused, and a missing key refused;
- the band: 0.42 runs prose and 0.43 runs JSON, with the argv asserted for both.

## 3. Commit 2: the words (`stikk-core`, `stikk-tui`)

1. **Put stikk's sentences in `stikk-core`**, as `pub const`s or `fn`s with numbers, so the TUI and the commit card
   share them. Use decision 2's two warning sentences and the interrupted lead-in exactly as written.
2. **Orientation (`view/orientation.rs`):**
   - **The interrupted field** shows the lead-in, then each route **verbatim** on its own row through `inert`, wrapped
     rather than clipped, as RFC 034 required.
   - **Drop *"prikk reports a checkout that stopped part-way —"*.** The words are stikk's now.
   - **The active-patch warning:** `Said` is drawn as today; `Threshold` is drawn with stikk's sentence.
3. **The commit card (`commit.rs`):**
   - the interrupted block becomes stikk's lead-in and the routes, with no *"prikk reports:"*;
   - the consequence appends stikk's warning sentence for `Threshold`, and prikk's line for `Said`, as today.

   **Update the two doc comments that say these are prikk's words verbatim** (`commit.rs:239-241` and `:415`). For the
   JSON band they now say the reverse.
4. **The status bar and the session logic** need no change for `Unresolved`, which still carries a `String`. Check that
   the displayed text reads well with decision 3's sentence.

## 4. Commit 3: the suite (`stikk-real-binary`)

1. **The equivalence leg**, `rfc057_json_and_prose_orientation_agree_at_the_ceiling`. For each of the five measured
   states, build the state, then:
   - **(a)** run `orientation` through the backend, which is the JSON path at the ceiling;
   - **(b)** run `prikk status` directly and parse the text with the prose parser.

   **Every field equals**, except the typed warning: assert that (a)'s `Threshold` numbers are the ones in (b)'s
   `Said` sentence. Reach the prose parser through whatever test-only seam is cleanest, and say which in the request.
   This leg is RFC 057's proof of "no visible change".
2. **The floor leg:** at 0.28, `orientation` spawns `status` without `--format`, which the argv recorder asserts.
3. **The thresholds**, set by environment variable in the harness (`PRIKK_ACTIVE_PATCH_WARN`,
   `PRIKK_ACTIVE_PATCH_LIMIT`), under `ENV_LOCK` and cleared after, as the key variables are.

## 5. Text

- **`CHANGELOG.md`, `## Unreleased`:**
  - `### Changed`: Orientation now reads prikk's JSON status at prikk 0.43 or newer, and the active-patch warning and
    an interrupted checkout are worded by stikk, with prikk's numbers and commands;
  - `### Breaking`: whatever `cargo public-api` shows.
- **`requirements.md`:** where `FR-001`/Orientation or RFC 034's note says Orientation reads prose, add one dated line.
- **`ROADMAP.md`'s *Later*:** the *"Orientation on `status --format json`"* entry is done. Remove it in this commit, and
  the release prep moves it to *Shipped*.

## 6. The traps

1. **Routes and numbers are prikk's, carried exactly.** Only the framing sentences are stikk's.
2. **No prose parsing on the JSON path**, not even for the warning.
3. **One read per Orientation**, in either band.
4. **Paste captures as the file holds them, and grep before committing.**

## 7. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/057-orientation-json-a/review-request-v1.md`. Lead with §1's diff and the equivalence leg,
and include 80-column captures of Orientation and the commit card in the warn, hard-limit and interrupted states.

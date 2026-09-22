# Handoff A — RFC 034: the repair, and the 0.46 ceiling (v1)

**Companion to:** [RFC 034](../../accepted/034-prikk-0-46-rebaseline.md). **Accepted 2026-09-22; Q1 ruled (b)** — this
is **not** a patch release; it lands on `main` for 0.9.0. **Amendment A1 applies: the ceiling raise comes first.**
**Read the RFC's F0–F3 before starting**; this handoff does not repeat their measurements, and every number below is
from them.
**Covers:** RFC 034 decisions **1** (the unpublished-ref read), **2** (the environment class) and **3** (the ceiling,
and the two wording-pinned tests).

> **What broke:** prikk **0.45.0** made `worktree-status --ref R` and `log --ref R` refuse when `R` has no published
> history. Until a repository's first `seal`, that is every ref — so the Changes view, History and **commit's
> preview** all refuse on a new repository. **prikk still reports it with no `--ref` at all.**
>
> **The floor of this work:** stikk must never show a report for a ref it did not ask for. The repair is only safe
> because the ref-less report **names the ref it used**, and stikk checks it.

---

## 1. Order, and the red window

**Do it in this order, one commit each.**

1. **Raise `VALIDATED_MAX_MINOR` to 46, alone.** Nothing else in that commit (RFC 029's `9b0a6e4` is the precedent).
   **The suite is then red at the 0.46 end on F0's four regressions** — expected. Run it, keep the output, and say so
   in the review request.
2. **The repair** (§3), which turns those four green.
3. **The environment class** (§4).
4. **The two wording-pinned tests made version-aware** (§5).

**Do not reorder 1 after 2.** The suite refuses any binary the constant does not name, so the raise is what lets it
measure the repair at all.

## 2. What is already measured — do not re-derive it

From RFC 034, at `cargo install prikk --version X --locked` binaries:

| prikk | `worktree-status --ref heads/main`, `heads/main` never published |
|---|---|
| 0.43.0, **0.44.0** | the full report |
| **0.45.0**, 0.46.0 | `error: precondition not met: ref heads/main does not exist in this repository`, exit 1 |

- **With no `--ref`, 0.46 still reports**, and the report names `"ref": "heads/main"` beside
  `"current_branch": "heads/main"`. `log` likewise returns an empty history.
- **`prikk commit --ref heads/main` still queues a first patch**, and after the first `seal` the explicit form works
  again.
- **Only the JSON path can meet this refusal.** stikk reads JSON at prikk ≥ 0.39 (`reads_json()`), and the refusal
  starts at 0.45. **The prose path needs no change** — say so in a comment rather than leaving it to be wondered at.

## 3. The repair (decision 1)

**Where:** `CliBackend::worktree_status` and `CliBackend::history`, in `crates/stikk-prikk/src/cli_backend.rs`,
**JSON paths only**.

**The rule, exactly:**

1. Run the command as today, with `--ref <reff>`.
2. **Only if prikk refuses with its own clause `does not exist in this repository`** — matched as a semantic clause,
   **never on the `precondition not met:` prefix**, which is the standing rule in `classify.rs`'s module doc — re-run
   **the same command with the `--ref <reff>` pair removed** and everything else identical.
3. **Accept the retry's report only if the report's own `ref` equals `reff`.** `worktree-status-report-v1` and
   `log-report-v1` both carry it.
4. **On any other outcome — the retry refuses, the report parses but names another ref, or the report does not parse
   — return the original refusal**, not the retry's. The user must never be told something about a ref they did not
   ask about.

**Nothing else decides this.** Do **not** consult `refs()`, and do **not** read `current_branch` to choose: the
report's own `ref` is the check, it costs no extra call, and it cannot race. One retry, only on that refusal, never
cached.

**A comment must say why the retry exists**, naming prikk 0.45.0, so that when prikk restores the behaviour (letter
015 asks) a later reader knows what the code is for and when it can go.

### The traps

1. **Showing another ref's report is the failure this must never have** (`T-T4`). Step 3 is not optional, and it is
   the reason a test drives an *absent* ref, not only an unpublished one.
2. **An absent ref must still refuse.** `--ref heads/typo` on any prikk: the retry returns the current branch's
   report, step 3 rejects it, and stikk shows prikk's original refusal. **Test this explicitly** — it is what stops
   the repair from becoming a lie for every mistyped ref.
3. **An unpublished ref that is not prikk's current branch stays unreadable** on ≥ 0.45, and there is no invocation
   that reads it. **stikk shows prikk's refusal**; it must not fabricate an empty view (`C-T2c′`).
4. **RFC 030's re-read must see the same thing the preview saw.** Both go through this one seam method, so the
   fallback applies identically — the suite leg that failed (`rfc030_a_file_added_…`) is the proof, and it must pass
   at both ends afterwards.
5. **The prose path is untouched**, and `reads_json()` already gates it.

## 4. The environment class (decision 2)

**Where:** `is_environment` in `crates/stikk-prikk/src/cli_backend/classify.rs`.

**Add two clauses, keeping the three that are there:**

| State | prikk's words | Clause to match |
|---|---|---|
| a directory holding no repository (≥ 0.45) | `precondition not met: no prikk repository at <path>` | `no prikk repository at` |
| a repository whose format the binary is too old for | `unsupported format version: 0` (measured: prikk 0.43 on a format-7 repository) | `unsupported format version` |

- **Match the clause, never the prefix**, exactly as the existing arms do.
- **Do not parse or assert the number.** prikk names `0` where the repository is format 7 — letter 015 reports it, and
  stikk must not depend on either value.
- **Keep `uses format` + `no longer supports`**: that is the *older* refusal for a *retired* format, provoked from a
  real format-2 repository, and it is a different state from this one.
- **Captured tests** for both new strings, in `classify/tests.rs`, alongside the existing captures.

## 5. The two wording-pinned tests (decision 3)

Both fail at 0.46 because they pin prikk's old text, not because stikk is wrong:

- `rfc032_a_declaration_whose_source_is_back_…` pins prikk **0.42**'s refusal. Make it version-aware: 0.42's text
  below 0.43, and at ≥ 0.43 prikk's refusal that names its own ways out.
- `rfc032_a_declaration_without_its_destination_…` pins *"destination is ignored"* for a **directory** destination;
  **prikk 0.44 says *"destination is a directory; recorded as a deletion, not a rename"*** — the fix stikk asked for
  in letter 014. Assert the new text at ≥ 0.44, the old below it.

**Name the surface in each assertion**, as RFC 032 A4 requires.

## 6. Tests

- **The four regressions of F0 must pass** at 0.28 and 0.46 after §3.
- **A new suite leg for the absent ref** (trap 2): a mistyped ref refuses at both ends, and stikk shows prikk's
  refusal.
- **A new suite leg for the first commit on a fresh repository** at 0.46: Changes lists the untracked files with RFC
  032's no-published-history words, commit previews, confirms and queues, and the ref reads normally after `seal`.
- **Core tests with a stand-in backend** for step 3's rejection path — a retry that returns another ref's report must
  surface the original refusal. The suite cannot produce that state; core can.
- **prikk 0.45 is not in the suite's matrix** (its two ends are 0.28 and the ceiling). **Measure 0.45 by hand**, with
  a script in the review-request folder, and report it beside the suite.

## 7. Gates and runs

The eight gates as always — 1–5, 7 and 8 on the MSRV from `Cargo.toml`; gate 6 (`cargo package`) on stable with a
fresh `CARGO_TARGET_DIR`. Then, from a `prep/034-a-the-repair` branch: `CI`, the real-binary suite with
`full_platform_matrix=true`, and the supply-chain gate, **all at one SHA**, every job and leg read, the branch deleted
before submitting.

**The suite now installs 0.46 at its ceiling end**, from the raised constant — check that the workflow picked it up
rather than a cached 0.42.

## 8. Acceptance criteria

1. **The ceiling raise is its own commit**, first, and the review request shows the suite red at that commit with the
   four F0 regressions named.
2. **The repair triggers only on prikk's own clause**, retries without `--ref`, and **accepts the report only when it
   names the requested ref**; every other path returns the original refusal.
3. **An absent ref still refuses**, driven by a test.
4. **No `refs()` or `current_branch` consultation** decides the fallback; the prose path is unchanged.
5. **The environment class matches the two new clauses**, with captured tests, and asserts no format number.
6. **The two wording pins are version-aware**, naming their surface.
7. **32+ suite cases green at 0.28 and 0.46**, prikk 0.45 measured by hand and reported.
8. **Nothing tagged or published.**

## 9. Submit

Package to `.git-exclude/review-request/034-a-the-repair/review-request-v1.md`.

**Lead with the suite at the ceiling commit and the suite after the repair**, so the cost of the raise and the effect
of the fix are visible as two numbers. **Then** the 0.45 hand measurement, and the absent-ref test's output.

**Push your own commits once approved.** Handoff B — the rest of the re-baseline — follows this one.

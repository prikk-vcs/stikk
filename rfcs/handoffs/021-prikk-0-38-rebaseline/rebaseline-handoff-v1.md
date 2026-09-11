# Handoff B — the prikk 0.38 re-baseline (v1)

**Companion to:** [RFC 021](../../accepted/021-prikk-0-38-rebaseline.md) (Accepted 2026-09-12).
Handoff A (F0) has landed; this is everything else in the RFC.
**Realizes:** ceiling 33 → 38; `UD-09`'s retirement; the amendments to `FR-030/033/034/051/052/103`.
**Design items:** `ASM-2`/`NFR-R03`, `TS-03`, `TS-07`, `UD-02`, `UD-09`, `C-T2c′`, `T-T4`.

> **This is RFC 019's suite's first real job.** Five prikk releases at once, and the suite exists so
> that a re-baseline is *run, read, decided* rather than performed by hand from five changelogs.
>
> **Builds no views.** RFC 021 Decision 5: Patch detail, the Queue view, three-valued `Ready`, and
> Compare are each their own increment after this one. Three releases of unblocked surface is exactly
> where one increment quietly becomes four; this one moves the claim and captures the evidence, nothing
> more.

---

## 1. Scope

**In**, in this order — the order is enforced, not preferred:
1. **Raise the ceiling** (§2). First. The suite's version guard refuses 0.38 until this moves.
2. **Run the suite at 0.28 and 0.38, and read it** (§3).
3. **Re-capture every fixture** across the five versions (§4).
4. **Amend the requirements** to what is now knowable (§5).
5. **Record the new upstream dependency** and the cost curves (§6).
6. **Changelog** (§7).

**Out:** every view. Any change to what stikk *renders*. `worktree-status --format json` (floor is
0.28). The symlink-refusal handling (carried; §6). The `--content-path` seam method — that is Compare's
increment, which needs the design work RFC 021 Q1 records, not a re-baseline.

## 2. The ceiling — first, alone, and then stop

`VALIDATED_MAX_MINOR = 38` in `version.rs`. Commit it **before touching anything else**. The doc comment
gets its fourth entry in the raise history, and it records that this raise spans **five** releases —
the previous three each spanned one.

**Then run §3 before any fixture moves.** The point of raising first is that the suite tells you what
the raise cost; if you re-capture first, the suite has nothing to find.

## 3. The suite's first real job

`cargo install prikk --version 0.38.0`, point `STIKK_TEST_PRIKK_CEILING_BIN` at it, run the suite at
0.28 and 0.38. **Expect failures.** Six of the classifier fixtures encode `lock conflict:` prefixes that
0.35 retired, and the suite diffs captures against committed fixtures and never rewrites them.

**Report the failures verbatim, both sides, before fixing anything.** That output is the re-baseline's
finding, and it is the first time this project gets it from a machine rather than a person. Then fix
per §4, re-run, and report the green run.

**If the suite finds nothing at 0.38**, check the deliberate-mismatch test twice and say so plainly — a
first real run that passes is more likely inert than correct. But six stale fixtures say it will find
something.

## 4. Fixtures — five versions, and the rule has not changed

**Captured, never written.** Every fixture that changes carries a provenance line naming 0.38.0 and the
command. Every fixture that does *not* change gets the re-verification paragraph
`parse/tests.rs` already uses (RFC 017 C2's shape), not a re-stamp — *re-verified unchanged* is a
weaker claim than *re-captured* and must not be dressed as the stronger one.

What to expect, from RFC 021 and the letters — **verify, do not copy**:

| Surface | Expected at 0.38 | Source |
|---|---|---|
| six precondition refusals | `lock conflict:` → `precondition not met:`, text unchanged | 0.35, letter 003 reply §3 |
| `trust maintainer add` output | `policy: required=1` → `adopted maintainer keys: N` | 0.34 |
| `worktree-status` | `live rename declarations: N` unconditional | 0.38, **already captured in Handoff A** |
| `status`, `log`, `branch list`, `tag list`, `commit`, `seal` | letter 005 reply claims no shape change 0.33 → 0.38 | **unverified — capture** |

**The `--help` at 0.38 also lists `--content-path`, `bundle preview`, `mv`, and `show`.** None is a
stikk seam method yet; **do not add one here.** Record their existence in the `UD-09` entry (§5) and
stop.

**One capture to make deliberately, for the next increment to inherit:** `status --format json` on a
queue where a file was edited then deleted before sealing — the `unresolved_node_id` case. RFC 021 F6
reached it; a committed capture is what lets the Queue view's increment build against it without
re-deriving the sequence.

## 5. The requirement amendments — the same discipline, upward

`FR-051` and `FR-052` were amended **down** when the surface was absent, each recording the upstream ask
that would restore it. **The surface arrived. Amend them back up, and say which release did it.**

- **`UD-09`** — content half **retires at prikk 0.36** (`show`). Record precisely what remains:
  no arbitrary-point comparison (`diff` refused as out of scope — a refusal, not a deferral), and no
  `log`/`branch`/`tag` `--format json` (asked for in letter 006 §1; prikk offered to schedule it on our
  say-so, and we said yes).
- **`FR-030`** (Patch detail) — **unblocked at 0.36**. Note RFC 021 F2: spans, never synthesized lines.
- **`FR-034`** — per-file diffs unblocked at 0.36; the view is still path-level until its own increment.
- **`FR-051`** — the Queue view **unblocked at 0.35** (`status --format json` enumerates the queue, and
  carries `threshold_status`/`warn_threshold`/`hard_limit` as data). Keep the amendment's history; add
  the release that reversed it.
- **`FR-052`** — "which patches" **satisfiable at 0.35**. The ceremony may now name them. Same shape.
- **`FR-103`** — `trust maintainer list`/`check` **exist at ≥ 0.34**; `Ready` is constructible there.
  **Version-conditional**: below 0.34 the answer is still `Unknown`, and the floor is 0.28.
- **`FR-033`** — the one that narrows *and* widens, per RFC 021 Q1 as amended 2026-09-09:
  **branch-tip comparison in full** (`--content-path` at each tip, compare two contents prikk handed
  us); **same-ref range comparison as state-level difference folded from reported operations, plus each
  block's own spans**; and **block-addressable content carried as a named dependency** — `--ref`
  resolves a published branch only, verified three ways in the RFC.

**Every amendment names the prikk release that changed the answer.** The entries are read by someone
deciding whether a version they have is enough.

## 6. New dependency, and two things to carry

**`UD-10` — block-addressable content.** The `UD-09` row's shape: what is missing (`--ref <block>`,
`--ref tags/…`, `branch create --from <block>` all refuse), what it blocks (`FR-033`'s range case), and
what stikk does meanwhile (state-level + per-block spans, honestly labelled). Letter 006 §5 filed it.

**Carried, not this increment's:**

- **prikk's cost curves.** Their documentation now states sealing rises quadratically with history
  depth and checkout/merge-evidence at roughly `depth^1.45`, tracking depth not size. Record it beside
  `NFR-P04` — a Compare view over a deep range is exactly the shape that pays it, and the design must
  see the curve before a user does.
- **A symlink is reported `untracked` while `unsupported paths: 0`, and one blocks every commit.**
  Letter 006 §4 reported it; the shape is prikk's to decide. For stikk: the refusal (`integrity error:
  worktree authoring: unsupported symlink authoring`) degrades to a verbatim `Refusal` today, which is
  honest. **Whether commit's preview should *prevent* it** — the posture we took for cross-ref and
  empty-commit — depends on what prikk does with the report. Carry it against RFC 014, not here.

## 7. Changelog

`## Unreleased` → **`### Changed`**: validated range now **≥ 0.28 through 0.38.0** (was 0.33.0). Say
what the suite found across the five versions — the six reclassified refusals, the rename section, the
trust-add line — and **say it was found by the suite**, because that sentence is the reason RFC 019
exists and this is the first time it is true.

Under **`### Added`**, nothing: no view landed. If you find yourself writing an Added line, something
left scope.

## 8. Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
```

**Plus the suite at 0.28 and 0.38, green, its output read and pasted.** And the sweep:
`git ls-files | xargs grep -ln "0\.33"` — every live statement of the old ceiling moves, the two named
exceptions stay.

## 9. Acceptance criteria

1. `VALIDATED_MAX_MINOR = 38`, committed first and alone.
2. The suite's failing run at 0.38 reported verbatim **before** any fixture changed; its green run after.
3. Every changed fixture re-captured with 0.38.0 provenance; every unchanged one re-verified, not
   re-stamped.
4. The `unresolved_node_id` capture committed for the Queue view's increment.
5. Six requirements amended, each naming the prikk release that changed the answer; `UD-09`'s
   remainder stated precisely.
6. `UD-10` recorded in the dependency table; the cost curves and the symlink report carried.
7. No seam method added; no view touched; no `### Added` line.
8. Changelog says the suite found it.
9. All gates green; the `0.33` sweep clean.
10. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/021-prikk-0-38-rebaseline/review-request-v1.md`.

**Lead with §3's failing run** — the suite's own diffs, both sides. That is the artefact this whole
increment exists to produce, and the first time this project has had one.

**Then tell me what the five versions changed that neither RFC 021 nor the letters predicted.** Five
releases is the widest jump this project has made; the letters list what prikk *meant* to change, and
the suite finds what *did*.

**Push once approved.** Then 0.5.0's release proposal — which now carries five increments and a
platform matrix that has never run.

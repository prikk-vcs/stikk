# Handoff A — the prikk 0.41 re-baseline, and a harness that must change first (v1)

**Companion to:** [RFC 026](../../accepted/026-readiness-after-the-key-directory.md) (Accepted
2026-09-12; **Q1 ruled (b)** by the architect after 0.41 published — read it, it decides what happens on
0.40).
**This handoff is the re-baseline and the three JSON parsers.** **B is the readiness rebuild**, and it
cannot start until this lands: readiness is built against `key status`, and we do not build against a
version we have not validated.
**Design items:** `ASM-2`/`NFR-R03`, `TS-03`, `TS-07`, `UD-02`, `C-I1a–e`.

> **Read §2 first.** Every previous re-baseline began by running the suite. **This one cannot** — the
> suite's own fixture builder configures prikk through `PRIKK_*_SEED`, which prikk 0.40 retired. The
> harness has to be rebuilt before the suite can tell you anything about 0.41.
>
> **Three prikk releases, and one of them changed how prikk is configured at all.** Expect this
> re-baseline to find more than the last four.

---

## 1. Scope

**In**, in this order — the order is enforced by what works, not by preference:
1. **The harness** (§2). Nothing else can be measured until it runs.
2. **The ceiling**, 38 → 41, alone (§3).
3. **The suite at 0.28 and 0.41**, failing run reported before any fixture moves (§4).
4. **Fixtures re-captured across three releases** (§5).
5. **The three JSON parsers** — `log`, `branch`, `tag` (§6).

**Out:** everything in Handoff B — the readiness model, `key status`, the key-id display gate, `C-S2`,
and RFC 023's two render leftovers. **`env.rs` is not touched in this handoff.** It is wrong at 0.40+
and B is where it is rebuilt; changing it here would mean changing it twice.

## 2. The harness, before anything else

`Fixture` sets `PRIKK_AUTHOR_SEED` / `PRIKK_MAINTAINER_SEED`. At **0.40** setting either is a **refusal**;
at **0.41** it is silently ignored and prikk reads its key directory instead. **Either way the suite
stops configuring the binary it is testing.**

**Use `PRIKK_*_SEED_FILE`, not a key directory.** Verified on a real 0.41.0:

```
PRIKK_AUTHOR_SEED_FILE=<fixture>/author.seed  →  source: seed-file-override · usable: true
```

**Why the override and not `XDG_CONFIG_HOME`:** each test owns its own repository *and its own keys*
today, and a shared key directory would couple them — the isolation RFC 022 measured at ~0.3s a fixture
and kept deliberately. The override keeps one seed pair per fixture with no shared state.

**But the floor still needs the old path.** prikk 0.28 has no `PRIKK_*_SEED_FILE`. So the harness picks
by version, exactly as it already picks between `prikk setup` and manual key derivation: **`_SEED` at
≤ 0.39, `_SEED_FILE` at ≥ 0.40.** Put both behind the one function that already chooses by era.

**And `prikk key public --seed-env` is gone at 0.40**, replaced by `--seed-file` / `--role`. The
harness's pre-0.33 derivation path uses `openssl`, so it is unaffected — **check that rather than assume
it**, and say which paths you found using the retired form.

## 3. The ceiling

`VALIDATED_MAX_MINOR = 41`, committed **alone**, after §2 and before §4. The suite's version guard
refuses a 0.41 binary until it moves, and re-capturing before running leaves the suite nothing to find.

## 4. The suite's run — report the failure first

Run at **0.28 and 0.41**. **Report the failing run verbatim, both sides, before changing any fixture.**

**Expect failures this time.** Unlike RFC 021's re-baseline — where the suite passed because the four
surfaces it covered had not moved — the suite now covers **ten** surfaces and three releases have
landed, one of which changed prikk's configuration model. **If it passes clean, something is wrong with
the harness rather than lucky with prikk**, and the deliberate-mismatch test is the first thing to
re-check.

## 5. Fixtures — three releases

Every changed fixture re-captured with 0.41.0 provenance; every unchanged one **re-verified, not
re-stamped** — *re-verified unchanged* is the weaker claim and must not be dressed as the stronger.

**Known from prikk's changelogs — verify each, do not copy:**

| Release | What moved |
|---|---|
| 0.39 | `worktree-status` now names paths `commit` would refuse; `branch` no longer lists tag refs; three trust refusals and two authoring refusals reclassified to `precondition not met:` |
| 0.40 | `setup` output (key directory, `using your keys in <dir>` on a second project); `key public` form; `PRIKK_*_SEED` refusal |
| 0.41 | that refusal removed; `--help` rewritten |

**`branch` no longer listing tag refs is the one to look at hardest.** stikk merges branches and tags
into the ref picker (`FR-014`). If prikk was previously including tags in `branch list`, stikk may have
been double-counting them — **check what the ref picker actually showed before and after**, and report
it either way.

## 6. The three JSON parsers

0.39 gave `log`, `branch` and `tag` a `--format json` — `log-report-v1`, `branch-list-v1`,
`tag-list-v1`. These are **the entire remainder of stikk's prose parsing** (`cli_backend.rs:209/227/233`)
and each has already cost a re-baseline.

**Version-gate them**: JSON at ≥ 0.39, the existing prose parsers below. **Do not delete the prose
parsers** — the floor is 0.28 and they are the only thing that works there.

**The prose fixtures stay too**, as the ≤ 0.38 path's regression suite. A parser that is still reachable
is a parser that is still tested.

## 7. Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
cargo deny check
```

**Plus the suite green at 0.28 and 0.41, full platform matrix.** The harness change in §2 touches the
Windows leg's key handling, which is where the last matrix run broke — run it there and name the run ID.

**And the `0.38` sweep**: `git ls-files | xargs grep -ln "0\.38"`, every live claim moved, the named
exceptions kept.

## 8. Acceptance criteria

1. The harness configures prikk by version — `_SEED` at ≤ 0.39, `_SEED_FILE` at ≥ 0.40 — with per-fixture
   isolation kept; any use of the retired `key public --seed-env` found and reported.
2. `VALIDATED_MAX_MINOR = 41`, committed alone, after §2 and before §4.
3. The suite's failing run reported verbatim **before** any fixture changed; the green run after.
4. Fixtures re-captured with provenance or re-verified unchanged — the two claims kept distinct.
5. `branch list`'s tag-ref change checked against the ref picker's actual output, reported either way.
6. `log`/`branch`/`tag` parse JSON at ≥ 0.39 and prose below; prose parsers and their fixtures retained.
7. **`env.rs` untouched** — `git diff` on it is empty.
8. All eight gates green; suite green on the full matrix at 0.28 and 0.41, run ID named.
9. Nothing tagged or published.

## 9. Submit

Package to `.git-exclude/review-request/026-a-rebaseline/review-request-v1.md`.

**Lead with §4's failing run** — the suite's own diffs, both sides. Three releases including a
configuration change is the widest this project has re-baselined, and that output is the increment's
evidence.

**Then tell me what the three releases changed that neither the RFC nor prikk's changelogs predicted.**
Their changelogs are unusually good and I have read them — so anything the *binary* shows that the
*notes* did not is the finding worth having.

**Push once approved.** B follows.

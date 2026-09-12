# Handoff — widening the real-binary suite (v1)

**Companion to:** [RFC 022](../../accepted/022-widening-the-real-binary-suite.md) (Accepted 2026-09-12;
**Q1 ruled by the architect** the same day, on evidence — read that section first, it changed the
premise). **Opens 0.6.0.**
**Design items:** `TS-07`, `TS-03`, `UD-02`, `UD-05`, `NFR-T01`, `T-T4`, `FR-106`, `ER-02`.

> **The suite's first real job found nothing in what it checks, and the six stale fixtures that
> re-baseline did move were found by hand.** That gap is this increment. It is five of ten seam methods,
> not four of nine — `handshake` was covered all along and I miscounted it into a shipped changelog.
>
> **Widening is cheap and that is measured, not hoped**: the 0.5.0 prep runs were 64s/74s/116s per
> platform, dominated by two `cargo install prikk` invocations, with the test body at 3.25s for ten
> fixture repositories. The expensive part is already paid.

---

## 1. Scope

**In:** the five unexercised surfaces (§2); F0's regression test moved into the suite (§3); the
classifier's reachability notes (§4); harness-vs-product failure (§5); the checkout action (§6); two
documentation corrections (§7).

**Out:** widening the **version** matrix — both ends, as now; exercising the middle is a different
question with a different cost curve and nothing has asked for it. A `schedule:` trigger (the owner's,
still open, on both workflows). Any product behaviour change other than §7's one-line doc fix. Every
0.6.0 feature increment — the key-id module, Glossary wrap+scroll, and the four views each wait their
own RFC.

## 2. The five surfaces — assert state, not parse

`worktree_status`, `block_state`, `refs`, `tags`, `change_token`, at **both ends**. RFC 019's discipline
is the reason that suite found anything: *does stikk's answer match the repository, not does the string
parse.*

- **`refs` / `tags`** — after a real `branch create` and `tag create`. `tag create` needs MAINTAINER
  readiness, which `Fixture` already establishes. Assert the created name **appears**, and that a
  `branch close`d branch is reported the way `--all` reports it.
- **`block_state`** — on a block the suite sealed itself, so the expected paths are known from what was
  committed rather than from a fixture's say-so.
- **`change_token`** — the one with a property rather than a value: **it must differ across a mutation
  and be identical across a read.** Assert both directions; a token that never changes and a token that
  always changes both pass a single-sided test.
- **`worktree_status`** — a dirty tree (modified + missing + untracked), and §3.

**Each needs its own `Fixture`**, per RFC 022 Decision 5 — keep per-test isolation, and put the ~0.3s
measurement in the comment so the next person tempted to share fixtures finds the reason rather than the
opportunity.

## 3. F0's regression test, moved from a fixture to a binary *[the highest-value single addition]*

F0 — stikk fabricating a Changes entry for a file that does not exist — is the one defect of its class
that **reached a shipped release**, and it is currently pinned by a captured fixture only.

**Provoke it**: at **0.38 only** (`prikk mv` does not exist below 0.33), create a path whose first
whitespace-delimited token is a change kind (`"modified draft.txt"`), commit, seal, `prikk mv` it, then
drive `Prikk::worktree_status` and assert **exactly two entries** and **no entry whose path contains
`->`**.

**Version-gate it the way the suite already gates by `bin.minor`** — and do not let the 0.28 end skip
silently: a test that quietly does nothing at one end is the inert-suite failure mode RFC 019's own
review warned about. If it skips, it says so.

## 4. The classifier — provoke what can be reached, annotate what cannot

**Q1 is ruled (a): the three source-read arms stay.** Read the RFC's ruling for why — the short version
is that `present()` adds **no gloss** to a `LockConflict`; it renders prikk's verbatim message in a
banner, so an unreachable arm's worst case is a banner instead of a refusal card, with no fabricated
claim either way.

**What each arm now carries, in the code**, and this is the deliverable rather than the reasoning:

| Arm clause | State | What the note must say |
|---|---|---|
| `lock already exists` | captured | keep the provenance it has |
| `belongs to a different repository authority` | source-read | needs a second repository authority; what would make it reachable |
| `cas mismatch` | source-read | **prikk documents their guard as unable to fire through the publish path**, kept against a future locking regression — quote them, and say stikk keeps the arm for the same reason |
| `changed during planning` | source-read | needs an in-flight rollback draft stikk does not build |

**A citation without a reachability note is what RFC 017 was about, one step removed.** The note is
checked at each re-baseline like a fixture's provenance: if a reason stops being true, the arm becomes
provokable and should be provoked.

**Then provoke what is reachable and not yet provoked.** Work the arms against real binaries and report
which moved from source-read to captured. The six precondition fixtures RFC 021 B moved by hand are the
measure of success: **the next re-baseline should find them without a person.**

## 5. Harness failure is not a test failure

RFC 021's Windows break produced **five identical panics** about `prikk key generate`, because every test
builds a fixture first. At ten-plus surfaces that is a misleading picture of a release's health, on the
gate whose only job is telling a human whether to ship.

**Make fixture construction failure distinguishable and said once.** Shape is yours — a single
precondition check before the suite body, a harness-error type that reports differently, whatever fits
what is there. **The requirement is the outcome:** a reader of a failed run can tell in one line whether
the harness could not build a repository or stikk got an answer wrong.

## 6. `actions/checkout`

Both dispatch-only workflows carry the Node 20 deprecation on every job, force-run on Node 24 today.
When the forcing stops they fail **at checkout** — and these are the two workflows nobody triggers
between releases, so it would land at a release, on a gate that blocks it. **Pin to a release that does
not depend on the forcing.** Check `ci.yml` and `docs.yml` for the same action while you are there, and
say whether they needed it too.

## 7. Two documentation corrections

- **`stikk-model/src/error.rs:29`** says `LockConflict` is *"Presented as 'another writer is active'"*.
  **It is presented as prikk's verbatim message in a banner.** True when written, false now — the F1
  shape in the error taxonomy's own docs, and the source of the wrong premise in RFC 022's own Q1.
  **Say what it is presented as**, and that the gloss's absence is deliberate (`ER-02`).
- **The coverage claim.** 0.5.0's changelog says the suite covers *"four of the nine surfaces"*. It is
  **five of ten** — `handshake` is driven by the version guard on every test at both ends. Correct it in
  `## Unreleased`, name that 0.5.0's entry understated it, and state the new number this increment
  reaches. **Do not edit 0.5.0's released section.**

## 7b. The index drifted from the folders for two releases — gate it

**Found while issuing this handoff.** `rfcs/README.md` says *"The folder is the source of truth for an
RFC's state, and the `Status` field inside each file mirrors it"* — and the index had **four shipped RFCs
(018, 019, 020, 021) still listed under Accepted**, RFC 018 still in `accepted/` a release after it
shipped as 0.4.1, RFC 022 in no table at all, and a link to a path that had moved. I repaired it by hand
(`rfcs/` is the architect's, so the repair is already committed); **what belongs to you is stopping it
recurring.**

**Add a test that asserts the index against the folders**, in whichever crate can see the repo root the
way the `C-I1e` boundary test does:

- every `rfcs/{proposed,accepted,done,archive}/NNN-*.md` appears in exactly one table, **the one matching
  its folder**;
- every `./<folder>/NNN-*.md` link in the index **resolves to a file that exists**;
- each file's `Status` line names the state its folder implies.

**This is the same defect class as every sweep finding this project has made** — a claim that was true
when written, in a file nothing reads mechanically. The difference is that this one is cheap to gate,
because both sides are on disk.

## 8. Gates

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

**Plus the suite at 0.28 and 0.38, green, and the full platform matrix** — Windows is where the last
first-run failed, and §2 adds five surfaces and §3 a `prikk mv` to ground nobody has exercised there.
The `prep/*`-branch dispatch pattern is sanctioned (spec §6) if you need a runner before approval.

## 9. Acceptance criteria

1. All five surfaces driven at both ends, each asserting repository **state**; `change_token` asserted in
   **both** directions.
2. F0 provoked against a real 0.38 binary; skips at 0.28 are announced, not silent.
3. Every `is_lock_conflict` clause carries provenance or a reachability note; arms that became
   provokable were provoked, and the report says which.
4. A failed run distinguishes harness failure from product failure in one line.
5. `actions/checkout` pinned; the other two workflows checked and reported.
6. `error.rs`'s `LockConflict` doc says what is actually presented; the coverage claim corrected in
   `## Unreleased` with 0.5.0's understatement named.
7. Per-test isolation kept, with the measurement in the comment.
8. All eight gates green; suite green on the **full platform matrix**, run ID named.
9. No product behaviour changed beyond §7's doc fix.
10. A test asserts the RFC index against the folders (§7b): one table per file, matching its folder;
    every link resolves; `Status` mirrors state.
11. Nothing tagged or published.

## 10. Submit

Package to `.git-exclude/review-request/022-widening-the-real-binary-suite/review-request-v1.md`.

**Lead with the coverage table** — ten methods, which are driven, which assert state — so the claim this
increment makes about itself is checkable at a glance rather than asserted.

**Then tell me what driving five new surfaces against two real binaries found.** RFC 019's first run
found nothing in its four; RFC 021's first matrix run found a Windows failure nobody predicted. **Five
new surfaces on ground nothing has exercised is the widest net this suite has cast** — and if it comes
back empty, say so plainly and say how you convinced yourself it was not inert.

**Push once approved.**

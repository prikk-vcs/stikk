# RFC 019 — The real-binary integration suite, and what four releases were verified by

**Status.** **Done** — shipped on `main` 2026-09-07 (`bdd6871` + `561c05a`), 0.5.0 candidate. Accepted 2026-09-06; proposed the same day.
**Deferred, carried forward:** a `schedule:` trigger (owner's, deliberately open); the full `NFR-T01` platform matrix has never actually run (the Windows leg's `openssl` dependency is untested — flagged in the review request, not assumed fine). Opens 0.5.0. Deferred since **RFC 009** as *"acceptable"*; that
judgement was made about a read-only product and **0.4.0 mutates repositories that hold people's
work.** The owner accepted shipping 0.4.0 without it on my recommendation, on the explicit
understanding it becomes 0.5.0's first increment.
**Tracks.** `TS-07`, `NFR-T01`, `TS-03` (captured fixtures), `UD-02`, `ASM-2`/`NFR-R03`.
**Touches.** a new integration test target, `.github/workflows/ci.yml`, `.git-exclude/specs/`.

## Summary

**Nothing in stikk's CI has ever run prikk.** Every one of 435 tests runs against `NullBackend` or a
captured string. Four releases — including the one that writes — were verified against a real binary
by a person building prikk from a tag, running commands by hand, and pasting the output into a review
request.

**That ritual has worked**, and it is worth saying why before replacing it: it found RFC 015's
straddling-block case, RFC 016's dynamic `RefState` ref, RFC 017's five ungrounded arms. **A human
looking at real output is not the weak part.** The weak part is that the ritual is *not repeatable* —
it leaves no artefact, runs only when someone remembers, and its results live in a review request
rather than in a gate.

Two things have changed since the deferral, and both raise the cost of leaving it deferred:

- **stikk writes now.** A defect in commit or seal damages a repository; a defect in a read renders a
  wrong screen.
- **prikk moves faster than we re-baseline.** It shipped **0.29 → 0.35 during this project's lifetime,
  two of those (0.34, 0.35) in the hours since our last re-baseline.** Manual re-verification is
  already the bottleneck.

## What this is not

**Not a replacement for looking.** The suite must make the ritual cheap and repeatable, not remove the
human from it. Every finding listed above came from someone *reading* output, not from an assertion
passing. A suite that turns re-baselining into a green check nobody reads would be a net loss, and
§Decisions 4 is written against that specific failure.

## Findings

### F1 — the manual ritual is undocumented and lives in review requests

Three increments performed real-binary verification. Each did it differently, each described it in
prose, and **each deleted the test afterwards** — *"this project has no checked-in real-binary tests"*
is stated in two review requests as though it were policy. It is not policy; it is the absence of one.

There is no record of **what was verified**, only that someone verified something. Re-answering *"was
seal's empty-queue refusal ever checked at 0.28?"* requires reading a review request.

### F2 — the gap is exactly where the risk moved

Fixtures cover **parsing**. `NullBackend` covers **operation branching**. Neither covers what happens
when stikk actually invokes prikk:

| Covered today | Not covered |
|---|---|
| Does this string parse? | Does prikk still emit this string? |
| Does the operation branch correctly on a scripted refusal? | Does prikk still refuse in that case? |
| Does the classifier route this message? | Does a real repository reach this message? |
| — | **Does `commit` leave the repository in the state stikk claims it did?** |

**The last row is the one 0.4.0 added and nothing checks.** stikk renders a commit result parsed from
prikk's stdout. Nothing asserts the patch is *in the queue afterwards*.

### F3 — prikk is on crates.io, so the hard part is already solved

`prikk` publishes to crates.io (0.29 through **0.35.0** available). CI can install an exact version
with `cargo install prikk --version <X> --locked`. **No submodule, no source checkout, no build from
git.** That removes the objection that made this expensive when RFC 009 deferred it.

### F4 — two versions matter, not one

`ASM-2` supports prikk **≥ 0.28**, validated through a ceiling. A suite that runs only at the ceiling
would not have caught anything RFC 016's floor-end verification caught, and the floor is where
behaviour is most likely to differ silently. **The suite runs at both ends of the supported range**,
and the "both ends" set is derived from `version.rs`, not hardcoded — so raising the ceiling
automatically widens what is tested.

## Decisions

1. **A checked-in integration suite**, behind a cargo feature or `#[ignore]`, driving `CliBackend`
   against a real binary through a temporary repository: **init → commit → seal → verify**, per
   `TS-07`. Not a new backend, not a new abstraction — the same `CliBackend` the product uses.
2. **It asserts repository *state*, not just stikk's parse.** After a commit, the queue depth
   increased; after a seal, the queue is empty and a new block exists. **This is the half nothing has
   ever covered** and the reason the suite exists at all.
3. **It runs at both ends of the supported range** (F4), versions derived from `version.rs`.
4. **It captures fixtures and diffs them; it never rewrites them.** A mismatch **fails and prints
   both sides**. Auto-updating fixtures would convert RFC 009's hardest-won rule into a rubber stamp —
   the rule is *captured, never written*, and a machine that writes what it captured without a human
   reading the diff satisfies the letter and destroys the point.
5. **It does not gate `ci.yml`'s main job.** A separate workflow, so an upstream build break or a
   crates.io outage does not redden stikk's PRs for a reason stikk did not cause. **Required before a
   release; advisory on a PR.** Where exactly it runs — nightly, on demand, on release branches — is
   Q1.
6. **Retire the "no checked-in real-binary tests" folklore.** Replace it with the actual rule in the
   handoff spec: real-binary tests are checked in, do not run by default, and a release requires a
   green run whose output a human has read.

## Open questions

### Both ruled by the architect, 2026-09-06

**The owner accepted without answering these, and neither needed them.** Both are scheduling and cost
decisions, which is the architect's remit under the owner's standing ruling (*"to make schedule and
manage release cycles is your role"*). Ruled below; if the owner intended otherwise they can say so
and these move.

**Q1 RULED: on demand, and required in release prep. No nightly yet.** The release-prep requirement is
the binding half — that is where the risk actually materializes, and it puts a human in front of the
output, which §Decisions 4 depends on. **Nightly is deferred until someone owns its failures**, because
a nightly that reddens for an upstream reason and gets muted is worse than no nightly: it creates
assurance nobody is actually providing. Revisit when the suite has a maintainer, not on a schedule.

**Q2 RULED: Linux for the routine run; the full `NFR-T01` matrix in release prep.** Every finding this
project has made against a real binary has been platform-independent — but **RFC 012 F-c found that
0.1.0 and 0.2.0 shipped binaries for macOS and Windows without ever resolving paths on them**, and that
was caught by review, not by test. Platform-specific breaks here are real and have gone unnoticed
before. Release prep is where the full matrix earns its cost, and it is cheap there because it runs
once.

---

**Q1 (original) — when does it run?** Nightly, on demand, or on every release-prep increment? Nightly finds
upstream drift without anyone asking, and prikk has shipped seven releases in this project's lifetime,
so drift is the normal case rather than the exception. Against: a nightly that fails for an upstream
reason and is muted becomes worse than nothing. **My lean: on demand + required in release prep, with
nightly added only once someone owns the failures.** Not settled; it is a process commitment, not a
technical one.

**Q2 — Windows and macOS.** `NFR-T01` says mutating stikk runs where mutating prikk runs: Linux,
macOS, Windows. Running the full matrix triples cost for a suite whose findings have all been
platform-independent so far. **My lean: Linux for the routine run, the full matrix in release prep**,
so a platform-specific break is caught before a tag rather than never. Wants a ruling.

## What this unblocks, and what it does not

**It does not unblock a feature.** It is the increment that makes the previous four trustworthy, and
its deliverable is a sentence we cannot currently say: *stikk's commit and seal are exercised against
a real prikk on every release.*

**It should be measured by what it finds**, and the honest expectation is that its first run finds
something — because prikk is at **0.35.0** against our validated **0.33.0**, and the re-baseline that
follows is where the suite proves itself. See the note below.

## A note on sequencing — prikk 0.34.0 and 0.35.0 shipped today

**Both of this project's outstanding upstream requests were answered within hours**, and they change
what 0.5.0 contains:

- **0.34.0** — `prikk trust maintainer list` and `check --key-id`, both with `--format json`, `check`
  exiting `0` on a negative answer. This is letter 002's request and prikk's RFC 138 ruling, shipped.
  **It makes `MaintainerReadiness::Ready` reachable for the first time and `FR-103` satisfiable** —
  and it puts RFC 016 Q1's robustness claim (that *unknown* resolves with no change to the type or
  the UI's shape) to a real test.
- **0.35.0** — `prikk status --format json` carrying **each queued patch's id, operation kinds, and
  affected paths**. This is letter 003's second request, and prikk's changelog gives our own reason
  back to us: *"the same information a seal ceremony needs to ask for informed consent before an
  irreversible act, not just a count."* **It unblocks the Queue view (`FR-051`) and lets seal's
  confirmation name which patches it will freeze.** Also: all six preconditions from letter 004 are
  reclassified.

**This does not change my recommendation that the suite comes first — it strengthens it.** A
0.33 → 0.35 re-baseline touching three requirements is exactly the job this suite exists to make
repeatable, and doing it by hand one more time immediately before building the machine would be the
wrong order. **The re-baseline is the suite's first real job and its proof**, and it follows as its
own increment rather than being folded in here.

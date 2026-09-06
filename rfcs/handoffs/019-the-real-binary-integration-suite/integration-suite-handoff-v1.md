# Handoff — the real-binary integration suite (v1)

**Companion to:** [RFC 019](../../accepted/019-the-real-binary-integration-suite.md) (Accepted
2026-09-06; both open questions ruled by the architect the same day). Inherits its state.
**Realizes:** `TS-07`, deferred since RFC 009. **Opens 0.5.0.**
**Design items:** `TS-07`, `TS-03`, `NFR-T01`, `UD-02`, `ASM-2`/`NFR-R03`, `C-I1a`/`C-I1e`.

> **Nothing in stikk's CI has ever run prikk.** 435 tests, four releases, one of which writes to
> repositories — all verified against `NullBackend` and captured strings, plus a manual ritual that
> left no artefact.
>
> **You have performed that ritual three times.** This increment is you writing it down so it runs
> without you.

---

## 1. Scope

**In:** the harness (§3), what it asserts (§4), fixture capture-and-diff (§5), the version matrix
(§6), where it runs (§7), retiring the folklore (§8).

**Out:** the **prikk 0.34/0.35 re-baseline** — it is the next increment and this suite's first real
job; build the machine, do not also do the work. Anything that changes product behaviour. The Queue
view, `MaintainerReadiness::Ready`, the key-id module, Glossary wrap+scroll — all 0.5.0, none here.

---

## 2. Ruled before you start

**Q1 — on demand, and required in release prep. No nightly.** A nightly nobody owns gets muted, and a
muted gate is worse than none. Revisit when it has a maintainer.

**Q2 — Linux routinely; the full `NFR-T01` matrix (Linux, macOS, Windows) in release prep.** RFC 012
F-c found two releases shipping binaries for platforms whose paths had never been resolved — caught by
review, not by test. Release prep is where the matrix earns its cost.

## 3. The harness — and a boundary that needs stating before you write a line

**Getting binaries is easy now**: `cargo install prikk --version <X> --locked --root <tmp>`. prikk
publishes to crates.io through 0.35.0. No submodule, no git build.

**Building a fixture repository is where the care goes.**

**The boundary, ruled here so nobody has to guess:** `C-I1e` says **stikk** never invokes `prikk key
generate`, `prikk key public --seed-env`, or wraps `prikk setup`. **That binds the product, not the
harness** — but the exemption must be *structural*, not a comment:

- The harness spawns those commands **itself**, with `std::process::Command`, in test code.
- **It must not reach them through `CliBackend`.** The product's command surface never learns those
  subcommands, so `the_command_surface_never_names_key_or_setup` keeps passing **unchanged** — do not
  add an exception to it. If you find yourself editing that test, you have put the call in the wrong
  place.
- **Seeds never reach test output.** Not on failure, not in a panic message, not in a captured
  `Command` output you print. The fixture repository lives in a temp dir and dies with the test.

**Two eras of fixture-repository setup, and you have already solved both.** `prikk setup` exists only
at ≥ 0.33; at 0.28 you used manual Ed25519 key derivation plus `prikk trust maintainer add` (your own
RFC 016 §1). **Put both behind one harness function** that picks by version — that function is the
suite's real complexity, and it is the part that will otherwise be rewritten per test.

## 4. What it asserts — the half nothing has ever covered

`init → commit → seal → verify`, per `TS-07`. **But the point is not that stikk parses the output.**
Fixtures already cover that. The point is:

- After a commit, **the queue actually contains the patch** — asserted by re-reading the repository,
  not by trusting the string stikk parsed.
- After a seal, **the queue is empty and a new block exists.**
- A refusal stikk *prevents* client-side (empty queue, cross-ref) is **actually one prikk would have
  given** — provoke it with prevention bypassed, and confirm prikk refuses. **Our prevention is built
  on a belief about prikk's behaviour; nothing has ever checked that belief.**

**That third bullet is the one I most want built.** RFC 014 and RFC 016 both chose to prevent rather
than classify, on my instruction, based on reading prikk's source. If prikk stops refusing one of
those cases, stikk silently blocks something legal and no test notices.

## 5. Fixtures — capture and diff, never rewrite

The suite captures prikk's output and **diffs it against the committed fixtures**. On a mismatch it
**fails and prints both sides**.

**It must never update a fixture file.** Not behind a flag, not with `--bless`, not "just for
re-baselining." RFC 009's rule is *captured, never written*; a machine that writes what it captured
without a human reading the diff satisfies the words and destroys the reason. **The re-baseline
workflow is: suite fails → human reads the diff → human decides → human updates.**

If you think a blessing mode is worth having, **say so in the review request rather than building it** —
it is a design change and it is mine, and I will probably say no.

## 6. The version matrix — derived, not written

Both ends of the supported range, **read from `version.rs`** (`SUPPORTED_MIN_MINOR` and
`VALIDATED_MAX_MINOR`), so raising the ceiling widens coverage automatically and nobody has to
remember. Today that is **0.28 and 0.33**.

**Do not hardcode 0.35 to get ahead of the re-baseline.** The suite tests the range stikk *claims*; the
next increment moves the claim and the suite follows it. A suite that tests an unclaimed version is
asserting something the product does not promise.

## 7. Where it runs

- **Not in `ci.yml`'s `gates` job.** A separate workflow, manually dispatchable
  (`workflow_dispatch`), so an upstream break or a crates.io outage never reddens an unrelated PR.
- **Release prep requires a green run** whose output a human has read. Add that to the release-prep
  checklist in `.git-exclude/specs/` alongside the gates.
- Full platform matrix on the release-prep path only (§2).

## 8. Retire the folklore

Two review requests state *"this project has no checked-in real-binary tests"* as though it were
policy. **It was the absence of one.** Replace it in `.git-exclude/specs/02-implementer-handoff.md`:
real-binary tests are checked in, do not run by default, and a release requires a green run someone
has read.

## 9. Test plan

- The suite itself, at **0.28 and 0.33**: init → commit → seal → verify, each asserting repository
  state (§4), not only stikk's parse.
- **The prevention checks** (§4, third bullet) — the beliefs RFC 014 and RFC 016 were built on.
- Fixture diffs pass against the committed fixtures at both versions.
- **A deliberate-mismatch check**: prove the suite *fails* when a fixture disagrees, and prints both
  sides. A capture-and-diff suite that has never been seen to fail is a hypothesis.
- The `C-I1e` boundary test passes **unchanged**.
- The existing five gates stay green; the suite does not run in them.

## 10. Acceptance criteria

1. Suite runs against real prikk at both ends of the range, versions derived from `version.rs`.
2. It asserts repository **state** after commit and after seal.
3. It verifies the two client-side preventions against real prikk refusals.
4. Fixtures are diffed, never rewritten; a mismatch fails and prints both sides; proven by a
   deliberate-mismatch test.
5. Fixture-repository setup works at both eras behind one version-picking function.
6. No key material in any test output; `C-I1e`'s test unchanged; no `key`/`setup` call through
   `CliBackend`.
7. Separate workflow, `workflow_dispatch`, not in `gates`; release-prep checklist updated.
8. The "no checked-in real-binary tests" line is replaced with the real rule.
9. All five existing gates green; no product behaviour changed.

## 11. Submit

Package to `.git-exclude/review-request/019-the-real-binary-integration-suite/review-request-v1.md`.

**Lead with the first full run's output at both versions** — including anything that surprised you.

**And tell me what it found.** The honest expectation is that it finds something: the manual ritual
found a defect every time it ran, and this is the first time it runs at 0.28 and 0.33 together, on
mutations, asserting state rather than parse. **If it finds nothing, say that plainly too** — but check
the deliberate-mismatch test twice first, because a suite that passes everything on its first run is
more likely to be inert than correct.

**Push once approved** (`.git-exclude/specs/02-implementer-handoff.md` §6).

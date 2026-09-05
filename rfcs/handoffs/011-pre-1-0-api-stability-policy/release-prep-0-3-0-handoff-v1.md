# Handoff — 0.3.0 release preparation (v1)

**Applies:** [RFC 011](../../accepted/011-pre-1-0-api-stability-policy.md)'s versioning policy — a
public API break before 1.0 is a **minor** bump. This is its second application; the first was
[0.2.0's prep](./release-prep-handoff-v1.md), which is worth reading beside this one.
**Release content:** [RFC 010](../../done/010-off-thread-seam-and-ui-responsiveness.md) +
[RFC 012](../../done/012-post-0-2-0-correctness-sweep.md), both on `main`, both reviewed and approved.
**Realizes:** the owner's release authorization of 2026-09-05.

**No behaviour changes.** This is the version, the changelog, and four pieces of user-facing text that
0.3.0 made stale. If you find a behaviour bug while doing it, **stop and report** rather than fixing it
inside a release-prep commit.

---

## 1. Scope

**In:**
1. Workspace version `0.2.0` → **`0.3.0`** (six strings), `Cargo.lock` regenerated.
2. `CHANGELOG.md`: a **new** `## 0.3.0 — <date>` section with a `### Breaking` subsection (§3).
3. The prikk validated range, **stated in four places**, moves from `through 0.30.0` to
   `through 0.31.0` (§4).
4. `docs/src/guide/getting-started.md`: the key reference is **missing `o`** (§5).
5. `docs/src/contributing/releasing.md`: the `## What a v0.2.x release is` heading and body (§5).
6. `docs/src/index.md`: the status paragraph still describes 0.2.0's content (§5).

**Out:** any behaviour change; RFC 003 or 0.4.0 work; the 19 rustdoc warnings (scheduled separately —
do **not** fix them here, it would make this a code change); tagging, pushing, publishing.

---

## 2. The version bump

Six strings in the root `Cargo.toml` — one in `[workspace.package]`, five in
`[workspace.dependencies]`. Then, **in this order** (`--locked` verifies a lock, it never rewrites one):

```sh
cargo update --workspace     # rewrites Cargo.lock's workspace-member versions
cargo build --locked         # verifies clean
git add Cargo.lock
```

`release.yml`'s guard requires **tag == workspace version**, and its publish job runs
`cargo publish --locked` per crate in dependency order.

---

## 3. The CHANGELOG

**Unlike 0.2.0, there is no `## Unreleased` section to rename** — write `## 0.3.0 — <date>` fresh above
`## 0.2.0`.

The `### Breaking` subsection must name **all four** breaks, with crate, item, and what a caller does
about it. Verify each against `git log 6f12bb6..HEAD` rather than trusting this list:

| Crate | Change | Who it breaks |
|---|---|---|
| `stikk-prikk` | `Prikk` gained the `Send + Sync` supertrait | anyone implementing `Prikk` outside the crate |
| `stikk-prikk` | `Prikk::tags` added (a required method) | same |
| `stikk-model` | **`Capability::may_operate` removed**; `Readiness::may_operate` added in its place | anyone calling it — and the *semantics* changed too: read-only now denies recovery |
| `stikk-tui` | `App`'s navigation methods no longer take `&impl Prikk`; results arrive via `App::apply` | anyone driving `App` directly |

Then the ordinary sections. The substance worth leading with, in your own words:

- **The UI no longer blocks.** `NFR-P01` was a Must and was unmet — every seam call froze the render
  loop. Reads now run on a worker, load states are observable, and a response for a view the user has
  navigated away from is discarded rather than surfacing over an unrelated screen.
- **Config and state now resolve on macOS and Windows** — platforms 0.1.0 and 0.2.0 shipped binaries
  for without resolving paths for. Linux paths are unchanged.
- **Read-only now denies recovery actions** (`FR-121`), which it did not.
- **Version skew stops pointing users at their signing keys.**
- **Tags appear in the ref picker**, and a repository written by prikk 0.31 that an older prikk refuses
  now gets an explanation instead of a bare refusal.
- **Validated against prikk ≥ 0.28, through 0.31.0** — fixtures re-captured against 0.31 and
  byte-identical.

Say plainly that there are still **no mutations** and that cancellation is deferred.

---

## 4. The prikk range — four places, all currently "through 0.30.0"

Grep, do not trust this list:

- `README.md:50`
- `docs/src/guide/getting-started.md:23`
- `docs/src/contributing/releasing.md:90`
- and the code's own ceiling, **already at 31** (`VALIDATED_MAX_MINOR`, raised by RFC 012) — do **not**
  touch it; the docs are what lag.

`grep -rn "0\.30\.0" README.md docs/` should return nothing about the *validated ceiling* when done.

---

## 5. The three stale doc surfaces

**`docs/src/guide/getting-started.md` — the key reference is missing `o`.** RFC 010 added
`Action::OpenOperations`, bound to `o`, opening the Background Operations overlay. The guide lists
`?`, `r`, `q`, `:`, `b`, `w`, `u`, `R` — eight of nine. Add it, and describe it as a *listing* (it has
no cancel action; that is deferred to `FR-100`) so the doc does not promise a control that is not
there.

**`docs/src/contributing/releasing.md:86` — `## What a v0.2.x release is (and is not)`.** Generalize to
`v0.3.x`. Its substance still holds (still read-only, still drives an external `prikk`); the version and
the range are what move.

**`docs/src/index.md:26` — the status paragraph** still describes 0.2.0's content ("re-baselined against
prikk 0.30 with parser-fidelity corrections"). Rewrite for 0.3.0: the read surfaces plus a
non-blocking UI, per-platform paths, and the prikk ≥ 0.28-through-0.31 range. Keep it to the same
length — it is an orientation paragraph, not a changelog.

---

## 6. Test plan

Nothing behavioural to test. What must hold:

- Gates green with the count **unchanged at 252**. A changed count means something behavioural moved —
  stop and report.
- `cargo build --locked` succeeds against the regenerated lock.
- `cargo build --examples -p stikk-tui --locked` — all four demos.
- `mdbook build` in `docs/`.
- **Read the rendered output yourself**: `stikk --version` prints `stikk 0.3.0`, and the getting-started
  key list matches `keys::dispatch` exactly. A green gate proves none of this.

---

## 7. Acceptance criteria

1. Version and all five path deps at `0.3.0`; `Cargo.lock` regenerated and committed.
2. A `## 0.3.0 — <date>` CHANGELOG section with a `### Breaking` subsection naming all four breaks,
   each verified against the log rather than copied from §3.
3. The validated range reads `through 0.31.0` everywhere it is stated; `VALIDATED_MAX_MINOR` untouched.
4. The guide's key reference includes `o` and does not imply a cancel action.
5. `releasing.md` generalized to `v0.3.x`; `index.md`'s status paragraph describes 0.3.0.
6. Gates green at **252 tests, unchanged**; examples build; `mdbook build` succeeds.
7. No `.rs` file changed. Nothing tagged, pushed, or published.

---

## 8. Submit

Package to `.git-exclude/review-request/011-release-0-3-0-preparation/review-request-v1.md`. As with
0.2.0 this is a **reading review**, so paste verbatim: the `### Breaking` subsection, the getting-started
key list, and `index.md`'s new status paragraph. Note the test count and confirm no `.rs` file moved.

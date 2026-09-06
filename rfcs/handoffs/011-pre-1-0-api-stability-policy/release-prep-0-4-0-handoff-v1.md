# Handoff — 0.4.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md) (adopted; in force since
0.2.0). This is the third release prepared under it.
**Authorized by the project owner** 2026-09-06, on
[the 0.4.0 release proposal](../../../.git-exclude/release/0-4-0-proposal.md).
**Covers:** RFCs [003](../../done/003-repository-change-token.md),
[013](../../done/013-preview-and-confirmation-machinery.md),
[014](../../done/014-commit-the-first-mutation.md),
[015](../../done/015-prikk-0-32-rebaseline.md),
[016](../../done/016-the-seal-ceremony.md),
[017](../../done/017-prikk-0-33-rebaseline-and-classifier-provenance.md).
**Design items:** RFC 011's semver rules, `NFR-R03` (version honesty), `ASM-2`.

> **This release is the one where stikk writes.** Everything else in it is in service of that: the
> change token, the confirmation machinery, two re-baselines, and a correction to a capability claim
> that had been wrong since 0.1.0.
>
> **No code behaviour changes in this increment.** If you find yourself fixing a defect, stop and
> report it — a behaviour fix during release prep is its own increment, and folding one in here would
> mean tagging something no review covered.

---

## 1. Scope

**In**, in this order:
1. **The changelog backfill** (§2) — the largest item, and the one that makes the release honest.
2. **The Breaking table** (§3), enumerated from the real API diff.
3. **Rustdoc: 23 warnings, then the CI gate** (§4).
4. **Version bump and lockfile** (§5).
5. **Final verification** (§6).

**Out:** any code behaviour change; the real-binary integration suite (`TS-07` — owner-ruled to 0.5.0's
first increment); the tag and the publish (the owner authorizes, the architect performs); anything in
the proposal's "what 0.4.0 does not have" list.

---

## 2. The changelog backfill *[the largest item — and it is mine, not yours]*

`## Unreleased` covers **RFC 016 and 017 only**. Four increments have no entry at all:

| RFC | Missing from the changelog |
|---|---|
| **003** | The repository change token |
| **013** | Preview + tiered-confirmation machinery |
| **014** | **Commit — the headline feature of this release** |
| **015** | The prikk 0.32 re-baseline |

**`commit` is not in the changelog.** The single most significant thing this release does is
undocumented, because I approved four increments without directing the entry `02-implementer-handoff.md`
§7 has always required. That is my failure, and you are fixing it.

**Write each entry from what shipped, not from the RFC's intentions.** The RFCs describe what was
designed; several were amended mid-flight, and at least one shipped with a deferral its RFC does not
mention. **Read the code and the review results**, not just the RFC — `.git-exclude/reviewed/` has the
review result for every one of these.

**Shape, following the existing sections:**

- **RFC 014 — commit — leads the release.** It is the user-visible headline. Say what it does and what
  it refuses to do: it previews against the real worktree, prevents a cross-ref or empty-change commit
  client-side rather than classifying the refusal, and carries prikk's own notes verbatim.
- **RFC 013 — the machinery, and its guarantee is worth stating precisely.** `preview() → PreviewToken →
  confirm() → ConfirmedToken → execute()`, where neither token has a public constructor and `execute`
  takes by value — **skipping a step does not compile.** That is a stronger claim than "mutations are
  confirmed" and it is true; make it exactly, without inflating it.
- **RFC 003 — the change token**, framed by what it prevents: a preview computed under one repository
  state cannot execute against another. Note the **fingerprint half was deliberately dropped** — prikk
  has no repository identity by design.
- **RFC 015 — the 0.32 re-baseline.** `UD-01` retired (commit messages persist at prikk ≥ 0.32), patch
  messages surfaced in Block detail **always with the count beside them**, since pre-0.32 patches carry
  none and a silent discrepancy would be a wrong picture.

**Then, having written all six, re-read the top summary paragraph.** It currently describes 017 and 016
only, because those were the increments in front of me when it was written. The release is six
increments and its summary should say so — commit first.

**One convention check while you are in the file.** `CHANGELOG.md` uses `## 0.3.0 — 2026-09-05` (em
dash). `docs/src/contributing/releasing.md` step 1 says `## [<version>] - <date>` (brackets, hyphen).
**They disagree, and the file's own three released sections are the real convention.** Follow the file;
correct `releasing.md` to match it. Flag it in the review request as a docs fix so I can see you did it
deliberately.

---

## 3. The Breaking table *[verify; do not copy mine]*

RFC 011 exists because I called a release `0.1.1` when five public structs had gained fields. **Do not
build this table from the proposal's version** — build it from the diff:

```sh
git diff 0.3.0..HEAD -- crates/*/src
```

**RFC 011's rules, restated so you are applying them and not remembering them:**
- A public struct **gaining a field** is breaking (no blanket `#[non_exhaustive]` before 1.0 — measured
  and deferred, RFC 011).
- A trait **gaining a required method** is breaking for outside implementors.
- A public field **changing type** is breaking.
- An enum **gaining a variant** is **not** breaking where the enum is `#[non_exhaustive]` — check each
  one rather than assuming; `StikkError` and `Presentation` are, and that is why RFC 016/017's new
  variants are additive.
- A **private** function's parameter rename is not an API change at all.

**Known starting points** (incomplete by construction — find the rest):

| Crate | Change |
|---|---|
| `stikk-prikk` | `Prikk` gained three required methods: `change_token`, `commit`, `seal` |
| `stikk-model` | `Readiness::maintainer_ready: bool` → `maintainer_readiness: MaintainerReadiness` |
| `stikk-core` | palette entry: `min_capability` removed, `operation` + `tier` added; `available_to`/`unmet_reason` take `Readiness` |
| `stikk-core` | `OrientationView` gained public fields (`validated_through`, `prikk_persists_messages`) |

**Report the diff you ran and the entries you rejected**, not only the ones you kept. An entry I named
that turns out not to be breaking is as useful to me as one I missed.

---

## 4. Rustdoc — 23 warnings, then the gate

RFC 012 deferred this "scheduled with 0.4.0 planning". It is due, and it has **grown from 19 to 23
while nobody was watching** — which is the argument for the gate, not just the cleanup.

They fall into four kinds; **fix the documentation, never the visibility**:

- **Public docs linking to private items** (`stikk-state::paths` → `config_base_with`/`state_base_with`/
  `Platform`; `stikk-prikk::env` → `read_readiness_with`; `stikk-tui::app` → `crate::worker::Request`,
  `App::apply`). Reword to name the item without linking it, or link the public thing it is reached
  through. **Do not make a private item public to satisfy rustdoc** — that adds public API during
  release prep, which is exactly what this handoff forbids.
- **Ambiguous links** (`env` is both a module and a macro; `orient` is both a function and a module).
  Disambiguate with rustdoc's prefix syntax (`mod@env`, `fn@orient`).
- **Unresolved links** (`preview` in `commit.rs`, `Capability` in `palette.rs`) — fix the path or drop
  the link.
- **Redundant explicit link targets** (`cli_backend.rs`, `confirm.rs` ×2) — mechanical.

**Then add the gate**, so this cannot silently grow again:

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
```

`docs.yml` already runs `cargo doc` without the flag, so warnings pass today. **Put the gate in
`ci.yml` beside the other three**, not in `docs.yml` — a docs-publishing job that fails is a broken
site; a CI job that fails is a caught regression, which is what this is for. Add it to the handoff
spec's gate list too (`.git-exclude/specs/02-implementer-handoff.md` §5) so the next increment runs it.

---

## 5. Version and lockfile

- `[workspace.package] version` in the root `Cargo.toml`: `0.3.0` → **`0.4.0`**. All six crates inherit
  via `version.workspace = true`; **do not** add per-crate versions.
- Refresh the lockfile with **`cargo update --workspace`**. `cargo build --locked` **verifies** the
  lockfile and never rewrites it — I told an implementer the opposite once, and it cost a round.
- Move `## Unreleased` to `## 0.4.0 — <the date you land it>`, per §2's convention note.
- Confirm nothing else states a version: `validated_ceiling_display()` is the single source for the
  prikk ceiling, and the **crate** version should appear only in `Cargo.toml`. Grep `crates/`,
  `examples/`, `README.md`, `docs/` and `CHANGELOG.md` — the wide sweep, the one whose narrow version
  let a stale ceiling ship in 0.3.0.

---

## 6. Final verification

All four gates, plus the new fifth:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
mdbook build docs
```

**And one check no gate performs:** `cargo package --workspace --locked` (or per-crate `--list`),
to confirm each crate packages cleanly at the new version before anything is tagged. A publish that
fails on the fourth of six crates is the worst possible time to find out.

## 7. Acceptance criteria

1. All six increments have changelog entries; **commit leads**; the top summary describes the release,
   not the last two increments.
2. Entries written from what shipped — reviewed against the code and `.git-exclude/reviewed/`, not
   only the RFCs.
3. The Breaking table is enumerated from `git diff 0.3.0..HEAD`, with the rejected candidates reported.
4. Zero rustdoc warnings, fixed by documentation rather than by widening visibility.
5. The rustdoc gate is in `ci.yml` and in the handoff spec's gate list.
6. Version is `0.4.0`; `Cargo.lock` refreshed with `cargo update --workspace`; the changelog heading
   moved; `releasing.md`'s heading convention corrected to match the file.
7. The wide version grep covers `crates/` and `examples/`, not just `README.md` and `docs/`.
8. All six commands above green; `cargo package` clean.
9. **No behaviour change in this increment.** If you fixed one, you have exceeded scope — report it
   instead.
10. Nothing tagged, nothing published.

## 8. Submit

Package to `.git-exclude/review-request/011-release-0-4-0-preparation/review-request-v1.md`.

**Lead with the changelog**, in full — it is the deliverable a reader of this release actually sees,
and the only part I cannot verify by running a command. Then the Breaking table and the diff behind it.

**Tell me what the changelog forced you to notice.** Writing four backfilled entries means reading four
increments end-to-end, months of decisions at once, against what actually shipped. That is a vantage
point nobody else in this project has had — including me, who reviewed them one at a time. **If two
entries contradict each other, or one describes something the code does not do, that is the most
valuable thing you could find this week.**

**Push once approved** (`.git-exclude/specs/02-implementer-handoff.md` §6). The tag and the publish are
not yours, not mine — the owner authorizes, and I perform.

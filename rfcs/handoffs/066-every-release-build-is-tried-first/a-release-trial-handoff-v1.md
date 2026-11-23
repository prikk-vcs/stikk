# Handoff A — RFC 066: every release build is tried before a tag (v1)

**Companion to:** [RFC 066](../../done/066-every-release-build-is-tried-first.md), **accepted 2026-11-24**. **Read
the RFC first**: F1–F3 and decisions 1–5. **Covers:** all of it. **0.22.0's increment 1.** **Not a release.**

> **What this increment is.** 0.21.0's release run died on a retired macOS image that nothing but a tag ever used.
> After this handoff, the release build is defined **once**, in a reusable workflow. The release calls it to publish,
> and a **trial** calls the same build with publishing off, at every prep, weekly, and when the release workflows or
> the installer templates change.
>
> **Only `.github/workflows/` changes**, plus the `CHANGELOG.md` line. **No branch.** Workflow changes are proved by
> running them: after approval, push, then dispatch the trial on `main`.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Is the build defined once, and does the release call it unchanged in behaviour? | review | the diff: `release.yml`'s `binaries` and `installers` bodies moved verbatim into `build-release.yml`, with the publish steps gated |
| Does the trial build every target and fill the installers, publishing nothing? | CI, **after approval and push** | one dispatched `release-trial.yml` run on `main`: every leg, and the artifacts' names |
| Are the trial's archives what a release publishes? | CI, by inspection | one `.tar.gz` and the Windows `.zip` artifact, compared with `0.21.1`'s published archives: names, layout, and the `.sha256` line's shape |
| Can the trial publish? | review | its permissions (none written), and the gating of every publish step |
| Does the release still publish as before? | the next tag | **0.22.0's own release run**, verified by the architect after the tag |

**Before approval** the workflows can only be linted. Run **`actionlint`** on the three files, if it is installed or
can be fetched as a pinned binary. Say which.

## 1. Commit 1: `build-release.yml`, and `release.yml` calling it

1. **`.github/workflows/build-release.yml`**, `on: workflow_call`:
   - **inputs:** `version` (string, required) and `publish` (boolean, required);
   - **jobs:** `binaries` and `installers`, moved from `release.yml` **verbatim**. The matrix, the musl tools, the
     packaging, the Windows checksum line and the placeholder filling all move unchanged.
2. **Gate the publishing steps on `inputs.publish`:** `Attest build provenance`, the binaries' `Upload to the
   release`, and `Upload the installers to the release`.
3. **With `publish: false`,** upload each archive and its `.sha256`, and each filled installer and its `.sha256`,
   with `actions/upload-artifact`, `retention-days: 7`.
4. **`release.yml`:**
   - its `binaries` and `installers` jobs are replaced by **one job that `uses: ./.github/workflows/build-release.yml`**
     with `version: ${{ needs.guard.outputs.version }}` and `publish: true`;
   - it **grants** `contents: write`, `id-token: write` and `attestations: write` on that job;
   - `aur`'s `needs:` points at the new job, in place of `binaries`;
   - **nothing else in `release.yml` changes**: guard, verify, release, crates and aur keep their steps.

**The tag's `github.ref_name`:** the upload steps use it today. In the reusable workflow, derive the tag from the
`version` input, so the file never depends on how it was called. Say what you chose.

## 2. Commit 2: `release-trial.yml`

1. **Triggers:**
   - `workflow_dispatch`;
   - `schedule` (weekly; pick a quiet hour, and say which);
   - `push` to `main` with `paths:` `.github/workflows/build-release.yml`, `.github/workflows/release.yml`,
     `.github/workflows/release-trial.yml` and `scripts/**`.
2. **One job reads the workspace version from `Cargo.toml`**, as `guard` does, reusing its command, not restating
   it. **One job calls `build-release.yml`** with that version and `publish: false`.
3. **`permissions: contents: read`** at the top, and nothing more anywhere. No `secrets: inherit`.

## 3. Commit 3: the text

- **`CHANGELOG.md`, `## Unreleased`, `### Changed`:** the RFC's line.
- **`docs/src/contributing/releasing.md`:** where it lists what a release prep runs, add the trial, and say that a red
  leg stops the cut. Remove any mention of the manual runner-label table.

## 4. The traps

1. **Move, don't copy.** After commit 1, `release.yml` holds no `cargo build` and no `softprops` upload for binaries
   or installers.
2. **No publish step runs with `publish: false`.** Each one is gated, and the review checks every `if:`.
3. **The trial writes nothing:** no `contents: write`, no `id-token`, no secrets.
4. **`aur`'s dependency moves** to the new job, or the AUR push starts before the binaries exist.

## 5. Submit (no branch)

Gates 1–3, `actionlint` if available, and a local `cargo build --release -p stikk` for one target, as a sanity check
of what the workflow runs. Package to
`.git-exclude/review-request/066-every-release-build-is-tried-first-a/review-request-v1.md`, with:
- the diff of `release.yml`;
- the new files;
- each gated step.

**After approval:** push, dispatch `release-trial.yml` on `main`, and report every leg and the artifacts, with §0's
archive comparison.

# Releasing

stikk ships from two GitHub Actions workflows, both driven by a version tag or a push to `main`:

- **`.github/workflows/docs.yml`** — builds the mdBook (this site) and the rustdoc API reference and
  publishes both to GitHub Pages (the book at the root, the API under `/api`). Runs on every push to
  `main` that touches docs or code.
- **`.github/workflows/release.yml`** — on a version tag: gates on the full CI suite, creates the
  GitHub Release (notes from `CHANGELOG.md`), publishes every crate to crates.io via **Trusted
  Publishing**, and attaches a checksummed, provenance-attested `stikk` binary for each supported
  target.

## Conventions

- **Tags are the bare Cargo version — no `v` prefix** (Rust convention): `0.1.0`, `0.2.0-rc.1`. The
  release workflow refuses a tag that does not equal the `[workspace.package] version` in `Cargo.toml`.
- A tag containing `-` (e.g. `0.2.0-rc.1`) is published as a **pre-release**.
- crates.io publishing is **idempotent**: a crate/version already on crates.io is skipped, so a
  re-run (or a tag cut after a manual first publish) is safe.

## One-time setup (maintainer, in the hosting UIs)

These cannot be done from CI; do them once.

### GitHub Pages

Repository **Settings → Pages → Build and deployment → Source: GitHub Actions**. The next push to
`main` (or a manual run of the **Docs** workflow) publishes the site.

### crates.io Trusted Publishing

Trusted Publishing mints a short-lived token over OIDC, so no crates.io token is ever stored in the
repo. crates.io requires a crate to exist before a trusted publisher can be attached, so the **first
`0.1.0` publish of each new crate name is done once, by hand**; every release after that is CI.

1. **First manual publish**, from a clean checkout of the tagged commit, in dependency order (each
   command waits for the index before the next resolves):

   ```sh
   cargo login            # a personal token, used only for this first publish
   cargo publish -p stikk-model --locked
   cargo publish -p stikk-prikk --locked
   cargo publish -p stikk-state --locked
   cargo publish -p stikk-core  --locked
   cargo publish -p stikk-app   --locked
   cargo publish -p stikk-tui   --locked
   cargo publish -p stikk       --locked
   ```

   (crates.io rate-limits brand-new crate names; if it pauses you mid-list, wait and re-run — the CI
   job and these commands both skip what is already published.)

   **A crate name added after 0.1.0 is published by hand too**, once, at the release that introduces
   it: the tag's `crates` job can only publish a name that already exists on crates.io. The section
   below is that procedure, written from `stikk-app`'s turn at 0.22.0.

2. **Attach a trusted publisher** to *each* of the seven crates: crate **Settings → Trusted Publishing →
   Add**, GitHub, with

   - Owner / repository: `prikk-vcs/stikk`
   - Workflow filename: `release.yml`
   - Environment: `release`

3. (Recommended) Add a **manual approval gate**: repository **Settings → Environments → `release` →
   Required reviewers**. The `crates` job then waits for a click before any publish.

## A new crate name at a tag (`stikk-app`, 0.22.0)

A release that introduces a crate name cannot publish it from CI: Trusted Publishing authorizes a
workflow to publish a crate that **already exists**, and there is nothing to authorize against for a
name crates.io has never seen. So the tag runs, stops at the new crate, and is finished by hand.

1. **Tag as usual, and let the run go.** The `crates` job publishes the crates below the new one in
   dependency order, then **fails on it**. That failure is expected and it is the only one:
   - the binaries, the installers and the GitHub Release are **not affected** — they do not depend on
     the `crates` job;
   - the **AUR** job waits, because it depends on `crates`.
2. **Publish the new crate by hand, from a clean checkout of the tag:**

   ```sh
   cargo publish -p stikk-app --locked
   ```

   Use a **scoped** crates.io token: scope **`publish-new`**, crate **`stikk-app`**, the shortest
   expiry offered. A scoped token gives the least privilege — it can publish this one crate and
   nothing else. crates.io's publish endpoint still accepts a token with full rights, which is no
   reason to use one here.
3. **Attach the trusted publisher** to the new crate, with the same values as the others (owner /
   repository `prikk-vcs/stikk`, workflow `release.yml`, environment `release`) — and then **revoke
   the token.** It has done its one job.
4. **Re-run the failed jobs** of the release run. The `crates` job skips every crate already
   published and publishes the rest; the AUR job follows it.

**Only the first release of a name needs any of this.** From the next release that crate is CI's like
every other.

## Cutting a release

1. Move the `## Unreleased` section of `CHANGELOG.md` under a `## <version> — <date>` heading (em dash,
   no brackets — match the file's own released sections) and set `[workspace.package] version` in
   `Cargo.toml` to the same `<version>`.
2. Land that on `main` (green CI), then tag and push:

   ```sh
   git tag 0.1.0
   git push origin 0.1.0
   ```

3. The **Release** workflow runs: guard → verify → create the GitHub Release → (approve the `release`
   environment, if gated) publish to crates.io → build and attest the binaries and fill the
   installers, through `build-release.yml`. The **Docs** workflow refreshes the site from `main`.

**Before the tag, every prep runs the release trial.** `release-trial.yml` calls
`build-release.yml` — the same file step 3 calls — with publishing off: every target is built on
its real runner, every archive is packaged and checksummed, the four installers are filled, and
the results are kept as artifacts for seven days. Dispatch it on `main` at the SHA to be tagged
and report every leg. **A red leg stops the cut**, as a red real-binary suite leg does. It also
runs weekly, and on any change to the release workflows or to `scripts/**`.

This is what 0.21.0 cost: its build machines were named only in `release.yml`, which nothing but a
tag ran, so a runner image GitHub had retired was met mid-publish — both macOS jobs died before
their first step and the release went out without its macOS archives or its installers (RFC 066).
The trial is the check that would have caught it, and it runs rather than being remembered.

## Verifying a binary

Each archive ships a `.sha256` and a signed build-provenance attestation:

```sh
sha256sum -c stikk-<version>-<target>.tar.gz.sha256
gh attestation verify stikk-<version>-<target>.tar.gz --repo prikk-vcs/stikk
```

**Which workflow signed it changed at 0.22.0.** From 0.22.0 the attestations are signed by
`build-release.yml`, called from `release.yml`; up to and including 0.21.1 they were signed by
`release.yml` itself (RFC 066 moved the build into the reusable workflow). The command above
accepts both, because `--repo` does not name a workflow. If you pin `--signer-workflow`, that is
why it has to change — and it is worth pinning the new one rather than dropping the flag.

## What a v0.22.x release is (and is not)

**0.22.0 changes nothing you see; underneath, stikk's program is its own crate** (`stikk-app`, RFC
067), the release build is tried before every tag (RFC 066), and the supply-chain check reports
every advisory in the dependency tree (RFC 068).

v0.22.x is **readable on light and dark terminals**, works with **text made much bigger**, and
**lists what you can do where you are** rather than asking you to remember a letter. It reads
orientation, history, block detail — including the files at **any** block, with their
modes, at prikk ≥ 0.46 — worktree changes, the Queue view, **Patch detail** (what one patch changed,
from prikk's own `show` report, with content shown byte for byte), **Compare** (what changed between
two points, from prikk's own `diff` hunks, at prikk ≥ 0.46 — two blocks, a block and the one before
it, any two refs, or the worktree), **Verify** (prikk's verdict on whether the repository is sound,
with its failing conditions and every stage it ran, at prikk ≥ 0.39), **a bundle file** (prikk's
own verification of it and what prikk says importing it would change, at prikk ≥ 0.53), **the
maintainer keys this repository trusts** (`t`, at prikk ≥ 0.54, with your own marked and each
recorded sealer judged against the list) and the
refusal/glossary surfaces, and performs fourteen mutations — **commit** (author a worktree capture),
**seal** (freeze the queue into signed history), **create a branch**, **create a tag**, **close
a branch** and **merge one branch into another** (at prikk ≥ 0.42), **switch branch** (at
prikk ≥ 0.46), **import a bundle** as a received branch and **export a branch** to a bundle
file (at prikk ≥ 0.53), **save one file** or **save a block's files** as they were at a block,
**undo one block** (putting back what it changed as a new commit, which history keeps beside the
block itself), and — from the Trust view — **adopt a maintainer key** or **remove one** (all at
prikk ≥ 0.54) — every one of them
preview-first with tiered confirmation, naming the key that will sign where one signs at all. A
confirmed commit authors only the worktree its preview showed, the seal confirmation names the
patches it freezes, a switch's confirmation names the exact files it will write and delete, and a
merge runs only where prikk's own plan proves the two sides confluent — never into the branch you
are on, and writing nothing to your worktree. An import creates and moves no branch of yours and
is declined where it would change nothing or move the received branch back; an export writes
**outside** the repository, never over an existing file, and changes nothing inside it. The two
saves write outside the repository too, never over an existing path — both refused in stikk's own
words before prikk is asked — and change nothing in the repository or the worktree. **The two
trust writes are confirmed by typing the key id**, stikk's first typed confirmations, and neither
needs a signing key; a removal says first that it can make blocks the key sealed fail Verify until
the key is adopted again. It also notices a change made outside it and refreshes what is on screen,
and its commit preview shows a declared rename as prikk's own verdict resolves it, makes commit
unavailable where prikk would refuse a declaration, and names a ref with no published history.
Every string it did not write is rendered so that what you read is what prikk reported.
Signing readiness is read from prikk itself on prikk ≥ 0.41. **It reopens each repository where
you left it** — the focused ref, the top-level view and the untracked filter, re-checked against
prikk before any of it is shown and stored outside every repository; `STIKK_PRIVATE=1` remembers
nothing. Sync, and checkout of a block
or a path, remain unbuilt, and so are **adopting a received tag** and **author keys** — the Trust
view adopts and removes maintainer keys, and nothing else — and a **doctor** or lock-inspector
view (`doctor` has no JSON): the `Prikk` seam has thirty-three methods — `handshake`, `readiness`,
`orientation`, `history`, `block_state`, `refs`, `tags`, `worktree_status`, `queue`, `show`,
`diff`, `verify`, `change_token`, `commit`, `seal`, `create_branch`, `create_tag`, `close_branch`,
`switch_branch`, `merge_plan`, `merge`, `bundle_verify`, `bundle_preview`, `bundle_import`,
`bundle_export`, `file_meta`, `save_file`, `extract_plan`, `extract`, `trusted_keys`,
`trust_add`, `trust_remove` and `rename_path` — and none for any of them.
A release also publishes **the four installer scripts** (`install.sh`, `uninstall.sh`,
`install.ps1`, `uninstall.ps1`), each with its own `.sha256`, filled with that release's version.
It drives the external `prikk` binary at runtime (not a Cargo dependency), validated against prikk **>= 0.28, through 0.58.0** — so the release
notes must state which prikk version it was validated against and how to install it — a
`cargo install stikk` with no `prikk` on `PATH` will open and then explain that prikk is missing, by
design.

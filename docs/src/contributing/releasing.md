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
   cargo publish -p stikk-tui   --locked
   cargo publish -p stikk       --locked
   ```

   (crates.io rate-limits brand-new crate names; if it pauses you mid-list, wait and re-run — the CI
   job and these commands both skip what is already published.)

2. **Attach a trusted publisher** to *each* of the six crates: crate **Settings → Trusted Publishing →
   Add**, GitHub, with

   - Owner / repository: `prikk-vcs/stikk`
   - Workflow filename: `release.yml`
   - Environment: `release`

3. (Recommended) Add a **manual approval gate**: repository **Settings → Environments → `release` →
   Required reviewers**. The `crates` job then waits for a click before any publish.

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
   environment, if gated) publish to crates.io → build and attest the binaries. The **Docs** workflow
   refreshes the site from `main`.

## Verifying a binary

Each archive ships a `.sha256` and a signed build-provenance attestation:

```sh
sha256sum -c stikk-<version>-<target>.tar.gz.sha256
gh attestation verify stikk-<version>-<target>.tar.gz --repo prikk-vcs/stikk
```

## What a v0.16.x release is (and is not)

v0.16.x reads orientation, history, block detail — including the files at **any** block, with their
modes, at prikk ≥ 0.46 — worktree changes, the Queue view, **Patch detail** (what one patch changed,
from prikk's own `show` report, with content shown byte for byte), **Compare** (what changed between
two points, from prikk's own `diff` hunks, at prikk ≥ 0.46 — two blocks, a block and the one before
it, any two refs, or the worktree), **Verify** (prikk's verdict on whether the repository is sound,
with its failing conditions and every stage it ran, at prikk ≥ 0.39), **a bundle file** (prikk's
own verification of it and what prikk says importing it would change, at prikk ≥ 0.53) and the
refusal/glossary surfaces, and performs nine mutations — **commit** (author a worktree capture),
**seal** (freeze the queue into signed history), **create a branch**, **create a tag**, **close
a branch** and **merge one branch into another** (at prikk ≥ 0.42), **switch branch** (at
prikk ≥ 0.46), and **import a bundle** as a received branch and **export a branch** to a bundle
file (at prikk ≥ 0.53) — every one of them
preview-first with tiered confirmation, naming the key that will sign where one signs at all. A
confirmed commit authors only the worktree its preview showed, the seal confirmation names the
patches it freezes, a switch's confirmation names the exact files it will write and delete, and a
merge runs only where prikk's own plan proves the two sides confluent — never into the branch you
are on, and writing nothing to your worktree. An import creates and moves no branch of yours and
is declined where it would change nothing or move the received branch back; an export writes
**outside** the repository, never over an existing file, and changes nothing inside it. It also notices a change made outside it and refreshes what is on screen,
and its commit preview shows a declared rename as prikk's own verdict resolves it, makes commit
unavailable where prikk would refuse a declaration, and names a ref with no published history.
Every string it did not write is rendered so that what you read is what prikk reported.
Signing readiness is read from prikk itself on prikk ≥ 0.41. **It reopens each repository where
you left it** — the focused ref, the top-level view and the untracked filter, re-checked against
prikk before any of it is shown and stored outside every repository; `STIKK_PRIVATE=1` remembers
nothing. Sync, and checkout of a block
or a path, remain unbuilt, and so is a **Trust & Keys** view, and a **doctor** or lock-inspector
view (`doctor` has no JSON): the `Prikk` seam has twenty-five methods — `handshake`, `readiness`,
`orientation`, `history`, `block_state`, `refs`, `tags`, `worktree_status`, `queue`, `show`,
`diff`, `verify`, `change_token`, `commit`, `seal`, `create_branch`, `create_tag`, `close_branch`,
`switch_branch`, `merge_plan`, `merge`, `bundle_verify`, `bundle_preview`, `bundle_import` and
`bundle_export` — and none for any of them.
A release also publishes **the four installer scripts** (`install.sh`, `uninstall.sh`,
`install.ps1`, `uninstall.ps1`), each with its own `.sha256`, filled with that release's version.
It drives the external `prikk` binary at runtime (not a Cargo dependency), validated against prikk **>= 0.28, through 0.54.1** — so the release
notes must state which prikk version it was validated against and how to install it — a
`cargo install stikk` with no `prikk` on `PATH` will open and then explain that prikk is missing, by
design.

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

1. Move the `## Unreleased` section of `CHANGELOG.md` under a `## [<version>] - <date>` heading and set
   `[workspace.package] version` in `Cargo.toml` to the same `<version>`.
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

## What a v0.2.x release is (and is not)

v0.2.x is still a **read-only preview**: orientation, history, block detail, worktree changes, and the
refusal/glossary surfaces. It performs **no repository mutations** yet. It drives the external `prikk`
binary at runtime (not a Cargo dependency), validated against prikk **>= 0.28, through 0.30.0** — so the
release notes must state which prikk version it was validated against and how to install it — a
`cargo install stikk` with no `prikk` on `PATH` will open and then explain that prikk is missing, by
design.

# Handoff A — RFC 051: stikk and stikk-bin on the AUR (v1)

**Companion to:** [RFC 051](../../accepted/051-arch-linux-packages.md), **accepted 2026-10-21**, no open question. **Read
the RFC first**, F2–F4 and decisions 1–7. The measured PKGBUILDs are in `.git-exclude/reports/051-aur/`
(`stikk.PKGBUILD`, `stikk-bin.PKGBUILD`, `measure051.txt`): start from them.
**Covers:** decisions 1–7. 0.16.0's increment 5; it touches packaging and CI only, so it runs beside exchange.
**Not a release.**

> **What this increment is.** Two PKGBUILD templates filled from the release, **one script** that fills them, a
> workflow that builds and lints them on every change and **pushes nothing**, and a release job that pushes them —
> **only when the owner's secret exists**. The owner will set the key when it is needed and wants the packages up as
> soon as possible; prikk's AUR packages are on their way, and prikk-bin will `provides=('prikk=<ver>')`.
>
> **You never see or handle key material.** The workflow reads `secrets.AUR_SSH_PRIVATE_KEY` into an `ssh-agent`;
> nothing prints it, writes it to the workspace, or logs it.
>
> **No branch.** Local gates before submitting; after approval, push to `main`; `aur.yml` runs on the push.

---

## 1. Commit 1: the templates and the filler

**a. `packaging/aur/stikk/PKGBUILD.in` and `packaging/aur/stikk-bin/PKGBUILD.in`**, from the measured PKGBUILDs, with:
- placeholders for **the version**, **the prikk floor** (`depends=('prikk>=0.@PRIKK_FLOOR@')`), and **each checksum**
  (`stikk`: the crate; `stikk-bin`: the `x86_64` and `aarch64` musl archives);
- `stikk`: `depends=('prikk>=…' 'glibc' 'libgcc')` (namcap's own suggestion; not `gcc-libs`), `makedepends=('cargo')`,
  `prepare`/`build`/`check`/`package` as measured;
- `stikk-bin`: `options=('!strip' '!debug')` — **the installed binary must stay byte-identical to the attested
  artifact** (RFC 051 F2) — `provides=("stikk=$pkgver")`, `conflicts=('stikk')`;
- `# Maintainer:` as the owner's AUR account names it (the placeholder PKGBUILDs carry none; use `nabbisen`, and say so
  in the request so the owner can correct it).

**b. `packaging/aur/render.sh <version> <outdir>`** — **the one filler both workflows run**:
- the floor from `cargo run --quiet -p stikk-prikk --example print_version_matrix` (as `release.yml`'s installer job);
- the `-bin` checksums from the release's own `.sha256` assets for that version;
- the crate checksum from the crates.io **index** (`https://index.crates.io/st/ik/stikk`, the line whose `vers` is the
  version, its `cksum`) — and **download the crate and check it against that value**, refusing on a mismatch;
- refuses if any placeholder survives, or any input is missing (a version with no release, or not on crates.io).

**c. `packaging/aur/known_hosts`** — the AUR host's keys, **pinned**. Take them with `ssh-keyscan aur.archlinux.org`,
then **verify each fingerprint against the ones published on the AUR's own home page** before committing; put both,
side by side, in the review request. The push uses `StrictHostKeyChecking=yes` with this file and nothing else.

## 2. Commit 2: `aur.yml`

- **Triggers:** `push` on `packaging/aur/**` and the workflow itself; `workflow_dispatch` with inputs `version`
  (default: the latest published release) and `push` (boolean, default `false`); and **`workflow_call`** with the same
  two inputs, for `release.yml`.
- **One job** in `archlinux:base-devel`: install `cargo`, `namcap`, `git`, `openssh`; create a non-root builder user
  (`makepkg` refuses root); run `render.sh`; for each package `makepkg -f -d` (prikk is not in the official
  repositories, so its dependency is not installable in the container — **`-d` skips only the check, and say so in a
  comment**), then `namcap` on the PKGBUILD and the package. **Refuse on any `namcap` line outside an allow-list** in
  the workflow, each entry with its reason: `x86_64` named in `source_x86_64`; `stikk-bin`'s unstripped ELF; `prikk`
  *"may not be needed"* (both); any line the measured run showed that you judge benign — **justify each** in the request.
- Generate `.SRCINFO` with `makepkg --printsrcinfo`.
- **The push step** runs only when the `push` input is true **and** the secret is non-empty; otherwise it prints a
  `::notice::` naming which was missing, and succeeds. When it runs: `ssh-agent` with the secret, the pinned
  `known_hosts`, clone `ssh://aur@aur.archlinux.org/<pkg>.git`, replace `PKGBUILD` and `.SRCINFO`, and **commit and
  push only if they changed** (a re-run is a no-op), the message `stikk <version>`. The job runs under GitHub
  environment **`aur`** (the owner may add a reviewer there; the job must not assume one).

## 3. Commit 3: `release.yml`, the page, the text

- **`release.yml`**: a job `aur`, **after** the crates are published and the assets uploaded, calling `aur.yml` with
  `version` = the tag and `push: true`, `secrets: inherit`. **It must not fail the release**: a failed AUR push is
  reported; the release is already public (RFC 051 decision 4). Say in a comment how to re-run it alone
  (`workflow_dispatch`, `push: true`).
- **The install page** (`docs/src/guide/install.md`): a short **Arch Linux** section after the two steps — `stikk-bin`
  (prebuilt) or `stikk` (built from source), from the AUR with `makepkg -si` in a clone of the AUR repository or any AUR
  helper, naming none as required; that it brings in **prikk's own AUR package**; and updating through the same route.
  **Do not publish the section before the packages exist**: write it now, and gate it with one sentence the release
  prep removes, or hold it back — your call, say which.
- `## Unreleased` `### Added`: RFC 051's Changelog line. `ROADMAP.md`: nothing (the architect's).

## 4. Measured before review

- **Locally**, on an Arch-based system if you have one: `render.sh 0.15.0` then `makepkg -f -d` and `namcap` for both;
  the `-bin` binary's sha256 against the archive's. If you have none, say so: §5's CI run is then the measurement.
- **The workflow itself** can only be dispatched once it is on `main`. If you can run its steps in an
  `archlinux:base-devel` container locally, do, and report the namcap output and the `.SRCINFO`; if not, say so — the
  `aur.yml` run on the push (which pushes nothing) is then the first run, and a failure there is a follow-up commit.
  The push path is proven only when the owner sets the secret — **report that as unproven, plainly**.

## 5. The traps

1. **Key material never reaches a log, the workspace, or an artifact.** `ssh-agent` only.
2. **The host key is pinned and verified against the AUR's published fingerprints**, never accepted on first use.
3. **`stikk-bin` stays unstripped**; the byte-identity with the attested artifact is the point.
4. **The crate's checksum comes from the index, and the download is checked against it.**
5. **A failed AUR push never fails the release.**
6. **No push without both the input and the secret**, and a missing secret is a notice, not a failure.
7. **prikk's packages are not ours**: nothing here touches `prikk` or `prikk-bin`.

## 6. Submit (no branch)

The eight gates (unaffected), `shellcheck` on `render.sh`, and §4's measurements. Package to
`.git-exclude/review-request/051-aur-a/review-request-v1.md`, leading with the two filled PKGBUILDs for 0.15.0 and their
`namcap` output, then the host-key fingerprints beside the AUR's published ones.

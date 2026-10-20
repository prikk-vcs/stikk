# Handoff B — RFC 048: a static build for 64-bit ARM Linux (v1)

**Companion to:** [RFC 048](../../accepted/048-delivery-for-new-users.md), F4 as **corrected 2026-10-20** and decision 9.
**Follows:** Handoff A, pushed at `9f91ee9`, whose `ubuntu-22.04-arm` job found the defect
(`.git-exclude/reports/048-delivery/measure048-f4-aarch64.txt`). 0.15.0's increment 4.
**Not a release.** The tap is now Handoff C, later.

> **What this fixes.** stikk's `aarch64-unknown-linux-gnu` build requires `GLIBC_2.39` (verneed *Flags: none* — your
> finding), so **it does not start** on Debian 12, Raspberry Pi OS bookworm or Ubuntu 22.04, and `install.sh` installs
> that build on every 64-bit ARM Linux. The fix is the one x86_64 already relies on: **a static musl build, and the
> installer takes it.** prikk is unaffected: its aarch64 build needs only `GLIBC_2.34` (measured, prikk 0.53.0).
>
> **No branch.** `main` is red on that one job until this lands; that is accepted, and right — you did not weaken the
> assertion, and you should not.

---

## 1. One commit

**a. `release.yml`** — a seventh target, `aarch64-unknown-linux-musl`, on `ubuntu-24.04-arm` (native; no cross
toolchain). *Install musl tools* runs for **both** musl targets (`endsWith(matrix.target, '-musl')` or an explicit
list). Packaging, checksum and attestation are the existing per-target steps. **The `aarch64-unknown-linux-gnu`
archive stays published**; nothing is removed.

Architect's evidence that it builds: stikk has no C dependency for this target (`cargo tree -e build`), and a
cross-build here (`--target aarch64-unknown-linux-musl`, linker `rust-lld`) gives *"ELF 64-bit LSB executable, ARM
aarch64 … statically linked"*, **no `NEEDED` entry and no interpreter**. Not run (no aarch64 machine here); §2 runs it.

**b. `install.sh`** — `Linux/aarch64|Linux/arm64` → `aarch64-unknown-linux-musl`. Rewrite the comment at `:84-92` to
cover both architectures, and **say why in one line**: the glibc build carries the glibc of the machine it was built
on, and on aarch64 that is enforced.

**c. `installers.yml`** — the `ubuntu-22.04-arm` job:
- **The published 0.14.0 has no aarch64 musl archive**, so this job cannot install a release until 0.15.0 is out. It
  **builds the archive at the commit under test**, packaged exactly as `release.yml` does (name, contents, `.sha256`),
  into a mirror, and installs it with `--base-url file://…` — the mirror mechanism the tamper leg already uses. It
  then tests the installer **and** the build on glibc 2.35 together, at every commit, not only at release.
- **Assert, hard**: the log names `aarch64-unknown-linux-musl`; `stikk --version` prints the expected version;
  **stderr is empty**; `readelf -l` shows no `INTERP`. The *record, don't fail* stance of Handoff A is over: the
  question is answered, so the answer is now a requirement.
- Update the target table at `:134` (`Linux/aarch64|Linux/arm64` → musl), so the mirror holds what `install.sh` asks
  for.

**d. The install page's manual table** (`docs/src/guide/install.md:147-148`): *Linux, ARM* → the musl archive; the
glibc one moves to a row for *Linux, ARM (glibc 2.39 or newer)*, as x86_64's two rows are shaped.

**e. Changelog** `## Unreleased`: `### Added` — the `aarch64-unknown-linux-musl` archive; `### Fixed` — *on 64-bit
ARM Linux older than glibc 2.39 (Debian 12, Raspberry Pi OS bookworm, Ubuntu 22.04), the installer now installs a
static build; the glibc build it installed until now did not start there.*

## 2. Gates and what proves it

The eight gates (unaffected, but run them), `shellcheck`. **The proof is the `ubuntu-22.04-arm` job on the push**,
green with the hard assertions above; report its log lines for the target, the version and stderr.

**`release.yml` is not exercised until the 0.15.0 release run.** So that it is not first run in production: dispatch
nothing that publishes, but **say in the request** how the aarch64-musl matrix entry was checked (the YAML read
against the x86_64-musl entry, the `musl-tools` condition, and `actionlint` if available). The 0.15.0 release-prep
handoff will list the new asset among those checked after the tag.

## 3. The traps

1. **Do not drop the gnu archive.** Someone may depend on its name; nothing is removed in a fix.
2. **The CI leg builds what it installs**; it must not quietly fall back to the published gnu build.
3. **Stderr empty is the assertion**, not a notice.

## 4. Submit (no branch)

Package to `.git-exclude/review-request/048-delivery-b/review-request-v1.md`, leading with the `readelf` of your
local aarch64-musl build if you can make one, and the base filled from `git rev-parse <commit>^`.

# Handoff A — RFC 049: the prikk 0.53 re-baseline (v1)

**Companion to:** [RFC 049](../../accepted/049-prikk-0-53-rebaseline.md), **accepted 2026-10-19**, no open question.
**Read the RFC first.** **Covers:** decisions 1–3. **Not a release.** **No branch.**

> **Small: one commit, no band moves.** Do this one **first**, before RFC 047's handoffs: they measure at the ceiling,
> and the ceiling should be 0.53 when they do.

---

1. **The suite's ceiling binary.** Install prikk 0.53.0 where your pinned binaries live
   (`cargo install prikk --version 0.53.0 --locked --root <dir>`); the architect's is at
   `~/.local/share/stikk-test-prikk/ceiling-0.53/bin/prikk`. The harness refuses a ceiling binary that is not the
   validated ceiling, so the suite is red until step 2 — **that is expected, not a finding.**
2. **`VALIDATED_MAX_MINOR` 52 → 53** (`crates/stikk-prikk/src/version.rs`). Run the suite at 0.28.0 + 0.53.0: RFC 049 F1
   says **58 passed, 0 failed**. **If any leg fails, stop and report.**
3. **Docs**: *validated through 0.53.0* in `ASM-2`, `README.md`, `crates/stikk/README.md`, `docs/src/index.md`, the guide
   and `ROADMAP.md`'s versioning paragraph. **Grep `0.52.0` and `validated through`, and read each hit**: the
   manual-install section's archive names are **stikk's** versions and stay; historical entries stay.
4. **Installers**: no change (they derive the pin). **Confirm** after the push that `installers.yml` derives **0.53.0** and
   installs it on the Unix `--with-prikk` legs.
5. **`## Unreleased`**, `### Changed`: validated through prikk 0.53.0.
6. **Submit**: the eight gates and the suite at 0.28.0 + 0.53.0. Package
   `.git-exclude/review-request/049-prikk-0-53-rebaseline/review-request-v1.md`. **After approval**: push, then the
   full-matrix suite, supply chain and `installers.yml` on `main`, every run reported.

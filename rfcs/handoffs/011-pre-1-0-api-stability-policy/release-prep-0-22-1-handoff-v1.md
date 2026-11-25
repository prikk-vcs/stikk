# Handoff — 0.22.1 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md).
**Issued** 2026-11-25. **The project owner authorized scheduling 0.22.1** the same day, and the dev team prepares it
by standing permission. **The tag and the publish each wait for the owner after review.**

**Follows** [0.22.0's prep handoff](./release-prep-0-22-0-handoff-v1.md). Its rules hold except where this one says
otherwise.

> **What 0.22.1 is.** 0.22.0 with the reading gate fixed
> ([RFC 067 Handoff C](../067-stikk-app-the-shared-program/c-the-reading-gate-holds-handoff-v1.md), `9166c77`..
> `5dc7921`), and nothing else. **No public API change** in any crate.
>
> **Sanctioned:**
> - the version bump, and `Cargo.lock`'s workspace entries;
> - `CHANGELOG.md`;
> - `ROADMAP.md`'s 0.22.0 *Shipped* section;
> - the install guide's transcript.
>
> **Not this handoff:**
> - the status texts, which already say 0.22.x and stay true;
> - `releasing.md`;
> - the workflows. **RFC 072 Handoff A stays paused until 0.22.1 is tagged.** 0.22.1 is built by the same workflows
>   that built 0.22.0.
>
> Anything else: stop and report. **No branch.**

---

## 1. The bump

`0.22.0` → `0.22.1` in `[workspace.package]` and **the seven pins**. `cargo update --workspace`: **only `stikk*` lines
may move.**

## 2. `CHANGELOG.md`

`## Unreleased` becomes `## 0.22.1 — <the date you push>`.
- **The bold summary:** *"0.22.0 with its confirmation gate fixed. Use 0.22.1: in 0.22.0 a confirmation that did not
  fit the window could be confirmed without being read."*
- **Keep the `### Fixed` entry** as delivered.
- **No `### Breaking`.** Run `cargo public-api -p <crate> diff 0.22.0..HEAD` for the six library crates yourself, and
  state the result in one line, as 0.21.1's entry did: no removed, no changed and no added item.

## 3. Stamps

- **`ROADMAP.md`:** under *"Shipped — the program every front end drives (0.22.0, breaking)"*, add a closing bullet in
  the style of 0.21.1's: ***Released again as 0.22.1**: 0.22.0 let a confirmation that did not fit the window be
  confirmed unread, and one behind the "window too small" notice be confirmed unseen. 0.22.1 is 0.22.0 with that
  fixed.*
- **The install guide's transcript** (`docs/src/guide/install.md:70`): 0.22.0 → **0.22.1** (the carried rule).

## 4. Submit, and the runs before the tag (no branch)

**Before submitting:**
- **all eight gates;**
- the **local real-binary suite at 0.28.0 + 0.58.0**;
- `release.yml`'s `awk` for `0.22.1`;
- `cargo public-api` as in §2;
- **the reading-gate probe on the release build of the prep head**, pasted verbatim:
  `.git-exclude/reports/reading-gate-2026-11-25/probe.sh`. The first line must say `stikk 0.22.1`, and the three
  lines must read `0`, `0` and `1`.

**Package it** to `.git-exclude/review-request/011-release-0-22-1-preparation/review-request-v1.md`.

**After approval:**
1. Push.
2. On `main`, dispatch, at **the SHA to be tagged**:
   - `release-trial.yml`, every leg (RFC 066 decision 4). A red leg stops the cut;
   - the full-matrix suite;
   - supply chain;
   - `installers.yml`;
   - `aur.yml` with `push` unchecked.
3. Report every run and leg at that SHA.

## 5. For the tag (the owner's, recorded here so nothing is missed)

- **`stikk-app`'s trusted publisher is exercised for the first time.** At 0.22.0 the crate was published by hand.
  After the tag, read the crates job's log for `stikk-app` and confirm it was published by the workflow. If it is
  refused, stop: do not publish it by hand without the owner.
- **Read the AUR push job's log,** not its colour (the carried rule).

# Handoff — 0.22.0 release preparation (v1)

**Companion to:** [RFC 011](../../done/011-pre-1-0-api-stability-policy.md).
**Issued** 2026-11-25. **The project owner authorized the 0.22.0 cut** the same day, and the dev team prepares it by
standing permission. **The tag waits for the owner after review.**

**Follows** [0.21.1's prep handoff](./release-prep-0-21-1-handoff-v1.md). Its rules hold except where this one says
otherwise. **Its §5 (the runner-label table) is retired by RFC 066:** the release trial replaces it.

> **What 0.22.0 is.** For a user, the same stikk as 0.21.1. Underneath, its program is its own crate, `stikk-app`
> (RFC 067), the release build is tried before every tag (RFC 066), and the supply-chain check sees every advisory
> (RFC 068). **RFC 065, the GUI toolkit, ships nothing and is not part of this release.**
>
> **Sanctioned:**
> - the version bump, and `Cargo.lock`'s workspace entries;
> - `CHANGELOG.md`, the status texts, `ROADMAP.md`, the install guide's transcript;
> - `docs/src/contributing/releasing.md`.
>
> RFC 011's row already links this handoff; the architect added it.
>
> Anything else: stop and report. **No branch.**

---

## 1. The bump

`0.21.1` → `0.22.0` in `[workspace.package]` and **the seven pins**, `stikk-app`'s included. `cargo update --workspace`:
**only `stikk*` lines may move.**

## 2. `CHANGELOG.md`

`## Unreleased` becomes `## 0.22.0 — <the date you push>`.
- **The bold summary:** *"The same stikk for a user. Its program is now its own crate, ready to be shared with a second
  front end. Every release build is tried before it is tagged, and the supply-chain check sees every advisory in the
  dependency tree."*
- **Keep the existing `### Security` and `### Changed` entries.**
- **Add `### Breaking`,** from `cargo public-api -p <crate> diff 0.21.1..HEAD`. **Run it yourself and confirm** what
  the architect measured on 2026-11-25:
  - **`stikk-tui`:** `App`, `Focus`, `Screen`, `Operation`, `OperationStatus`, `OrientationState`, `RefFocus`,
    `CompareFocus`, `CompareSource`, `PatchSource`, `MarkedBlock`, `Overlay`, `RefNameKind`, `RefWriteKind`,
    `SavePathKind` and the `app` module move to **`stikk-app`**, where each still exists under `stikk_app::`;
  - `run` and `overlay::render` take `stikk_app`'s `Session` and `Overlay`;
  - `handle_key` is added as a free function;
  - **`stikk-app`:** new;
  - **no change:** `stikk-model`, `stikk-prikk`, `stikk-state`, `stikk-core` and `stikk`.

  One line of reasoning: *a front end's imports change, and nothing it can do changes.* Add a line that `Focus` is now
  `Send` and `Sync`.

## 3. Text and stamps

- **The three status texts** say 0.21.x: `crates/stikk/README.md:64`, `docs/src/index.md:23` and
  `docs/src/contributing/releasing.md:110–112`. Make each **0.22.x**, keep its sentence about what a user gets
  (unchanged since 0.21.x), and add one plain sentence: *0.22.0 changes nothing you see; underneath, stikk's program
  is its own crate.* **Promise no GUI.** RFC 065 has not closed.
- **The install guide's *"What you should see"* transcript** (`docs/src/guide/install.md:70`): 0.21.1 → **0.22.0**
  (the carried rule).
- **`ROADMAP.md`:** a *Shipped* section for 0.22.0, in the style of 0.21.0's, with the three increments. RFC 065 is
  listed as **in progress, not shipped**.

## 4. `releasing.md`: the first publish of `stikk-app`

Replace *"The release-prep handoff for 0.22.0 carries the exact steps"* (`:52` or nearby) with the steps below, as a
short section, **"A new crate name at a tag (stikk-app, 0.22.0)"**:
1. **The tag's crates job** publishes the crates below the new one, then fails on it: its Trusted Publishing token
   cannot publish a crate name that does not exist yet. The binaries, the installers and the GitHub Release are not
   affected; the AUR job waits.
2. **From a clean checkout of the tag:** `cargo publish -p stikk-app --locked`.
   - Use a **scoped** crates.io token: scope `publish-new`, crate `stikk-app`, short expiry. crates.io rejects
     unscoped tokens.
3. **Attach the trusted publisher** to the new crate (the same values as the others), then **revoke the token.**
4. **Re-run the failed jobs** of the release run. The crates job skips what is published, and publishes the rest; the
   AUR job follows.

**The list in §1 of the one-time setup already names `stikk-app`.** Check that it reads consistently with this new
section.

## 5. Submit, and the runs before the tag (no branch)

**Before submitting:**
- **all eight gates;**
- the **local real-binary suite at 0.28.0 + 0.58.0**;
- `release.yml`'s `awk` for `0.22.0`;
- `cargo public-api` as in §2.

**Package it** to `.git-exclude/review-request/011-release-0-22-0-preparation/review-request-v1.md`.

**After approval:**
1. Push.
2. On `main`, dispatch:
   - **`release-trial.yml`.** RFC 066 decision 4: every leg, at the SHA to be tagged. A red leg stops the cut;
   - the full-matrix suite;
   - supply chain;
   - `installers.yml`;
   - `aur.yml` with `push` unchecked.
3. Report every run and leg at **the SHA to be tagged**.

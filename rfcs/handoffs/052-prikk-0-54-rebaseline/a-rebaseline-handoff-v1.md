# Handoff A — RFC 052: the prikk 0.54 re-baseline (v1)

**Companion to:** [RFC 052](../../accepted/052-prikk-0-54-rebaseline.md). **Read the RFC first**, F1–F3 and decisions
1–4. **Covers:** all four decisions. 0.16.0's increment 4.
**Not a release.**

> **What this increment is.** The validated ceiling to 54, and **prikk's own Windows installer printed where stikk has
> pointed at prikk's guide** — the switch RFC 048 decision 8 set aside for this re-baseline. stikk still runs nothing
> of prikk's.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main`, and dispatch the
> suite and supply chain there; `installers.yml` runs on the push.

---

## 1. The pinned binary

Install 0.54.1 for the suite: `cargo install prikk --version 0.54.1 --locked --root
~/.local/share/stikk-test-prikk/ceiling-0.54` (the architect's is already there). **0.54.1, not 0.54.0.**

## 2. Commit 1: the ceiling

`VALIDATED_MAX_MINOR` → **54** (`stikk-prikk/src/version.rs:91`), its doc comment saying 0.54.1 was measured and why the
patch matters (RFC 052 F3). Nothing else in this commit. **Run the suite at 0.28.0 + 0.54.1 at this commit** and report
it; RFC 052 F1 measured **69 passed** on a scratch copy.

## 3. Commit 2: Windows prints prikk's own installer

- **`scripts/install.ps1`**: in `Show-PrikkHowToGetIt`, replace the guide-and-release-page text and *"A Windows
  installer is coming from prikk"* with **prikk's one line**, exactly
  `irm https://github.com/prikk-vcs/prikk/releases/latest/download/install.ps1 | iex`, then prikk's guide URL on the
  next line — the shape `install.sh`'s `prikk_how_to_get_it` has. It serves **missing** and **too old** alike. Remove
  the comment that named this re-baseline as the place to change, and the now-unused release-page variable.
- **`scripts/uninstall.ps1`**: prikk is removed by prikk's own uninstaller —
  `irm https://github.com/prikk-vcs/prikk/releases/latest/download/uninstall.ps1 | iex` — as `uninstall.sh` prints
  prikk's.
- **The version report names the minor** (RFC 052 decision 4): the filled ceiling reads `0.54`, not `0.54.0`, in both
  scripts' *"0.28 through …"* lines. Change the fill in `release.yml`'s installer job and `installers.yml`'s
  *derive* job together (and the AUR filler if it reads the same value — say if it does), so the assertions move with
  the scripts.
- **`installers.yml`**: the Windows legs assert the new lines (missing, too old, uninstall), and still assert that
  nothing of prikk's is downloaded or written. **Grep every Windows assertion against what the two scripts print**, as
  you did in RFC 048 A's v3, and say so.

## 4. Commit 3: the text

- The install page's **Step 1 on Windows**: prikk's one line, linked to prikk's guide as the authority; *"prikk does not
  publish a Windows installer yet — one is coming"* goes. `README.md` and `crates/stikk/README.md` likewise (*"prikk's own
  Windows installer is on its way"* goes).
- **`validated through 0.54.1`** in every live statement (`README.md`, `crates/stikk/README.md`, `docs/src/index.md`,
  `getting-started.md`, `version.rs`'s module doc, `requirements.md`'s `ASM-2`).
- `## Unreleased` `### Changed`: RFC 052's Changelog line.

## 5. The traps

1. **0.54.1**, never 0.54.0, for the pinned binary and in prose.
2. **Still nothing of prikk's run, downloaded or pinned** by stikk's scripts.
3. **The minor, not `0.<n>.0`,** in the version report.
4. **Every Windows assertion checked against the scripts' output** — the Windows half still runs only in CI.
5. **Paste captures as the file holds them; grep before committing.**

## 6. Submit (no branch)

The eight gates, `shellcheck`, and the local suite at 0.28.0 + 0.54.1. Package to
`.git-exclude/review-request/052-prikk-0-54-rebaseline/review-request-v1.md`, leading with the suite result at commit 1.

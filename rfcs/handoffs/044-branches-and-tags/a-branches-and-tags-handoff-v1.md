# Handoff A — RFC 044: create a branch, create a tag, close a branch (v1)

**Companion to:** [RFC 044](../../accepted/044-branches-and-tags.md), **accepted 2026-10-17**, no open question. **Read the
RFC first**, F2 and F3 especially.
**Covers:** decisions 1–8. 0.13.0's increment 2.
**Not a release.**

> **What this increment is.** stikk's first writes since seal, and the first that create or retire **refs**. **They
> change pointers, never files.** Every rule commit and seal already follow applies: preview-first, the tier its
> category sets, prikk's refusals verbatim, and nothing run that the preview did not show.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main` and dispatch the
> suite, supply chain and installers there.

---

## 1. Measure first, at 0.52

Re-run `.git-exclude/reports/044-branches-and-tags/measure044.sh` against a pinned 0.52.0 and confirm F1–F3, **F3
especially**: closing the current branch with no queued work succeeds, and leaves a default-source `branch create`
refusing. **If any row differs, stop and report.**

## 2. Commit 1: the seam

Three `Prikk` methods, category **`Publication`** (Tier Three, as seal):
- `create_branch(repo, name, from_ref)` → `branch create <name> --from <from_ref>`. **Always pass `--from`**: stikk
  knows which ref the user chose, and prikk's default source is the current branch, which may not be what the user
  selected;
- `create_tag(repo, name, target, message)` → `tag create <name> --target <target>`, plus `-m <message>` only when the
  message is non-empty. `target` is a ref name or a block id;
- `close_branch(repo, name)` → `branch close <name>`.

Each returns prikk's printed lines **verbatim** on exit 0 (for the result screen). **Do not parse them for state.** The
new state is read back from `branch list` / `tag list`, which stikk already parses. Every other exit classifies as
today. **Band all three at `CURRENT_BRANCH_FROM_MINOR` (0.42)**: below it, spawn nothing and answer *unavailable*.

**Tests:** a stand-in for each success; each F2 refusal, verbatim; and the argument vectors (`--from` always present,
`-m` absent for an empty message, the name passed as **one** argument, so `heads/bad name` reaches prikk intact).

## 3. Commit 2: previews, prompts, keys and views

- **Previews** in `stikk-core`, one per operation, modelled on `seal_preview`. Each builds a `ConfirmationSummary`
  (RFC 044 decision 6): the operation, the new or closed ref's name, the source or target (ref name and short block
  id, from data already read), the tag message if any, and for a close prikk's own *"nothing is reclaimed … the branch
  is recoverable"* in stikk's restatement. **Tier and capability come from `Publication`**, so the existing readiness
  check stops a user without a ready maintainer key **before prikk runs**, in stikk's words.
- **The name prompt**, modelled on `Overlay::CommitMessage`: the fixed prefix (`heads/` or `tags/`) shown, the rest
  typed. **An empty name is not sent.** **No naming rules of stikk's own.** In the preview, the name renders as an
  identifier (RFC 036). **A tag's message prompt** follows the name and may be left empty.
- **Keys** (RFC 044 decision 4), from the free ones:
  - in the ref picker: **`n`** new branch from the selected ref, **`t`** tag the selected ref's tip, **`x`** close the
    selected branch;
  - in History and Block detail: **`t`** tag the selected block, by block id.

  List them in the Glossary with the others. **`n` on a tag row** creates a branch from the tag's tip (`--from
  tags/<n>`), which prikk accepts.
- **Closing the current branch is not offered** (decision 7): `x` on it says *"this is the branch you are on; closing it
  would leave the repository on a closed branch. Switch to another branch first."*, and while RFC 045 has not landed it
  names `prikk branch switch`. **`x` on a closed branch** says it is already closed, from the list's own `closed` flag,
  without running prikk. **`x` and `n` on the unpublished current branch's row** (RFC 037 B): not offered, with stikk's
  reason (it has no tip).
- **After success**: the result screen shows prikk's lines verbatim, and the ref lists are re-read. **A failure keeps the
  user's typed name**, so they can correct it and try again, as the commit message prompt does.
- **Below 0.42**: the keys give stikk's reason (`C-T4d`).

## 4. Tests and the suite

- **Core:** each preview's summary; the tier; the readiness stop without a maintainer key; F3's refusal for the current
  branch; the closed and unpublished rows.
- **Suite legs at 0.52:** create a branch from `heads/main` and see it listed; tag a **block id** from History and see it
  listed at that block; close a non-current branch and see it closed; a **duplicate name** shows prikk's refusal
  verbatim; **the current branch's `x` refuses without running prikk** (assert no `branch close` was spawned). **At
  0.28**, the below-band reason.
- **Captures at exactly 80 columns:** each preview, the name prompt, a result screen, and the current-branch refusal.

## 5. `requirements.md` and the changelog

`FR-070`/`FR-071` gain *create, tag and close delivered at prikk ≥ 0.42; switching is RFC 045*. `## Unreleased`:
`### Added`, the three operations. `### Breaking` for the new `Prikk` methods (no defaults) and anything else
`cargo public-api` shows.

## 6. The traps

1. **Always pass `--from`.** The user chose a ref; prikk's default is a different question.
2. **Never parse the success prose for state.** Read the lists back.
3. **One argument per name.** A space is legal to prikk.
4. **The current branch is never offered for closing**, whatever prikk would allow.
5. **Readiness first.** prikk's signature refusal is the fallback, never the first thing a user sees.
6. **Paste captures as the file holds them; grep before committing**, including for repeated sections.

## 7. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.52.0. Package to
`.git-exclude/review-request/044-branches-and-tags/review-request-v1.md`, leading with §1's measurement.

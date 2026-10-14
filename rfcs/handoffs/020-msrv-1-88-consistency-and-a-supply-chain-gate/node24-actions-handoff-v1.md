# Handoff — the workflows' node20 actions (v1)

**Companion to:** [RFC 020](../../done/020-msrv-1-88-consistency-and-a-supply-chain-gate.md), the RFC that owns the
CI and supply-chain workflows. **A chore, with no RFC of its own**: 0.10.0's increment 4 (`ROADMAP.md`, *Next —
0.10.0*), authorized in the 0.10.0 scope (`.git-exclude/release/0-10-0-scope.md`).
**Issued** 2026-10-14. **Independent of RFC 036**: it touches no Rust source, so it can land before, after or
between that handoff's commits.

> **Why now.** GitHub has deprecated Node.js 20 for actions and already forces node20 actions onto node24. The 0.9.0
> release run (`38487394190`) says so for `release.yml`'s attestation. The next step in that deprecation is a
> failure, and `release.yml` only runs at a tag, so it would surface **during a release**.
>
> **Sanctioned: `.github/workflows/docs.yml` and `.github/workflows/release.yml` only.** Nothing in Rust, no test,
> no other workflow. If a change elsewhere looks necessary, stop and report.

---

## 1. Inventory first: every action, and what it runs on

For **every** `uses:` in **all** workflows (not only the two named), record the pinned ref and **its `action.yml`'s
`runs.using` at that ref**, read from the action's repository (`gh api repos/<owner>/<repo>/contents/action.yml?ref=<ref>`,
decoded). Put the table in the request. **Composite actions** (`using: composite`) run the actions *they* call, so
follow one level in. `actions/attest-build-provenance` is composite, and its node20 is
`actions/attest-build-provenance/predicate` and `actions/attest` inside it, which is what the annotation named.

**Anything still on node20 after §2 and §3 is reported, with why** (for example, no node24 release exists yet).

## 2. Bump what has a node24 release

For each node20 action with a release whose `action.yml` declares `using: node24`, move to that release. **Keep the
repository's convention: pin by tag, as today** (`@v5`), not by SHA, unless a reason applies to that one action. For
each bump, **read the release notes between the two tags** and list in the request anything that changes inputs,
outputs or permissions this workflow relies on. `actions/attest-build-provenance` and the Pages trio
(`configure-pages`, `upload-pages-artifact`, `deploy-pages`) are the likely ones.

## 3. Replace `peaceiris/actions-mdbook`

It has no node24 release. **Replace it with a step that installs a pinned mdBook without a third-party action**:
download the release binary from `rust-lang/mdBook`'s GitHub releases for a **pinned version** and **verify its
checksum** in the step, or `cargo install mdbook --locked --version <pinned>` (slower, cached by `rust-cache`). Your
choice; say why.

**Pin the version.** Today the workflow asks for `latest`, which means an mdBook release can change the site with
no commit in this repository. Pin the version `latest` resolves to today, and **check that `mdbook build docs` gives
the same output** (diff the built `docs/book` from the old and new mdBook locally; equal is expected).

**Gate 7 builds the book locally too.** Note in the request which mdBook version the gates use, so the two stay
equal.

## 4. Guard the Pages deploy to `main`

`docs.yml` has `workflow_dispatch`, and its `deploy` job has **no branch condition**. **Dispatching it from any branch
would publish that branch's docs to the live site.** Add `if: github.ref == 'refs/heads/main'` to `deploy`, so a
dispatch from a `prep/` branch builds and uploads but does not deploy. That is also what makes §5's check possible.

## 5. Verify

- **`docs.yml`:** push to `prep/node24-actions` and **dispatch it there**. `build` runs green, `deploy` is skipped by
  §4's guard, and **the run shows no node20 annotation**. Read the annotations.
- **`release.yml` cannot be run without a tag. Do not create one.** Verify it by the §1 table (every action and
  every composite's inner actions on node24 at the pinned ref), plus `actionlint` or an equivalent parse check if
  available. **The real check is the next release run**, and the request should say so.
- **CI and the supply-chain gate at the same SHA**, as for any change, even though no Rust changed.
- Delete the `prep/` branch before submitting.

## 6. Changelog

**No entry.** Nothing a user of the crates or the release binaries can observe changes. The attestation itself keeps
its format (confirm this from the release notes in §2, and say so).

## 7. Submit

Package to `.git-exclude/review-request/020-node24-actions/review-request-v1.md`: the §1 table (before and after),
the release-note findings, §3's choice and the book-diff result, the `docs.yml` dispatch run with `deploy` skipped
and no node20 annotation, and the CI and supply-chain run ids. **Push your own commits once approved.**

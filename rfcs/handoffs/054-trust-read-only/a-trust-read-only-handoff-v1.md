# Handoff A — RFC 054: trust, read-only (v1)

**Companion to:** [RFC 054](../../accepted/054-trust-read-only.md), **accepted 2026-10-29**. **Read the RFC first**,
F1–F4 and decisions 1–5. **Covers:** all of it. 0.17.0's increment 2.
**Start when RFC 053's [Handoff A](../053-get-files-back/a-get-files-back-handoff-v1.md) is pushed.**
**Not a release.**

> **What this increment is.** A **Trust view** on `T` listing the keys this repository trusts, **yours marked**; and
> on the bundle cards, **beside each sealer, whether this repository trusts it**. Read-only: nothing is added,
> removed or confirmed.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main`, and dispatch the
> suite and supply chain there.

---

## 1. Measure first

Run `.git-exclude/reports/054-trust-read/measure054.sh <prikk> <empty scratch dir>` at **0.54.1** (and 0.55.0 if you
have it) and diff against `measure054-0.54.txt` / `measure054-0.55.txt`, ignoring ids, keys and scratch paths. If any
other row differs, stop and report. **The script generates seeds with `--out` and reads only `public key:` lines; keep
it that way** — never print or store a seed.

## 2. Commit 1: the seam (`stikk-prikk`)

**`trusted_keys(&self, repo) -> Result<Vec<TrustedKey>>`** — `trust maintainer list --format json`, category
**`ReadState`**, banded at **`FILES_FROM_MINOR`** (RFC 053's constant; RFC 054 decision 4 — one measured band; do not
add a second constant). `TrustedKey { key_id, public_key }` from `trust-list-v1`, **in prikk's order**; `keys: []` is
an empty list, not an error. A missing or unknown-typed field is a shape error (`UD-02`).

**`trust maintainer check` is not used** (RFC 054 F2: it exits 1 for "not trusted").

**Tests:** the argument vector; the two captures (empty, two keys) parsed; a truncated report a shape error; below the
band, unavailable and **nothing spawned**.

## 3. Commit 2: the Trust read and the sealer row (`stikk-core`)

**a. `trust::read(prikk, repo) -> TrustView`**: `trusted_keys` plus the existing `key status` read (RFC 026). A row is
**yours** when its `key_id` **and** `public_key` both equal `key status`'s **maintainer** role's. If `key status`
cannot be read, no row is marked and the view says so in one line; the list still shows.

**b. The bundle cards' sealer row** (RFC 050's Bundle view and import card, today `SEALED_BY_*` with
`preview.sealed_by.join(", ")`): **one row per sealer**, each with **one** of *trusted by this repository* · *yours* ·
*not trusted by this repository*, from **one** `trusted_keys` read made with the preview (inside `confirm::preview` for
the import card, so it is re-read at confirmation). prikk's `sealed_by_note` stays verbatim beneath. **If the read
fails**, each row says *whether this repository trusts it could not be read*, and nothing else on the card changes
(RFC 035 A3's rule). **No guard is added**: an untrusted sealer never blocks an import. Below the band: the row is
exactly 0.16.0's.

## 4. Commit 3: the Trust view (`stikk-tui`)

**`T`** anywhere a repository is open, advertised in the help/glossary. The view:

1. **A heading** — *Keys this repository trusts* — and **stikk's sentence** (RFC 054 decision 2): *prikk accepts an
   adopted key's signatures on what that key signed. Adopting a key never lets it move one of your branches.*
2. **One row per key, in prikk's order**: the key id and the public key, both through `identifier` (the 64-hex key
   hard-split, as Block detail splits ids); **yours** marked on the matching row.
3. **None adopted**: *This repository trusts no maintainer key.*, then prikk's command as text — `prikk trust maintainer
   add --key-id ID --public-key HEX` — and *get the public key from its owner, not from a bundle they sent.* stikk runs
   nothing.
4. **The list cannot be read**: the failure verbatim, and nothing else.

**Never shown:** a seed, a seed path, or `key status`'s `path` field (`NFR-S03`). Scrolls like Verify. `Esc` returns.

## 5. Tests and the suite

- **Core:** *yours* needs both id and key to match (a matching id with a different key is **not** yours); `key status`
  unreadable leaves rows unmarked and says so; the sealer row's three answers and the failed read; an untrusted sealer
  does not block import.
- **TUI:** the empty state; two keys with one marked; the long key wrapped to its column; no path or seed text anywhere
  in the rendered buffer (assert on the buffer).
- **Suite legs at the ceiling** (and **unavailable at 0.28**, nothing spawned):
  1. a bare repository (`init` only): the view's empty state;
  2. after `setup`: one row, **yours**;
  3. a second key added in the harness with `prikk trust maintainer add`: two rows, the second unmarked;
  4. a bundle from a second repository whose key the receiver has not adopted: the card says *not trusted*; after
     `trust maintainer add` of that key in the harness, *trusted*.
- **Captures at 80 columns:** the view (empty, and two keys with yours), and a bundle card with each sealer answer.

## 6. Text

- `requirements.md`: `FR-103` — its read half delivered at prikk ≥ 0.54; adding, removing and the TOFU event not.
  `FR-104` unchanged.
- The guide: in **Sharing work**, a paragraph on trust — what the sealer row means, that importing trusts nobody, and
  where `T` shows the keys you trust.
- `CHANGELOG.md` `## Unreleased`: `### Added`, and the `### Breaking` row for the trait's new method.

## 7. The traps

1. **No key material**: ids and public keys only; no seed, no seed path.
2. **`list`, not `check`** — one read, exit 0.
3. **Yours is id *and* public key**, against the maintainer role.
4. **Read-only**: no add, no remove, no confirmation, no guard on import.
5. **Paste captures as the file holds them; grep before committing.**

## 8. Submit (no branch)

The eight gates and the local suite at 0.28.0 and the current ceiling. Package to
`.git-exclude/review-request/054-trust-read-only-a/review-request-v1.md`, leading with §1's measurement diff and suite
leg 4.

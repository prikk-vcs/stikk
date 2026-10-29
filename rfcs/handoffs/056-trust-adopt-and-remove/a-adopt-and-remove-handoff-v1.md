# Handoff A — RFC 056: trust, adopt and remove (v1)

**Companion to:** [RFC 056](../../done/056-trust-adopt-and-remove.md), **accepted 2026-10-29**. **Read the RFC
first**, F1–F4 and decisions 1–7. **Covers:** all of it. This is 0.18.0's increment 1.
**Independent of RFC 053's [Handoff B](../053-get-files-back/b-carried-follow-ups-handoff-v1.md).** Land either one
first, but submit them separately.
**Not a release.**

> **What this increment is.** In the Trust view, `a` adopts a maintainer key and `x` removes one. Each is previewed
> and confirmed by **typing the key id**. These are **stikk's first Tier Three typed operations**: the machinery has
> existed since RFC 013 and is unit-tested, but nothing has used it yet. Neither operation needs a key.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite with `full_platform_matrix: true`, and supply chain, there.

---

## 1. Measure first

Run `.git-exclude/reports/056-trust-write/measure056.sh <prikk> <empty scratch dir>` at **0.54.1** and **0.55.0**.
Diff each against `measure056-0.55.txt`, ignoring ids, keys and scratch paths. If any other line differs at 0.54.1,
stop and report: the band (decision 1) rests on both versions.

The script writes seeds with `--out` and reads only `public key:` lines. **Keep it that way.**

## 2. Commit 1: the seam (`stikk-prikk`)

**`trust_add(repo, key_id, public_key) -> Result<TrustWritten>`** runs `trust maintainer add --key-id ID --public-key
HEX`. **`trust_remove(repo, key_id) -> Result<TrustWritten>`** runs `trust maintainer remove --key-id ID`.

- **Category:** both are **`RequestCategory::Trust`**. Do not change the category's tier or its requirement: both are
  already right (RFC 056 F1).
- **Band:** gate both at `FILES_FROM_MINOR`, the constant `trusted_keys` uses.
- **`TrustWritten`** is prikk's stdout lines, verbatim. **Parse nothing.** A non-zero exit is the usual refusal error,
  carrying prikk's stderr verbatim.
- **Never add a `--format` flag**: prikk refuses it (F1).

**Tests:**

- each argument vector, exactly;
- both success outputs and the *"already trusted"* / *"was not trusted"* outputs carried through as lines;
- a refusal carried verbatim;
- below the band, unavailable, with **nothing spawned**.

## 3. Commit 2: the previews, guards and writes (`stikk-core`, in `trust.rs`)

The shape is `bundle.rs`'s `import_preview` / `import_confirm_and_execute`: **the guards live inside the preview**, so
`confirm` takes them again at confirmation.

**a. `adopt_preview(prikk, repo, readiness, key_id, public_key)`**

1. **Check the inputs (decision 2).** Trim the public key. It must be exactly 64 characters from `[0-9a-f]`. The key id
   must be non-empty and contain no whitespace or control character. A failure returns stikk's sentence, not an
   error. The TUI keeps the prompt open with it.
2. **Read one `trusted_keys`.**
   - The id is adopted with **this** key: decline with *"This repository already trusts this key."*, and spawn no
     `add`.
   - The id is adopted with a **different** key: return the **security event** (decision 4) as its own outcome,
     carrying both public keys. **It is not a confirmation.** Spawn no `add`.
3. **Build the summary.** Category `Trust`; `target_name` is **the key id** (the typed evidence). The asides are the
   key id and the public key. The body holds `WHAT_TRUST_MEANS` and `WHERE_TO_GET_THE_KEY`. The consequence is
   decision 2's *"Work this key sealed will verify here. It cannot move one of your branches."*
   - **Where the id is not `default_key_id(public_key)`**, add decision 2's notice to `notices`.
   - `default_key_id` is `"ed25519-"` followed by the first 16 characters of the public key. Make it public, and test
     it against the measured pairs.

**b. `remove_preview(prikk, repo, readiness, key_id)`**

1. **Read one `trusted_keys`.**
   - The id is not adopted: decline with *"This repository no longer trusts this key."*
   - The list has **one** key: decline with decision 3's sentence (*"prikk keeps at least one trusted key …"*).
   - Spawn no `remove` in either case.
2. **Build the summary.** `target_name` is the key id. In `notices`, **always** put decision 3's sentence: *"If this
   key sealed blocks in this repository, Verify will fail them until the same key is adopted again under the same
   id."*
   - **Where the row is yours**, add *"This is your own key. …"* as well. "Yours" means id **and** public key match
     `key status`, which is `is_yours` from RFC 054. A failed `key status` costs only this notice.
   - The consequence is decision 3's *"prikk will no longer accept this key's signatures in this repository."*

**c. The two `*_confirm_and_execute` functions** run the write, then **re-read `trusted_keys`**. They return
`TrustWritten` and the new `TrustView`. A removal's result also carries *"Press `V` to see what Verify says now."*

**d. A refused adopt (decision 5).** prikk's words go verbatim under stikk's heading, *"prikk refused to adopt this
key"*. **Do not match on prikk's text**: decision 5 says why.

**Tests:**

- every guard declines with **zero writes spawned**, proved by a call counter, as RFC 050 B did;
- the security event carries both public keys;
- the default-id notice appears exactly when the id is not the key's default;
- the "yours" notice needs both halves to match, and is absent when `key status` fails;
- `confirm` refuses evidence that is not the typed key id: an explicit yes, a near miss, and empty input;
- a stale change token refuses.

## 4. Commit 3: the Trust view acts (`stikk-tui`)

1. **A row cursor** (`↑`/`↓`) over the keys. The view still scrolls, and the cursor's row is always visible.
2. **`a`** opens a **public-key prompt**, then a **key-id prompt prefilled with `default_key_id`**, which the user may
   edit.
   - Model both prompts on `RefName`'s overlay.
   - **Pasting a 64-character key must work.** Test it with one input event per character, and with a single pasted
     string if the event loop delivers one.
   - `Esc` in either prompt cancels and spawns nothing.
3. **`x`** on the selected row opens the remove preview. A guard's decline shows in place.
4. **The confirmation is the existing `Confirmation` overlay** at `Tier::ThreeTyped`. It asks for the key id. **Read
   how its typed input is drawn and checked before you rely on it**: this is its first real use. Any gap you find
   (prompt wording, a long id, the echo of what was typed) is a finding to report, not something to work around
   quietly.
5. **The security event** (decision 4) is its **own overlay**, drawn in warn, with both public keys hard-split and no
   confirm key. Only `Esc` closes it.
6. **The results:**
   - prikk's lines verbatim, then the new list (the view re-renders from the returned `TrustView`);
   - after a removal, the `V` pointer;
   - a refused adopt under decision 5's heading.
7. **Advertise `a` and `x`** in the Trust view's title hint and in the glossary.

**Never shown, as before:** a seed, a seed path, or `key status`'s `path`. Assert this on the rendered buffer for every
new card and prompt.

## 5. Commit 4: the suite (`stikk-real-binary`)

Write RFC 056's eight legs at the ceiling, plus *unavailable* at 0.28 with nothing spawned.

- **Leg 7 is the one that matters.**
  - **Setup:** the fixture seals under its key. Adopt a second key so that removal is allowed, then remove the
    fixture's key through `remove_confirm_and_execute`.
  - **Assert:** `stikk_core::verify::verify` reports `ok: false` with `publication-trust`. Adopt the same key under the
    same id, through `adopt_confirm_and_execute`. Verify reports `ok: true`.
  - **Doc:** the leg's doc says it is the evidence for decision 3's sentence, and that if prikk changes this, it goes
    red.
- **The fixture's key id is `maintainer`** (`support.rs`, `MAINTAINER_KEY_ID`), not a default id. That is fine for the
  legs. Leg 1 should adopt a key **with** its default id, so the no-notice path also runs against real prikk.
- Generate every key with `key generate --out`, and read only the public key, the way `measure056.sh` does. **Never put
  a seed in an assertion message.**

## 6. Text

- **`requirements.md`:** in `FR-103`, the write half is delivered at prikk ≥ 0.54. Name what stays undelivered: tag
  adoption, author keys, and counting the blocks a removal affects.
- **The guide**, in **Sharing work**, after RFC 054's subsection: a short **Adopting and removing keys** section. Cover
  where a public key should come from, what a removal can do to Verify and how to undo it, and what the security-event
  card means.
- **`CHANGELOG.md` `## Unreleased`:** the RFC's `### Added`, and the `### Breaking` row for the two trait methods plus
  anything else `public-api` shows.

## 7. The traps

1. **Typed evidence is the key id, matched exactly.** Never fall back to an explicit yes.
2. **Guards inside the preview, with zero writes when they decline.**
3. **Parse none of prikk's prose**, neither the success lines nor the refusals.
4. **No key material**, in the view, the cards, the prompts, the tests or their messages.
5. **Paste captures as the file holds them, and grep before committing.**

## 8. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/056-trust-adopt-and-remove-a/review-request-v1.md`. Lead with §1's measurement diff, suite
leg 7, and anything §4 item 4 found. Include the 80-column captures the RFC's *Delivery* lists.

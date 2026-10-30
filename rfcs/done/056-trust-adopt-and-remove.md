# RFC 056 — Trust, adopt and remove: choosing whose keys this repository trusts

**Status.** **Done 2026-10-30**: Handoff A delivered on `main` (`de50098`..`e33b31a`); **released in 0.18.0**. **Accepted by the project owner 2026-10-29**; [Handoff A](../handoffs/056-trust-adopt-and-remove/a-adopt-and-remove-handoff-v1.md). **Proposed 2026-10-29** by the architect, as 0.18.0's increment 1 (`.git-exclude/release/0-18-0-scope.md`,
authorized 2026-10-29). **No open question**: the scope's one design point, how to present a refused adopt, is decided
here (decision 5).
**Measured** against a real prikk **0.55.0**: `.git-exclude/reports/056-trust-write/` (`measure056.sh` →
`measure056-0.55.txt`). It covers one repository after `setup`, a second repository whose history its own key sealed,
and trust writes with no key material at all. Seeds went to `--out` files; only `public key:` lines were read.
**Tracks.** `FR-103` (its write half), `UC-15`, `P-4`, `NFR-S03`, `NFR-S04`, `C-T2a`, `ER-02`; RFC 013 (Tier Three
typed), RFC 026 (`key status`), RFC 054 (the Trust view, `trusted_keys`).
**Touches.** `stikk-prikk` (two seam methods), `stikk-core` (two previews and two writes, with their guards),
`stikk-tui` (a row cursor and two keys in the Trust view, two prompts), `stikk-real-binary`, `requirements.md`, the
guide, the changelog.

## Summary

0.17.0 shows which keys a repository trusts, and its bundle cards say which trusted key a sealer's id *names*. The
receiver's next step is to act on that: **adopt a collaborator's key**, so that what they seal verifies here, or
**remove a key** that should no longer be trusted. stikk does both from the Trust view, each previewed and confirmed
by **typing the key id**. The cards say what prikk does not say before it acts. **Removing the key that sealed your
history makes Verify fail it until that key is adopted again**, and removing your own key stops you sealing. A
different key under an id the repository already trusts is shown as the security event it is.

## Findings (measured at 0.55.0)

### F1 — two commands, prose only, no key

- **`trust maintainer add --key-id ID --public-key HEX`** prints *"trusted maintainer key: ID"* and *"adopted
  maintainer keys: N"*, exit 0.
  - Adding the **same** key again prints *"maintainer key already trusted: ID"*, exit 0.
  - A public key that is not 64 lowercase hex characters is refused: *"malformed persisted data: maintainer public key
    must be 64 lowercase hex characters"*, exit 1.
- **`trust maintainer remove --key-id ID`** prints *"revoked maintainer key: ID"*, exit 0. An id that is not adopted
  prints *"maintainer key was not trusted: ID"*, **also exit 0**.
- **Neither takes `--format json`**: `--format` is refused as an unknown argument, exit 2.
- **Neither needs key material.** Both ran with no seed anywhere and `key status` reporting both roles unusable.

### F2 — a different key under a known id is refused, even after removal

Adopting a **different** public key under an id already adopted is refused, exit 1:

> *"precondition not met: maintainer key id ID is already adopted with a different public key. A key id travels in
> every signature, so adopting this key under another local id would match none of its blocks: its maintainer must
> sign under a distinct id …"*

**The same refusal holds after that id is removed**: re-adopting a removed id with another key is refused with the
same words. `trust-list-v1` lists only current keys, so stikk can see the first case before asking and **cannot see
the second**.

### F3 — prikk does not tie an id to its key

prikk accepted `ed25519-2222222222222222` for a key whose own default id is different. A key's default id is
`ed25519-` plus the first 16 hex characters of its public key (prikk 0.45.0's rule, the one `setup` and
`key generate --out` use), and stikk can compute it. A mismatch is legitimate when the key's owner signs under a chosen
id. It is also what a mix-up looks like.

### F4 — removal: the last key, your own key, and the history a key sealed

- **The last key cannot be removed**: *"integrity error: cannot remove the last trusted maintainer key; a repository
  policy must not be explicitly empty"*, exit 1.
- **Removing your own key succeeds**, and then `seal` refuses: *"maintainer signer key id ID is not trusted by policy
  …"*. `key status` then reports the maintainer role as `binding: not-adopted`.
- **Removing the key that sealed your history makes `verify` fail the repository.** Every block it sealed, its
  ref-state and its ref-update read *"has no trusted MAINTAINER signature"* (`PRIKK-TRUST-PUBLICATION-UNTRUSTED`), and
  the verdict fails `publication-trust`. **Adopting the same key under the same id again repairs it**: `verify` is
  green again.
- **`log-report-v1` does not say which key sealed each block**, so stikk cannot count the affected blocks from JSON.
  Only `verify`'s prose names a block's sealer.

## Decisions

1. **Two seam methods**, both category **`Trust`**, which is already Tier Three typed with requirement `ReadWrite`
   (`category.rs`), matching F1:
   - `trust_add(repo, key_id, public_key) -> TrustWritten`;
   - `trust_remove(repo, key_id) -> TrustWritten`.

   `TrustWritten` carries prikk's stdout lines **verbatim**, and stikk parses none of them. After every success, stikk
   **re-reads `trusted_keys`** and shows the new list: that is the answer, and the prose is prikk's account of it. A
   non-zero exit is a refusal, shown verbatim (`ER-02`).

   **Band:** the Trust view's own, `TRUST_FROM_MINOR`. Below it, `T` is unavailable and so are these.

2. **Adopt: `a` in the Trust view.**
   - **Two prompts**, the public key first. The id prompt is then **prefilled with that key's default id** (F3), and
     the user may edit it.
   - **stikk checks both before prikk is asked:**
     - the public key must be exactly 64 lowercase hex characters, with surrounding whitespace trimmed from a paste;
     - the id must be non-empty and contain no whitespace or control character.

     A failure keeps the prompt open and says why in stikk's words.
   - **The guards, inside `confirm::preview`, so they are taken again at confirmation.** Both read one
     `trusted_keys`, and each declines with **nothing spawned**:
     - the id is adopted **with this key**: *"This repository already trusts this key."*;
     - the id is adopted **with a different key**: the security-event card (decision 4).
   - **The card** shows the key id and the public key (hard-split, as the Trust view draws it). Then, in stikk's
     words:
     - **what adopting means**: `WHAT_TRUST_MEANS`;
     - **the caution**: `WHERE_TO_GET_THE_KEY`;
     - **where the id is not the key's default** (F3), a notice: *"This id is not this key's default
       (`ed25519-…`). That is allowed, but it must be the id its owner signs with: ask them."*;
     - **the consequence**: *"Work this key sealed will verify here. It cannot move one of your branches."*

     Confirm by typing the **key id**.

3. **Remove: `x` on a row of the Trust view**, which gains a row cursor (`↑`/`↓`).
   - **Never offered for the last key** (F4). On a list of one, `x` says *"prikk keeps at least one trusted key; adopt
     another before removing this one."*, and nothing is spawned.
   - **The card**, after the key id and public key, carries these notices, in warn, above the consequence:
     - **always**: *"If this key sealed blocks in this repository, Verify will fail them until the same key is
       adopted again under the same id."* stikk cannot count those blocks (F4), so the sentence says *if*. This is
       the measured worst case, and the reason this card exists;
     - **where the row is yours** (RFC 054's id **and** public key match): *"This is your own key. After removing it,
       you cannot seal here until it is adopted again."*
   - **The consequence**: *"prikk will no longer accept this key's signatures in this repository."* Confirm by typing
     the **key id**.
   - **The guard**, inside `confirm::preview`: if the id is no longer adopted, decline with *"This repository no longer
     trusts this key."*, and nothing is spawned.
   - **After a removal**, the result says what prikk printed and adds stikk's pointer: *"Press `V` to see what Verify
     says now."*

4. **A different key under a known id is a security event, said so** (`FR-103`). Where the guard in decision 2 sees
   it, stikk shows its own card, in warn, and offers no confirmation:

   > *"This repository already trusts a different key under the id ID. Two keys cannot share an id: an id travels in
   > every signature. If its owner really has a new key, they must sign under a new id, and you adopt it under that
   > one. If they did not tell you they changed keys, treat this key as someone else's."*

   It shows both public keys, the adopted one and the one given, so the user can see they differ.

5. **A refused adopt is shown in prikk's words**, which is the case decision 4 cannot see (F2, a removed id). stikk
   does not parse prose to tell which refusal it was. **prikk's refusal for that case already says exactly what
   happened** (*"already adopted with a different public key … its maintainer must sign under a distinct id"*), so
   showing it verbatim, under stikk's heading *"prikk refused to adopt this key"*, presents it as what it is. The
   held letter item asks prikk for a `refusal-v1` id, so a later RFC can give it decision 4's card too.

6. **No key material, as before.** Ids and public keys only, with no seed, no seed path and no `key status` `path`
   (`NFR-S03`). The two writes need none (F1).

7. **Not in this RFC:** tag adoption, author keys, counting the blocks a removal affects (decision 3; held for a
   letter), and running Verify automatically after a removal. Verify can be slow on a large repository, so stikk
   points at `V` and does not run it.

## Changelog

- `### Added`: adopt a maintainer key from the Trust view (`a`) and remove one (`x`), each previewed and confirmed by
  typing the key id. The remove card says that removing a key which sealed your history makes Verify fail it until
  the key is adopted again. At prikk ≥ 0.54.
- `### Breaking`: whatever `cargo public-api` shows. The `Prikk` trait gains two required methods.

## Delivery

**One handoff, after RFC 053's Handoff B or alongside it**, since the two touch different files.

1. It re-runs `measure056.sh` at **0.54.1** and **0.55.0** first; the band rests on both.
2. **Suite legs at the ceiling**, each with the list re-read through `trusted_keys`:
   1. adopt a new key → the list grows;
   2. adopt the same key → declined, **zero `add` spawned**;
   3. adopt a different key under an adopted id → decision 4's card, zero spawned;
   4. remove a key, then adopt a different key under its id → prikk's refusal, verbatim (F2);
   5. remove → the list shrinks;
   6. a list of one → `x` declines, zero spawned;
   7. **remove the key that sealed the fixture's history → stikk's Verify fails with `publication-trust`; adopt it
      again → Verify passes.** This leg is the evidence for decision 3's sentence;
   8. remove your own key → `key status`'s binding is `not-adopted`.

   **At 0.28**, unavailable, with nothing spawned.
3. **Captures at 80 columns:** the adopt card with and without F3's notice; the remove card for a key and for your
   own; decision 4's card; a refusal.

## What this RFC does not do

- **Show or ask for any key material** beyond ids and public keys.
- **Run `check`**, or parse prikk's prose.
- **Count** the blocks a removal affects.
- **Adopt tags or author keys.**

## Delivered

**One handoff, six commits, green on `main` at `e33b31a`**: CI, Docs, supply chain, and the real-binary suite on the
full platform matrix (85 cases at 0.28.0 and 0.55.0 on Linux and macOS; 84 on Windows, where the execute-bit leg is
`#[cfg(unix)]`). F1–F4 reproduced at 0.54.1 and 0.55.0.

- **The seam**: `trust_add` and `trust_remove`, category `Trust`, banded at `FILES_FROM_MINOR`, through one private
  helper. prikk's lines are carried verbatim and nothing is parsed. `--format` is asserted absent.
- **The guards**, inside `confirm::preview`, so they run again at confirmation. Each declines with no write spawned,
  proved by counters:
  - already trusted;
  - a different key under a known id (`TwoKeys`, the security event, never armed);
  - no longer trusted;
  - the last key.
- **The cards**, confirmed by typing the key id:
  - adopt carries what trust means, where a key should come from, and a notice when the id is not the key's default;
  - remove always carries the Verify sentence, plus the "your own key" notice when id and public key both match.
- **Suite leg 7 is the evidence for decision 3**: removing the key that sealed the fixture's history fails stikk's
  Verify with `publication-trust`, and adopting it again passes.
- **Accepted deviation**: the public key is in an aside's body, which wraps, rather than in a row that clips.
- **Carried to the confirmation overlay's follow-up**, with its older *prose against the right border* item:
  - a long key id clips the typed confirmation's prompt and echo, pinned by
    `capture_rfc056_typed_confirmation_at_80_columns`;
  - a 64-hex key splits 63 + 1 rather than 32 + 32.
- **Held for the next necessary letter**: a per-block sealer in `log-report-v1` (to count what a removal breaks), and a
  `refusal-v1` id for the different-key refusal (to give a removed id the security-event card too).

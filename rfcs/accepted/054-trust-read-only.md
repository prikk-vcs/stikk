# RFC 054 — Trust, read-only: which keys this repository trusts

**Status.** **Accepted by the project owner 2026-10-29**; [Handoff A](../handoffs/054-trust-read-only/a-trust-read-only-handoff-v1.md) **delivered 2026-10-29** (pushed `18b4223`), with **decision 3 amended** in its review. **Proposed 2026-10-29** by the architect, as 0.17.0's increment 2 (`.git-exclude/release/0-17-0-scope.md`,
authorized 2026-10-29). **No open question.**
Measured against a real prikk **0.54.1**: `.git-exclude/reports/054-trust-read/` (`measure054.sh` →
`measure054-0.54.txt`, and `measure054-0.55.txt` at 0.55.0, identical but for the version: a bare repository, one after `setup`, a second adopted key, and a receiver that adopted
nothing). Key material: seeds generated with `--out` into the scratch directory; only `public key:` lines were read.
**Tracks.** `FR-103` (its read half), `FR-104`, `UC-15`, `NFR-S03`, `C-T2a`, `UD-02`; RFC 026 (`key status`), RFC 050
(the bundle cards' *sealed by*).
**Touches.** `stikk-prikk` (one seam method), `stikk-core` (a Trust read, the bundle cards' sealer row), `stikk-tui`
(a Trust view on `T`), `stikk-real-binary`, `requirements.md`, the guide, the changelog.

## Summary

0.16.0 brought bundles in from other people, and its cards say who sealed one — prikk's key id, with prikk's
*"continuity only — not a trust decision"*. **The receiver's next question is whether that is a key they trust**, and
stikk shows no trust at all. prikk lists the adopted maintainer keys as JSON. stikk shows them in a **Trust view**,
marks **your own** key, and says **on the bundle cards which adopted key, if any, each sealer's id names**
(decision 3, amended). Nothing is added or removed: changing trust is Tier Three Typed, with a changed key under a known id as a security
event (`FR-103`), and gets its own RFC after this one.

## Findings (measured at 0.54.1)

### F1 — the list is JSON; its note is prose only

`trust maintainer list --format json` → **`trust-list-v1`**: `keys`, each `{key_id, public_key}`, **in adoption
order**; `[]` in a repository that adopted nothing (prose: *"no maintainer keys adopted"*). The prose form adds a
note **absent from the JSON**: *"this is object trust, not ref authority -- an adopted key's signatures are accepted on
objects it signed, but adopting a key never lets it move a ref by itself"*. Reads only; exit 0 either way.

### F2 — `check` exits 1 for "not trusted"

`trust maintainer check --key-id ID --format json` → `trust-check-v1` (`trusted`, `public_key`). **`trusted: false`
exits 1 with a valid report**, for an unknown id and a malformed one alike. One `list` answers every id at once, with
exit 0.

### F3 — after `setup`, the list holds your own maintainer key

`setup` adopts the maintainer key it creates: the list's one row has the `key_id` and `public_key` that `key status
--format json` reports for the `maintainer` role (`binding: matches`). A repository made with `init` alone lists none.

### F4 — a bundle's `sealed_by` ids are the list's ids

`bundle-preview-v1`'s `sealed_by` carries `ed25519-<16 hex>`, the form `trust-list-v1` uses. In a receiver that adopted
nothing, the sealer is not trusted (`check`: `trusted: false`).

## Decisions

1. **One seam method**, `trusted_keys(repo) -> Vec<TrustedKey>` (`trust maintainer list --format json`), category
   **ReadState** (Tier One, as `key status` is). **`check` is not used** (F2): one list answers every row and every
   sealer with exit 0. A missing or unknown field is a shape error (`UD-02`).

2. **The Trust view, on `T`** anywhere a repository is open. Read-only; nothing to confirm.
   - **Each adopted key, in prikk's order**: its key id and its public key, both through `identifier`; the row whose
     id and public key match `key status`'s maintainer role (RFC 026's read) is marked **yours**.
   - **What trust means here, in stikk's words** (F1's note is prose-only, so stikk says it rather than spawn twice):
     *prikk accepts an adopted key's signatures on what that key signed. Adopting a key never lets it move one of your
     branches.*
   - **None adopted**: *This repository trusts no maintainer key.* With what to do in prikk's own command — `prikk
     trust maintainer add --key-id ID --public-key HEX` — and stikk's caution: *get the public key from its owner, not
     from a bundle they sent.* stikk does not run it.
   - **The list cannot be read**: the failure, verbatim; nothing else in stikk depends on it.
   No seed path, seed or other key material is shown (`NFR-S03`); `key status`'s `path` field is not displayed.

3. **The bundle cards' sealer row** (RFC 050's Bundle view and import card). Beneath each `sealed_by` id, **one of**:
   *names a key this repository trusts* · *names your key* · *not trusted by this repository*, from one `trusted_keys`
   read made with the preview. Wherever a row names an adopted key, stikk's sentence follows the rows: *A bundle names
   its sealer but does not prove it. Import it, then Verify (V) checks each signature against the key trusted here.*
   prikk's continuity note stays verbatim beneath. If that read fails, the row says *whether this repository trusts it
   could not be read*, and the rest of the card is unchanged (RFC 035 A3's rule). **No guard is added**: importing an
   untrusted bundle is what prikk's received refs are for.

   *Amended 2026-10-29 by the architect, in Handoff A's review (v1), before anything shipped.* The decision first
   read *trusted by this repository* · *yours*. **`sealed_by` is the id the sealer chose, checked against nothing at
   preview time.** Measured at 0.55.0 (`spoof054.sh`, `spoof054-0.55.txt`, beside this RFC's other reports): a sender
   adopts its own public key under the **receiver's** id and seals with it. `bundle preview` then reports the
   receiver's id and `bundle import` exits 0; only `verify` afterwards fails, with `publication-trust`
   (`PRIKK-TRUST-PUBLICATION-INVALID-SIGNATURE`). So a card can say only what an id **names**. The Trust view's
   *yours* is unaffected: it compares id **and** public key. The dev team found the case in the suite, where the
   harness gives every fixture's key the id `maintainer`. prikk's own default since 0.45.0 is
   `ed25519-<16 hex of the public key>`, so honest strangers do not normally collide, but a sender can choose any id.
   Suite leg `rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import` pins the measured case.

4. **Band: prikk ≥ 0.54**, the measured version, as RFC 053's band. Below it, `T` answers *unavailable* and the sealer
   row is prikk's id and note alone, as in 0.16.0.

5. **Not in this RFC**: adding or removing a key, the TOFU security event, tag adoption, and author keys (`trust-list-v1`
   lists maintainer keys only).

## Changelog

`### Added`: a Trust view (`T`) listing the keys this repository trusts, with yours marked; the bundle cards say
which adopted key, if any, each sealer's id names, and that Verify after import is what proves it — at prikk ≥ 0.54. `### Breaking`: whatever `cargo public-api` shows (the `Prikk` trait
gains a required method).

## Delivery

**One handoff, after RFC 053's.** It re-runs `measure054.sh` at 0.54.1 first. **Suite legs at 0.54**: an empty list in a
bare repository; after `setup`, one row marked yours; a second adopted key unmarked; a bundle from a repository whose
key the receiver has not adopted shows *not trusted*, and after `prikk trust maintainer add` in the harness, *names a
key this repository trusts*. **At 0.28**, unavailable.

## What this RFC does not do

- **Change trust** (decision 5).
- **Show any key material** beyond public keys and ids.
- **Decline an import** because its sealer is not trusted.

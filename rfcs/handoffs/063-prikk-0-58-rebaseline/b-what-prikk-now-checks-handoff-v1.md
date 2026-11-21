# Handoff B — RFC 063: what prikk now checks for you (v1)

**Companion to:** [RFC 063](../../done/063-prikk-0-58-rebaseline.md), **accepted 2026-11-18**. **Read the RFC
first**, F5–F7 and decisions 6–8. **Covers:** decisions 6–8.
**Starts after [Handoff A](./a-what-0-56-changed-handoff-v1.md) is approved**, on top of it: A's fixture ids and
`rfc054` change are this handoff's base.
**0.21.0's increment 1, second half.** **Not a release.**

> **What this increment is.** Since 0.56, prikk checks a bundle's seals itself, against the keys this repository
> trusts. From 0.58 it also tells stikk, before an import, everything the import would refuse. stikk's bundle card
> still says *"A bundle names its sealer but does not prove it"*, which is no longer true (F6). This handoff makes the
> card say what prikk checked, withholds Import when prikk would refuse it, and shows a seal that fails against a
> trusted key, and a different key under a removed id, as the security warning they are.
>
> **No branch.** As Handoff A: local gates and suite, then after approval push, and dispatch the full-matrix suite and
> supply chain on `main`.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does prikk still answer as `measure063.sh` §D and §E recorded? | raw prikk | §1's diff |
| Does each parser read the new fields, and refuse a wrong shape? | unit | test names |
| Do the bands (56, 58) pick the right path at 0.55, 0.56, 0.57 and 0.58? | unit | test names |
| Do the bundle card's three outcomes, its refusal list and the warning card read as decided, and fit? | headless (`TestBackend`) | text captures from the buffer, at 80 × 24 and 120 × 40, dark and light: one per outcome, one with refusals, the warning card from each caller |
| Does stikk's own path meet real prikk as decided? | real binary, 0.28.0 + 0.58.0 | the new legs (§5), and the counts |
| macOS and Windows | real binary, full matrix | CI after the push |

The warning reuses an existing card form, and nothing changes colour, so no real-terminal screenshot is needed.

## 1. Measure first

Run `.git-exclude/reports/063-prikk-0-58-rebaseline/measure063.sh <0.58 prikk> <empty scratch dir>`, and diff §D and §E
against `measure063-0.58.txt`. Ignore ids. **Anything else differing: stop and report.**

## 2. Commit 1: the seam (`stikk-prikk`)

1. **Bands** in `version.rs`, each documented with its RFC 063 finding:
   - `SEAL_OUTCOMES_FROM_MINOR: u32 = 56` (F6);
   - `IMPORT_REFUSALS_FROM_MINOR: u32 = 58` (F6);
   - `TRUST_ADD_JSON_FROM_MINOR: u32 = 56` (F7).
2. **The bundle preview parser** (`parse_json.rs:1546-1592`):
   - **At ≥ 56,** `sealed_by_signatures` must be present: an array of `{key_id: string, outcome: "sound" |
     "unverifiable" | "invalid"}`. Any other outcome, or a missing field, is the parser's shape error naming it. Read
     it into **`seal_outcomes: Option<Vec<SealOutcome>>`** on the preview model, `None` below the band.
   - **At ≥ 58,** `import_refusals` must be present: an array of objects. Read **`id`** and **`message`** (both
     strings) and nothing else, into **`import_refusals: Option<Vec<ImportRefusal>>`**, `None` below the band.
   - `sealed_by` is still read as today.
3. **`trust maintainer add`** (`cli_backend.rs:1256-1275`). At ≥ 56, pass `--format json`.
   - **On success,** read `trust-add-v1`: `key_id`, `public_key`, `newly_added` and `adopted_count`.
   - **On a non-zero exit,** read stdout's `refusal-v1` `refusal` id, as `extract_report` reads its document
     (`cli_backend.rs:209-227`).
     - The two key-id conflicts come back as a **typed** outcome carrying prikk's `message`:
       `maintainer-key-id-adopted-with-different-key` and `maintainer-key-id-bound-to-removed-key`.
     - Every other id, or no document at all, keeps today's path through `classify`.
   - Below the band: prose, as today. Update the comment at `:351`, which says prikk refuses `--format`; that is true
     only below 0.56.

**Tests:**
- the parsers, from the 0.58 captures (§D1–D3, §E0–E1), and a wrong shape for each field;
- each band at its edge: 55 and 56, 57 and 58;
- the argv at 55 and at 56.

## 3. Commit 2: core (`stikk-core`)

1. **The sealer rows** (`trust.rs`, `sealers()`, `:302`). When `seal_outcomes` is `Some`, each sealer row's sentence is
   decision 6's, keyed by **outcome**, and stikk's own name match is not consulted. A row is `sound`, `unverifiable`
   or `invalid`; an `invalid` row also marks the preview as carrying a **security warning**.
   - `NAMES_BUT_DOES_NOT_PROVE` (`:198`) stays only when `seal_outcomes` is `None`.
   - Below 56, everything is as today.
2. **Import is withheld** when prikk says it would fail:
   - **At ≥ 58,** when `import_refusals` is non-empty, `import_preview` (`bundle.rs:403`) returns its blocked outcome
     with the refusals themselves, not a string: id and message, and no token.
   - **At 56–57,** when any outcome is `invalid`, `import_preview` returns blocked with the warning. prikk refuses that
     import from 0.56 (F5), and at 56–57 the preview does not list refusals.
   - **Keyed by id and outcome, never by words.**
3. **The warning's content** (decision 8): one type, built by the two callers. Generalise `TwoKeysOneId` (`trust.rs`),
   or put a sibling beside it that the same card renders. It has:
   - a heading;
   - rows of label and value;
   - a body;
   - prikk's message, when there is one.

   Its content per caller:
   - **The trust caller**, for both conflict ids from §2.3:
     - heading: today's heading;
     - rows: `key id`, then `offered now` (the public key);
     - body: `TWO_KEYS_ONE_ID`, then, for the removed id, decision 7's added line;
     - detail: prikk's message.
   - **The bundle caller**, for an `invalid` outcome:
     - heading: decision 8's;
     - rows: `key id`, then `trusted here` (the public key adopted under that id, from the trust list the card already
       reads);
     - body: decision 8's.

   A removed id is not in the list, so the trust path finds it only from prikk's refusal (F7). That is fine: the card
   comes from prikk's typed answer.

**Tests:**
- each outcome's sentence;
- `NAMES_BUT_DOES_NOT_PROVE` present only below 56;
- import withheld: at 58 by a refusal; at 56 by an `invalid` outcome; and **not** at 58 when the list is empty, even
  with `unverifiable` rows;
- the warning built from each caller;
- zero mutating spawns in every withheld case.

## 4. Commit 3: the interface (`stikk-tui`)

1. **The bundle card** (`view/bundle.rs`, the sealed-by block at `:296-370`). Each sealer row shows its outcome
   sentence.
   - When refusals are present, a section under the heading *"prikk would refuse this import"* lists each `message`
     verbatim, through `inert`, wrapped and never clipped. The card's key line does not offer Import.
   - When there is an `invalid` row, the warning card opens over the bundle card. Esc returns to the bundle card, which
     still shows its refusals.
2. **The warning card** is today's `Overlay::TrustConflict` (`overlay.rs:3140-3200`), fed by §3.3's type. It keeps the
   title *Two keys, one id* and Esc-only actions. **One renderer, two callers.**
3. **Adopt.** The two conflict ids open that card, in place of the generic `PRIKK_REFUSED_ADOPT` gloss
   (`present.rs:595`). Every other refused adopt keeps the gloss.
4. **The adopt result** at ≥ 56 is read from `trust-add-v1` rather than printed lines. Its words: *"Adopted <id>. This
   repository now trusts <n> maintainer keys."*

## 5. Commit 4: the suite (`stikk-real-binary`)

At the ceiling (0.58):
1. **`rfc063_the_bundle_card_reads_each_seal_as_prikk_checked_it`**: §D's three receivers through stikk's own
   `open_bundle`.
   - The clash (built with A's `build_with_maintainer_id`) is `invalid`, with Import withheld by the
     `forged-publication-signature` refusal. **Nothing is imported:** assert that `refs` is unchanged.
   - The not-adopted key is `unverifiable`; the adopted one is `sound`. Both imports go through stikk and land.
2. **`rfc054_a_sealer_naming_a_trusted_key_is_proved_only_by_verify_after_import`, at ≥ 0.58:** add that stikk's
   import preview is blocked by that refusal, before any import is spawned.
3. **`rfc056_removing_shrinks_the_list_and_a_removed_id_is_prikks_refusal_at_the_ceiling`, at ≥ 0.56:** the re-adopt
   of a removed id under another key comes back as the typed conflict with `…-bound-to-removed-key`, and the list is
   unchanged. Below 0.56: today's assertion.
4. **The floor (0.28):** the bundle and trust bands are below their minors, so nothing new runs. Say so in the
   request.

## 6. Text

- **`requirements.md`, `FR-103`:** append one italic paragraph in its form.
  - Delivered by RFC 063, at prikk ≥ 0.56: the card shows prikk's own check of each seal; at ≥ 0.58, it withholds an
    import prikk would refuse; a different key under a removed id is the same security warning.
  - It corrects the paragraph's last sentences, *"checked against nothing … both preview and import run clean
    (measured at 0.55.0)"*: they stay true of 0.55, and from 0.56 prikk refuses that import.
- **`CHANGELOG.md`, `## Unreleased`, `### Added`:** the bundle card shows whether each seal checks out, and what an
  import would refuse; the two-keys warning for a removed id, and for a seal that fails against a trusted key.

## 7. The traps

1. **Withhold by id and outcome only.** Never read a refusal's or an outcome's words.
2. **`unverifiable` is not a warning.** It means this repository trusts no key under that id. It is the ordinary case
   for a first exchange, and it must not withhold Import.
3. **The preview is not the last word** (prikk's own note). The import's own refusal path stays exactly as it is.
4. **`inert`** on every key id, public key and message.
5. **One renderer for the warning.** A second overlay that looks the same will drift.

## 8. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Package to
`.git-exclude/review-request/063-prikk-0-58-rebaseline-b/review-request-v1.md`, and lead with:
- §1's diff;
- the withheld-import table: each case, its test, and its leg;
- §0's captures.

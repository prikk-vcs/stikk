# RFC 004 — stikk-export report schema

**Status.** Proposed
**Tracks.** The versioned shape of stikk-authored report exports. Referenced by the external design
(`CT-02`, `CT-04`) and data model (`DM-10`, `INV-7`) as deferred.
**Touches.** `stikk-state`'s `export.rs`; the report-producing view-models in `stikk-view`
(evidence, verify, refusal).

## Summary

stikk produces two kinds of file output on explicit user action (`CT-02`): prikk's `verify --format
json` passed through **byte-verbatim** (prikk's own `verify-report-v1`, not stikk's to define), and
**stikk-authored** exports of evidence/report/refusal views as text and versioned JSON (labelled
`stikk-export-v1`). This RFC proposes what `stikk-export-v1` guarantees. It does not fix every field
(those grow with the views); it fixes the invariants the schema must hold so a reader can trust an
export and never mistake it for live repository authority.

## The problem

An export is a **point-in-time snapshot** a user hands to someone else or reads back later. Two
failure modes to design against: (1) it is mistaken for live authority (someone acts on a stale merge
verdict as if current), and (2) it silently drops the honesty prikk and stikk worked to preserve (a
refusal exported without its witnesses, an authorship shown without its `Unverifiable` status). The
schema exists to make both impossible.

## Proposed invariants (`stikk-export-v1`)

1. **Every export is stamped** (`INV-7`): repository fingerprint, prikk version, stikk version, and
   capture time, stated in-band. The stamp is what makes it un-mistakable for live authority — a
   reader always knows *what* it is a snapshot of and *when*.
2. **prikk's bytes are never reserialized.** A verify export is prikk's `verify-report-v1` passed
   through unchanged (`CT-02`); a stikk export that *includes* prikk content carries it verbatim, not
   through a stikk re-encoder, so it stays valid against prikk's own schema.
3. **The honesty content is mandatory, not optional.** An exported refusal carries prikk's verbatim
   message and its witnesses; an exported signature outcome carries the three-valued status
   (`Sound` / `Unverifiable` / a blocking failure) — the export may not present authorship in a way
   that reads as verified when it is not (`NFR-I03`, `FS-04`-equivalent discipline).
4. **The export carries no secret and no redaction-listed content** (threat model `C-I3`): no blob
   bytes, no raw span/replacement text beyond what the user was viewing, no absolute host paths, no
   `.prikk` private paths, and never key material.
5. **It is versioned and self-describing.** `stikk-export-v1` names its version in-band; a `v2` is a
   new label, and a reader can refuse a version it does not understand rather than misread it.
6. **Text and JSON forms carry the same facts.** The human text is a rendering of the same content
   the JSON carries; neither omits what the other states.

## Consequences

- `export.rs` (`stikk-04` MOD-03) becomes implementable with a clear contract: stamp, verbatim
  passthrough vs. authored snapshot, temp-then-atomic-rename (`LC-12`), and the redaction filter.
- A CI pipeline can consume `stikk-export-v1` JSON (`CT-04`) to gate on evidence/verify outcomes,
  pinning the version.

## Open questions

- The concrete JSON field set per report kind (verify, evidence, refusal) — deferred until those
  views exist, since the schema should follow the view-models, not lead them.
- Whether the repository *fingerprint* in the stamp (RFC 003) is stable enough across prikk versions
  to serve as a cross-time identifier in an export, or whether the stamp should carry raw source
  identifiers instead. Settle alongside RFC 003.

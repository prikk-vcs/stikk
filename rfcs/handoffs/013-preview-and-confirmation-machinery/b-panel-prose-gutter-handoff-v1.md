# Handoff B — RFC 013: a right gutter for the panels' prose, not their identifiers (v1)

**Companion to:** [RFC 013](../../done/013-preview-and-confirmation-machinery.md) (the confirmation machinery).
**Covers:** 0.16.0's increment 1 (`.git-exclude/release/0-16-0-scope.md`, authorized 2026-10-21). **No RFC of its
own**: a carried review finding (RFC 047 Handoff B, review v2), and a rendering rule, not a new behaviour.
**Not a release.**

> **What this fixes.** The confirmation card wraps stikk's prose to the full inner width, so a word can land flush
> against the right border and read as cut off — measured on the merge card at 80×24: *"…from this baseline. A│"*.
> **The full width is deliberate for identifiers** (`overlay.rs:2583`: `block id: ` plus 64 hex is exactly the inner
> width of a 78-wide card), so the fix is **a gutter for prose only**. Rollback (increments 2–3) adds a card; this lands
> first.
>
> **No branch.** Local gates before submitting; after approval, push to `main`.

---

## 1. The rule

Two widths, each named for what it holds:

- **Prose stikk writes** — a consequence, a note, a notice, an error sentence, a message stikk composed — wraps at the
  panel's inner width **less one column**.
- **Identifiers and prikk's verbatim lines** — target rows, `block id:` lines, prikk's quoted output, anything drawn
  through `identifier` or `wrap_verbatim` — keep the **full** inner width. A 64-hex id must stay on one row, and
  prikk's lines are prikk's to wrap.

Introduce the prose width beside the existing constants (`PANEL_TEXT_WIDTH`, and the seal and refusal panels'
`SEAL_TEXT_WIDTH`, `REFUSAL_TEXT_WIDTH`) and **document both on the constants**, replacing the *"No right gutter,
deliberately"* paragraph with the two-width rule and its reason. **Apply it to all three panel kinds**: the same flush
word can land on a seal or refusal card.

## 2. Classify every call site

`grep -n "PANEL_TEXT_WIDTH\|SEAL_TEXT_WIDTH\|REFUSAL_TEXT_WIDTH" crates/stikk-tui/src/overlay.rs` lists them (about 45
today). **Each one is prose or identifier/verbatim.** Put the classification in the review request as a table — line,
what it wraps, which width, and why — so a reviewer can check every row rather than trust the sweep. Where a call
wraps a mix (a sentence that *contains* an id), say how you decided.

Do not change `text::wrap_*` themselves; the width is the caller's choice.

## 3. Tests

- **The merge card at 80×24**: the consequence no longer ends a row in the last inner column. Assert on the rendered
  buffer: **no prose row's last inner cell holds a non-space character**; and the target rows and the baseline id still
  render on one row each.
- **A 64-hex `block id:` line** on a card still fits one row (the reason for the full width; keep a test that fails if
  someone narrows it).
- **The captures**: every card capture that re-wraps is re-taken; paste them **as the file holds them**, and list which
  captures changed in the request. Captures whose prose did not re-wrap must be byte-identical.
- The overlay clipping gate (every variant, counted from the enum) stays green.

## 4. Text

`### Changed` in `## Unreleased`: one line — confirmation, seal and refusal cards keep a one-column margin after their
prose, so no word sits against the border. No `### Breaking` (nothing public changes; confirm with `cargo
public-api`).

## 5. The traps

1. **Identifiers keep the full width.** A 64-hex id folding onto two rows is the regression this rule exists to
   prevent.
2. **prikk's verbatim lines keep the full width.** They are prikk's, not stikk's prose.
3. **Re-take only what re-wrapped**, and say which.
4. **Paste captures as the file holds them; grep before committing**, including for repeated sections.

## 6. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.53.0. Package to
`.git-exclude/review-request/013-panel-prose-gutter/review-request-v1.md`, leading with the merge card's capture before
and after, then the call-site table.

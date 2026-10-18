# Handoff C — the declarations card drops whole declarations, never part of one (v1)

**Companion to:** [RFC 034](../../done/034-prikk-0-46-rebaseline.md) (§5 is this card). **No new RFC**: 0.15.0's
increment 1 (`.git-exclude/release/0-15-0-scope.md`, authorized 2026-10-18). Found reviewing RFC 040's Handoff C.
**Not a release.** **No branch.**

> **The defect.** `render_commit_would_refuse_declarations` (`crates/stikk-tui/src/overlay.rs:936`) builds every
> declaration into one prose block and lets `Panel` clip it. **This overlay does not scroll** (`app.rs:1660`: `↓` falls
> through), so at 80×24 the reader sees `lines 1–20 of 42` and **can never reach rows 21–42** — and the last visible rows
> can be a heading and `prikk reported —` with nothing under it. The title is honest; the card is still incomplete.

---

1. **Fit the declarations to the rows available, whole**, in [`frozen_lines`](../../../crates/stikk-tui/src/overlay.rs)'s
   shape (RFC 028 Handoff B §4): as many **complete** declarations as fit with room kept for a remainder line, then
   that line, its count exactly the declarations not shown. **At least the first declaration is always whole** — it is
   the one prikk's commit would stop on first.
2. **The remainder line is stikk's words, a fact and no key** (the `unshown_*_line` rule: `Esc` here closes the card,
   so do not invite it). Put it in `stikk-core` beside `unshown_paths_line`. It should say how many more declarations
   prikk would refuse, and that each has its own reason. Do not name a stikk view that does not list them.
3. **Rows are counted as drawn** (cells, not characters), so the count is exact for a wide-script path.
4. **The clipping gate's `CommitWouldRefuseDeclarations` case** (added in `1498736`) must now show **no** clip
   indicator at 80×24 — the card fits by construction. Keep its overflowing content; add a case with one declaration
   that fits outright.
5. **Changelog:** `### Fixed` — a card that could not show every refused declaration now shows whole ones and says how
   many more there are.
6. **Submit:** one commit, the eight gates, the grep, the 80×24 capture pasted as the file holds it. Package
   `.git-exclude/review-request/034-c-declarations-card/review-request-v1.md`. **After approval:** push, then CI's result
   on `main`.

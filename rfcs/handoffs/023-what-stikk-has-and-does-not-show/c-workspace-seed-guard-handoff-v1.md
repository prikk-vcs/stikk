# Handoff C — a workspace-wide seed-read guard (v1)

**Companion to:** [RFC 023](../../done/023-what-stikk-has-and-does-not-show.md), whose Handoff B wrote the key-id
module's guard. **No new RFC**: 0.14.0's increment 2 (`.git-exclude/release/0-14-0-scope.md`, authorized 2026-10-18;
`ROADMAP` *Later*, "a workspace-wide seed-read guard"). **Not a release.** **No branch.**

> **Why.** The README says *"No key material ever enters stikk … This is enforced by test"*. Today two tests hold it,
> each over **one file**: `stikk-prikk/src/env/tests.rs` (`env.rs` materializes no value) and
> `stikk-prikk/src/key_id/tests.rs` (`key_id.rs` never names a seed). A seed read added anywhere else in the six
> shipped crates would pass every test. **Close that before increment 3 gives stikk its first durable writes.**

---

## 1. The rule the guard enforces

Across **every `.rs` file under `src/` of the six published crates** (`stikk`, `stikk-model`, `stikk-prikk`,
`stikk-state`, `stikk-core`, `stikk-tui`) — code, not comments — **the substring `_SEED` appears only in an explicit
allowlist of files, each with its reason in the test.** Today's legitimate sites, from a grep (verify it; do not trust
this list):

- `stikk-prikk/src/env.rs` — reads **presence** only (its own guard proves no value is materialized);
- `stikk/src/main.rs` — the `--help` text that *states* the rule;
- `stikk-tui/src/view/orientation.rs` — Orientation's sentence that a `PRIKK_*_SEED` variable is set and this prikk
  ignores it (RFC 026 §4: presence, reported);
- any others the grep finds: **each one is a finding to justify in the request**, not to allowlist silently.

Test files (`tests.rs`, `*/tests/*`) and `#[cfg(test)]`-only modules may name seeds (they set fake ones); exclude them
**by path**, and say exactly which paths you excluded and why each is test-only.

## 2. Where it lives, and what proves it is looking

A plain (not `#[ignore]`d) test beside `rfc_index` in `crates/stikk-real-binary/tests/` — it reads the repository
tree, as that one does, and that crate is never published. **Two sanity checks**, so it cannot pass by scanning
nothing: every one of the six crates contributes at least one scanned file, and `env.rs` is among them **and does
contain `_SEED`** (the allowlist entry is live).

## 3. Text

- `README.md`'s *"enforced by test"* sentence becomes true as written; leave it, but **check the other claims near it
  still are**.
- `ROADMAP.md`: remove the *Later* bullet for this guard.
- **Changelog: no entry.** Nothing a user sees changes, and the test is in an unpublished crate.

## 4. Submit

One commit, the eight gates, the grep. Package
`.git-exclude/review-request/023-c-workspace-seed-guard/review-request-v1.md` with the allowlist, the exclusions and
the scanned-file count per crate. **After approval:** push, then CI's result on `main`.
